//! Squad tests with scripted bots and a fake claim server. Spec: specs/K6-squad-leader.md §8
use crate::*;
use null_bus::{Bus, CommandRx, InProcBus, IntentRx};
use null_op::{op_claims, op_intents, squad_jobs};
use null_proto::*;
use null_simbot::{bot_cmd, bot_jobs, bot_summary, bot_up};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

fn srv() -> ServerId {
    ServerId::new("smp1")
}
fn sq() -> SquadId {
    SquadId::new("sq1")
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Grant,
    Deny,
    Preempt,
    /// Never answers.
    Silent,
    /// Grants a lease that ends 5 ticks on.
    Short,
}

/// Stands in for the operation's claim service: answers renews by mode and counts them.
struct FakeClaims {
    mode: Arc<Mutex<Mode>>,
    clock: Arc<AtomicU64>,
    seen: Arc<Mutex<Vec<ClaimId>>>,
}

fn fake_claims(bus: &InProcBus) -> FakeClaims {
    let f = FakeClaims { mode: Arc::new(Mutex::new(Mode::Grant)), clock: Arc::new(AtomicU64::new(0)), seen: Arc::default() };
    let mut server = bus.serve_claims(&op_claims(&srv(), &OperationId::new("op1"))).unwrap();
    let (mode, clock, seen) = (f.mode.clone(), f.clock.clone(), f.seen.clone());
    tokio::spawn(async move {
        while let Some(call) = server.next().await {
            let id = call.req.id().clone();
            seen.lock().unwrap().push(id.clone());
            let m = *mode.lock().unwrap();
            match m {
                Mode::Grant => call.reply(ClaimReply::Granted { id, token: 1, expires_at: Gt(clock.load(Ordering::SeqCst) + 100) }),
                Mode::Short => call.reply(ClaimReply::Granted { id, token: 1, expires_at: Gt(clock.load(Ordering::SeqCst) + 5) }),
                Mode::Deny => call.reply(ClaimReply::Denied { id, reason: DenyReason::NotHeld }),
                Mode::Preempt => call.reply(ClaimReply::Preempt { id, by: ClaimOwner::Squad(SquadId::new("other")) }),
                Mode::Silent => std::mem::forget(call),
            }
        }
    });
    f
}

/// What a scripted bot received.
struct Bot {
    id: BotId,
    jobs: IntentRx,
    cmds: CommandRx,
}

impl Bot {
    fn new(bus: &InProcBus, id: &str) -> Self {
        let id = BotId::new(id);
        Self { jobs: bus.subscribe_intents(&bot_jobs(&srv(), &id)).unwrap(), cmds: bus.subscribe_commands(&bot_cmd(&srv(), &id)), id }
    }
    fn bodies(&mut self) -> Vec<IntentBody> {
        std::iter::from_fn(|| self.jobs.try_recv()).map(|i| i.body).collect()
    }
    fn halted(&mut self) -> bool {
        std::iter::from_fn(|| self.cmds.try_recv()).any(|c| matches!(c, Command::Halt { .. }))
    }
    fn state(&self, bus: &InProcBus, tick: u64, life: Life, x: i32) {
        let at_loc = Some(Loc { server: srv(), dim: Dimension::Overworld, pos: Pos::new(x, 64, 0) });
        let s = BotState { bot: self.id.clone(), life, at_loc, health: 20.0, job: None, tick: Gt(tick) };
        bus.publish_summary(&bot_summary(&srv(), &self.id), SummaryBody::Bot(s));
    }
    async fn send(&self, bus: &InProcBus, key: &str, body: IntentBody) {
        bus.publish_intent(&bot_up(&srv(), &self.id), Intent { key: key.into(), body }).await.unwrap();
    }
    async fn outcome(&self, bus: &InProcBus, job: &Job, kind: OutcomeKind, key: &str) {
        let outcome = Outcome { job: job.id.clone(), kind, report: Report::default() };
        let body = IntentBody::JobOutcome(JobOutcome { bot: self.id.clone(), outcome, tick: Gt(0), attempt: job.attempt });
        self.send(bus, key, body).await;
    }
}

