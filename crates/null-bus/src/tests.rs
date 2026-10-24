//! Each class keeps its guarantee. Spec: specs/F6-bus-deployment.md §3
use crate::*;
use null_proto::*;
use std::time::Duration;
use tokio::time::timeout;

fn srv() -> ServerId {
    ServerId::new("smp1")
}
fn subj(level: Level, id: Option<&str>, k: &str) -> Subject {
    Subject::new(&srv(), level, id, k)
}
fn intent(key: &str) -> Intent {
    Intent { key: key.into(), body: IntentBody::Command(Command::Status) }
}
fn summary(done: u32) -> SummaryBody {
    SummaryBody::Operation(OpSummary {
        op: OperationId::new("op1"),
        server: srv(),
        goal: GoalId::new("g1"),
        state: OpState::Running,
        headcount: Headcount { assigned: 2, alive: 2 },
        progress: Progress { done, target: 64 },
        stock: vec![],
        health: Health::Ok,
        top_issue: None,
        exceptions: vec![],
        at: Gt(0),
    })
}
fn escalation(key: &str) -> Escalation {
    Escalation {
        key: key.into(),
        from_op: OperationId::new("op1"),
        from_squad: None,
        from_bot: None,
        kind: "k".into(),
        detail: "d".into(),
        remedies_tried: vec![],
        at: Gt(1),
    }
}
fn take(id: &str) -> ClaimRequest {
    ClaimRequest::Take {
        id: ClaimId::new(id),
        kind: ClaimKind::Bot(BotId::new("Bot1")),
        owner: ClaimOwner::Squad(SquadId::new("s1")),
        priority: Priority(1),
        ttl: Gt(100),
    }
}

#[test]
fn subject_scheme() {
    let s = subj(Level::Op, Some("castle"), kind::JOBS);
    assert_eq!(s.to_string(), "null.smp1.op.castle.jobs");
    assert_eq!(Subject::parse("null.smp1.op.castle.jobs"), Some(s));
    let d = subj(Level::Hq, None, kind::DIGEST);
    assert_eq!(d.to_string(), "null.smp1.hq.digest");
    assert_eq!(Subject::parse("null.smp1.hq.digest"), Some(d));
    assert_eq!(Subject::parse("x.smp1.hq.digest"), None);
    assert_eq!(Subject::parse("null.smp1.zzz.digest"), None);
}

#[tokio::test]
async fn duplicate_intent_applied_once() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::JOBS);
    // Published before anyone subscribes: stays buffered.
    for k in ["k1", "k1", "k2", "k1"] {
        bus.publish_intent(&s, intent(k)).await.unwrap();
    }
    let mut rx = bus.subscribe_intents(&s).unwrap();
    assert_eq!(rx.recv().await.unwrap().key, "k1");
    assert_eq!(rx.recv().await.unwrap().key, "k2");
    assert!(rx.try_recv().is_none());
    assert_eq!(bus.subscribe_intents(&s).err(), Some(BusError::AlreadySubscribed));
}

#[tokio::test]
async fn summary_keeps_latest() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::SUMMARY);
    let mut rx = bus.subscribe_summary(&s);
    assert_eq!(rx.latest(), None);
    for n in 1..=5 {
        bus.publish_summary(&s, summary(n));
    }
    assert_eq!(rx.changed().await, Some(summary(5)));
    assert_eq!(rx.latest(), Some(summary(5)));
    // A late subscriber sees only the newest too.
    assert_eq!(bus.subscribe_summary(&s).latest(), Some(summary(5)));
    // Nothing newer: changed() waits.
    assert!(timeout(Duration::from_millis(20), rx.changed()).await.is_err());
}

#[tokio::test]
async fn escalations_deduplicated_by_key() {
    let bus = InProcBus::default();
    let s = subj(Level::Hq, None, kind::ESCALATE);
    let mut rx = bus.subscribe_escalations(&s).unwrap();
    for k in ["no_trees", "no_trees", "stuck", "no_trees"] {
        bus.publish_escalation(&s, escalation(k)).await.unwrap();
    }
    assert_eq!(rx.recv().await.unwrap().key, "no_trees");
    assert_eq!(rx.recv().await.unwrap().key, "stuck");
    assert!(rx.try_recv().is_none());
}

#[tokio::test(start_paused = true)]
async fn claim_request_times_out_without_reply() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::CLAIMS);
    // No server at all.
    assert_eq!(bus.request_claim(&s, take("c1"), WallMs(500)).await, Err(BusError::Timeout));
    // A server that receives the call and never answers.
    let mut server = bus.serve_claims(&s).unwrap();
    let held = tokio::spawn(async move {
        let call = server.next().await.unwrap();
        tokio::time::sleep(Duration::from_secs(3600)).await;
        drop(call);
    });
    assert_eq!(bus.request_claim(&s, take("c2"), WallMs(500)).await, Err(BusError::Timeout));
    held.abort();
}

