//! Operation tests on simulated bots. Spec: specs/K5-operation-service.md §8
use crate::*;
use null_bus::{Bus, InProcBus, IntentRx};
use null_proto::*;
use null_simbot::*;
use std::collections::{BTreeMap, VecDeque};

fn srv() -> ServerId {
    ServerId::new("smp1")
}
fn log_item() -> ItemId {
    oak_log()
}
fn opid() -> OperationId {
    OperationId::new("op1")
}

/// A few lines of squad: gives each job to an idle bot, keeps claims alive for running jobs, relays outcomes.
/// Stand-in only; S3b builds the real one (K6).
struct Squad {
    bus: InProcBus,
    jobs: IntentRx,
    ups: BTreeMap<BotId, IntentRx>,
    queue: VecDeque<Job>,
    /// (job, attempt) -> (bot, the claim it runs under).
    running: BTreeMap<(JobId, u32), (BotId, ClaimId)>,
    /// Bots whose claims the squad does not renew (it lost contact), as in a stall.
    mute: std::collections::BTreeSet<BotId>,
}

impl Squad {
    fn new(bus: &InProcBus, bots: &[&str]) -> Self {
        Self {
            bus: bus.clone(),
            jobs: bus
                .subscribe_intents(&squad_jobs(&srv(), &SquadId::new("sq1")))
                .unwrap(),
            ups: bots
                .iter()
                .map(|b| {
                    (
                        BotId::new(*b),
                        bus.subscribe_intents(&bot_up(&srv(), &BotId::new(*b)))
                            .unwrap(),
                    )
                })
                .collect(),
            queue: VecDeque::new(),
            running: BTreeMap::new(),
            mute: Default::default(),
        }
    }
    fn alive(&self, b: &BotId) -> bool {
        matches!(self.bus.subscribe_summary(&bot_summary(&srv(), b)).latest(), Some(SummaryBody::Bot(s)) if s.life == Life::Alive)
    }
    async fn tick(&mut self, now: Gt) {
        while let Some(i) = self.jobs.try_recv() {
            if let IntentBody::JobBatch { jobs, .. } = i.body {
                self.queue.extend(jobs);
            }
        }
        for rx in self.ups.values_mut() {
            while let Some(i) = rx.try_recv() {
                if let IntentBody::JobOutcome(o) = &i.body {
                    self.running.remove(&(o.outcome.job.clone(), o.attempt));
                }
                self.bus
                    .publish_intent(&op_intents(&srv(), &opid()), i)
                    .await
                    .unwrap();
            }
        }
        let bots: Vec<BotId> = self.ups.keys().cloned().collect();
        for b in bots {
            if self.alive(&b) && !self.running.values().any(|r| r.0 == b) {
                if let Some(job) = self.queue.pop_front() {
                    self.running.insert(
                        (job.id.clone(), job.attempt),
                        (b.clone(), job.claims[0].clone()),
                    );
                    let key = format!("assign:{}:{}", job.id, now.0);
                    let body = IntentBody::JobBatch {
                        squad: SquadId::new("sq1"),
                        jobs: vec![job],
                    };
                    self.bus
                        .publish_intent(&bot_jobs(&srv(), &b), Intent { key, body })
                        .await
                        .unwrap();
                }
            }
        }
        if now.0 % 20 == 0 {
            for ((job, _), (bot, claim)) in self.running.clone() {
                if !self.alive(&bot) || self.mute.contains(&bot) {
                    continue; // a dead bot's claims run out on their own
                }
                let req = ClaimRequest::Renew {
                    id: claim,
                    owner: ClaimOwner::Job(job),
                    ttl: Gt(100),
                };
                // Lease rule: any failed renew (denied, Preempt, no answer) halts the bot (K6 §4.8, §5).
                let renewed = matches!(
                    self.bus
                        .request_claim(&op_claims(&srv(), &opid()), req, WallMs(2000))
                        .await,
                    Ok(ClaimReply::Granted { .. })
                );
                if !renewed {
                    self.bus
                        .send_command(
                            &bot_cmd(&srv(), &bot),
                            Command::Halt {
                                scope: Scope::Bot(bot.clone()),
                            },
                        )
                        .unwrap();
                }
            }
        }
    }
}

struct Rig {
    bus: InProcBus,
    world: SharedWorld,
    trees: Vec<(Loc, u32)>,
    op: Option<Operation<InProcBus>>,
    squad: Squad,
    bots: Vec<SimBot<InProcBus>>,
    now: u64,
    cfg: OpConfig,
}

