//! The squad leader. One `tick` per game tick: commands, new jobs, bot messages, liveness, claim renewals,
//! assignment, roster. Never blocks on anything but a claim reply (bounded by `renew_timeout`).
use null_bus::{Bus, CommandRx, IntentRx, Level, SummaryRx, Subject, kind};
use null_op::{op_claims, op_intents, squad_jobs};
use null_proto::*;
use null_bus::{bot_cmd, bot_jobs, bot_summary, bot_up};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// Spec: specs/F6-bus-deployment.md §4.2
/// Where HQ sends `Halt` and `Recall` for this squad.
pub fn squad_cmd(s: &ServerId, squad: &SquadId) -> Subject {
    Subject::new(s, Level::Squad, Some(squad.as_str()), kind::CMD)
}

// Spec: specs/K6-squad-leader.md §3
#[derive(Clone, Debug)]
pub struct SquadConfig {
    pub server: ServerId,
    pub squad: SquadId,
    pub op: OperationId,
    pub bots: Vec<BotId>,
    /// Game ticks between renews of a running job's claim. Well under `claim_ttl` (K6 §4.8).
    pub renew_every: Gt,
    pub claim_ttl: Gt,
    /// A bot whose state is older than this is silent: no renewing for it, reported lost.
    pub silent_after: Gt,
    /// Wall time to wait for a claim reply. No reply counts as a failed renew.
    pub renew_timeout: WallMs,
    /// Same-bot retries of a transient failure before the failure goes up (K6 §4.2).
    pub local_retries: u32,
}

impl SquadConfig {
    pub fn new(server: &str, squad: &str, op: &str, bots: &[&str]) -> Self {
        Self {
            server: ServerId::new(server),
            squad: SquadId::new(squad),
            op: OperationId::new(op),
            bots: bots.iter().map(|b| BotId::new(*b)).collect(),
            renew_every: Gt(20),
            claim_ttl: Gt(100),
            silent_after: Gt(30),
            renew_timeout: WallMs(2000),
            local_retries: 1,
        }
    }
}

// Spec: specs/K6-squad-leader.md §4.8
/// A job a bot is working: the claim lease and the tries so far.
struct Running {
    job: Job,
    expires_at: Gt,
    last_renew: Gt,
    tries: u32,
    /// What earlier tries reported, for the note when the failure goes up (K6 §4.2).
    failures: Vec<String>,
}

// Spec: specs/K6-squad-leader.md §3
struct BotRec {
    up: IntentRx,
    state: SummaryRx,
}

// Spec: specs/K6-squad-leader.md §3
pub struct Squad<B: Bus + Clone> {
    cfg: SquadConfig,
    bus: B,
    jobs: IntentRx,
    cmds: CommandRx,
    bots: BTreeMap<BotId, BotRec>,
    queue: VecDeque<Job>,
    running: BTreeMap<BotId, Running>,
    /// Bots that are dead, disconnected or silent.
    lost: BTreeSet<BotId>,
    last_roster: Vec<BotId>,
    /// Bots a bot-scope halt stopped; they get no new job.
    held: BTreeSet<BotId>,
    /// Set by a squad-wide halt or recall: nothing new is assigned.
    stopped: bool,
    /// Reflex settings set at squad scope; a job's changes revert to these (CONVENTIONS §4.2).
    inherited: Vec<ReflexSetting>,
    seen_obs: BTreeSet<String>,
    /// Every job this squad was given. An outcome for any other job came from an earlier squad's bot (H3).
    known_jobs: BTreeSet<JobId>,
    /// Outcomes passed up, kept so an operation restart can be answered (K6 §3).
    relayed: VecDeque<Intent>,
    resends: u32,
    now: Gt,
    /// Set by `begin_retire`: the halted bots' final outcomes still owed, and the tick the wait ends (H3).
    retiring: Option<Retiring>,
    /// Counters for tests and `inspect`.
    pub exceptions: Vec<Exception>,
}

