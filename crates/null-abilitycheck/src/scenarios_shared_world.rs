//! Shared-world prediction test: can bot B read bot A's unconfirmed block? Plan: specs/plans/SHARED_WORLD_TEST.md.
//! These rows aim to show harm, so they run only when named (`--scenario SW1`), never in `all`.
//! - SW1 (design 1, chance): A breaks an instant-break block (TNT) that the companion B stands
//!   on. A's prediction is written into the shared `World`; B's physics may read air and start to
//!   fall before the server's update could reach B. Run with `--companion` on Paper + Grim.
//! - SW1C (control): the same setup with B in a second process (`--account <A>b --scenario SW1B`,
//!   its own swarm and `World`), placed by RCON.
//! A probe in the `GameTick` of every bot logs, before and after the physics step, each watched
//! bot's view of the cell, A's pending prediction (seq above acked seq), and B's position and
//! on-ground state. Server truth is read by RCON; Grim alerts of B's account are read from the
//! server log only. Since patch 53 (per-bot prediction, now in the bot's own level) SW1 passes when the action was reached
//! and B drew zero Grim alerts and zero server-log warnings (`NO HARM`); any alert or warning fails it
//! (`HARM`). Before the fix it drew `Simulation` x4 and `GroundSpoof` x1 (`chance1.log`), so the bar can fail.
//! SW1C (control) still passes on the action reached and A pending; it has no harm bar. Area: corner (1100, 342); floor y=199.

use std::{
    fs,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::sleep,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use azalea::{
    account::Account,
    app::{App, Plugin},
    core::tick::GameTick,
    ecs::prelude::*,
    entity::{HasClientLoaded, LocalEntity, Physics, Position},
    interact::BlockStatePredictionHandler,
    local_player::WorldHolder,
    physics::PhysicsSystems,
    registry::builtin::BlockKind,
};
use null_bot_api::{BlockOutcome, BlockPos};

use crate::{
    scenarios::{Ctx, Outcome, pos, scan_log},
    skill::{Info, Op},
};

const AREA: (i32, i32) = (1100, 342);
/// Bot A stands here, facing +z; B stands on the TNT block three blocks ahead.
const A_STAND: (f64, f64, f64) = (1110.5, 200.0, 349.5);
/// The TNT block (hardness 0: broken in one tick by hand) on the stone floor; B stands on it.
const CELL: (i32, i32, i32) = (1110, 200, 352);
const B_STAND: (f64, f64, f64) = (1110.5, 201.0, 352.5);
/// How long the control's second process waits for the stop file, in seconds.
const IDLE_MAX_S: u64 = 240;

/// Names of the watched bots, in order (A first), and whether the probe samples.
static WATCH: Mutex<Vec<String>> = Mutex::new(Vec::new());
static ON: AtomicBool = AtomicBool::new(false);
static TRACE: Mutex<Vec<Sample>> = Mutex::new(Vec::new());

#[derive(Clone)]
struct BotSample {
    name: String,
    cell: String,
    pending: bool,
    seq: u32,
    acked: u32,
    y: f64,
    ground: bool,
    vy: f64,
}

#[derive(Clone)]
struct Sample {
    ms: u128,
    label: &'static str,
    bots: Vec<BotSample>,
}

impl Sample {
    fn line(&self) -> String {
        let bots = self
            .bots
            .iter()
            .map(|b| format!("{}[cell={} pending={} seq={}/{} y={:.4} on_ground={} vy={:.4}]", b.name, b.cell, b.pending, b.seq, b.acked, b.y, b.ground, b.vy))
            .collect::<Vec<_>>()
            .join(" ");
        format!("ms={} {} {}", self.ms, self.label, bots)
    }
}

type Bots<'w, 's> = Query<
    'w,
    's,
    (&'static Account, &'static WorldHolder, &'static BlockStatePredictionHandler, &'static Position, &'static Physics),
    (With<LocalEntity>, With<HasClientLoaded>),
>;

fn now_ms() -> u128 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis())
}