impl Rig {
    async fn new(seed: u64, bots: &[&str], trees: usize) -> Self {
        let bus = InProcBus::default();
        let world = SimWorld::generate(seed, srv(), trees, 30).shared();
        let tl = world.lock().unwrap().tree_list();
        let cfg = OpConfig::new("op1", "smp1", "sq1");
        let squad = Squad::new(&bus, bots);
        let sim = bots
            .iter()
            .map(|b| SimBot::new(&bus, &world, b, seed))
            .collect();
        let op = Operation::new(&bus, cfg.clone(), tl.clone());
        let rig = Self {
            bus,
            world,
            trees: tl,
            op: Some(op),
            squad,
            bots: sim,
            now: 0,
            cfg,
        };
        let alloc = Allocation {
            op: opid(),
            bots: bots.iter().map(|b| BotId::new(*b)).collect(),
        };
        rig.send(IntentBody::Allocation(alloc), "alloc").await;
        rig
    }
    async fn send(&self, body: IntentBody, key: &str) {
        self.bus
            .publish_intent(
                &op_intents(&srv(), &opid()),
                Intent {
                    key: key.into(),
                    body,
                },
            )
            .await
            .unwrap();
    }
    async fn goal(&self, n: u32) {
        let goal = GoalSpec {
            id: GoalId::new("g1"),
            goal: Goal::Obtain {
                item: log_item(),
                n,
            },
            priority: Priority(1),
            reason: "test".into(),
        };
        self.send(
            IntentBody::Goal {
                op: opid(),
                region: None,
                goal,
                quotas: vec![],
            },
            "goal",
        )
        .await;
    }
    fn op(&mut self) -> &mut Operation<InProcBus> {
        self.op.as_mut().unwrap()
    }
    async fn step(&mut self) {
        self.now += 1;
        let now = Gt(self.now);
        self.op().tick(now).await;
        self.squad.tick(now).await;
        for b in &mut self.bots {
            b.tick(now).await;
        }
    }
    async fn run_until(&mut self, max: u64, mut f: impl FnMut(&Operation<InProcBus>) -> bool) {
        for _ in 0..max {
            if f(self.op.as_ref().unwrap()) {
                return;
            }
            self.step().await;
        }
        panic!(
            "condition not met in {max} ticks; state {:?}, jobs {:?}",
            self.op().state(),
            self.op().job_states()
        );
    }
    fn log(&self) -> Vec<OpEvent> {
        decode(&self.bus_log())
    }
    fn bus_log(&self) -> Vec<LogEvent> {
        self.bus
            .read_events(&op_events(&srv(), &opid()), 0)
            .into_iter()
            .map(|(_, e)| e)
            .collect()
    }
}

/// No tree is under two claims at once, read from the log (done-when 1).
fn assert_no_double_claim(evs: &[OpEvent]) {
    let mut active: BTreeMap<ClaimId, Vec<Loc>> = BTreeMap::new();
    for e in evs {
        let OpEvent::Claim(c) = e else { continue };
        match c {
            ClaimEvent::Granted {
                id,
                kind: ClaimKind::BlockSet(locs),
                ..
            } => {
                for (other, held) in &active {
                    assert!(
                        !held.iter().any(|l| locs.contains(l)),
                        "{id} granted a tree that {other} still holds"
                    );
                }
                active.insert(id.clone(), locs.clone());
            }
            ClaimEvent::Released { id }
            | ClaimEvent::Expired { id, .. }
            | ClaimEvent::Preempted { id, .. } => {
                active.remove(id);
            }
            _ => {}
        }
    }
}

/// A job is finished or retried because of an outcome only when that outcome answers the job's current attempt.
fn assert_state_moves_only_on_current_outcomes(evs: &[OpEvent]) {
    let mut attempt: BTreeMap<JobId, u32> = BTreeMap::new();
    let mut last: Option<(JobId, u32)> = None;
    for e in evs {
        match e {
            OpEvent::Dispatched { job, attempt: a } => {
                attempt.insert(job.clone(), *a);
            }
            OpEvent::Outcome {
                outcome,
                attempt: a,
                ..
            } => last = Some((outcome.job.clone(), *a)),
            OpEvent::JobDone { job } => assert_eq!(
                last,
                Some((job.clone(), attempt[job])),
                "{job} finished by a stale outcome"
            ),
            OpEvent::Requeued { job, reason } if reason == "outcome_not_ok" => {
                assert_eq!(
                    last,
                    Some((job.clone(), attempt[job])),
                    "{job} requeued by a stale outcome"
                )
            }
            _ => {}
        }
    }
}

fn count(evs: &[OpEvent], kind: &str) -> usize {
    evs.iter().filter(|e| e.kind() == kind).count()
}