struct Rig {
    bus: InProcBus,
    claims: FakeClaims,
    squad: Squad<InProcBus>,
    bots: Vec<Bot>,
    up: IntentRx,
    now: u64,
}

impl Rig {
    fn new(names: &[&str]) -> Self {
        let bus = InProcBus::default();
        let claims = fake_claims(&bus);
        let mut cfg = SquadConfig::new("smp1", "sq1", "op1", names);
        cfg.renew_timeout = WallMs(50);
        let squad = Squad::new(&bus, cfg);
        let bots: Vec<Bot> = names.iter().map(|n| Bot::new(&bus, n)).collect();
        let up = bus.subscribe_intents(&op_intents(&srv(), &OperationId::new("op1"))).unwrap();
        let r = Self { bus, claims, squad, bots, up, now: 0 };
        r.publish_states(&[]);
        r
    }
    fn publish_states(&self, skip: &[usize]) {
        for (i, b) in self.bots.iter().enumerate() {
            if !skip.contains(&i) {
                b.state(&self.bus, self.now, Life::Alive, i as i32 * 30);
            }
        }
    }
    async fn tick(&mut self, n: u64, skip: &[usize]) {
        for _ in 0..n {
            self.now += 1;
            self.claims.clock.store(self.now, Ordering::SeqCst);
            self.publish_states(skip);
            self.squad.tick(Gt(self.now)).await;
        }
    }
    async fn batch(&self, jobs: Vec<Job>) {
        let body = IntentBody::JobBatch { squad: sq(), jobs: jobs.clone() };
        let key = format!("b:{}", jobs[0].id);
        self.bus.publish_intent(&squad_jobs(&srv(), &sq()), Intent { key, body }).await.unwrap();
    }
    fn to_op(&mut self) -> Vec<IntentBody> {
        std::iter::from_fn(|| self.up.try_recv()).map(|i| i.body).collect()
    }
}

fn job(id: &str, x: i32, attempt: u32) -> Job {
    let loc = Loc { server: srv(), dim: Dimension::Overworld, pos: Pos::new(x, 64, 0) };
    Job {
        id: JobId::new(id),
        kind: "fell".into(),
        params: BTreeMap::from([("trees".to_string(), ParamValue::Locs(vec![loc]))]),
        items: vec![],
        claims: vec![ClaimId::new(format!("claim-{id}-a{attempt}"))],
        preconditions: vec![],
        success: Condition::Always,
        budget: Budget { ticks: Gt(500), attempts: 1 },
        priority: Priority(1),
        reflexes: vec![],
        checkpoint: Checkpoint::None,
        preemption_floor: None,
        attempt,
    }
}

fn is_batch(b: &IntentBody) -> Option<&Job> {
    match b {
        IntentBody::JobBatch { jobs, .. } => jobs.first(),
        _ => None,
    }
}

#[tokio::test]
async fn jobs_go_to_the_nearest_idle_bot_one_each() {
    // bot0 stands at x=0, bot1 at x=30.
    let mut r = Rig::new(&["b0", "b1"]);
    r.batch(vec![job("j1", 29, 1), job("j2", 1, 1), job("j3", 15, 1)]).await;
    r.tick(1, &[]).await;
    let (a, b) = (r.bots[0].bodies(), r.bots[1].bodies());
    assert_eq!(a.iter().filter_map(is_batch).map(|j| j.id.as_str()).collect::<Vec<_>>(), ["j2"]);
    assert_eq!(b.iter().filter_map(is_batch).map(|j| j.id.as_str()).collect::<Vec<_>>(), ["j1"]);
    assert!(matches!(a[0], IntentBody::Lease(_)), "the lease comes before the job");
    assert_eq!(r.squad.queued(), 1, "j3 waits: both bots are busy");
}

#[tokio::test]
async fn a_third_job_waits_for_a_free_bot() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.batch(vec![job("j1", 29, 1), job("j2", 1, 1), job("j3", 15, 1)]).await;
    r.tick(3, &[]).await;
    assert_eq!(r.squad.queued(), 1);
    let j2 = job("j2", 1, 1);
    r.bots[0].outcome(&r.bus, &j2, OutcomeKind::Succeeded, "o1").await;
    r.tick(1, &[]).await;
    assert_eq!(r.squad.queued(), 0);
    assert!(r.bots[0].bodies().iter().filter_map(is_batch).any(|j| j.id.as_str() == "j3"));
}

