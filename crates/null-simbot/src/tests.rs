//! Fake bots over the bus. Spec: specs/F3-test-harness.md §4.5
use crate::*;
use null_bus::{Bus, InProcBus};
use null_proto::*;
use std::collections::BTreeMap;

fn srv() -> ServerId {
    ServerId::new("smp1")
}

fn fell_job(id: &str, trees: Vec<Loc>) -> Job {
    Job {
        id: JobId::new(id),
        kind: "fell".into(),
        params: BTreeMap::from([("trees".to_string(), ParamValue::Locs(trees))]),
        items: vec![],
        claims: vec![],
        preconditions: vec![Condition::Always],
        success: Condition::Always,
        budget: Budget {
            ticks: Gt(500),
            attempts: 1,
        },
        priority: Priority(1),
        reflexes: vec![],
        checkpoint: Checkpoint::None,
        preemption_floor: None,
        attempt: 0,
    }
}

/// Sends the job with a far-off lease, as the squad does (the bot refuses a job without one).
async fn send(bus: &InProcBus, bot: &str, job: Job) {
    lease(bus, bot, &job, 1 << 40).await;
    send_no_lease(bus, bot, job).await;
}

async fn send_no_lease(bus: &InProcBus, bot: &str, job: Job) {
    let body = IntentBody::JobBatch {
        squad: SquadId::new("s"),
        jobs: vec![job.clone()],
    };
    bus.publish_intent(
        &bot_jobs(&srv(), &BotId::new(bot)),
        Intent {
            key: format!("j:{}", job.id),
            body,
        },
    )
    .await
    .unwrap();
}

/// Run one bot to the first outcome (or `max` ticks); return the outcome and the tick.
async fn run(seed: u64, die: Option<u64>) -> (Option<JobOutcome>, Vec<BotState>) {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 6, 20).shared();
    let trees: Vec<Loc> = world
        .lock()
        .unwrap()
        .tree_list()
        .into_iter()
        .take(2)
        .map(|t| t.0)
        .collect();
    let mut bot = SimBot::new(&bus, &world, "b1", seed);
    if let Some(t) = die {
        bot.die_at(Gt(t));
    }
    let mut up = bus
        .subscribe_intents(&bot_up(&srv(), &BotId::new("b1")))
        .unwrap();
    let sum = bus.subscribe_summary(&bot_summary(&srv(), &BotId::new("b1")));
    send(&bus, "b1", fell_job("j1", trees)).await;
    let mut states = vec![];
    for t in 1..400 {
        bot.tick(Gt(t)).await;
        if let Some(SummaryBody::Bot(s)) = sum.latest() {
            states.push(s);
        }
        if let Some(Intent {
            body: IntentBody::JobOutcome(o),
            ..
        }) = up.try_recv()
        {
            return (Some(o), states);
        }
    }
    (None, states)
}

#[tokio::test]
async fn fell_reports_yield_of_both_trees() {
    let world = SimWorld::generate(7, srv(), 6, 20);
    let want: u32 = world.tree_list().iter().take(2).map(|t| t.1).sum();
    let (o, _) = run(1, None).await;
    let o = o.expect("outcome");
    assert_eq!(o.outcome.kind, OutcomeKind::Succeeded);
    assert_eq!(
        o.outcome.report.items_gained,
        vec![ItemCount {
            item: oak_log(),
            n: want
        }]
    );
}

#[tokio::test]
async fn same_seed_same_ticks_other_seed_may_differ() {
    let a = run(1, None).await.0.unwrap().tick;
    assert_eq!(a, run(1, None).await.0.unwrap().tick);
    let mut differs = false;
    for s in 2..40 {
        differs |= run(s, None).await.0.unwrap().tick != a;
    }
    assert!(differs, "walk jitter should depend on the seed");
}

