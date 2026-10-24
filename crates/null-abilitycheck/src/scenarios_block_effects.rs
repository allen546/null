//! A15: movement effects of the blocks the bot is in or on (fork patch 26). Plan:
//! specs/plans/ABILITY_CHECK.md. The bot walks with plain forward input into a cobweb, a sweet
//! berry bush and powdered snow, and across soul sand and honey. The server's idea of where the bot
//! ends up is read over RCON and compared with a small reference model written from the vanilla
//! formulas (`Entity#move` with the stuck multiplier, `Entity#getBlockSpeedFactor`, ground
//! friction 0.6 * 0.91); it shares no code with the fork. A bot that ignored the effects would
//! walk at normal speed, end up far from the model, and Grim's Simulation check would alert.
//! A control row on plain stone tells how many input ticks `ItemOp::Walk` really gives, so the
//! model needs no guess about the tick at which the input starts or stops.
//! Area: corner (830, 342), floor y=199; one lane (x cell) per row, the lane runs along +z.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, pos},
    skill::{Info, Op},
    skill_items::ItemOp,
};

const AREA: (i32, i32) = (830, 342);
/// First lane cell; row `i` uses `LANE0 + 2 * i`.
const LANE0: i32 = 832;
const START_Z: f64 = 345.5;
/// The lane cells with the block under test (cell z range, inclusive).
const BLOCKS: (i32, i32) = (348, 357);
/// Input ticks asked of `ItemOp::Walk`.
const WALK: u32 = 20;
/// Ticks of coasting after the input stops, enough for the velocity to fall under the cutoff.
const COAST: u32 = 40;
/// The velocity gained per tick at full forward input on the ground is speed 0.1 (on friction 0.6)
/// times the 0.98 that `LivingEntity#aiStep` applies to every impulse (walking speed 4.317
/// blocks/s). The velocity kept per tick on the ground is block friction 0.6 times 0.91.
const KEEP: f64 = 0.6 * 0.91;
const HALF_WIDTH: f64 = 0.3;
/// Largest difference between the server's z and the model's, in blocks. Both follow the same
/// tick arithmetic, so the true difference is far smaller (0.0000 on the first run).
const TOL: f64 = 0.005;

/// What the lane does to the walker, in the reference model.
#[derive(Clone, Copy)]
struct Lane {
    /// Stuck multiplier on z; the flag is true when only the feet cell counts (powdered snow),
    /// false when any overlap of the bot's box does (cobweb, berry bush).
    stuck: Option<(f64, bool)>,
    /// Velocity factor after each move (soul sand, honey).
    speed: f64,
    /// z of a wall the walker stops against.
    wall: Option<f64>,
    /// Powdered-snow frost: the server counts a tick in the snow, and the bot's movement speed
    /// drops by 0.05 * ticks / 140 (`LivingEntity#tryAddFrost`) once the attribute packet reaches
    /// it, `lag` ticks after the bot was in the snow.
    frost: bool,
}

const PLAIN: Lane = Lane { stuck: None, speed: 1.0, wall: None, frost: false };

fn overlaps(z: f64, cells: (i32, i32)) -> bool {
    z + HALF_WIDTH - 1e-5 > cells.0 as f64 && z - HALF_WIDTH + 1e-5 < (cells.1 + 1) as f64
}

