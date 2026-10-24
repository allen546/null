//! A fake bot: runs `fell` jobs in simulated ticks and talks only S2 messages. Spec: specs/K6-squad-leader.md §3
use crate::{SharedWorld, SimRng, bot_cmd, bot_jobs, bot_summary, bot_up, oak_log};
use null_bus::{Bus, CommandRx, IntentRx};
use null_proto::*;
use std::collections::VecDeque;

// Spec: specs/K6-squad-leader.md §3
struct Running {
    job: Job,
    trees: Vec<Pos>,
    idx: usize,
    /// Ticks left on the current walk or fell step.
    left: u32,
    felling: bool,
    gained: u32,
    started: Gt,
    skipped: u32,
}

// Spec: specs/K6-squad-leader.md §3
pub struct SimBot<B: Bus + Clone> {
    pub id: BotId,
    bus: B,
    server: ServerId,
    world: SharedWorld,
    rng: SimRng,
    jobs: IntentRx,
    cmds: CommandRx,
    queue: VecDeque<Job>,
    running: Option<Running>,
    life: Life,
    pos: Pos,
    die_at: Option<Gt>,
    disconnect_at: Option<Gt>,
    /// A stall: the bot is alive but does nothing in this tick range (a hung client, a lag spike).
    stall: Option<(Gt, Gt)>,
}

impl<B: Bus + Clone> SimBot<B> {
    /// Subscribes at once. The RNG seed mixes the run seed with the bot's name, so each bot replays on its own.
    // Spec: specs/F3-test-harness.md §4.5
    pub fn new(bus: &B, world: &SharedWorld, id: &str, seed: u64) -> Self {
        let id = BotId::new(id);
        let server = world.lock().unwrap().server.clone();
        let mix = id
            .as_str()
            .bytes()
            .fold(seed, |h, b| h.wrapping_mul(31).wrapping_add(u64::from(b)));
        Self {
            jobs: bus
                .subscribe_intents(&bot_jobs(&server, &id))
                .expect("bot jobs subject is free"),
            cmds: bus.subscribe_commands(&bot_cmd(&server, &id)),
            bus: bus.clone(),
            server,
            world: world.clone(),
            rng: SimRng::new(mix),
            id,
            queue: VecDeque::new(),
            running: None,
            life: Life::Alive,
            pos: Pos::new(0, 64, 0),
            die_at: None,
            disconnect_at: None,
            stall: None,
        }
    }
    /// Tests trigger death and disconnect at a chosen tick.
    pub fn die_at(&mut self, t: Gt) {
        self.die_at = Some(t);
    }
    pub fn disconnect_at(&mut self, t: Gt) {
        self.disconnect_at = Some(t);
    }
    /// Alive but frozen from `from` up to (not including) `until`: no work, no outcome, state still published.
    pub fn stall(&mut self, from: Gt, until: Gt) {
        self.stall = Some((from, until));
    }
    pub fn life(&self) -> Life {
        self.life
    }
    pub fn is_idle(&self) -> bool {
        self.life == Life::Alive && self.running.is_none() && self.queue.is_empty()
    }

    /// One game tick: read messages, advance the job, publish state.
    // Spec: specs/K6-squad-leader.md §3
    pub async fn tick(&mut self, now: Gt) {
        if self.life == Life::Alive {
            if self.die_at.is_some_and(|t| now >= t) {
                self.life = Life::Dead;
            } else if self.disconnect_at.is_some_and(|t| now >= t) {
                self.life = Life::Disconnected;
            }
            if self.life != Life::Alive {
                // A lost bot sends no outcome: its squad learns from `BotState` (CONVENTIONS §3.1, K5 §4.6).
                self.running = None;
                self.queue.clear();
            }
        }
        let stalled = self.stall.is_some_and(|(a, z)| now >= a && now < z);
        if self.life == Life::Alive && !stalled {
            while let Some(Command::Halt { .. }) = self.cmds.try_recv() {
                if let Some(r) = self.running.take() {
                    self.finish(r, OutcomeKind::Preempted, now).await;
                }
            }
            while let Some(i) = self.jobs.try_recv() {
                if let IntentBody::JobBatch { jobs, .. } = i.body {
                    self.queue.extend(jobs);
                }
            }
            if self.running.is_none() {
                if let Some(job) = self.queue.pop_front() {
                    self.running = Some(self.begin(job, now));
                }
            }
            self.advance(now).await;
        }
        self.publish_state(now);
    }

