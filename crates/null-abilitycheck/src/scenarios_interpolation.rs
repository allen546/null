//! A18: interpolation of other entities and the bot-to-bot view (fork patch 31; P30.1). Plan:
//! specs/plans/PATCH_30_TIER1.md §2.3, specs/F5-version-layer.md §4.6. A vanilla client moves
//! another entity toward each new server position over three ticks (`InterpolationHandler`), and
//! the crosshair ray and reach read the moved box. Rows:
//! - a villager moved by RCON `tp` reaches its new place in three equal steps on three
//!   consecutive ticks (the bot's own trace of the entity's position and box, from a probe in its
//!   `GameTick`; the expected steps come from the vanilla formula, not from the fork);
//! - the crosshair ray reads the box of that tick: the ray hits the villager while the moving box
//!   still covers it and misses once it has moved off, step by step;
//! - two bots near each other each hold the other in their `PeerView` (built from the server's
//!   packets), a server move of one shows in the other's view in three steps while its own
//!   position has already jumped, a bot beyond 16 blocks is not kept, and the counter shows the
//!   entries (current and peak).
//!
//! The second bot is the A8 attacker account (`<account>2`, no skill), which `main` starts for
//! A18 too. Server truth is the RCON `tp`; the server log is scanned for warnings and Grim alerts
//! on every row. Area: corner (920, 342), floor y=199.

use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use azalea::{
    account::Account,
    app::{App, Plugin},
    core::{aabb::Aabb, hit_result::HitResult, tick::GameTick},
    ecs::prelude::*,
    entity::{
        EntityKindComponent, LocalEntity, LookDirection, Position, dimensions::EntityDimensions, interpolation::PeerView,
        view_vector,
    },
    interact::pick::{HitResultComponent, update_hit_result_component},
    physics::{PhysicsSystems, collision::entity_collisions::LastBoundingBox},
    registry::builtin::EntityKind,
};

use crate::scenarios::{Ctx, Outcome};

const AREA: (i32, i32) = (920, 342);
/// The main bot: facing +z (yaw 0), level.
const STAND: (f64, f64, f64) = (923.5, 200.0, 352.5);
/// The villager, three blocks ahead: 1.95 tall, so the level ray at eye height 1.62 hits its middle.
const VILLAGER: (f64, f64, f64) = (923.5, 200.0, 355.5);
const SEL: &str = "@e[tag=a18v,limit=1]";
/// Where the second bot stands: near (6 blocks), moved 6 blocks (still near), and far (over 16).
const NEAR: (f64, f64, f64) = (928.5, 200.0, 349.5);
const MOVED: (f64, f64, f64) = (934.5, 200.0, 349.5);
const FAR: (f64, f64, f64) = (939.5, 200.0, 360.5);

/// One tick of the villager as the bot's ray saw it: sampled where the click systems run, after
/// the tick's ray and before the entity step.
#[derive(Clone, Debug)]
pub struct MobTick {
    pub x: f64,
    pub box_centre: f64,
    /// The ray the bot cast this tick hit the villager.
    pub hit: bool,
    /// The same ray, tested here against the box the bot holds for the villager.
    pub ray_on_box: bool,
}

/// One tick of the pair: the main bot's view of the second bot, and the second bot's own position.
#[derive(Clone, Debug)]
pub struct BotTick {
    pub view_x: Option<f64>,
    pub own_x: f64,
    pub main_len: usize,
    pub other_len: usize,
}

pub static MOB_ON: AtomicBool = AtomicBool::new(false);
pub static BOT_ON: AtomicBool = AtomicBool::new(false);
pub static MOB_TRACE: Mutex<Vec<MobTick>> = Mutex::new(Vec::new());
pub static BOT_TRACE: Mutex<Vec<BotTick>> = Mutex::new(Vec::new());
/// The bots of the swarm and the main bot's view entries as of the last traced tick (evidence for the rows).
static WHO: Mutex<String> = Mutex::new(String::new());
/// The most entries any bot's view held while a row watched, and the cap hits seen.
static PEAK: AtomicUsize = AtomicUsize::new(0);
static CAP_HITS: AtomicUsize = AtomicUsize::new(0);

