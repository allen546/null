//! The operation: expands a goal to jobs, claims trees, hands batches to the squad, counts logs from outcomes
//! and rebuilds itself from its log. Every state change is an [`OpEvent`]; replay uses the same `apply`.
use crate::claims::{ClaimEvent, ClaimRegistry};
use crate::events::{EventLog, OpEvent, decode, wall_of};
use null_bus::{Bus, ClaimServer, CommandRx, IntentRx, Level, Subject, kind};
use null_proto::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

/// Blocks from a tree's trunk to the edge of its claim box on each side (the canopy reaches two, one block of room).
pub const TREE_BOX_RADIUS: i32 = 3;
/// Blocks above the top log that the claim box reaches (the canopy).
pub const TREE_BOX_HEAD: i32 = 2;

// Subjects (F6 §4.2). The squad and HQ use the same functions, so the names live in one place.
// Spec: specs/F6-bus-deployment.md §4.2
/// Inbound intents of an operation: goal, allocation, job outcomes, commands.
pub fn op_intents(s: &ServerId, op: &OperationId) -> Subject {
    Subject::new(s, Level::Op, Some(op.as_str()), kind::JOBS)
}
// Spec: specs/F6-bus-deployment.md §4.2
pub fn op_cmd(s: &ServerId, op: &OperationId) -> Subject {
    Subject::new(s, Level::Op, Some(op.as_str()), kind::CMD)
}
// Spec: specs/F6-bus-deployment.md §4.2
pub fn op_claims(s: &ServerId, op: &OperationId) -> Subject {
    Subject::new(s, Level::Op, Some(op.as_str()), kind::CLAIMS)
}
// Spec: specs/F6-bus-deployment.md §4.2
pub fn op_events(s: &ServerId, op: &OperationId) -> Subject {
    Subject::new(s, Level::Op, Some(op.as_str()), kind::EVENTS)
}
// Spec: specs/F6-bus-deployment.md §4.2
pub fn op_summary(s: &ServerId, op: &OperationId) -> Subject {
    Subject::new(s, Level::Op, Some(op.as_str()), kind::SUMMARY)
}
// Spec: specs/F6-bus-deployment.md §4.2
/// Where the operation sends `IntentBody::JobBatch` to its squad.
pub fn squad_jobs(s: &ServerId, squad: &SquadId) -> Subject {
    Subject::new(s, Level::Squad, Some(squad.as_str()), kind::JOBS)
}
// Spec: specs/F6-bus-deployment.md §4.2
pub fn hq_escalate(s: &ServerId) -> Subject {
    Subject::new(s, Level::Hq, None, kind::ESCALATE)
}

// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Debug)]
pub struct OpConfig {
    pub op: OperationId,
    pub server: ServerId,
    pub squad: SquadId,
    pub priority: Priority,
    /// Game ticks a claim lives without a renew.
    pub claim_ttl: Gt,
    /// Extra logs to plan for, in percent of the goal, so a lost tree does not stall the goal.
    pub margin_pct: u32,
    /// Dispatches of one job before it is given up and escalated.
    pub max_attempts: u32,
    pub job_budget: Gt,
    /// Claim box of a `fell` job: blocks out from the trunk on each side, and above the top log. 0 and 0 is the trunk's
    /// column only (the simulated trees stand on neighbouring cells; a real tree's canopy needs the defaults).
    pub tree_box: (i32, i32),
    /// Mirror the log to this JSONL file.
    pub jsonl: Option<PathBuf>,
}

impl OpConfig {
    pub fn new(op: &str, server: &str, squad: &str) -> Self {
        Self {
            op: OperationId::new(op),
            server: ServerId::new(server),
            squad: SquadId::new(squad),
            priority: Priority(1),
            claim_ttl: Gt(100),
            margin_pct: 25,
            max_attempts: 3,
            job_budget: Gt(600),
            tree_box: (TREE_BOX_RADIUS, TREE_BOX_HEAD),
            jsonl: None,
        }
    }
}

/// The transport handles. A restarted operation takes the old ones back, as a NATS reconnect would (F6 §4.6).
// Spec: specs/K5-operation-service.md §4.9
pub struct OpPorts {
    pub intents: IntentRx,
    pub cmds: CommandRx,
    pub claims: ClaimServer,
}

// Spec: specs/K5-operation-service.md §4.2
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobSt {
    Pending,
    InFlight,
    Done,
    Dead,
}

