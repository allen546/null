//! The HQ state machine. Commands come in through `handle`; `tick` reads summaries and escalations.
use null_bus::{Bus, EscalationRx, Level, SummaryRx, Subject, kind};
use null_op::{hq_escalate, op_cmd, op_intents, op_summary};
use null_proto::*;
use null_squad::squad_cmd;
use serde::Serialize;
use std::collections::BTreeMap;

// Spec: specs/F6-bus-deployment.md §4.2
/// Where HQ publishes the digest.
pub fn hq_digest(s: &ServerId) -> Subject {
    Subject::new(s, Level::Hq, None, kind::DIGEST)
}

// Spec: specs/K1-hq.md §3
#[derive(Clone, Debug)]
pub struct HqConfig {
    pub server: ServerId,
    /// Every bot HQ may allocate.
    pub pool: Vec<BotId>,
    /// Game ticks between digests.
    pub digest_every: Gt,
    /// An operation whose summary is older than this is marked stale (K1 §5).
    pub stale_after: Gt,
    /// Ticks an operation stays after it reports Done before its bots go back to the pool: bots still working
    /// for it lose their claims, halt and report (K1 §4.2 dwell).
    pub retire_after: Gt,
    /// Escalations kept in the inbox; the oldest is dropped past this (K1 §5).
    pub inbox_cap: usize,
}

impl HqConfig {
    pub fn new(server: &str, pool: Vec<BotId>) -> Self {
        Self { server: ServerId::new(server), pool, digest_every: Gt(20), stale_after: Gt(200), retire_after: Gt(40), inbox_cap: 8 }
    }
}

/// What the host must start for a new goal: an operation and its squad, with these bots.
// Spec: specs/K1-hq.md §4.1
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpPlan {
    pub op: OperationId,
    pub squad: SquadId,
    pub goal: GoalSpec,
    pub bots: Vec<BotId>,
}

// Spec: specs/K2-commander-api.md §4.1
#[derive(Clone, Debug, Serialize)]
pub struct InspectReport {
    pub goal: Option<GoalSpec>,
    pub op: Option<OperationId>,
    pub summary: Option<OpSummary>,
    pub bots: Vec<BotId>,
    pub escalations: Vec<Escalation>,
}

/// What a command returns: the effect in concrete terms (K2 §4.1), or why it was refused (K2 §5).
// Spec: specs/K2-commander-api.md §4.1
#[derive(Clone, Debug, Serialize)]
pub enum Reply {
    Accepted { effect: String },
    Rejected { why: String },
    Status(Digest),
    Inspect(InspectReport),
}

// Spec: specs/K1-hq.md §4.1
struct GoalRec {
    spec: GoalSpec,
    op: Option<OperationId>,
}

// Spec: specs/K1-hq.md §4.2
struct OpRec {
    squad: SquadId,
    goal: GoalId,
    bots: Vec<BotId>,
    summary: SummaryRx,
    /// Done: its bots went back to the pool.
    retired: bool,
    /// When HQ first saw the operation Done.
    done_at: Option<Gt>,
    created: Gt,
}

// Spec: specs/K1-hq.md §3
pub struct Hq<B: Bus + Clone> {
    cfg: HqConfig,
    bus: B,
    esc: EscalationRx,
    goals: Vec<GoalRec>,
    ops: BTreeMap<OperationId, OpRec>,
    free: Vec<BotId>,
    launches: Vec<OpPlan>,
    retired: Vec<OperationId>,
    inbox: Vec<Escalation>,
    next_op: u32,
    generation: u64,
    last_digest: Option<Gt>,
    now: Gt,
}

impl<B: Bus + Clone> Hq<B> {
    // Spec: specs/K1-hq.md §3
    pub fn new(bus: &B, cfg: HqConfig) -> Self {
        Self {
            esc: bus.subscribe_escalations(&hq_escalate(&cfg.server)).expect("hq escalate subject is free"),
            free: cfg.pool.clone(),
            bus: bus.clone(),
            goals: vec![],
            ops: BTreeMap::new(),
            launches: vec![],
            retired: vec![],
            inbox: vec![],
            next_op: 1,
            generation: 0,
            last_digest: None,
            now: Gt(0),
            cfg,
        }
    }

    /// Operations the host must now create (drained).
    pub fn take_launches(&mut self) -> Vec<OpPlan> {
        std::mem::take(&mut self.launches)
    }
    /// Operations the host may now drop (drained).
    pub fn take_retired(&mut self) -> Vec<OperationId> {
        std::mem::take(&mut self.retired)
    }
    /// Goals still waiting for bots.
    pub fn queued(&self) -> usize {
        self.goals.iter().filter(|g| g.op.is_none()).count()
    }
    pub fn free_bots(&self) -> usize {
        self.free.len()
    }
    pub fn inbox(&self) -> &[Escalation] {
        &self.inbox
    }