#[tokio::test]
async fn a_job_whose_claim_is_gone_is_never_started() {
    let mut r = Rig::new(&["b0"]);
    *r.claims.mode.lock().unwrap() = Mode::Deny;
    r.batch(vec![job("j1", 5, 1)]).await;
    r.tick(2, &[]).await;
    assert!(r.bots[0].bodies().is_empty());
    assert!(r.squad.running().is_empty());
    // The operation hears of it at once (a failed outcome for that attempt), so it requeues without waiting.
    let up: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o) } else { None }).collect();
    assert_eq!(up.len(), 1);
    assert_eq!((up[0].outcome.job.as_str(), up[0].attempt, up[0].bot.as_str()), ("j1", 1, "b0"));
    assert!(matches!(up[0].outcome.kind, OutcomeKind::Failed(_)));
}

async fn failed_renew_halts(mode: Mode) {
    let mut r = Rig::new(&["b0"]);
    r.batch(vec![job("j1", 5, 1)]).await;
    r.tick(1, &[]).await;
    assert_eq!(r.squad.running().len(), 1);
    r.bots[0].bodies();
    *r.claims.mode.lock().unwrap() = mode;
    r.tick(21, &[]).await;
    assert!(r.bots[0].halted(), "a failed renew halts the bot");
    assert!(r.squad.running().is_empty());
    let n = r.claims.seen.lock().unwrap().len();
    r.tick(60, &[]).await;
    assert_eq!(r.claims.seen.lock().unwrap().len(), n, "no more renews for a dropped job");
}

#[tokio::test]
async fn denied_renew_halts_the_bot() {
    failed_renew_halts(Mode::Deny).await;
}
#[tokio::test]
async fn preempted_renew_halts_the_bot() {
    failed_renew_halts(Mode::Preempt).await;
}
#[tokio::test]
async fn unanswered_renew_halts_the_bot() {
    failed_renew_halts(Mode::Silent).await;
}

#[tokio::test]
async fn renews_use_the_jobs_own_claim_id_and_push_a_later_lease() {
    let mut r = Rig::new(&["b0"]);
    r.batch(vec![job("j1", 5, 3)]).await;
    r.tick(45, &[]).await;
    let seen = r.claims.seen.lock().unwrap().clone();
    assert!(seen.len() >= 3 && seen.iter().all(|c| c.as_str() == "claim-j1-a3"));
    let leases: Vec<u64> = r.bots[0]
        .bodies()
        .into_iter()
        .filter_map(|b| if let IntentBody::Lease(l) = b { Some(l.expires_at.0) } else { None })
        .collect();
    assert!(leases.windows(2).all(|w| w[0] < w[1]), "each lease runs later: {leases:?}");
}

#[tokio::test]
async fn a_silent_bot_is_not_renewed_is_halted_and_reported_lost() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.batch(vec![job("j1", 1, 1)]).await;
    r.tick(5, &[]).await;
    assert_eq!(r.squad.running()[0].0.as_str(), "b0");
    let before = r.claims.seen.lock().unwrap().len();
    r.tick(60, &[0]).await; // b0 stops publishing state
    assert_eq!(r.squad.lost(), vec![BotId::new("b0")]);
    assert!(r.squad.running().is_empty());
    assert!(r.bots[0].halted(), "the halt waits for the bot to wake");
    assert!(r.claims.seen.lock().unwrap().len() <= before + 1, "renewing stopped");
    let roster = r.to_op().into_iter().find_map(|b| if let IntentBody::Roster { lost, .. } = b { Some(lost) } else { None });
    assert_eq!(roster, Some(vec![BotId::new("b0")]));
    r.tick(2, &[]).await; // it speaks again
    assert!(r.squad.lost().is_empty());
}

