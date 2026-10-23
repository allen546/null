//! A22: place prediction, stage 1 (fork patch 33). Spec: specs/B6-placement-breaking.md §4.10; plan: specs/plans/ABILITY_CHECK.md. A vanilla client writes a placed block into its own world
//! at once and rolls it back if the server disagrees; the bot's physics then reads the block the
//! server and Grim already have. Rows:
//! - a pillar jump: five jumps, a block placed under the feet at the top of each, no wait for any
//!   ack; the bot ends five blocks up, standing, and the server agrees;
//! - a bridge: the bot sneaks backward off an edge and places three blocks without falling;
//! - a place the server refuses (the clicked block is outside the world border) shows the block
//!   in the bot's world until the ack and the server's state after it;
//! - inside the movement zone, a block outside the stage 1 families (a fence) is refused on the
//!   client and nothing is sent; so is a place next to a fence, which would change its shape;
//!   outside the zone the fence is sent unpredicted and placed.
//!
//! A probe in the bot's `GameTick` copies the cells the row watches after the physics, so a block
//! the client wrote and took back within one round trip shows. Server truth is read by RCON; the
//! server log is scanned for warnings and Grim alerts on every row. Area: corner (1040, 342),
//! floor y=199.

use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

use azalea::{
    account::Account,
    app::{App, Plugin},
    core::tick::GameTick,
    ecs::prelude::*,
    entity::{HasClientLoaded, LocalEntity},
    interact::BlockStatePredictionHandler,
    local_player::WorldHolder,
    physics::PhysicsSystems,
    registry::builtin::BlockKind,
};
use null_bot_api::{ActionRecord, BlockAction, BlockOutcome, Face};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, pos},
    scenarios_block_effects::bot_pos,
    skill::{Info, Op},
    skill_predict::{PredInfo, PredOp},
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (1040, 342);
/// The column the bot stands in, facing +z.
const COL: (i32, i32) = (1050, 352);
const STAND: (f64, f64, f64) = (1050.5, 200.0, 352.5);

/// The cells the probe watches, as (x, y, z).
pub static CELLS: Mutex<Vec<(i32, i32, i32)>> = Mutex::new(Vec::new());
pub static PROBE_ON: AtomicBool = AtomicBool::new(false);
/// Per tick, after the physics: the name of each watched cell as the bot's world holds it.
pub static TRACE: Mutex<Vec<Vec<String>>> = Mutex::new(Vec::new());
/// Per tick, before the physics step and after the place click: the first watched cell's name
/// and whether a place is still unacked.
pub static PRE_TRACE: Mutex<Vec<(String, bool)>> = Mutex::new(Vec::new());

// Spec: specs/B6-placement-breaking.md §4.10 (place prediction: the bot's world after each tick)
/// Adds the probe to the bot's app. `main` is the runner's account.
pub struct PredictionProbe {
    pub main: String,
}

impl Plugin for PredictionProbe {
    fn build(&self, app: &mut App) {
        let main = self.main.clone();
        app.add_systems(
            GameTick,
            (move |q: Query<(&Account, &WorldHolder), (With<LocalEntity>, With<HasClientLoaded>)>| {
                if !PROBE_ON.load(Ordering::Relaxed) {
                    return;
                }
                for (a, w) in &q {
                    if a.username() == main {
                        let world = w.shared.read();
                        let cells = CELLS.lock().unwrap().clone();
                        let row = cells
                            .iter()
                            .map(|&(x, y, z)| {
                                world
                                    .get_block_state(azalea::BlockPos::new(x, y, z))
                                    .map_or("unloaded".to_string(), |s| BlockKind::from(s).to_str().trim_start_matches("minecraft:").to_string())
                            })
                            .collect();
                        TRACE.lock().unwrap().push(row);
                    }
                }
            })
            .after(PhysicsSystems)
            .ambiguous_with_all(),
        );
        // The same cell the moment before the physics step, after the tick's place click: the
        // name, and whether a prediction is still unacked (`seq` above `acked_seq`).
        let main = self.main.clone();
        app.add_systems(
            GameTick,
            (move |q: Query<(&Account, &WorldHolder, &BlockStatePredictionHandler), (With<LocalEntity>, With<HasClientLoaded>)>| {
                if !PROBE_ON.load(Ordering::Relaxed) {
                    return;
                }
                for (a, w, p) in &q {
                    if a.username() == main
                        && let Some(&(x, y, z)) = CELLS.lock().unwrap().first()
                    {
                        let name = w
                            .shared
                            .read()
                            .get_block_state(azalea::BlockPos::new(x, y, z))
                            .map_or("unloaded".to_string(), |s| BlockKind::from(s).to_str().trim_start_matches("minecraft:").to_string());
                        PRE_TRACE.lock().unwrap().push((name, p.seq() > p.acked_seq()));
                    }
                }
            })
            .after(azalea::block_actions::handle_place_block_queued)
            .before(PhysicsSystems)
            .ambiguous_with_all(),
        );
    }
}