// Spec: specs/K1-hq.md §4.2
struct Retiring {
    pending: BTreeSet<(BotId, JobId, u32)>,
    deadline: Gt,
}

/// A squad's receivers for its bots. A retired squad hands them on, so the next squad can use the same bots
/// (a one-consumer queue can be subscribed once; F6 §4.6).
// Spec: specs/K6-squad-leader.md §3
#[derive(Default)]
pub struct BotPorts(BTreeMap<BotId, BotRec>);

impl BotPorts {
    pub fn absorb(&mut self, other: BotPorts) {
        self.0.extend(other.0);
    }
}

const RELAY_KEEP: usize = 64;
/// Ticks a retiring squad waits for its halted bots' final outcomes. A silent bot cannot hold retire longer.
pub const RETIRE_WAIT: Gt = Gt(40);

impl<B: Bus + Clone> Squad<B> {
    /// Subscribes at once to its jobs, its commands and every bot's outcomes and state.
    // Spec: specs/K6-squad-leader.md §3
    pub fn new(bus: &B, cfg: SquadConfig) -> Self {
        Self::with_ports(bus, cfg, &mut BotPorts::default())
    }

    /// Like `new`, taking the receivers it needs from `ports` first.
    // Spec: specs/K6-squad-leader.md §3
    pub fn with_ports(bus: &B, cfg: SquadConfig, ports: &mut BotPorts) -> Self {
        let bots = cfg
            .bots
            .iter()
            .map(|b| {
                let rec = ports.0.remove(b).unwrap_or_else(|| BotRec {
                    up: bus
                        .subscribe_intents(&bot_up(&cfg.server, b))
                        .expect("bot up subject is free"),
                    state: bus.subscribe_summary(&bot_summary(&cfg.server, b)),
                });
                (b.clone(), rec)
            })
            .collect();
        Self {
            jobs: bus
                .subscribe_intents(&squad_jobs(&cfg.server, &cfg.squad))
                .expect("squad jobs subject is free"),
            cmds: bus.subscribe_commands(&squad_cmd(&cfg.server, &cfg.squad)),
            bus: bus.clone(),
            bots,
            queue: VecDeque::new(),
            running: BTreeMap::new(),
            lost: BTreeSet::new(),
            last_roster: vec![],
            held: BTreeSet::new(),
            stopped: false,
            inherited: vec![],
            seen_obs: BTreeSet::new(),
            known_jobs: BTreeSet::new(),
            relayed: VecDeque::new(),
            resends: 0,
            now: Gt(0),
            retiring: None,
            exceptions: vec![],
            cfg,
        }
    }

    /// Start retiring: halt every bot and put back the reflexes its jobs changed. The squad then keeps ticking
    /// (no new work) until `retire_done`, so the halted bots' last outcomes, with the logs they took, still go up
    /// to the operation (H3). A bot never keeps working for a squad that no longer renews its claims.
    // Spec: specs/K1-hq.md §4.2
    pub async fn begin_retire(&mut self) {
        let pending = self.running.iter().map(|(b, r)| (b.clone(), r.job.id.clone(), r.job.attempt)).collect();
        self.retiring = Some(Retiring { pending, deadline: self.now + RETIRE_WAIT });
        self.stopped = true;
        self.queue.clear();
        let ids: Vec<BotId> = self.bots.keys().cloned().collect();
        for b in ids {
            self.stop_bot(&b, "retired").await;
        }
    }

    /// True once every halted bot's final outcome came, or the wait bound passed.
    // Spec: specs/K1-hq.md §4.2
    pub fn retire_done(&self) -> bool {
        self.retiring.as_ref().is_some_and(|r| r.pending.is_empty() || self.now >= r.deadline)
    }

