//! A17: the safe envelope, tier 1, first part (fork patch 30; AD-F5.5). Plan:
//! specs/plans/PATCH_30_TIER1.md §4. Levitation and Slow Falling are modelled: the bot's own
//! per-tick trace (a probe plugin in the bot's `GameTick`, written below) is compared with a model
//! written from the vanilla formulas (`LivingEntity#travelInAir`, `#getEffectiveGravity`); it shares
//! no code with the fork. The server's truth comes from RCON (height, health, ground state) and
//! from the log: a vanilla server logs "moved wrongly" and Grim alerts `Simulation` if the bot's
//! positions disagree with its own simulation of the effect. A `Movement` entry (scale away from 1,
//! a vehicle, a bubble column, Dolphin's Grace) makes the fork close the connection before the next
//! position is computed: those rows use a spare account each (`d_name`), because the attribute or
//! effect stays on the player's data after the leave, and a rejoin would be disconnected at once.
//! The spare accounts are idle bots that join at the start of the run (`main.rs`).
//! Area: corner (890, 342), floor y=199.

use std::{
    fs,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant},
};

use azalea::{
    account::Account,
    app::{App, Plugin, Update},
    core::tick::GameTick,
    ecs::prelude::*,
    entity::{ActiveEffects, EntityKindComponent, HasClientLoaded, LocalEntity, Physics, Position, metadata::Health},
    physics::PhysicsSystems,
    tick_counter::TicksConnected,
    unmodelled_disconnect::{PacketsAfterUnmodelledDisconnect, UnmodelledDisconnect},
};
use mc_adapter::{BotState, HostEvent};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome},
    scenarios_block_effects::{bot_pos, floats},
    scenarios_server_attributes::{REVIVE, TOL, WALK, walk_model},
    skill::{Info, Op},
    skill_items::ItemOp,
};

const AREA: (i32, i32) = (890, 342);
/// Where the main bot stands for the air rows (centre of the area).
const CX: f64 = 900.5;
const CZ: f64 = 352.5;
/// The vanilla air drag (`0.98F` widened to a double).
const DRAG: f64 = 0.98f32 as f64;
/// The vanilla jump speed with the base `jump_strength` and a block factor of 1.
const JUMP_SPEED: f64 = 0.42;
/// Spare accounts for the rows that end in a disconnect: A16 uses 1 and 2, A17 uses 3 to 5.
pub const SPARES: u32 = 5;

// Spec: specs/F5-version-layer.md §4.6
/// The name of spare account `k` (1-based). The process id keeps names new on each run: the
/// attribute a row sets stays on the account's data on the server.
pub fn d_name(k: u32) -> String {
    format!("null_d{}_{k}", std::process::id() % 100_000)
}

// Spec: specs/F5-version-layer.md §4.6 (patch 30: outside the world border is a disconnect)
/// Before a row shrinks the world border: bring the other bots (A8/A18/A19's second account,
/// A16/A17's spares) inside it, 2 blocks apart from `at` so none pushes another. Outside the
/// border patch 30 disconnects them for good, so later rows that need them fail (found by the
/// Pi pairs p33_full for A22 and p36_full for A21). An offline name only gets a "no player" reply.
pub fn bring_others_inside(cx: &mut Ctx, at: (i32, i32)) {
    let others = std::iter::once(format!("{}2", cx.name)).chain((1..=SPARES).map(d_name));
    for (k, name) in others.enumerate() {
        let (x, z) = (at.0 + 2 * (k as i32 % 2), at.1 + 2 * (k as i32 / 2));
        cx.r(&format!("execute in minecraft:overworld run tp {name} {x}.5 {}.0 {z}.5 0 0", FLOOR_Y + 1));
    }
}

/// One tick of the bot as it ended: the probe copies it from the ECS after the physics.
#[derive(Clone, Debug)]
pub struct TraceTick {
    pub tick: u64,
    pub y: f64,
    pub vy: f64,
    pub on_ground: bool,
    /// Levitation's amplifier while the bot knows the effect.
    pub lev: Option<i32>,
    pub slow: bool,
    /// The bot's position (x, z) and health as its client holds them (A17 miss trace).
    pub xz: (f64, f64),
    pub hp: Option<f32>,
    /// Arrows in the client's entity view this tick: (position, velocity).
    pub arrows: Vec<([f64; 3], [f64; 3])>,
}

pub static TRACE_ON: AtomicBool = AtomicBool::new(false);
pub static TRACE: Mutex<Vec<TraceTick>> = Mutex::new(Vec::new());

/// What the probe saw of an account this frame.
#[derive(Clone, Debug, Default)]
pub struct Seen {
    pub name: String,
    pub connected: bool,
    /// (tick, inputs, packets sent after the trigger) from the fork's record.
    pub disconnect: Option<(u64, Vec<String>, u32)>,
}
pub static SEEN: Mutex<Vec<Seen>> = Mutex::new(Vec::new());

pub fn seen(name: &str) -> Option<Seen> {
    SEEN.lock().unwrap().iter().find(|s| s.name == name).cloned()
}

