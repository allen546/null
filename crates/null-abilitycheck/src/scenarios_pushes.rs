//! A19: pushes from other entities, teams and the entity tick order (fork patch 32; P30.1). Plan:
//! specs/plans/PATCH_30_TIER1.md §2.3 and §4, specs/B1-movement.md §4.2. A vanilla client pushes
//! the local player from the living entities near it (`LivingEntity#pushEntities`,
//! `Entity#push(Entity)`) and the server never tells it; so the bot computes the push alone and
//! the vanilla server sees only the resulting positions. The exact check is a model written from
//! the vanilla text (it shares no code with the fork) against the bot's own per-tick trace, taken
//! by a probe in its `GameTick` after both push passes. Server truth: RCON (the bot's position at
//! the end equals the trace's last position; the mob's position; `team list` for membership), and
//! the server log (the `Ctx::run` scan for "moved wrongly" and Grim alerts).
//!
//! Rows here: `pushed_by_mob`, `team_never_no_push`, `armor_stand_no_push` (task 1), `pushed_by_bot`,
//! `pushed_by_boat`, `team_push_own_team_allied_no_push`, `push_tick_order` (task 2). The second bot
//! is `<account>2`; its trace has its own buffer.
//! The mobs have no AI, no gravity and no sound, so the server does not move them. The bot logs in
//! before any mob exists, so it ticks first and a mob's push lands after the bot's move.
//! Area: corner (950, 342), floor y=199.

use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::Duration,
};

use azalea::{
    account::Account,
    app::{App, Plugin},
    core::tick::GameTick,
    ecs::prelude::*,
    entity::{
        EntityKindComponent, HasClientLoaded, LocalEntity, Physics, Position, dimensions::EntityDimensions,
        interpolation::PeerView, push::PushRecord,
    },
    pushes::PushSystems,
    registry::builtin::EntityKind,
    tick_counter::TicksConnected,
};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome},
    scenarios_block_effects::{bot_pos, floats},
};

const AREA: (i32, i32) = (950, 342);
/// The bot stands level, in the middle of the area.
const STAND: (f64, f64, f64) = (960.5, 200.0, 352.5);
const SEL: &str = "@e[tag=a19c,limit=1]";
const TEAM: &str = "a19t";
/// The vanilla ground drag on a stone block at rest: block friction 0.6F times 0.91F, in floats.
const FRICTION: f64 = (0.6f32 * 0.91f32) as f64;
/// A box half-width as the game computes it (`width / 2` in floats): the bot 0.6, a cow 0.9.
const BOT_HALF: f64 = (0.6f32 / 2.0) as f64;
const COW_HALF: f64 = (0.9f32 / 2.0) as f64;

/// One tick of the bot as it ended: the probe copies it from the ECS after both push passes.
#[derive(Clone, Debug)]
pub struct PushTick {
    pub tick: u64,
    pub x: f64,
    pub z: f64,
    pub vx: f64,
    pub vz: f64,
    pub on_ground: bool,
    /// The fork's evidence: pushes applied so far, and the velocity they added in this tick.
    pub pushes_total: u64,
    pub pushed: (f64, f64),
    pub max_in_tick: u32,
    /// Entities whose box strictly meets the bot's box at the end of this tick.
    pub touching: Vec<EntityKind>,
    /// The other bot as this bot's patch 31 view holds it (smoothed), when the view has it.
    pub peer: Option<(f64, f64)>,
    /// The nearest cow, boat or armor stand within 3 blocks, at the position the push passes used
    /// (the probe runs after pass B, which runs after the interpolation step of this tick).
    pub near: Option<(f64, f64)>,
}

pub static TRACE_ON: AtomicBool = AtomicBool::new(false);
pub static TRACE: Mutex<Vec<PushTick>> = Mutex::new(Vec::new());
/// The second bot's own trace (`<account>2`), on while `TRACE_ON` is.
pub static TRACE2: Mutex<Vec<PushTick>> = Mutex::new(Vec::new());

// Spec: specs/B1-movement.md §4.2 (Pushes from entities); the probe is the Null side of patch 32's rows
/// Adds the trace probe to the bot's app. `main` is the runner's account.
pub struct PushProbe {
    pub main: String,
}