#[tokio::test]
async fn obtain_64_with_two_bots_no_double_claims() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    rig.goal(64).await;
    rig.run_until(3000, |o| o.state() == OpState::Done).await;
    assert!(rig.op().ledger(&log_item()) >= 64);
    let evs = rig.log();
    assert_no_double_claim(&evs);
    let planned: u32 = evs
        .iter()
        .filter_map(|e| {
            if let OpEvent::JobCreated { logs, .. } = e {
                Some(*logs)
            } else {
                None
            }
        })
        .sum();
    assert!(planned >= 80, "margin: planned {planned} logs for 64");
    let s = rig.op().summary();
    assert_eq!(
        (s.state, s.progress),
        (
            OpState::Done,
            Progress {
                done: 64,
                target: 64
            }
        )
    );
    assert!(
        rig.op().claims().is_empty(),
        "claims are released when the goal is met"
    );
}

#[tokio::test]
async fn dead_bot_claim_expires_and_job_is_requeued() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    rig.bots[0].die_at(Gt(30));
    rig.goal(64).await;
    rig.run_until(6000, |o| o.state() == OpState::Done).await;
    let evs = rig.log();
    assert!(
        count(&evs, "claim_expired") >= 1,
        "the dead bot's claim must run out"
    );
    assert!(count(&evs, "requeued") >= 1);
    assert_no_double_claim(&evs);
    assert!(rig.op().ledger(&log_item()) >= 64);
    // The requeued job ran again and was dispatched twice.
    assert!(
        rig.op()
            .job_states()
            .iter()
            .any(|(_, _, attempts)| *attempts >= 2)
    );
}

#[tokio::test]
async fn higher_priority_take_preempts_and_old_owner_is_told() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    rig.goal(64).await;
    rig.run_until(200, |o| {
        o.job_states().iter().any(|j| j.1 == JobSt::InFlight)
    })
    .await;
    let (job, _, _) = rig
        .op()
        .job_states()
        .into_iter()
        .find(|j| j.1 == JobSt::InFlight)
        .unwrap();
    let claim = claim_id(&job, 1);
    let held = rig
        .op()
        .claims()
        .into_iter()
        .find(|(id, _)| *id == claim)
        .unwrap()
        .1;
    let tree = match held.kind {
        ClaimKind::BlockSet(l) => l,
        k => panic!("{k:?}"),
    };
    let claims = op_claims(&srv(), &opid());
    let take = ClaimRequest::Take {
        id: ClaimId::new("raid-1"),
        kind: ClaimKind::BlockSet(tree),
        owner: ClaimOwner::Squad(SquadId::new("raiders")),
        priority: Priority(5),
        ttl: Gt(60),
    };
    assert!(matches!(
        rig.bus
            .request_claim(&claims, take, WallMs(2000))
            .await
            .unwrap(),
        ClaimReply::Granted { .. }
    ));
    let renew = ClaimRequest::Renew {
        id: claim,
        owner: ClaimOwner::Job(job.clone()),
        ttl: Gt(100),
    };
    match rig
        .bus
        .request_claim(&claims, renew, WallMs(2000))
        .await
        .unwrap()
    {
        ClaimReply::Preempt { by, .. } => {
            assert_eq!(by, ClaimOwner::Squad(SquadId::new("raiders")))
        }
        r => panic!("old owner should hear Preempt, got {r:?}"),
    }
    // The squad's next renew gets Preempt too and halts the bot: the job ends `preempted`.
    rig.run_until(100, |o| {
        o.event_log()
            .iter()
            .any(|e| e.kind == "outcome" && e.data.contains("Preempted"))
    })
    .await;
    rig.run_until(6000, |o| o.state() == OpState::Done).await;
    let evs = rig.log();
    assert!(count(&evs, "claim_preempted") >= 1 && count(&evs, "requeued") >= 1);
    assert!(
        count(&evs, "claim_expired") >= 1,
        "the raiders' claim runs out on its own"
    );
    assert_no_double_claim(&evs);
}

