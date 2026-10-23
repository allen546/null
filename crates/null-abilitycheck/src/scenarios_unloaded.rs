//! A23: a fall in a chunk the client has not received yet (fork patch 38). Plan:
//! specs/B1-movement.md §4.2. A vanilla client with no chunk under the player does not apply gravity
//! (`LivingEntity#travelInAir`: -0.1 a tick above the world bottom, 0 below it), so the bot must do
//! the same. Grim does not simulate there; it resyncs the player until it has the chunk, and that
//! resync can draw alerts a vanilla client would draw too (F5 §4.6), so the row allows them.
//! The row builds a stone floor in a column 3000 blocks away, frees it (forceload off) and waits
//! until the server has unloaded it, so the server must load it again when the bot arrives and the
//! client has no chunk under its feet for some ticks. RCON sends the bot there mid-air. The bot
//! falls, the chunk arrives, the bot lands on the floor. Server truth: RCON position. Grim and the
//! server log: `Ctx::run_allowing`. Resistance V removes fall damage.
//! Area: corner (1070, 342) for the start; the far floor is at x = 1070 + 3000, same z.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use null_bot_api::BlockPos;

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome},
    skill::{Info, Op},
    scenarios_block_effects::try_bot_pos,
};

const AREA: (i32, i32) = (1070, 342);
const FAR_DX: i32 = 3000;
/// The height above the far floor the bot arrives at: a fall long enough that the chunk arrives mid-air.
const DROP: f64 = 40.0;
/// Half-width in blocks of the generated square around the far floor: 6 chunks of view distance plus a margin.
const NEAR: i32 = 108;

