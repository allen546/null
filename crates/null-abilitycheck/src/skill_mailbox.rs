//! Bot-side ops for A13 (the result mailbox, AD-F5.4). `RemoteSkill::step` calls `step` for
//! `Op::Mail`; everything goes through `BotView` / `BotCommands` only.
//! The ops send the next command without waiting for the last one's ack and read every id at the
//! end, which the old one-record-per-kind readback could not do.
// Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox)

use null_bot_api::{
    ActionId, ActionRecord, ActionState, BlockPos, BotCommands, BotView, Face, Vec3,
};

use crate::skill::{Info, look_at};

/// Two ticks between the look and the first click: the hit result is recomputed after the look lands.
const LOOK_TICKS: u64 = 2;
/// The op gives up after this many ticks, before the runner's 60 s wait.
const MAX_TICKS: u64 = 1000;

#[derive(Clone, Debug)]
pub enum MailOp {
    /// Each line is a row of blocks straight ahead of the bot, level with its eyes. The bot looks
    /// down the line once; each next block's break command goes out the tick after the previous
    /// block's finishing packet, with no look in between (the crosshair falls through the air the
    /// client predicted) and no wait for the ack.
    ChainNoWait(Vec<Vec<BlockPos>>),
    /// Break `brk`, and the tick after its finishing packet place on the `face` of `on`, which
    /// the crosshair falls on once `brk` is gone. Then release both ids and read them once more.
    BreakThenPlace { brk: BlockPos, on: BlockPos, face: Face, aim: Vec3 },
    /// Look at `aim`, place on the `face` of `first`, and in the very next tick place on the
    /// `face` of `second` (the block the first placement made, which the crosshair now falls on),
    /// with no wait for the first ack. Then read both ids.
    TwoPlaces { first: BlockPos, face1: Face, second: BlockPos, face2: Face, aim: Vec3 },
}

#[derive(Default)]
pub struct MailState {
    line: usize,
    idx: usize,
    looked_at: Option<u64>,
    /// Breaks sent: block, id, host tick of the command.
    issued: Vec<(BlockPos, ActionId, u64)>,
    /// For each break sent without a look in between: the previous break's result was still
    /// pending when the command went out (the case the old readback lost).
    follower_sent_while_prev_pending: Vec<bool>,
    brk: Option<(ActionId, u64)>,
    place: Option<(ActionId, u64)>,
    place2: Option<(ActionId, u64)>,
    /// `TwoPlaces`: the first placement's result was still pending when the second went out.
    second_sent_while_first_pending: Option<bool>,
    released_at: Option<u64>,
    /// The states when both finished, before the release.
    before_release: Option<(ActionState, ActionState)>,
}

#[derive(Clone, Debug)]
pub struct MailInfo {
    /// Per break: block, host tick of its command, its state when read at the end.
    pub breaks: Vec<(BlockPos, u64, ActionState)>,
    /// `BreakThenPlace`: the placement's command tick and state.
    pub place: Option<(u64, ActionState)>,
    /// `ChainNoWait`: how many in-line breaks went out while the previous break's result was still
    /// pending, out of how many were in-line.
    pub followers_pending: Option<(usize, usize)>,
    /// `BreakThenPlace`: both ids read `Expired` the tick after `release`.
    pub expired_after_release: Option<bool>,
    /// `TwoPlaces`: the second placement's command tick and state, and whether the first one's
    /// result was still pending when the second went out.
    pub second_place: Option<(u64, ActionState, bool)>,
}

/// The finishing packet of the break behind `id` has gone out (its partial record says so), or the
/// result is final.
fn sent(v: &dyn BotView, id: ActionId) -> bool {
    match v.action(id) {
        ActionState::Finished(_) => true,
        ActionState::Pending(Some(ActionRecord::Block(b))) => b.finish_tick.is_some(),
        _ => false,
    }
}