#[tokio::test]
async fn killed_operation_restarts_from_log_with_ledger_intact() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    let initial: u32 = rig.trees.iter().map(|t| t.1).sum();
    rig.goal(64).await;
    rig.run_until(2000, |o| {
        o.ledger(&log_item()) >= 20 && o.job_states().iter().any(|j| j.1 == JobSt::InFlight)
    })
    .await;
    let before = rig.op().fingerprint();
    let ports = rig.op.take().unwrap().kill().await;
    rig.now += 7; // down for a few ticks; bots hold and finish their step
    for _ in 0..7 {
        for b in &mut rig.bots {
            b.tick(Gt(rig.now)).await;
        }
    }
    let log = rig.bus_log();
    rig.op = Some(Operation::restore(
        &rig.bus,
        rig.cfg.clone(),
        rig.trees.clone(),
        ports,
        &log,
        Gt(rig.now),
    ));
    assert_eq!(
        rig.op().fingerprint(),
        before,
        "restart rebuilds claims, jobs and ledger"
    );
    // A resent outcome (new receiver has no memory of keys) must not count twice.
    let had = rig.op().ledger(&log_item());
    let (key, bot, outcome, attempt) = rig
        .log()
        .into_iter()
        .rev()
        .find_map(|e| {
            if let OpEvent::Outcome {
                key,
                bot,
                outcome,
                attempt,
            } = e
            {
                Some((key, bot, outcome, attempt))
            } else {
                None
            }
        })
        .expect("an outcome was logged before the kill");
    rig.send(
        IntentBody::JobOutcome(JobOutcome {
            bot,
            outcome,
            tick: Gt(rig.now),
            attempt,
        }),
        "resent",
    )
    .await;
    rig.step().await;
    let same_key = rig
        .log()
        .iter()
        .filter(|e| matches!(e, OpEvent::Outcome { key: k, .. } if *k == key))
        .count();
    assert_eq!(same_key, 1, "the resent outcome is dropped");
    assert!(rig.op().ledger(&log_item()) >= had);
    rig.run_until(6000, |o| o.state() == OpState::Done).await;
    // Bots still felling when the goal was met report late; the ledger still takes them in.
    for _ in 0..150 {
        rig.step().await;
    }
    let evs = rig.log();
    assert_no_double_claim(&evs);
    let mut keys = std::collections::BTreeSet::new();
    let mut from_log = 0;
    for e in &evs {
        if let OpEvent::Outcome { key, outcome, .. } = e {
            if keys.insert(key.clone()) {
                from_log += outcome.report.items_gained.iter().map(|c| c.n).sum::<u32>();
            }
        }
    }
    let ledger = rig.op().ledger(&log_item());
    assert!(ledger >= had);
    assert_eq!(
        ledger, from_log,
        "ledger equals the distinct outcomes in the log"
    );
    let left: u32 = rig
        .world
        .lock()
        .unwrap()
        .tree_list()
        .iter()
        .map(|t| t.1)
        .sum();
    assert_eq!(
        ledger,
        initial - left,
        "every log felled in the world is in the ledger, none twice"
    );
}