// Spec: specs/K5-operation-service.md §4.2
#[derive(Clone, Debug)]
struct JobRec {
    job: Job,
    logs: u32,
    st: JobSt,
    attempts: u32,
}

/// The replayable part of the operation. Changes only through [`Core::apply`].
// Spec: specs/K5-operation-service.md §4.9
struct Core {
    goal: Option<GoalSpec>,
    state: OpState,
    bots: Vec<BotId>,
    /// Bots the squad reported as lost; they stay in `bots` (their late outcomes still count).
    lost: BTreeSet<BotId>,
    jobs: BTreeMap<JobId, JobRec>,
    used_trees: BTreeSet<Loc>,
    /// Ledger (K5 §4.4): items counted from outcomes only.
    ledger: BTreeMap<ItemId, u32>,
    seen_outcomes: BTreeSet<String>,
    next_job: u64,
    exceptions: Vec<Exception>,
}

impl Core {
    fn new() -> Self {
        Self {
            goal: None,
            state: OpState::Planning,
            bots: vec![],
            lost: BTreeSet::new(),
            jobs: BTreeMap::new(),
            used_trees: BTreeSet::new(),
            ledger: BTreeMap::new(),
            seen_outcomes: BTreeSet::new(),
            next_job: 1,
            exceptions: vec![],
        }
    }

    // Spec: specs/K5-operation-service.md §4.9
    fn apply(&mut self, ev: &OpEvent, at: Gt) {
        let set = |c: &mut Core, j: &JobId, st: JobSt| {
            if let Some(r) = c.jobs.get_mut(j) {
                r.st = st;
            }
        };
        match ev {
            OpEvent::GoalSet { goal } => self.goal = Some(goal.clone()),
            OpEvent::Allocated { bots } => self.bots = bots.clone(),
            OpEvent::JobCreated { job, tree, logs } => {
                self.used_trees.insert(tree.clone());
                self.next_job = self.next_job.max(job_number(&job.id) + 1);
                self.jobs.insert(
                    job.id.clone(),
                    JobRec {
                        job: job.clone(),
                        logs: *logs,
                        st: JobSt::Pending,
                        attempts: 0,
                    },
                );
            }
            OpEvent::Claim(_) | OpEvent::Ignored { .. } => {}
            OpEvent::Dispatched { job, attempt } => {
                set(self, job, JobSt::InFlight);
                if let Some(r) = self.jobs.get_mut(job) {
                    r.attempts = *attempt;
                }
            }
            OpEvent::Outcome { key, outcome, .. } => {
                if self.seen_outcomes.insert(key.clone()) {
                    for c in &outcome.report.items_gained {
                        *self.ledger.entry(c.item.clone()).or_default() += c.n;
                    }
                }
            }
            OpEvent::JobDone { job } => set(self, job, JobSt::Done),
            OpEvent::Requeued { job, reason } => {
                set(self, job, JobSt::Pending);
                merge_exception(&mut self.exceptions, reason, job.as_str(), at);
            }
            OpEvent::JobDead { job, reason } => {
                set(self, job, JobSt::Dead);
                if reason != GOAL_MET {
                    merge_exception(
                        &mut self.exceptions,
                        &format!("dead:{reason}"),
                        job.as_str(),
                        at,
                    );
                }
            }
            OpEvent::State { state } => self.state = *state,
            OpEvent::Roster { lost } => self.lost = lost.iter().cloned().collect(),
        }
    }

    /// Assigned bots the squad can still use.
    fn alive(&self) -> usize {
        self.bots.iter().filter(|b| !self.lost.contains(*b)).count()
    }

    fn outstanding(&self) -> u32 {
        self.jobs
            .values()
            .filter(|r| matches!(r.st, JobSt::Pending | JobSt::InFlight))
            .map(|r| r.logs)
            .sum()
    }
    fn target(&self) -> Option<(&ItemId, u32)> {
        match &self.goal {
            Some(GoalSpec {
                goal: Goal::Obtain { item, n },
                ..
            }) => Some((item, *n)),
            _ => None,
        }
    }
    fn have(&self) -> u32 {
        self.target()
            .map_or(0, |(i, _)| self.ledger.get(i).copied().unwrap_or(0))
    }
}

const GOAL_MET: &str = "goal met";

