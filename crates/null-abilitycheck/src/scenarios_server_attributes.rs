//! A16: attributes and effects the server sets for the bot (fork patch 27). Plan:
//! specs/plans/ABILITY_CHECK.md. RCON gives the bot an attribute (`attribute <bot> ... base set`,
//! `modifier add`) and the server's idea of what the bot did is compared with a small reference
//! model written from the vanilla formulas (`LivingEntity#travelInAir`, `#jumpFromGround`,
//! `AttributeInstance#calculateValue`, `Player#getDestroySpeed`); it shares no code with the fork.
//! A bot that ignored the attribute would walk, jump or break at the default and end far from the
//! model; Grim's Simulation and FastBreak checks would alert. Inputs the fork does not model
//! (levitation, slow falling) stop the skill and keep the bot connected (`Action`); the ones that
//! move the bot by themselves (scale, dolphin's grace) make the fork close the connection
//! (`Movement`, patch 30; spare accounts, see scenarios_envelope).
//! Every row clears its attribute afterwards. Area: corner (860, 342), floor y=199, one lane per row.

use std::{
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::{Duration, Instant},
};

use mc_adapter::HostEvent;
use null_bot_api::SkillStatus;

use crate::{
    scenarios::{Acts, Ctx, FLOOR_Y, Outcome, pos, ticks_between_packets, vanilla_ticks},
    scenarios_block_effects::bot_pos,
    skill::{Info, Op},
    skill_items::ItemOp,
};

const AREA: (i32, i32) = (860, 342);
const LANE0: i32 = 863;
const START_Z: f64 = 345.5;
pub(crate) const WALK: u32 = 20;
const COAST: u32 = 40;
/// Velocity kept per tick on the ground: block friction 0.6 times 0.91.
const KEEP: f64 = 0.6 * 0.91;
pub(crate) const TOL: f64 = 0.005;

/// Set by a row that expects the host to end the runner's skill; the node plugin then starts a fresh one.
pub static REVIVE: AtomicBool = AtomicBool::new(false);

/// Vanilla's walk along z from rest: `input_ticks` ticks of input worth `per_tick` of velocity each
/// (ground speed times the 0.98 of `LivingEntity#aiStep`, times the sneaking factor when crouched),
/// then `COAST` ticks without.
pub(crate) fn walk_model(z0: f64, input_ticks: u32, per_tick: f64) -> f64 {
    let (mut z, mut v) = (z0, 0.0_f64);
    for t in 0..input_ticks + COAST {
        if v * v < 9.0e-6 {
            v = 0.0;
        }
        if t < input_ticks {
            v += per_tick;
        }
        z += v;
        v *= KEEP;
    }
    z
}

/// Highest point above the start of a standing jump: the jump sets the upward speed, the move adds
/// it, then gravity and the 0.98 drag (`LivingEntity#jumpFromGround`, `#travelInAir`).
fn apex_model(jump: f64, gravity: f64) -> f64 {
    let (mut y, mut vy, mut best) = (0.0_f64, jump, 0.0_f64);
    for _ in 0..400 {
        y += vy;
        best = best.max(y);
        vy = (vy - gravity) * 0.98f32 as f64;
        if y <= 0.0 && vy < 0.0 {
            break;
        }
    }
    best
}

fn attr(cx: &mut Ctx, a: &str, rest: &str) -> String {
    let n = cx.name.clone();
    cx.r(&format!("attribute {n} minecraft:{a} {rest}"))
}

fn reading(cx: &mut Ctx, a: &str) -> String {
    attr(cx, a, "get").trim().to_string()
}