impl Plugin for PushProbe {
    fn build(&self, app: &mut App) {
        let main = self.main.clone();
        app.add_systems(
            GameTick,
            (move |me: Query<(&Account, &Position, &Physics, &PushRecord, &TicksConnected, &EntityDimensions, Option<&PeerView>), (With<LocalEntity>, With<HasClientLoaded>)>,
                   others: Query<(&EntityKindComponent, &Position, &EntityDimensions), Without<LocalEntity>>| {
                if !TRACE_ON.load(Ordering::Relaxed) {
                    return;
                }
                for (a, p, ph, rec, t, d, view) in &me {
                    let buf = if a.username() == main {
                        &TRACE
                    } else if a.username() == format!("{main}2") {
                        &TRACE2
                    } else {
                        continue;
                    };
                    let mine = d.make_bounding_box(**p);
                    let touching = others
                        .iter()
                        .filter(|(_, op, od)| mine.intersects_aabb(&od.make_bounding_box(***op)))
                        .map(|(k, ..)| k.0)
                        .collect();
                    let near = others
                        .iter()
                        .filter(|(k, ..)| matches!(k.0, EntityKind::Cow | EntityKind::OakBoat | EntityKind::ArmorStand))
                        .map(|(_, op, _)| (op.x, op.z))
                        .filter(|o| (o.0 - p.x).abs() < 3.0 && (o.1 - p.z).abs() < 3.0)
                        .min_by(|a, b| (a.0 - p.x).abs().total_cmp(&(b.0 - p.x).abs()));
                    let peer = view.and_then(|v| v.iter().next().map(|(_, e)| (e.position.x, e.position.z)));
                    buf.lock().unwrap().push(PushTick {
                        tick: t.0,
                        x: p.x,
                        z: p.z,
                        vx: ph.velocity.x,
                        vz: ph.velocity.z,
                        on_ground: ph.on_ground(),
                        pushes_total: rec.total,
                        pushed: rec.tick_delta,
                        max_in_tick: rec.max_in_tick,
                        touching,
                        peer,
                        near,
                    });
                }
            })
            .after(PushSystems)
            .ambiguous_with_all(),
        );
    }
}

fn trace_start() {
    TRACE.lock().unwrap().clear();
    TRACE2.lock().unwrap().clear();
    TRACE_ON.store(true, Ordering::Relaxed);
}