#[tokio::test]
async fn jsonl_file_matches_bus_log_and_restores() {
    let path = std::env::temp_dir().join(format!("null-op-test-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut rig = Rig::new(5, &["b1", "b2"], 40).await;
    rig.op.take().unwrap().kill().await;
    rig.cfg.jsonl = Some(path.clone());
    // Fresh bus ports are gone with the killed op, so build the second op on a new bus.
    let bus = InProcBus::default();
    let mut op = Operation::new(&bus, rig.cfg.clone(), rig.trees.clone());
    let goal = GoalSpec {
        id: GoalId::new("g1"),
        goal: Goal::Obtain {
            item: log_item(),
            n: 10,
        },
        priority: Priority(1),
        reason: "t".into(),
    };
    bus.publish_intent(
        &op_intents(&srv(), &opid()),
        Intent {
            key: "g".into(),
            body: IntentBody::Goal {
                op: opid(),
                region: None,
                goal,
                quotas: vec![],
            },
        },
    )
    .await
    .unwrap();
    op.tick(Gt(1)).await;
    op.tick(Gt(2)).await;
    let from_file = load_jsonl(&path).unwrap();
    assert!(!from_file.is_empty());
    assert_eq!(from_file, op.event_log());
    let fp = op.fingerprint();
    let ports = op.kill().await;
    let again = Operation::restore(
        &bus,
        rig.cfg.clone(),
        rig.trees.clone(),
        ports,
        &from_file,
        Gt(2),
    );
    assert_eq!(again.fingerprint(), fp);
    let _ = std::fs::remove_file(&path);
}

/// Drive a stall: a bot goes quiet while its job runs, its claim expires, the job moves to another bot, then the
/// first bot wakes and reports late. Returns the rig, the job, and the stale bot's index, stopped at the moment the
/// stale outcome (attempt 1) reached the log. `halt`: wake it with a Halt (late `Preempted`), else it finishes.
async fn stale_outcome_rig(halt: bool) -> (Rig, JobId, usize) {
    // Returns the index of the NEW assignee; the caller resumes it (it is held slow so the stale report comes first).
    let mut rig = Rig::new(3, &["b1", "b2", "b3"], 40).await;
    rig.goal(64).await;
    for _ in 0..300 {
        if rig.squad.running.keys().any(|k| k.1 == 1) {
            break;
        }
        rig.step().await;
    }
    let ((job, _), (stale, _)) = rig
        .squad
        .running
        .iter()
        .next()
        .map(|(k, v)| (k.clone(), v.clone()))
        .unwrap();
    let idx = rig.bots.iter().position(|b| b.id == stale).unwrap();
    rig.bots[idx].stall(Gt(rig.now), Gt(u64::MAX));
    rig.squad.mute.insert(stale.clone());
    let again = job.clone();
    rig.run_until(600, |o| {
        o.job_states()
            .iter()
            .any(|(j, st, a)| *j == again && *st == JobSt::InFlight && *a == 2)
    })
    .await;
    // The new assignee works slowly (held), so the late report from the old one arrives while it still holds the tree.
    rig.step().await;
    let new_bot = rig
        .squad
        .running
        .get(&(job.clone(), 2))
        .expect("attempt 2 was handed to a bot")
        .0
        .clone();
    let nidx = rig.bots.iter().position(|b| b.id == new_bot).unwrap();
    assert_ne!(nidx, idx);
    rig.bots[nidx].stall(Gt(rig.now), Gt(u64::MAX));
    // The stalled bot's renew with its attempt-1 claim is denied: that claim ran out.
    let old = ClaimRequest::Renew {
        id: claim_id(&job, 1),
        owner: ClaimOwner::Job(job.clone()),
        ttl: Gt(100),
    };
    let reply = rig
        .bus
        .request_claim(&op_claims(&srv(), &opid()), old, WallMs(2000))
        .await
        .unwrap();
    assert_eq!(
        reply,
        ClaimReply::Denied {
            id: claim_id(&job, 1),
            reason: DenyReason::NotHeld
        }
    );
    // It wakes while attempt 2 is still being worked.
    rig.bots[idx].stall(Gt(0), Gt(0));
    if halt {
        rig.bus
            .send_command(
                &bot_cmd(&srv(), &stale),
                Command::Halt {
                    scope: Scope::Bot(stale.clone()),
                },
            )
            .unwrap();
    }
    for _ in 0..400 {
        if rig.log().iter().any(
            |e| matches!(e, OpEvent::Outcome { attempt: 1, outcome, .. } if outcome.job == job),
        ) {
            return (rig, job, nidx);
        }
        rig.step().await;
    }
    panic!("the stale bot never reported");
}

#[tokio::test]
async fn stalled_bot_late_success_does_not_touch_the_new_attempt() {
    let (mut rig, job, new_idx) = stale_outcome_rig(false).await;
    let n = rig
        .log()
        .iter()
        .position(|e| matches!(e, OpEvent::Outcome { attempt: 1, outcome, .. } if outcome.job == job))
        .unwrap();
    // The late outcome is in the log. The job is still attempt 2, in flight, under its own claim.
    assert!(
        rig.op()
            .job_states()
            .contains(&(job.clone(), JobSt::InFlight, 2)),
        "state: {:?}",
        rig.op().job_states()
    );
    assert!(
        rig.op()
            .claims()
            .iter()
            .any(|(id, c)| *id == claim_id(&job, 2) && c.owner == ClaimOwner::Job(job.clone()))
    );
    let after: Vec<OpEvent> = rig.log().split_off(n);
    assert!(!after.iter().any(|e| matches!(
        e,
        OpEvent::JobDone { .. }
            | OpEvent::Requeued { .. }
            | OpEvent::Claim(ClaimEvent::Released { .. })
    )));
    let late = rig.log().into_iter().find_map(|e| match e {
        OpEvent::Outcome {
            attempt: 1,
            outcome,
            ..
        } if outcome.job == job => Some(outcome.kind),
        _ => None,
    });
    assert_eq!(
        late,
        Some(OutcomeKind::Succeeded),
        "the late report is a success"
    );
    let before = rig.op().ledger(&log_item());
    assert!(before > 0, "the stale bot's logs are counted");
    rig.bots[new_idx].stall(Gt(0), Gt(0));
    rig.run_until(6000, |o| o.state() == OpState::Done).await;
    for _ in 0..200 {
        rig.step().await;
    }
    let evs = rig.log();
    assert_no_double_claim(&evs);
    let dispatches = evs
        .iter()
        .filter(|e| matches!(e, OpEvent::Dispatched { job: j, .. } if *j == job))
        .count();
    assert_state_moves_only_on_current_outcomes(&evs);
    assert_eq!(
        dispatches, 2,
        "never a third bot on the tree while the second works it"
    );
    let initial: u32 = rig.trees.iter().map(|t| t.1).sum();
    let left: u32 = rig
        .world
        .lock()
        .unwrap()
        .tree_list()
        .iter()
        .map(|t| t.1)
        .sum();
    assert_eq!(
        rig.op().ledger(&log_item()),
        initial - left,
        "every log felled is counted once"
    );
}

#[tokio::test]
async fn stale_preempted_outcome_does_not_requeue_the_new_attempt() {
    let (mut rig, job, new_idx) = stale_outcome_rig(true).await;
    let late = rig.log().into_iter().find_map(|e| match e {
        OpEvent::Outcome {
            attempt: 1,
            outcome,
            ..
        } if outcome.job == job => Some(outcome.kind),
        _ => None,
    });
    assert_eq!(late, Some(OutcomeKind::Preempted));
    assert!(
        rig.op()
            .job_states()
            .contains(&(job.clone(), JobSt::InFlight, 2)),
        "state: {:?}",
        rig.op().job_states()
    );
    assert!(
        rig.op()
            .claims()
            .iter()
            .any(|(id, _)| *id == claim_id(&job, 2)),
        "attempt 2 keeps its claim"
    );
    let evs = rig.log();
    let requeues = evs
        .iter()
        .filter(|e| matches!(e, OpEvent::Requeued { job: j, .. } if *j == job))
        .count();
    assert_eq!(requeues, 1, "only the expiry requeued it");
    assert_no_double_claim(&evs);
    rig.bots[new_idx].stall(Gt(0), Gt(0));
    rig.run_until(6000, |o| o.state() == OpState::Done).await;
}

#[tokio::test]
async fn resent_outcome_with_another_tick_counts_once() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    rig.goal(64).await;
    rig.run_until(2000, |o| o.ledger(&log_item()) >= 10).await;
    let (bot, outcome, attempt) = rig
        .log()
        .into_iter()
        .find_map(|e| {
            if let OpEvent::Outcome {
                bot,
                outcome,
                attempt,
                ..
            } = e
            {
                Some((bot, outcome, attempt))
            } else {
                None
            }
        })
        .unwrap();
    let had = rig.op().ledger(&log_item());
    for (i, tick) in [rig.now + 3, rig.now + 900].into_iter().enumerate() {
        rig.send(
            IntentBody::JobOutcome(JobOutcome {
                bot: bot.clone(),
                outcome: outcome.clone(),
                tick: Gt(tick),
                attempt,
            }),
            &format!("resend{i}"),
        )
        .await;
    }
    rig.step().await;
    let n = rig.log().iter().filter(|e| matches!(e, OpEvent::Outcome { outcome: o, attempt: a, .. } if o.job == outcome.job && *a == attempt)).count();
    assert_eq!(n, 1, "one counted outcome per (job, attempt)");
    assert!(rig.op().ledger(&log_item()) >= had);
    // Outcomes for a job or attempt the operation never handed out are ignored and logged, not counted.
    let mut bogus = outcome.clone();
    bogus.job = JobId::new("op1-j9999");
    rig.send(
        IntentBody::JobOutcome(JobOutcome {
            bot: bot.clone(),
            outcome: bogus,
            tick: Gt(1),
            attempt: 1,
        }),
        "bogus1",
    )
    .await;
    rig.send(
        IntentBody::JobOutcome(JobOutcome {
            bot: bot.clone(),
            outcome: outcome.clone(),
            tick: Gt(2),
            attempt: attempt + 7,
        }),
        "bogus2",
    )
    .await;
    rig.send(
        IntentBody::JobOutcome(JobOutcome {
            bot: BotId::new("stranger"),
            outcome: outcome.clone(),
            tick: Gt(3),
            attempt,
        }),
        "bogus3",
    )
    .await;
    rig.step().await;
    assert_eq!(count(&rig.log(), "ignored"), 3);
    let counted_bogus = rig.log().iter().any(|e| matches!(e, OpEvent::Outcome { bot: b, outcome: o, attempt: a, .. }
        if o.job == JobId::new("op1-j9999") || *a == attempt + 7 || *b == BotId::new("stranger")));
    assert!(!counted_bogus, "a bogus outcome was counted");
}

#[tokio::test]
async fn op_down_longer_than_the_claim_ttl_recovers_with_an_exact_ledger() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    let initial: u32 = rig.trees.iter().map(|t| t.1).sum();
    rig.goal(64).await;
    rig.run_until(2000, |o| {
        o.ledger(&log_item()) >= 10 && o.job_states().iter().any(|j| j.1 == JobSt::InFlight)
    })
    .await;
    let ports = rig.op.take().unwrap().kill().await;
    // Down for more than twice the claim TTL: nobody renews, the bots carry on and finish their trees.
    for _ in 0..250 {
        rig.now += 1;
        for b in &mut rig.bots {
            b.tick(Gt(rig.now)).await;
        }
    }
    let log = rig.bus_log();
    rig.op = Some(Operation::restore(
        &rig.bus,
        rig.cfg.clone(),
        rig.trees.clone(),
        ports,
        &log,
        Gt(rig.now),
    ));
    rig.run_until(8000, |o| o.state() == OpState::Done).await;
    for _ in 0..300 {
        rig.step().await;
    }
    let evs = rig.log();
    assert!(
        count(&evs, "claim_expired") >= 1,
        "claims ran out while the operation was down"
    );
    assert!(count(&evs, "requeued") >= 1, "their jobs were requeued");
    assert_no_double_claim(&evs);
    assert_state_moves_only_on_current_outcomes(&evs);
    let left: u32 = rig
        .world
        .lock()
        .unwrap()
        .tree_list()
        .iter()
        .map(|t| t.1)
        .sum();
    assert_eq!(
        rig.op().ledger(&log_item()),
        initial - left,
        "ledger equals logs removed from the world"
    );
}