/// Row lane `i`: floor, air and walls restored around it, the bot at the lane's start, settled.
fn lane(cx: &mut Ctx, i: i32) -> i32 {
    let x = LANE0 + i;
    let n = cx.name.clone();
    cx.r(&format!("fill {} {FLOOR_Y} {} {} {FLOOR_Y} {} minecraft:stone", x - 2, AREA.1, x + 2, AREA.1 + 20));
    cx.r(&format!("fill {} {} {} {} {} {} minecraft:air", x - 2, FLOOR_Y + 1, AREA.1, x + 2, FLOOR_Y + 6, AREA.1 + 20));
    cx.r(&format!("tp {n} {x}.5 {} {START_Z} 0 0", FLOOR_Y + 1));
    sleep(Duration::from_millis(1500));
    x
}

/// Runs the row's act `set`, the body, then (if `reset_after`) its act `reset`; the reset runs whatever the body found.
/// The acts are RCON inside the window, so the vanilla client's scene runs them at the same input tick.
fn with_attrs<T>(cx: &mut Ctx, acts: &Acts, reset_after: bool, body: impl FnOnce(&mut Ctx) -> T) -> T {
    cx.act(acts, "set");
    // the server sends the attribute packet within a tick or two; wait well past that
    sleep(Duration::from_millis(800));
    let out = body(cx);
    if reset_after {
        cx.act(acts, "reset");
    }
    sleep(Duration::from_millis(500));
    out
}

fn at(a: &str, rest: &str) -> String {
    format!("attribute {{n}} minecraft:{a} {rest}")
}

/// Lane and RCON acts (`set`, `reset`) of a row that uses them, by row name: the Null run and the vanilla
/// client's scene (`prepare_row`) both read this table.
// Spec: specs/plans/VD3_BRIEF.md T5
fn row_scene(name: &str) -> (i32, Acts) {
    let slow = "minecraft:a16_slow";
    let fast = "minecraft:a16_fast";
    let (lane, set, reset): (i32, Vec<String>, Vec<String>) = match name {
        "control_plain_walk_matches_the_model" => (0, vec![], vec![]),
        "movement_speed_modifier_halves_the_walk" => (1, vec![at("movement_speed", &format!("modifier add {slow} -0.5 add_multiplied_total"))], vec![at("movement_speed", &format!("modifier remove {slow}"))]),
        "movement_speed_base_0.2_doubles_the_walk" => (2, vec![at("movement_speed", "base set 0.2")], vec![at("movement_speed", "base reset")]),
        "sneaking_speed_base_0.6_doubles_the_crouch_walk" => (3, vec![at("sneaking_speed", "base set 0.6")], vec![at("sneaking_speed", "base reset")]),
        "control_default_jump_reaches_1.2522" => (4, vec![at("jump_strength", "base get")], vec![at("jump_strength", "base get")]),
        "jump_strength_base_0.84_jumps_higher" => (5, vec![at("jump_strength", "base set 0.84")], vec![at("jump_strength", "base reset")]),
        "gravity_base_0.04_floats_higher" => (6, vec![at("gravity", "base set 0.04")], vec![at("gravity", "base reset")]),
        "control_a_one_block_step_stops_the_default_bot" => (7, vec![], vec![at("step_height", "base reset")]),
        "step_height_base_1_climbs_a_one_block_step" => (8, vec![at("step_height", "base set 1.0")], vec![at("step_height", "base reset")]),
        "block_break_speed_x3_breaks_in_a_third_of_the_ticks" => (9, vec![at("block_break_speed", &format!("modifier add {fast} 2 add_multiplied_total"))], vec![at("block_break_speed", &format!("modifier remove {fast}"))]),
        "breaking_with_the_eyes_in_water_takes_5_times_longer" => (10, vec![], vec![]),
        "server_only_attribute_and_modelled_effect_do_not_stop_the_skill" => (
            11,
            vec![at("max_health", "base set 40"), at("armor", "base set 10"), "effect give {n} minecraft:jump_boost 20 0 true".into()],
            vec![at("max_health", "base reset"), at("armor", "base reset"), "effect clear {n} minecraft:jump_boost".into()],
        ),
        "levitation_is_an_action_entry_and_stops_the_skill_without_a_disconnect" => (12, vec!["effect give {n} minecraft:levitation 30 0 true".into()], vec!["effect clear {n} minecraft:levitation".into()]),
        "slow_falling_is_an_action_entry_and_stops_the_skill_without_a_disconnect" => (13, vec!["effect give {n} minecraft:slow_falling 30 0 true".into()], vec!["effect clear {n} minecraft:slow_falling".into()]),
        _ => panic!("no A16 scene for row {name}"),
    };
    (lane, vec![("set", set), ("reset", reset)])
}