// Spec: specs/F5-version-layer.md §4.6 (What Null computes: interpolation of other entities)
/// Adds the probes to the bot's app. `main` is the runner's account; the second bot is `<main>2`.
pub struct InterpolationProbe {
    pub main: String,
}

impl Plugin for InterpolationProbe {
    fn build(&self, app: &mut App) {
        let main = self.main.clone();
        let other = format!("{main}2");
        app.add_systems(
            GameTick,
            (move |me: Query<(&Account, &Position, &EntityDimensions, &LookDirection, Option<&HitResultComponent>), With<LocalEntity>>,
                   mobs: Query<(&EntityKindComponent, &Position, &LastBoundingBox), Without<LocalEntity>>,
                   bots: Query<(Entity, &Account, &Position, Option<&PeerView>), With<LocalEntity>>,
                   players: Query<(Entity, &EntityKindComponent, &Position, Option<&azalea::core::entity_id::MinecraftEntityId>, Has<LocalEntity>)>| {
                if MOB_ON.load(Ordering::Relaxed)
                    && let Some((_, p, d, look, hit)) = me.iter().find(|(a, ..)| a.username() == main)
                    && let Some((_, mp, mbox)) = mobs.iter().find(|(k, ..)| k.0 == EntityKind::Villager)
                {
                    let eye = p.up(d.eye_height.into());
                    let to = eye + view_vector(*look) * 6.0;
                    let hit_mob = hit.is_some_and(|h| match &**h {
                        HitResult::Entity(r) => mobs.get(r.entity).is_ok_and(|(k, ..)| k.0 == EntityKind::Villager),
                        _ => false,
                    });
                    let b: &Aabb = &mbox.0;
                    MOB_TRACE.lock().unwrap().push(MobTick {
                        x: mp.x,
                        box_centre: (b.min.x + b.max.x) / 2.0,
                        hit: hit_mob,
                        ray_on_box: b.clip(eye, to).is_some(),
                    });
                }
                if BOT_ON.load(Ordering::Relaxed) {
                    let find = |name: &str| bots.iter().find(|(_, a, ..)| a.username() == name);
                    if let (Some((_, _, _, main_view)), Some((other_e, _, other_pos, other_view))) = (find(&main), find(&other)) {
                        // Companion mode (`<main>c`) puts a third bot in range: the rows are about the second bot's entries, so the
                        // companion's are left out of the counts. Spec: specs/plans/COMPANION_BRIEF.md (checks)
                        let companion = find(&format!("{main}c")).map(|b| b.0);
                        let count = |v: Option<&PeerView>| v.map_or(0, |v| v.iter().filter(|(e, _)| Some(*e) != companion).count());
                        let (ml, ol) = (count(main_view), count(other_view));
                        for v in [main_view, other_view].into_iter().flatten() {
                            PEAK.fetch_max(v.peak(), Ordering::Relaxed);
                            CAP_HITS.fetch_max(v.cap_hits() as usize, Ordering::Relaxed);
                        }
                        let all: Vec<String> = bots.iter().map(|(e, a, p, _)| format!("{}={e:?}@({:.1},{:.1},{:.1})", a.username(), p.x, p.y, p.z)).collect();
                        let seen: Vec<String> = main_view.into_iter().flat_map(|v| v.iter().map(|(e, pe)| format!("{e:?}@{:.1}", pe.position.x))).collect();
                        let pl: Vec<String> = players.iter().filter(|(_, k, ..)| k.0 == EntityKind::Player).map(|(e, _, p, id, l)| format!("{e:?} id {:?} local {l} x{:.1}", id.map(|i| i.0), p.x)).collect();
                        *WHO.lock().unwrap() = format!("swarm [{}]; main's view [{}]; every player entity [{}]", all.join(", "), seen.join(", "), pl.join("; "));
                        BOT_TRACE.lock().unwrap().push(BotTick {
                            view_x: main_view.and_then(|v| v.get(other_e)).map(|e| e.position.x),
                            own_x: other_pos.x,
                            main_len: ml,
                            other_len: ol,
                        });
                    }
                }
            })
            .after(update_hit_result_component)
            .before(PhysicsSystems)
            .ambiguous_with_all(),
        );
    }
}