async fn full_run(seed: u64) -> Vec<LogEvent> {
    let mut rig = Rig::new(seed, &["b1", "b2"], 40).await;
    rig.bots[1].die_at(Gt(60));
    rig.goal(64).await;
    rig.run_until(8000, |o| o.state() == OpState::Done).await;
    rig.bus_log()
}

#[tokio::test]
async fn same_seed_gives_the_same_event_log() {
    let a = full_run(11).await;
    assert_eq!(a, full_run(11).await);
    assert_ne!(
        a,
        full_run(12).await,
        "a different seed is a different world"
    );
}

#[tokio::test]
async fn too_few_trees_blocks_and_escalates() {
    let mut rig = Rig::new(3, &["b1"], 3).await;
    let mut esc = rig.bus.subscribe_escalations(&hq_escalate(&srv())).unwrap();
    rig.goal(64).await;
    rig.step().await;
    assert_eq!(rig.op().state(), OpState::Blocked);
    let e = esc.try_recv().expect("escalation");
    assert_eq!(e.kind, "quota_infeasible");
}

#[tokio::test]
async fn halt_stops_new_dispatch() {
    let mut rig = Rig::new(3, &["b1", "b2"], 40).await;
    rig.goal(64).await;
    rig.run_until(50, |o| {
        o.job_states().iter().any(|j| j.1 == JobSt::InFlight)
    })
    .await;
    rig.bus
        .send_command(
            &op_cmd(&srv(), &opid()),
            Command::Halt {
                scope: Scope::Operation(opid()),
            },
        )
        .unwrap();
    rig.step().await;
    let n = count(&rig.log(), "dispatched");
    for _ in 0..200 {
        rig.step().await;
    }
    assert_eq!(rig.op().state(), OpState::Halted);
    assert_eq!(count(&rig.log(), "dispatched"), n);
}

