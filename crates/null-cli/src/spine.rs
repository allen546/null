//! The spine in one process: HQ, operations, squads and simulated bots on an in-process bus (S3).
//! `step` is one game tick in a fixed order, so a seeded run replays exactly.
use crate::audit::Audit;
use crate::parse::{HELP, Line, parse_line, render};
use null_bus::{Bus, InProcBus};
use null_hq::{Hq, HqConfig, OpPlan, Reply};
use null_op::{OpConfig, OpPorts, Operation};
use null_proto::*;
use null_simbot::{SimBot, SimWorld};
use null_squad::{BotPorts, Squad, SquadConfig};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

// Spec: specs/F3-test-harness.md §4.5
#[derive(Clone, Debug)]
pub struct SpineConfig {
    pub seed: u64,
    pub bots: usize,
    pub trees: usize,
    pub extent: i32,
    pub server: String,
    /// Ticks an operation stays after Done before HQ retires it (K1 §4.2).
    pub retire_after: Gt,
    /// Each operation mirrors its event log to `<dir>/<op>.jsonl` (live runs keep it for the audit of claims).
    pub jsonl_dir: Option<PathBuf>,
    /// Claim box of a `fell` job: blocks out from the trunk, blocks above the top log. The toy trees stand on
    /// neighbouring cells, so the simulation claims the trunk's column only; real trees need `null_op::TREE_BOX_*`.
    pub tree_box: (i32, i32),
    /// Ticks a `fell` job may run before the bot gives up on it.
    pub job_budget: Gt,
}

impl SpineConfig {
    pub fn new(seed: u64, bots: usize) -> Self {
        Self { seed, bots, trees: 60, extent: 40, server: "smp1".into(), retire_after: Gt(40), jsonl_dir: None, tree_box: (0, 0), job_budget: Gt(600) }
    }
}

/// The bots of a spine: simulated ones, or real ones behind `null-botlink` (S6). A bot ticks once per game tick,
/// after HQ, the operations and the squads, and speaks only S2 messages on the bus.
// Spec: specs/F3-test-harness.md §4.5
pub trait SpineBot {
    fn tick(&mut self, now: Gt) -> impl std::future::Future<Output = ()>;
}

impl SpineBot for SimBot<InProcBus> {
    async fn tick(&mut self, now: Gt) {
        SimBot::tick(self, now).await
    }
}

/// What an operation believes about the trees: (location, logs). The toy world answers with the trees left.
pub type TreeSource = Box<dyn Fn() -> Vec<(Loc, u32)> + Send>;

// Spec: specs/F3-test-harness.md §4.5
pub struct Spine<Bt: SpineBot = SimBot<InProcBus>> {
    pub bus: InProcBus,
    trees: TreeSource,
    cfg: SpineConfig,
    pub hq: Hq<InProcBus>,
    pub ops: BTreeMap<OperationId, Operation<InProcBus>>,
    pub squads: BTreeMap<OperationId, Squad<InProcBus>>,
    pub bots: Vec<Bt>,
    pub audit: Audit,
    pub now: u64,
    server: ServerId,
    op_cfgs: BTreeMap<OperationId, (OpConfig, Vec<(Loc, u32)>)>,
    killed: BTreeMap<OperationId, OpPorts>,
    ports: BotPorts,
    /// Retiring operations: squad and operation keep ticking until the halted bots' last outcomes are counted (H3).
    pub retiring: BTreeMap<OperationId, (Squad<InProcBus>, Operation<InProcBus>)>,
    /// Plans that wait for the retiring squads to hand their bot receivers on.
    waiting: Vec<OpPlan>,
    /// Exceptions of retired squads, e.g. a halted bot that never reported (H3).
    pub retired_exceptions: Vec<Exception>,
    /// Trees already given to an operation. Each operation has its own claims registry, so two must never share a tree.
    given: BTreeSet<Loc>,
    goals: u32,
    launched: u32,
}

impl Spine {
    /// The simulated spine: toy world, fake bots.
    // Spec: specs/F3-test-harness.md §4.5
    pub fn new(cfg: &SpineConfig) -> Self {
        let server = ServerId::new(&cfg.server);
        let bus = InProcBus::default();
        let world = SimWorld::generate(cfg.seed, server.clone(), cfg.trees, cfg.extent).shared();
        let names: Vec<BotId> = (1..=cfg.bots).map(|i| BotId::new(format!("bot{i}"))).collect();
        let bots = names.iter().map(|b| SimBot::new(&bus, &world, b.as_str(), cfg.seed)).collect();
        let source: TreeSource = Box::new(move || world.lock().unwrap().tree_list());
        Spine::with_bots(cfg, bus, names, bots, source)
    }
}

