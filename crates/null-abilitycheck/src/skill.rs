//! The bot side: a skill that takes one `Op` at a time from the runner thread and does it through
//! `BotView` / `BotCommands` only. It reports back with the host tick and wall time.

use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc::{Receiver, Sender},
    },
    time::Instant,
};

/// Wall time between client ticks, for diagnosing Grim's Timer check (packets faster than real
/// time): the longest gap, and how many ticks came under 25 ms after the one before (catch-up).
pub static TICK_GAP_MAX_MS: AtomicU64 = AtomicU64::new(0);
pub static TICK_FAST: AtomicU64 = AtomicU64::new(0);

/// The bot's own view when it last refused a click on the client (reason set, nothing sent):
/// position, look, ground, velocity and the target block as it sees it. Printed on the row, so an
/// intermittent "crosshair does not hit it" shows whether the eye, the look or the world was off.
pub static REFUSAL_VIEW: Mutex<Option<String>> = Mutex::new(None);

use null_bot_api::{
    ActionId, ActionRecord, BlockAction, BlockOutcome, BlockPos, BotCommands, BotView, ClickEvidence, ClickHit, Face,
    ItemStack, Look,
    NavStatus, Skill, SkillStatus, Vec3,
};

/// The client's own record of a refused click (fork patch 28): what its ray hit, from which eye,
/// and the real eye height, so a refusal can be told from a transient or a wrong aim.
// Spec: specs/B6-placement-breaking.md §4.1 (refusals carry their evidence)
pub fn describe_evidence(e: &ClickEvidence) -> String {
    let hit = match e.hit {
        ClickHit::Miss => "ray hit nothing".to_string(),
        ClickHit::Block { pos, face, point } => format!("ray hit block {pos:?} face {face:?} at {point:?}"),
        ClickHit::Entity { id, kind } => format!("ray hit entity {} ({kind})", id.0),
    };
    format!(
        "evidence at client tick {}: {hit}; eye {:?}, eye height {:.3}, crouching {}, yaw {:.2} pitch {:.2}, look changed after the ray was cast: {}",
        e.check_tick, e.eye, e.eye_height, e.crouching, e.yaw, e.pitch, e.look_changed_after_hit
    )
}