#[tokio::test]
async fn dead_bot_sends_no_outcome_and_reports_dead() {
    let (o, states) = run(1, Some(5)).await;
    assert!(o.is_none());
    assert_eq!(states.last().unwrap().life, Life::Dead);
}

#[tokio::test]
async fn disconnected_bot_reports_disconnected() {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 3, 20).shared();
    let mut bot = SimBot::new(&bus, &world, "b1", 1);
    bot.disconnect_at(Gt(3));
    for t in 1..6 {
        bot.tick(Gt(t)).await;
    }
    assert_eq!(bot.life(), Life::Disconnected);
}

#[tokio::test]
async fn halt_ends_the_job_as_preempted() {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 6, 20).shared();
    let trees: Vec<Loc> = world
        .lock()
        .unwrap()
        .tree_list()
        .into_iter()
        .map(|t| t.0)
        .collect();
    let mut bot = SimBot::new(&bus, &world, "b1", 1);
    let mut up = bus
        .subscribe_intents(&bot_up(&srv(), &BotId::new("b1")))
        .unwrap();
    send(&bus, "b1", fell_job("j1", trees)).await;
    bot.tick(Gt(1)).await;
    bus.send_command(
        &bot_cmd(&srv(), &BotId::new("b1")),
        Command::Halt {
            scope: Scope::Bot(BotId::new("b1")),
        },
    )
    .unwrap();
    bot.tick(Gt(2)).await;
    let Some(Intent {
        body: IntentBody::JobOutcome(o),
        ..
    }) = up.try_recv()
    else {
        panic!("no outcome")
    };
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert!(bot.is_idle());
}

#[tokio::test]
async fn a_job_read_in_the_same_tick_as_a_halt_never_starts() {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 6, 20).shared();
    let logs = |w: &SharedWorld| w.lock().unwrap().tree_list().iter().map(|t| t.1).sum::<u32>();
    let before = logs(&world);
    let trees: Vec<Loc> = world.lock().unwrap().tree_list().into_iter().map(|t| t.0).collect();
    let mut bot = SimBot::new(&bus, &world, "b1", 1);
    bus.send_command(&bot_cmd(&srv(), &BotId::new("b1")), Command::Halt { scope: Scope::Bot(BotId::new("b1")) }).unwrap();
    send(&bus, "b1", fell_job("j1", trees)).await;
    for t in 1..400 {
        bot.tick(Gt(t)).await;
    }
    assert!(bot.is_idle());
    assert_eq!(logs(&world), before, "a halted bot took a log");
}

async fn lease(bus: &InProcBus, bot: &str, job: &Job, expires: u64) {
    let l = Lease {
        bot: BotId::new(bot),
        job: job.id.clone(),
        attempt: job.attempt,
        claim: ClaimId::new("c"),
        expires_at: Gt(expires),
    };
    let key = format!("l:{bot}:{expires}");
    bus.publish_intent(&bot_jobs(&srv(), &BotId::new(bot)), Intent { key, body: IntentBody::Lease(l) })
        .await
        .unwrap();
}