// Spec: specs/plans/SHARED_WORLD_TEST.md (evidence per tick)
fn sample(label: &'static str, bots: &Bots) {
    if !ON.load(Ordering::Relaxed) {
        return;
    }
    let watch = WATCH.lock().unwrap().clone();
    let mut row = Vec::new();
    for name in &watch {
        for (a, w, p, at, ph) in bots {
            if a.username() != name {
                continue;
            }
            let cell = w
                .shared
                .read()
                .get_block_state(azalea::BlockPos::new(CELL.0, CELL.1, CELL.2))
                .map_or("unloaded".to_string(), |s| BlockKind::from(s).to_str().trim_start_matches("minecraft:").to_string());
            row.push(BotSample { name: name.clone(), cell, pending: p.seq() > p.acked_seq(), seq: p.seq(), acked: p.acked_seq(), y: at.y, ground: ph.on_ground(), vy: ph.velocity.y });
        }
    }
    if !row.is_empty() {
        TRACE.lock().unwrap().push(Sample { ms: now_ms(), label, bots: row });
    }
}

// Spec: specs/plans/SHARED_WORLD_TEST.md (evidence per tick)
/// Adds the probe to every bot's app; it samples only while a row turns it on.
pub struct SharedWorldProbe;

impl Plugin for SharedWorldProbe {
    fn build(&self, app: &mut App) {
        // Before the physics step: what B's step is about to read.
        app.add_systems(GameTick, (|q: Bots| sample("pre ", &q)).before(PhysicsSystems).ambiguous_with_all());
        app.add_systems(GameTick, (|q: Bots| sample("post", &q)).after(PhysicsSystems).ambiguous_with_all());
    }
}

fn out_dir() -> PathBuf {
    std::env::var_os("SW_OUT").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join("Desktop/null-spikes/run-logs/shared_world"))
}

fn watch(names: &[&str]) {
    *WATCH.lock().unwrap() = names.iter().map(|s| s.to_string()).collect();
    TRACE.lock().unwrap().clear();
    ON.store(true, Ordering::Relaxed);
    sleep(Duration::from_millis(150));
}