    /// End the retire: record each (job, attempt) that never reported, and hand the bot receivers on.
    // Spec: specs/K1-hq.md §4.2
    pub fn finish_retire(mut self) -> (BotPorts, Vec<Exception>) {
        if let Some(r) = self.retiring.take() {
            for (b, job, attempt) in r.pending {
                merge_exception(&mut self.exceptions, "retire_unreported", &format!("{b} {job} a{attempt}"), self.now);
            }
        }
        (BotPorts(self.bots), self.exceptions)
    }

    pub fn lost(&self) -> Vec<BotId> {
        self.lost.iter().cloned().collect()
    }
    /// (bot, job, attempt) for each running job.
    pub fn running(&self) -> Vec<(BotId, JobId, u32)> {
        self.running
            .iter()
            .map(|(b, r)| (b.clone(), r.job.id.clone(), r.job.attempt))
            .collect()
    }
    pub fn queued(&self) -> usize {
        self.queue.len()
    }
    pub fn is_stopped(&self) -> bool {
        self.stopped
    }

    /// One squad step at game tick `now`.
    // Spec: specs/K6-squad-leader.md §4.1
    pub async fn tick(&mut self, now: Gt) {
        self.now = now;
        if self.retiring.is_some() {
            // Only forward the halted bots' final outcomes; no commands, jobs, renewals or assignment.
            self.read_bots().await;
            return;
        }
        self.read_commands().await;
        self.read_jobs().await;
        self.read_bots().await;
        self.check_liveness().await;
        self.renew_due().await;
        self.assign().await;
        self.report_roster().await;
    }

    /// Halt and recall (K2 §4.9). Halt also stops the covered bots at once; recall only stops new work, since the
    /// skeleton has no base to return to.
    // Spec: specs/K2-commander-api.md §4.9
    async fn read_commands(&mut self) {
        while let Some(cmd) = self.cmds.try_recv() {
            let (Command::Halt { scope } | Command::Recall { scope }) = &cmd else {
                continue;
            };
            let halt = matches!(cmd, Command::Halt { .. });
            let covered: Vec<BotId> = self.bots.keys().filter(|b| self.covers(scope, b)).cloned().collect();
            // A bot-scope command on a one-bot squad covers every bot but is not a squad-wide stop.
            if !matches!(scope, Scope::Bot(_)) && covered.len() == self.bots.len() {
                self.stopped = true;
                self.queue.clear();
            } else {
                self.held.extend(covered.iter().cloned());
            }
            if halt {
                for b in covered {
                    self.stop_bot(&b, "halt").await;
                }
            }
        }
    }

    // Spec: specs/K2-commander-api.md §4.9
    fn covers(&self, scope: &Scope, bot: &BotId) -> bool {
        match scope {
            Scope::Swarm => true,
            Scope::Server(s) => *s == self.cfg.server,
            Scope::Operation(o) => *o == self.cfg.op,
            Scope::Squad(s) => *s == self.cfg.squad,
            Scope::Bot(b) => b == bot,
        }
    }

    /// Halt the bot and forget its job and lease. The bot's own `Preempted` outcome still goes up when it comes.
    // Spec: specs/K6-squad-leader.md §4.8
    async fn stop_bot(&mut self, bot: &BotId, why: &str) {
        let _ = self.bus.send_command(
            &bot_cmd(&self.cfg.server, bot),
            Command::Halt { scope: Scope::Bot(bot.clone()) },
        );
        if self.running.contains_key(bot) {
            merge_exception(&mut self.exceptions, &format!("stopped:{why}"), bot.as_str(), self.now);
            self.end_job(bot).await;
        }
    }

    /// Drop the running record and restore the reflexes the job changed (K6 §4.7).
    // Spec: specs/K6-squad-leader.md §4.7
    async fn end_job(&mut self, bot: &BotId) {
        let Some(run) = self.running.remove(bot) else {
            return;
        };
        // Lost bots too: the restore waits in the bot's queue behind the Halt and applies when it wakes.
        if run.job.reflexes.is_empty() {
            return;
        }
        let back = run
            .job
            .reflexes
            .iter()
            .map(|s| ReflexSetting {
                target: s.target.clone(),
                on: self.inherited.iter().find(|i| i.target == s.target).is_none_or(|i| i.on),
            })
            .collect();
        let key = format!("reflex-restore:{}:a{}", run.job.id, run.job.attempt);
        self.send_reflexes(bot, back, key, "job end").await;
    }

