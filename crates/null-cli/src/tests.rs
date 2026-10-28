//! End-to-end runs of the spine on simulated bots. Spec: specs/K1-hq.md §8, specs/BUILD_PLAN.md S3
use crate::*;
use null_bus::Bus;
use null_op::{ClaimEvent, OpEvent, decode, op_events, op_summary};
use null_proto::*;
use std::collections::BTreeMap;

fn srv() -> ServerId {
    ServerId::new("smp1")
}
fn op1() -> OperationId {
    OperationId::new("op1")
}
fn log_item() -> ItemId {
    null_simbot::oak_log()
}

fn op_log(s: &Spine, op: &OperationId) -> Vec<OpEvent> {
    let evs: Vec<LogEvent> = s.bus.read_events(&op_events(&srv(), op), 0).into_iter().map(|(_, e)| e).collect();
    decode(&evs)
}

fn summary(s: &Spine, op: &OperationId) -> OpSummary {
    match s.bus.subscribe_summary(&op_summary(&srv(), op)).latest() {
        Some(SummaryBody::Operation(x)) => x,
        other => panic!("no summary: {other:?}"),
    }
}

/// No tree is under two live claims at once, read from the claims log.
fn assert_no_double_claim(evs: &[OpEvent]) {
    let clash = |a: &ClaimKind, b: &ClaimKind| match (a, b) {
        (ClaimKind::BlockSet(x), ClaimKind::BlockSet(y)) => x.iter().any(|l| y.contains(l)),
        (ClaimKind::Region(x), ClaimKind::Region(y)) => x.overlaps(y),
        (ClaimKind::Region(r), ClaimKind::BlockSet(s)) | (ClaimKind::BlockSet(s), ClaimKind::Region(r)) => s.iter().any(|l| r.contains(l)),
        _ => false,
    };
    let mut active: BTreeMap<ClaimId, ClaimKind> = BTreeMap::new();
    let mut granted = 0;
    for e in evs {
        let OpEvent::Claim(c) = e else { continue };
        match c {
            ClaimEvent::Granted { id, kind, .. } => {
                granted += 1;
                for (other, held) in &active {
                    assert!(!clash(held, kind), "{id} took a tree that {other} still holds");
                }
                active.insert(id.clone(), kind.clone());
            }
            ClaimEvent::Released { id } | ClaimEvent::Expired { id, .. } | ClaimEvent::Preempted { id, .. } => {
                active.remove(id);
            }
            _ => {}
        }
    }
    assert!(granted > 0, "the check saw no granted claim");
}

fn count(evs: &[OpEvent], kind: &str) -> usize {
    evs.iter().filter(|e| e.kind() == kind).count()
}

/// Logs felled in the world so far.
fn felled(s: &Spine, before: u32) -> u32 {
    before - s.trees().iter().map(|t| t.1).sum::<u32>()
}
fn all_logs(s: &Spine) -> u32 {
    s.trees().iter().map(|t| t.1).sum()
}

async fn started(seed: u64, bots: usize) -> (Spine, u32) {
    let mut s = Spine::new(&SpineConfig::new(seed, bots));
    let before = all_logs(&s);
    let out = s.exec("set-goal obtain oak_log 64 --reason test").await;
    assert!(out.contains("op1"), "{out}");
    (s, before)
}

#[tokio::test]
async fn obtain_64_with_two_bots_through_the_cli_has_no_double_claims() {
    let (mut s, before) = started(3, 2).await;
    assert!(s.exec("run-done").await.starts_with("idle"));
    let sum = summary(&s, &op1());
    assert_eq!((sum.state, sum.progress), (OpState::Done, Progress { done: 64, target: 64 }));
    let evs = op_log(&s, &op1());
    assert_no_double_claim(&evs);
    let granted = count(&evs, "claim_granted");
    assert!(granted >= 13, "one claim per tree: {granted}");
    let status = s.exec("status").await;
    assert!(status.contains("op1") && status.contains("Done") && status.contains("64/64"), "{status}");
    // The ledger counts every log felled, once.
    let ledger: u32 = sum.stock.iter().map(|c| c.n).sum();
    assert_eq!(ledger, felled(&s, before));
    assert_eq!(s.audit.entries.iter().filter(|e| e.accepted).count(), 2, "set-goal and status are audited");
    assert_eq!(s.audit.entries[0].reason, "test");
}

