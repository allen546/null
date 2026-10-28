//! The bot's side of the squad protocol on a real bot. Mirrors `null_simbot::SimBot` (the reference for subjects, halt,
//! outcome shape and `BotState`); the work itself runs on the host, not in simulated ticks.
use crate::run::{FellSpec, RunEnd, RunHandle, RunStatus};
use null_bus::{Bus, CommandRx, IntentRx, bot_cmd, bot_jobs, bot_summary, bot_up};
use null_proto::*;
use std::collections::{BTreeMap, VecDeque};

/// Ticks a stopped job waits for its skill to report before the bot answers without it (a frozen client).
const STOP_WAIT: u64 = 100;

/// A host's view of its bot, read once per tick. Reads never wait.
#[derive(Clone, Debug)]
pub struct HostStatus {
    pub life: Life,
    pub pos: Option<Pos>,
    pub health: f32,
}

// Spec: specs/K6-squad-leader.md §3
/// The bot body. `start` hands a `fell` run to the bot's tick loop and returns at once.
pub trait Host {
    fn start(&mut self, spec: FellSpec) -> RunHandle;
    fn status(&mut self) -> HostStatus;
}

struct Active {
    job: Job,
    handle: RunHandle,
    started: Gt,
    /// Set when this link asked for a stop: the outcome kind to report when the skill has stopped.
    stopping: Option<(OutcomeKind, Gt)>,
}

// Spec: specs/K6-squad-leader.md §3
pub struct BotLink<B: Bus + Clone> {
    pub id: BotId,
    bus: B,
    server: ServerId,
    jobs: IntentRx,
    cmds: CommandRx,
    queue: VecDeque<Job>,
    running: Option<Active>,
    life: Life,
    pos: Option<Pos>,
    health: f32,
    /// Latest `expires_at` the squad sent per (job, attempt). No lease means no work: the check fails closed (K6 §4.8).
    leases: BTreeMap<(JobId, u32), Gt>,
}

impl<B: Bus + Clone> BotLink<B> {
    /// Subscribes at once.
    // Spec: specs/K6-squad-leader.md §3
    pub fn new(bus: &B, server: &ServerId, id: &str) -> Self {
        let id = BotId::new(id);
        Self {
            jobs: bus.subscribe_intents(&bot_jobs(server, &id)).expect("bot jobs subject is free"),
            cmds: bus.subscribe_commands(&bot_cmd(server, &id)),
            bus: bus.clone(),
            server: server.clone(),
            id,
            queue: VecDeque::new(),
            running: None,
            life: Life::Alive,
            pos: None,
            health: 20.0,
            leases: BTreeMap::new(),
        }
    }

    pub fn is_idle(&self) -> bool {
        self.life == Life::Alive && self.running.is_none() && self.queue.is_empty()
    }
    pub fn life(&self) -> Life {
        self.life
    }

    /// One tick: read messages, start or stop the job, publish state. Never waits on the bot.
    // Spec: specs/K6-squad-leader.md §3
    pub async fn tick(&mut self, now: Gt, host: &mut dyn Host) {
        let st = host.status();
        (self.life, self.pos, self.health) = (st.life, st.pos, st.health);
        if self.life != Life::Alive {
            // A lost bot sends no outcome: its squad learns from `BotState` (CONVENTIONS §3.1, K5 §4.6). What waited
            // for it is stale; the squad hands out fresh work after the bot is back.
            self.running = None;
            self.queue.clear();
            self.leases.clear();
            while self.cmds.try_recv().is_some() {}
            while self.jobs.try_recv().is_some() {}
        } else {
            self.read(now);
            self.begin(now, host).await;
            self.advance(now).await;
        }
        self.publish_state(now);
    }

    // Spec: specs/K6-squad-leader.md §3
    fn read(&mut self, now: Gt) {
        let mut halted = false;
        while let Some(Command::Halt { .. }) = self.cmds.try_recv() {
            halted = true;
            self.stop(OutcomeKind::Preempted, now);
        }
        while let Some(i) = self.jobs.try_recv() {
            match i.body {
                IntentBody::JobBatch { jobs, .. } => self.queue.extend(jobs),
                // The bus keeps order per subject, so the last lease wins, a shorter one too (K6 §4.8).
                IntentBody::Lease(l) => {
                    self.leases.insert((l.job, l.attempt), l.expires_at);
                }
                _ => {}
            }
        }
        // A halt stops all work; jobs read in the same tick sit behind it in time. The squad already dropped them.
        if halted {
            self.queue.clear();
        }
    }

    fn lease_ok(&self, job: &Job, now: Gt) -> bool {
        self.leases.get(&(job.id.clone(), job.attempt)).is_some_and(|exp| now.0 + LEASE_MARGIN < exp.0)
    }