/// Vanilla's walk along z from rest: `input_ticks` ticks of full input, then `COAST` ticks without.
// Spec: specs/B1-movement.md §4.2 (the reference model for A15, from the vanilla formulas)
fn model(z0: f64, input_ticks: u32, lane: Lane, lag: usize) -> f64 {
    let (mut z, mut v, mut stuck) = (z0, 0.0_f64, false);
    // Whether the feet were in the snow after each tick so far.
    let mut in_snow: Vec<bool> = Vec::new();
    for t in 0..input_ticks + COAST {
        // A velocity under 0.003 blocks a tick is zeroed (`LivingEntity#aiStep`).
        if v * v < 9.0e-6 {
            v = 0.0;
        }
        if t < input_ticks {
            let seen = in_snow.len().saturating_sub(lag);
            let frozen = if lane.frost { in_snow[..seen].iter().filter(|b| **b).count().min(140) } else { 0 };
            v += (0.1 - 0.05 * frozen as f64 / 140.0) * 0.98;
        }
        let mut dz = v;
        if stuck {
            // `Entity#move`: scale the movement by the multiplier, then clear it and the velocity.
            dz *= lane.stuck.map_or(1.0, |s| s.0);
            v = 0.0;
        }
        z += dz;
        if lane.wall.is_some_and(|w| z > w) {
            z = lane.wall.unwrap();
            v = 0.0;
        }
        // `Entity#getBlockSpeedFactor` scales the velocity after the move.
        v *= lane.speed;
        v *= KEEP;
        in_snow.push(lane.frost && (BLOCKS.0..=BLOCKS.1).contains(&(z.floor() as i32)));
        stuck = match lane.stuck {
            // powdered snow only counts when the feet cell is the snow itself
            Some((_, true)) => (BLOCKS.0..=BLOCKS.1).contains(&(z.floor() as i32)),
            Some((_, false)) => overlaps(z, BLOCKS),
            None => false,
        };
    }
    z
}

pub(crate) fn floats(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .filter_map(|t| t.parse().ok())
        .collect()
}

/// The server's position of the bot.
pub(crate) fn bot_pos(cx: &mut Ctx) -> [f64; 3] {
    try_bot_pos(cx).expect("the bot is not on the server")
}

/// The server's position of the bot, or `None` when it is not online (patch 30 disconnected it).
/// An online bot whose reply does not parse is retried, then panics with the raw replies (p38_full
/// stopped A16 on such a reply with no text kept).
// Spec: specs/plans/ABILITY_CHECK.md "Which modules a change reruns"
pub(crate) fn try_bot_pos(cx: &mut Ctx) -> Option<[f64; 3]> {
    let n = cx.name.clone();
    let mut replies = Vec::new();
    for attempt in 0..4 {
        let reply = cx.r(&format!("data get entity {n} Pos"));
        let f = floats(&reply);
        if let Some(k) = f.len().checked_sub(3).filter(|_| reply.contains('[')) {
            return Some([f[k], f[k + 1], f[k + 2]]);
        }
        let list = cx.r("list");
        let online = list.split(':').nth(1).is_some_and(|l| l.split(',').any(|x| x.trim() == n));
        replies.push(format!("pos reply {reply:?}, list reply {list:?}"));
        if !online {
            // Off the server only when a second look agrees (a wrong reading must not end the run).
            std::thread::sleep(std::time::Duration::from_millis(200));
            let again = cx.r("list");
            if !again.split(':').nth(1).is_some_and(|l| l.split(',').any(|x| x.trim() == n)) {
                eprintln!("the bot {n} is off the server: {}; list again {again:?}", replies.join("; "));
                return None;
            }
            replies.push(format!("second list reply {again:?} names the bot"));
        }
        if attempt < 3 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    }
    panic!("the bot {n} is online but its Pos reply does not parse after 4 tries: {}", replies.join("; "))
}

fn walk(cx: &mut Ctx, ticks: u32) {
    match cx.call(Op::Items(ItemOp::Walk(ticks))).info {
        Info::Items(_) => {}
        i => panic!("expected an items report: {i:?}"),
    }
    sleep(Duration::from_millis(1500));
}

/// Row lane `i`: floor and air restored, the bot standing at the lane's start, settled.
fn lane(cx: &mut Ctx, i: i32, z: f64, setup: &[String]) -> i32 {
    let x = LANE0 + 2 * i;
    let n = cx.name.clone();
    cx.r(&format!("fill {x} {FLOOR_Y} {} {x} {FLOOR_Y} {} minecraft:stone", AREA.1, AREA.1 + 20));
    cx.r(&format!("fill {x} {} {} {x} {} {} minecraft:air", FLOOR_Y + 1, AREA.1, FLOOR_Y + 4, AREA.1 + 20));
    for c in setup {
        cx.r(&c.replace("{x}", &x.to_string()));
    }
    cx.r(&format!("tp {n} {}.5 {} {z} 0 0", x, FLOOR_Y + 1));
    sleep(Duration::from_millis(1500));
    x
}