#[tokio::test]
async fn a_bot_killed_mid_job_has_its_claim_expire_and_its_job_requeued() {
    let (mut s, before) = started(3, 2).await;
    s.bots[0].die_at(Gt(30));
    assert!(s.run_until_idle(6000).await);
    let evs = op_log(&s, &op1());
    assert!(count(&evs, "claim_expired") >= 1);
    assert!(count(&evs, "requeued") >= 1);
    assert_no_double_claim(&evs);
    let sum = summary(&s, &op1());
    assert_eq!(sum.state, OpState::Done);
    assert_eq!(sum.headcount, Headcount { assigned: 2, alive: 1 }, "the headcount counts live bots");
    assert_eq!(sum.stock.iter().map(|c| c.n).sum::<u32>(), felled(&s, before));
}

#[tokio::test]
async fn halt_stops_new_work_within_one_squad_tick() {
    let (mut s, _) = started(3, 2).await;
    for _ in 0..60 {
        s.step().await;
        if s.squads[&op1()].running().len() == 2 {
            break;
        }
    }
    assert_eq!(s.squads[&op1()].running().len(), 2, "both bots are working");
    let out = s.exec("halt --reason test").await;
    assert!(out.contains("halt sent"), "{out}");
    let dispatched = count(&op_log(&s, &op1()), "dispatched");
    s.step().await; // one tick: squad and op both see the halt
    assert!(s.squads[&op1()].is_stopped());
    assert!(s.squads[&op1()].running().is_empty(), "no job is left running");
    assert!(s.bots.iter().all(|b| b.is_idle()), "the bots stopped in the same tick");
    s.run(300).await;
    assert_eq!(count(&op_log(&s, &op1()), "dispatched"), dispatched, "nothing new went out");
    assert_eq!(summary(&s, &op1()).state, OpState::Halted);
    let last = s.audit.entries.last().unwrap();
    assert!(last.command.is_urgent() && last.accepted, "the halt is audited");
}

#[tokio::test]
async fn a_killed_operation_restarts_from_its_log_with_the_ledger_intact() {
    let (mut s, before) = started(3, 2).await;
    for _ in 0..3000 {
        s.step().await;
        let o = &s.ops[&op1()];
        if o.ledger(&log_item()) >= 20 && o.job_states().iter().any(|j| j.1 == null_op::JobSt::InFlight) {
            break;
        }
    }
    let fp = s.ops[&op1()].fingerprint();
    s.kill_op(&op1()).await;
    s.restart_op(&op1()).await; // the squad sends its kept outcomes again
    assert_eq!(s.ops[&op1()].fingerprint(), fp, "claims, jobs and ledger are rebuilt");
    let had = s.ops[&op1()].ledger(&log_item());
    s.step().await;
    assert!(s.ops[&op1()].ledger(&log_item()) >= had);
    let evs = op_log(&s, &op1());
    let mut keys = std::collections::BTreeSet::new();
    for e in &evs {
        if let OpEvent::Outcome { key, .. } = e {
            assert!(keys.insert(key.clone()), "outcome {key} was logged twice");
        }
    }
    assert!(s.run_until_idle(6000).await);
    s.run(150).await;
    let evs = op_log(&s, &op1());
    assert_no_double_claim(&evs);
    let sum = summary(&s, &op1());
    assert_eq!(sum.state, OpState::Done);
    assert_eq!(sum.stock.iter().map(|c| c.n).sum::<u32>(), felled(&s, before), "every log felled, counted once");
}

async fn digest_bytes(bots: usize) -> (usize, usize) {
    let (mut s, _) = started(3, bots).await;
    let size = |d: &Digest| serde_json::to_string(d).unwrap().len();
    s.run(5).await;
    let mid = size(&s.hq.digest());
    assert!(s.run_until_idle(6000).await);
    s.run(30).await;
    let d = s.hq.digest();
    assert_eq!(d.rows.len(), 1);
    (mid, size(&d))
}

#[tokio::test]
async fn the_digest_does_not_grow_with_the_bot_count() {
    let (m2, e2) = digest_bytes(2).await;
    let (m50, e50) = digest_bytes(50).await;
    // Only the digits of the headcount differ.
    assert!(m50 <= m2 + 16 && e50 <= e2 + 16, "2 bots: {m2}/{e2} bytes, 50 bots: {m50}/{e50} bytes");
}

