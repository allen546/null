//! Bot-side ops for A22 (place prediction, fork patch 33). `RemoteSkill::step` calls `step` for
//! `Op::Predict`; everything goes through `BotView` / `BotCommands` only.
//! The ops place a block the bot then stands on or walks onto, without waiting for the ack: the
//! next move depends on the block the client predicted.
// Spec: specs/B6-placement-breaking.md §4.10 (place prediction)

use null_bot_api::{ActionId, ActionState, BlockPos, BotCommands, BotView, Face};

use crate::skill::Info;

/// Two ticks between a look and the click: the hit result is recomputed after the look lands.
const LOOK_TICKS: u64 = 2;
/// Ticks for the crouch pose (and so the lower eye) to be in place before the bridge aims.
const CROUCH_TICKS: u64 = 4;
/// The op gives up after this many ticks, before the runner's 60 s wait.
const MAX_TICKS: u64 = 500;
/// A jump is 1.2522 high; from 1.1 up the box clears a block placed in the cell it left, whichever
/// of the two ticks around the host's sample the click is handled in.
const PLACE_ABOVE: f64 = 1.1;

#[derive(Clone, Debug)]
pub enum PredOp {
    /// Jump and place a block on the one under the feet at the top of the jump, `n` times, with
    /// the look straight down and no wait for any ack. The bot stands on the block it placed.
    Pillar { n: u32 },
    /// Sneak backward off an edge and place `count` blocks of a bridge: each on the north face of
    /// the last support, the first support being `support`. The bot walks onto each block as
    /// soon as it has placed it, without waiting for the ack.
    Bridge { support: BlockPos, count: u32 },
}

#[derive(Default)]
pub struct PredState {
    /// Pillar: jumps started; Bridge: blocks placed.
    k: u32,
    phase: Phase,
    /// Pillar: the standing height when the jump started.
    base_y: f64,
    /// Pillar: the first standing height, and the column.
    start_y: f64,
    col: (i32, i32),
    /// Bridge: the support the next block goes on.
    support: Option<BlockPos>,
    since: u64,
    ids: Vec<ActionId>,
    landed: Vec<f64>,
    min_y: f64,
    gave_up: bool,
    /// Bridge: the last z and the age it last changed at, to tell a bot that does not move.
    last_z: f64,
    moved_at: u64,
}

#[derive(Debug, Default, PartialEq, Clone, Copy)]
enum Phase {
    #[default]
    Start,
    Idle,
    Up,
    Down,
    Back,
    /// Stopped walking; waiting for the glide to end before aiming.
    Settle,
    Aim,
    Done,
}

#[derive(Clone, Debug)]
pub struct PredInfo {
    /// Every place the op sent, read at the end.
    pub states: Vec<ActionState>,
    /// Pillar: the height the bot stood at after each jump.
    pub landed: Vec<f64>,
    /// The lowest height the bot had at any step of the op.
    pub min_y: f64,
    pub end: (f64, f64, f64),
    pub on_ground: bool,
    pub gave_up: bool,
    /// Where the op stood when it ended: phase, position, input and speed, for a failed row.
    pub note: String,
}