    // Spec: specs/K6-squad-leader.md §4.7
    async fn send_reflexes(&self, bot: &BotId, settings: Vec<ReflexSetting>, key: String, reason: &str) {
        let body = IntentBody::Command(Command::SetReflexes {
            scope: Scope::Bot(bot.clone()),
            settings,
            reason: reason.into(),
        });
        let _ = self.bus.publish_intent(&bot_jobs(&self.cfg.server, bot), Intent { key, body }).await;
    }

    /// New batches from the operation, and reflex settings at squad scope.
    // Spec: specs/K6-squad-leader.md §3
    async fn read_jobs(&mut self) {
        while let Some(i) = self.jobs.try_recv() {
            match i.body {
                IntentBody::JobBatch { jobs, .. } if !self.stopped => {
                    self.known_jobs.extend(jobs.iter().map(|j| j.id.clone()));
                    self.queue.extend(jobs);
                }
                IntentBody::JobBatch { jobs, .. } => {
                    for job in jobs {
                        self.report_unstarted(job, "batch_dropped_stopped", "squad stopped: job not started").await;
                    }
                }
                IntentBody::Command(Command::SetReflexes { settings, .. }) => {
                    for s in &settings {
                        self.inherited.retain(|o| o.target != s.target);
                        self.inherited.push(s.clone());
                    }
                    // Running bots keep their job's settings; they revert to the new inherited ones at job end.
                    let idle: Vec<BotId> = self.bots.keys().filter(|b| !self.running.contains_key(*b)).cloned().collect();
                    for b in idle {
                        let key = format!("reflex-squad:{}:{}", b, i.key);
                        self.send_reflexes(&b, settings.clone(), key, "squad setting").await;
                    }
                }
                _ => {}
            }
        }
    }

    /// Tell the operation at once that a job will not start, so it requeues without waiting for the claim to
    /// expire. `Preempted`: `Failed(Other)` is retried locally and `TargetGone` counts the job as done.
    // Spec: specs/K6-squad-leader.md §4.1
    async fn report_unstarted(&mut self, job: Job, exception: &str, note: &str) {
        merge_exception(&mut self.exceptions, exception, job.id.as_str(), self.now);
        self.known_jobs.insert(job.id.clone());
        let report = Report { note: note.into(), ..Report::default() };
        let outcome = Outcome { job: job.id.clone(), kind: OutcomeKind::Preempted, report };
        let bot = self.cfg.bots[0].clone();
        let body = IntentBody::JobOutcome(JobOutcome { bot, outcome, tick: self.now, attempt: job.attempt });
        self.pass_up(Intent { key: format!("fwd:{}:a{}", job.id, job.attempt), body }).await;
    }

    /// Outcomes (retry or pass up) and observations (merge, pass up once).
    // Spec: specs/K6-squad-leader.md §4.4
    async fn read_bots(&mut self) {
        let ids: Vec<BotId> = self.bots.keys().cloned().collect();
        for b in ids {
            while let Some(i) = self.bots.get_mut(&b).and_then(|r| r.up.try_recv()) {
                match i.body {
                    IntentBody::JobOutcome(o) => self.on_outcome(&b, o).await,
                    IntentBody::Observation(o) => self.on_observation(o).await,
                    _ => {}
                }
            }
        }
    }

    // Spec: specs/K6-squad-leader.md §4.4
    async fn on_observation(&mut self, o: Observation) {
        // The same block or entity seen by five bots is one observation (K6 §4.4).
        let key = serde_json::to_string(&o.what).expect("observation serializes");
        if self.seen_obs.insert(key.clone()) {
            let intent = Intent { key: format!("obs:{key}"), body: IntentBody::Observation(o) };
            let _ = self.bus.publish_intent(&op_intents(&self.cfg.server, &self.cfg.op), intent).await;
        }
    }