impl<Bt: SpineBot> Spine<Bt> {
    /// A spine over bots the caller made on `bus` (one per name, subscribed already) and the trees the operations may use.
    // Spec: specs/F3-test-harness.md §4.5
    pub fn with_bots(cfg: &SpineConfig, bus: InProcBus, names: Vec<BotId>, bots: Vec<Bt>, trees: TreeSource) -> Self {
        let server = ServerId::new(&cfg.server);
        let mut hq_cfg = HqConfig::new(&cfg.server, names);
        hq_cfg.retire_after = cfg.retire_after;
        let hq = Hq::new(&bus, hq_cfg);
        Self {
            bus,
            trees,
            cfg: cfg.clone(),
            hq,
            ops: BTreeMap::new(),
            squads: BTreeMap::new(),
            bots,
            audit: Audit::default(),
            now: 0,
            server,
            op_cfgs: BTreeMap::new(),
            killed: BTreeMap::new(),
            ports: BotPorts::default(),
            retiring: BTreeMap::new(),
            waiting: vec![],
            retired_exceptions: vec![],
            given: BTreeSet::new(),
            goals: 0,
            launched: 0,
        }
    }

    /// One game tick: HQ, then each operation, squad and bot.
    // Spec: specs/F3-test-harness.md §4.5
    pub async fn step(&mut self) {
        self.now += 1;
        let now = Gt(self.now);
        self.hq.tick(now).await;
        // Retire first: a finished squad hands its bot receivers to the squad that replaces it.
        for op in self.hq.take_retired() {
            self.retire(&op).await;
        }
        self.waiting.extend(self.hq.take_launches());
        // A retiring squad that was done at the start of this step gets one more operation tick, then ends.
        let finished: Vec<OperationId> = self.retiring.iter().filter(|(_, (s, _))| s.retire_done()).map(|(o, _)| o.clone()).collect();
        if self.retiring.is_empty() {
            for plan in std::mem::take(&mut self.waiting) {
                self.launch(plan);
            }
        }
        for op in self.ops.values_mut().chain(self.retiring.values_mut().map(|(_, o)| o)) {
            op.tick(now).await;
        }
        for sq in self.squads.values_mut().chain(self.retiring.values_mut().map(|(s, _)| s)) {
            sq.tick(now).await;
        }
        for b in &mut self.bots {
            b.tick(now).await;
        }
        for op in finished {
            let (sq, o) = self.retiring.remove(&op).expect("listed above");
            let (ports, exceptions) = sq.finish_retire();
            self.ports.absorb(ports);
            self.retired_exceptions.extend(exceptions);
            o.kill().await;
        }
    }

    // Spec: specs/F3-test-harness.md §4.5
    /// The trees the operations may still use, as the source gives them.
    pub fn trees(&self) -> Vec<(Loc, u32)> {
        (self.trees)()
    }

    pub async fn run(&mut self, ticks: u64) {
        for _ in 0..ticks {
            self.step().await;
        }
    }

    /// Run until every operation is retired, failed or halted (or `max` ticks). True when everything finished.
    // Spec: specs/F3-test-harness.md §4.5
    pub async fn run_until_idle(&mut self, max: u64) -> bool {
        for _ in 0..max {
            self.step().await;
            if self.finished() {
                return true;
            }
        }
        false
    }

    // Spec: specs/F3-test-harness.md §4.5
    pub fn finished(&self) -> bool {
        self.launched > 0
            && self.hq.queued() == 0
            && self.retiring.is_empty()
            && self.waiting.is_empty()
            && self.ops.values().all(|o| matches!(o.state(), OpState::Failed | OpState::Halted))
    }