    /// One HQ step: read escalations and summaries, retire finished operations, start queued goals, publish
    /// the digest on its period.
    // Spec: specs/K1-hq.md §4.2
    pub async fn tick(&mut self, now: Gt) {
        self.now = now;
        while let Some(e) = self.esc.try_recv() {
            self.inbox.retain(|o| o.key != e.key);
            self.inbox.push(e);
            if self.inbox.len() > self.cfg.inbox_cap {
                self.inbox.remove(0);
            }
        }
        let now = self.now;
        let after = self.cfg.retire_after;
        let mut done = vec![];
        for (o, r) in self.ops.iter_mut().filter(|(_, r)| !r.retired) {
            if matches!(r.summary.latest(), Some(SummaryBody::Operation(s)) if s.state == OpState::Done) {
                let at = *r.done_at.get_or_insert(now);
                if now - at >= after {
                    r.retired = true;
                    self.free.extend(r.bots.iter().cloned());
                    done.push(o.clone());
                }
            }
        }
        self.retired.extend(done);
        self.launch_queued().await;
        if self.last_digest.is_none_or(|t| now - t >= self.cfg.digest_every) {
            self.publish_digest();
        }
    }

    /// A command from the commander (K2). Halt and recall go straight onto the command class, ahead of anything else.
    // Spec: specs/K2-commander-api.md §4.1
    pub async fn handle(&mut self, cmd: Command) -> Reply {
        match cmd {
            Command::SetGoal(spec) => self.set_goal(spec).await,
            Command::Status => Reply::Status(self.digest()),
            Command::Inspect { target } => self.inspect(target),
            Command::Halt { scope } => self.stop(scope.clone(), Command::Halt { scope }),
            Command::Recall { scope } => self.stop(scope.clone(), Command::Recall { scope }),
            other => Reply::Rejected { why: format!("{other:?} is not in the skeleton command set") },
        }
    }

    // Spec: specs/K2-commander-api.md §4.2
    async fn set_goal(&mut self, spec: GoalSpec) -> Reply {
        if spec.reason.trim().is_empty() {
            return Reply::Rejected { why: "a reason is required for every write".into() };
        }
        if let Err(u) = spec.goal.check_supported() {
            return Reply::Rejected { why: format!("{} goals are not supported yet", u.kind) };
        }
        if let Goal::Obtain { n: 0, .. } = spec.goal {
            return Reply::Rejected { why: "quantity must be at least 1".into() };
        }
        if let Some(g) = self.goals.iter().find(|g| g.spec.id == spec.id) {
            // Re-sending the same write is harmless (K2 §4.2).
            return if g.spec == spec {
                Reply::Accepted { effect: format!("goal {} was already set", spec.id) }
            } else {
                Reply::Rejected { why: format!("goal id {} is taken by a different goal", spec.id) }
            };
        }
        let id = spec.id.clone();
        self.goals.push(GoalRec { spec, op: None });
        self.launch_queued().await;
        let g = self.goals.iter().find(|g| g.spec.id == id).expect("just added");
        let effect = match g.op.as_ref().and_then(|o| self.ops.get(o).map(|r| (o, r))) {
            Some((o, r)) => format!("goal {id} accepted: operation {o}, squad {}, {} bots", r.squad, r.bots.len()),
            None => format!("goal {id} accepted: queued until bots are free"),
        };
        Reply::Accepted { effect }
    }

    /// Queued goals start by priority, then age. The free bots are shared between the goals waiting now.
    // Spec: specs/K1-hq.md §4.2
    async fn launch_queued(&mut self) {
        loop {
            let mut waiting: Vec<usize> = (0..self.goals.len()).filter(|&i| self.goals[i].op.is_none()).collect();
            if waiting.is_empty() || self.free.is_empty() {
                return;
            }
            waiting.sort_by_key(|&i| (std::cmp::Reverse(self.goals[i].spec.priority), i));
            let share = self.free.len().div_ceil(waiting.len());
            let bots: Vec<BotId> = self.free.drain(..share).collect();
            let gi = waiting[0];
            let n = self.next_op;
            self.next_op += 1;
            let (op, squad) = (OperationId::new(format!("op{n}")), SquadId::new(format!("sq{n}")));
            let spec = self.goals[gi].spec.clone();
            self.goals[gi].op = Some(op.clone());
            let subj = op_intents(&self.cfg.server, &op);
            let alloc = Intent { key: format!("alloc:{op}"), body: IntentBody::Allocation(Allocation { op: op.clone(), bots: bots.clone() }) };
            let goal = Intent {
                key: format!("goal:{op}"),
                body: IntentBody::Goal { op: op.clone(), region: None, goal: spec.clone(), quotas: vec![] },
            };
            let _ = self.bus.publish_intent(&subj, alloc).await;
            let _ = self.bus.publish_intent(&subj, goal).await;
            self.ops.insert(
                op.clone(),
                OpRec {
                    squad: squad.clone(),
                    goal: spec.id.clone(),
                    bots: bots.clone(),
                    summary: self.bus.subscribe_summary(&op_summary(&self.cfg.server, &op)),
                    retired: false,
                    done_at: None,
                    created: self.now,
                },
            );
            self.launches.push(OpPlan { op, squad, goal: spec, bots });
        }
    }