/// Sets the scene of A16 row `row` for a vanilla client (`--prepare-scene A16:<row>`) and returns the row's RCON actions.
/// The `scale_2` and `dolphins_grace` rows are Null closing the connection on purpose: skipped in `rows.json`.
// Spec: specs/plans/VD3_BRIEF.md T5
pub(crate) fn prepare_row(cx: &mut Ctx, row: &str) -> Acts {
    cx.scene_in(AREA, &[], (LANE0 as f64 + 0.5, (FLOOR_Y + 1) as f64, START_Z));
    let (i, acts) = row_scene(row);
    let x = lane(cx, i);
    match row {
        "control_a_one_block_step_stops_the_default_bot" | "step_height_base_1_climbs_a_one_block_step" => step_scene(cx, x),
        "block_break_speed_x3_breaks_in_a_third_of_the_ticks" => {
            break_scene(cx, x, "oak_log", false);
        }
        "breaking_with_the_eyes_in_water_takes_5_times_longer" => {
            break_scene(cx, x, "dirt", true);
        }
        _ => {}
    }
    acts
}

fn walk(cx: &mut Ctx, ticks: u32) {
    match cx.call(Op::Items(ItemOp::Walk(ticks))).info {
        Info::Items(_) => {}
        i => panic!("expected an items report: {i:?}"),
    }
    sleep(Duration::from_millis(1500));
}

fn cmd(a: &str, rest: &str, n: &str) -> String {
    format!("attribute {n} minecraft:{a} {rest}")
}

/// A walk row: the lane's walk against the model with `per_tick`, and against the default's `plain`.
fn walk_row(cx: &mut Ctx, name: &str, a: &str, per_tick: f64, plain: f64, k: u32, sneak: bool) -> Outcome {
    let (i, acts) = row_scene(name);
    let x = lane(cx, i);
    cx.run("A16", name, |cx| {
        let tol = if sneak { 0.03 } else { TOL };
        let (before, after, read) = with_attrs(cx, &acts, true, |cx| {
            let read = reading(cx, a);
            if sneak {
                cx.call(Op::Sneak(true));
                sleep(Duration::from_millis(400));
            }
            let before = bot_pos(cx);
            walk(cx, WALK);
            let after = bot_pos(cx);
            if sneak {
                cx.call(Op::Sneak(false));
            }
            (before, after, read)
        });
        let want = walk_model(START_Z, k, per_tick);
        let def = walk_model(START_Z, k, plain);
        let dx = (after[0] - (x as f64 + 0.5)).abs();
        (
            (after[2] - want).abs() < tol && (after[2] - def).abs() > 0.3 && dx < 0.05,
            format!("server says {read}; walked {WALK} input ticks from z {:.3} to z {:.4} (vanilla model {want:.4}, off by {:.4}, limit {tol}); with the default it would end at {def:.4}; x drift {dx:.3}", before[2], after[2], (after[2] - want).abs()),
        )
    })
}

