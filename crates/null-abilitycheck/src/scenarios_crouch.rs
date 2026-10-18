//! Crouch handling for placing clicks (milestone SA, A4C). Plan: specs/plans/ABILITY_CHECK.md.
//! A click on a block that uses the click (a chest, a note block) places the held item only while
//! crouching (vanilla `MultiPlayerGameMode#performUseItemOn`). Each block has three rows: standing
//! with `auto_crouch` off (refused on the client, nothing sent), standing with it on (crouch,
//! click next tick, stand again), and already crouching with it off (placed, still crouching).
//! The server's truth is read back over RCON; the crouch itself is the bot's own input state.

use std::{thread::sleep, time::Duration};

use null_bot_api::{BlockOutcome, BlockPos, Face};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, pos},
    skill::{Info, Op},
    skill_inventory::InvOp,
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (650, 342);
/// The bot stands here, facing south; the block under test is `T`, three blocks ahead.
const STAND: (f64, f64, f64) = (660.5, 200.0, 348.5);
const T: (i32, i32, i32) = (660, 200, 351);

/// The refusal text the fork gives (`BlockActionRefusal::BlockUsesClick`).
const REASON: &str = "block uses the click: crouch first (or auto_crouch) to place";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Standing, `auto_crouch` off: refused on the client.
    StandingOff,
    /// Standing, `auto_crouch` on: placed, standing again after.
    StandingAuto,
    /// Crouching, `auto_crouch` off: placed, still crouching after.
    CrouchedOff,
}

fn target() -> BlockPos {
    pos(T.0, T.1, T.2)
}

fn reset(cx: &mut Ctx) {
    let n = cx.name.clone();
    for c in [
        format!("fill 657 {} 346 664 {} 356 minecraft:air", FLOOR_Y + 1, FLOOR_Y + 7),
        format!("fill 657 {FLOOR_Y} 346 664 {FLOOR_Y} 356 minecraft:stone"),
        format!("kill @e[type=minecraft:item,x=650,y=199,z=342,dx=21,dy=10,dz=21]"),
        format!("clear {n}"),
    ] {
        cx.r(&c);
    }
}

/// One row: `block` at `T`, the bot holds stone and clicks its top face.
// Spec: specs/B6-placement-breaking.md §4.2 (crouch handling for placing clicks)
fn row(cx: &mut Ctx, block: &'static str, state: &'static str, mode: Mode) -> Outcome {
    reset(cx);
    let n = cx.name.clone();
    let t = target();
    cx.r(&format!("setblock {} {} {} minecraft:{state}", t.x, t.y, t.z));
    cx.wait_block(t, Box::leak(format!("minecraft:{block}").into_boxed_str()));
    cx.r(&format!("item replace entity {n} hotbar.0 with minecraft:stone 1"));
    sleep(Duration::from_millis(400));
    let name = match mode {
        Mode::StandingOff => format!("{block}_standing_auto_crouch_off_refused"),
        Mode::StandingAuto => format!("{block}_standing_auto_crouch_on_places_and_stands_again"),
        Mode::CrouchedOff => format!("{block}_crouched_auto_crouch_off_places_and_stays_crouched"),
    };
    cx.run("A4C", &name, |cx| {
        if mode == Mode::CrouchedOff {
            cx.call(Op::Sneak(true));
        }
        let before = cx.count("stone");
        let rep = cx.call(Op::PlaceAimCrouch { pos: t, face: Face::Up, auto_crouch: mode == Mode::StandingAuto });
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        sleep(Duration::from_millis(400));
        let sneaking = cx.snap().sneaking;
        if mode == Mode::CrouchedOff {
            cx.call(Op::Sneak(false));
        }
        let after = cx.count("stone");
        let above = pos(t.x, t.y + 1, t.z);
        let placed = cx.block_is(above, "stone");
        // The block's own action did not happen: a note block keeps note 0; a chest opened no screen.
        let untouched = if block == "note_block" {
            cx.block_is(t, "note_block[note=0]")
        } else {
            match cx.call(Op::Inv(InvOp::Status)).info {
                Info::Inv(i) => i.window.is_none_or(|w| w.id == 0),
                _ => false,
            }
        };
        let (want_out, want_placed, want_sneak) = match mode {
            Mode::StandingOff => (BlockOutcome::Refused, false, false),
            Mode::StandingAuto => (BlockOutcome::Done, true, false),
            Mode::CrouchedOff => (BlockOutcome::Done, true, true),
        };
        let reason_ok = match mode {
            Mode::StandingOff => a.reason == Some(REASON) && a.ack_tick.is_none(),
            _ => a.reason.is_none(),
        };
        let items_ok = after == if want_placed { before - 1 } else { before };
        (
            a.outcome == want_out && reason_ok && placed == want_placed && items_ok && untouched && sneaking == want_sneak,
            format!(
                "top face of {block}, holding stone; bot outcome {:?} (want {want_out:?}), reason {:?}; stone above on server: {placed} (want {want_placed}); stone count {before} -> {after}; block's own action did not happen: {untouched}; bot crouching afterwards: {sneaking} (want {want_sneak})",
                a.outcome, a.reason
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.2 (crouch handling: refuse, auto_crouch, already crouching)
pub fn a4c(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let mut out = Vec::new();
    for (block, state) in [("note_block", "note_block[note=0]"), ("chest", "chest[facing=south]")] {
        for mode in [Mode::StandingOff, Mode::StandingAuto, Mode::CrouchedOff] {
            out.push(row(cx, block, state, mode));
        }
    }
    cx.clean_in(AREA);
    out
}
