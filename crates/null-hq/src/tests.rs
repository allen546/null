//! HQ tests against a bare bus. Spec: specs/K1-hq.md §8
use crate::*;
use null_bus::{Bus, CommandRx, InProcBus, IntentRx};
use null_op::{hq_escalate, op_cmd, op_intents, op_summary};
use null_proto::*;
use null_squad::squad_cmd;

fn srv() -> ServerId {
    ServerId::new("smp1")
}
fn goal(id: &str, n: u32, prio: u8) -> GoalSpec {
    GoalSpec {
        id: GoalId::new(id),
        goal: Goal::Obtain { item: ItemId::new("minecraft:oak_log"), n },
        priority: Priority(prio),
        reason: "test".into(),
    }
}
fn pool(n: usize) -> Vec<BotId> {
    (1..=n).map(|i| BotId::new(format!("b{i}"))).collect()
}
fn hq(bus: &InProcBus, n: usize) -> Hq<InProcBus> {
    Hq::new(bus, HqConfig::new("smp1", pool(n)))
}
fn op(n: u32) -> OperationId {
    OperationId::new(format!("op{n}"))
}
fn summary(o: &OperationId, state: OpState, at: u64) -> SummaryBody {
    SummaryBody::Operation(OpSummary {
        op: o.clone(),
        server: srv(),
        goal: GoalId::new("g1"),
        state,
        headcount: Headcount { assigned: 2, alive: 2 },
        progress: Progress { done: 0, target: 64 },
        stock: vec![],
        health: Health::Ok,
        top_issue: None,
        exceptions: vec![],
        at: Gt(at),
    })
}
fn halts(rx: &mut CommandRx) -> Vec<Command> {
    std::iter::from_fn(|| rx.try_recv()).collect()
}

#[tokio::test]
async fn a_goal_makes_one_operation_with_all_free_bots_and_tells_it() {
    let bus = InProcBus::default();
    let mut rx: IntentRx = bus.subscribe_intents(&op_intents(&srv(), &op(1))).unwrap();
    let mut hq = hq(&bus, 3);
    let r = hq.handle(Command::SetGoal(goal("g1", 64, 1))).await;
    assert!(matches!(&r, Reply::Accepted { effect } if effect.contains("op1") && effect.contains("3 bots")), "{r:?}");
    let plans = hq.take_launches();
    assert_eq!(plans.len(), 1);
    assert_eq!((plans[0].op.as_str(), plans[0].squad.as_str(), plans[0].bots.len()), ("op1", "sq1", 3));
    let kinds: Vec<_> = std::iter::from_fn(|| rx.try_recv()).map(|i| i.body).collect();
    assert!(matches!(kinds[0], IntentBody::Allocation(_)) && matches!(kinds[1], IntentBody::Goal { .. }));
    assert_eq!(hq.free_bots(), 0);
}

#[tokio::test]
async fn writes_are_checked_and_resending_is_harmless() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 2);
    let mut g = goal("g1", 64, 1);
    g.reason = " ".into();
    assert!(matches!(hq.handle(Command::SetGoal(g)).await, Reply::Rejected { .. }));
    let build = GoalSpec { goal: Goal::KillDragon, ..goal("g2", 1, 1) };
    assert!(matches!(hq.handle(Command::SetGoal(build)).await, Reply::Rejected { .. }));
    assert!(matches!(hq.handle(Command::SetGoal(goal("g3", 0, 1))).await, Reply::Rejected { .. }));
    assert!(matches!(hq.handle(Command::SetGoal(goal("g1", 64, 1))).await, Reply::Accepted { .. }));
    assert!(matches!(hq.handle(Command::SetGoal(goal("g1", 64, 1))).await, Reply::Accepted { effect } if effect.contains("already")));
    assert!(matches!(hq.handle(Command::SetGoal(goal("g1", 65, 1))).await, Reply::Rejected { .. }));
    assert_eq!(hq.take_launches().len(), 1, "the resend started nothing");
}

#[tokio::test]
async fn a_goal_waits_for_bots_and_starts_when_the_first_operation_is_done() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 2);
    hq.handle(Command::SetGoal(goal("g1", 64, 1))).await;
    let r = hq.handle(Command::SetGoal(goal("g2", 10, 1))).await;
    assert!(matches!(&r, Reply::Accepted { effect } if effect.contains("queued")), "{r:?}");
    assert_eq!(hq.take_launches().len(), 1);
    bus.publish_summary(&op_summary(&srv(), &op(1)), summary(&op(1), OpState::Done, 5));
    hq.tick(Gt(6)).await;
    assert!(hq.take_retired().is_empty(), "the operation stays while its bots wind down");
    hq.tick(Gt(50)).await;
    assert_eq!(hq.take_retired(), vec![op(1)]);
    let plans = hq.take_launches();
    assert_eq!((plans[0].op.as_str(), plans[0].goal.id.as_str(), plans[0].bots.len()), ("op2", "g2", 2));
}