/// Lease rule (K6 §4.8): bot A stalls past its lease; attempt 2 goes to bot B, which holds the tree; A wakes.
/// Returns A's outcome and whether the tree still stood when A had woken and run for 190 ticks.
async fn stale_bot_wakes(with_lease: bool) -> (JobOutcome, bool) {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 6, 30).shared();
    let tree = world.lock().unwrap().tree_list().into_iter().map(|t| t.0).max_by_key(|l| l.pos.x + l.pos.z).unwrap();
    let mut a = SimBot::new(&bus, &world, "a", 1);
    let mut b = SimBot::new(&bus, &world, "b", 1);
    let mut up = bus.subscribe_intents(&bot_up(&srv(), &BotId::new("a"))).unwrap();
    let mut j1 = fell_job("j1", vec![tree.clone()]);
    j1.attempt = 1;
    let mut j2 = j1.clone();
    j2.attempt = 2;
    if with_lease {
        lease(&bus, "a", &j1, 60).await;
    }
    send_no_lease(&bus, "a", j1).await;
    a.stall(Gt(5), Gt(200)); // frozen from tick 5; the lease ends at 60, the claim passes on
    let mut left = None;
    for t in 1..=399 {
        if t == 100 {
            lease(&bus, "b", &j2, 500).await;
            send_no_lease(&bus, "b", j2.clone()).await;
            b.stall(Gt(110), Gt(400)); // B holds the tree and has not felled it yet
        }
        a.tick(Gt(t)).await;
        b.tick(Gt(t)).await;
        if t == 389 {
            left = Some(world.lock().unwrap().logs_at(tree.pos).is_some());
        }
    }
    let o = loop {
        match up.try_recv() {
            Some(Intent { body: IntentBody::JobOutcome(o), .. }) => break o,
            Some(_) => {}
            None => panic!("bot a never reported"),
        }
    };
    (o, left.unwrap())
}

#[tokio::test]
async fn bot_past_its_lease_stops_and_leaves_the_tree_to_the_next_attempt() {
    let (o, standing) = stale_bot_wakes(true).await;
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert!(o.outcome.report.items_gained.is_empty(), "it took no log");
    assert!(standing, "the tree is still there for attempt 2");
}

/// Fail closed: a job with no lease is refused, not run unchecked.
#[tokio::test]
async fn a_job_without_a_lease_is_refused() {
    let (o, standing) = stale_bot_wakes(false).await;
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert!(o.outcome.report.items_gained.is_empty());
    assert!(standing);
}

/// A lease for another job or attempt does not count.
#[tokio::test]
async fn a_lease_for_another_attempt_does_not_count() {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 6, 20).shared();
    let tree = world.lock().unwrap().tree_list()[0].0.clone();
    let mut bot = SimBot::new(&bus, &world, "b1", 1);
    let mut up = bus.subscribe_intents(&bot_up(&srv(), &BotId::new("b1"))).unwrap();
    let mut j = fell_job("j1", vec![tree]);
    j.attempt = 2;
    let mut other = j.clone();
    other.attempt = 1;
    lease(&bus, "b1", &other, 1 << 40).await;
    let mut other_job = j.clone();
    other_job.id = JobId::new("j9");
    lease(&bus, "b1", &other_job, 1 << 40).await;
    send_no_lease(&bus, "b1", j).await;
    for t in 1..=5 {
        bot.tick(Gt(t)).await;
    }
    let Some(Intent { body: IntentBody::JobOutcome(o), .. }) = up.try_recv() else { panic!("no refusal") };
    assert_eq!(o.outcome.kind, OutcomeKind::Preempted);
    assert!(o.outcome.report.items_gained.is_empty());
}

#[tokio::test]
async fn reflex_settings_are_recorded_and_a_hung_bot_publishes_no_state() {
    let bus = InProcBus::default();
    let world = SimWorld::generate(7, srv(), 3, 20).shared();
    let mut bot = SimBot::new(&bus, &world, "b1", 1);
    let rx = bus.subscribe_summary(&bot_summary(&srv(), &BotId::new("b1")));
    let set = Command::SetReflexes {
        scope: Scope::Bot(BotId::new("b1")),
        settings: vec![ReflexSetting { target: ReflexTarget::All, on: false }],
        reason: "t".into(),
    };
    bus.publish_intent(&bot_jobs(&srv(), &BotId::new("b1")), Intent { key: "r".into(), body: IntentBody::Command(set) })
        .await
        .unwrap();
    bot.tick(Gt(1)).await;
    assert_eq!(bot.reflex(&ReflexTarget::All), Some(false));
    bot.stall(Gt(2), Gt(10));
    bot.tick(Gt(5)).await;
    assert!(matches!(rx.latest(), Some(SummaryBody::Bot(s)) if s.tick == Gt(1)), "no state while hung");
}