/// The claim of one dispatch. Each attempt has its own id, so a stalled bot holding an older attempt can never
/// renew or release the claim of the attempt that replaced it (it gets `NotHeld`).
// Spec: specs/K5-operation-service.md §4.6
pub fn claim_id(job: &JobId, attempt: u32) -> ClaimId {
    ClaimId::new(format!("claim-{job}-a{attempt}"))
}

/// Number part of a job id like `op1-j0007`.
fn job_number(id: &JobId) -> u64 {
    id.as_str()
        .rsplit("-j")
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or(0)
}

/// State shared with the claim service task.
// Spec: specs/K5-operation-service.md §4.5
struct Shared {
    reg: Mutex<ClaimRegistry>,
    clock: AtomicU64,
}

// Spec: specs/K5-operation-service.md §4.5
struct Service {
    stop: oneshot::Sender<()>,
    task: JoinHandle<ClaimServer>,
}

/// Answers claim requests from the bus (the squad's renews) against the registry, at the operation's clock.
/// Lease rule: a holder whose renew fails (denied or `Preempt`) must stop that job's work at once. S3b's squad
/// must do the same (K6 §4.8, §5), or two bots could fell one tree.
// Spec: specs/K5-operation-service.md §4.5
fn spawn_service(shared: Arc<Shared>, mut server: ClaimServer) -> Service {
    let (stop, mut stopped) = oneshot::channel::<()>();
    let task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = &mut stopped => return server,
                call = server.next() => match call {
                    None => return server,
                    Some(c) => {
                        let now = Gt(shared.clock.load(Ordering::SeqCst));
                        let reply = shared.reg.lock().unwrap().handle(now, &c.req);
                        c.reply(reply);
                    }
                },
            }
        }
    });
    Service { stop, task }
}

// Spec: specs/K5-operation-service.md §3
pub struct Operation<B: Bus> {
    cfg: OpConfig,
    bus: B,
    intents: IntentRx,
    cmds: CommandRx,
    service: Service,
    shared: Arc<Shared>,
    log: EventLog<B>,
    core: Core,
    /// Trees the operation believes in (its world shard): location and logs.
    known: BTreeMap<Loc, u32>,
    now: Gt,
}

impl<B: Bus + Clone> Operation<B> {
    /// A new operation; subscribes its subjects.
    // Spec: specs/K5-operation-service.md §3
    pub fn new(bus: &B, cfg: OpConfig, trees: Vec<(Loc, u32)>) -> Self {
        let ports = OpPorts {
            intents: bus
                .subscribe_intents(&op_intents(&cfg.server, &cfg.op))
                .expect("op intents subject is free"),
            cmds: bus.subscribe_commands(&op_cmd(&cfg.server, &cfg.op)),
            claims: bus
                .serve_claims(&op_claims(&cfg.server, &cfg.op))
                .expect("op claims subject is free"),
        };
        Self::build(bus, cfg, trees, ports, &[], Gt(0))
    }

    /// Rebuild from the log: claims, jobs, ledger and counters. Jobs in flight stay in flight (their bots hold
    /// and finish); a claim that ran out meanwhile expires on the next tick and its job is requeued.
    // Spec: specs/K5-operation-service.md §4.9
    pub fn restore(
        bus: &B,
        cfg: OpConfig,
        trees: Vec<(Loc, u32)>,
        ports: OpPorts,
        log: &[LogEvent],
        now: Gt,
    ) -> Self {
        Self::build(bus, cfg, trees, ports, log, now)
    }

    fn build(
        bus: &B,
        cfg: OpConfig,
        trees: Vec<(Loc, u32)>,
        ports: OpPorts,
        past: &[LogEvent],
        now: Gt,
    ) -> Self {
        let shared = Arc::new(Shared {
            reg: Mutex::new(ClaimRegistry::default()),
            clock: AtomicU64::new(now.0),
        });
        let mut core = Core::new();
        for (le, ev) in past.iter().zip(decode(past)) {
            match &ev {
                OpEvent::Claim(c) => shared.reg.lock().unwrap().apply(c),
                _ => core.apply(&ev, Gt(le.at.0 / 50)),
            }
        }
        let log = EventLog::new(
            bus.clone(),
            op_events(&cfg.server, &cfg.op),
            cfg.jsonl.clone(),
        )
        .expect("event log opens");
        Self {
            service: spawn_service(shared.clone(), ports.claims),
            intents: ports.intents,
            cmds: ports.cmds,
            bus: bus.clone(),
            shared,
            log,
            core,
            known: trees.into_iter().collect(),
            now,
            cfg,
        }
    }