    /// The job's `fell` parameters. A job without a claim box is refused: the bot never breaks outside a held claim.
    // Spec: specs/CONVENTIONS.md §3.1
    fn spec(&self, job: &Job) -> Result<FellSpec, String> {
        if job.kind != "fell" {
            return Err(format!("unknown job kind {}", job.kind));
        }
        let Some(ParamValue::Region(r)) = job.params.get("claim") else {
            return Err("fell job has no claim box".into());
        };
        if r.server != self.server || r.dim != Dimension::Overworld {
            return Err("claim box is on another server or dimension".into());
        }
        let quota = match job.params.get("quota") {
            Some(ParamValue::Int(n)) if *n > 0 => *n as u32,
            _ => return Err("fell job has no quota".into()),
        };
        let tree_type = match job.params.get("tree_type") {
            Some(ParamValue::Text(t)) => t.clone(),
            _ => "oak".into(),
        };
        Ok(FellSpec { tree_type, quota, min: r.min, max: r.max })
    }

    // Spec: specs/K6-squad-leader.md §4.8
    async fn begin(&mut self, now: Gt, host: &mut dyn Host) {
        while self.running.is_none() {
            let Some(job) = self.queue.pop_front() else { return };
            if !self.lease_ok(&job, now) {
                // No lease, or one about to end: the job does not start.
                self.refuse(job, OutcomeKind::Preempted, "no live lease", now).await;
            } else {
                match self.spec(&job) {
                    Ok(spec) => {
                        let handle = host.start(spec);
                        self.running = Some(Active { job, handle, started: now, stopping: None });
                    }
                    Err(why) => self.refuse(job, OutcomeKind::Failed(FailReason::Other(why.clone())), &why, now).await,
                }
            }
        }
    }

    async fn refuse(&mut self, job: Job, kind: OutcomeKind, note: &str, now: Gt) {
        let a = Active { job, handle: RunHandle::new(), started: now, stopping: None };
        self.finish(a, kind, None, note, now).await;
    }

    /// Ask the skill to stop; the outcome goes when it has stopped.
    fn stop(&mut self, kind: OutcomeKind, now: Gt) {
        if let Some(a) = &mut self.running {
            if a.stopping.is_none() {
                a.handle.request_stop();
                a.stopping = Some((kind, now));
            }
        }
    }

    // Spec: specs/K6-squad-leader.md §4.8
    async fn advance(&mut self, now: Gt) {
        let Some(a) = &self.running else { return };
        if let Some(end) = a.handle.take_end() {
            let a = self.running.take().expect("checked");
            let kind = match (&a.stopping, &end.status) {
                (Some((k, _)), _) => k.clone(),
                (None, RunStatus::Done) if end.items.is_empty() && end.gathered == 0 => OutcomeKind::Failed(FailReason::TargetGone),
                (None, RunStatus::Done) => OutcomeKind::Succeeded,
                (None, RunStatus::Failed(why)) => OutcomeKind::Failed(FailReason::Other(why.clone())),
                (None, RunStatus::Stopped) => OutcomeKind::Preempted,
            };
            let note = format!("gathered {}, targets left {}, drops left {}", end.gathered, end.left, end.drops_left);
            return self.finish(a, kind, Some(end), &note, now).await;
        }
        match &a.stopping {
            Some((kind, since)) if now.0 - since.0 > STOP_WAIT => {
                let kind = kind.clone();
                let a = self.running.take().expect("checked");
                return self.finish(a, kind, None, "the skill did not report after the stop", now).await;
            }
            Some(_) => return,
            None => {}
        }
        // Lease rule: stop before the claim can pass to another attempt, even if no Halt ever arrives.
        if now.0 - a.started.0 > a.job.budget.ticks.0 {
            self.stop(OutcomeKind::Failed(FailReason::BudgetExhausted), now);
        } else if !self.lease_ok(&a.job.clone(), now) {
            self.stop(OutcomeKind::Preempted, now);
        }
    }

    // Spec: specs/CONVENTIONS.md §3.1
    async fn finish(&mut self, a: Active, kind: OutcomeKind, end: Option<RunEnd>, note: &str, now: Gt) {
        self.leases.remove(&(a.job.id.clone(), a.job.attempt));
        let items_gained = end
            .iter()
            .flat_map(|e| e.items.iter())
            .map(|(item, n)| ItemCount {
                item: ItemId::new(if item.contains(':') { item.clone() } else { format!("minecraft:{item}") }),
                n: *n,
            })
            .collect();
        let outcome = Outcome {
            job: a.job.id.clone(),
            kind,
            report: Report { elapsed: now - a.started, items_gained, items_lost: vec![], at: None, note: note.to_string() },
        };
        let key = format!("out:{}:{}:{}:{}", a.job.id, a.job.attempt, self.id, now.0);
        let body = IntentBody::JobOutcome(JobOutcome { bot: self.id.clone(), outcome, tick: now, attempt: a.job.attempt });
        let _ = self.bus.publish_intent(&bot_up(&self.server, &self.id), Intent { key, body }).await;
    }

    // Spec: specs/K6-squad-leader.md §3
    fn publish_state(&self, now: Gt) {
        let st = BotState {
            bot: self.id.clone(),
            life: self.life,
            at_loc: self.pos.map(|pos| Loc { server: self.server.clone(), dim: Dimension::Overworld, pos }),
            health: if self.life == Life::Dead { 0.0 } else { self.health },
            job: self.running.as_ref().map(|a| a.job.id.clone()),
            tick: now,
        };
        self.bus.publish_summary(&bot_summary(&self.server, &self.id), SummaryBody::Bot(st));
    }
}
