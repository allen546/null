//! A23: a fall in a chunk the client has not received yet (fork patch 38). Plan:
//! specs/B1-movement.md §4.2. A vanilla client with no chunk under the player does not apply gravity
//! (`LivingEntity#travelInAir`: -0.1 a tick above the world bottom, 0 below it), so the bot must do
//! the same or Grim's prediction (`Simulation`) disagrees on the first ticks after a teleport.
//! The row builds a stone floor in a column 3000 blocks away, frees it (forceload off) and waits
//! until the server has unloaded it, so the server must load it again when the bot arrives and the
//! client has no chunk under its feet for some ticks. RCON sends the bot there mid-air. The bot
//! falls, the chunk arrives, the bot lands on the floor. Server truth: RCON position. Grim and the
//! server log: `Ctx::run`. Resistance V removes fall damage.
//! Area: corner (1070, 342) for the start; the far floor is at x = 1070 + 3000, same z.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome},
    scenarios_block_effects::try_bot_pos,
};

const AREA: (i32, i32) = (1070, 342);
const FAR_DX: i32 = 3000;
/// The height above the far floor the bot arrives at: a fall long enough that the chunk arrives mid-air.
const DROP: f64 = 40.0;

// Spec: specs/B1-movement.md §4.2 (travel in an unloaded chunk)
pub fn a23(cx: &mut Ctx) -> Vec<Outcome> {
    let n = cx.name.clone();
    let start = (AREA.0 as f64 + 10.5, (FLOOR_Y + 1) as f64, AREA.1 as f64 + 10.5);
    cx.scene_in(AREA, &[], start);
    let (fx0, fz0) = (AREA.0 + FAR_DX, AREA.1);
    let (fx1, fz1) = (fx0 + 20, fz0 + 20);
    let far = (fx0 as f64 + 10.5, fz0 as f64 + 10.5);
    let floor = (FLOOR_Y + 1) as f64;
    for c in [
        format!("forceload add {fx0} {fz0} {fx1} {fz1}"),
        format!("fill {fx0} {FLOOR_Y} {fz0} {fx1} {FLOOR_Y} {fz1} minecraft:stone"),
        format!("fill {fx0} {} {fz0} {fx1} {} {fz1} minecraft:air", FLOOR_Y + 1, FLOOR_Y + 60),
        format!("forceload remove {fx0} {fz0} {fx1} {fz1}"),
        format!("effect give {n} minecraft:resistance 120 4 true"),
    ] {
        cx.r(&c);
    }
    // Wait until the server has dropped the far chunk (it is kept only while a ticket holds it).
    let unloaded = cx.poll(90.0, |cx| cx.r(&format!("execute if loaded {} {} {}", far.0 as i32, FLOOR_Y, far.1 as i32)).contains("failed"));
    let out = cx.run("A23", "fall_into_unsent_chunk", |cx| {
        if unloaded.is_none() {
            return (false, "the server did not unload the far chunk in 90 s, so the row tests nothing".into());
        }
        cx.r(&format!("execute in minecraft:overworld run tp {n} {} {} {} 0 0", far.0, floor + DROP, far.1));
        let t = Instant::now();
        let (mut min_y, mut last, mut still_since) = (f64::MAX, f64::MAX, None::<Instant>);
        let mut settled = None;
        while t.elapsed() < Duration::from_secs(25) {
            if let Some(p) = try_bot_pos(cx) {
                min_y = min_y.min(p[1]);
                // at rest for 1 s: it landed (the server may hold the bot at the start while it loads the chunk)
                if (p[1] - last).abs() < 0.005 {
                    let since = *still_since.get_or_insert_with(Instant::now);
                    if since.elapsed() > Duration::from_secs(1) {
                        settled = Some(p);
                        break;
                    }
                } else {
                    still_since = None;
                }
                last = p[1];
            }
            sleep(Duration::from_millis(25));
        }
        let ok = settled.is_some_and(|p| p[1] >= floor - 0.01 && p[1] <= floor + 0.5) && min_y >= floor - 0.01;
        (
            ok,
            format!(
                "far chunk unloaded after {:.0} s; teleported {DROP} blocks above the far floor (y {floor}); settled after {:.2} s at {settled:?}; lowest y seen {min_y:.3}",
                unloaded.unwrap_or(-1.0),
                t.elapsed().as_secs_f64()
            ),
        )
    });
    // Back to the start area, with the effect off.
    cx.r(&format!("execute in minecraft:overworld run tp {n} {} {} {} 0 0", start.0, start.1, start.2));
    cx.r(&format!("effect clear {n}"));
    sleep(Duration::from_millis(1500));
    cx.clean_in(AREA);
    vec![out]
}