#[tokio::test]
async fn claim_request_gets_reply() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::CLAIMS);
    let mut server = bus.serve_claims(&s).unwrap();
    tokio::spawn(async move {
        while let Some(call) = server.next().await {
            let id = call.req.id().clone();
            call.reply(ClaimReply::Granted { id, token: 1, expires_at: Gt(100) });
        }
    });
    let r = bus.request_claim(&s, take("c1"), WallMs(500)).await;
    assert_eq!(r, Ok(ClaimReply::Granted { id: ClaimId::new("c1"), token: 1, expires_at: Gt(100) }));
}

#[tokio::test]
async fn claim_server_dropping_call_is_no_reply() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::CLAIMS);
    let mut server = bus.serve_claims(&s).unwrap();
    tokio::spawn(async move { drop(server.next().await) });
    assert_eq!(bus.request_claim(&s, take("c1"), WallMs(500)).await, Err(BusError::NoReply));
}

#[test]
fn event_log_is_append_only_and_ordered() {
    let bus = InProcBus::default();
    let s = subj(Level::Op, Some("a"), kind::EVENTS);
    let ev = |n: u64| LogEvent { at: WallMs(n), kind: "e".into(), data: n.to_string() };
    assert!(bus.read_events(&s, 0).is_empty());
    for n in 0..4 {
        assert_eq!(bus.append_event(&s, ev(n)), n);
    }
    let all = bus.read_events(&s, 0);
    assert_eq!(all.iter().map(|(i, _)| *i).collect::<Vec<_>>(), [0, 1, 2, 3]);
    assert_eq!(all[2].1, ev(2));
    // Reading does not consume; a later read from a sequence number returns the tail.
    assert_eq!(bus.read_events(&s, 3), vec![(3, ev(3))]);
    bus.append_event(&s, ev(4));
    assert_eq!(bus.read_events(&s, 0).len(), 5);
    assert_eq!(bus.read_events(&s, 0)[..4], all[..]);
    // Logs on other subjects are separate.
    assert!(bus.read_events(&subj(Level::Op, Some("b"), kind::EVENTS), 0).is_empty());
}

#[tokio::test]
async fn halt_overtakes_full_intent_queue() {
    let bus = InProcBus::with_capacity(2);
    let jobs = subj(Level::Op, Some("a"), kind::JOBS);
    let cmd = subj(Level::Op, Some("a"), kind::CMD);
    let mut crx = bus.subscribe_commands(&cmd);
    let mut irx = bus.subscribe_intents(&jobs).unwrap();
    bus.publish_intent(&jobs, intent("k1")).await.unwrap();
    bus.publish_intent(&jobs, intent("k2")).await.unwrap();
    // The queue is full: a third intent waits.
    assert!(timeout(Duration::from_millis(20), bus.publish_intent(&jobs, intent("k3"))).await.is_err());
    // Halt goes straight through, and arrives while both intents are still queued.
    let halt = Command::Halt { scope: Scope::Swarm };
    bus.send_command(&cmd, halt.clone()).unwrap();
    assert_eq!(crx.try_recv(), Some(halt));
    assert_eq!(irx.try_recv().unwrap().key, "k1");
    // Non-urgent commands are refused on this class.
    assert_eq!(bus.send_command(&cmd, Command::Status), Err(BusError::NotUrgent));
}

#[tokio::test]
async fn command_reaches_every_subscriber() {
    let bus = InProcBus::default();
    let cmd = subj(Level::Hq, None, kind::CMD);
    let mut a = bus.subscribe_commands(&cmd);
    let mut b = bus.subscribe_commands(&cmd);
    let recall = Command::Recall { scope: Scope::Swarm };
    bus.send_command(&cmd, recall.clone()).unwrap();
    assert_eq!(a.recv().await, Some(recall.clone()));
    assert_eq!(b.recv().await, Some(recall));
}

#[tokio::test]
async fn command_sent_before_subscribe_is_held() {
    let bus = InProcBus::default();
    let cmd = subj(Level::Hq, None, kind::CMD);
    let halt = Command::Halt { scope: Scope::Swarm };
    bus.send_command(&cmd, halt.clone()).unwrap();
    let mut rx = bus.subscribe_commands(&cmd);
    assert_eq!(rx.try_recv(), Some(halt));
    // Only the first subscriber gets the held command.
    let mut late = bus.subscribe_commands(&cmd);
    assert_eq!(late.try_recv(), None);
}
