//! The bot side: a skill that takes one `Op` at a time from the runner thread and does it through
//! `BotView` / `BotCommands` only. It reports back with the host tick and wall time.

use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex, mpsc::{Receiver, Sender}},
};

use null_bot_api::{
    BlockAction, BlockOutcome, BlockPos, BotCommands, BotView, Face, ItemStack, Look, NavStatus, Skill,
    SkillStatus, Vec3,
};

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
    /// Look at a visible point of the block's real shape, then break it.
    BreakAim(BlockPos),
    /// Look at the block, then send `break_block` every tick until the result settles.
    BreakSpam(BlockPos),
    Sneak(bool),
    /// Break these blocks one after another, as fast as the bot may (held-button mining).
    BreakChain(Vec<BlockPos>),
    /// Item use, attack and attacker ops (A5-A8), in `skill_items`.
    Items(crate::skill_items::ItemOp),
}

#[derive(Clone, Debug)]
pub struct Snap {
    pub pos: Vec3,
    pub selected: u8,
    pub offhand: Option<ItemStack>,
    pub counts: BTreeMap<String, u32>,
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
    /// Per block of a chain: (block, ticks from start to finish, outcome).
    Chain(Vec<(BlockPos, Option<u64>, BlockOutcome)>),
}

#[derive(Clone, Debug)]
pub struct Report {
    pub info: Info,
}

pub struct RemoteSkill {
    ops: Arc<Mutex<Receiver<Op>>>,
    out: Sender<Report>,
    cur: Option<Run>,
    items: crate::skill_items::ItemState,
    chain: Chain,
}

/// Progress of a `BreakChain`: the block looked at (and when), and the breaks logged so far.
#[derive(Default)]
struct Chain {
    looked: Option<(BlockPos, u64)>,
    log: Vec<(BlockPos, Option<u64>, BlockOutcome)>,
}

struct Run {
    op: Op,
    t0: u64,
    offhand_before: Option<ItemStack>,
}

impl RemoteSkill {
    pub fn new(ops: Arc<Mutex<Receiver<Op>>>, out: Sender<Report>) -> Self {
        Self { ops, out, cur: None, items: Default::default(), chain: Chain::default() }
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

    // Spec: specs/plans/ABILITY_CHECK.md (scenario ops: A1 slots, A2 break, A3 pickup, A4 place)
    fn step(&mut self, v: &dyn BotView, c: &mut dyn BotCommands) -> SkillStatus {
        let now = v.tick();
        if self.cur.is_none()
            && let Ok(op) = self.ops.lock().unwrap().try_recv()
        {
            self.cur = Some(Run { op, t0: now, offhand_before: v.offhand().cloned() });
        }
        let Some(run) = &self.cur else { return SkillStatus::Running };
        let age = now - run.t0;
        let say = |info: Info| {
            let _ = self.out.send(Report { info });
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
                    c.break_block(*pos);
                    say(Info::Started);
                }
                match v.last_break() {
                    Some(a) if age > LOOK_TICKS && a.start_tick >= run.t0 && a.outcome != BlockOutcome::Pending => {
                        Some(Info::Action(a))
                    }
                    _ => (age > 600).then_some(Info::Timeout),
                }
            }
            Op::BreakThenCancel(pos, after) => {
                if age == 0 {
                    look_at(v, c, pos.center());
                } else if age == LOOK_TICKS {
                    c.break_block(*pos);
                } else if age == LOOK_TICKS + after {
                    c.cancel_break();
                }
                match v.last_break() {
                    Some(a) if age > LOOK_TICKS + after && a.start_tick >= run.t0 && a.outcome != BlockOutcome::Pending => {
                        Some(Info::Action(a))
                    }
                    _ => (age > 200).then_some(Info::Timeout),
                }
            }
            Op::Place { pos, face, aim } => {
                if age == 0 {
                    let (dx, dy, dz) = face.delta();
                    let k = pos.center();
                    look_at(v, c, Vec3::new(k.x + dx as f64 * 0.5 + aim.x, k.y + dy as f64 * 0.5 + aim.y, k.z + dz as f64 * 0.5 + aim.z));
                } else if age == LOOK_TICKS {
                    c.place_block(*pos, *face);
                    say(Info::Started);
                }
                match v.last_place() {
                    Some(a) if age > LOOK_TICKS && a.start_tick >= run.t0 && a.outcome != BlockOutcome::Pending => {
                        Some(Info::Action(a))
                    }
                    _ => (age > 200).then_some(Info::Timeout),
                }
            }
            Op::PlaceAim { pos, face } => {
                if age == 0 {
                    c.look_at_block(*pos, Some(*face));
                } else if age == LOOK_TICKS {
                    c.place_block(*pos, *face);
                    say(Info::Started);
                }
                match v.last_place() {
                    Some(a) if age > LOOK_TICKS && a.start_tick >= run.t0 && a.outcome != BlockOutcome::Pending => {
                        Some(Info::Action(a))
                    }
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
                    c.break_block(*pos);
                    if age == LOOK_TICKS {
                        say(Info::Started);
                    }
                }
                match v.last_break() {
                    Some(a) if age > LOOK_TICKS && a.start_tick >= run.t0 && a.outcome != BlockOutcome::Pending => {
                        Some(Info::Action(a))
                    }
                    _ => (age > 600).then_some(Info::Timeout),
                }
            }
            Op::BreakChain(list) => {
                let ch = &mut self.chain;
                if age == 0 {
                    *ch = Chain::default();
                }
                if let Some(a) = v.last_break()
                    && a.start_tick >= run.t0
                    && a.outcome != BlockOutcome::Pending
                    && !ch.log.iter().any(|(p, _, _)| *p == a.pos)
                {
                    ch.log.push((a.pos, a.finish_tick.map(|f| f - a.start_tick), a.outcome));
                }
                let next = list.iter().copied().find(|p| v.block_at(*p).as_deref() != Some("minecraft:air"));
                match next {
                    // Look at the next block as soon as the last one is gone; the fork holds the
                    // start until vanilla's destroy delay has passed.
                    Some(p) if ch.looked.map(|(q, _)| q) != Some(p) => {
                        c.look_at_block(p, None);
                        ch.looked = Some((p, now));
                        None
                    }
                    Some(p) => {
                        if ch.looked.is_some_and(|(_, t)| now >= t + LOOK_TICKS) {
                            c.break_block(p);
                        }
                        (age > 3000).then_some(Info::Timeout)
                    }
                    None if ch.log.len() >= list.len() || age > 3000 => Some(Info::Chain(ch.log.clone())),
                    None => None,
                }
            }
            Op::Sneak(on) => {
                if age == 0 {
                    c.sneak(*on);
                }
                (age >= 3).then(|| Info::Snapshot(snapshot(v)))
            }
            Op::Items(io) => crate::skill_items::step(io, age, &mut self.items, v, c, &say),
            Op::Goto(pos) => {
                if age == 0 {
                    c.goto(*pos, 0.0);
                }
                let near = v.position().distance(pos.center()) < 1.2;
                ((age > 5 && v.nav_status() == NavStatus::Idle && near) || age > 300)
                    .then(|| Info::Snapshot(snapshot(v)))
            }
        };
        if let Some(info) = done {
            say(info);
            self.cur = None;
        }
        SkillStatus::Running
    }
}
