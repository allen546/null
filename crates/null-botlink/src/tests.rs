//! The bot side of the protocol against a fake host. Spec: specs/K6-squad-leader.md §3, §4.8
use crate::*;
use null_bus::{Bus, InProcBus, bot_cmd, bot_jobs, bot_summary, bot_up};
use null_proto::*;
use std::collections::BTreeMap;

fn srv() -> ServerId {
    ServerId::new("smp1")
}

/// A host whose runs end when the test says so.
#[derive(Default)]
struct FakeHost {
    life: Option<Life>,
    started: Vec<FellSpec>,
    handles: Vec<RunHandle>,
    /// Hard stops the link asked for, each with the lease-relative moment unknown to the host: only the count matters.
    halts: u32,
}

impl Host for FakeHost {
    fn start(&mut self, spec: FellSpec) -> RunHandle {
        self.started.push(spec);
        self.handles.push(RunHandle::new());
        self.handles.last().unwrap().clone()
    }
    fn status(&mut self) -> HostStatus {
        HostStatus { life: self.life.unwrap_or(Life::Alive), pos: Some(Pos::new(1, 64, 2)), health: 20.0 }
    }
    fn halt_actions(&mut self) {
        self.halts += 1;
    }
}

fn end(status: RunStatus, logs: u32) -> RunEnd {
    RunEnd { status, items: BTreeMap::from([("minecraft:oak_log".to_string(), logs)]), gathered: 1, left: 0, drops_left: 0 }
}

fn region() -> Region {
    Region { server: srv(), dim: Dimension::Overworld, min: Pos::new(7, 64, 7), max: Pos::new(13, 72, 13) }
}

fn fell_job(id: &str) -> Job {
    Job {
        id: JobId::new(id),
        kind: "fell".into(),
        params: BTreeMap::from([
            ("trees".to_string(), ParamValue::Locs(vec![])),
            ("quota".to_string(), ParamValue::Int(6)),
            ("tree_type".to_string(), ParamValue::Text("oak".into())),
            ("claim".to_string(), ParamValue::Region(region())),
        ]),
        items: vec![],
        claims: vec![],
        preconditions: vec![Condition::Always],
        success: Condition::Always,
        budget: Budget { ticks: Gt(500), attempts: 1 },
        priority: Priority(1),
        reflexes: vec![],
        checkpoint: Checkpoint::None,
        preemption_floor: None,
        attempt: 1,
    }
}

async fn put(bus: &InProcBus, key: String, body: IntentBody) {
    bus.publish_intent(&bot_jobs(&srv(), &BotId::new("b1")), Intent { key, body }).await.unwrap();
}

async fn lease(bus: &InProcBus, job: &Job, expires: u64) {
    let l = Lease { bot: BotId::new("b1"), job: job.id.clone(), attempt: job.attempt, claim: ClaimId::new("c"), expires_at: Gt(expires) };
    put(bus, format!("l:{}:{expires}", job.id), IntentBody::Lease(l)).await;
}

async fn batch(bus: &InProcBus, job: &Job) {
    put(bus, format!("j:{}", job.id), IntentBody::JobBatch { squad: SquadId::new("s"), jobs: vec![job.clone()] }).await;
}

fn halt(bus: &InProcBus) {
    bus.send_command(&bot_cmd(&srv(), &BotId::new("b1")), Command::Halt { scope: Scope::Bot(BotId::new("b1")) }).unwrap();
}

struct Rig {
    bus: InProcBus,
    link: BotLink<InProcBus>,
    host: FakeHost,
    up: null_bus::IntentRx,
}

fn rig() -> Rig {
    let bus = InProcBus::default();
    let link = BotLink::new(&bus, &srv(), "b1");
    let up = bus.subscribe_intents(&bot_up(&srv(), &BotId::new("b1"))).unwrap();
    Rig { bus, link, host: FakeHost::default(), up }
}

impl Rig {
    async fn tick(&mut self, t: u64) {
        self.link.tick(Gt(t), &mut self.host).await;
    }
    fn outcome(&mut self) -> Option<JobOutcome> {
        match self.up.try_recv()?.body {
            IntentBody::JobOutcome(o) => Some(o),
            b => panic!("{b:?}"),
        }
    }
}

#[tokio::test]
async fn a_job_with_a_lease_starts_in_its_claim_box_and_reports_its_logs() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 1000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    assert_eq!(r.host.started, vec![FellSpec { tree_type: "oak".into(), quota: 6, min: region().min, max: region().max }]);
    assert!(r.outcome().is_none());
    r.host.handles[0].finish(end(RunStatus::Done, 6));
    r.tick(2).await;
    let o = r.outcome().expect("outcome");
    assert_eq!((o.attempt, o.outcome.kind), (1, OutcomeKind::Succeeded));
    assert_eq!(o.outcome.report.items_gained, vec![ItemCount { item: ItemId::new("minecraft:oak_log"), n: 6 }]);
    assert!(r.link.is_idle());
}

