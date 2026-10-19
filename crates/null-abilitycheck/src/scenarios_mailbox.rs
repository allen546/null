//! Milestone SA, A13: the result mailbox (AD-F5.4). The old readback kept one record per kind, so a
//! skill that sent the next break before reading the last ack lost that result. Here the skill
//! sends without waiting and reads every id at the end. Plan: specs/plans/ABILITY_CHECK.md.

use null_bot_api::{ActionRecord, ActionState, BlockOutcome, Face, Vec3};

use crate::{
    scenarios::{Ctx, Outcome, pos, ticks_between_packets, vanilla_ticks},
    skill::{Info, Op},
    skill_mailbox::MailOp,
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (770, 342);
const STAND: (f64, f64, f64) = (780.5, 200.0, 352.5);
/// Floor y, as the scene lays it.
const FLOOR_Y: i32 = crate::scenarios::FLOOR_Y;

/// The break record inside a state, partial or final.
fn block(s: &ActionState) -> Option<null_bot_api::BlockAction> {
    match s.record() {
        Some(ActionRecord::Block(b)) => Some(*b),
        _ => None,
    }
}

// Spec: specs/B6-placement-breaking.md §4.6 (results read by id after the next command was sent)
fn chain_without_waiting(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "iron_axe", 1)], STAND);
    // Four rows of four logs, level with the bot's eyes (y = floor + 2), one row per direction.
    let (sx, sz) = (STAND.0 as i32, STAND.2 as i32);
    let y = FLOOR_Y + 2;
    let lines: Vec<Vec<_>> = vec![
        (1..=4).map(|k| pos(sx + k, y, sz)).collect(),
        (1..=4).map(|k| pos(sx - k, y, sz)).collect(),
        (1..=4).map(|k| pos(sx, y, sz + k)).collect(),
        (1..=4).map(|k| pos(sx, y, sz - k)).collect(),
    ];
    let all: Vec<_> = lines.iter().flatten().copied().collect();
    for l in &lines {
        let (a, b) = (l[0], l[l.len() - 1]);
        cx.r(&format!("fill {} {} {} {} {} {} minecraft:oak_log", a.x, a.y, a.z, b.x, b.y, b.z));
    }
    cx.wait_block(*all.last().unwrap(), "minecraft:oak_log");
    let n = vanilla_ticks(2.0, 6.0, true);
    let want = ticks_between_packets(n) as u64;
    cx.run("A13", "chain_of_breaks_sent_without_waiting_for_acks", |cx| {
        let rep = cx.call(Op::Mail(MailOp::ChainNoWait(lines.clone())));
        let Info::Mail(m) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        std::thread::sleep(std::time::Duration::from_millis(500));
        let mut not_done = Vec::new();
        let mut bad_ticks = Vec::new();
        for (p, _issued, st) in &m.breaks {
            let b = block(st);
            if !st.is_finished() || b.is_none_or(|b| b.outcome != BlockOutcome::Done) {
                not_done.push((*p, st.clone()));
            }
            if let Some(b) = b {
                if b.finish_tick.map(|f| f - b.start_tick) != Some(want) {
                    bad_ticks.push((*p, b.finish_tick.map(|f| f - b.start_tick)));
                }
            }
        }
        // A command sent while the previous result was still pending: with one record per kind the
        // skill would have lost the previous result here.
        let (pending_at_send, followers) = m.followers_pending.unwrap_or((0, 0));
        let mut still = Vec::new();
        for p in &all {
            if !cx.block_is(*p, "air") {
                still.push(*p);
            }
        }
        (
            m.breaks.len() == all.len() && not_done.is_empty() && bad_ticks.is_empty() && still.is_empty() && followers == 12 && pending_at_send == followers,
            format!(
                "{} breaks sent, each the tick after the last one's finishing packet and without a look; all read by id at the end; {} of {} Done and finished (want all; off: {not_done:?}); each took {want} ticks between packets (vanilla {n}; off: {bad_ticks:?}); {pending_at_send} of {followers} in-line follower commands went out while the previous break's result was still pending (want all 12, or the row proves nothing); blocks still not air on the server: {still:?}",
                m.breaks.len(),
                m.breaks.len() - not_done.len(),
                all.len()
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.6 (a break and a place in flight together)
fn place_right_after_break(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "dirt", 4)], STAND);
    // Sand by hand takes 15 ticks. The bot looks past it at the floor beneath: once the sand is
    // gone the crosshair falls on the floor's top face, and the placement fills the hole.
    let (sx, sz) = (STAND.0 as i32, STAND.2 as i32);
    let sand = pos(sx + 1, FLOOR_Y + 1, sz);
    let floor = pos(sx + 1, FLOOR_Y, sz);
    cx.r(&format!("setblock {} {} {} minecraft:sand", sand.x, sand.y, sand.z));
    cx.wait_block(sand, "minecraft:sand");
    cx.run("A13", "place_sent_right_after_a_break_both_results_read", |cx| {
        let before = cx.count("dirt");
        let aim = Vec3::new(floor.x as f64 + 0.5, floor.y as f64 + 1.0, floor.z as f64 + 0.5);
        let rep = cx.call(Op::Mail(MailOp::BreakThenPlace { brk: sand, on: floor, face: Face::Up, aim }));
        let Info::Mail(m) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        std::thread::sleep(std::time::Duration::from_millis(500));
        let after = cx.count("dirt");
        let dirt_there = cx.block_is(sand, "dirt");
        let brk = m.breaks.first().and_then(|(_, _, s)| block(s));
        let plc = m.place.as_ref().and_then(|(_, s)| block(s));
        let b_done = brk.is_some_and(|b| b.outcome == BlockOutcome::Done);
        let p_done = plc.is_some_and(|p| p.outcome == BlockOutcome::Done);
        let gap = match (&m.breaks.first(), &m.place, brk) {
            (Some((_, _, _)), Some((pt, _)), Some(b)) => Some((*pt as i64) - (b.start_tick as i64 + b.finish_tick.map_or(0, |f| f - b.start_tick) as i64)),
            _ => None,
        };
        (
            b_done && p_done && dirt_there && after == before - 1 && m.expired_after_release == Some(true),
            format!(
                "break outcome {:?}, place outcome {:?} (want Done both; place command applied {gap:?} ticks after the break's finishing tick, before the break's ack {:?}); server: dirt now at the sand's spot {dirt_there}, dirt count {before} -> {after} (want one fewer); both ids read Expired the tick after release: {:?}",
                brk.map(|b| b.outcome),
                plc.map(|p| p.outcome),
                brk.and_then(|b| b.ack_tick),
                m.expired_after_release
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.6
pub fn a13(cx: &mut Ctx) -> Vec<Outcome> {
    let out = vec![chain_without_waiting(cx), place_right_after_break(cx)];
    cx.clean_in(AREA);
    out
}