#[tokio::test]
async fn a_bot_that_stalls_past_its_claim_never_works_the_tree_of_the_next_attempt() {
    let (mut s, before) = started(3, 3).await;
    for _ in 0..300 {
        s.step().await;
        if !s.squads[&op1()].running().is_empty() {
            break;
        }
    }
    let (stalled, job, _) = s.squads[&op1()].running()[0].clone();
    let idx = s.bots.iter().position(|b| b.id == stalled).unwrap();
    s.bots[idx].lose_halts(); // the squad's halt never arrives: only the bot's own lease rule can stop it
    let from = s.now;
    s.bots[idx].stall(Gt(from), Gt(from + 250)); // wakes while the operation still runs
    for _ in 0..200 {
        s.step().await;
    }
    let sum = summary(&s, &op1());
    assert_eq!(sum.headcount.alive, 2, "the silent bot is reported lost");
    let evs = op_log(&s, &op1());
    assert!(
        evs.iter().any(|e| matches!(e, OpEvent::Dispatched { job: j, attempt: 2 } if *j == job)),
        "the job moved to another bot"
    );
    assert!(s.run_until_idle(6000).await);
    s.run(150).await;
    let evs = op_log(&s, &op1());
    assert_no_double_claim(&evs);
    assert_eq!(s.audit.entries.len(), 1);
    // The stalled bot's late outcome for the old attempt is a refusal, not a run to the end (a bot with no lease
    // check would find the tree felled and report TargetGone).
    let late: Vec<_> = evs
        .iter()
        .filter_map(|e| match e {
            OpEvent::Outcome { bot, attempt: 1, outcome, .. } if *bot == stalled && outcome.job == job => Some(outcome.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(late.len(), 1, "the stalled bot reported its old attempt once");
    assert_eq!(late[0].kind, OutcomeKind::Preempted);
    assert!(late[0].report.items_gained.is_empty());
    assert_eq!(summary(&s, &op1()).stock.iter().map(|c| c.n).sum::<u32>(), felled(&s, before), "no tree was felled twice");
    assert!(s.bots[idx].is_idle());
}

#[tokio::test]
async fn two_goals_run_one_after_the_other_on_the_same_bots_and_trees_are_not_shared() {
    let mut s = Spine::new(&SpineConfig::new(5, 2));
    let before = all_logs(&s);
    s.exec("set-goal obtain oak_log 20 --reason a").await;
    let queued = s.exec("set-goal obtain oak_log 16 --priority 3 --reason b").await;
    assert!(queued.contains("queued"), "{queued}");
    assert!(s.run_until_idle(10_000).await);
    s.run(100).await;
    for op in ["op1", "op2"] {
        let sum = summary(&s, &OperationId::new(op));
        assert_eq!(sum.state, OpState::Done, "{op}");
    }
    let (a, b) = (summary(&s, &op1()), summary(&s, &OperationId::new("op2")));
    let total: u32 = [a, b].iter().flat_map(|x| x.stock.iter()).map(|c| c.n).sum();
    assert_eq!(total, felled(&s, before));
}

#[tokio::test]
async fn commands_parse_and_refuse_bad_input() {
    let mut s = Spine::new(&SpineConfig::new(1, 2));
    assert!(s.exec("set-goal build house").await.starts_with("error"));
    assert!(s.exec("set-goal obtain oak_log many").await.starts_with("error"));
    assert!(s.exec("halt --reason x").await.starts_with("refused"), "nothing runs yet");
    assert!(s.exec("inspect op9").await.starts_with("refused"));
    s.exec("set-goal obtain oak_log 8 --reason go").await;
    assert!(s.exec("inspect op1").await.contains("\"bots\""));
    assert!(s.exec("recall op1 --reason x").await.contains("recall sent"));
    assert!(s.exec("halt bot:bot1 --reason \"stop that bot\"").await.contains("halt sent"));
    let refused = s.audit.entries.iter().filter(|e| !e.accepted).count();
    assert_eq!(refused, 2, "refused commands are audited too");
}

#[tokio::test]
async fn the_audit_log_is_written_as_jsonl() {
    let path = std::env::temp_dir().join(format!("null-cli-audit-{}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&path);
    let mut s = Spine::new(&SpineConfig::new(1, 2));
    s.audit = Audit::to_file(&path).unwrap();
    s.exec("set-goal obtain oak_log 8 --reason because").await;
    s.exec("status").await;
    let lines: Vec<AuditEntry> = std::fs::read_to_string(&path).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect();
    std::fs::remove_file(&path).unwrap();
    assert_eq!(lines.len(), 2);
    assert_eq!((lines[0].principal.as_str(), lines[0].reason.as_str(), lines[0].accepted), ("cli", "because", true));
}

#[tokio::test]
async fn a_write_without_a_reason_is_refused_and_audited() {
    let mut s = Spine::new(&SpineConfig::new(1, 2));
    for line in ["set-goal obtain oak_log 8", "halt", "recall op1"] {
        let out = s.exec(line).await;
        assert!(out.starts_with("refused") && out.contains("reason"), "{line}: {out}");
    }
    assert!(s.hq.queued() == 0 && s.hq.free_bots() == 2, "nothing started");
    assert_eq!(s.audit.entries.len(), 3, "each refusal is audited");
    assert!(s.audit.entries.iter().all(|e| !e.accepted && e.reason.is_empty()));
    assert!(!s.exec("status").await.starts_with("refused"), "reads need no reason");
    assert!(s.exec("help").await.contains("--reason"));
    assert!(s.exec("set-goal obtain oak_log 8 --reason \"two words\"").await.contains("op1"));
    assert_eq!(s.audit.entries.last().unwrap().reason, "two words");
}

/// An empty or valueless `--reason` is still a write: refused and audited, not a parse error (K2 §4.3).
#[tokio::test]
async fn an_empty_or_valueless_reason_is_refused_and_audited() {
    let mut s = Spine::new(&SpineConfig::new(1, 2));
    for line in ["set-goal obtain oak_log 8 --reason \"\"", "halt --reason \"\"", "halt --reason", "recall op1 --reason"] {
        let out = s.exec(line).await;
        assert!(out.starts_with("refused") && out.contains("reason"), "{line}: {out}");
    }
    assert_eq!(s.audit.entries.len(), 4, "each refusal is audited");
    assert!(s.audit.entries.iter().all(|e| !e.accepted && e.reason.is_empty()));
}

/// K1 §4.2: with retire_after below the renew period, retiring halts the bots at once; nothing else stops them.
#[tokio::test]
async fn retiring_an_operation_halts_its_working_bots() {
    let mut cfg = SpineConfig::new(3, 3);
    cfg.retire_after = Gt(1);
    let mut s = Spine::new(&cfg);
    s.exec("set-goal obtain oak_log 64 --reason test").await;
    let mut busy_at_retire = 0;
    for _ in 0..6000 {
        let busy = s.bots.iter().filter(|b| !b.is_idle()).count();
        s.step().await;
        if s.ops.is_empty() {
            busy_at_retire = busy;
            break;
        }
    }
    assert!(busy_at_retire > 0, "a bot was still working when the operation retired");
    assert!(s.bots.iter().all(|b| b.is_idle()), "retiring halted every bot in the same tick");
}

/// H3: a bot halted at retire after it took logs but before it reported has its yield counted once by the old
/// operation, which stays alive until then.
#[tokio::test]
async fn the_late_yield_of_a_halted_bot_is_counted_by_the_old_operation() {
    let mut s = Spine::new(&SpineConfig::new(3, 3));
    s.exec("set-goal obtain oak_log 64 --reason test").await;
    let mut held = 0;
    for _ in 0..6000 {
        s.step().await;
        held = s.bots.iter().map(|b| b.holding()).sum();
        if held > 0 {
            break;
        }
    }
    assert!(held > 0, "a bot holds logs it has not reported");
    let before = s.ops[&op1()].ledger(&log_item());
    s.retire(&op1()).await;
    assert!(s.ops.is_empty() && s.retiring.contains_key(&op1()), "the old operation stays while the bots wind down");
    let mut after = 0;
    for _ in 0..100 {
        if let Some((_, o)) = s.retiring.get(&op1()) {
            after = o.ledger(&log_item());
        }
        s.step().await;
        if s.retiring.is_empty() {
            break;
        }
    }
    assert!(s.retiring.is_empty(), "retire ended");
    // The ledger is read just before the step that ends the wait, so check the log for the final count too.
    let mut keys = vec![];
    let mut total = 0;
    for e in op_log(&s, &op1()) {
        if let OpEvent::Outcome { key, outcome, .. } = e {
            keys.push(key);
            total += outcome.report.items_gained.iter().map(|c| c.n).sum::<u32>();
        }
    }
    let unique: std::collections::BTreeSet<_> = keys.iter().collect();
    assert_eq!(unique.len(), keys.len(), "each (job, attempt) counted once");
    assert_eq!(total, before + held, "the held logs are in the old op's log: {before} + {held}, ledger {after}");
    assert!(s.retired_exceptions.iter().all(|e| e.kind != "retire_unreported"), "{:?}", s.retired_exceptions);
}