#[tokio::test]
async fn a_dead_bot_is_lost_and_gets_no_work() {
    let mut r = Rig::new(&["b0"]);
    r.bots[0].state(&r.bus, 1, Life::Dead, 0);
    r.batch(vec![job("j1", 1, 1)]).await;
    r.now = 1;
    r.squad.tick(Gt(1)).await;
    assert_eq!(r.squad.lost().len(), 1);
    assert!(r.squad.running().is_empty());
    assert_eq!(r.squad.queued(), 0, "no bot is free, so the job is reported up, not queued (H4)");
    let up: Vec<_> = r.to_op().into_iter().filter(|b| matches!(b, IntentBody::JobOutcome(o) if o.outcome.kind == OutcomeKind::Preempted)).collect();
    assert_eq!(up.len(), 1);
}

#[tokio::test]
async fn a_transient_failure_runs_again_on_the_same_bot_then_goes_up_with_what_was_tried() {
    let mut r = Rig::new(&["b0"]);
    let j = job("j1", 5, 1);
    r.batch(vec![j.clone()]).await;
    r.tick(1, &[]).await;
    r.bots[0].bodies();
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Failed(FailReason::NoPath), "o1").await;
    r.tick(1, &[]).await;
    assert!(r.to_op().iter().all(|b| !matches!(b, IntentBody::JobOutcome(_))), "first failure stays local");
    let again = r.bots[0].bodies();
    assert!(again.iter().filter_map(is_batch).any(|x| x.id == j.id && x.attempt == 1), "same job, same attempt");
    assert!(again.iter().any(|b| matches!(b, IntentBody::Lease(_))), "the lease is pushed again");
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Failed(FailReason::NoPath), "o2").await;
    r.tick(1, &[]).await;
    let up: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o) } else { None }).collect();
    assert_eq!(up.len(), 1);
    assert!(up[0].outcome.report.note.contains("earlier"), "{}", up[0].outcome.report.note);
    assert!(r.squad.running().is_empty());
}

#[tokio::test]
async fn success_after_a_retry_goes_up_once_and_other_failures_go_up_at_once() {
    let mut r = Rig::new(&["b0"]);
    let j = job("j1", 5, 1);
    r.batch(vec![j.clone(), job("j2", 6, 1)]).await;
    r.tick(1, &[]).await;
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Failed(FailReason::Other("x".into())), "o1").await;
    r.tick(1, &[]).await;
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Succeeded, "o2").await;
    r.tick(1, &[]).await;
    let ups: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o.outcome.kind) } else { None }).collect();
    assert_eq!(ups, vec![OutcomeKind::Succeeded]);
    // j2 now runs; a vanished tree is not transient.
    let j2 = job("j2", 6, 1);
    r.bots[0].outcome(&r.bus, &j2, OutcomeKind::Failed(FailReason::TargetGone), "o3").await;
    r.tick(1, &[]).await;
    let ups: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o.outcome.kind) } else { None }).collect();
    assert_eq!(ups, vec![OutcomeKind::Failed(FailReason::TargetGone)]);
}

fn setting(t: ReflexTarget, on: bool) -> ReflexSetting {
    ReflexSetting { target: t, on }
}
fn reflex_msgs(b: Vec<IntentBody>) -> Vec<Vec<ReflexSetting>> {
    b.into_iter().filter_map(|b| if let IntentBody::Command(Command::SetReflexes { settings, .. }) = b { Some(settings) } else { None }).collect()
}

#[tokio::test]
async fn a_jobs_reflex_settings_apply_at_start_and_revert_to_the_inherited_ones_at_end() {
    let mut r = Rig::new(&["b0"]);
    let combat = ReflexTarget::Group(ReflexGroup::Combat);
    let all = ReflexTarget::All;
    // The squad itself runs with `all` off.
    let sq_set = Command::SetReflexes { scope: Scope::Squad(sq()), settings: vec![setting(all.clone(), false)], reason: "t".into() };
    r.bus.publish_intent(&squad_jobs(&srv(), &sq()), Intent { key: "s1".into(), body: IntentBody::Command(sq_set) }).await.unwrap();
    r.tick(1, &[]).await;
    r.bots[0].bodies();
    let mut j = job("j1", 5, 1);
    j.reflexes = vec![setting(combat.clone(), false), setting(all.clone(), true)];
    r.batch(vec![j.clone()]).await;
    r.tick(1, &[]).await;
    let b = r.bots[0].bodies();
    let at = b.iter().position(|b| matches!(b, IntentBody::Command(Command::SetReflexes { .. }))).expect("applied");
    let job_at = b.iter().position(|b| is_batch(b).is_some()).unwrap();
    assert!(at < job_at, "reflexes are set before the job starts");
    assert_eq!(reflex_msgs(b), vec![j.reflexes.clone()]);
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Succeeded, "o1").await;
    r.tick(1, &[]).await;
    // `all` goes back to the squad's off; `combat` has no squad setting, so it goes back to on.
    assert_eq!(reflex_msgs(r.bots[0].bodies()), vec![vec![setting(combat, true), setting(all, false)]]);
}