// Spec: specs/plans/PATCH_30_TIER1.md §4 (A17 rows read the bot's own trace and its disconnect record)
/// Adds the trace and the account probe to the bot's app. `main` is the runner's account.
pub struct EnvelopeProbe {
    pub main: String,
}

impl Plugin for EnvelopeProbe {
    fn build(&self, app: &mut App) {
        let main = self.main.clone();
        app.add_systems(
            GameTick,
            (move |q: Query<(&Account, &Position, &Physics, &ActiveEffects, &TicksConnected, Option<&Health>), (With<LocalEntity>, With<HasClientLoaded>)>,
                   others: Query<(&EntityKindComponent, &Position, Option<&Physics>), Without<LocalEntity>>| {
                if !TRACE_ON.load(Ordering::Relaxed) {
                    return;
                }
                for (a, p, ph, fx, t, hp) in &q {
                    if a.username() == main {
                        TRACE.lock().unwrap().push(TraceTick {
                            tick: t.0,
                            y: p.y,
                            vy: ph.velocity.y,
                            on_ground: ph.on_ground(),
                            lev: fx.get_level(azalea::registry::builtin::MobEffect::Levitation),
                            slow: fx.get(azalea::registry::builtin::MobEffect::SlowFalling).is_some(),
                            xz: (p.x, p.z),
                            hp: hp.map(|h| h.0),
                            arrows: others
                                .iter()
                                .filter(|(k, ..)| k.0 == azalea::registry::builtin::EntityKind::Arrow)
                                .map(|(_, ap, aph)| ([ap.x, ap.y, ap.z], aph.map_or([0.0; 3], |v| [v.velocity.x, v.velocity.y, v.velocity.z])))
                                .collect(),
                        });
                    }
                }
            })
            .after(PhysicsSystems)
            .ambiguous_with_all(),
        )
        .add_systems(
            Update,
            (|q: Query<(&Account, Has<HasClientLoaded>, Option<&UnmodelledDisconnect>, Option<&PacketsAfterUnmodelledDisconnect>)>| {
                let mut out = SEEN.lock().unwrap();
                out.clear();
                for (a, loaded, d, after) in &q {
                    out.push(Seen {
                        name: a.username().to_string(),
                        connected: loaded,
                        disconnect: d.map(|d| (d.tick, d.inputs.iter().map(|i| format!("{i:?}")).collect(), after.map_or(0, |n| n.0))),
                    });
                }
            })
            .ambiguous_with_all(),
        );
    }
}

fn wait_until(secs: u64, mut f: impl FnMut() -> bool) -> bool {
    let end = Instant::now() + Duration::from_secs(secs);
    while Instant::now() < end {
        if f() {
            return true;
        }
        sleep(Duration::from_millis(25));
    }
    f()
}

fn trace_start() {
    TRACE.lock().unwrap().clear();
    TRACE_ON.store(true, Ordering::Relaxed);
}