/// A standing jump: the server's highest y against the model.
fn apex_row(cx: &mut Ctx, name: &str, a: &str, jump: f64, gravity: f64) -> Outcome {
    let (i, acts) = row_scene(name);
    lane(cx, i);
    cx.run("A16", name, |cx| {
        let (top, read, y0) = with_attrs(cx, &acts, true, |cx| {
            let read = reading(cx, a);
            let y0 = bot_pos(cx)[1];
            // The server's y is sampled over RCON, which can skip the one tick that holds the
            // apex; the model's apex is the best of the ticks, so the best of three jumps is
            // compared with it.
            let mut top = y0;
            for _ in 0..3 {
                cx.send(Op::Items(ItemOp::Jump));
                let t = Instant::now();
                while t.elapsed() < Duration::from_millis(2500) {
                    top = top.max(bot_pos(cx)[1]);
                }
            }
            (top, read, y0)
        });
        let want = apex_model(jump, gravity);
        let def = apex_model(0.42f32 as f64, 0.08);
        let got = top - y0;
        (
            (got - want).abs() < 0.015 && (got - def).abs() > 0.1 || (name.starts_with("control") && (got - want).abs() < 0.015),
            format!("server says {read}; highest point of three jumps {got:.4} above the start (vanilla model {want:.4}, off by {:.4}, limit 0.015; default apex {def:.4})", (got - want).abs()),
        )
    })
}

/// The one-block step across the lane at z 349..355.
fn step_scene(cx: &mut Ctx, x: i32) {
    cx.r(&format!("fill {x} {} 349 {x} {} 355 minecraft:stone", FLOOR_Y + 1, FLOOR_Y + 1));
}

/// A step of one block in the walk's way, 25 input ticks.
fn step_row(cx: &mut Ctx, name: &str, climbs: bool) -> Outcome {
    let (i, acts) = row_scene(name);
    let x = lane(cx, i);
    step_scene(cx, x);
    cx.run("A16", name, |cx| {
        let (after, read) = with_attrs(cx, &acts, true, |cx| {
            let read = reading(cx, "step_height");
            walk(cx, 25);
            (bot_pos(cx), read)
        });
        let up = after[1] > (FLOOR_Y + 1) as f64 + 0.5;
        (
            up == climbs && if climbs { after[2] > 349.3 } else { after[2] < 349.0 },
            format!("server says {read}; after 25 input ticks the bot is at z {:.3}, y {:.3} (want {} the one-block step at z 349..355: on top, y {})", after[2], after[1], if climbs { "up on" } else { "stopped at" }, FLOOR_Y + 2),
        )
    })
}

/// The block to break at the lane's far end (and the water cell around the bot for the water row); returns its cell.
fn break_scene(cx: &mut Ctx, x: i32, block: &str, water: bool) -> null_bot_api::BlockPos {
    if water {
        cx.r(&format!("fill {} {FLOOR_Y} 344 {} {} 351 minecraft:stone hollow", x - 2, x + 2, FLOOR_Y + 5));
        cx.r(&format!("fill {} {} 345 {} {} 350 minecraft:water", x - 1, FLOOR_Y + 1, x + 1, FLOOR_Y + 4));
    }
    let target = pos(x, FLOOR_Y + 1, 348);
    cx.r(&format!("setblock {x} {} 348 minecraft:{block}", FLOOR_Y + 1));
    cx.wait_block(target, Box::leak(format!("minecraft:{block}").into_boxed_str()));
    sleep(Duration::from_millis(500));
    target
}

/// Breaking a block at the lane's far end with the attribute in force; ticks against vanilla.
fn break_row(cx: &mut Ctx, name: &str, block: &str, hardness: f32, speed: f32, water: bool) -> Outcome {
    let (i, acts) = row_scene(name);
    let x = lane(cx, i);
    let target = break_scene(cx, x, block, water);
    cx.run("A16", name, |cx| {
        let (rep, air) = with_attrs(cx, &acts, true, |cx| {
            let rep = cx.call(Op::BreakAim(target));
            sleep(Duration::from_millis(250));
            (rep, cx.block_is(target, "air") || cx.block_is(target, "water"))
        });
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        let n = vanilla_ticks(hardness, speed, true);
        let want = ticks_between_packets(n) as u64;
        let ticks = a.finish_tick.map(|f| f - a.start_tick);
        (
            air && a.outcome == null_bot_api::BlockOutcome::Done && ticks == Some(want),
            format!("{block}, hardness {hardness}, speed {speed}: vanilla {n} mining ticks = {want} between start and finish; bot {ticks:?} (outcome {:?}, reason {:?}); gone on server: {air}", a.outcome, a.reason),
        )
    })
}