    /// Simulate a crash: drop all state, keep only the transport handles.
    // Spec: specs/K5-operation-service.md §4.9
    pub async fn kill(self) -> OpPorts {
        let _ = self.service.stop.send(());
        let claims = self.service.task.await.expect("claim service ends cleanly");
        OpPorts {
            intents: self.intents,
            cmds: self.cmds,
            claims,
        }
    }

    /// One scheduler step at game tick `now`: read messages, expire claims, plan, dispatch, publish the summary.
    // Spec: specs/K5-operation-service.md §4.2
    pub async fn tick(&mut self, now: Gt) {
        self.now = now;
        self.shared.clock.store(now.0, Ordering::SeqCst);
        while let Some(cmd) = self.cmds.try_recv() {
            if matches!(cmd, Command::Halt { .. } | Command::Recall { .. }) {
                self.record(OpEvent::State {
                    state: OpState::Halted,
                });
            }
        }
        while let Some(i) = self.intents.try_recv() {
            match i.body {
                IntentBody::Goal { goal, .. } | IntentBody::Command(Command::SetGoal(goal)) => {
                    self.start_goal(goal).await
                }
                IntentBody::Allocation(a) if a.op == self.cfg.op => {
                    self.record(OpEvent::Allocated { bots: a.bots })
                }
                IntentBody::JobOutcome(o) => self.on_outcome(o).await,
                IntentBody::Roster { squad, lost } if squad == self.cfg.squad => {
                    let mut lost = lost;
                    lost.sort();
                    if lost.iter().cloned().collect::<BTreeSet<_>>() != self.core.lost {
                        self.record(OpEvent::Roster { lost });
                    }
                }
                _ => {}
            }
        }
        self.shared.reg.lock().unwrap().expire(now);
        self.flush_claims().await;
        if matches!(self.core.state, OpState::Running | OpState::Blocked) {
            self.check_done();
            if self.core.state != OpState::Done {
                self.top_up().await;
                self.dispatch().await;
            }
        }
        self.bus.publish_summary(
            &op_summary(&self.cfg.server, &self.cfg.op),
            SummaryBody::Operation(self.summary()),
        );
    }

    /// Record a state change: apply it, then append it to the log.
    // Spec: specs/K5-operation-service.md §4.9
    fn record(&mut self, ev: OpEvent) {
        self.core.apply(&ev, self.now);
        self.log.append(self.now, &ev);
    }

    /// Log the registry's changes in order; a job whose claim expired or was taken away is requeued.
    // Spec: specs/K5-operation-service.md §4.6
    async fn flush_claims(&mut self) {
        let evs = self.shared.reg.lock().unwrap().drain_events();
        for ev in evs {
            self.log.append(self.now, &OpEvent::Claim(ev.clone()));
            match ev {
                // Only the current attempt's claim matters; an older attempt's claim was already released.
                ClaimEvent::Expired {
                    id,
                    owner: ClaimOwner::Job(j),
                } if self.is_current_claim(&j, &id) => self.lost(&j, "claim_expired").await,
                ClaimEvent::Preempted {
                    id,
                    owner: ClaimOwner::Job(j),
                    ..
                } if self.is_current_claim(&j, &id) => self.lost(&j, "claim_preempted").await,
                _ => {}
            }
        }
    }

    fn is_current_claim(&self, j: &JobId, id: &ClaimId) -> bool {
        self.core
            .jobs
            .get(j)
            .is_some_and(|r| r.attempts > 0 && *id == claim_id(j, r.attempts))
    }

    /// A job in flight lost its claim or failed: requeue it, or give up after `max_attempts`.
    // Spec: specs/K5-operation-service.md §4.6
    async fn lost(&mut self, j: &JobId, reason: &str) {
        let Some(r) = self.core.jobs.get(j).filter(|r| r.st == JobSt::InFlight) else {
            return;
        };
        if r.attempts >= self.cfg.max_attempts {
            self.record(OpEvent::JobDead {
                job: j.clone(),
                reason: reason.into(),
            });
            self.escalate(
                format!("job_dead:{j}"),
                "job_dead",
                format!(
                    "{j} gave up after {} tries: {reason}",
                    self.cfg.max_attempts
                ),
            )
            .await;
        } else {
            self.record(OpEvent::Requeued {
                job: j.clone(),
                reason: reason.into(),
            });
        }
    }