/// Run lengths of a sequence: (value, how many consecutive samples), values within `tol` joined.
fn runs(xs: &[f64], tol: f64) -> Vec<(f64, usize)> {
    let mut out: Vec<(f64, usize)> = Vec::new();
    for &x in xs {
        match out.last_mut() {
            Some((v, n)) if (x - *v).abs() <= tol => *n += 1,
            _ => out.push((x, 1)),
        }
    }
    out
}

/// What vanilla's `InterpolationHandler#interpolate` gives for a move of `gap` from `from`: a third
/// of what is left, then a half, then the rest.
fn vanilla_steps(from: f64, gap: f64) -> Vec<f64> {
    let to = from + gap;
    let mut at = from;
    let mut out = vec![from];
    for left in [3.0, 2.0, 1.0] {
        at += (to - at) / left;
        out.push(at);
    }
    out
}

/// Whether `got` (run lengths) is the start value for at least one sample, then the three vanilla
/// values, the two in the middle on exactly one sample each, the last for at least one.
fn is_three_steps(got: &[(f64, usize)], from: f64, gap: f64, tol: f64) -> bool {
    let want = vanilla_steps(from, gap);
    got.len() == 4 && got.iter().zip(&want).all(|(g, w)| (g.0 - w).abs() <= tol) && got[1].1 == 1 && got[2].1 == 1
}

fn wait_for(secs: f64, mut f: impl FnMut() -> bool) -> bool {
    let end = std::time::Instant::now() + Duration::from_secs_f64(secs);
    while std::time::Instant::now() < end {
        if f() {
            return true;
        }
        sleep(Duration::from_millis(25));
    }
    f()
}

