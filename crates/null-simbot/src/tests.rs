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

async fn send(bus: &InProcBus, bot: &str, job: Job) {
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