#[derive(Clone, Debug)]
pub enum Op {
    Snapshot,
    SelectSlot(u8),
    SwapOffhand,
    /// Wait until the bot's own world shows this block.
    WaitBlock { pos: BlockPos, name: &'static str },
    Peek(BlockPos),
    Break(BlockPos),
    /// Start a break, cancel it after this many ticks.
    BreakThenCancel(BlockPos, u64),
    /// Click `face` of `pos`, aiming `aim` away from the face centre.
    Place { pos: BlockPos, face: Face, aim: Vec3 },
    Goto(BlockPos),
    /// Look at a visible point of the block's real shape on `face`, then click it.
    PlaceAim { pos: BlockPos, face: Face },
    /// As `PlaceAim`, with `auto_crouch` (B6 §4.2, crouch handling).
    PlaceAimCrouch { pos: BlockPos, face: Face, auto_crouch: bool },
    /// As `PlaceAim`, but the loop stalls `ms` inside the tick between the look and the click, so
    /// several catch-up ticks run before the click (fork patch 29, row AT).
    PlaceAimStall { pos: BlockPos, face: Face, ms: u64 },
    /// Look at a visible point of the block's real shape, then break it.
    BreakAim(BlockPos),
    /// Look at the block, then send `break_block` every tick until the result settles.
    BreakSpam(BlockPos),
    Sneak(bool),
    /// Test hook (AT): hold the ECS for this many ms inside a tick, then wait for the catch-up.
    Stall(u64),
    /// Break these blocks one after another, as fast as the bot may (held-button mining).
    BreakChain(Vec<BlockPos>),
    /// Item use, attack and attacker ops (A5-A8), in `skill_items`.
    Items(crate::skill_items::ItemOp),
    /// Inventory window ops (A9), in `skill_inventory`.
    Inv(crate::skill_inventory::InvOp),
    /// Result-mailbox ops (A13), in `skill_mailbox`.
    Mail(crate::skill_mailbox::MailOp),
    /// Death and respawn ops (A12), in `skill_respawn`.
    Life(crate::skill_respawn::LifeOp),
    /// `gather` and the tree policy (A14), in `skill_gather`.
    Gather(crate::skill_gather::GatherOp),
    /// Place prediction ops (A22), in `skill_predict`.
    Predict(crate::skill_predict::PredOp),
    /// Client refusal ops (A20), in `skill_refusals`.
    Refuse(crate::skill_refusals::RefuseOp),
}

#[derive(Clone, Debug)]
pub struct Snap {
    pub pos: Vec3,
    pub selected: u8,
    pub offhand: Option<ItemStack>,
    pub counts: BTreeMap<String, u32>,
    /// The bot's crouch input (`BotView::sneaking`).
    pub sneaking: bool,
}

#[derive(Clone, Debug)]
pub enum Info {
    /// A break or place command was just issued; the runner samples the server clock now.
    Started,
    Snapshot(Snap),
    Waited(bool),
    Block(Option<String>),
    Action(BlockAction),
    Timeout,
    Items(crate::skill_items::ItemsInfo),
    Inv(crate::skill_inventory::InvInfo),
    Life(crate::skill_respawn::LifeInfo),
    Mail(crate::skill_mailbox::MailInfo),
    Gather(Box<crate::skill_gather::GatherInfo>),
    Predict(crate::skill_predict::PredInfo),
    Refuse(crate::skill_refusals::RefuseInfo),
    /// Per block of a chain: (block, ticks from start to finish, outcome).
    Chain(Vec<(BlockPos, Option<u64>, BlockOutcome)>),
    /// The skill ended or was dropped with this op still open: it never finished (the host's safety stop).
    Ended,
}

#[derive(Clone, Debug)]
pub struct Report {
    /// The number of the op this answers (`Ctx::send`), so a late reply never reaches another op.
    pub seq: u64,
    pub info: Info,
}

pub struct RemoteSkill {
    ops: Arc<Mutex<Receiver<(u64, Op)>>>,
    out: Sender<Report>,
    cur: Option<Run>,
    items: crate::skill_items::ItemState,
    chain: Chain,
    /// The ids of the latest click, open and respawn, so a status op can report them (A9, A12).
    latest: crate::skill_inventory::Latest,
    mail: crate::skill_mailbox::MailState,
    gather: crate::skill_gather::GatherState,
    predict: crate::skill_predict::PredState,
    refuse: crate::skill_refusals::RefuseState,
    last_step: Option<Instant>,
    keep: crate::skill_respawn::KeepState,
}

/// Progress of a `BreakChain`: the block looked at (and when), and the id of every break sent.
#[derive(Default)]
struct Chain {
    looked: Option<(BlockPos, u64)>,
    started: Vec<(BlockPos, ActionId)>,
    /// First tick every block read as air.
    all_air: Option<u64>,
}

/// The chain gives up after this many ticks, before the runner's 60 s wait.
const CHAIN_TICKS: u64 = 1000;

struct Run {
    seq: u64,
    op: Op,
    t0: u64,
    offhand_before: Option<ItemStack>,
    /// The ids the op's commands got (AD-F5.4); released when the op ends.
    ids: Vec<ActionId>,
}

/// The finished break or placement behind the op's first id (B6 §4.6).
fn block_result(v: &dyn BotView, ids: &[ActionId]) -> Option<BlockAction> {
    match v.action(*ids.first()?).finished() {
        Some(ActionRecord::Block(b)) => Some(*b),
        _ => None,
    }
}

/// An op still open when the skill goes away is answered, so the runner does not wait for it.
impl Drop for RemoteSkill {
    // Spec: specs/plans/ABILITY_CHECK.md (scenario ops: replies are paired to ops by number)
    fn drop(&mut self) {
        if let Some(run) = self.cur.take() {
            let _ = self.out.send(Report { seq: run.seq, info: Info::Ended });
        }
    }
}

impl RemoteSkill {
    pub fn new(ops: Arc<Mutex<Receiver<(u64, Op)>>>, out: Sender<Report>) -> Self {
        Self { ops, out, cur: None, items: Default::default(), chain: Chain::default(), latest: Default::default(), mail: Default::default(), gather: Default::default(), predict: Default::default(), refuse: Default::default(), last_step: None, keep: Default::default() }
    }
}

fn snapshot(v: &dyn BotView) -> Snap {
    let mut counts = BTreeMap::new();
    for s in v.inventory().iter().flatten().chain(v.offhand()) {
        *counts.entry(s.item.clone()).or_insert(0) += s.count as u32;
    }
    Snap {
        pos: v.position(),
        selected: v.selected_slot(),
        offhand: v.offhand().cloned(),
        counts,
        sneaking: v.sneaking(),
    }
}

/// Eye height standing (B6 §4.1).
const EYE: f64 = 1.62;

pub(crate) fn look_at(v: &dyn BotView, c: &mut dyn BotCommands, to: Vec3) {
    let p = v.position();
    let l = Look::towards(Vec3::new(p.x, p.y + EYE, p.z), to);
    c.look(l.yaw, l.pitch);
}

/// Two ticks between the look and the click: the hit result is recomputed after the look lands.
const LOOK_TICKS: u64 = 2;

impl Skill for RemoteSkill {
    fn id(&self) -> &str {
        "ability-check"
    }

