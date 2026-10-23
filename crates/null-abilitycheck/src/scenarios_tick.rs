//! Tick catch-up like the vanilla client (milestone SA, AT). Plan: specs/plans/ABILITY_CHECK.md.
//! After a stall the client runs the ticks due, at most 10 back to back, drops the rest and keeps
//! the fraction (vanilla `Minecraft#runTick`, `DeltaTracker.Timer#advanceGameTime`; fork patch 23).
//! A test hook holds the bot's ECS for a while inside a tick (`Op::Stall`). The fork counts, for
//! the update with the most ticks due, what was due, what ran and how many
//! `ServerboundClientTickEnd` it queued. On Paper + Grim the stall draws `Timer` alerts, which
//! AD-F5.3 expects (a vanilla client would too); they go in the evidence and do not fail the row.

use std::{sync::atomic::Ordering::Relaxed, thread::sleep, time::Duration};

use azalea::tick_clock as tc;

use null_bot_api::{BlockOutcome, Face};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, last_int, pos},
    skill::{Info, Op, describe_evidence},
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (710, 342);
const STAND: (f64, f64, f64) = (715.5, 200.0, 348.5);

/// Players online (the bot, and the A8 attacker account when it is up).
fn online(cx: &mut Ctx) -> u64 {
    let r = cx.r("list");
    r.strip_prefix("There are ").and_then(|s| s.split(' ').next()?.parse().ok()).unwrap_or(1)
}

// Spec: specs/F5-version-layer.md §4.6 (tick catch-up like vanilla)
fn row(cx: &mut Ctx, ms: u64, bots: u64, to_x: i32, slot: u8) -> Outcome {
    let n = cx.name.clone();
    cx.run_allowing("AT", &format!("stall_{ms}_ms_runs_vanillas_count_and_drops_the_rest"), &["Timer"], "the forced stall (AD-F5.3)", |cx| {
        sleep(Duration::from_millis(300));
        tc::reset_loop_stats();
        cx.call(Op::Stall(ms));
        let (due, ran, ends, dropped) =
            (tc::MAX_DUE.load(Relaxed), tc::RAN_AT_MAX_DUE.load(Relaxed), tc::TICK_ENDS_AT_MAX_DUE.load(Relaxed), tc::TICKS_DROPPED.load(Relaxed));
        // The stall is `ms` long, so about ms/50 ticks are due (a little over: the work around it).
        let want_due = ms / 50;
        let due_ok = (want_due..=want_due + 3).contains(&due);
        let want_ran = due.min(10);
        // Every client queues one tick-end packet per tick.
        let ok = due_ok && ran == want_ran && ends == ran * bots && dropped == due.saturating_sub(10);
        // The bot still works: a short walk and a slot select, read back from the server.
        let start = cx.snap().pos;
        let to = pos(to_x, 200, STAND.2 as i32);
        let walked = cx.snap_after(Op::Goto(to)).pos;
        let moved = walked.distance(to.center()) < 1.5 && walked.distance(start) > 1.5;
        cx.snap_after(Op::SelectSlot(slot));
        let server_slot = cx.poll(1.0, |cx| last_int(&cx.r(&format!("data get entity {n} SelectedItemSlot"))) == Some(slot as i64)).is_some();
        (
            ok && moved && server_slot,
            format!(
                "client stalled {ms} ms inside a tick: the update after it had {due} ticks due (want {want_due}..{}), ran {ran} (want {want_ran} = min(10, due)), queued {ends} ClientTickEnd ({bots} client(s) x {ran} ticks), dropped {dropped} (want {}); then walked 3 blocks (reached {walked:?}: {moved}) and selected slot {slot} (server agrees: {server_slot})",
                want_due + 3,
                due.saturating_sub(10)
            ),
        )
    })
}

/// One stalled click: the bot faces away, the loop stalls `ms`, then the look at the top face of
/// floor block `(x, FLOOR_Y, z)` (tick 1) and the click (tick 2) both fall in the catch-up ticks
/// of one update. A ray cast once per update would predate the look (the negative control fails here).
/// The crosshair ray is cast in every `GameTick` (fork patch 29; `Minecraft#tick` ->
/// `GameRenderer#pick`), so the click sees the new look even when several ticks run back to back.
/// Pass: the click is Done (a refusal would carry its evidence, including
/// `look_changed_after_hit`) and the server has the stone.
// Spec: specs/F5-version-layer.md §4.6 (the crosshair ray is cast once per tick); specs/B6-placement-breaking.md §4.2
fn click_row(cx: &mut Ctx, ms: u64, x: i32, z: i32) -> Outcome {
    let n = cx.name.clone();
    let floor = pos(x, FLOOR_Y, z);
    let above = pos(x, FLOOR_Y + 1, z);
    cx.r(&format!("setblock {x} {} {z} minecraft:air", FLOOR_Y + 1));
    cx.r(&format!("item replace entity {n} hotbar.0 with minecraft:stone 8"));
    sleep(Duration::from_millis(400));
    cx.run_allowing("AT", &format!("stall_{ms}_ms_then_look_and_click_in_one_update_places_at_the_new_look"), &["Timer"], "the forced stall (AD-F5.3)", |cx| {
        tc::reset_loop_stats();
        let rep = cx.call(Op::PlaceAimStall { pos: floor, face: Face::Up, ms });
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        let (due, ran) = (tc::MAX_DUE.load(Relaxed), tc::RAN_AT_MAX_DUE.load(Relaxed));
        sleep(Duration::from_millis(400));
        let placed = cx.block_is(above, "stone");
        let refusal = a.evidence.as_ref().map_or(String::new(), |e| format!("; {} ({:?})", describe_evidence(e), a.reason));
        let changed = a.evidence.as_ref().is_some_and(|e| e.look_changed_after_hit);
        (
            a.outcome == BlockOutcome::Done && placed && !changed,
            format!(
                "stalled {ms} ms, then look and click in one update (most ticks due in one update: {due}, ran {ran}); click on top of floor ({x},{z}): bot outcome {:?} (want Done), stone above on server: {placed}{refusal}",
                a.outcome
            ),
        )
    })
}

// Spec: specs/F5-version-layer.md §4.6 (tick catch-up like vanilla; the crosshair ray is cast once per tick)
pub fn at(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let bots = online(cx);
    let mut out = vec![row(cx, 400, bots, 718, 4), row(cx, 1500, bots, 715, 6)];
    // Back at the stand point after the walks; the two targets lie in opposite directions, so each
    // click needs a turn of well over 90 degrees.
    cx.scene_in(AREA, &[], STAND);
    out.push(click_row(cx, 250, 713, 346));
    out.push(click_row(cx, 250, 715, 351));
    cx.clean_in(AREA);
    out
}