#[tokio::test]
async fn the_same_observation_from_three_bots_goes_up_once() {
    let mut r = Rig::new(&["b0", "b1", "b2"]);
    let at = Loc { server: srv(), dim: Dimension::Overworld, pos: Pos::new(3, 64, 3) };
    for (i, b) in r.bots.iter().enumerate() {
        let o = Observation { observer: b.id.clone(), tick: Gt(i as u64), what: Observed::Block { at: at.clone(), block: ItemId::new("minecraft:oak_log") } };
        b.send(&r.bus, &format!("k{i}"), IntentBody::Observation(o)).await;
    }
    r.tick(1, &[]).await;
    let n = r.to_op().iter().filter(|b| matches!(b, IntentBody::Observation(_))).count();
    assert_eq!(n, 1);
}

#[tokio::test]
async fn halt_stops_bots_and_new_work_in_the_same_tick_recall_only_stops_new_work() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.batch(vec![job("j1", 1, 1), job("j2", 29, 1), job("j3", 15, 1)]).await;
    r.tick(2, &[]).await;
    r.bots.iter_mut().for_each(|b| {
        b.bodies();
    });
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Recall { scope: Scope::Swarm }).unwrap();
    r.tick(1, &[]).await;
    assert!(r.squad.is_stopped());
    assert!(!r.bots[0].halted() && !r.bots[1].halted(), "recall leaves running bots alone");
    assert_eq!(r.squad.running().len(), 2);
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Swarm }).unwrap();
    r.bots[0].outcome(&r.bus, &job("j1", 1, 1), OutcomeKind::Preempted, "p1").await;
    r.tick(1, &[]).await;
    assert!(r.bots[0].halted() && r.bots[1].halted());
    assert!(r.squad.running().is_empty());
    assert_eq!(r.squad.queued(), 0);
    assert!(r.to_op().iter().any(|b| matches!(b, IntentBody::JobOutcome(_))), "the halted bot's outcome still goes up");
}

#[tokio::test]
async fn a_bot_scope_halt_holds_one_bot_only() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Bot(BotId::new("b0")) }).unwrap();
    r.batch(vec![job("j1", 1, 1)]).await;
    r.tick(2, &[]).await;
    assert!(r.bots[0].halted());
    assert!(!r.squad.is_stopped());
    assert_eq!(r.squad.running()[0].0.as_str(), "b1", "b0 is nearest but held");
}

#[tokio::test]
async fn kept_outcomes_are_sent_again_under_new_keys() {
    let mut r = Rig::new(&["b0"]);
    let j = job("j1", 5, 1);
    r.batch(vec![j.clone()]).await;
    r.tick(1, &[]).await;
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Succeeded, "o1").await;
    r.tick(1, &[]).await;
    assert_eq!(r.to_op().iter().filter(|b| matches!(b, IntentBody::JobOutcome(_))).count(), 1);
    r.squad.resend_outcomes().await;
    assert_eq!(r.to_op().iter().filter(|b| matches!(b, IntentBody::JobOutcome(_))).count(), 1);
}

fn reflex_job(id: &str) -> Job {
    let mut j = job(id, 1, 1);
    j.reflexes = vec![setting(ReflexTarget::Group(ReflexGroup::Combat), false)];
    j
}