    // Spec: specs/F1-swarm-runtime.md §4.4 (A12 `skill_on_death_keep` sets the flag)
    fn on_death(&mut self, _v: &dyn BotView, _d: &null_bot_api::Death) -> null_bot_api::AfterDeath {
        if crate::skill_respawn::KEEP_ARMED.load(std::sync::atomic::Ordering::Relaxed) {
            null_bot_api::AfterDeath::Keep
        } else {
            null_bot_api::AfterDeath::End
        }
    }

    // Spec: specs/plans/ABILITY_CHECK.md (scenario ops: A1 slots, A2 break, A3 pickup, A4 place)
    fn step(&mut self, v: &dyn BotView, c: &mut dyn BotCommands) -> SkillStatus {
        let wall = Instant::now();
        if let Some(t) = self.last_step.replace(wall) {
            let gap = wall.duration_since(t).as_millis() as u64;
            TICK_GAP_MAX_MS.fetch_max(gap, Ordering::Relaxed);
            if gap < 25 {
                TICK_FAST.fetch_add(1, Ordering::Relaxed);
            }
        }
        if let Some(status) = crate::skill_respawn::keep_step(&mut self.keep, v, c) {
            return status;
        }
        self.items.shield.update(v);
        let now = v.tick();
        if self.cur.is_none()
            && let Ok((seq, op)) = self.ops.lock().unwrap().try_recv()
        {
            self.cur = Some(Run { seq, op, t0: now, offhand_before: v.offhand().cloned(), ids: Vec::new() });
        }
        let Some(run) = &self.cur else { return SkillStatus::Running };
        let age = now - run.t0;
        let mut ids = run.ids.clone();
        let seq = run.seq;
        let say = |info: Info| {
            let _ = self.out.send(Report { seq, info });
        };
        let done: Option<Info> = match &run.op {
            Op::Snapshot => Some(Info::Snapshot(snapshot(v))),
            Op::SelectSlot(s) => {
                if age == 0 {
                    c.select_slot(*s);
                }
                (age >= 2).then(|| Info::Snapshot(snapshot(v)))
            }
            Op::SwapOffhand => {
                if age == 0 {
                    c.swap_offhand();
                }
                (age >= 2 && (v.offhand().cloned() != run.offhand_before || age >= 20))
                    .then(|| Info::Snapshot(snapshot(v)))
            }
            Op::WaitBlock { pos, name } => {
                let ok = v.block_at(*pos).as_deref() == Some(*name);
                (ok || age > 200).then_some(Info::Waited(ok))
            }
            Op::Peek(pos) => Some(Info::Block(v.block_at(*pos))),
            Op::Break(pos) => {
                if age == 0 {
                    look_at(v, c, pos.center());
                } else if age == LOOK_TICKS {
                    ids.push(c.break_block(*pos));
                    say(Info::Started);
                }
                match block_result(v, &ids) {
                    Some(a) => Some(Info::Action(a)),
                    _ => (age > 600).then_some(Info::Timeout),
                }
            }
            Op::BreakThenCancel(pos, after) => {
                if age == 0 {
                    look_at(v, c, pos.center());
                } else if age == LOOK_TICKS {
                    ids.push(c.break_block(*pos));
                } else if age == LOOK_TICKS + after {
                    c.cancel_break();
                }
                match block_result(v, &ids) {
                    Some(a) if age > LOOK_TICKS + after => Some(Info::Action(a)),
                    _ => (age > 200).then_some(Info::Timeout),
                }
            }
            Op::Place { pos, face, aim } => {
                if age == 0 {
                    let (dx, dy, dz) = face.delta();
                    let k = pos.center();
                    look_at(v, c, Vec3::new(k.x + dx as f64 * 0.5 + aim.x, k.y + dy as f64 * 0.5 + aim.y, k.z + dz as f64 * 0.5 + aim.z));
                } else if age == LOOK_TICKS {
                    ids.push(c.place_block(*pos, *face, false));
                    say(Info::Started);
                }
                match block_result(v, &ids) {
                    Some(a) => Some(Info::Action(a)),
                    _ => (age > 200).then_some(Info::Timeout),
                }
            }
            // Spec: specs/F5-version-layer.md §4.6 (the crosshair ray is cast once per tick; patch 29)
            Op::PlaceAim { pos, face } | Op::PlaceAimCrouch { pos, face, .. } | Op::PlaceAimStall { pos, face, .. } => {
                // The stall row runs the stall first, so the look (age 1) and the click (age
                // LOOK_TICKS) both fall in the catch-up ticks of ONE update. The old code cast
                // its ray once at the start of that update, before the look: the click saw a
                // stale ray. The bot first faces away from the target, so the stale ray misses.
                let look_age = if matches!(run.op, Op::PlaceAimStall { .. }) { 1 } else { 0 };
                if age == 0 && let Op::PlaceAimStall { ms, .. } = run.op {
                    let (p, k) = (v.position(), pos.center());
                    look_at(v, c, Vec3::new(2.0 * p.x - k.x, p.y + EYE, 2.0 * p.z - k.z));
                    // Sleeping inside `step` holds the ECS lock like a long frame.
                    std::thread::sleep(std::time::Duration::from_millis(ms));
                } else if age == look_age {
                    c.look_at_block(*pos, Some(*face));
                } else if age == LOOK_TICKS {
                    let auto = matches!(run.op, Op::PlaceAimCrouch { auto_crouch: true, .. });
                    ids.push(c.place_block(*pos, *face, auto));
                    say(Info::Started);
                }
                match block_result(v, &ids) {
                    Some(a) => Some(Info::Action(a)),
                    _ => (age > 200).then_some(Info::Timeout),
                }
            }
            Op::BreakAim(pos) | Op::BreakSpam(pos) => {
                if age == 0 {
                    c.look_at_block(*pos, None);
                } else if age == LOOK_TICKS
                    // spamming stops when the block is gone: a break of air is a new break
                    || (age > LOOK_TICKS && matches!(run.op, Op::BreakSpam(_)) && v.block_at(*pos).as_deref() != Some("minecraft:air"))
                {
                    // Repeated commands on one block name the same action (B6 §4.6).
                    ids.push(c.break_block(*pos));
                    if age == LOOK_TICKS {
                        say(Info::Started);
                    }
                }
                match block_result(v, &ids) {
                    Some(a) => Some(Info::Action(a)),
                    _ => (age > 600).then_some(Info::Timeout),
                }
            }
            Op::BreakChain(list) => {
                let ch = &mut self.chain;
                if age == 0 {
                    *ch = Chain::default();
                }
                // The next break starts as soon as the last block reads as air (the client's own
                // prediction at the finishing packet), without waiting for its ack: the mailbox
                // keeps every result by id (AD-F5.4), read all at the end.
                let next = list.iter().copied().find(|p| v.block_at(*p).as_deref() != Some("minecraft:air"));
                if next.is_none() && ch.all_air.is_none() {
                    ch.all_air = Some(now);
                }
                let results = |ch: &Chain| -> Vec<(BlockPos, Option<u64>, BlockOutcome)> {
                    ch.started
                        .iter()
                        .map(|(p, id)| match v.action(*id).record() {
                            Some(ActionRecord::Block(a)) => (a.pos, a.finish_tick.map(|f| f - a.start_tick), a.outcome),
                            _ => (*p, None, BlockOutcome::Pending),
                        })
                        .collect()
                };
                let all_finished = |ch: &Chain| ch.started.iter().all(|(_, id)| v.action(*id).is_finished());
                match next {
                    // Look at the next block as soon as the last one is gone; the fork holds the
                    // start until vanilla's destroy delay has passed.
                    Some(p) if ch.looked.map(|(q, _)| q) != Some(p) => {
                        c.look_at_block(p, None);
                        ch.looked = Some((p, now));
                        None
                    }
                    Some(p) => {
                        // One command per block; a new one only after the last for it finished
                        // and the block is still there.
                        let last = ch.started.iter().rev().find(|(q, _)| *q == p).map(|(_, id)| *id);
                        let idle = last.is_none_or(|id| v.action(id).is_finished());
                        if ch.looked.is_some_and(|(_, t)| now >= t + LOOK_TICKS) && idle {
                            ch.started.push((p, c.break_block(p)));
                        }
                        (age > CHAIN_TICKS).then(|| Info::Chain(results(ch)))
                    }
                    // All air: done when every id has finished; a missing result fails the row
                    // with the log after 40 ticks rather than hanging.
                    None if all_finished(ch) || ch.all_air.is_some_and(|t| now > t + 40) || age > CHAIN_TICKS => {
                        Some(Info::Chain(results(ch)))
                    }
                    None => None,
                }
            }
            Op::Stall(ms) => {
                // Sleeping inside `step` holds the ECS write lock, which stalls the run loop
                // exactly as a long frame would. Then 12 more ticks for the catch-up to finish.
                if age == 0 {
                    std::thread::sleep(std::time::Duration::from_millis(*ms));
                }
                (age >= 12).then(|| Info::Snapshot(snapshot(v)))
            }
            Op::Sneak(on) => {
                if age == 0 {
                    c.sneak(*on);
                }
                (age >= 3).then(|| Info::Snapshot(snapshot(v)))
            }
            Op::Items(io) => crate::skill_items::step(io, age, &mut self.items, v, c, &say),
            Op::Inv(io) => crate::skill_inventory::step(io, age, v, c, &mut self.latest),
            Op::Mail(io) => crate::skill_mailbox::step(io, age, v, c, &mut self.mail),
            Op::Life(io) => crate::skill_respawn::step(io, age, v, c, &mut self.latest),
            Op::Gather(io) => crate::skill_gather::step(io, age, v, c, &mut self.gather),
            Op::Predict(io) => crate::skill_predict::step(io, age, v, c, &mut self.predict),
            Op::Refuse(io) => crate::skill_refusals::step(io, age, v, c, &mut self.refuse),
            Op::Goto(pos) => {
                if age == 0 {
                    c.goto(*pos, 0.0);
                }
                let near = v.position().distance(pos.center()) < 1.2;
                ((age > 5 && v.nav_status() == NavStatus::Idle && near) || age > 300)
                    .then(|| Info::Snapshot(snapshot(v)))
            }
        };
        if let Some(Info::Action(a)) = &done
            && a.outcome == BlockOutcome::Refused
            && a.ack_tick.is_none()
            && let Some(why) = a.reason
        {
            let clicked = match &run.op {
                Op::Place { pos, .. } | Op::PlaceAim { pos, .. } | Op::PlaceAimCrouch { pos, .. } | Op::PlaceAimStall { pos, .. } => Some(*pos),
                Op::Break(pos) | Op::BreakAim(pos) | Op::BreakSpam(pos) | Op::BreakThenCancel(pos, _) => Some(*pos),
                _ => None,
            };
            *REFUSAL_VIEW.lock().unwrap() = Some(format!(
                "bot view at refusal ({why}): {}; sneaking (input) {}, pos {:?}, look {:?}, on ground {}, velocity {:?}, target {:?} seen as {:?}, clicked block {:?} seen as {:?}",
                a.evidence.as_ref().map_or("no click evidence".to_string(), describe_evidence),
                v.sneaking(),
                v.position(),
                v.look(),
                v.on_ground(),
                v.velocity(),
                a.pos,
                v.block_at(a.pos),
                clicked,
                clicked.and_then(|p| v.block_at(p))
            ));
        }
        if let Some(info) = done {
            for id in &ids {
                c.release(*id);
            }
            if matches!(run.op, Op::BreakChain(_)) {
                for (_, id) in &self.chain.started {
                    c.release(*id);
                }
            }
            say(info);
            self.cur = None;
        } else if let Some(run) = self.cur.as_mut() {
            run.ids = ids;
        }
        SkillStatus::Running
    }
}