/// Stops the probe, writes the trace to `<out>/<file>` and returns it.
fn unwatch(file: &str) -> Vec<Sample> {
    ON.store(false, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
    let t = std::mem::take(&mut *TRACE.lock().unwrap());
    let _ = fs::create_dir_all(out_dir());
    let _ = fs::write(out_dir().join(file), t.iter().map(|s| s.line() + "\n").collect::<String>());
    t
}

fn log_since(cx: &Ctx, mark: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let mut s = String::new();
    if let Ok(mut f) = fs::File::open(&cx.log) {
        let _ = f.seek(SeekFrom::Start(mark));
        let _ = f.read_to_string(&mut s);
    }
    s
}

/// The server's position of `who` from `data get entity`, or the raw reply.
fn server_pos(cx: &mut Ctx, who: &str) -> Result<[f64; 3], String> {
    let r = cx.r(&format!("data get entity {who} Pos"));
    let inner = r.split_once('[').and_then(|(_, t)| t.split_once(']')).map(|(t, _)| t.to_string()).ok_or_else(|| r.clone())?;
    let v: Vec<f64> = inner.split(',').filter_map(|t| t.trim().trim_end_matches('d').parse().ok()).collect();
    if v.len() == 3 { Ok([v[0], v[1], v[2]]) } else { Err(r) }
}

/// Sets the cell to TNT and stands `who` on it, by RCON (`who` is never op).
fn stand_on_tnt(cx: &mut Ctx, who: &str) {
    cx.r(&format!("setblock {} {} {} minecraft:tnt", CELL.0, CELL.1, CELL.2));
    cx.r(&format!("gamemode survival {who}"));
    cx.r(&format!("effect clear {who}"));
    cx.r(&format!("execute in minecraft:overworld run tp {who} {} {} {} 0 0", B_STAND.0, B_STAND.1, B_STAND.2));
}

/// What the trace says. `a`/`b` index the bots in each sample; `None` when B is in another process.
struct Verdict {
    /// Samples where A's prediction was pending and B's view of the cell was air.
    read_pending: usize,
    first_read: Option<String>,
    /// First sample where B had left its standing height while A's prediction was pending.
    fall_pending: Option<String>,
    min_y: f64,
}

fn verdict(t: &[Sample], a: &str, b: &str) -> Verdict {
    let mut v = Verdict { read_pending: 0, first_read: None, fall_pending: None, min_y: f64::MAX };
    let stand_y = B_STAND.1;
    for s in t {
        let (Some(sa), Some(sb)) = (s.bots.iter().find(|x| x.name == a), s.bots.iter().find(|x| x.name == b)) else { continue };
        v.min_y = v.min_y.min(sb.y);
        if sa.pending && sb.cell == "air" {
            v.read_pending += 1;
            v.first_read.get_or_insert_with(|| s.line());
            if sb.y < stand_y - 0.01 {
                v.fall_pending.get_or_insert_with(|| s.line());
            }
        }
    }
    v
}

fn lines_with(t: &[Sample], f: impl Fn(&Sample) -> bool, n: usize) -> String {
    t.iter().filter(|s| f(s)).take(n).map(|s| s.line()).collect::<Vec<_>>().join(" // ")
}

// Spec: specs/plans/SHARED_WORLD_TEST.md (design 1: break under a bot)
/// SW1, one chance: A breaks the TNT block B stands on; B is the companion of the same swarm.
pub fn sw1(cx: &mut Ctx) -> Vec<Outcome> {
    let Some(b) = cx.companion.clone() else {
        return vec![Outcome { id: "SW1", name: "break_the_block_under_the_companion".into(), pass: false, evidence: "needs --companion (B is the companion of the same swarm)".into(), shown: false, secs: 0.0 }];
    };
    let a = cx.name.clone();
    let m0 = cx.log_len();
    cx.scene_in_spot(AREA, &[], A_STAND, Some((0.0, 3.0)));
    let o = cx.run_allowing("SW1", "break_the_block_under_the_companion", &[""], "the shared-world test (alerts on A or B are the evidence, read below)", |cx| {
        stand_on_tnt(cx, &b);
        sleep(Duration::from_millis(2500));
        let before = server_pos(cx, &b);
        let tnt = cx.block_is(pos(CELL.0, CELL.1, CELL.2), "tnt");
        if !tnt || !before.as_ref().is_ok_and(|p| (p[1] - B_STAND.1).abs() < 0.05) {
            return (false, format!("setup failed before the action: TNT {tnt}, B server pos {before:?}"));
        }
        let setup_b = scan_log(&log_since(cx, m0), &b);
        let mark = cx.log_len();
        watch(&[&a, &b]);
        let act = cx.call(Op::Break(BlockPos::new(CELL.0, CELL.1, CELL.2)));
        sleep(Duration::from_millis(1500));
        let t = unwatch("SW1_trace.log");
        let after = server_pos(cx, &b);
        let cell_air = cx.block_is(pos(CELL.0, CELL.1, CELL.2), "air");
        let text = log_since(cx, mark);
        let (b_warns, b_grim) = scan_log(&text, &b);
        let (a_warns, a_grim) = scan_log(&text.lines().filter(|l| !l.contains(&b)).collect::<Vec<_>>().join("\n"), &a);
        let v = verdict(&t, &a, &b);
        // The bar: the break was done (not refused), the cell is air on the server, and at least one
        // sample shows A pending while B still reads TNT and stands on it (the window that tests the prediction).
        let done = matches!(&act.info, Info::Action(a) if a.outcome == BlockOutcome::Done);
        let b_on_tnt_while_a_pending = t
            .iter()
            .filter(|s| {
                let (Some(sa), Some(sb)) = (s.bots.iter().find(|x| x.name == a), s.bots.iter().find(|x| x.name == b)) else { return false };
                sa.pending && sb.cell == "tnt" && sb.ground && (sb.y - B_STAND.1).abs() < 0.05
            })
            .count();
        let reached = done && cell_air && b_on_tnt_while_a_pending > 0;
        let harm = !b_grim.is_empty() || !b_warns.is_empty();
        let evidence = format!(
            "{}: action {:?}; server cell air {cell_air}, B server pos before {before:?} after {after:?}; samples with A pending, B on TNT: {b_on_tnt_while_a_pending}; cause (B read air while A pending): {} samples, first {:?}; B fell while A pending: {:?}; B min y {:.3}; \
             B setup-window log (before the action): warnings {:?} alerts {:?}; B after the action: warnings {:?} GRIM {:?}; A after the action: warnings {:?} GRIM {:?}; trace file {}",
            if harm { "HARM" } else { "NO HARM" },
            act.info,
            v.read_pending,
            v.first_read,
            v.fall_pending,
            v.min_y,
            setup_b.0,
            setup_b.1,
            b_warns,
            b_grim,
            a_warns,
            a_grim,
            out_dir().join("SW1_trace.log").display()
        );
        // With patch 53 the row passes on no harm (zero Grim alerts and zero warnings on B); an alert fails it.
        (reached && !harm, evidence)
    });
    cx.clean_in(AREA);
    vec![o]
}

// Spec: specs/plans/SHARED_WORLD_TEST.md (the control: separate swarms)
/// SW1C: the SW1 setup with B in a second process (`--account <A>b --scenario SW1B`), so A and B
/// have separate worlds. Start the second process first and wait until B is on the server.
pub fn sw1c(cx: &mut Ctx) -> Vec<Outcome> {
    let a = cx.name.clone();
    let b = format!("{a}b");
    let _ = fs::create_dir_all(out_dir());
    let stop = out_dir().join(format!("stop_{b}"));
    let m0 = cx.log_len();
    cx.scene_in(AREA, &[], A_STAND);
    let o = cx.run_allowing("SW1C", "break_the_block_under_a_bot_of_another_swarm", &[""], "the shared-world control (alerts on A or B are the evidence, read below)", |cx| {
        if cx.poll(30.0, |cx| cx.r("list").contains(&b)).is_none() {
            let _ = fs::write(&stop, "");
            return (false, format!("B ({b}) is not on the server: the second process is not up; the action was not reached"));
        }
        stand_on_tnt(cx, &b);
        sleep(Duration::from_millis(2500));
        let before = server_pos(cx, &b);
        let slime = cx.block_is(pos(CELL.0, CELL.1, CELL.2), "tnt");
        if !slime || !before.as_ref().is_ok_and(|p| (p[1] - B_STAND.1).abs() < 0.05) {
            let _ = fs::write(&stop, "");
            return (false, format!("setup failed before the action: slime {slime}, B server pos {before:?}"));
        }
        let setup_b = scan_log(&log_since(cx, m0), &b);
        let mark = cx.log_len();
        watch(&[&a]);
        let act = cx.call(Op::Break(BlockPos::new(CELL.0, CELL.1, CELL.2)));
        sleep(Duration::from_millis(1500));
        let t = unwatch("SW1C_trace_A.log");
        let after = server_pos(cx, &b);
        let cell_air = cx.block_is(pos(CELL.0, CELL.1, CELL.2), "air");
        let text = log_since(cx, mark);
        let (b_warns, b_grim) = scan_log(&text, &b);
        let (a_warns, a_grim) = scan_log(&text.lines().filter(|l| !l.contains(&b)).collect::<Vec<_>>().join("\n"), &a);
        let _ = fs::write(&stop, "");
        let pend = t.iter().filter(|s| s.bots.iter().any(|x| x.pending)).count();
        let harm = !b_grim.is_empty() || !b_warns.is_empty();
        let evidence = format!(
            "{}: action {:?}; server cell air {cell_air}, B server pos before {before:?} after {after:?}; A pending in {pend} samples (B's own trace is in the second process's file {}); \
             B setup-window log: warnings {:?} alerts {:?}; B after the action: warnings {:?} GRIM {:?}; A after the action: warnings {:?} GRIM {:?}; A trace {}",
            if harm { "HARM" } else { "NO HARM" },
            act.info,
            out_dir().join(format!("{b}_trace.log")).display(),
            setup_b.0,
            setup_b.1,
            b_warns,
            b_grim,
            a_warns,
            a_grim,
            out_dir().join("SW1C_trace_A.log").display()
        );
        (matches!(act.info, Info::Action(_)) && pend > 0, evidence)
    });
    cx.clean_in(AREA);
    vec![o]
}

// Spec: specs/plans/SHARED_WORLD_TEST.md (the control: separate swarms)
/// SW1B: the second process of the control. Its bot stands still while the other process sets the
/// scene and breaks; it logs its own view of the cell until the stop file appears, then writes the
/// trace. It touches no scene and no area.
pub fn sw1b(cx: &mut Ctx) -> Vec<Outcome> {
    let b = cx.name.clone();
    let stop = out_dir().join(format!("stop_{b}"));
    let _ = fs::create_dir_all(out_dir());
    let _ = fs::remove_file(&stop);
    let o = cx.run_allowing("SW1B", "idle_and_trace_as_the_other_swarm", &[""], "the control's second process (alerts are read by the other process)", |_cx| {
        watch(&[&b]);
        let t0 = Instant::now();
        while !stop.exists() && t0.elapsed().as_secs() < IDLE_MAX_S {
            sleep(Duration::from_millis(200));
        }
        let waited = t0.elapsed().as_secs_f64();
        let t = unwatch(&format!("{b}_trace.log"));
        let fell = t.iter().filter(|s| s.bots.iter().any(|x| x.y < B_STAND.1 - 0.01)).count();
        (stop.exists(), format!("traced {} samples in {waited:.0} s; {fell} with y under {}; first fall: {}", t.len(), B_STAND.1, lines_with(&t, |s| s.bots.iter().any(|x| x.y < B_STAND.1 - 0.01), 1)))
    });
    vec![o]
}