/// K6 §4.7: a job that turned combat off must turn it back on even when its bot went silent or dead.
#[tokio::test]
async fn a_lost_bots_reflexes_are_restored_after_the_halt() {
    for dead in [false, true] {
        let mut r = Rig::new(&["b0"]);
        r.batch(vec![reflex_job("j1")]).await;
        r.tick(3, &[]).await;
        assert_eq!(r.squad.running().len(), 1);
        r.bots[0].bodies();
        if dead {
            r.now += 1;
            r.bots[0].state(&r.bus, r.now, Life::Dead, 0);
            r.squad.tick(Gt(r.now)).await;
        } else {
            r.tick(40, &[0]).await;
        }
        assert_eq!(r.squad.lost().len(), 1);
        assert!(r.squad.running().is_empty());
        let back = reflex_msgs(r.bots[0].bodies());
        assert_eq!(back, vec![vec![setting(ReflexTarget::Group(ReflexGroup::Combat), true)]], "dead={dead}");
        assert_eq!(r.bots[0].halted(), !dead, "a silent bot is halted; a dead one needs no halt");
    }
}

/// K1 §4.2: retiring a squad halts its running bots and puts their reflexes back.
#[tokio::test]
async fn a_retired_squad_halts_its_bots_and_restores_their_reflexes() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.batch(vec![reflex_job("j1")]).await;
    r.tick(3, &[]).await;
    r.bots[0].bodies();
    r.squad.begin_retire().await;
    assert!(r.bots[0].halted() && r.bots[1].halted(), "every bot is halted");
    assert_eq!(reflex_msgs(r.bots[0].bodies()).len(), 1);
    let (_ports, _) = r.squad.finish_retire();
}

/// H3: a retiring squad keeps forwarding the halted bot's final outcome, with its logs, and is done only then.
#[tokio::test]
async fn a_retiring_squad_forwards_the_halted_bots_last_outcome() {
    let mut r = Rig::new(&["b0"]);
    let j = job("j1", 5, 1);
    r.batch(vec![j.clone()]).await;
    r.tick(3, &[]).await;
    r.to_op();
    r.squad.begin_retire().await;
    assert!(!r.squad.retire_done(), "the bot's last outcome is still owed");
    let report = Report { items_gained: vec![ItemCount { item: ItemId::new("minecraft:oak_log"), n: 3 }], ..Report::default() };
    let outcome = Outcome { job: j.id.clone(), kind: OutcomeKind::Preempted, report };
    let body = IntentBody::JobOutcome(JobOutcome { bot: r.bots[0].id.clone(), outcome, tick: Gt(4), attempt: 1 });
    r.bots[0].send(&r.bus, "late", body).await;
    r.tick(1, &[]).await;
    let up: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o) } else { None }).collect();
    assert_eq!(up.len(), 1, "forwarded once");
    assert_eq!(up[0].outcome.report.items_gained[0].n, 3);
    assert!(r.squad.retire_done());
    assert!(r.squad.finish_retire().1.iter().all(|e| e.kind != "retire_unreported"));
}

/// H3: a bot that never reports does not hold retire past the bound; the missing (job, attempt) is recorded.
#[tokio::test]
async fn a_silent_bot_does_not_hold_retire_past_the_bound() {
    let mut r = Rig::new(&["b0"]);
    r.batch(vec![job("j1", 5, 1)]).await;
    r.tick(3, &[]).await;
    r.squad.begin_retire().await;
    r.tick(crate::squad::RETIRE_WAIT.0 - 2, &[]).await;
    assert!(!r.squad.retire_done(), "still inside the bound");
    r.tick(2, &[]).await;
    assert!(r.squad.retire_done(), "the bound passed");
    let (_, ex) = r.squad.finish_retire();
    assert!(ex.iter().any(|e| e.kind == "retire_unreported" && e.subject == "b0 j1 a1"), "{ex:?}");
}