    // Spec: specs/K6-squad-leader.md §3
    fn begin(&mut self, job: Job, now: Gt) -> Running {
        let trees = match job.params.get("trees") {
            Some(ParamValue::Locs(l)) => l.iter().map(|l| l.pos).collect(),
            _ => vec![],
        };
        let mut r = Running {
            job,
            trees,
            idx: 0,
            left: 0,
            felling: false,
            gained: 0,
            started: now,
            skipped: 0,
        };
        self.start_walk(&mut r);
        r
    }

    // Spec: specs/K6-squad-leader.md §3
    fn start_walk(&mut self, r: &mut Running) {
        r.felling = false;
        if let Some(t) = r.trees.get(r.idx) {
            let d = ((t.x - self.pos.x).abs() + (t.z - self.pos.z).abs()) as u32;
            r.left = d * self.rng.range(1, 2) + 1;
        }
    }

    // Spec: specs/K6-squad-leader.md §3
    async fn advance(&mut self, now: Gt) {
        let Some(mut r) = self.running.take() else {
            return;
        };
        if now.0 - r.started.0 > r.job.budget.ticks.0 {
            return self
                .finish(r, OutcomeKind::Failed(FailReason::BudgetExhausted), now)
                .await;
        }
        if r.idx >= r.trees.len() {
            let kind = if r.gained == 0 && r.skipped > 0 {
                OutcomeKind::Failed(FailReason::TargetGone)
            } else {
                OutcomeKind::Succeeded
            };
            return self.finish(r, kind, now).await;
        }
        r.left = r.left.saturating_sub(1);
        if r.left == 0 {
            let tree = r.trees[r.idx];
            if !r.felling {
                // Arrived: felling takes about 3 ticks per log, if the tree is still there.
                let here = self.world.lock().unwrap().logs_at(tree);
                match here {
                    Some(logs) => {
                        r.felling = true;
                        r.left = logs * 3 + self.rng.range(0, 3);
                        self.pos = tree;
                    }
                    None => {
                        r.skipped += 1;
                        r.idx += 1;
                        self.start_walk(&mut r);
                    }
                }
            } else {
                let took = self.world.lock().unwrap().take(tree);
                match took {
                    Some(logs) => r.gained += logs,
                    None => r.skipped += 1,
                }
                r.idx += 1;
                self.start_walk(&mut r);
            }
        }
        self.running = Some(r);
    }

    // Spec: specs/CONVENTIONS.md §3.1
    async fn finish(&mut self, r: Running, kind: OutcomeKind, now: Gt) {
        let items_gained = if r.gained > 0 {
            vec![ItemCount {
                item: oak_log(),
                n: r.gained,
            }]
        } else {
            vec![]
        };
        let outcome = Outcome {
            job: r.job.id.clone(),
            kind,
            report: Report {
                elapsed: now - r.started,
                items_gained,
                items_lost: vec![],
                at: None,
                note: format!("{} trees gone", r.skipped),
            },
        };
        let key = format!("out:{}:{}:{}", r.job.id, self.id, now.0);
        let body = IntentBody::JobOutcome(JobOutcome {
            bot: self.id.clone(),
            outcome,
            tick: now,
            attempt: r.job.attempt,
        });
        let _ = self
            .bus
            .publish_intent(&bot_up(&self.server, &self.id), Intent { key, body })
            .await;
    }

    // Spec: specs/K6-squad-leader.md §3
    fn publish_state(&self, now: Gt) {
        let st = BotState {
            bot: self.id.clone(),
            life: self.life,
            at_loc: Some(Loc {
                server: self.server.clone(),
                dim: Dimension::Overworld,
                pos: self.pos,
            }),
            health: if self.life == Life::Dead { 0.0 } else { 20.0 },
            job: self.running.as_ref().map(|r| r.job.id.clone()),
            tick: now,
        };
        self.bus
            .publish_summary(&bot_summary(&self.server, &self.id), SummaryBody::Bot(st));
    }
}