// Spec: specs/B6-placement-breaking.md §4.6 (several results in flight at once, read by id)
pub fn step(op: &MailOp, age: u64, v: &dyn BotView, c: &mut dyn BotCommands, st: &mut MailState) -> Option<Info> {
    if age == 0 {
        *st = MailState::default();
    }
    match op {
        MailOp::ChainNoWait(lines) => {
            let now = v.tick();
            if let Some(blocks) = lines.get(st.line) {
                let prev_sent = st.issued.last().is_none_or(|(_, id, _)| sent(v, *id));
                if st.idx == 0 {
                    match st.looked_at {
                        // Looking away from a block still being mined would abort it.
                        None if prev_sent => {
                            look_at(v, c, blocks[0].center());
                            st.looked_at = Some(age);
                        }
                        Some(t) if age >= t + LOOK_TICKS => {
                            st.issued.push((blocks[0], c.break_block(blocks[0]), now));
                            st.idx = 1;
                        }
                        _ => {}
                    }
                } else if prev_sent {
                    let prev_pending = st.issued.last().is_some_and(|(_, id, _)| v.action(*id).is_pending());
                    st.follower_sent_while_prev_pending.push(prev_pending);
                    st.issued.push((blocks[st.idx], c.break_block(blocks[st.idx]), now));
                    st.idx += 1;
                }
                if st.idx >= blocks.len() {
                    st.line += 1;
                    st.idx = 0;
                    st.looked_at = None;
                }
                return (age > MAX_TICKS).then(|| finish_chain(v, c, st));
            }
            let all = st.issued.iter().all(|(_, id, _)| v.action(*id).is_finished());
            (all || age > MAX_TICKS).then(|| finish_chain(v, c, st))
        }
        MailOp::BreakThenPlace { brk, on, face, aim } => {
            let now = v.tick();
            if age == 0 {
                look_at(v, c, *aim);
            } else if age == LOOK_TICKS {
                st.brk = Some((c.break_block(*brk), now));
            }
            if let Some((b, _)) = st.brk
                && st.place.is_none()
                && sent(v, b)
            {
                st.place = Some((c.place_block(*on, *face, false), now));
            }
            if let (Some((b, _)), Some((p, _))) = (st.brk, st.place) {
                match st.released_at {
                    None if v.action(b).is_finished() && v.action(p).is_finished() => {
                        st.before_release = Some((v.action(b), v.action(p)));
                        c.release(b);
                        c.release(p);
                        st.released_at = Some(age);
                    }
                    Some(t) if age > t => {
                        let expired = v.action(b).is_expired() && v.action(p).is_expired();
                        let (sb, sp) = st.before_release.clone().unwrap();
                        return Some(Info::Mail(MailInfo {
                            breaks: vec![(*brk, st.brk.unwrap().1, sb)],
                            place: Some((st.place.unwrap().1, sp)),
                            expired_after_release: Some(expired),
                            followers_pending: None,
                            second_place: None,
                        }));
                    }
                    _ => {}
                }
            }
            (age > MAX_TICKS).then(|| {
                let state = |o: Option<(ActionId, u64)>| o.map(|(i, t)| (t, v.action(i)));
                Info::Mail(MailInfo {
                    breaks: st.brk.map(|(i, t)| (*brk, t, v.action(i))).into_iter().collect(),
                    place: state(st.place),
                    expired_after_release: None,
                    followers_pending: None,
                    second_place: None,
                })
            })
        }
        MailOp::TwoPlaces { first, face1, second, face2, aim } => {
            let now = v.tick();
            if age == 0 {
                look_at(v, c, *aim);
            } else if age == LOOK_TICKS {
                st.place = Some((c.place_block(*first, *face1, false), now));
            } else if age == LOOK_TICKS + 1 {
                let pending = st.place.is_some_and(|(i, _)| v.action(i).is_pending());
                st.second_sent_while_first_pending = Some(pending);
                st.place2 = Some((c.place_block(*second, *face2, false), now));
            }
            if let (Some((a, ta)), Some((b, tb))) = (st.place, st.place2) {
                if v.action(a).is_finished() && v.action(b).is_finished() || age > MAX_TICKS {
                    let (sa, sb) = (v.action(a), v.action(b));
                    c.release(a);
                    c.release(b);
                    return Some(Info::Mail(MailInfo {
                        breaks: Vec::new(),
                        place: Some((ta, sa)),
                        followers_pending: None,
                        expired_after_release: None,
                        second_place: Some((tb, sb, st.second_sent_while_first_pending.unwrap_or(false))),
                    }));
                }
            }
            (age > MAX_TICKS).then(|| Info::Mail(MailInfo {
                breaks: Vec::new(),
                place: None,
                followers_pending: None,
                expired_after_release: None,
                second_place: None,
            }))
        }
    }
}

/// Reads every break's id once and lets them go.
fn finish_chain(v: &dyn BotView, c: &mut dyn BotCommands, st: &MailState) -> Info {
    let breaks = st.issued.iter().map(|(p, id, t)| (*p, *t, v.action(*id))).collect();
    for (_, id, _) in &st.issued {
        c.release(*id);
    }
    let f = &st.follower_sent_while_prev_pending;
    Info::Mail(MailInfo {
        breaks,
        place: None,
        expired_after_release: None,
        followers_pending: Some((f.iter().filter(|p| **p).count(), f.len())),
        second_place: None,
    })
}