mod registry {
    use crate::ClaimRegistry;
    use null_proto::*;

    fn tree(x: i32) -> Vec<Loc> {
        vec![Loc {
            server: ServerId::new("s"),
            dim: Dimension::Overworld,
            pos: Pos::new(x, 64, 0),
        }]
    }
    fn take(id: &str, who: &str, prio: u8, x: i32, ttl: u64) -> ClaimRequest {
        ClaimRequest::Take {
            id: ClaimId::new(id),
            kind: ClaimKind::BlockSet(tree(x)),
            owner: ClaimOwner::Squad(SquadId::new(who)),
            priority: Priority(prio),
            ttl: Gt(ttl),
        }
    }
    fn owner(who: &str) -> ClaimOwner {
        ClaimOwner::Squad(SquadId::new(who))
    }

    #[test]
    fn equal_priority_is_denied_higher_preempts() {
        let mut r = ClaimRegistry::default();
        assert!(matches!(
            r.handle(Gt(0), &take("a", "x", 1, 5, 100)),
            ClaimReply::Granted { token: 1, .. }
        ));
        assert!(matches!(
            r.handle(Gt(1), &take("b", "y", 1, 5, 100)),
            ClaimReply::Denied {
                reason: DenyReason::Held { .. },
                ..
            }
        ));
        assert!(
            matches!(
                r.handle(Gt(1), &take("c", "z", 1, 6, 100)),
                ClaimReply::Granted { .. }
            ),
            "other tree is free"
        );
        let g = r.handle(Gt(2), &take("b", "y", 2, 5, 100));
        assert!(
            matches!(g, ClaimReply::Granted { token: 3, .. }),
            "tokens grow with each grant: {g:?}"
        );
        let renew = ClaimRequest::Renew {
            id: ClaimId::new("a"),
            owner: owner("x"),
            ttl: Gt(10),
        };
        assert_eq!(
            r.handle(Gt(3), &renew),
            ClaimReply::Preempt {
                id: ClaimId::new("a"),
                by: owner("y")
            }
        );
    }