    /// A transient failure runs again on the same bot once, under the same claim; anything else goes up.
    // Spec: specs/K6-squad-leader.md §4.2
    async fn on_outcome(&mut self, bot: &BotId, mut o: JobOutcome) {
        if let Some(r) = &mut self.retiring {
            r.pending.remove(&(bot.clone(), o.outcome.job.clone(), o.attempt));
        }
        // A retired squad's bot reports late. Our operation does not know that job and would only log `Ignored`; the
        // owning operation is retired and gone, so its ledger cannot take the logs. Record them here instead.
        if !self.known_jobs.contains(&o.outcome.job) {
            let n: u32 = o.outcome.report.items_gained.iter().map(|c| c.n).sum();
            merge_exception(&mut self.exceptions, "late_outcome", &format!("{} items:{n}", o.outcome.job), self.now);
            return;
        }
        let mine = self
            .running
            .get(bot)
            .is_some_and(|r| r.job.id == o.outcome.job && r.job.attempt == o.attempt);
        if mine && !self.stopped && !self.held.contains(bot) && transient(&o.outcome.kind) {
            let run = self.running.get_mut(bot).expect("checked above");
            // A retry runs under the old lease; with the lease nearly out the bot would refuse it, so go up instead.
            let lease_ok = run.expires_at.0 > self.now.0 + LEASE_MARGIN;
            if run.tries <= self.cfg.local_retries && lease_ok {
                run.tries += 1;
                run.failures.push(format!("try {}: {:?}", run.tries - 1, o.outcome.kind));
                let (job, tries) = (run.job.clone(), run.tries);
                merge_exception(&mut self.exceptions, "local_retry", job.id.as_str(), self.now);
                self.send_job(bot, &job, format!("retry:{}:a{}:t{tries}", job.id, job.attempt)).await;
                return;
            }
        }
        if mine {
            let run = &self.running[bot];
            if !run.failures.is_empty() {
                o.outcome.report.note = format!("{} | earlier: {}", o.outcome.report.note, run.failures.join("; "));
            }
            self.end_job(bot).await;
        }
        let intent = Intent {
            key: format!("fwd:{}:a{}", o.outcome.job, o.attempt),
            body: IntentBody::JobOutcome(o),
        };
        self.pass_up(intent).await;
    }

    // Spec: specs/K6-squad-leader.md §4.4
    async fn pass_up(&mut self, intent: Intent) {
        let _ = self
            .bus
            .publish_intent(&op_intents(&self.cfg.server, &self.cfg.op), intent.clone())
            .await;
        self.relayed.push_back(intent);
        while self.relayed.len() > RELAY_KEEP {
            self.relayed.pop_front();
        }
    }

    /// After an operation restart: send the kept outcomes again under new keys. The operation counts each
    /// (job, attempt) once, so a repeat changes nothing (K5 §4.4).
    // Spec: specs/K6-squad-leader.md §3
    pub async fn resend_outcomes(&mut self) {
        self.resends += 1;
        let kept: Vec<Intent> = self.relayed.iter().cloned().collect();
        for mut i in kept {
            i.key = format!("{}~r{}", i.key, self.resends);
            let _ = self
                .bus
                .publish_intent(&op_intents(&self.cfg.server, &self.cfg.op), i)
                .await;
        }
    }