    /// Halt or recall: to each covered operation's command subject and its squad's. Not queued, not gated.
    // Spec: specs/K2-commander-api.md §4.9
    fn stop(&mut self, scope: Scope, cmd: Command) -> Reply {
        let mut hit = 0;
        for (op, r) in self.ops.iter().filter(|(_, r)| !r.retired) {
            let covered = match &scope {
                Scope::Swarm => true,
                Scope::Server(s) => *s == self.cfg.server,
                Scope::Operation(o) => o == op,
                Scope::Squad(s) => *s == r.squad,
                Scope::Bot(b) => r.bots.contains(b),
            };
            if !covered {
                continue;
            }
            hit += 1;
            let _ = self.bus.send_command(&squad_cmd(&self.cfg.server, &r.squad), cmd.clone());
            // A bot-scope stop is the squad's alone; the operation keeps planning.
            if !matches!(scope, Scope::Bot(_)) {
                let _ = self.bus.send_command(&op_cmd(&self.cfg.server, op), cmd.clone());
            }
        }
        let verb = if matches!(cmd, Command::Halt { .. }) { "halt" } else { "recall" };
        if hit == 0 {
            Reply::Rejected { why: format!("{verb}: no running operation in scope {scope:?}") }
        } else {
            Reply::Accepted { effect: format!("{verb} sent to {hit} operation(s) in scope {scope:?}") }
        }
    }

    // Spec: specs/K2-commander-api.md §3
    fn inspect(&mut self, target: InspectTarget) -> Reply {
        if target == InspectTarget::Scope(Scope::Swarm) {
            return Reply::Status(self.digest());
        }
        let op = match &target {
            InspectTarget::Goal(g) => self.goals.iter().find(|r| r.spec.id == *g).map(|r| (r.op.clone(), Some(r.spec.clone()))),
            InspectTarget::Scope(Scope::Operation(o)) => {
                Some((self.ops.contains_key(o).then(|| o.clone()), self.goal_of(o)))
            }
            InspectTarget::Scope(Scope::Squad(s)) => {
                let o = self.ops.iter().find(|(_, r)| r.squad == *s).map(|(o, _)| o.clone());
                Some((o.clone(), o.and_then(|o| self.goal_of(&o))))
            }
            InspectTarget::Scope(_) => None,
        };
        let Some((op, goal)) = op else {
            return Reply::Rejected { why: format!("cannot inspect {target:?}") };
        };
        let Some(rec) = op.as_ref().and_then(|o| self.ops.get(o)) else {
            return Reply::Rejected { why: format!("{target:?} has no operation") };
        };
        let summary = match rec.summary.latest() {
            Some(SummaryBody::Operation(s)) => Some(s),
            _ => None,
        };
        Reply::Inspect(InspectReport { goal, op, summary, bots: rec.bots.clone(), escalations: self.inbox.clone() })
    }

    fn goal_of(&self, op: &OperationId) -> Option<GoalSpec> {
        let g = &self.ops.get(op)?.goal;
        self.goals.iter().find(|r| r.spec.id == *g).map(|r| r.spec.clone())
    }

    /// One row per operation. Its size depends on the operation count, never on bots (K1 §4.8).
    // Spec: specs/K1-hq.md §4.8
    pub fn digest(&mut self) -> Digest {
        self.generation += 1;
        let rows = self
            .ops
            .iter()
            .map(|(op, r)| {
                let s = match r.summary.latest() {
                    Some(SummaryBody::Operation(s)) => Some(s),
                    _ => None,
                };
                let seen = s.as_ref().map_or(r.created, |s| s.at);
                let stale = !r.retired && self.now - seen > self.cfg.stale_after;
                match s {
                    Some(s) => DigestRow {
                        op: op.clone(),
                        server: s.server,
                        goal: s.goal,
                        state: s.state,
                        headcount: s.headcount,
                        progress: s.progress,
                        top_issue: s.top_issue,
                        stale,
                    },
                    None => DigestRow {
                        op: op.clone(),
                        server: self.cfg.server.clone(),
                        goal: r.goal.clone(),
                        state: OpState::Planning,
                        headcount: Headcount { assigned: r.bots.len() as u32, alive: r.bots.len() as u32 },
                        progress: Progress { done: 0, target: 0 },
                        top_issue: None,
                        stale,
                    },
                }
            })
            .collect();
        let mut warnings = vec![];
        if let Some(last) = self.inbox.last() {
            warnings.push(format!("{} escalation(s) open, latest {} from {}", self.inbox.len(), last.kind, last.from_op));
        }
        Digest { generation: self.generation, at: self.now, rows, warnings }
    }

    // Spec: specs/K1-hq.md §4.8
    fn publish_digest(&mut self) {
        let d = self.digest();
        self.bus.publish_summary(&hq_digest(&self.cfg.server), SummaryBody::Digest(d));
        self.last_digest = Some(self.now);
    }
}