fn trace_stop() -> Vec<PushTick> {
    TRACE_ON.store(false, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
    std::mem::take(&mut *TRACE.lock().unwrap())
}

fn trace2_take() -> Vec<PushTick> {
    std::mem::take(&mut *TRACE2.lock().unwrap())
}

/// What vanilla's `Entity#push(Entity)` adds to a player's velocity for a pusher at `(dx, dz)`
/// from it, written from the vanilla text: the larger axis under 0.01 does nothing; divide both
/// axes by its square root, scale by min(1, 1/√), then by 0.05, and push away from the pusher.
fn vanilla_push(dx: f64, dz: f64) -> Option<(f64, f64)> {
    let f = dx.abs().max(dz.abs());
    if f < 0.01f32 as f64 {
        return None;
    }
    let root = f.sqrt();
    let scale = (1.0 / root).min(1.0) * 0.05f32 as f64;
    Some((-(dx / root * scale), -(dz / root * scale)))
}

/// One tick of an idle bot on stone with a still cow at `cow` that ticks after the bot: the speed
/// floor, the move, the ground drag, then the cow's push against the box where the bot ended
/// (`LivingEntity#aiStep`, `#travelInAir`, `ClientLevel#getPushableEntities` with its strict test).
fn model_step(s: (f64, f64, f64, f64), cow: (f64, f64), reach: f64) -> (f64, f64, f64, f64) {
    let (mut x, mut z, mut vx, mut vz) = s;
    if vx * vx + vz * vz < 9.0e-6 {
        vx = 0.0;
        vz = 0.0;
    }
    x += vx;
    z += vz;
    vx *= FRICTION;
    vz *= FRICTION;
    if (cow.0 - x).abs() < reach && (cow.1 - z).abs() < reach
        && let Some((px, pz)) = vanilla_push(cow.0 - x, cow.1 - z)
    {
        vx += px;
        vz += pz;
    }
    (x, z, vx, vz)
}

/// The model against a trace from the first push on (one sample before it, so the first push is
/// predicted too): ticks checked, the worst error in x and in speed, the first miss.
struct Fit {
    checked: usize,
    worst: f64,
    miss: Option<String>,
}

fn fit(trace: &[PushTick], cow: (f64, f64)) -> Fit {
    fit_with(trace, BOT_HALF + COW_HALF, |_| cow)
}

/// `fit` for any pusher: `reach` is the half-widths (plus any band) summed, and `pusher` gives its
/// position as the sample at the end of a tick shows it (the bot-to-bot case reads the view).
fn fit_with(trace: &[PushTick], reach: f64, pusher: impl Fn(&PushTick) -> (f64, f64)) -> Fit {
    let mut f = Fit { checked: 0, worst: 0.0, miss: None };
    let Some(k0) = trace.windows(2).position(|w| w[1].pushes_total > w[0].pushes_total) else {
        return f;
    };
    for w in trace[k0..].windows(2) {
        if w[1].tick != w[0].tick + 1 {
            f.miss.get_or_insert(format!("tick {} to {} (a gap in the trace)", w[0].tick, w[1].tick));
            break;
        }
        // A teleport (the other bot joining the cell) is not a model step, and the model has the
        // ground drag only: skip a pair that starts in the air (the tick after a teleport).
        if (w[1].x - w[0].x).abs() > 1.0 || (w[1].z - w[0].z).abs() > 1.0 || !w[0].on_ground {
            continue;
        }
        let m = model_step((w[0].x, w[0].z, w[0].vx, w[0].vz), pusher(&w[1]), reach);
        let err = (m.0 - w[1].x).abs().max((m.1 - w[1].z).abs()).max((m.2 - w[1].vx).abs()).max((m.3 - w[1].vz).abs());
        f.checked += 1;
        f.worst = f.worst.max(err);
        if err > 1e-6 {
            f.miss.get_or_insert(format!(
                "tick {}: bot x {:.9} vx {:.9}, model x {:.9} vx {:.9}",
                w[1].tick, w[1].x, w[1].vx, m.0, m.2
            ));
        }
    }
    f
}

/// Position of the tagged mob by RCON, (x, z).
fn mob_pos(cx: &mut Ctx) -> Option<(f64, f64)> {
    let f = floats(&cx.r(&format!("data get entity {SEL} Pos")));
    let k = f.len().checked_sub(3)?;
    Some((f[k], f[k + 2]))
}

fn summon(cx: &mut Ctx, kind: &str, dx: f64) {
    summon_at(cx, kind, dx, STAND.1);
}

fn summon_at(cx: &mut Ctx, kind: &str, dx: f64, y: f64) {
    cx.r(&format!(
        "summon minecraft:{kind} {} {y} {} {{NoAI:1b,NoGravity:1b,Silent:1b,Invulnerable:1b,PersistenceRequired:1b,Tags:[\"a19c\"]}}",
        STAND.0 + dx, STAND.2
    ));
}

/// Put the bot back at the stand with no mob, and wait for it to settle.
fn reset(cx: &mut Ctx) {
    let n = cx.name.clone();
    // A killed mob stays on the client for its death animation (about 20 ticks) and pushes until
    // the server removes it; wait until it is gone before the bot goes back.
    cx.r("kill @e[tag=a19c]");
    // Members leave before the team goes: Grim's `EntityTeam` never fills `entries`, so a removed
    // team keeps its members mapped and Grim then allows no push from them (ABILITY_CHECK, A19).
    cx.r(&format!("team empty {TEAM}"));
    cx.r(&format!("team remove {TEAM}"));
    cx.poll(5.0, |cx| cx.r("execute if entity @e[tag=a19c]").contains("failed"));
    sleep(Duration::from_millis(1500));
    cx.r(&format!("tp {n} {} {} {} 0 0", STAND.0, STAND.1, STAND.2));
    sleep(Duration::from_millis(1500));
}

fn members(cx: &mut Ctx) -> String {
    cx.r(&format!("team list {TEAM}")).trim().to_string()
}

fn wait_for_ticks(n: usize) {
    let end = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < end && TRACE.lock().unwrap().len() < n {
        sleep(Duration::from_millis(25));
    }
}

/// Samples taken so far.
fn samples() -> usize {
    TRACE.lock().unwrap().len()
}

fn show_fit(f: &Fit) -> String {
    format!(
        "{} ticks checked against the vanilla model, worst error {:.2e}{}",
        f.checked,
        f.worst,
        f.miss.as_ref().map(|m| format!(", FIRST MISS {m}")).unwrap_or_default()
    )
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities)
fn pushed_by_mob(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "pushed_by_mob", |cx| {
        reset(cx);
        trace_start();
        wait_for_ticks(6);
        summon(cx, "cow", 0.3);
        sleep(Duration::from_millis(2500));
        let t = trace_stop();
        let cow = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let f = fit(&t, cow);
        let last = t.last().cloned();
        let pushes = last.as_ref().map_or(0, |l| l.pushes_total);
        let grounded = t.iter().all(|s| s.on_ground);
        let end = last.as_ref().map_or(f64::NAN, |l| l.x);
        let apart = (cow.0 - end).abs() >= BOT_HALF + COW_HALF;
        let server = bot_pos(cx);
        let same = last.as_ref().is_some_and(|l| (server[0] - l.x).abs() < 1e-6 && (server[2] - l.z).abs() < 1e-6);
        // The first push came with the cow in contact: the probe saw the cow touching the bot.
        let touched = t.iter().any(|s| s.touching.contains(&EntityKind::Cow));
        (
            f.miss.is_none() && f.checked >= 8 && pushes >= 3 && grounded && apart && same && touched,
            format!(
                "a no-AI cow summoned 0.3 from the idle bot (server says it stands at x {:.4}, z {:.4}): {pushes} pushes applied; {}; the bot ended at x {end:.4}, {:.3} from the cow (want at least {:.2}, boxes apart); server Pos x {:.6} equals the trace end: {same}; cow touched the bot's box: {touched}; most pushes in one tick {}",
                cow.0, cow.1, show_fit(&f), (cow.0 - end).abs(), BOT_HALF + COW_HALF, server[0],
                last.as_ref().map_or(0, |l| l.max_in_tick)
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: armor stand)
fn armor_stand_no_push(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "armor_stand_no_push", |cx| {
        reset(cx);
        trace_start();
        wait_for_ticks(6);
        summon(cx, "armor_stand", 0.3);
        sleep(Duration::from_millis(2000));
        let t = trace_stop();
        let stand_ticks = t.iter().filter(|s| s.touching.contains(&EntityKind::ArmorStand)).count();
        let pushes = t.last().map_or(0, |l| l.pushes_total) - t.first().map_or(0, |l| l.pushes_total);
        let mut kinds: Vec<String> = t.iter().flat_map(|s| s.touching.iter().map(|k| format!("{k:?}"))).collect();
        kinds.sort();
        kinds.dedup();
        let moved = t.iter().map(|s| (s.x - STAND.0).abs()).fold(0.0, f64::max);
        let still = t.iter().all(|s| s.vx == 0.0 && s.vz == 0.0);
        let server = bot_pos(cx);
        (
            stand_ticks >= 20 && pushes == 0 && moved < 1e-9 && still && (server[0] - STAND.0).abs() < 1e-6,
            format!(
                "an armor stand 0.3 from the idle bot: its box met the bot's on {stand_ticks} ticks (kinds touching at any time: {kinds:?}), {pushes} pushes, the bot moved at most {moved:.2e} and had speed 0 on every tick: {still}; server Pos x {:.6} (stand at {})",
                server[0], STAND.0
            ),
        )
    })
}

/// A short account of a trace slice for the evidence: samples, x range, top speed, pushes, kinds touching.
fn describe(part: &[PushTick]) -> String {
    let xs = part.iter().map(|s| s.x);
    let (lo, hi) = (xs.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max));
    let v = part.iter().map(|s| s.vx.abs()).fold(0.0, f64::max);
    let pushes = part.last().map_or(0, |l| l.pushes_total) - part.first().map_or(0, |l| l.pushes_total);
    let mut kinds: Vec<String> = part.iter().flat_map(|s| s.touching.iter().map(|k| format!("{k:?}"))).collect();
    kinds.sort();
    kinds.dedup();
    format!("{} samples, x {lo:.4}..{hi:.4}, top |vx| {v:.4}, {pushes} pushes, touching {kinds:?}", part.len())
}

/// Run `n` ticks with the cow in place and report (pushes added, bot moved at most).
fn hold(n: usize) -> (u64, f64, usize) {
    let from = samples();
    wait_for_ticks(from + n);
    let t = TRACE.lock().unwrap();
    let part = &t[from.min(t.len())..];
    let pushes = part.last().map_or(0, |l| l.pushes_total) - part.first().map_or(0, |l| l.pushes_total);
    let moved = part.iter().map(|s| (s.x - STAND.0).abs()).fold(0.0, f64::max);
    (pushes, moved, part.len())
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: team collision rules)
fn team_never_no_push(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "team_never_no_push", |cx| {
        let n = cx.name.clone();
        reset(cx);
        cx.r(&format!("team add {TEAM}"));
        cx.r(&format!("team modify {TEAM} collisionRule never"));
        // Phase A: only the bot is in the team (the rule is read from the target's team too).
        cx.r(&format!("team join {TEAM} {n}"));
        sleep(Duration::from_millis(300));
        summon(cx, "cow", 0.3);
        trace_start();
        sleep(Duration::from_millis(500));
        let ma = members(cx);
        let a = hold(40);
        // Phase B: only the cow (a mob is a member by its UUID text).
        cx.r(&format!("team leave {n}"));
        cx.r(&format!("team join {TEAM} {SEL}"));
        sleep(Duration::from_millis(300));
        let mb = members(cx);
        let b = hold(40);
        // Phase C: both.
        cx.r(&format!("team join {TEAM} {n}"));
        sleep(Duration::from_millis(300));
        let mc = members(cx);
        let c = hold(40);
        // Phase D: neither (Leave on both): the cow pushes again, as the model says.
        cx.r(&format!("team leave {n}"));
        cx.r(&format!("team leave {SEL}"));
        sleep(Duration::from_millis(300));
        let md = members(cx);
        let mark = samples();
        sleep(Duration::from_millis(2500));
        let t = trace_stop();
        let cow = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let after: Vec<PushTick> = t[mark.min(t.len())..].to_vec();
        let f = fit(&after, cow);
        let resumed = after.last().map_or(0, |l| l.pushes_total) > after.first().map_or(0, |l| l.pushes_total);
        let end = after.last().map_or(STAND.0, |l| l.x);
        let ok_members = ma.contains("1 member") && mb.contains("1 member") && mc.contains("2 member") && (md.contains("no members") || md.contains("0 member"));
        (
            a.0 == 0 && b.0 == 0 && c.0 == 0 && a.1 < 1e-9 && b.1 < 1e-9 && c.1 < 1e-9 && a.2 >= 20 && b.2 >= 20 && c.2 >= 20
                && resumed && f.miss.is_none() && f.checked >= 8 && end < STAND.0 - 0.01 && ok_members,
            format!(
                "team {TEAM} collisionRule never; cow 0.3 from the bot. Only the bot in it ({ma:?}): {} pushes in {} ticks, bot moved {:.1e}; only the cow ({mb:?}): {} pushes in {} ticks, moved {:.1e}; both ({mc:?}): {} pushes in {} ticks, moved {:.1e}; after Leave on both ({md:?}): pushes resumed {resumed}, {}; the bot ended at x {end:.4}",
                a.0, a.2, a.1, b.0, b.2, b.1, c.0, c.2, c.1, show_fit(&f)
            ),
        )
    })
}

/// A boat's half-width (`1.375F / 2`) and the sideways band of its push box (`inflate(0.2F, ..)`).
const BOAT_HALF: f64 = (1.375f32 / 2.0) as f64;
const BOAT_BAND: f64 = 0.2f32 as f64;

/// Position of an online player by RCON, (x, z).
fn player_pos(cx: &mut Ctx, who: &str) -> (f64, f64) {
    let f = floats(&cx.r(&format!("data get entity {who} Pos")));
    match f.len().checked_sub(3) {
        Some(k) => (f[k], f[k + 2]),
        None => (f64::NAN, f64::NAN),
    }
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: the other bot, read from its view)
fn pushed_by_bot(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "pushed_by_bot", |cx| {
        let other = format!("{}2", cx.name);
        if cx.poll(20.0, |cx| cx.r(&format!("execute if entity {other}")).contains("passed")).is_none() {
            return (false, format!("{other} never came online"));
        }
        cx.r(&format!("gamemode survival {other}"));
        // Both bots stand on the same spot first (offset 0 pushes nothing: the larger axis is under
        // 0.01), so each already holds the other in its view; then the second steps 0.3 aside.
        // Teleporting it in from afar would let it push the first bot away before the first bot's
        // smoothed view of it arrives (a vanilla client would see the same lag).
        reset(cx);
        cx.r(&format!("tp {other} {} {} {} 0 0", STAND.0, STAND.1, STAND.2));
        sleep(Duration::from_millis(2000));
        let log_mark = cx.log_len();
        trace_start();
        wait_for_ticks(6);
        cx.r(&format!("tp {other} {} {} {} 0 0", STAND.0 + 0.3, STAND.1, STAND.2));
        sleep(Duration::from_millis(2500));
        let t1 = trace_stop();
        let t2 = trace2_take();
        let peer = |s: &PushTick| s.peer.unwrap_or((f64::NAN, f64::NAN));
        // Each bot's push is read from that bot's own view of the other, half-widths 0.3 + 0.3.
        let f1 = fit_with(&t1, 2.0 * BOT_HALF, peer);
        // The second bot's own step aside (the teleport) is not a model step: start at its first sample there.
        let from2 = t2.iter().position(|s| (s.x - STAND.0).abs() > 0.1).unwrap_or(0);
        let f2 = fit_with(&t2[from2..], 2.0 * BOT_HALF, peer);
        let (l1, l2) = (t1.last().cloned(), t2.last().cloned());
        let me = cx.name.clone();
        let (s1, s2) = (player_pos(cx, &me), player_pos(cx, &other));
        let same = |l: &Option<PushTick>, s: (f64, f64)| l.as_ref().is_some_and(|l| (s.0 - l.x).abs() < 1e-6 && (s.1 - l.z).abs() < 1e-6);
        let apart = (s1.0 - s2.0).abs() >= 2.0 * BOT_HALF;
        let (p1, p2) = (l1.as_ref().map_or(0, |l| l.pushes_total), l2.as_ref().map_or(0, |l| l.pushes_total));
        // `Ctx::run` skips lines that name another `null_` account, so read the second bot's alerts here.
        sleep(Duration::from_millis(300));
        let mine = std::mem::replace(&mut cx.name, other.clone());
        let (second_warn, second_grim) = cx.log_problems(log_mark);
        cx.name = mine;
        let peers1 = t1.iter().filter(|s| s.peer.is_some()).count();
        let peers_last = t1.last().and_then(|l| l.peer);
        let most = l1.iter().chain(l2.iter()).map(|l| l.max_in_tick).max().unwrap_or(0);
        (
            f1.miss.is_none() && f2.miss.is_none() && f1.checked >= 3 && f2.checked >= 3 && p1 >= 1 && p2 >= 2
                && second_warn.is_empty() && second_grim.is_empty() && apart && same(&l1, s1) && same(&l2, s2) && s1.0 < STAND.0 && s2.0 > STAND.0 + 0.3,
            format!(
                "two bots 0.3 apart: first bot got {p1} pushes ({}), second {p2} ({}); both moved apart: first server x {:.4}, second {:.4} (from {} and {}), gap {:.3} (want at least {:.1}); server Pos equals the trace end for both: {} {}; most pushes in one tick {most}; the first bot's view held the other in {peers1} of {} samples, last place {peers_last:?}; first bot: {}; second bot: {}",
                show_fit(&f1), show_fit(&f2), s1.0, s2.0, STAND.0, STAND.0 + 0.3, (s1.0 - s2.0).abs(), 2.0 * BOT_HALF,
                same(&l1, s1), same(&l2, s2), t1.len(), describe(&t1), describe(&t2)
            ) + &format!("; the second bot's log lines: {} warnings, Grim alerts {second_grim:?}", second_warn.len()),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: the boat band and its height rule)
fn pushed_by_boat(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "pushed_by_boat", |cx| {
        // Boat 1.1 to the side: the boxes have a gap of 0.11 (under the 0.2 band), so no overlap is needed.
        // Phase 1: its bottom (y 200.5) is above the bot's feet (200.0), so it pushes.
        reset(cx);
        trace_start();
        wait_for_ticks(6);
        summon_at(cx, "oak_boat", 1.1, 200.5);
        sleep(Duration::from_millis(2500));
        let t = trace_stop();
        let boat = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let nan = (f64::NAN, f64::NAN);
        let f = fit_with(&t, BOT_HALF + BOAT_HALF + BOAT_BAND, |s| s.near.unwrap_or(nan));
        let last = t.last().cloned();
        let pushes = last.as_ref().map_or(0, |l| l.pushes_total);
        let end = last.as_ref().map_or(f64::NAN, |l| l.x);
        let server = bot_pos(cx);
        let same = last.as_ref().is_some_and(|l| (server[0] - l.x).abs() < 1e-6);
        let boat_end = last.as_ref().and_then(|l| l.near).unwrap_or(boat);
        let out_of_band = (boat_end.0 - end).abs() >= BOT_HALF + BOAT_HALF + BOAT_BAND || t.last().is_some_and(|l| l.near.is_none());
        let in_contact = t.iter().any(|s| s.touching.contains(&EntityKind::OakBoat));
        // Phase 2: the boat's bottom (199.5) is below the bot's feet, so `AbstractBoat#push` refuses.
        reset(cx);
        trace_start();
        wait_for_ticks(6);
        summon_at(cx, "oak_boat", 1.1, 199.5);
        sleep(Duration::from_millis(1500));
        let t2 = trace_stop();
        let low = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let pushes2 = t2.last().map_or(0, |l| l.pushes_total) - t2.first().map_or(0, |l| l.pushes_total);
        let moved2 = t2.iter().map(|s| (s.x - STAND.0).abs()).fold(0.0, f64::max);
        (
            f.miss.is_none() && f.checked >= 3 && pushes >= 2 && end < STAND.0 && out_of_band && same && !in_contact
                && pushes2 == 0 && moved2 < 1e-9,
            format!(
                "oak boat 1.1 to the side, bottom above the bot's feet (server says it stands at x {:.4} at the end; the server pushes it too, so the model reads its place from the trace): {pushes} pushes; {}; the bot ended at x {end:.4}, {:.3} from the boat (want at least {:.3}, outside the band); the boxes never met (so only the band pushed): {}; server Pos x {:.6} equals the trace end: {same}. Boat bottom 0.5 below the bot's feet (x {:.4}): {pushes2} pushes, the bot moved at most {moved2:.1e} (height rule; {})",
                boat.0, show_fit(&f), (boat_end.0 - end).abs(), BOT_HALF + BOAT_HALF + BOAT_BAND, !in_contact, server[0], low.0, describe(&t2)
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: team collision rules, pushOwnTeam)
fn team_push_own_team_allied_no_push(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "team_push_own_team_allied_no_push", |cx| {
        let n = cx.name.clone();
        // Phase A: bot and cow in one team with pushOwnTeam: allied, so no push. The cow waits 6
        // blocks away while both join (a cow outside the team would push), then comes to 0.3.
        reset(cx);
        cx.r(&format!("team add {TEAM}"));
        cx.r(&format!("team modify {TEAM} collisionRule pushOwnTeam"));
        summon(cx, "cow", 6.0);
        cx.r(&format!("team join {TEAM} {n}"));
        cx.r(&format!("team join {TEAM} {SEL}"));
        sleep(Duration::from_millis(300));
        let ma = members(cx);
        trace_start();
        wait_for_ticks(6);
        cx.r(&format!("tp {SEL} {} {} {} 0 0", STAND.0 + 0.3, STAND.1, STAND.2));
        sleep(Duration::from_millis(500));
        let a = hold(40);
        let a_diag = describe(&TRACE.lock().unwrap());
        let cow_a = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        // Phase B: the bot leaves; the cow alone is in a pushOwnTeam team. Not allied (no team never
        // is), and neither is pushOtherTeams, so it pushes.
        let mark = samples();
        cx.r(&format!("team leave {n}"));
        sleep(Duration::from_millis(300));
        let mb = members(cx);
        sleep(Duration::from_millis(2500));
        let t = trace_stop();
        let cow = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let after: Vec<PushTick> = t[mark.min(t.len())..].to_vec();
        let f = fit(&after, cow);
        let resumed = after.last().map_or(0, |l| l.pushes_total) > after.first().map_or(0, |l| l.pushes_total);
        // Phase C: pushOtherTeams with both in it: allied, so it pushes ("or when allied").
        reset(cx);
        cx.r(&format!("team add {TEAM}"));
        cx.r(&format!("team modify {TEAM} collisionRule pushOtherTeams"));
        cx.r(&format!("team join {TEAM} {n}"));
        summon(cx, "cow", 0.3);
        sleep(Duration::from_millis(800));
        trace_start();
        wait_for_ticks(6);
        cx.r(&format!("team join {TEAM} {SEL}"));
        sleep(Duration::from_millis(2000));
        let tc = trace_stop();
        let cow_c = mob_pos(cx).unwrap_or((f64::NAN, f64::NAN));
        let mc = members(cx);
        let pushes_c = tc.last().map_or(0, |l| l.pushes_total) - tc.first().map_or(0, |l| l.pushes_total);
        let fc = fit(&tc, cow_c);
        let ok_members = ma.contains("2 member") && mb.contains("1 member") && mc.contains("2 member");
        (
            a.0 == 0 && a.1 < 1e-9 && a.2 >= 20 && resumed && f.miss.is_none() && f.checked >= 8
                && pushes_c >= 2 && fc.miss.is_none() && fc.checked >= 3 && ok_members,
            format!(
                "pushOwnTeam, bot and cow in {TEAM} ({ma:?}), cow 0.3 away: {} pushes in {} ticks, bot moved {:.1e} (whole phase: {a_diag}; cow at {:.4}, {:.4}); after the bot leaves ({mb:?}): pushes resumed {resumed}, {}; pushOtherTeams with both in it ({mc:?}): {pushes_c} pushes, {}",
                a.0, a.2, a.1, cow_a.0, cow_a.1, show_fit(&f), show_fit(&fc)
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities: the entity tick order)
fn push_tick_order(cx: &mut Ctx) -> Outcome {
    cx.run("A19", "push_tick_order", |cx| {
        // Only the "bot ticks first" half is live. Entities arrive after the bot's own number
        // (login, or a respawn, which the server follows with new add packets), so no RCON setup puts
        // a pusher before the bot; that half is a fork simulation test.
        reset(cx);
        trace_start();
        wait_for_ticks(6);
        summon(cx, "cow", 0.3);
        sleep(Duration::from_millis(2000));
        let t = trace_stop();
        let Some(k) = t.windows(2).position(|w| w[1].pushes_total > w[0].pushes_total) else {
            return (false, "no push seen".into());
        };
        let (a, b) = (&t[k], &t[k + 1]);
        let Some(c) = t.get(k + 2) else {
            return (false, "trace ends at the first push".into());
        };
        // Cow after the bot: the push tick has the push in the velocity and no move; the move
        // comes one tick later by exactly that velocity.
        let still = (b.x - a.x).abs() < 1e-12 && (b.z - a.z).abs() < 1e-12;
        let gained = (b.vx - b.pushed.0).abs() < 1e-12 && b.pushed.0 != 0.0;
        let moved = (c.x - b.x - b.vx).abs() < 1e-9;
        (
            still && gained && moved,
            format!(
                "first push at tick {}: bot x {:.9} (before {:.9}, so no move in the push tick: {still}), velocity {:.6} equals the push {:.6}: {gained}; next tick x {:.9}, moved by the velocity: {moved}. A pusher that ticks before the bot (move in the push tick) has no RCON setup; covered by the fork simulation test",
                b.tick, b.x, a.x, b.vx, b.pushed.0, c.x
            ),
        )
    })
}

// Spec: specs/B1-movement.md §4.2 (Pushes from entities)
pub fn a19(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let mut out = Vec::new();
    // The second bot waits 8 blocks away so it cannot push the first one in the other rows.
    let other = format!("{}2", cx.name);
    cx.r(&format!("tp {other} {} {} {} 0 0", STAND.0 + 8.0, STAND.1, STAND.2));
    // `A19_ONLY=text` runs only the rows whose name contains it (to rerun a failed row)
    let want = |name: &str| std::env::var("A19_ONLY").map_or(true, |t| name.contains(&t));
    // The armor stand row doubles as the control for the helper: the same contact, no push.
    for (name, row) in [
        ("pushed_by_mob", pushed_by_mob as fn(&mut Ctx) -> Outcome),
        ("armor_stand_no_push", armor_stand_no_push),
        ("team_never_no_push", team_never_no_push),
        ("pushed_by_boat", pushed_by_boat),
        ("team_push_own_team_allied_no_push", team_push_own_team_allied_no_push),
        ("push_tick_order", push_tick_order),
        ("pushed_by_bot", pushed_by_bot),
    ] {
        if want(name) {
            out.push(row(cx));
        }
    }
    let n = cx.name.clone();
    cx.r("kill @e[tag=a19c]");
    cx.r(&format!("team empty {TEAM}"));
    cx.r(&format!("team remove {TEAM}"));
    cx.r(&format!("tp {n} {} {} {} 0 0", STAND.0, STAND.1, STAND.2));
    cx.clean_in(AREA);
    let _ = FLOOR_Y;
    out
}