#[tokio::test]
async fn a_job_without_a_lease_is_refused_and_never_starts() {
    let mut r = rig();
    let job = fell_job("j1");
    batch(&r.bus, &job).await;
    r.tick(1).await;
    assert!(r.host.started.is_empty(), "no lease, no work");
    assert_eq!(r.outcome().expect("outcome").outcome.kind, OutcomeKind::Preempted);
    // A lease for another attempt does not cover this one.
    let mut other = fell_job("j2");
    other.attempt = 2;
    let mut first = other.clone();
    first.attempt = 1;
    lease(&r.bus, &first, 1000).await;
    batch(&r.bus, &other).await;
    r.tick(2).await;
    assert!(r.host.started.is_empty());
    assert_eq!(r.outcome().expect("outcome").outcome.kind, OutcomeKind::Preempted);
}

#[tokio::test]
async fn the_bot_stops_ten_ticks_before_its_lease_ends_and_reports_what_it_took() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 100).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    for t in 2..90 {
        r.tick(t).await;
    }
    assert!(!r.host.handles[0].stop_requested(), "tick 89 is the last with 10 ticks to spare");
    r.tick(90).await;
    assert!(r.host.handles[0].stop_requested(), "now + 10 reaches the lease end");
    assert!(r.outcome().is_none(), "the outcome waits for the skill to stop");
    r.host.handles[0].finish(end(RunStatus::Stopped, 3));
    r.tick(91).await;
    let o = r.outcome().expect("outcome");
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert_eq!(o.outcome.report.items_gained[0].n, 3, "the logs it took are counted");
}

#[tokio::test]
async fn the_last_lease_wins_even_when_shorter() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 1000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    lease(&r.bus, &job, 30).await;
    r.tick(19).await;
    assert!(!r.host.handles[0].stop_requested());
    r.tick(20).await;
    assert!(r.host.handles[0].stop_requested());
}

#[tokio::test]
async fn a_halt_stops_the_job_as_preempted_and_drops_jobs_read_in_the_same_tick() {
    let mut r = rig();
    let (a, b) = (fell_job("j1"), fell_job("j2"));
    lease(&r.bus, &a, 1000).await;
    batch(&r.bus, &a).await;
    r.tick(1).await;
    halt(&r.bus);
    lease(&r.bus, &b, 1000).await;
    batch(&r.bus, &b).await;
    r.tick(2).await;
    assert!(r.host.handles[0].stop_requested());
    r.host.handles[0].finish(end(RunStatus::Stopped, 2));
    r.tick(3).await;
    assert_eq!(r.outcome().expect("outcome").outcome.kind, OutcomeKind::Preempted);
    assert_eq!(r.host.started.len(), 1, "the job behind the halt never starts");
    assert!(r.link.is_idle());
}