#[tokio::test]
async fn the_higher_priority_goal_starts_first_when_bots_come_free() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 1);
    hq.handle(Command::SetGoal(goal("g1", 8, 1))).await;
    hq.handle(Command::SetGoal(goal("low", 8, 1))).await;
    hq.handle(Command::SetGoal(goal("high", 8, 9))).await;
    hq.take_launches();
    bus.publish_summary(&op_summary(&srv(), &op(1)), summary(&op(1), OpState::Done, 5));
    hq.tick(Gt(6)).await;
    hq.tick(Gt(50)).await;
    assert_eq!(hq.take_launches()[0].goal.id.as_str(), "high");
}

#[tokio::test]
async fn halt_reaches_the_covered_operations_and_squads_at_once() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 4);
    hq.handle(Command::SetGoal(goal("g1", 8, 1))).await;
    hq.handle(Command::SetGoal(goal("g2", 8, 1))).await; // queued: no free bots, so no op2
    let (mut oc, mut sc) = (bus.subscribe_commands(&op_cmd(&srv(), &op(1))), bus.subscribe_commands(&squad_cmd(&srv(), &SquadId::new("sq1"))));
    // No tick between the command and its delivery: halt is not queued behind anything.
    let r = hq.handle(Command::Halt { scope: Scope::Swarm }).await;
    assert!(matches!(r, Reply::Accepted { .. }), "{r:?}");
    assert_eq!(halts(&mut oc).len(), 1);
    assert_eq!(halts(&mut sc).len(), 1);
    // A bot-scope halt goes to the squad only; the operation keeps planning.
    hq.handle(Command::Recall { scope: Scope::Bot(BotId::new("b2")) }).await;
    assert!(halts(&mut oc).is_empty());
    assert_eq!(halts(&mut sc).len(), 1);
    let none = hq.handle(Command::Halt { scope: Scope::Operation(op(7)) }).await;
    assert!(matches!(none, Reply::Rejected { .. }));
}

#[tokio::test]
async fn digest_has_one_row_per_operation_and_marks_a_silent_one_stale() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 2);
    hq.handle(Command::SetGoal(goal("g1", 64, 1))).await;
    let Reply::Status(d1) = hq.handle(Command::Status).await else { panic!() };
    assert_eq!((d1.rows.len(), d1.rows[0].state, d1.rows[0].stale), (1, OpState::Planning, false));
    bus.publish_summary(&op_summary(&srv(), &op(1)), summary(&op(1), OpState::Running, 10));
    hq.tick(Gt(11)).await;
    hq.tick(Gt(500)).await;
    let Reply::Status(d2) = hq.handle(Command::Status).await else { panic!() };
    assert!(d2.generation > d1.generation);
    assert_eq!((d2.rows[0].state, d2.rows[0].stale), (OpState::Running, true));
    assert!(matches!(bus.subscribe_summary(&crate::hq_digest(&srv())).latest(), Some(SummaryBody::Digest(_))));
}

#[tokio::test]
async fn escalations_merge_by_key_and_show_as_one_warning_line() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 1);
    for (k, at) in [("a", 1), ("b", 2), ("b2", 3)] {
        let e = Escalation {
            key: k.into(),
            from_op: op(1),
            from_squad: None,
            from_bot: None,
            kind: "job_dead".into(),
            detail: String::new(),
            remedies_tried: vec![],
            at: Gt(at),
        };
        bus.publish_escalation(&hq_escalate(&srv()), e).await.unwrap();
    }
    hq.tick(Gt(5)).await;
    assert_eq!(hq.inbox().len(), 3);
    let Reply::Status(d) = hq.handle(Command::Status).await else { panic!() };
    assert_eq!(d.warnings.len(), 1);
}

#[tokio::test]
async fn inspect_shows_goal_operation_and_bots() {
    let bus = InProcBus::default();
    let mut hq = hq(&bus, 2);
    hq.handle(Command::SetGoal(goal("g1", 64, 1))).await;
    bus.publish_summary(&op_summary(&srv(), &op(1)), summary(&op(1), OpState::Running, 3));
    let Reply::Inspect(i) = hq.handle(Command::Inspect { target: InspectTarget::Scope(Scope::Operation(op(1))) }).await else { panic!() };
    assert_eq!((i.bots.len(), i.goal.map(|g| g.id)), (2, Some(GoalId::new("g1"))));
    assert_eq!(i.summary.map(|s| s.state), Some(OpState::Running));
    assert!(matches!(hq.handle(Command::Inspect { target: InspectTarget::Goal(GoalId::new("nope")) }).await, Reply::Rejected { .. }));
}