fn watch(cells: &[(i32, i32, i32)]) {
    *CELLS.lock().unwrap() = cells.to_vec();
    TRACE.lock().unwrap().clear();
    PRE_TRACE.lock().unwrap().clear();
    PROBE_ON.store(true, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(150));
}

/// The samples of watched cell `i`, in tick order.
fn unwatch(i: usize) -> Vec<String> {
    PROBE_ON.store(false, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
    std::mem::take(&mut *TRACE.lock().unwrap()).into_iter().filter_map(|r| r.get(i).cloned()).collect()
}

/// The names in order, repeats joined: `air air dirt dirt air` -> `air dirt air`.
fn runs(names: &[String]) -> Vec<&str> {
    let mut out: Vec<&str> = Vec::new();
    for n in names {
        if out.last() != Some(&n.as_str()) {
            out.push(n);
        }
    }
    out
}

fn block(st: &null_bot_api::ActionState) -> Option<BlockAction> {
    match st.finished() {
        Some(ActionRecord::Block(b)) => Some(*b),
        _ => None,
    }
}

fn predict(cx: &mut Ctx, op: crate::skill_predict::PredOp) -> Result<PredInfo, String> {
    match cx.call(Op::Predict(op)).info {
        Info::Predict(p) => Ok(p),
        i => Err(format!("bot reported {i:?}")),
    }
}

/// The finished block action of a place op, or why not.
fn placed(cx: &mut Ctx, pos: null_bot_api::BlockPos, face: Face) -> Result<BlockAction, String> {
    match cx.call(Op::PlaceAim { pos, face }).info {
        Info::Action(a) => Ok(a),
        i => Err(format!("bot reported {i:?}")),
    }
}

// Spec: specs/B6-placement-breaking.md §4.10 (a pillar jump stands on the block it just placed)
fn pillar_jump(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "dirt", 8)], STAND);
    cx.run("A22", "pillar_jump_places_under_feet", |cx| {
        let before = cx.count("dirt");
        let pi = match predict(cx, PredOp::Pillar { n: 5 }) {
            Ok(p) => p,
            Err(e) => return (false, e),
        };
        std::thread::sleep(std::time::Duration::from_millis(500));
        let outcomes: Vec<_> = pi.states.iter().map(|s| block(s).map(|b| b.outcome)).collect();
        let all_done = pi.states.len() == 5 && outcomes.iter().all(|o| *o == Some(BlockOutcome::Done));
        let want: Vec<f64> = (1..=5).map(|k| (FLOOR_Y + 1 + k) as f64).collect();
        let heights_ok = pi.landed.len() == 5 && pi.landed.iter().zip(&want).all(|(a, b)| (a - b).abs() < 0.02);
        let mut dirt_at = Vec::new();
        for k in 0..5 {
            dirt_at.push(cx.block_is(pos(COL.0, FLOOR_Y + 1 + k, COL.1), "dirt"));
        }
        let after = cx.count("dirt");
        let srv = bot_pos(cx);
        let srv_ok = (srv[1] - (FLOOR_Y + 6) as f64).abs() < 0.01 && (srv[0] - STAND.0).abs() < 0.5 && (srv[2] - STAND.2).abs() < 0.5;
        (
            all_done && heights_ok && dirt_at.iter().all(|d| *d) && after == before - 5 && srv_ok && pi.on_ground && !pi.gave_up,
            format!(
                "5 jumps, a block placed under the feet at the top of each, no wait for an ack: outcomes {outcomes:?} (want Done x5); standing heights after each jump {:?} (want {want:?}); bot ended on the ground {} at {:?}, gave up {}; server: bot at {srv:?} (want y {}), dirt in the column {dirt_at:?}, dirt {before} -> {after} (want five fewer)",
                pi.landed, pi.on_ground, pi.end, pi.gave_up, FLOOR_Y + 6
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (a bridge: walk onto each block as soon as it is placed)
fn bridge(cx: &mut Ctx) -> Outcome {
    // Solid ground from z = 350; the cells at z 342..349 are air, with a catch floor far below.
    let edge_z = 350;
    cx.scene_in(AREA, &[(0, "dirt", 8)], (STAND.0, STAND.1, edge_z as f64 + 0.5));
    cx.r(&format!("fill {} {FLOOR_Y} {} {} {FLOOR_Y} {} minecraft:air", AREA.0, AREA.1, AREA.0 + 20, edge_z - 1));
    cx.r(&format!("fill {} {} {} {} {} {} minecraft:stone", AREA.0, FLOOR_Y - 9, AREA.1, AREA.0 + 20, FLOOR_Y - 9, AREA.1 + 20));
    cx.wait_block(pos(COL.0, FLOOR_Y, edge_z - 1), "minecraft:air");
    cx.run("A22", "bridge_three_blocks", |cx| {
        let before = cx.count("dirt");
        let pi = match predict(cx, PredOp::Bridge { support: pos(COL.0, FLOOR_Y, edge_z), count: 3 }) {
            Ok(p) => p,
            Err(e) => return (false, e),
        };
        std::thread::sleep(std::time::Duration::from_millis(500));
        let outcomes: Vec<_> = pi.states.iter().map(|s| block(s).map(|b| (b.outcome, b.reason))).collect();
        let all_done = pi.states.len() == 3 && outcomes.iter().all(|o| o.is_some_and(|o| o.0 == BlockOutcome::Done));
        let mut dirt_at = Vec::new();
        for k in 1..=3 {
            dirt_at.push(cx.block_is(pos(COL.0, FLOOR_Y, edge_z - k), "dirt"));
        }
        let after = cx.count("dirt");
        let srv = bot_pos(cx);
        // never fell: the lowest height is the standing height, and the bot is out over the bridge
        let held = pi.min_y > (FLOOR_Y + 1) as f64 - 0.05 && srv[1] > (FLOOR_Y + 1) as f64 - 0.05;
        let out = pi.end.2 < (edge_z - 1) as f64 - 0.3;
        (
            all_done && held && out && dirt_at.iter().all(|d| *d) && after == before - 3 && !pi.gave_up,
            format!(
                "sneaking backward off the edge at z {edge_z}, 3 blocks placed one after another without waiting for an ack: outcomes {outcomes:?} (want Done x3); lowest height {:.3} (want over {}), bot ended at {:?} (want z under {}), gave up {} ({}); server: bot at {srv:?}, dirt at the three cells {dirt_at:?}, dirt {before} -> {after} (want three fewer)",
                pi.min_y,
                (FLOOR_Y + 1) as f64 - 0.05,
                pi.end,
                (edge_z - 1) as f64 - 0.3,
                pi.gave_up,
                pi.note
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (the write comes before the physics step of the click tick)
/// The pillar and bridge rows also pass with prediction off, since the server's update lands
/// before the bot reaches the block. This row looks at the moment itself: after the tick's click
/// and before its physics step, the bot's world already holds the block while the place is still
/// unacked. A server update cannot be that early (the ack and the update come together, one
/// round trip after the click), so only the client's own write can make the cell dirt there.
fn write_before_physics(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "dirt", 8)], STAND);
    let clicked = pos(COL.0 + 1, FLOOR_Y, COL.1);
    let target = (COL.0 + 1, FLOOR_Y + 1, COL.1);
    cx.run("A22", "write_precedes_the_physics_step", |cx| {
        watch(&[target]);
        let a = placed(cx, clicked, Face::Up);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let post = unwatch(0);
        let pre = std::mem::take(&mut *PRE_TRACE.lock().unwrap());
        let a = match a {
            Ok(a) => a,
            Err(e) => return (false, e),
        };
        // the first tick the place was unacked: the click tick
        let first_unacked = pre.iter().position(|(_, unacked)| *unacked);
        let early = first_unacked.map(|i| pre[i].0.clone());
        let post_same_tick = first_unacked.and_then(|i| post.get(i).cloned());
        let dirt = cx.block_is(pos(target.0, target.1, target.2), "dirt");
        (
            a.outcome == BlockOutcome::Done && early.as_deref() == Some("dirt") && post_same_tick.as_deref() == Some("dirt") && dirt,
            format!(
                "dirt placed beside the feet; on the first tick with the place unacked, the cell before the physics step is {early:?} and after it {post_same_tick:?} (want dirt both: written by the client at the click, the ack not yet in); outcome {:?}; server: dirt at the target {dirt}; before-physics trace {pre:?}",
                a.outcome
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (write and roll back: the server's state after the ack)
fn rejected_place_rolls_back(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "dirt", 8)], STAND);
    let clicked = pos(COL.0 + 1, FLOOR_Y, COL.1);
    let target = (COL.0 + 1, FLOOR_Y + 1, COL.1);
    // A border from x 1042 to 1051: the bot's cell is inside, the clicked block is outside, so
    // the server refuses the use (`ServerLevel#mayInteract`) and resends the two cells.
    // (the command puts a whole-number centre at the block's middle: x 1046.5, so the border
    // is x 1042 to 1051 and the cell x 1051 is outside)
    crate::scenarios_envelope::bring_others_inside(cx, (COL.0 - 7, COL.1 - 3));
    cx.r(&format!("worldborder center {} {}", COL.0 - 4, COL.1));
    cx.r("worldborder set 9");
    std::thread::sleep(std::time::Duration::from_millis(400));
    let out = cx.run("A22", "rejected_place_rolls_back", |cx| {
        let before = cx.count("dirt");
        watch(&[target]);
        let a = placed(cx, clicked, Face::Up);
        std::thread::sleep(std::time::Duration::from_millis(400));
        let seen = unwatch(0);
        let a = match a {
            Ok(a) => a,
            Err(e) => return (false, e),
        };
        let r = runs(&seen);
        let wrote_then_rolled_back = r.contains(&"dirt") && r.last() == Some(&"air");
        let air = cx.block_is(pos(target.0, target.1, target.2), "air");
        let after = cx.count("dirt");
        (
            a.outcome == BlockOutcome::Refused && a.ack_tick.is_some() && a.reason.is_none() && wrote_then_rolled_back && air && after == before,
            format!(
                "place outcome {:?} (want Refused by the server: acked {}, client refusal {:?}); the bot's world at the target, tick by tick: {r:?} (want dirt written at once, then air after the ack); server: air at the target {air}, dirt {before} -> {after} (want unchanged)",
                a.outcome,
                a.ack_tick.is_some(),
                a.reason
            ),
        )
    });
    cx.r("worldborder set 59999968");
    cx.r("worldborder center 0.0 0.0");
    out
}

// Spec: specs/B6-placement-breaking.md §4.10 (the safety rule: inside the zone, predicted or refused)
fn unknown_family_in_zone_refused(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "oak_fence", 4)], STAND);
    let clicked = pos(COL.0 + 1, FLOOR_Y, COL.1);
    let target = pos(COL.0 + 1, FLOOR_Y + 1, COL.1);
    cx.run("A22", "unknown_family_in_zone_refused", |cx| {
        let before = cx.count("oak_fence");
        let a = match placed(cx, clicked, Face::Up) {
            Ok(a) => a,
            Err(e) => return (false, e),
        };
        std::thread::sleep(std::time::Duration::from_millis(400));
        let air = cx.block_is(target, "air");
        let after = cx.count("oak_fence");
        (
            a.outcome == BlockOutcome::Refused && a.ack_tick.is_none() && a.reason.is_some_and(|r| r.contains("cannot be predicted")) && air && after == before,
            format!(
                "fence one block from the bot's feet: outcome {:?}, client refusal {:?}, acked {} (want Refused on the client, no ack, so nothing was sent); server: air at the target {air}, fences {before} -> {after} (want unchanged: a valid place would have been accepted)",
                a.outcome,
                a.reason,
                a.ack_tick.is_some()
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (outside the zone a place stage 1 cannot compute is sent as before)
fn unknown_family_outside_zone_sent(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "oak_fence", 4)], STAND);
    let clicked = pos(COL.0 + 3, FLOOR_Y, COL.1);
    let target = pos(COL.0 + 3, FLOOR_Y + 1, COL.1);
    cx.run("A22", "unknown_family_outside_zone_sent_unpredicted", |cx| {
        let before = cx.count("oak_fence");
        let a = match placed(cx, clicked, Face::Up) {
            Ok(a) => a,
            Err(e) => return (false, e),
        };
        std::thread::sleep(std::time::Duration::from_millis(400));
        let fence = cx.block_is(target, "oak_fence");
        let after = cx.count("oak_fence");
        (
            a.outcome == BlockOutcome::Done && a.reason.is_none() && fence && after == before - 1,
            format!(
                "fence three blocks from the bot (outside its movement zone): outcome {:?}, client refusal {:?} (want Done, not refused); server: fence at the target {fence}, fences {before} -> {after} (want one fewer)",
                a.outcome, a.reason
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10 (neighbour shape updates are stage 3: refuse in the zone)
fn reactive_neighbour_in_zone_refused(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[(0, "dirt", 8)], STAND);
    let clicked = pos(COL.0 + 1, FLOOR_Y, COL.1);
    let target = pos(COL.0 + 1, FLOOR_Y + 1, COL.1);
    // The fence stands beside the target cell, inside the zone: the dirt would change its shape.
    let fence = pos(COL.0 + 1, FLOOR_Y + 1, COL.1 + 1);
    cx.r(&format!("setblock {} {} {} minecraft:oak_fence", fence.x, fence.y, fence.z));
    cx.wait_block(fence, "minecraft:oak_fence");
    cx.run("A22", "place_beside_a_fence_in_zone_refused", |cx| {
        let before = cx.count("dirt");
        let a = match placed(cx, clicked, Face::Up) {
            Ok(a) => a,
            Err(e) => return (false, e),
        };
        std::thread::sleep(std::time::Duration::from_millis(400));
        let air = cx.block_is(target, "air");
        let after = cx.count("dirt");
        (
            a.outcome == BlockOutcome::Refused && a.ack_tick.is_none() && a.reason.is_some_and(|r| r.contains("neighbour")) && air && after == before,
            format!(
                "dirt beside a fence inside the zone: outcome {:?}, client refusal {:?}, acked {} (want Refused on the client, no ack); server: air at the target {air}, dirt {before} -> {after} (want unchanged)",
                a.outcome,
                a.reason,
                a.ack_tick.is_some()
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.10
pub fn a22(cx: &mut Ctx) -> Vec<Outcome> {
    let out = vec![
        pillar_jump(cx),
        bridge(cx),
        write_before_physics(cx),
        rejected_place_rolls_back(cx),
        unknown_family_in_zone_refused(cx),
        unknown_family_outside_zone_sent(cx),
        reactive_neighbour_in_zone_refused(cx),
    ];
    cx.r(&format!("fill {} {} {} {} {} {} minecraft:air", AREA.0, FLOOR_Y - 9, AREA.1, AREA.0 + 20, FLOOR_Y - 9, AREA.1 + 20));
    cx.clean_in(AREA);
    out
}