    /// Start the operation and squad HQ asked for, on trees no other operation has.
    // Spec: specs/K1-hq.md §4.1
    fn launch(&mut self, plan: OpPlan) {
        let want = match plan.goal.goal {
            Goal::Obtain { n, .. } => n * 3,
            _ => 0,
        };
        let mut trees = vec![];
        let mut logs = 0;
        for (loc, n) in (self.trees)() {
            if logs >= want {
                break;
            }
            if self.given.insert(loc.clone()) {
                logs += n;
                trees.push((loc, n));
            }
        }
        let mut cfg = OpConfig::new(plan.op.as_str(), self.server.as_str(), plan.squad.as_str());
        cfg.priority = plan.goal.priority;
        cfg.tree_box = self.cfg.tree_box;
        cfg.job_budget = self.cfg.job_budget;
        cfg.jsonl = self.cfg.jsonl_dir.as_ref().map(|d| d.join(format!("{}.jsonl", plan.op)));
        let op = Operation::new(&self.bus, cfg.clone(), trees.clone());
        let names: Vec<&str> = plan.bots.iter().map(BotId::as_str).collect();
        let sq = Squad::with_ports(
            &self.bus,
            SquadConfig::new(self.server.as_str(), plan.squad.as_str(), plan.op.as_str(), &names),
            &mut self.ports,
        );
        self.launched += 1;
        self.op_cfgs.insert(plan.op.clone(), (cfg, trees));
        self.ops.insert(plan.op.clone(), op);
        self.squads.insert(plan.op, sq);
    }

    /// A finished operation stops: its squad halts its bots, then squad and operation keep ticking until the
    /// bots' last outcomes are counted (or a bound passes), and only then hand the bot receivers on (H3).
    // Spec: specs/K1-hq.md §4.2
    pub async fn retire(&mut self, op: &OperationId) {
        if let (Some(mut sq), Some(o)) = (self.squads.remove(op), self.ops.remove(op)) {
            sq.begin_retire().await;
            self.retiring.insert(op.clone(), (sq, o));
        }
    }

    /// Simulate an operation crash: only the transport handles survive (K5 §4.9).
    // Spec: specs/K5-operation-service.md §4.9
    pub async fn kill_op(&mut self, op: &OperationId) {
        let ports = self.ops.remove(op).expect("operation exists").kill().await;
        self.killed.insert(op.clone(), ports);
    }

    /// Restart a killed operation from its log; its squad then sends its kept outcomes again.
    // Spec: specs/K5-operation-service.md §4.9
    pub async fn restart_op(&mut self, op: &OperationId) {
        let ports = self.killed.remove(op).expect("operation was killed");
        let (cfg, trees) = self.op_cfgs[op].clone();
        let log: Vec<LogEvent> = self
            .bus
            .read_events(&null_op::op_events(&self.server, op), 0)
            .into_iter()
            .map(|(_, e)| e)
            .collect();
        let o = Operation::restore(&self.bus, cfg, trees, ports, &log, Gt(self.now));
        self.ops.insert(op.clone(), o);
        if let Some(sq) = self.squads.get_mut(op) {
            sq.resend_outcomes().await;
        }
    }

    /// A K2 command with its audit entry. Writes need a reason (K2 §4.3).
    // Spec: specs/K2-commander-api.md §4.3
    pub async fn command(&mut self, principal: &str, reason: &str, cmd: Command) -> Reply {
        let write = !matches!(cmd, Command::Status | Command::Inspect { .. });
        let reply = if write && reason.trim().is_empty() {
            Reply::Rejected { why: "a reason is required for every write".into() }
        } else {
            self.hq.handle(cmd.clone()).await
        };
        self.audit.record(Gt(self.now), principal, &cmd, reason, &reply);
        reply
    }

    /// Execute one line of CLI input and return the text to print.
    // Spec: specs/K4-human-interface.md §4.1
    pub async fn exec(&mut self, line: &str) -> String {
        let parsed = parse_line(line, self.goals + 1);
        match parsed {
            Err(e) => format!("error: {e}"),
            Ok(Line::Help) => HELP.to_string(),
            Ok(Line::Run(n)) => {
                self.run(n).await;
                format!("ran {n} ticks, now tick {}", self.now)
            }
            Ok(Line::RunDone(max)) => {
                let done = self.run_until_idle(max).await;
                format!("{} at tick {}", if done { "idle" } else { "still running" }, self.now)
            }
            Ok(Line::Command { cmd, reason }) => {
                let is_goal = matches!(cmd, Command::SetGoal(_));
                let reply = self.command("cli", &reason, cmd).await;
                if is_goal && matches!(reply, Reply::Accepted { .. }) {
                    self.goals += 1;
                }
                render(&reply)
            }
        }
    }
}
