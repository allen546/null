//! Round-trip every message family through JSON. Spec: specs/F6-bus-deployment.md §4.2
use crate::*;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::BTreeMap;

fn rt<T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug>(v: T) {
    let s = serde_json::to_string(&v).unwrap();
    let back: T = serde_json::from_str(&s).unwrap();
    assert_eq!(v, back, "{s}");
}

fn loc() -> Loc {
    Loc { server: ServerId::new("smp1"), dim: Dimension::Overworld, pos: Pos::new(1, 64, -3) }
}
fn region() -> Region {
    Region { server: ServerId::new("smp1"), dim: Dimension::Nether, min: Pos::new(0, 0, 0), max: Pos::new(9, 9, 9) }
}
fn log(n: u32) -> ItemCount {
    ItemCount { item: ItemId::new("minecraft:oak_log"), n }
}
fn job() -> Job {
    let mut params = BTreeMap::new();
    params.insert("trees".into(), ParamValue::Locs(vec![loc()]));
    params.insert("settle".into(), ParamValue::Ticks(Gt(20)));
    Job {
        id: JobId::new("j1"),
        kind: "fell".into(),
        params,
        items: vec![log(1)],
        claims: vec![ClaimId::new("c1")],
        preconditions: vec![Condition::All(vec![Condition::Always, Condition::HaveItems { item: ItemId::new("minecraft:iron_axe"), n: 1 }])],
        success: Condition::BlockAbsent { at: loc() },
        budget: Budget { ticks: Gt(1200), attempts: 2 },
        priority: Priority(5),
        reflexes: vec![ReflexSetting { target: ReflexTarget::Group(ReflexGroup::Combat), on: false }],
        checkpoint: Checkpoint::Resumable,
        preemption_floor: Some(2),
        attempt: 2,
    }
}
fn summary() -> OpSummary {
    OpSummary {
        op: OperationId::new("op1"),
        server: ServerId::new("smp1"),
        goal: GoalId::new("g1"),
        state: OpState::Running,
        headcount: Headcount { assigned: 2, alive: 2 },
        progress: Progress { done: 10, target: 64 },
        stock: vec![log(10)],
        health: Health::Ok,
        top_issue: None,
        exceptions: vec![],
        at: Gt(100),
    }
}

#[test]
fn ids_time_loc() {
    rt(BotId::new("Bot1"));
    rt(Gt(7));
    rt(WallMs(250));
    rt(loc());
    rt(region());
    assert_eq!(serde_json::to_string(&Gt(7)).unwrap(), "7");
}

#[test]
fn goals() {
    let all = vec![
        Goal::Obtain { item: ItemId::new("minecraft:oak_log"), n: 64 },
        Goal::Build { schematic: "hut".into(), site: loc() },
        Goal::Defend { region: region() },
        Goal::KillDragon,
        Goal::Explore { area: region() },
        Goal::Hold { position: loc() },
    ];
    for g in &all {
        rt(g.clone());
    }
    assert!(all[0].check_supported().is_ok());
    for g in &all[1..] {
        assert!(g.check_supported().is_err());
    }
}

#[test]
fn jobs_and_outcomes() {
    rt(job());
    let report = Report { elapsed: Gt(40), items_gained: vec![log(4)], items_lost: vec![], at: Some(loc()), note: "ok".into() };
    for kind in [
        OutcomeKind::Succeeded,
        OutcomeKind::Failed(FailReason::NoPath),
        OutcomeKind::Failed(FailReason::Other("x".into())),
        OutcomeKind::Blocked(BlockedOn::ReflexPolicy("upkeep.eat off".into())),
        OutcomeKind::Blocked(BlockedOn::Claim(ClaimId::new("c1"))),
        OutcomeKind::Preempted,
        OutcomeKind::Interrupted(Incident { kind: "lava".into(), at: Some(loc()), detail: "d".into() }),
    ] {
        rt(Outcome { job: JobId::new("j1"), kind, report: report.clone() });
    }
}

#[test]
fn claims() {
    let owner = ClaimOwner::Squad(SquadId::new("s1"));
    for kind in [
        ClaimKind::Region(region()),
        ClaimKind::BlockSet(vec![loc()]),
        ClaimKind::Container(loc()),
        ClaimKind::Items { container: loc(), item: ItemId::new("minecraft:coal"), n: 3 },
        ClaimKind::Entity(9),
        ClaimKind::Bot(BotId::new("Bot1")),
    ] {
        rt(ClaimRequest::Take { id: ClaimId::new("c1"), kind, owner: owner.clone(), priority: Priority(1), ttl: Gt(600) });
    }
    rt(ClaimRequest::Renew { id: ClaimId::new("c1"), owner: owner.clone(), ttl: Gt(600) });
    rt(ClaimRequest::Release { id: ClaimId::new("c1"), owner: owner.clone() });
    rt(ClaimReply::Granted { id: ClaimId::new("c1"), token: 3, expires_at: Gt(700) });
    rt(ClaimReply::Denied { id: ClaimId::new("c1"), reason: DenyReason::Held { by: owner.clone() } });
    rt(ClaimReply::Preempt { id: ClaimId::new("c1"), by: owner });
    rt(ClaimReply::Released { id: ClaimId::new("c1") });
}