/// The bot walks 20 ticks; true when it moved (the skill ran).
fn moved_by_walk(cx: &mut Ctx) -> (bool, f64) {
    let before = bot_pos(cx);
    walk(cx, WALK);
    let after = bot_pos(cx);
    ((after[2] - before[2]).abs() > 0.5, after[2] - before[2])
}

/// An `Action` entry (levitation, slow falling: modelled physics, but skills plan with normal
/// gravity): the walk is sent, the host ends the skill with the input named, the bot does not walk
/// and stays connected. Afterwards `reset` clears it and a fresh skill walks again.
fn stop_row(cx: &mut Ctx, name: &str, needle: &str) -> Outcome {
    let me = cx.name.clone();
    let (i, acts) = row_scene(name);
    lane(cx, i);
    cx.run("A16", name, |cx| {
        let (ended, reply, still, moved_after, connected, no_record) = {
            while cx.events.try_recv().is_ok() {}
            let (ended, reply, before, after) = with_attrs(cx, &acts, false, |cx| {
                let before = bot_pos(cx);
                let walk_seq = cx.send(Op::Items(ItemOp::Walk(WALK)));
                let t = Instant::now();
                let mut ended = None;
                while t.elapsed() < Duration::from_secs(6) && ended.is_none() {
                    if let Ok(HostEvent::SkillEnded { status, .. }) = cx.events.recv_timeout(Duration::from_millis(100)) {
                        ended = Some(status);
                    }
                }
                // The walk's own reply: `Ended` when the skill went first, else the op stays queued
                // for the next skill (its late reply is dropped by number).
                let reply = cx.recv(walk_seq, Duration::from_secs(2)).map(|r| format!("{:?}", r.info)).unwrap_or_else(|_| "none (op still queued)".into());
                sleep(Duration::from_millis(1500));
                (ended, reply, before, bot_pos(cx))
            });
            let me_seen = crate::scenarios_envelope::seen(&me);
            let connected = me_seen.as_ref().is_some_and(|s| s.connected);
            let no_record = me_seen.is_some_and(|s| s.disconnect.is_none());
            // clear the input, start a fresh skill, and walk again
            cx.act(&acts, "reset");
            sleep(Duration::from_millis(800));
            REVIVE.store(true, Ordering::Relaxed);
            let (moved, _) = moved_by_walk(cx);
            (ended, reply, (after[2] - before[2]).abs() < 0.1, moved, connected, no_record)
        };
        let why = match &ended {
            Some(SkillStatus::Failed(r)) => r.clone(),
            other => format!("{other:?}"),
        };
        (
            why.starts_with("unmodelled input") && why.contains(needle) && why.contains("Action") && still && moved_after && connected && no_record,
            format!("skill ended with {why:?} (want \"unmodelled input\" naming {needle}, kind Action); the walk's reply: {reply}; bot stayed put: {still}; still connected: {connected}, no disconnect record: {no_record}; after clearing it a new skill walks: {moved_after}"),
        )
    })
}