/// H4: a one-bot squad whose bot a bot-scope halt holds reports each new job up as Preempted instead of queueing.
#[tokio::test]
async fn a_batch_on_an_all_held_squad_is_reported_preempted() {
    let mut r = Rig::new(&["b0"]);
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Bot(BotId::new("b0")) }).unwrap();
    r.tick(1, &[]).await;
    assert!(!r.squad.is_stopped(), "a bot-scope halt is not a squad stop");
    r.batch(vec![job("j1", 5, 1), job("j2", 6, 1)]).await;
    r.tick(1, &[]).await;
    assert_eq!(r.squad.queued(), 0, "nothing waits forever");
    let up: Vec<_> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o) } else { None }).collect();
    assert_eq!(up.len(), 2);
    assert!(up.iter().all(|o| o.outcome.kind == OutcomeKind::Preempted && o.outcome.report.note == "no free bot: all held"));
}

/// H4: with one of two bots held, a job still runs on the other.
#[tokio::test]
async fn a_job_still_runs_on_the_free_bot_when_one_is_held() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Bot(BotId::new("b0")) }).unwrap();
    r.tick(1, &[]).await;
    r.batch(vec![job("j1", 1, 1)]).await;
    r.tick(1, &[]).await;
    assert_eq!(r.squad.running(), vec![(BotId::new("b1"), JobId::new("j1"), 1)]);
    assert!(r.to_op().iter().all(|b| !matches!(b, IntentBody::JobOutcome(_))));
}

/// A retry runs under the old lease; with that lease about to end the failure goes up instead.
#[tokio::test]
async fn no_local_retry_under_a_lease_that_is_nearly_out() {
    let mut r = Rig::new(&["b0"]);
    *r.claims.mode.lock().unwrap() = Mode::Short;
    let j = job("j1", 5, 1);
    r.batch(vec![j.clone()]).await;
    r.tick(1, &[]).await;
    r.bots[0].bodies();
    r.bots[0].outcome(&r.bus, &j, OutcomeKind::Failed(FailReason::NoPath), "o1").await;
    r.tick(1, &[]).await;
    assert!(r.bots[0].bodies().iter().filter_map(is_batch).next().is_none(), "the job was not sent again");
    let up = r.to_op().into_iter().filter(|b| matches!(b, IntentBody::JobOutcome(_))).count();
    assert_eq!(up, 1, "the failure goes up");
}

/// A squad with one real `SimBot` on the same bus, for tests that need the bot's own lease rule.
struct Live {
    bus: InProcBus,
    claims: FakeClaims,
    squad: Squad<InProcBus>,
    bot: null_simbot::SimBot<InProcBus>,
    world: null_simbot::SharedWorld,
    up: IntentRx,
    now: u64,
}

impl Live {
    fn new(trees: usize, extent: i32) -> Self {
        let bus = InProcBus::default();
        let claims = fake_claims(&bus);
        let world = null_simbot::SimWorld::generate(5, srv(), trees, extent).shared();
        let mut cfg = SquadConfig::new("smp1", "sq1", "op1", &["b0"]);
        cfg.renew_timeout = WallMs(50);
        let squad = Squad::new(&bus, cfg);
        let bot = null_simbot::SimBot::new(&bus, &world, "b0", 5);
        let up = bus.subscribe_intents(&op_intents(&srv(), &OperationId::new("op1"))).unwrap();
        Self { bus, claims, squad, bot, world, up, now: 0 }
    }
    async fn tick(&mut self, n: u64) {
        for _ in 0..n {
            self.now += 1;
            self.claims.clock.store(self.now, Ordering::SeqCst);
            self.squad.tick(Gt(self.now)).await;
            self.bot.tick(Gt(self.now)).await;
        }
    }
    fn to_op(&mut self) -> Vec<IntentBody> {
        std::iter::from_fn(|| self.up.try_recv()).map(|i| i.body).collect()
    }
}