// Spec: specs/B1-movement.md §4.2 (travel in an unloaded chunk)
pub fn a23(cx: &mut Ctx) -> Vec<Outcome> {
    let n = cx.name.clone();
    let start = (AREA.0 as f64 + 10.5, (FLOOR_Y + 1) as f64, AREA.1 as f64 + 10.5);
    cx.scene_in(AREA, &[], start);
    let (fx0, fz0) = (AREA.0 + FAR_DX, AREA.1);
    let (fx1, fz1) = (fx0 + 20, fz0 + 20);
    let far = (fx0 as f64 + 10.5, fz0 as f64 + 10.5);
    let floor = (FLOOR_Y + 1) as f64;
    // The server must be able to send the far chunk at once when the bot arrives: a vanilla
    // server kicks a player held in the air for 80 ticks ("Flying is not enabled",
    // `ServerGamePacketListenerImpl#tickPlayer`), as in Pi pair p38b_full (kick 4 s after the teleport).
    // So the chunks around it (view distance 6) are generated and saved first; only reading the
    // saved chunk is left for the arrival. Spec: specs/reference/grim-a23-resync-loop.md (floating kick)
    let (bx, bz) = (far.0 as i32, far.1 as i32);
    let (wx0, wz0, wx1, wz1) = (bx - NEAR, bz - NEAR, bx + NEAR, bz + NEAR);
    // Vanilla `ServerLevel#setChunkForced` calls `getChunk` on the main thread for each newly forced
    // chunk, so one command for ~196 chunks outlasts the 30 s RCON read on a slow host (Pi pair p39_full).
    // Strips of 32 blocks (2 chunk rows) keep each command short. Paper loads in the background.
    let strips: Vec<(i32, i32)> = (0..).map(|k| wz0 + k * 32).take_while(|z| *z <= wz1).map(|z| (z, (z + 31).min(wz1))).collect();
    for (z0, z1) in &strips {
        cx.r(&format!("forceload add {wx0} {z0} {wx1} {z1}"));
    }
    // wait for the far corners
    let ready = cx.poll(120.0, |cx| {
        [(wx0, wz0), (wx1, wz1)].iter().all(|(x, z)| !cx.r(&format!("execute if loaded {x} {FLOOR_Y} {z}")).contains("failed"))
    });
    for c in [
        format!("fill {fx0} {FLOOR_Y} {fz0} {fx1} {FLOOR_Y} {fz1} minecraft:stone"),
        format!("fill {fx0} {} {fz0} {fx1} {} {fz1} minecraft:air", FLOOR_Y + 1, FLOOR_Y + 60),
        format!("effect give {n} minecraft:resistance 120 4 true"),
    ] {
        cx.r(&c);
    }
    // the same strips for the remove, for the same reason as the add
    for (z0, z1) in &strips {
        cx.r(&format!("forceload remove {wx0} {z0} {wx1} {z1}"));
    }
    // Wait until the server has dropped the far chunk (it is kept only while a ticket holds it).
    let unloaded = cx.poll(90.0, |cx| cx.r(&format!("execute if loaded {} {} {}", far.0 as i32, FLOOR_Y, far.1 as i32)).contains("failed"));
    // On Paper + Grim the fall can draw Simulation and GroundSpoof as a vanilla client would:
    // Grim's unloaded-chunk resync and vanilla's send threshold (F5 §4.6, OPEN item 0).
    let out = cx.run_allowing("A23", "fall_into_unsent_chunk", &["Simulation", "GroundSpoof"], "Grim's unloaded-chunk resync loop (F5 §4.6)", |cx| {
        if ready.is_none() {
            return (false, "the server did not load the chunks around the far floor in 120 s, so the row tests nothing".into());
        }
        if unloaded.is_none() {
            return (false, "the server did not unload the far chunk in 90 s, so the row tests nothing".into());
        }
        cx.r(&format!("execute in minecraft:overworld run tp {n} {} {} {} 0 0", far.0, floor + DROP, far.1));
        let t = Instant::now();
        let (mut min_y, mut last, mut still_since) = (f64::MAX, f64::MAX, None::<Instant>);
        let mut settled = None;
        // when the server first saw the bot move, and its first y (the wait before motion is the evidence on a slow chunk load)
        let (mut first_y, mut first_move) = (None::<f64>, None::<f64>);
        // the bot's own view: when its world first holds the far floor, and where it was then
        // (tells a client that lacks the chunk from a server that holds the bot)
        let probe = BlockPos::new(fx0 + 10, FLOOR_Y, fz0 + 10);
        let (mut chunk_seen, mut client_y_then, mut client_first_move) = (None::<f64>, None::<f64>, None::<f64>);
        while t.elapsed() < Duration::from_secs(25) {
            let seen = matches!(cx.call(Op::Peek(probe)).info, Info::Block(Some(_)));
            let cy = cx.snap().pos.y;
            if client_first_move.is_none() && (cy - (floor + DROP)).abs() > 0.005 {
                client_first_move = Some(t.elapsed().as_secs_f64());
            }
            if seen && chunk_seen.is_none() {
                chunk_seen = Some(t.elapsed().as_secs_f64());
                client_y_then = Some(cy);
            }
            if let Some(p) = try_bot_pos(cx) {
                min_y = min_y.min(p[1]);
                let y0 = *first_y.get_or_insert(p[1]);
                if first_move.is_none() && (p[1] - y0).abs() > 0.005 {
                    first_move = Some(t.elapsed().as_secs_f64());
                }
                // At rest for 1 s near the floor: it landed. Rest high up is no landing: the server
                // holds the bot there until it has the chunk, and the row waits for the fall.
                if (p[1] - last).abs() < 0.005 && p[1] <= floor + 0.5 {
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
        // The chunk must come within the server's 80-tick floating limit (4 s); later is a slow
        // server, not a fall the row can judge.
        let prompt = chunk_seen.is_some_and(|s| s < 3.0);
        let ok = prompt && settled.is_some_and(|p| p[1] >= floor - 0.01 && p[1] <= floor + 0.5) && min_y >= floor - 0.01;
        (
            ok,
            format!(
                "far chunk unloaded after {:.0} s; teleported {DROP} blocks above the far floor (y {floor}); settled after {:.2} s at {settled:?}; server saw first motion at {first_move:?} s (first y {first_y:?}); client chunk seen at {chunk_seen:?} s with the bot at y {client_y_then:?}, client first moved at {client_first_move:?} s; lowest y seen {min_y:.3}; chunk within 3 s: {prompt}",
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