    // Spec: specs/K5-operation-service.md §4.7
    async fn escalate(&self, key: String, kind: &str, detail: String) {
        let e = Escalation {
            key,
            from_op: self.cfg.op.clone(),
            from_squad: Some(self.cfg.squad.clone()),
            from_bot: None,
            kind: kind.into(),
            detail,
            remedies_tried: vec![],
            at: self.now,
        };
        let _ = self
            .bus
            .publish_escalation(&hq_escalate(&self.cfg.server), e)
            .await;
    }

    // Spec: specs/K5-operation-service.md §4.1
    async fn start_goal(&mut self, goal: GoalSpec) {
        if self.core.goal.is_some() {
            return;
        }
        if let Err(u) = goal.goal.check_supported() {
            return self
                .escalate(
                    format!("unsupported:{}", goal.id),
                    "unsupported_goal",
                    format!("{} is not supported yet", u.kind),
                )
                .await;
        }
        let Goal::Obtain { n, .. } = goal.goal.clone() else {
            return;
        };
        self.record(OpEvent::GoalSet { goal });
        self.record(OpEvent::State {
            state: OpState::Running,
        });
        self.expand(n).await;
    }

    /// `Obtain` becomes one `fell` job per tree, each with its own block-set claim id, until the planned
    /// logs cover `need` plus the margin. Trees come in position order (skeleton: fell only).
    // Spec: specs/K5-operation-service.md §4.1
    async fn expand(&mut self, need: u32) {
        let want = (u64::from(need) * u64::from(100 + self.cfg.margin_pct)).div_ceil(100) as u32;
        let mut planned = self.core.outstanding();
        let free: Vec<(Loc, u32)> = self
            .known
            .iter()
            .filter(|(l, _)| !self.core.used_trees.contains(*l))
            .map(|(l, n)| (l.clone(), *n))
            .collect();
        for (loc, logs) in free {
            if planned >= want {
                break;
            }
            let id = JobId::new(format!("{}-j{:04}", self.cfg.op, self.core.next_job));
            let job = self.fell_job(id, &loc, logs);
            self.record(OpEvent::JobCreated {
                job,
                tree: loc,
                logs,
            });
            planned += logs;
        }
        if planned < need {
            self.escalate(
                format!("quota_infeasible:{}", self.cfg.op),
                "quota_infeasible",
                format!("trees cover {planned} of {need} logs"),
            )
            .await;
        }
    }

    /// The box a `fell` job holds as its claim: the trunk and its canopy, with a block of room (`OpConfig::tree_box`). A real bot breaks only inside it, so the registry sees what the bot can touch.
    // Spec: specs/K5-operation-service.md §4.2
    fn tree_box(&self, tree: &Loc, logs: u32) -> Region {
        let (radius, head) = self.cfg.tree_box;
        let p = tree.pos;
        Region {
            server: tree.server.clone(),
            dim: tree.dim.clone(),
            min: Pos::new(p.x - radius, p.y, p.z - radius),
            max: Pos::new(p.x + radius, p.y + logs as i32 - 1 + head, p.z + radius),
        }
    }

    /// Parameter names for `fell`: `trees` (Locs), `quota` (Int), `tree_type` (Text), `claim` (Region, the box the
    /// job is claimed under and the only place the bot may break).
    // Spec: specs/CONVENTIONS.md §3.1
    fn fell_job(&self, id: JobId, tree: &Loc, logs: u32) -> Job {
        Job {
            claims: vec![], // set per attempt at dispatch (`claim_id`)
            kind: "fell".into(),
            params: BTreeMap::from([
                ("trees".to_string(), ParamValue::Locs(vec![tree.clone()])),
                ("quota".to_string(), ParamValue::Int(i64::from(logs))),
                ("tree_type".to_string(), ParamValue::Text("oak".into())),
                ("claim".to_string(), ParamValue::Region(self.tree_box(tree, logs))),
            ]),
            items: vec![],
            preconditions: vec![Condition::Always],
            success: Condition::BlockAbsent { at: tree.clone() },
            budget: Budget {
                ticks: self.cfg.job_budget,
                attempts: 1,
            },
            priority: self.cfg.priority,
            reflexes: vec![],
            checkpoint: Checkpoint::None,
            preemption_floor: None,
            attempt: 0,
            id,
        }
    }

