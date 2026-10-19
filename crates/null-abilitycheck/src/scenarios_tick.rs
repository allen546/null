//! Tick catch-up like the vanilla client (milestone SA, AT). Plan: specs/plans/ABILITY_CHECK.md.
//! After a stall the client runs the ticks due, at most 10 back to back, drops the rest and keeps
//! the fraction (vanilla `Minecraft#runTick`, `DeltaTracker.Timer#advanceGameTime`; fork patch 23).
//! A test hook holds the bot's ECS for a while inside a tick (`Op::Stall`). The fork counts, for
//! the update with the most ticks due, what was due, what ran and how many
//! `ServerboundClientTickEnd` it queued. On Paper + Grim the stall draws `Timer` alerts, which
//! AD-F5.3 expects (a vanilla client would too); they go in the evidence and do not fail the row.

use std::{sync::atomic::Ordering::Relaxed, thread::sleep, time::Duration};

use azalea::tick_clock as tc;

use crate::{
    scenarios::{Ctx, Outcome, last_int, pos},
    skill::Op,
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
    cx.run_allowing("AT", &format!("stall_{ms}_ms_runs_vanillas_count_and_drops_the_rest"), &["Timer"], |cx| {
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

// Spec: specs/F5-version-layer.md §4.6 (tick catch-up like vanilla)
pub fn at(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let bots = online(cx);
    let out = vec![row(cx, 400, bots, 718, 4), row(cx, 1500, bots, 715, 6)];
    cx.clean_in(AREA);
    out
}