    /// Dead, disconnected and silent bots stop getting renewals and work; a silent one that wakes counts again.
    // Spec: specs/K6-squad-leader.md §5
    async fn check_liveness(&mut self) {
        let ids: Vec<BotId> = self.bots.keys().cloned().collect();
        for b in ids {
            let state = self.bots[&b].state.latest();
            let (life, seen) = match state {
                Some(SummaryBody::Bot(s)) => (s.life, s.tick),
                _ => (Life::Alive, Gt(0)),
            };
            let silent = self.now - seen > self.cfg.silent_after;
            let gone = life != Life::Alive || silent;
            if gone && self.lost.insert(b.clone()) {
                let why = if life != Life::Alive { "bot_lost" } else { "bot_silent" };
                merge_exception(&mut self.exceptions, why, b.as_str(), self.now);
                if silent {
                    // Queued, so a bot that wakes reads the halt before it works another tick.
                    self.stop_bot(&b, "silent").await;
                }
                // Its claim runs out by itself and the operation requeues the job. The job's reflexes still go back.
                self.end_job(&b).await;
            } else if !gone {
                self.lost.remove(&b);
            }
        }
    }

    /// Lease rule: renew each running job's claim with the id the job carries. Any failed renew halts that bot.
    // Spec: specs/K6-squad-leader.md §4.8
    async fn renew_due(&mut self) {
        let due: Vec<BotId> = self
            .running
            .iter()
            .filter(|(_, r)| self.now - r.last_renew >= self.cfg.renew_every)
            .map(|(b, _)| b.clone())
            .collect();
        for b in due {
            let job = self.running[&b].job.clone();
            match self.renew(&job).await {
                Ok(expires_at) => {
                    let run = self.running.get_mut(&b).expect("still running");
                    run.expires_at = expires_at;
                    run.last_renew = self.now;
                    self.send_lease(&b, &job, expires_at).await;
                }
                Err(_) => {
                    merge_exception(&mut self.exceptions, "renew_failed", job.id.as_str(), self.now);
                    self.stop_bot(&b, "renew_failed").await;
                    // The Halt can be lost; a lease that ends now stops the bot on its next tick anyway (K6 §4.8).
                    self.send_lease(&b, &job, self.now).await;
                }
            }
        }
    }

    /// The new `expires_at`, or why the claim is no longer ours.
    // Spec: specs/K6-squad-leader.md §4.8
    async fn renew(&self, job: &Job) -> Result<Gt, String> {
        let req = ClaimRequest::Renew {
            id: job.claims[0].clone(),
            owner: ClaimOwner::Job(job.id.clone()),
            ttl: self.cfg.claim_ttl,
        };
        match self
            .bus
            .request_claim(&op_claims(&self.cfg.server, &self.cfg.op), req, self.cfg.renew_timeout)
            .await
        {
            Ok(ClaimReply::Granted { expires_at, .. }) => Ok(expires_at),
            Ok(other) => Err(format!("{other:?}")),
            Err(e) => Err(e.to_string()),
        }
    }

    // Spec: specs/K6-squad-leader.md §4.8
    async fn send_lease(&self, bot: &BotId, job: &Job, expires_at: Gt) {
        let lease = Lease {
            bot: bot.clone(),
            job: job.id.clone(),
            attempt: job.attempt,
            claim: job.claims[0].clone(),
            expires_at,
        };
        // The try number is in the key: a retry needs its lease even when `expires_at` is unchanged.
        let tries = self.running.get(bot).map_or(0, |r| r.tries);
        let key = format!("lease:{}:a{}:t{tries}:{}", job.id, job.attempt, expires_at.0);
        let _ = self
            .bus
            .publish_intent(&bot_jobs(&self.cfg.server, bot), Intent { key, body: IntentBody::Lease(lease) })
            .await;
    }

    // Spec: specs/K6-squad-leader.md §4.1
    async fn send_job(&self, bot: &BotId, job: &Job, key: String) {
        if let Some(r) = self.running.get(bot) {
            self.send_lease(bot, job, r.expires_at).await;
        }
        let body = IntentBody::JobBatch { squad: self.cfg.squad.clone(), jobs: vec![job.clone()] };
        let _ = self.bus.publish_intent(&bot_jobs(&self.cfg.server, bot), Intent { key, body }).await;
    }