    /// Keep enough planned logs for the goal: more trees when jobs were lost. Blocked when no trees are left.
    // Spec: specs/K5-operation-service.md §4.6
    async fn top_up(&mut self) {
        let Some((_, n)) = self.core.target() else {
            return;
        };
        let need = n.saturating_sub(self.core.have());
        if self.core.outstanding() < need {
            self.expand(need).await;
        }
        let want = if self.core.outstanding() < need {
            OpState::Blocked
        } else {
            OpState::Running
        };
        if self.core.state != want {
            self.record(OpEvent::State { state: want });
        }
    }

    /// One job per assigned bot goes out, each under its tree's claim. A denied claim leaves the job pending.
    // Spec: specs/K5-operation-service.md §4.2
    async fn dispatch(&mut self) {
        let cap = self.core.alive().max(1);
        let inflight = self
            .core
            .jobs
            .values()
            .filter(|r| r.st == JobSt::InFlight)
            .count();
        let pending: Vec<JobId> = self
            .core
            .jobs
            .iter()
            .filter(|(_, r)| r.st == JobSt::Pending)
            .map(|(i, _)| i.clone())
            .collect();
        let mut batch = vec![];
        let mut slots = cap.saturating_sub(inflight);
        for id in pending {
            if slots == 0 {
                break;
            }
            let mut job = self.core.jobs[&id].job.clone();
            let ParamValue::Locs(trees) = &job.params["trees"] else {
                continue;
            };
            let attempt = self.core.jobs[&id].attempts + 1;
            if attempt > 1 {
                // Never hold two claims on one tree: the previous attempt's claim goes first.
                self.release(&id).await;
            }
            job.claims = vec![claim_id(&id, attempt)];
            job.attempt = attempt;
            let req = ClaimRequest::Take {
                id: job.claims[0].clone(),
                kind: match job.params.get("claim") {
                    Some(ParamValue::Region(r)) => ClaimKind::Region(r.clone()),
                    _ => ClaimKind::BlockSet(trees.clone()),
                },
                owner: ClaimOwner::Job(id.clone()),
                priority: self.cfg.priority,
                ttl: self.cfg.claim_ttl,
            };
            let reply = self.shared.reg.lock().unwrap().handle(self.now, &req);
            let granted = matches!(reply, ClaimReply::Granted { .. });
            self.flush_claims().await;
            if granted {
                self.record(OpEvent::Dispatched {
                    job: id.clone(),
                    attempt,
                });
                batch.push(job);
                slots -= 1;
            } else {
                merge_exception(
                    &mut self.core.exceptions,
                    "claim_denied",
                    id.as_str(),
                    self.now,
                );
            }
        }
        if let Some(first) = batch.first() {
            let key = format!("batch:{}#{}", first.id, self.core.jobs[&first.id].attempts);
            let body = IntentBody::JobBatch {
                squad: self.cfg.squad.clone(),
                jobs: batch,
            };
            let _ = self
                .bus
                .publish_intent(
                    &squad_jobs(&self.cfg.server, &self.cfg.squad),
                    Intent { key, body },
                )
                .await;
        }
    }

    /// Count the outcome's items once per (job, attempt), then move the job on only if the outcome answers the
    /// job's current attempt. Items count from outcomes only (K5 §4.4). An older attempt's logs are real, so they
    /// are counted, but they change no job state and release no claim. K5 §4.6.
    // Spec: specs/K5-operation-service.md §4.4
    async fn on_outcome(&mut self, o: JobOutcome) {
        let (job, attempt) = (o.outcome.job.clone(), o.attempt);
        let known = self.core.jobs.get(&job).map(|r| r.attempts);
        let why = match known {
            None => Some("unknown job"),
            Some(n) if attempt == 0 || attempt > n => Some("attempt never dispatched"),
            _ if !self.core.bots.contains(&o.bot) => Some("bot not assigned to this operation"),
            _ => None,
        };
        if let Some(reason) = why {
            return self.record(OpEvent::Ignored {
                job,
                bot: o.bot,
                attempt,
                reason: reason.into(),
            });
        }
        let key = format!("out:{job}:a{attempt}");
        if self.core.seen_outcomes.contains(&key) {
            return;
        }
        let kind = o.outcome.kind.clone();
        self.record(OpEvent::Outcome {
            key,
            bot: o.bot,
            attempt,
            outcome: o.outcome,
        });
        let rec = &self.core.jobs[&job];
        if attempt != rec.attempts || rec.st != JobSt::InFlight {
            return;
        }
        self.release(&job).await;
        match kind {
            // The tree is gone either way: felled by this job, or by an earlier try.
            OutcomeKind::Succeeded | OutcomeKind::Failed(FailReason::TargetGone) => {
                self.record(OpEvent::JobDone { job })
            }
            _ => self.lost(&job, "outcome_not_ok").await,
        }
    }