// Spec: specs/F5-version-layer.md §4.6 (What Null computes: interpolation of other entities)
/// Trace the villager while `cmd` moves it. Returns the samples.
fn trace_mob(cx: &mut Ctx, cmd: &str) -> Vec<MobTick> {
    MOB_TRACE.lock().unwrap().clear();
    MOB_ON.store(true, Ordering::Relaxed);
    wait_for(5.0, || MOB_TRACE.lock().unwrap().len() >= 6);
    cx.r(cmd);
    sleep(Duration::from_millis(1000));
    MOB_ON.store(false, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
    std::mem::take(&mut *MOB_TRACE.lock().unwrap())
}

fn show(runs: &[(f64, usize)]) -> String {
    runs.iter().map(|(v, n)| format!("{v:.3}x{n}")).collect::<Vec<_>>().join(" ")
}

// Spec: specs/F5-version-layer.md §4.6 (What Null computes: interpolation of other entities); specs/B1-movement.md §4.2
pub fn a18(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    let n = cx.name.clone();
    let other = format!("{n}2");
    cx.scene_in(AREA, &[], STAND);
    let online = cx.poll(20.0, |cx| cx.r(&format!("execute if entity {other}")).contains("passed"));
    if online.is_none() {
        out.push(Outcome { id: "A18", name: "setup".into(), pass: false, evidence: format!("{other} never came online"), shown: false, secs: 0.0 });
        return out;
    }
    // The second bot is out of the way while the villager rows run.
    cx.r(&format!("gamemode survival {other}"));
    cx.r(&format!("tp {other} {} {} {} 0 0", FAR.0, FAR.1, FAR.2));
    let summon = format!(
        "summon minecraft:villager {} {} {} {{NoAI:1b,Silent:1b,Invulnerable:1b,PersistenceRequired:1b,Tags:[\"a18v\"],Rotation:[0f,0f]}}",
        VILLAGER.0, VILLAGER.1, VILLAGER.2
    );
    cx.r(&summon);
    sleep(Duration::from_millis(1500));

    // ---- row 1: three equal steps on three consecutive ticks --------------------------------
    out.push(cx.run("A18", "mob_moved_by_rcon_is_reached_in_three_steps", |cx| {
        let t = trace_mob(cx, &format!("execute as {SEL} at @s run tp @s ~6 ~ ~"));
        let xs: Vec<f64> = t.iter().map(|m| m.x).collect();
        let r = runs(&xs, 0.002);
        let ok = is_three_steps(&r, VILLAGER.0, 6.0, 0.005);
        // The box follows the entity in the same tick (the next tick's ray reads it).
        let box_ok = t.iter().all(|m| (m.box_centre - m.x).abs() < 0.001);
        let want = vanilla_steps(VILLAGER.0, 6.0).iter().map(|v| format!("{v:.3}")).collect::<Vec<_>>().join(" ");
        (
            ok && box_ok,
            format!(
                "RCON tp +6 blocks; {} ticks traced; the bot's villager x in runs: {} (want {want}, each middle value on one tick); box centre equals x on every tick: {box_ok}",
                t.len(),
                show(&r)
            ),
        )
    }));
    // Back to the bot's aim, then a move small enough that the box leaves the ray in the middle
    // of the run: 0.6 in steps of 0.2 (the box is 0.6 wide, so it covers the ray up to 0.3 off).
    cx.r(&format!("execute as {SEL} at @s run tp @s {} {} {}", VILLAGER.0, VILLAGER.1, VILLAGER.2));
    sleep(Duration::from_millis(1500));
    out.push(cx.run("A18", "ray_reads_the_smoothed_box_step_by_step", |cx| {
        let t = trace_mob(cx, &format!("execute as {SEL} at @s run tp @s ~0.6 ~ ~"));
        let xs: Vec<f64> = t.iter().map(|m| m.x).collect();
        let r = runs(&xs, 0.002);
        let steps_ok = is_three_steps(&r, VILLAGER.0, 0.6, 0.004);
        // Each sample: the ray the bot cast agrees with the box it holds in that sample.
        let agree = t.iter().all(|m| m.hit == m.ray_on_box);
        // The ray is on the villager while the box is within 0.3 of it: at the start and after one
        // step (0.2 off), off after two (0.4 off) and at the end (0.6 off).
        let hit_at = |i: usize| t.iter().position(|m| (m.x - (VILLAGER.0 + 0.2 * i as f64)).abs() < 0.004).map(|p| t[p].hit);
        let pattern = [hit_at(0), hit_at(1), hit_at(2), hit_at(3)];
        let pattern_ok = pattern == [Some(true), Some(true), Some(false), Some(false)];
        (
            steps_ok && agree && pattern_ok,
            format!(
                "RCON tp +0.6; villager x runs: {}; ray hit at offsets 0/0.2/0.4/0.6: {pattern:?} (want true,true,false,false: the ray reads the moving box, not the server's end position); ray agrees with the held box on every tick: {agree}",
                show(&r)
            ),
        )
    }));
    cx.r(&format!("kill {SEL}"));

    // ---- rows 3-5: the bot-to-bot view -----------------------------------------------------
    cx.r(&format!("tp {other} {} {} {} 0 0", NEAR.0, NEAR.1, NEAR.2));
    BOT_TRACE.lock().unwrap().clear();
    BOT_ON.store(true, Ordering::Relaxed);
    sleep(Duration::from_millis(1500));
    out.push(cx.run("A18", "two_bots_near_each_other_hold_each_other_from_server_packets", |_| {
        let last = BOT_TRACE.lock().unwrap().last().cloned();
        let (ml, ol, peak) = last.as_ref().map_or((0, 0, 0), |b| (b.main_len, b.other_len, PEAK.load(Ordering::Relaxed)));
        let view_close = last
            .as_ref()
            .and_then(|b| b.view_x.map(|v| (v - b.own_x).abs() < 0.05))
            .unwrap_or(false);
        (
            ml == 1 && ol == 1 && peak >= 1 && view_close,
            format!(
                "second bot 6 blocks away: entries now {ml} in the main bot's view and {ol} in the second bot's (want 1 each); peak entries seen {peak}; cap hits {}; the main bot's view of it is within 0.05 of its true x: {view_close}; {}",
                CAP_HITS.load(Ordering::Relaxed),
                WHO.lock().unwrap()
            ),
        )
    }));

    out.push(cx.run("A18", "server_move_of_a_bot_shows_in_the_other_bots_view_in_three_steps", |cx| {
        BOT_TRACE.lock().unwrap().clear();
        wait_for(3.0, || BOT_TRACE.lock().unwrap().len() >= 6);
        cx.r(&format!("tp {other} {} {} {} 0 0", MOVED.0, MOVED.1, MOVED.2));
        sleep(Duration::from_millis(1000));
        let t = BOT_TRACE.lock().unwrap().clone();
        let views: Vec<f64> = t.iter().filter_map(|b| b.view_x).collect();
        let r = runs(&views, 0.002);
        let ok = is_three_steps(&r, NEAR.0, MOVED.0 - NEAR.0, 0.005);
        // When the view first moves, the second bot's own position is already at the new place.
        let first_moved = t.iter().find(|b| b.view_x.is_some_and(|v| (v - NEAR.0).abs() > 0.5));
        let own_ahead = first_moved.is_some_and(|b| (b.own_x - MOVED.0).abs() < 0.05);
        let kept = t.iter().all(|b| b.main_len == 1);
        (
            ok && own_ahead && kept,
            format!(
                "RCON tp of the second bot +6 blocks: the main bot's view x in runs: {} (want 3 vanilla steps, each middle value on one tick); the bot's own x when the view first moved: {:?} (want {}); the bot stayed in the view throughout: {kept}; {}",
                show(&r),
                first_moved.map(|b| b.own_x),
                MOVED.0,
                WHO.lock().unwrap()
            ),
        )
    }));

    out.push(cx.run("A18", "bot_beyond_16_blocks_is_not_kept_and_comes_back", |cx| {
        cx.r(&format!("tp {other} {} {} {} 0 0", FAR.0, FAR.1, FAR.2));
        sleep(Duration::from_millis(1500));
        let far = BOT_TRACE.lock().unwrap().last().cloned();
        let (fm, fo) = far.as_ref().map_or((9, 9), |b| (b.main_len, b.other_len));
        let none_view = far.as_ref().is_some_and(|b| b.view_x.is_none());
        cx.r(&format!("tp {other} {} {} {} 0 0", NEAR.0, NEAR.1, NEAR.2));
        sleep(Duration::from_millis(1500));
        let back = BOT_TRACE.lock().unwrap().last().cloned();
        let (bm, bo) = back.as_ref().map_or((9, 9), |b| (b.main_len, b.other_len));
        let peak = PEAK.load(Ordering::Relaxed);
        (
            fm == 0 && fo == 0 && none_view && bm == 1 && bo == 1 && peak >= 1,
            format!(
                "second bot 18 blocks away: entries {fm} and {fo} (want 0, nothing kept); back at 6 blocks: {bm} and {bo} (want 1); peak entries any bot held (counter): {peak} (want at least 1); {}",
                WHO.lock().unwrap()
            ),
        )
    }));
    BOT_ON.store(false, Ordering::Relaxed);
    cx.r(&format!("tp {other} {} {} {} 0 0", FAR.0, FAR.1, FAR.2));
    cx.clean_in(AREA);
    out
}