    /// Each queued job goes to the idle bot nearest its first tree. A job whose claim is no longer ours
    /// (it expired while queued; the operation has requeued it) is dropped, never started.
    // Spec: specs/K6-squad-leader.md §4.1
    async fn assign(&mut self) {
        if self.stopped {
            return;
        }
        // Every bot held or lost: nothing can start these jobs, so the operation must hear of it (H4).
        if !self.queue.is_empty() && self.bots.keys().all(|b| self.held.contains(b) || self.lost.contains(b)) {
            while let Some(job) = self.queue.pop_front() {
                self.report_unstarted(job, "batch_dropped_held", "no free bot: all held").await;
            }
            return;
        }
        while let Some(job) = self.queue.front() {
            let Some(bot) = self.nearest_idle(job) else {
                return;
            };
            let job = self.queue.pop_front().expect("front exists");
            let expires_at = match self.renew(&job).await {
                Ok(e) => e,
                Err(e) => {
                    merge_exception(&mut self.exceptions, "stale_job", job.id.as_str(), self.now);
                    // Tell the operation now, so it requeues without waiting for the claim to expire.
                    let kind = OutcomeKind::Failed(FailReason::Other(format!("claim lost before start: {e}")));
                    let outcome = Outcome { job: job.id.clone(), kind, report: Report::default() };
                    let body = IntentBody::JobOutcome(JobOutcome { bot, outcome, tick: self.now, attempt: job.attempt });
                    let intent = Intent { key: format!("fwd:{}:a{}", job.id, job.attempt), body };
                    self.pass_up(intent).await;
                    continue;
                }
            };
            if !job.reflexes.is_empty() {
                let key = format!("reflex-apply:{}:a{}", job.id, job.attempt);
                self.send_reflexes(&bot, job.reflexes.clone(), key, "job start").await;
            }
            let key = format!("assign:{}:a{}", job.id, job.attempt);
            self.running.insert(
                bot.clone(),
                Running { job: job.clone(), expires_at, last_renew: self.now, tries: 1, failures: vec![] },
            );
            self.send_job(&bot, &job, key).await;
        }
    }

    // Spec: specs/K6-squad-leader.md §4.1
    fn nearest_idle(&self, job: &Job) -> Option<BotId> {
        let target = match job.params.get("trees") {
            Some(ParamValue::Locs(l)) => l.first().map(|l| l.pos),
            _ => None,
        };
        self.bots
            .iter()
            .filter(|(b, _)| !self.running.contains_key(*b) && !self.lost.contains(*b) && !self.held.contains(*b))
            .map(|(b, rec)| {
                let at = match rec.state.latest() {
                    Some(SummaryBody::Bot(s)) => s.at_loc.map(|l| l.pos),
                    _ => None,
                }
                .unwrap_or(Pos::new(0, 64, 0));
                let d = target.map_or(0, |t| (t.x - at.x).abs() + (t.z - at.z).abs());
                (d, b.clone())
            })
            .min()
            .map(|(_, b)| b)
    }

    /// Tell the operation which bots are lost, when that list changes (so its headcount is the live count).
    // Spec: specs/K6-squad-leader.md §5
    async fn report_roster(&mut self) {
        let lost = self.lost();
        if lost == self.last_roster {
            return;
        }
        let key = format!("roster:{}:{}", self.cfg.squad, self.now.0);
        let body = IntentBody::Roster { squad: self.cfg.squad.clone(), lost: lost.clone() };
        let _ = self.bus.publish_intent(&op_intents(&self.cfg.server, &self.cfg.op), Intent { key, body }).await;
        self.last_roster = lost;
    }
}

/// Failures worth one more try on the same bot. A vanished tree, a missing tool or a halt are not (K6 §4.2).
// Spec: specs/K6-squad-leader.md §4.2
fn transient(k: &OutcomeKind) -> bool {
    matches!(
        k,
        OutcomeKind::Failed(FailReason::NoPath | FailReason::BudgetExhausted | FailReason::Other(_)) | OutcomeKind::Interrupted(_)
    )
}