fn lane_fill(y0: i32, y1: i32, z0: i32, z1: i32, block: &str) -> String {
    format!("fill {{x}} {y0} {z0} {{x}} {y1} {z1} minecraft:{block}")
}

/// One walking row: the lane built by `setup`, the model `lane_model`, the control's input ticks.
fn effect_row(cx: &mut Ctx, i: i32, name: &str, ticks: u32, setup: Vec<String>, lane_model: Lane, z0: f64, min_gap: f64) -> Outcome {
    let x = lane(cx, i, z0, &setup);
    cx.run("A15", name, |cx| {
        let before = bot_pos(cx);
        walk(cx, WALK);
        let after = bot_pos(cx);
        // With frost the speed change reaches the bot a tick or two after the server counted it.
        let lags: &[usize] = if lane_model.frost { &[0, 1, 2] } else { &[0] };
        let (lag, want) = lags
            .iter()
            .map(|l| (*l, model(z0, ticks, lane_model, *l)))
            .min_by(|a, b| (a.1 - after[2]).abs().total_cmp(&(b.1 - after[2]).abs()))
            .unwrap();
        let no_frost = model(z0, ticks, Lane { frost: false, ..lane_model }, 0);
        let plain = model(z0, ticks, PLAIN, 0);
        let dx = (after[0] - (x as f64 + 0.5)).abs();
        let ok = (after[2] - want).abs() < TOL && (after[2] - plain).abs() > min_gap && dx < 0.05;
        let frost = if lane_model.frost { format!("; frost reached the bot {lag} tick(s) late, without frost the model ends at {no_frost:.4}") } else { String::new() };
        (
            ok,
            format!(
                "walked {WALK} input ticks from z {:.3} to z {:.4} (vanilla model {want:.4}, off by {:.4}, limit {TOL}); without the effect it would end at {plain:.4} (gap {:.3}, want over {min_gap}); x drift {dx:.3}; y {:.3}{frost}",
                before[2],
                after[2],
                (after[2] - want).abs(),
                (after[2] - plain).abs(),
                after[1]
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (a walk the model can be fitted to)
fn control(cx: &mut Ctx) -> (Outcome, u32) {
    let x = lane(cx, 0, START_Z, &[]);
    let mut ticks = 0;
    let o = cx.run("A15", "plain_stone_walk_matches_the_model", |cx| {
        walk(cx, WALK);
        let after = bot_pos(cx);
        // The number of input ticks the walk really gave: the one whose model is closest.
        let (best, err) = (WALK.saturating_sub(2)..=WALK + 3)
            .map(|k| (k, (model(START_Z, k, PLAIN, 0) - after[2]).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        ticks = best;
        (
            err < TOL && (after[0] - (x as f64 + 0.5)).abs() < 0.05,
            format!("walked {WALK} requested ticks on stone to z {:.4}; the model matches with {best} input ticks (off by {err:.4}, limit {TOL})", after[2]),
        )
    });
    (o, ticks)
}

// Spec: specs/B1-movement.md §4.2 (WebBlock#entityInside, Entity#move)
fn cobweb(cx: &mut Ctx, ticks: u32) -> Outcome {
    let setup = vec![lane_fill(FLOOR_Y + 1, FLOOR_Y + 2, BLOCKS.0, BLOCKS.1, "cobweb")];
    effect_row(cx, 1, "walk_into_cobweb_is_slowed_like_vanilla", ticks, setup, Lane { stuck: Some((0.25, false)), ..PLAIN }, START_Z, 1.0)
}

// Spec: specs/B1-movement.md §4.2 (SweetBerryBushBlock#entityInside)
fn berry_bush(cx: &mut Ctx, ticks: u32) -> Outcome {
    // A bush needs dirt under it to stay, and age 0 does not hurt.
    let setup = vec![
        lane_fill(FLOOR_Y, FLOOR_Y, BLOCKS.0, BLOCKS.1, "dirt"),
        lane_fill(FLOOR_Y + 1, FLOOR_Y + 1, BLOCKS.0, BLOCKS.1, "sweet_berry_bush[age=0]"),
    ];
    effect_row(cx, 2, "walk_into_sweet_berry_bush_is_slowed_like_vanilla", ticks, setup, Lane { stuck: Some((0.8_f32 as f64, false)), ..PLAIN }, START_Z, 0.5)
}

// Spec: specs/B1-movement.md §4.2 (Entity#getBlockSpeedFactor)
fn slow_floor(cx: &mut Ctx, ticks: u32, i: i32, block: &str) -> Outcome {
    // The whole lane is the block, so the bot stands on it from the start (its top is lower than
    // stone's, and the bot settles onto it before the walk).
    let setup = vec![lane_fill(FLOOR_Y, FLOOR_Y, 342, 362, block)];
    effect_row(cx, i, &format!("walk_across_{block}_is_slowed_like_vanilla"), ticks, setup, Lane { speed: 0.4, ..PLAIN }, START_Z, 1.0)
}

// Spec: specs/B1-movement.md §4.2 (PowderSnowBlock#entityInside, in-block state)
fn powder_snow(cx: &mut Ctx, ticks: u32) -> Outcome {
    let setup = vec![lane_fill(FLOOR_Y + 1, FLOOR_Y + 1, BLOCKS.0, BLOCKS.1, "powder_snow")];
    effect_row(cx, 4, "walk_into_powder_snow_is_slowed_like_vanilla", ticks, setup, Lane { stuck: Some((0.9_f32 as f64, true)), frost: true, ..PLAIN }, START_Z, 0.2)
}

// Spec: specs/B1-movement.md §4.2 (PowderSnowBlock#canEntityWalkOnPowderSnow: no boots, no climbing)
fn powder_snow_no_climb(cx: &mut Ctx) -> Outcome {
    // The bot starts three cells before a wall and walks into it inside the snow. Touching a wall in
    // powdered snow lifted a bot with or without boots before the patch.
    let z0 = 353.5;
    let setup = vec![
        lane_fill(FLOOR_Y + 1, FLOOR_Y + 1, 355, 358, "powder_snow"),
        lane_fill(FLOOR_Y + 1, FLOOR_Y + 3, 359, 359, "stone"),
    ];
    let x = lane(cx, 5, z0, &setup);
    cx.run("A15", "powder_snow_without_boots_does_not_lift_the_bot_at_a_wall", |cx| {
        // About 4 blocks at the snow's 0.09 blocks per tick.
        walk(cx, 70);
        let after = bot_pos(cx);
        let wall = 359.0 - HALF_WIDTH;
        let at_wall = (after[2] - wall).abs() < 0.05;
        let on_floor = (after[1] - (FLOOR_Y + 1) as f64).abs() < 0.01;
        (
            at_wall && on_floor && (after[0] - (x as f64 + 0.5)).abs() < 0.05,
            format!("walked into the wall inside powdered snow: z {:.4} (against the wall {wall:.2}: {at_wall}), y {:.4} (want {} = no climb: {on_floor})", after[2], after[1], FLOOR_Y + 1),
        )
    })
}

/// The pathfinder rows: the straight line to the goal crosses a cobweb (and the same ground without
/// one, as the control: its "inside" count is then only the empty cell).
/// The pathfinder row: the straight line to the goal crosses a cobweb. It must arrive, and no
/// sample of the server's position may put the bot's box inside the cobweb's cells.
// Spec: specs/B1-movement.md §4.3 (a cobweb is not passable for the planner)
fn goto_around_cobweb(cx: &mut Ctx, with_web: bool) -> Outcome {
    let web_z = 349;
    let setup = if with_web { vec![lane_fill(FLOOR_Y + 1, FLOOR_Y + 2, web_z, web_z, "cobweb")] } else { vec![] };
    let x = lane(cx, if with_web { 6 } else { 8 }, START_Z, &setup);
    let goal = pos(x, FLOOR_Y + 1, 353);
    let name = if with_web { "goto_across_a_cobweb_goes_around_it" } else { "goto_over_the_same_ground_without_a_cobweb" };
    cx.run("A15", name, |cx| {
        let mark = cx.log_len();
        let mut alert_at: Option<String> = None;
        let seq = cx.send(Op::Goto(goal));
        let t = Instant::now();
        let (mut samples, mut inside, mut min_gap) = (0, 0, f64::MAX);
        let mut near: Vec<String> = Vec::new();
        loop {
            if cx.try_recv(seq).is_some_and(|r| !matches!(r.info, Info::Started)) {
                break;
            }
            if t.elapsed() > Duration::from_secs(40) {
                return (false, "the goto did not finish in 40 s".into());
            }
            let p = bot_pos(cx);
            samples += 1;
            if alert_at.is_none() && !cx.log_problems(mark).1.is_empty() {
                alert_at = Some(format!("first Grim alert seen at sample {samples} ({} ms), bot at ({:.2},{:.2},{:.2})", t.elapsed().as_millis(), p[0], p[2], p[1]));
            }
            // The bot's box against the cobweb cell [x, x+1] x [web_z, web_z+1] (feet height).
            let gap_x = ((p[0] - (x as f64 + 0.5)).abs() - 0.5 - HALF_WIDTH).max(0.0);
            let gap_z = ((p[2] - (web_z as f64 + 0.5)).abs() - 0.5 - HALF_WIDTH).max(0.0);
            let gap = gap_x.max(gap_z);
            min_gap = min_gap.min(gap);
            if gap < 0.5 && near.last().is_none_or(|l| !l.starts_with(&format!("({:.2},{:.2}", p[0], p[2]))) {
                near.push(format!("({:.2},{:.2},{:.2}) gap {gap:.2}", p[0], p[2], p[1]));
            }
            if gap_x == 0.0 && gap_z == 0.0 {
                inside += 1;
            }
        }
        sleep(Duration::from_millis(300));
        let end = bot_pos(cx);
        let arrived = (end[0] - goal.x as f64 - 0.5).abs() < 1.0 && (end[2] - goal.z as f64 - 0.5).abs() < 1.0;
        let web_kept = !with_web || cx.block_is(pos(x, FLOOR_Y + 1, web_z), "cobweb");
        (
            arrived && (inside == 0 || !with_web) && web_kept && samples >= 5,
            format!(
                "goto ({}, {}, {}) from z {START_Z}: ended at ({:.2}, {:.2}, {:.2}) (arrived: {arrived}); {samples} position samples during the walk, {inside} with the bot's box in the cobweb's cell (want 0), closest approach {min_gap:.2} blocks; cobweb still there: {web_kept}; positions within 0.5 of the cobweb: {}; {}",
                goal.x, goal.y, goal.z, end[0], end[1], end[2],
                near.join(" "),
                alert_at.unwrap_or_else(|| "no alert during the walk".into())
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2, §4.3
pub fn a15(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], (LANE0 as f64 + 0.5, (FLOOR_Y + 1) as f64, START_Z));
    let (control_row, ticks) = control(cx);
    let mut out = vec![control_row];
    out.push(cobweb(cx, ticks));
    out.push(berry_bush(cx, ticks));
    out.push(slow_floor(cx, ticks, 3, "soul_sand"));
    out.push(slow_floor(cx, ticks, 7, "honey_block"));
    out.push(powder_snow(cx, ticks));
    out.push(powder_snow_no_climb(cx));
    out.push(goto_around_cobweb(cx, false));
    out.push(goto_around_cobweb(cx, true));
    cx.clean_in(AREA);
    out
}