fn info(v: &dyn BotView, st: &PredState) -> Info {
    let p = v.position();
    Info::Predict(PredInfo {
        states: st.ids.iter().map(|id| v.action(*id)).collect(),
        landed: st.landed.clone(),
        min_y: st.min_y,
        end: (p.x, p.y, p.z),
        on_ground: v.on_ground(),
        gave_up: st.gave_up,
        note: format!("phase {:?}, {} placed, sneaking {}, velocity {:?}, look {:?}", st.phase, st.ids.len(), v.sneaking(), v.velocity(), v.look()),
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (place prediction: pillar jump and bridge)
pub fn step(op: &PredOp, age: u64, v: &dyn BotView, c: &mut dyn BotCommands, st: &mut PredState) -> Option<Info> {
    if age == 0 {
        *st = PredState { min_y: f64::MAX, ..PredState::default() };
    }
    let p = v.position();
    st.min_y = st.min_y.min(p.y);
    if age > MAX_TICKS && st.phase != Phase::Done {
        st.gave_up = true;
        c.walk(0, 0);
        c.sneak(false);
        return Some(info(v, st));
    }
    match op {
        PredOp::Pillar { n } => {
            match st.phase {
                Phase::Start => {
                    if age == 0 {
                        c.look(0.0, 90.0);
                    } else if age > LOOK_TICKS && v.on_ground() {
                        st.start_y = p.y;
                        st.col = (p.x.floor() as i32, p.z.floor() as i32);
                        st.phase = Phase::Idle;
                        st.since = age;
                    }
                }
                Phase::Idle => {
                    if st.k == *n {
                        // all jumps done: read the results once they are in
                        let settled = st.ids.iter().all(|id| v.action(*id).is_finished());
                        if settled || age > st.since + 80 {
                            st.phase = Phase::Done;
                            return Some(info(v, st));
                        }
                    } else if v.on_ground() && age > st.since + 1 {
                        // standing on the block it placed: the expected height
                        if (p.y - (st.start_y + st.k as f64)).abs() > 0.02 {
                            if age > st.since + 40 {
                                st.gave_up = true;
                                return Some(info(v, st));
                            }
                        } else {
                            st.base_y = p.y;
                            c.jump();
                            st.k += 1;
                            st.phase = Phase::Up;
                        }
                    }
                }
                Phase::Up => {
                    if p.y >= st.base_y + PLACE_ABOVE {
                        // the block under the feet when the jump began
                        let under = BlockPos::new(st.col.0, st.base_y.round() as i32 - 1, st.col.1);
                        st.ids.push(c.place_block(under, Face::Up, false));
                        st.phase = Phase::Down;
                    }
                }
                Phase::Down => {
                    if v.on_ground() {
                        st.landed.push(p.y);
                        st.phase = Phase::Idle;
                        st.since = age;
                    }
                }
                _ => {}
            }
            None
        }
        PredOp::Bridge { support, count } => {
            if st.support.is_none() {
                st.support = Some(*support);
            }
            let sup = st.support.unwrap();
            match st.phase {
                Phase::Start => {
                    if age == 0 {
                        c.look(0.0, 0.0);
                        c.sneak(true);
                    } else if age >= CROUCH_TICKS {
                        st.phase = Phase::Back;
                    }
                }
                Phase::Back => {
                    if (p.z - st.last_z).abs() > 0.005 {
                        st.last_z = p.z;
                        st.moved_at = age;
                    } else if age > st.moved_at + 60 {
                        // the bot does not move: report where it stands rather than wait
                        st.gave_up = true;
                        c.walk(0, 0);
                        c.sneak(false);
                        return Some(info(v, st));
                    }
                    // the eye must be past the support's north plane (z = support.z) to see that face
                    if p.z < sup.z as f64 - 0.08 {
                        c.walk(0, 0);
                        st.since = age;
                        st.phase = Phase::Settle;
                    } else {
                        c.walk(-1, 0);
                    }
                }
                Phase::Settle => {
                    // aim from where the bot rests: the face is a hand's breadth away, and a glide
                    // of a tenth of a block after the aim moves the eye off the ray
                    let vel = v.velocity();
                    if (age >= st.since + 3 && vel.z.abs() < 0.004 && vel.x.abs() < 0.004) || age > st.since + 20 {
                        c.look_at_block(sup, Some(Face::North));
                        st.since = age;
                        st.phase = Phase::Aim;
                    }
                }
                Phase::Aim => {
                    if age >= st.since + LOOK_TICKS {
                        st.ids.push(c.place_block(sup, Face::North, false));
                        st.k += 1;
                        st.support = Some(sup.offset(Face::North));
                        st.phase = if st.k == *count { Phase::Idle } else { Phase::Back };
                        st.since = age;
                    }
                }
                Phase::Idle => {
                    let settled = st.ids.iter().all(|id| v.action(*id).is_finished());
                    if settled || age > st.since + 80 {
                        st.phase = Phase::Done;
                        c.sneak(false);
                        return Some(info(v, st));
                    }
                }
                _ => {}
            }
            None
        }
    }
}