#[test]
fn upward_and_digest() {
    let mut s = summary();
    merge_exception(&mut s.exceptions, "no_path", "tree7", Gt(10));
    merge_exception(&mut s.exceptions, "no_path", "tree7", Gt(30));
    merge_exception(&mut s.exceptions, "no_path", "tree8", Gt(31));
    assert_eq!(s.exceptions.len(), 2);
    assert_eq!((s.exceptions[0].count, s.exceptions[0].first_seen, s.exceptions[0].last_seen), (2, Gt(10), Gt(30)));
    rt(s.clone());
    rt(Escalation {
        key: "op1:no_trees".into(),
        from_op: OperationId::new("op1"),
        from_squad: Some(SquadId::new("s1")),
        from_bot: None,
        kind: "no_trees".into(),
        detail: "none in range".into(),
        remedies_tried: vec![Remedy { action: "widen search".into(), result: "none".into() }],
        at: Gt(5),
    });
    rt(Digest {
        generation: 4,
        at: Gt(100),
        rows: vec![DigestRow {
            op: s.op,
            server: s.server,
            goal: s.goal,
            state: OpState::Blocked,
            headcount: s.headcount,
            progress: s.progress,
            top_issue: Some("x".into()),
            stale: false,
        }],
        warnings: vec!["survival reflexes off in op1".into()],
    });
}

#[test]
fn bot_messages() {
    rt(BotState { bot: BotId::new("Bot1"), life: Life::Alive, at_loc: Some(loc()), health: 20.0, job: None, tick: Gt(3) });
    let obs = Observation { observer: BotId::new("Bot1"), tick: Gt(3), what: Observed::Block { at: loc(), block: ItemId::new("minecraft:oak_log") } };
    rt(obs.clone());
    rt(Observation { what: Observed::Entity { id: 4, kind: "zombie".into(), at: loc() }, ..obs });
    rt(JobOutcome {
        bot: BotId::new("Bot1"),
        outcome: Outcome { job: JobId::new("j1"), kind: OutcomeKind::Succeeded, report: Report::default() },
        tick: Gt(9),
        attempt: 2,
    });
}

#[test]
fn commands_and_bodies() {
    let gs = GoalSpec { id: GoalId::new("g1"), goal: Goal::Obtain { item: ItemId::new("minecraft:oak_log"), n: 64 }, priority: Priority(1), reason: "start".into() };
    let cmds = vec![
        Command::Status,
        Command::Inspect { target: InspectTarget::Goal(GoalId::new("g1")) },
        Command::Inspect { target: InspectTarget::Scope(Scope::Bot(BotId::new("Bot1"))) },
        Command::SetGoal(gs.clone()),
        Command::CancelGoal { id: GoalId::new("g1"), reason: "r".into() },
        Command::Halt { scope: Scope::Swarm },
        Command::Recall { scope: Scope::Server(ServerId::new("smp1")) },
        Command::SetReflexes { scope: Scope::Operation(OperationId::new("op1")), settings: vec![ReflexSetting { target: ReflexTarget::All, on: false }], reason: "r".into() },
    ];
    let urgent: Vec<bool> = cmds.iter().map(Command::is_urgent).collect();
    assert_eq!(urgent, [false, false, false, false, false, true, true, false]);
    for c in cmds {
        rt(c);
    }
    for body in [
        IntentBody::Goal { op: OperationId::new("op1"), region: Some(region()), goal: gs, quotas: vec![Quota { item: ItemId::new("minecraft:oak_log"), n: 64 }] },
        IntentBody::Allocation(Allocation { op: OperationId::new("op1"), bots: vec![BotId::new("Bot1")] }),
        IntentBody::JobBatch { squad: SquadId::new("s1"), jobs: vec![job()] },
        IntentBody::Command(Command::Status),
    ] {
        rt(Intent { key: "k".into(), body });
    }
    rt(SummaryBody::Operation(summary()));
    rt(LogEvent { at: WallMs(1), kind: "k".into(), data: "{}".into() });
}

#[test]
fn envelope_rejects_other_major() {
    let e = Envelope::new(Gt(1));
    rt(e.clone());
    assert_eq!(e.open(), Ok(Gt(1)));
    let bad = Envelope { proto_version: ProtoVersion { major: PROTO_VERSION.major + 1, minor: 0 }, body: Gt(1) };
    assert!(bad.open().is_err());
    let newer_minor = Envelope { proto_version: ProtoVersion { major: PROTO_VERSION.major, minor: 9 }, body: Gt(1) };
    assert!(newer_minor.open().is_ok());
}

#[test]
fn region_geometry() {
    let r = region();
    assert!(!r.contains(&loc()));
    let inside = Loc { dim: Dimension::Nether, pos: Pos::new(5, 5, 5), ..loc() };
    assert!(r.contains(&inside));
    let other_server = Loc { server: ServerId::new("smp2"), ..inside };
    assert!(!r.contains(&other_server));
}

#[test]
fn exceptions_stay_capped() {
    let mut list = vec![];
    for i in 0..(MAX_EXCEPTIONS as u64 + 5) {
        merge_exception(&mut list, "no_path", &format!("tree{i}"), Gt(i));
    }
    assert_eq!(list.len(), MAX_EXCEPTIONS);
    // The oldest by last seen went first; the newest stays.
    assert!(list.iter().all(|e| e.subject != "tree0"));
    assert!(list.iter().any(|e| e.subject == format!("tree{}", MAX_EXCEPTIONS + 4)));
}