// Spec: specs/F5-version-layer.md §4.6 (server-set attributes, effects the client does not model)
pub fn a16(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], (LANE0 as f64 + 0.5, (FLOOR_Y + 1) as f64, START_Z));
    let mut out = Vec::new();
    // Control: plain walk fits the number of input ticks the op really gives (as A15).
    let plain = 0.1 * 0.98;
    lane(cx, 0);
    let mut k = WALK;
    out.push(cx.run("A16", "control_plain_walk_matches_the_model", |cx| {
        walk(cx, WALK);
        let after = bot_pos(cx);
        let (best, err) = (WALK.saturating_sub(2)..=WALK + 3)
            .map(|k| (k, (walk_model(START_Z, k, plain) - after[2]).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        k = best;
        (err < TOL, format!("walked {WALK} requested ticks to z {:.4}; the model matches with {best} input ticks (off by {err:.4}, limit {TOL})", after[2]))
    }));
    let n = cx.name.clone();
    out.push(walk_row(cx, "movement_speed_modifier_halves_the_walk", "movement_speed", plain * 0.5, plain, k, false));
    out.push(walk_row(cx, "movement_speed_base_0.2_doubles_the_walk", "movement_speed", plain * 2.0, plain, k, false));
    out.push(walk_row(cx, "sneaking_speed_base_0.6_doubles_the_crouch_walk", "sneaking_speed", plain * 0.6, plain * 0.3, k, true));
    out.push(apex_row(cx, "control_default_jump_reaches_1.2522", "jump_strength", 0.42f32 as f64, 0.08));
    out.push(apex_row(cx, "jump_strength_base_0.84_jumps_higher", "jump_strength", 0.84, 0.08));
    out.push(apex_row(cx, "gravity_base_0.04_floats_higher", "gravity", 0.42f32 as f64, 0.04));
    out.push(step_row(cx, "control_a_one_block_step_stops_the_default_bot", false));
    out.push(step_row(cx, "step_height_base_1_climbs_a_one_block_step", true));
    out.push(break_row(cx, "block_break_speed_x3_breaks_in_a_third_of_the_ticks", "oak_log", 2.0, 3.0, false));
    out.push(break_row(cx, "breaking_with_the_eyes_in_water_takes_5_times_longer", "dirt", 0.5, 0.2, true));
    // A server-only attribute and a modelled effect stop nothing.
    let name = "server_only_attribute_and_modelled_effect_do_not_stop_the_skill";
    let (i, acts) = row_scene(name);
    lane(cx, i);
    out.push(cx.run("A16", name, |cx| {
        let (moved, dz) = with_attrs(cx, &acts, true, moved_by_walk);
        (moved, format!("max_health 40, armor 10 and jump boost I in force: the walk moved the bot {dz:.3} blocks (want over 0.5)"))
    }));
    out.push(stop_row(cx, "levitation_is_an_action_entry_and_stops_the_skill_without_a_disconnect", "effect minecraft:levitation"));
    out.push(stop_row(cx, "slow_falling_is_an_action_entry_and_stops_the_skill_without_a_disconnect", "effect minecraft:slow_falling"));
    // Movement entries: the fork closes the connection; each row uses a spare account.
    let spare_at = (LANE0 as f64 + 12.5, (FLOOR_Y + 1) as f64, START_Z + 6.0);
    cx.r(&format!("fill {} {FLOOR_Y} {} {} {FLOOR_Y} {} minecraft:stone", spare_at.0 as i32 - 2, spare_at.2 as i32 - 2, spare_at.0 as i32 + 4, spare_at.2 as i32 + 2));
    out.push(crate::scenarios_envelope::disconnect_row(
        cx, "A16", "scale_2_is_a_movement_entry_and_disconnects_the_bot", 1, spare_at,
        &[], &["attribute {d} minecraft:scale base set 2".to_string()], "Scale",
    ));
    out.push(crate::scenarios_envelope::disconnect_row(
        cx, "A16", "dolphins_grace_is_a_movement_entry_and_disconnects_the_bot", 2, (spare_at.0 + 3.0, spare_at.1, spare_at.2),
        &[], &["effect give {d} minecraft:dolphins_grace 30 0 true".to_string()], "DolphinsGrace",
    ));
    // Leave the defaults behind.
    for c in [cmd("scale", "base reset", &n), format!("effect clear {n}")] {
        cx.r(&c);
    }
    cx.clean_in(AREA);
    out
}