fn trace_stop() -> Vec<TraceTick> {
    TRACE_ON.store(false, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
    std::mem::take(&mut *TRACE.lock().unwrap())
}

/// The bot has been in the air and now stands on the ground again.
fn landed() -> bool {
    let t = TRACE.lock().unwrap();
    t.len() > 5 && t.iter().any(|x| !x.on_ground) && t[t.len() - 3..].iter().all(|x| x.on_ground)
}

// Spec: specs/B1-movement.md §4.2 (Levitation, Slow Falling)
/// One airborne tick of vanilla: the vertical speed of the tick before (zeroed under 0.003 by
/// `aiStep`) is the move; then Levitation pulls a fifth of the way to 0.05 x (level), or gravity
/// (capped at 0.01 for Slow Falling at a speed that is not upward) is taken off; then the drag.
/// Returns (the move, the new vertical speed).
fn model_step(v_prev: f64, lev: Option<i32>, slow: bool) -> (f64, f64) {
    let v = if v_prev.abs() < 0.003 { 0.0 } else { v_prev };
    let v2 = match lev {
        Some(a) => v + (0.05 * (a + 1) as f64 - v) * 0.2,
        None => v - if slow && v <= 0.0 { 0.01 } else { 0.08 },
    };
    (v, v2 * DRAG)
}

// Spec: specs/B1-movement.md §4.2
/// Compares every airborne tick of the trace with the model, fed with the speed the trace had the
/// tick before. With `server_motion` a tick may instead start from the speed the server believes
/// (the last move): a hit makes the server send `ClientboundSetEntityMotion` with its own idea of
/// the player's speed, which is the last move, and the client takes it (`handleSetEntityMotion`).
/// Returns (ticks compared, largest error, the first miss, ticks that needed the server's speed).
fn compare(trace: &[TraceTick], server_motion: bool) -> (usize, f64, Option<String>, usize) {
    let (mut n, mut worst, mut miss, mut resets) = (0, 0.0_f64, None, 0);
    let first_effect = trace.iter().position(|t| t.slow || t.lev.is_some());
    for (i, w) in trace.windows(2).enumerate() {
        let (p, c) = (&w[0], &w[1]);
        // A tick that ends on the floor is a collision (the flag can lag by one tick when the bot
        // lands exactly on the floor, as vanilla's `verticalCollision` needs a clipped move).
        if c.on_ground || c.y <= (FLOOR_Y + 1) as f64 + 1e-9 {
            continue;
        }
        // A hit also sets the speed the server sends with it, which the trace cannot see, and the
        // motion packet may land in any of the first four ticks with the effect. There the move
        // itself is taken as the speed the tick started from, and what the tick did to it (gravity,
        // the Slow Falling cap, the drag) is still compared; every tick after them is compared in full.
        if server_motion && first_effect.is_some_and(|f| (f..=f + 3).contains(&(i + 1))) {
            let (dy, v) = model_step(c.y - p.y, c.lev, c.slow);
            let e = (c.vy - v).abs();
            n += 1;
            worst = worst.max(e);
            if e > 1e-6 && miss.is_none() {
                miss = Some(format!("tick {}: speed {:.7} want {v:.7} from a start of {dy:.7} (lev {:?}, slow {})", c.tick, c.vy, c.lev, c.slow));
            }
            continue;
        }
        let err = |v_prev: f64| {
            let (dy, v) = model_step(v_prev, c.lev, c.slow);
            (((c.y - p.y) - dy).abs().max((c.vy - v).abs()), dy, v)
        };
        // A take-off tick: `Player#jumpFromGround` sets the rise to 0.42 before the move, so that is
        // the speed this tick starts from (the probe can hold a standing tick before it).
        let take_off = p.on_ground && c.lev.is_none() && c.y - p.y > 0.3;
        let (mut e, mut dy, mut v) = err(if take_off { JUMP_SPEED } else { p.vy });
        if e > 1e-6 && server_motion && i > 0 {
            let last_move = p.y - trace[i - 1].y;
            let (e2, dy2, v2) = err(last_move);
            if e2 <= 1e-6 {
                (e, dy, v) = (e2, dy2, v2);
                resets += 1;
            }
        }
        n += 1;
        worst = worst.max(e);
        if e > 1e-6 && miss.is_none() {
            miss = Some(format!("tick {}: moved {:.7} want {dy:.7}, speed {:.7} want {v:.7} (lev {:?}, slow {})", c.tick, c.y - p.y, c.vy, c.lev, c.slow));
        }
    }
    (n, worst, miss, resets)
}

// Spec: specs/plans/PATCH_30_TIER1.md §4
/// Puts a fresh runner skill back after the host ended it for an `Action` entry or a death.
pub(crate) fn revive(cx: &mut Ctx) {
    REVIVE.store(true, Ordering::Relaxed);
    sleep(Duration::from_millis(700));
    let _ = cx.snap();
}

fn bot_effect(cx: &mut Ctx, cmd: &str) {
    let n = cx.name.clone();
    cx.r(&format!("effect {cmd}").replace("{n}", &n));
}

// Spec: specs/plans/PATCH_30_TIER1.md §4
/// Floor, air and the bot at the area's centre, settled, with the air column above it cleared.
fn stand(cx: &mut Ctx, tall: i32) {
    let n = cx.name.clone();
    cx.r(&format!("fill {} {} {} {} {} {} minecraft:air", CX as i32 - 1, FLOOR_Y + 1, CZ as i32 - 1, CX as i32 + 1, FLOOR_Y + tall, CZ as i32 + 1));
    cx.r(&format!("tp {n} {CX} {} {CZ} 0 0", FLOOR_Y + 1));
    sleep(Duration::from_millis(1500));
}

fn health(cx: &mut Ctx) -> f64 {
    let n = cx.name.clone();
    floats(&cx.r(&format!("data get entity {n} Health"))).last().copied().unwrap_or(f64::NAN)
}

fn on_ground_server(cx: &mut Ctx) -> bool {
    let n = cx.name.clone();
    cx.r(&format!("data get entity {n} OnGround")).contains("1b")
}

// Spec: specs/B1-movement.md §4.2
/// The vanilla rise over the `n` ticks Levitation of this level was in force, from rest.
fn rise_model(amp: i32, n: usize) -> f64 {
    let (mut v, mut y) = (0.0, 0.0);
    for _ in 0..n {
        let (dy, v2) = model_step(v, Some(amp), false);
        y += dy;
        v = v2;
    }
    y
}

// Spec: specs/B1-movement.md §4.2 (Levitation in air travel)
fn levitation_row(cx: &mut Ctx, name: &str, amp: i32, clear: bool) -> Outcome {
    stand(cx, 8);
    cx.run("A17", name, |cx| {
        trace_start();
        sleep(Duration::from_millis(400));
        // 3 s: the server removes it itself (ClientboundRemoveMobEffect); the other row clears it
        bot_effect(cx, &format!("give {{n}} minecraft:levitation {} {amp} true", if clear { 30 } else { 3 }));
        let enough = wait_until(8, || TRACE.lock().unwrap().iter().filter(|t| t.lev.is_some()).count() >= 40);
        let server_y = bot_pos(cx)[1];
        let (lo, hi) = {
            let t = TRACE.lock().unwrap();
            let last: Vec<f64> = t.iter().rev().take(10).map(|x| x.y).collect();
            (last.iter().cloned().fold(f64::MAX, f64::min), last.iter().cloned().fold(f64::MIN, f64::max))
        };
        if clear {
            bot_effect(cx, "clear {n} minecraft:levitation");
        } else {
            // wait for the 3 s to run out on the server
            sleep(Duration::from_millis(3500));
        }
        let down = wait_until(20, landed);
        sleep(Duration::from_millis(300));
        let trace = trace_stop();
        let landed_y = bot_pos(cx)[1];
        let grounded = on_ground_server(cx);
        bot_effect(cx, "clear {n}");
        let first = trace.iter().position(|t| t.lev.is_some());
        let n_lev = trace.iter().filter(|t| t.lev.is_some()).count();
        let (checked, worst, miss, _) = compare(&trace, false);
        let (rise, want_rise) = match first {
            Some(f) if f > 0 => {
                let last_lev = trace.iter().rposition(|t| t.lev.is_some()).unwrap();
                (trace[last_lev].y - trace[f - 1].y, rise_model(amp, n_lev))
            }
            _ => (f64::NAN, f64::NAN),
        };
        let server_ok = server_y >= lo - 0.02 && server_y <= hi + 0.02;
        revive(cx);
        (
            enough && down && checked >= 40 && miss.is_none() && (rise - want_rise).abs() < 1e-6 && server_ok && grounded && (landed_y - (FLOOR_Y + 1) as f64).abs() < 0.01 && trace.iter().any(|t| t.lev.is_some()),
            format!(
                "level {}: Levitation in force for {n_lev} ticks, rose {rise:.5} (vanilla {want_rise:.5}); server y {server_y:.3} while the bot's last ten ticks span {lo:.3}..{hi:.3}; {checked} airborne ticks match the formula (worst error {worst:.2e}){}; ended by {}; after it the bot fell and landed at y {landed_y:.3} (server on_ground {grounded})",
                amp + 1,
                miss.map(|m| format!(", FIRST MISS {m}")).unwrap_or_default(),
                if clear { "/effect clear" } else { "the server's removal at 3 s" },
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Slow Falling caps gravity)
fn slow_falling_fall(cx: &mut Ctx) -> Outcome {
    stand(cx, 24);
    cx.run("A17", "slow_falling_fall_20_blocks", |cx| {
        let n = cx.name.clone();
        bot_effect(cx, "give {n} minecraft:slow_falling 60 0 true");
        sleep(Duration::from_millis(900));
        let hp0 = health(cx);
        trace_start();
        cx.r(&format!("tp {n} {CX} {} {CZ}", FLOOR_Y + 21));
        let down = wait_until(15, || landed());
        sleep(Duration::from_millis(300));
        let trace = trace_stop();
        let hp1 = health(cx);
        let grounded = on_ground_server(cx);
        bot_effect(cx, "clear {n}");
        // from the first tick after the teleport
        let from = trace.iter().position(|t| t.y > (FLOOR_Y + 5) as f64).unwrap_or(0);
        let trace = trace[from..].to_vec();
        let (checked, worst, miss, _) = compare(&trace, false);
        // ticks to land by the formula, from the first traced tick's state
        let want_ticks = trace.first().map(|f| {
            let (mut y, mut v, mut k) = (f.y, f.vy, 0);
            while k < 400 {
                let (dy, v2) = model_step(v, None, true);
                y += dy;
                v = v2;
                k += 1;
                if y <= (FLOOR_Y + 1) as f64 {
                    break;
                }
            }
            k
        });
        let got_ticks = trace.iter().position(|t| t.on_ground);
        let slow_all = trace.iter().take_while(|t| !t.on_ground).all(|t| t.slow);
        revive(cx);
        (
            down && checked >= 30 && miss.is_none() && want_ticks == got_ticks && (hp1 - hp0).abs() < 1e-9 && grounded && slow_all,
            format!(
                "fell 20 blocks with Slow Falling: {checked} airborne ticks match the formula (worst error {worst:.2e}){}; landed after {got_ticks:?} ticks from the first traced tick (formula {want_ticks:?}); health {hp0} before and {hp1} after (no fall damage); server on_ground {grounded}",
                miss.map(|m| format!(", FIRST MISS {m}")).unwrap_or_default(),
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (evidence for a missed Slow Falling arrow)
/// Prints, for a missed attempt, one line per client tick from the summon: the arrow as the
/// bot's client sees it, the bot's position and health, then the RCON samples. Evidence only.
fn miss_trace(trace: &[TraceTick], from: Option<u64>, attempt: u32, rcon: &[String]) {
    println!("A17 slow_falling_arrow attempt {attempt} MISSED; per-tick trace from the summon (client view):");
    let mut last_arrow = None;
    for (i, t) in trace.iter().enumerate() {
        if from.is_some_and(|f| t.tick < f) {
            continue;
        }
        if !t.arrows.is_empty() {
            last_arrow = Some(i);
        } else if last_arrow.is_some_and(|l| i > l + 5) {
            break;
        }
        let arrows: Vec<String> = t
            .arrows
            .iter()
            .map(|(p, v)| format!("arrow ({:.3},{:.3},{:.3}) v ({:.3},{:.3},{:.3})", p[0], p[1], p[2], v[0], v[1], v[2]))
            .collect();
        println!(
            "  t{} bot ({:.3},{:.3},{:.3}) vy {:.3} ground {} hp {:?} | {}",
            t.tick, t.xz.0, t.y, t.xz.1, t.vy, t.on_ground, t.hp, if arrows.is_empty() { "no arrow".into() } else { arrows.join(" ") }
        );
    }
    for r in rcon {
        println!("{r}");
    }
}

/// Arrow start height above the server's feet: over the inflated box top (2.1) plus one tick of rise (0.42).
const ARROW_DY: f64 = 2.6;

// Spec: specs/B1-movement.md §4.2 (Slow Falling from a tipped arrow, the case AD-F5.5 names)
fn slow_falling_arrow(cx: &mut Ctx) -> Outcome {
    stand(cx, 8);
    cx.run("A17", "slow_falling_arrow", |cx| {
        let n = cx.name.clone();
        let hp0 = health(cx);
        // A hit knocks the bot sideways and a miss is possible (the bot moves while the arrow
        // falls), so a miss is tried again from a new jump, up to three times. A hit counts only
        // when the server itself holds the effect (RCON), not on the bot's trace alone.
        let (mut got, mut attempts, mut p) = (false, 0, [0.0; 3]);
        let mut hits = Vec::new();
        while !got && attempts < 3 {
            attempts += 1;
            // Restart the trace per attempt: the final trace must hold only the attempt that hit
            // (a missed attempt's take-off tick has no jump model in `compare`).
            if attempts > 1 {
                trace_stop();
            }
            trace_start();
            let hp_before = health(cx);
            let jump = cx.send(Op::Items(ItemOp::Jump));
            sleep(Duration::from_millis(220));
            p = bot_pos(cx);
            // The server's own position of the bot decides the hit, and it can lead or lag the
            // client view `p` by a tick or more of the jump. The arrow's hit test
            // (`AABB#clip` on the box inflated by 0.3, top at feet + 2.1) finds only a box the
            // path enters from outside; an arrow that starts inside it passes through (q1 repro:
            // 39 missed arrows, 7 of 30 Grim runs failed). So the arrow is summoned `~ ~ARROW_DY` from the bot as the server
            // holds it at that moment: above feet + 2.1 + the largest one-tick rise (about 0.42),
            // and with Motion -0.6 it crosses the top face in its first tick. It still deals
            // damage (a player is not hurt by 0 damage and then no effect is applied).
            // where the server holds the bot at the summon (a miss on Grim needs this to be told apart)
            let srv = cx.r(&format!("data get entity {n} Pos"));
            cx.r(&format!(
                "execute at {n} run summon minecraft:arrow ~ ~{ARROW_DY} ~ {{item:{{id:\"minecraft:tipped_arrow\",count:1,components:{{\"minecraft:potion_contents\":{{potion:\"minecraft:slow_falling\"}}}}}},Motion:[0.0,-0.6,0.0]}}"
            ));
            // Every attempt keeps RCON samples of the server's arrow and the bot's HurtTime (taken
            // in place of the old 150 ms sleep); they are printed only if the attempt misses.
            let summon_tick = TRACE.lock().unwrap().last().map(|t| t.tick);
            let mut rcon = Vec::new();
            let t_sum = Instant::now();
            while t_sum.elapsed() < Duration::from_millis(150) {
                let at = TRACE.lock().unwrap().last().map(|t| t.tick);
                let sel = format!("@e[type=minecraft:arrow,x={},y={},z={},distance=..8,limit=1,sort=nearest]", p[0], p[1], p[2]);
                let tail = |s: String| s.trim().split(" has the following entity data: ").last().unwrap_or("").trim().to_string();
                let m = tail(cx.r(&format!("data get entity {sel} Motion")));
                let ps = tail(cx.r(&format!("data get entity {sel} Pos")));
                let g = tail(cx.r(&format!("data get entity {sel} inGround")));
                let h = tail(cx.r(&format!("data get entity {n} HurtTime")));
                rcon.push(format!("  rcon@t{at:?} arrow Pos {ps} Motion {m} inGround {g}; bot HurtTime {h}"));
            }
            let arrow = cx.r(&format!("data get entity @e[type=minecraft:arrow,x={},y={},z={},distance=..8,limit=1,sort=nearest] Pos", p[0], p[1], p[2]));
            let on_server = wait_until(2, || cx.r(&format!("data get entity {n} active_effects")).contains("slow_falling"));
            got = on_server && wait_until(2, || TRACE.lock().unwrap().iter().any(|t| t.slow));
            if !got {
                miss_trace(&TRACE.lock().unwrap(), summon_tick, attempts, &rcon);
            }
            hits.push(format!("#{attempts}: arrow start +{ARROW_DY} above the server's feet, server pos {}, bot pos y {:.2}, server effect {on_server}, health {hp_before} -> {}, arrow after 150 ms {}", srv.trim().rsplit(':').next().unwrap_or("").trim(), p[1], health(cx), arrow.trim()));
            let _ = cx.recv(jump, Duration::from_secs(3));
            if !got {
                wait_until(5, landed);
                sleep(Duration::from_millis(800));
            }
        }
        let down = wait_until(15, landed);
        sleep(Duration::from_millis(300));
        // The bot ends its skill when it sees the effect leave (an unmodelled input). On Grim that
        // packet can arrive after `revive` put a new skill in, and the next call then waits 60 s
        // for a skill that is gone (seen once, 2026-10-24). So clear the effect first and wait
        // until the bot's own trace shows it gone; only then stop the trace and revive.
        let effect = cx.r(&format!("effect clear {n} minecraft:slow_falling"));
        let gone = wait_until(5, || TRACE.lock().unwrap().last().is_some_and(|t| !t.slow));
        sleep(Duration::from_millis(300));
        let trace = trace_stop();
        let hp1 = health(cx);
        cx.r(&format!("kill @e[type=minecraft:arrow,x={},y={FLOOR_Y},z={},dx=21,dy=10,dz=21]", AREA.0, AREA.1));
        let first_slow = trace.iter().position(|t| t.slow);
        let airborne_at_hit = first_slow.is_some_and(|i| !trace[i].on_ground);
        let slow_ticks_in_air = trace.iter().filter(|t| t.slow && !t.on_ground).count();
        let (checked, worst, miss, resets) = compare(&trace, true);
        revive(cx);
        (
            got && down && gone && airborne_at_hit && slow_ticks_in_air >= 3 && miss.is_none() && checked >= 8,
            format!(
                "an arrow tipped with slow_falling hit the bot at y {:.2} in a jump; the bot knew the effect from tick {:?} (in the air: {airborne_at_hit}), {slow_ticks_in_air} airborne ticks with it; {checked} airborne ticks match the formula (worst error {worst:.2e}; {resets} started from the speed the server sent with the hit){}; health {hp0} to {hp1}; clear said {:?} (bot saw it gone: {gone}); {attempts} attempt(s): {}",
                p[1], first_slow.map(|i| trace[i].tick), miss.map(|m| format!(", FIRST MISS {m}")).unwrap_or_default(), effect.trim(), hits.join("; ")
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (the bot walks with the same speed as vanilla after a respawn)
fn respawn_keeps_base_attribute(cx: &mut Ctx) -> Outcome {
    let n = cx.name.clone();
    cx.r(&format!("fill {} {FLOOR_Y} {} {} {FLOOR_Y} {} minecraft:stone", AREA.0, AREA.1 + 2, AREA.0 + 4, AREA.1 + 20));
    let lane = |cx: &mut Ctx| {
        cx.r(&format!("fill {} {} {} {} {} {} minecraft:air", AREA.0, FLOOR_Y + 1, AREA.1 + 2, AREA.0 + 4, FLOOR_Y + 5, AREA.1 + 20));
        cx.r(&format!("tp {n} {}.5 {} {}.5 0 0", AREA.0 + 2, FLOOR_Y + 1, AREA.1 + 3));
        sleep(Duration::from_millis(1500));
    };
    lane(cx);
    cx.run("A17", "respawn_keeps_base_attribute", |cx| {
        let start_z = AREA.1 as f64 + 3.5;
        let plain = 0.1 * 0.98;
        // control: the plain walk tells how many input ticks the op really gives
        let walk = |cx: &mut Ctx| {
            match cx.call(Op::Items(ItemOp::Walk(WALK))).info {
                Info::Items(_) => {}
                i => panic!("expected an items report: {i:?}"),
            }
            sleep(Duration::from_millis(1500));
            bot_pos(cx)
        };
        let control = walk(cx);
        let (k, kerr) = (WALK.saturating_sub(2)..=WALK + 3)
            .map(|k| (k, (walk_model(start_z, k, plain) - control[2]).abs()))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        lane(cx);
        cx.r(&format!("attribute {n} minecraft:movement_speed base set 0.15"));
        // respawn at the lane start: world spawn is in the air, and the fall there kills the bot again
        cx.r(&format!("spawnpoint {n} {} {} {}", AREA.0 + 2, FLOOR_Y + 1, AREA.1 + 3));
        sleep(Duration::from_millis(800));
        while cx.events.try_recv().is_ok() {}
        cx.r(&format!("kill {n}"));
        // the death screen holds 20 ticks, then the node plugin respawns the bot
        let back = wait_until(20, || {
            while let Ok(e) = cx.events.try_recv() {
                if let HostEvent::State(c) = e {
                    if matches!(c.state, BotState::Respawning) {
                        return true;
                    }
                }
            }
            false
        });
        sleep(Duration::from_millis(3000));
        let base = cx.r(&format!("attribute {n} minecraft:movement_speed base get")).trim().to_string();
        // the node plugin gave the new life a fresh skill
        let _ = cx.snap();
        lane(cx);
        let before = bot_pos(cx);
        let after = walk(cx);
        cx.r(&format!("attribute {n} minecraft:movement_speed base reset"));
        let want = walk_model(start_z, k, plain * 1.5);
        let def = walk_model(start_z, k, plain);
        let dx = (after[0] - (AREA.0 as f64 + 2.5)).abs();
        (
            back && kerr < TOL && (after[2] - want).abs() < TOL && (after[2] - def).abs() > 0.3 && dx < 0.05,
            format!("base movement_speed 0.15, killed, respawned (seen: {back}); server says {base}; walked from z {:.3} to z {:.4} (vanilla model {want:.4}, off by {:.4}, limit {TOL}; with the default it would end at {def:.4}); control walk fit with {k} input ticks (off by {kerr:.4}); x drift {dx:.3}", before[2], after[2], (after[2] - want).abs()),
        )
    })
}

/// A free fall on the floor of the area, to show that the trace and the model agree before any
/// effect is involved.
// Spec: specs/B1-movement.md §4.2
fn control_free_fall(cx: &mut Ctx) -> Outcome {
    stand(cx, 10);
    cx.run("A17", "control_free_fall_matches_the_model", |cx| {
        let n = cx.name.clone();
        trace_start();
        cx.r(&format!("tp {n} {CX} {} {CZ}", FLOOR_Y + 7));
        let down = wait_until(10, landed);
        sleep(Duration::from_millis(300));
        // Ticks before the teleport arrives (RCON answers before the bot sees it) are not part of the fall.
        let trace: Vec<TraceTick> = trace_stop().into_iter().skip_while(|t| t.y < (FLOOR_Y + 3) as f64).collect();
        let (checked, worst, miss, _) = compare(&trace, false);
        (
            down && checked >= 8 && miss.is_none(),
            format!("fell 6 blocks with no effect: {checked} airborne ticks match the formula (worst error {worst:.2e}){}", miss.map(|m| format!(", FIRST MISS {m}")).unwrap_or_default()),
        )
    })
}

// Spec: specs/F5-version-layer.md §4.6
/// Waits for spare account `k` to be in the world, puts it at `at` (its own cell of the area) and
/// returns its name.
fn spare(cx: &mut Ctx, k: u32, at: (f64, f64, f64)) -> String {
    let d = d_name(k);
    let ok = wait_until(40, || seen(&d).is_some_and(|s| s.connected));
    assert!(ok, "spare account {d} did not join");
    // survival, no op: Grim exempts ops
    cx.r(&format!("gamemode survival {d}"));
    cx.r(&format!("tp {d} {} {} {}", at.0, at.1, at.2));
    sleep(Duration::from_millis(2500));
    d
}

/// A row that ends in a disconnect: `trigger` makes the server set the input on spare account
/// `k`; the fork's record must name `needle`, show 0 packets sent after the trigger, and the bot
/// must stay off. The server log must show the leave and no complaint about the account.
/// Lines naming the spare account are skipped by the usual log check (another `null_` account), so
/// this checks them itself.
// Spec: specs/F5-version-layer.md §4.6 (the disconnect for a Movement entry)
pub(crate) fn disconnect_row(cx: &mut Ctx, id: &'static str, name: &str, k: u32, at: (f64, f64, f64), prepare: &[String], trigger: &[String], needle: &str) -> Outcome {
    let d = spare(cx, k, at);
    for c in prepare {
        cx.r(&c.replace("{d}", &d));
    }
    sleep(Duration::from_millis(1500));
    cx.run(id, name, |cx| {
        let mark = cx.log_len();
        while cx.events.try_recv().is_ok() {}
        let before = seen(&d).is_some_and(|s| s.connected);
        for c in trigger {
            cx.r(&c.replace("{d}", &d));
        }
        let gone = wait_until(8, || seen(&d).is_some_and(|s| s.disconnect.is_some()));
        sleep(Duration::from_millis(500));
        let s = seen(&d).unwrap_or_default();
        let (tick, inputs, after) = s.disconnect.clone().unwrap_or((0, vec![], u32::MAX));
        // stays off: the retry of an ordinary kick would come within a few seconds
        sleep(Duration::from_millis(4000));
        let still_off = seen(&d).is_some_and(|s| !s.connected);
        let mut off_event = false;
        while let Ok(e) = cx.events.try_recv() {
            if let HostEvent::State(c) = &e {
                if c.bot == d && matches!(c.state, BotState::UnmodelledOff { .. }) {
                    off_event = true;
                }
            }
        }
        let log = fs::read_to_string(&cx.log).ok().map(|t| t.get(mark as usize..).unwrap_or("").to_string()).unwrap_or_default();
        let mine: Vec<&str> = log.lines().filter(|l| l.contains(d.as_str())).collect();
        let left = mine.iter().any(|l| l.contains("left the game") || l.contains("lost connection"));
        let complaints: Vec<&str> = mine
            .iter()
            .copied()
            .filter(|l| ["moved too quickly", "moved wrongly", "Mismatch", "Rejecting", "Grim", "kicked"].iter().any(|p| l.contains(p)))
            .collect();
        let named = inputs.iter().any(|i| i.contains(needle));
        (
            before && gone && named && after == 0 && still_off && off_event && left && complaints.is_empty(),
            format!(
                "{d}: the fork's record is tick {tick}, inputs {inputs:?} (want one naming {needle}), packets after the trigger {after}; connected before: {before}; stayed off 4 s: {still_off}; host reported UnmodelledOff: {off_event}; server log shows the leave: {left}; server complaints about it: {complaints:?}"
            ),
        )
    })
}

// Spec: specs/F5-version-layer.md §4.6 (disconnect rows A17)
fn disconnect_rows(cx: &mut Ctx) -> Vec<Outcome> {
    let (x, z) = (AREA.0 as f64, AREA.1 as f64);
    let floor = (FLOOR_Y + 1) as f64;
    let mut out = Vec::new();
    cx.r(&format!("fill {} {FLOOR_Y} {} {} {FLOOR_Y} {} minecraft:stone", AREA.0, AREA.1, AREA.0 + 20, AREA.1 + 8));
    out.push(disconnect_row(
        cx, "A17", "disconnect_on_scale", 3, (x + 3.5, floor, z + 3.5),
        &[],
        &["attribute {d} minecraft:scale base set 2".to_string()],
        "Scale",
    ));
    out.push(disconnect_row(
        cx, "A17", "disconnect_on_ride", 4, (x + 6.5, floor, z + 3.5),
        &[],
        &["execute at {d} run summon minecraft:oak_boat ~ ~ ~".to_string(), "execute at {d} run ride {d} mount @e[type=minecraft:oak_boat,distance=..3,limit=1,sort=nearest]".to_string()],
        "Passenger",
    ));
    // A shaft of water over soul sand makes a bubble column; the bot is put into it afterwards.
    let (sx, sz) = (AREA.0 + 9, AREA.1 + 3);
    let shaft = vec![
        format!("fill {} {} {} {} {} {} minecraft:stone", sx - 1, FLOOR_Y, sz - 1, sx + 1, FLOOR_Y + 6, sz + 1),
        format!("fill {sx} {} {sz} {sx} {} {sz} minecraft:water", FLOOR_Y + 1, FLOOR_Y + 5),
        format!("setblock {sx} {FLOOR_Y} {sz} minecraft:soul_sand"),
    ];
    for c in &shaft {
        cx.r(c);
    }
    out.push(disconnect_row(
        cx, "A17", "disconnect_in_bubble_column", 5, (x + 12.5, floor, z + 3.5),
        &[],
        &["tp {d} ".to_string() + &format!("{}.5 {} {}.5", sx, FLOOR_Y + 3, sz)],
        "BubbleColumn",
    ));
    out
}

// Spec: specs/B1-movement.md §4.2, specs/F5-version-layer.md §4.6
pub fn a17(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], (CX, (FLOOR_Y + 1) as f64, CZ));
    // `A17_ONLY=text` runs only the rows whose name contains it (to rerun a failed row)
    let want = |name: &str| std::env::var("A17_ONLY").map_or(true, |t| name.contains(&t));
    let mut out = Vec::new();
    if want("control_free_fall") {
        out.push(control_free_fall(cx));
    }
    if want("levitation_i_rises") {
        out.push(levitation_row(cx, "levitation_i_rises", 0, false));
    }
    if want("levitation_ii_rises") {
        out.push(levitation_row(cx, "levitation_ii_rises", 1, true));
    }
    if want("slow_falling_fall_20_blocks") {
        out.push(slow_falling_fall(cx));
    }
    if want("slow_falling_arrow") {
        out.push(slow_falling_arrow(cx));
    }
    if want("respawn_keeps_base_attribute") {
        out.push(respawn_keeps_base_attribute(cx));
    }
    if want("disconnect") {
        out.extend(disconnect_rows(cx));
    }
    if let Ok(secs) = std::env::var("A17_HOLD") {
        // debugging aid: keep the bots online for manual RCON commands
        sleep(Duration::from_secs(secs.parse().unwrap_or(0)));
    }
    cx.clean_in(AREA);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tick(n: u64, y: f64, vy: f64, on_ground: bool) -> TraceTick {
        TraceTick { tick: n, y, vy, on_ground, lev: None, slow: false, xz: (0.0, 0.0), hp: None, arrows: Vec::new() }
    }

    /// A trace that holds a standing tick before the jump's take-off tick (the probe starts a tick
    /// before the bot's jump input lands; seen in `p42_sel_grim.log` A17).
    fn jump_trace(standing_first: bool) -> Vec<TraceTick> {
        let floor = (FLOOR_Y + 1) as f64;
        let mut t = Vec::new();
        let (mut y, mut v, mut n) = (floor, -0.0784, 0);
        if standing_first {
            t.push(tick(n, y, v, true));
            n += 1;
        }
        v = 0.42;
        for _ in 0..8 {
            let (dy, v2) = model_step(v, None, false);
            y += dy;
            v = v2;
            t.push(tick(n, y, v, false));
            n += 1;
        }
        t
    }

    // Spec: specs/B1-movement.md §4.2 (the jump's take-off tick: vanilla `jumpFromGround` sets 0.42 before the move)
    #[test]
    fn a_standing_tick_before_take_off_is_checked_as_a_jump_not_a_free_fall() {
        for standing_first in [false, true] {
            let (n, worst, miss, _) = compare(&jump_trace(standing_first), true);
            assert!(miss.is_none(), "standing_first {standing_first}: {miss:?}");
            assert!(worst < 1e-9, "worst {worst}");
            assert!(n >= 7);
        }
    }

    // Negative control for the rule above: a take-off that rises by anything but the jump speed still fails.
    #[test]
    fn a_take_off_with_the_wrong_rise_is_still_a_miss() {
        let mut t = jump_trace(true);
        t[1].y -= 0.1;
        assert!(compare(&t, true).2.is_some());
    }
}