/// H1: a failed renew must end the bot's lease too, so a lost Halt cannot leave it working a tree the claim no longer covers.
#[tokio::test]
async fn a_failed_renew_stops_a_bot_whose_halt_is_lost() {
    let mut l = Live::new(1, 20);
    let tree = l.world.lock().unwrap().tree_list()[0].clone();
    let d = (tree.0.pos.x + tree.0.pos.z) as u64;
    assert!((15..35).contains(&d), "the tree is far enough that the renew comes first: {d}");
    l.bot.lose_halts();
    let mut j = job("j1", 0, 1);
    j.params = BTreeMap::from([("trees".to_string(), ParamValue::Locs(vec![tree.0.clone()]))]);
    let body = IntentBody::JobBatch { squad: sq(), jobs: vec![j] };
    l.bus.publish_intent(&squad_jobs(&srv(), &sq()), Intent { key: "b".into(), body }).await.unwrap();
    l.tick(5).await;
    assert_eq!(l.squad.running().len(), 1);
    *l.claims.mode.lock().unwrap() = Mode::Preempt;
    l.tick(100).await;
    assert!(l.squad.running().is_empty(), "the squad dropped the job");
    assert_eq!(l.world.lock().unwrap().trees_left(), 1, "the bot took no log after the failed renew");
    assert!(l.bot.is_idle());
    let preempted = l.to_op().into_iter().any(|b| matches!(b, IntentBody::JobOutcome(o) if o.outcome.kind == OutcomeKind::Preempted));
    assert!(preempted, "the bot stopped by its own lease rule and said so");
}

/// H4: a bot-scope halt on a one-bot squad stops that bot's work, not the squad (a later job is reported, below).
#[tokio::test]
async fn a_bot_scope_halt_on_a_one_bot_squad_does_not_stop_the_squad() {
    let mut r = Rig::new(&["b0"]);
    r.batch(vec![job("j1", 1, 1)]).await;
    r.tick(2, &[]).await;
    r.bots[0].bodies();
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Bot(BotId::new("b0")) }).unwrap();
    r.tick(1, &[]).await;
    assert!(r.bots[0].halted(), "that bot's work stops");
    assert!(r.squad.running().is_empty());
    assert!(!r.squad.is_stopped(), "the squad is not stopped");
    r.batch(vec![job("j2", 1, 1)]).await;
    r.tick(2, &[]).await;
    assert_eq!(r.squad.queued(), 0, "the only bot is held, so the job is reported, not queued");
    assert!(!r.squad.is_stopped());
}

/// H4: a batch that arrives while the squad is stopped is reported up, one outcome per job, not dropped silently.
#[tokio::test]
async fn a_batch_dropped_while_stopped_is_reported_up() {
    let mut r = Rig::new(&["b0", "b1"]);
    r.bus.send_command(&squad_cmd(&srv(), &sq()), Command::Halt { scope: Scope::Squad(sq()) }).unwrap();
    r.tick(1, &[]).await;
    assert!(r.squad.is_stopped(), "a squad-scope halt still stops the squad");
    r.batch(vec![job("j1", 1, 1), job("j2", 2, 2)]).await;
    r.tick(1, &[]).await;
    assert_eq!(r.squad.queued(), 0);
    let ups: Vec<JobOutcome> = r.to_op().into_iter().filter_map(|b| if let IntentBody::JobOutcome(o) = b { Some(o) } else { None }).collect();
    let got: Vec<_> = ups.iter().map(|o| (o.outcome.job.as_str(), o.attempt)).collect();
    assert_eq!(got, [("j1", 1), ("j2", 2)]);
    assert!(ups.iter().all(|o| o.outcome.kind == OutcomeKind::Preempted));
}

/// H3: a late outcome from an earlier squad's bot is not forwarded to this squad's operation; the squad records it.
#[tokio::test]
async fn a_late_outcome_for_a_job_this_squad_never_had_is_recorded_not_forwarded() {
    let mut r = Rig::new(&["b0"]);
    let old = job("op0-j0001", 5, 1);
    let mut o = Outcome { job: old.id.clone(), kind: OutcomeKind::Preempted, report: Report::default() };
    o.report.items_gained = vec![ItemCount { item: ItemId::new("minecraft:oak_log"), n: 4 }];
    let body = IntentBody::JobOutcome(JobOutcome { bot: BotId::new("b0"), outcome: o, tick: Gt(0), attempt: 1 });
    r.bots[0].send(&r.bus, "late", body).await;
    r.tick(2, &[]).await;
    assert!(r.to_op().iter().all(|b| !matches!(b, IntentBody::JobOutcome(_))), "nothing goes to the new operation");
    let e = r.squad.exceptions.iter().find(|e| e.kind == "late_outcome").expect("recorded");
    assert_eq!(e.subject, "op0-j0001 items:4");
}