    /// Free the claim of the job's current attempt (a no-op if it is already gone).
    // Spec: specs/K5-operation-service.md §4.5
    async fn release(&mut self, job: &JobId) {
        let id = claim_id(job, self.core.jobs[job].attempts);
        self.shared.reg.lock().unwrap().handle(
            self.now,
            &ClaimRequest::Release {
                id,
                owner: ClaimOwner::Job(job.clone()),
            },
        );
        self.flush_claims().await;
    }

    /// Goal met when the ledger reaches `n`: stop the rest and free their claims.
    // Spec: specs/K5-operation-service.md §4.4
    fn check_done(&mut self) {
        let Some((_, n)) = self.core.target() else {
            return;
        };
        if self.core.have() < n {
            return;
        }
        self.record(OpEvent::State {
            state: OpState::Done,
        });
        let open: Vec<JobId> = self
            .core
            .jobs
            .iter()
            .filter(|(_, r)| matches!(r.st, JobSt::Pending | JobSt::InFlight))
            .map(|(i, _)| i.clone())
            .collect();
        for j in open {
            let id = claim_id(&j, self.core.jobs[&j].attempts);
            self.shared.reg.lock().unwrap().handle(
                self.now,
                &ClaimRequest::Release {
                    id,
                    owner: ClaimOwner::Job(j.clone()),
                },
            );
            self.record(OpEvent::JobDead {
                job: j,
                reason: GOAL_MET.into(),
            });
        }
        let evs = self.shared.reg.lock().unwrap().drain_events();
        for ev in evs {
            self.log.append(self.now, &OpEvent::Claim(ev));
        }
    }

    /// The fixed-size summary for HQ (K5 §3).
    // Spec: specs/K5-operation-service.md §3
    pub fn summary(&self) -> OpSummary {
        let c = &self.core;
        let (target, done) = c.target().map_or((0, 0), |(_, n)| (n, c.have().min(n)));
        let top = c
            .exceptions
            .iter()
            .max_by_key(|e| (e.count, e.last_seen))
            .map(|e| format!("{} {} x{}", e.kind, e.subject, e.count));
        let health = match c.state {
            OpState::Failed => Health::Down,
            OpState::Blocked => Health::Degraded,
            _ if c.exceptions.is_empty() => Health::Ok,
            _ => Health::Degraded,
        };
        let bots = c.bots.len() as u32;
        OpSummary {
            op: self.cfg.op.clone(),
            server: self.cfg.server.clone(),
            goal: c
                .goal
                .as_ref()
                .map_or_else(|| GoalId::new("none"), |g| g.id.clone()),
            state: c.state,
            headcount: Headcount {
                assigned: bots,
                alive: c.alive() as u32,
            },
            progress: Progress { done, target },
            stock: c
                .ledger
                .iter()
                .map(|(item, n)| ItemCount {
                    item: item.clone(),
                    n: *n,
                })
                .collect(),
            health,
            top_issue: top,
            exceptions: c.exceptions.clone(),
            at: self.now,
        }
    }

    pub fn state(&self) -> OpState {
        self.core.state
    }
    pub fn ledger(&self, item: &ItemId) -> u32 {
        self.core.ledger.get(item).copied().unwrap_or(0)
    }
    pub fn job_states(&self) -> Vec<(JobId, JobSt, u32)> {
        self.core
            .jobs
            .iter()
            .map(|(i, r)| (i.clone(), r.st, r.attempts))
            .collect()
    }
    pub fn claims(&self) -> Vec<(ClaimId, crate::Entry)> {
        self.shared.reg.lock().unwrap().snapshot()
    }
    pub fn event_log(&self) -> Vec<LogEvent> {
        self.log.read_all()
    }
    /// Comparable picture of the replayable state (restart tests).
    pub fn fingerprint(&self) -> String {
        format!(
            "{:?}|{:?}|{:?}|{:?}",
            self.core.ledger,
            self.job_states(),
            self.claims(),
            self.core.state
        )
    }
    pub fn wall(&self) -> WallMs {
        wall_of(self.now)
    }
}