    #[test]
    fn claim_frees_itself_and_renew_extends() {
        let mut r = ClaimRegistry::default();
        r.handle(Gt(0), &take("a", "x", 1, 5, 10));
        let renew = ClaimRequest::Renew {
            id: ClaimId::new("a"),
            owner: owner("x"),
            ttl: Gt(10),
        };
        assert!(matches!(
            r.handle(Gt(8), &renew),
            ClaimReply::Granted {
                expires_at: Gt(18),
                ..
            }
        ));
        assert!(
            matches!(
                r.handle(Gt(12), &take("b", "y", 1, 5, 10)),
                ClaimReply::Denied { .. }
            ),
            "still held after the renew"
        );
        assert!(
            matches!(
                r.handle(Gt(18), &take("b", "y", 1, 5, 10)),
                ClaimReply::Granted { .. }
            ),
            "expired claim is free"
        );
        let ev = r.drain_events();
        assert!(
            ev.iter()
                .any(|e| matches!(e, crate::ClaimEvent::Expired { .. }))
        );
        assert_eq!(
            r.handle(Gt(19), &renew),
            ClaimReply::Denied {
                id: ClaimId::new("a"),
                reason: DenyReason::NotHeld
            }
        );
    }

    #[test]
    fn retake_of_a_live_id_by_its_owner_renews_it() {
        let mut r = ClaimRegistry::default();
        assert!(matches!(
            r.handle(Gt(0), &take("a", "x", 1, 5, 10)),
            ClaimReply::Granted { token: 1, .. }
        ));
        // A resend: same owner, same id, same block set. Same token, longer expiry, no second grant.
        assert!(matches!(
            r.handle(Gt(4), &take("a", "x", 1, 5, 10)),
            ClaimReply::Granted {
                token: 1,
                expires_at: Gt(14),
                ..
            }
        ));
        // Same id, another block set: denied, nothing replaced.
        assert!(matches!(
            r.handle(Gt(5), &take("a", "x", 1, 6, 10)),
            ClaimReply::Denied {
                reason: DenyReason::Held { .. },
                ..
            }
        ));
        let grants = r
            .drain_events()
            .iter()
            .filter(|e| matches!(e, crate::ClaimEvent::Granted { .. }))
            .count();
        assert_eq!((grants, r.len()), (1, 1));
    }

    #[test]
    fn same_owner_new_id_on_a_held_tree_is_denied() {
        // An old attempt's take resent under another id must not hold the tree a second time.
        let mut r = ClaimRegistry::default();
        assert!(matches!(r.handle(Gt(0), &take("j1-a1", "x", 1, 5, 10)), ClaimReply::Granted { .. }));
        assert!(matches!(
            r.handle(Gt(1), &take("j1-a2", "x", 1, 5, 10)),
            ClaimReply::Denied { reason: DenyReason::Held { .. }, .. }
        ));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn release_checks_owner() {
        let mut r = ClaimRegistry::default();
        r.handle(Gt(0), &take("a", "x", 1, 5, 10));
        let bad = ClaimRequest::Release {
            id: ClaimId::new("a"),
            owner: owner("y"),
        };
        assert_eq!(
            r.handle(Gt(1), &bad),
            ClaimReply::Denied {
                id: ClaimId::new("a"),
                reason: DenyReason::NotOwner
            }
        );
        let ok = ClaimRequest::Release {
            id: ClaimId::new("a"),
            owner: owner("x"),
        };
        assert_eq!(
            r.handle(Gt(1), &ok),
            ClaimReply::Released {
                id: ClaimId::new("a")
            }
        );
        assert!(r.is_empty());
    }

    #[test]
    fn replaying_events_rebuilds_the_registry() {
        let mut r = ClaimRegistry::default();
        r.handle(Gt(0), &take("a", "x", 1, 5, 10));
        r.handle(Gt(1), &take("b", "y", 2, 5, 50));
        r.handle(Gt(2), &take("c", "z", 1, 9, 50));
        let mut copy = ClaimRegistry::default();
        for e in r.drain_events() {
            copy.apply(&e);
        }
        assert_eq!(copy.snapshot(), r.snapshot());
    }
}