#[tokio::test]
async fn a_job_over_budget_stops_and_fails_with_budget_exhausted() {
    let mut r = rig();
    let mut job = fell_job("j1");
    job.budget.ticks = Gt(50);
    lease(&r.bus, &job, 10_000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    r.tick(52).await;
    assert!(r.host.handles[0].stop_requested());
    r.host.handles[0].finish(end(RunStatus::Stopped, 1));
    r.tick(53).await;
    assert_eq!(r.outcome().unwrap().outcome.kind, OutcomeKind::Failed(FailReason::BudgetExhausted));
}

#[tokio::test]
async fn a_job_without_a_claim_box_fails_and_nothing_starts() {
    let mut r = rig();
    let mut job = fell_job("j1");
    job.params.remove("claim");
    lease(&r.bus, &job, 1000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    assert!(r.host.started.is_empty());
    assert!(matches!(r.outcome().unwrap().outcome.kind, OutcomeKind::Failed(FailReason::Other(_))));
}

#[tokio::test]
async fn a_failed_skill_and_an_empty_one_map_to_failed_outcomes() {
    let mut r = rig();
    let (a, b) = (fell_job("j1"), fell_job("j2"));
    for j in [&a, &b] {
        lease(&r.bus, j, 1000).await;
        batch(&r.bus, j).await;
    }
    r.tick(1).await;
    r.host.handles[0].finish(RunEnd { status: RunStatus::Failed("no path".into()), items: BTreeMap::new(), gathered: 0, left: 1, drops_left: 0 });
    r.tick(2).await;
    assert_eq!(r.outcome().unwrap().outcome.kind, OutcomeKind::Failed(FailReason::Other("no path".into())));
    r.tick(3).await;
    r.host.handles[1].finish(RunEnd { status: RunStatus::Done, items: BTreeMap::new(), gathered: 0, left: 0, drops_left: 0 });
    r.tick(4).await;
    assert_eq!(r.outcome().unwrap().outcome.kind, OutcomeKind::Failed(FailReason::TargetGone));
}

#[tokio::test]
async fn a_bot_that_dies_drops_its_job_sends_no_outcome_and_publishes_dead() {
    let mut r = rig();
    let sum = r.bus.subscribe_summary(&bot_summary(&srv(), &BotId::new("b1")));
    let job = fell_job("j1");
    lease(&r.bus, &job, 1000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    r.host.life = Some(Life::Dead);
    r.tick(2).await;
    assert!(r.outcome().is_none());
    let Some(SummaryBody::Bot(s)) = sum.latest() else { panic!("no state") };
    assert_eq!((s.life, s.job), (Life::Dead, None));
    // Back alive: nothing from before is replayed.
    r.host.life = None;
    r.tick(3).await;
    assert!(r.link.is_idle());
    assert_eq!(r.host.started.len(), 1);
}

/// A skill that never steps its stop may have a break in flight, so no outcome goes up (S6b1): the claim frees at lease
/// end. When the skill finally steps, the run ends and reports as usual.
#[tokio::test]
async fn a_skill_that_never_steps_its_stop_sends_no_outcome_until_it_does() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 1000).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    halt(&r.bus);
    r.tick(2).await;
    r.tick(50).await;
    assert!(r.outcome().is_none());
    assert!(!r.link.stuck(Gt(50)));
    r.tick(150).await;
    r.tick(400).await;
    assert!(r.outcome().is_none(), "no clean outcome while the skill has not stopped");
    assert!(r.link.stuck(Gt(400)) && !r.link.is_idle());
    r.host.handles[0].finish(end(RunStatus::Stopped, 2));
    r.tick(401).await;
    let o = r.outcome().expect("the skill stopped, so the outcome goes");
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert!(!r.link.stuck(Gt(401)));
}

/// The hard stop does not depend on the skill: the host is told at stop time, and again inside the lease margin when the
/// skill still has not ended (S6b1 hard-stop fix). A skill that stops at once gets one call only.
#[tokio::test]
async fn a_skill_that_never_steps_its_stop_still_gets_the_host_hard_stop_at_stop_time_and_at_the_margin() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 300).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    assert_eq!(r.host.halts, 0);
    halt(&r.bus);
    r.tick(2).await;
    assert_eq!(r.host.halts, 1, "at stop time");
    r.tick(100).await;
    assert_eq!(r.host.halts, 1, "not repeated before the margin");
    r.tick(291).await; // 291 + LEASE_MARGIN >= 300
    assert_eq!(r.host.halts, 2, "again at the margin");
    r.tick(292).await;
    assert_eq!(r.host.halts, 2, "once at the margin");
    assert!(r.outcome().is_none());
}

/// A lease margin stop is already the last chance: one hard stop, not two.
#[tokio::test]
async fn a_stop_at_the_lease_margin_is_one_hard_stop() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 200).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    r.tick(195).await;
    r.tick(196).await;
    assert_eq!(r.host.halts, 1);
}

/// A stuck run past its lease still sends nothing, and a host that dies ends the run without an outcome.
#[tokio::test]
async fn a_stuck_run_past_its_lease_and_then_a_dead_host_send_no_outcome() {
    let mut r = rig();
    let job = fell_job("j1");
    lease(&r.bus, &job, 200).await;
    batch(&r.bus, &job).await;
    r.tick(1).await;
    r.tick(195).await; // inside the lease margin: the link asks for the stop
    r.tick(400).await;
    assert!(r.outcome().is_none() && r.link.stuck(Gt(400)));
    r.host.life = Some(Life::Dead);
    r.tick(401).await;
    assert!(r.outcome().is_none());
    assert_eq!(r.link.life(), Life::Dead);
    assert!(!r.link.stuck(Gt(401)), "the run ended with the host");
}

/// The skill side: a stop ends `FellRun` at its next step, stops the walk and reports the inventory gain.
#[test]
fn a_stopped_fell_run_stops_navigation_and_reports_the_logs_in_the_inventory() {
    use null_bot_api::fake::{FakeCommands, FakeView};
    use null_bot_api::{Command as Cmd, ItemStack, Skill, SkillStatus};
    let handle = RunHandle::new();
    let spec = FellSpec { tree_type: "oak".into(), quota: 6, min: region().min, max: region().max };
    let mut run = FellRun::new(&spec, "b1", handle.clone());
    let view = FakeView::default();
    let mut cmds = FakeCommands::default();
    assert_eq!(run.step(&view, &mut cmds), SkillStatus::Running);
    assert!(handle.take_end().is_none());
    // The fake view reads its inventory once, so the later state is a second view.
    let mut later = FakeView::default();
    later.hotbar[0] = Some(ItemStack::new("minecraft:oak_log", 4));
    handle.request_stop();
    assert_eq!(run.step(&later, &mut cmds), SkillStatus::Done);
    let end = handle.take_end().expect("end");
    assert_eq!(end.status, RunStatus::Stopped);
    assert_eq!(end.items, BTreeMap::from([("minecraft:oak_log".to_string(), 4)]));
    assert!(cmds.all().contains(&Cmd::StopNav), "{:?}", cmds.all());
    assert!(cmds.all().contains(&Cmd::CancelBreak), "a stop aborts the break in progress: {:?}", cmds.all());
}
