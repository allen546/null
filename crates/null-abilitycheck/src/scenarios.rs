//! The scenarios. RCON sets the scene, the bot acts through `Op`, RCON reads the server's truth.
//! Plan: specs/plans/ABILITY_CHECK.md (A1-A4).

use std::{
    fs,
    io::{Read, Seek, SeekFrom},
    path::PathBuf,
    sync::mpsc::{Receiver, RecvTimeoutError, Sender},
    thread::sleep,
    time::{Duration, Instant},
};

use null_bot_api::{BlockOutcome, BlockPos, Face, Vec3};

use crate::{
    rcon::Rcon,
    skill::{Info, Op, Report, Snap},
};

/// Test area: a stone floor at FLOOR_Y, 21x21, air above; far from spawn and from the oak trees.
const X0: i32 = 470;
const Z0: i32 = 342;
pub(crate) const FLOOR_Y: i32 = 199;
const SPAWN: (f64, f64, f64) = (480.5, 200.0, 352.5);

pub struct Outcome {
    pub id: &'static str,
    pub name: String,
    pub pass: bool,
    pub evidence: String,
    /// Already printed (by `Ctx::run`, as soon as the row finished).
    pub shown: bool,
}

pub struct Ctx {
    pub rc: Rcon,
    pub name: String,
    pub ops: Sender<Op>,
    pub reports: Receiver<Report>,
    pub log: PathBuf,
    /// The host's bot state changes (A12 times deaths with them).
    pub events: Receiver<mc_adapter::HostEvent>,
    /// What the bot's death probe reflex saw (A12).
    pub probe: crate::skill_respawn::SharedProbe,
}

/// Entity-selector bounds for a test area, so cleanup never touches another run's area.
fn in_area((x, z): (i32, i32)) -> String {
    format!("x={x},y={},z={z},dx=21,dy=10,dz=21", FLOOR_Y)
}

pub(crate) fn pos(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

/// Last integer in an RCON reply ("... has the following entity data: 3").
pub(crate) fn last_int(s: &str) -> Option<i64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '-')).filter(|t| !t.is_empty() && *t != "-").last()?.parse().ok()
}

/// Removes ANSI colour codes (ESC [ ... letter) from a log line.
fn strip_ansi(l: &str) -> String {
    let mut out = String::new();
    let mut it = l.chars();
    while let Some(c) = it.next() {
        if c == '\u{1b}' {
            for d in it.by_ref() {
                if d.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Whether `line` names the account `who` as a whole word (`null_blk` is not `null_blk2`).
fn names(line: &str, who: &str) -> bool {
    line.match_indices(who)
        .any(|(i, _)| !line[i + who.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_'))
}

/// Server-log slice to (warning lines, Grim alerts as "Check xN"). Lines naming another `null_`
/// account are skipped, so runs on one server at once don't fail each other.
fn scan_log(s: &str, me: &str) -> (Vec<String>, Vec<String>) {
    let pats = ["moved too quickly", "moved wrongly", "Mismatch", "kicked", "lost connection", "Rejecting"];
    let (mut warns, mut grim): (Vec<String>, Vec<(String, u32)>) = (vec![], vec![]);
    for line in s.lines().map(strip_ansi) {
        if !names(&line, me) && line.contains("null_") {
            continue;
        }
        // Grim: "Grim » <player> failed <Check> (x3) ..."
        if line.contains("Grim") && let Some((_, rest)) = line.split_once(" failed ") {
            let check = rest.split_whitespace().next().unwrap_or("?").to_string();
            match grim.iter_mut().find(|(c, _)| *c == check) {
                Some(e) => e.1 += 1,
                None => grim.push((check, 1)),
            }
        } else if pats.iter().any(|p| line.contains(p)) {
            warns.push(line.trim().to_string());
        }
    }
    (warns, grim.into_iter().map(|(c, n)| format!("{c} x{n}")).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grim_alerts_are_found_through_colour_codes() {
        let log = "[13:08:53 INFO]: \u{1b}[38;5;1mGrim \u{1b}[0m» null_check failed PositionPlace (x1) \n\
[13:09:01 INFO]: Grim » null_check failed PositionPlace (x1)\n[13:09:01 INFO]: x moved too quickly! 1,2,3\n[13:09:02 INFO]: ok\n";
        let (warns, grim) = scan_log(log, "null_check");
        assert_eq!(grim, vec!["PositionPlace x2"]);
        assert_eq!(warns.len(), 1);
    }

    #[test]
    fn an_account_that_extends_ours_is_another_account() {
        let log = "Grim » null_blk2 failed Simulation (x1)\nGrim » null_blk failed Reach (x1)\n";
        let (_, grim) = scan_log(log, "null_blk");
        assert_eq!(grim, vec!["Reach x1"]);
    }
}

impl Ctx {
    pub(crate) fn r(&mut self, c: &str) -> String {
        self.rc.cmd(c).unwrap_or_else(|e| panic!("rcon {c:?}: {e}"))
    }

    /// Sends an op and waits for its final report.
    pub(crate) fn call(&mut self, op: Op) -> Report {
        self.ops.send(op).unwrap();
        loop {
            match self.reports.recv_timeout(Duration::from_secs(60)) {
                Ok(r) if !matches!(r.info, Info::Started) => return r,
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => panic!("bot did not answer in 60 s"),
                Err(e) => panic!("bot gone: {e}"),
            }
        }
    }

    pub(crate) fn snap(&mut self) -> Snap {
        self.snap_after(Op::Snapshot)
    }

    pub(crate) fn snap_after(&mut self, op: Op) -> Snap {
        match self.call(op).info {
            Info::Snapshot(s) => s,
            i => panic!("expected a snapshot: {i:?}"),
        }
    }

    pub(crate) fn wait_block(&mut self, p: BlockPos, name: &'static str) {
        if !matches!(self.call(Op::WaitBlock { pos: p, name }).info, Info::Waited(true)) {
            eprintln!("warning: the bot never saw {name} at {p:?}");
        }
    }

    pub(crate) fn block_is(&mut self, p: BlockPos, name: &str) -> bool {
        self.r(&format!("execute if block {} {} {} minecraft:{name}", p.x, p.y, p.z)).contains("passed")
    }

    /// `clear <bot> <item> 0` counts without removing.
    pub(crate) fn count(&mut self, item: &str) -> u32 {
        let n = self.name.clone();
        // "Found 3 matching item(s) on player X" or "No items were found ...".
        let r = self.r(&format!("clear {n} minecraft:{item} 0"));
        r.strip_prefix("Found ").and_then(|s| s.split(' ').next()?.parse().ok()).unwrap_or(0)
    }

    /// Polls an RCON condition for up to `secs`; returns the seconds it took.
    pub(crate) fn poll(&mut self, secs: f64, mut f: impl FnMut(&mut Self) -> bool) -> Option<f64> {
        let t = Instant::now();
        while t.elapsed().as_secs_f64() < secs {
            if f(self) {
                return Some(t.elapsed().as_secs_f64());
            }
            sleep(Duration::from_millis(20));
        }
        None
    }

    /// Fresh scene: floor, air, no items or armor stands, bot at the spawn point with `hotbar` items.
    pub fn scene(&mut self, hotbar: &[(u8, &str, u8)]) {
        self.scene_at(hotbar, SPAWN);
    }

    /// As `scene`, with the bot standing at `at`.
    pub fn scene_at(&mut self, hotbar: &[(u8, &str, u8)], at: (f64, f64, f64)) {
        self.scene_in((X0, Z0), hotbar, at);
    }

    /// As `scene_at`, in the 21x21 area whose corner is `area` (each scenario module has its own).
    pub(crate) fn scene_in(&mut self, area: (i32, i32), hotbar: &[(u8, &str, u8)], at: (f64, f64, f64)) {
        let n = self.name.clone();
        let (x0, z0) = area;
        let (x1, z1) = (x0 + 20, z0 + 20);
        for c in [
            format!("forceload add {x0} {z0} {x1} {z1}"),
            format!("fill {x0} {FLOOR_Y} {z0} {x1} {FLOOR_Y} {z1} minecraft:stone"),
            format!("fill {x0} {} {z0} {x1} {} {z1} minecraft:air", FLOOR_Y + 1, FLOOR_Y + 8),
            format!("kill @e[type=minecraft:item,{}]", in_area(area)),
            format!("kill @e[type=minecraft:armor_stand,{}]", in_area(area)),
            format!("gamemode survival {n}"),
            format!("clear {n}"),
            format!("tp {n} {} {} {} 0 0", at.0, at.1, at.2),
        ] {
            self.r(&c);
        }
        for (slot, item, count) in hotbar {
            self.r(&format!("item replace entity {n} hotbar.{slot} with minecraft:{item} {count}"));
        }
        sleep(Duration::from_millis(1500));
        self.wait_block(pos(x0 + 10, FLOOR_Y, z0 + 10), "minecraft:stone");
        self.wait_block(pos(x0 + 10, FLOOR_Y + 1, z0 + 10), "minecraft:air");
        let here = self.snap_after(Op::SelectSlot(0)).pos;
        if (here.x - at.0).abs() > 0.5 || (here.y - at.1).abs() > 0.5 || (here.z - at.2).abs() > 0.5 {
            eprintln!("warning: bot is at {here:?}, not at {at:?}");
        }
    }

    pub fn clean(&mut self) {
        self.clean_in((X0, Z0));
    }

    pub(crate) fn clean_in(&mut self, area: (i32, i32)) {
        let (x0, z0) = area;
        let (x1, z1) = (x0 + 20, z0 + 20);
        for c in [
            format!("kill @e[type=minecraft:item,{}]", in_area(area)),
            format!("kill @e[type=minecraft:armor_stand,{}]", in_area(area)),
            format!("fill {x0} {FLOOR_Y} {z0} {x1} {} {z1} minecraft:air", FLOOR_Y + 8),
            format!("forceload remove {x0} {z0} {x1} {z1}"),
        ] {
            self.r(&c);
        }
    }

    pub(crate) fn log_len(&self) -> u64 {
        fs::metadata(&self.log).map_or(0, |m| m.len())
    }

    /// Warning lines the server wrote since `mark` (bar: no anticheat or movement warnings), and
    /// the Grim checks that alerted, with counts.
    pub(crate) fn log_problems(&self, mark: u64) -> (Vec<String>, Vec<String>) {
        let mut s = String::new();
        if let Ok(mut f) = fs::File::open(&self.log) {
            let _ = f.seek(SeekFrom::Start(mark));
            let _ = f.read_to_string(&mut s);
        }
        scan_log(&s, &self.name)
    }

    /// Runs a scenario body and appends the log verdict.
    pub(crate) fn run(&mut self, id: &'static str, name: &str, body: impl FnOnce(&mut Self) -> (bool, String)) -> Outcome {
        self.run_allowing(id, name, &[], body)
    }

    /// As `run`, but Grim alerts of the checks whose name starts with one of `allowed` are listed
    /// in the evidence and do not fail the row (AT: a forced stall draws `Timer`, AD-F5.3).
    pub(crate) fn run_allowing(&mut self, id: &'static str, name: &str, allowed: &[&str], body: impl FnOnce(&mut Self) -> (bool, String)) -> Outcome {
        let mark = self.log_len();
        sleep(Duration::from_millis(100));
        crate::skill::TICK_GAP_MAX_MS.store(0, std::sync::atomic::Ordering::Relaxed);
        crate::skill::TICK_FAST.store(0, std::sync::atomic::Ordering::Relaxed);
        *crate::skill::REFUSAL_VIEW.lock().unwrap() = None;
        let (mut pass, mut evidence) = body(self);
        // Only on a failed row: refusal rows that expect a refusal stay short.
        if !pass && let Some(v) = crate::skill::REFUSAL_VIEW.lock().unwrap().take() {
            evidence += &format!("; {v}");
        }
        sleep(Duration::from_millis(300));
        let (bad, grim) = self.log_problems(mark);
        let (expected, grim): (Vec<String>, Vec<String>) = grim.into_iter().partition(|g| allowed.iter().any(|a| g.starts_with(a)));
        if !expected.is_empty() {
            evidence += &format!("; expected Grim alerts from the forced stall (AD-F5.3, vanilla would draw them too): {}", expected.join(", "));
        }
        if bad.is_empty() && grim.is_empty() {
            evidence += if expected.is_empty() { "; server log: no warnings, no Grim alerts" } else { "; server log: no warnings, no other Grim alerts" };
        } else {
            pass = false;
            if !bad.is_empty() {
                evidence += &format!("; server log: {} warning(s): {}", bad.len(), bad.join(" | "));
            }
            if !grim.is_empty() {
                evidence += &format!("; GRIM ALERTS: {}", grim.join(", "));
            }
        }
        let (gap, fast) = (crate::skill::TICK_GAP_MAX_MS.load(std::sync::atomic::Ordering::Relaxed), crate::skill::TICK_FAST.load(std::sync::atomic::Ordering::Relaxed));
        if !grim.is_empty() || !expected.is_empty() || gap > 150 {
            evidence += &format!("; client ticks: longest gap {gap} ms, {fast} under 25 ms (catch-up)");
        }
        let o = Outcome { id, name: name.into(), pass, evidence, shown: true };
        println!("{}", report_line(&o));
        o
    }
}

// ---- A1 ------------------------------------------------------------------------------------

// Spec: specs/B2-survival-upkeep.md §4.4, specs/B3-inventory-crafting.md §4.1 (select a slot; offhand swap)
pub fn a1(cx: &mut Ctx) -> Vec<Outcome> {
    let n = cx.name.clone();
    cx.scene(&[(3, "diamond_sword", 1), (8, "apple", 4)]);
    let mut out = Vec::new();
    out.push(cx.run("A1", "select_slot", |cx| {
        let mut ev = Vec::new();
        let mut ok = true;
        for (slot, item) in [(3u8, "diamond_sword"), (8, "apple"), (0, "")] {
            let view_slot = cx.snap_after(Op::SelectSlot(slot)).selected;
            let mut server = None;
            let got = cx.poll(1.0, |cx| {
                server = last_int(&cx.r(&format!("data get entity {n} SelectedItemSlot")));
                server == Some(slot as i64)
            });
            let held = if item.is_empty() {
                cx.r(&format!("execute unless items entity {n} weapon.mainhand *")).contains("passed")
            } else {
                cx.r(&format!("execute if items entity {n} weapon.mainhand minecraft:{item}")).contains("passed")
            };
            ok &= got.is_some() && held;
            ev.push(format!("slot {slot}: bot view {view_slot}, server SelectedItemSlot {server:?} ({} ms), mainhand {}", got.map_or(-1, |s| (s * 1000.0) as i64), if item.is_empty() { "empty" } else { item }));
        }
        (ok, ev.join("; "))
    }));
    cx.scene(&[(3, "diamond_sword", 1)]);
    out.push(cx.run("A1", "swap_offhand", |cx| {
        cx.call(Op::SelectSlot(3));
        cx.call(Op::SwapOffhand);
        let off = cx.poll(1.0, |cx| cx.r(&format!("execute if items entity {n} weapon.offhand minecraft:diamond_sword")).contains("passed"));
        let main_empty = cx.r(&format!("execute unless items entity {n} weapon.mainhand *")).contains("passed");
        let view = cx.snap().offhand.map(|s| s.item);
        cx.call(Op::SwapOffhand);
        let back = cx.poll(1.0, |cx| cx.r(&format!("execute if items entity {n} weapon.mainhand minecraft:diamond_sword")).contains("passed"));
        let off_empty = cx.r(&format!("execute unless items entity {n} weapon.offhand *")).contains("passed");
        (
            off.is_some() && main_empty && back.is_some() && off_empty,
            format!(
                "after swap: server offhand has diamond_sword={} ({:?} s), mainhand empty={main_empty}, bot view offhand={view:?}; after second swap: mainhand has it={}, offhand empty={off_empty}",
                off.is_some(), off, back.is_some()
            ),
        )
    }));
    out
}

// ---- A2 ------------------------------------------------------------------------------------

/// 30 oak logs broken back to back with an iron axe, no Haste: each break must take vanilla's
/// ticks, and the gaps must satisfy Grim's FastBreak (it flagged after ~22 when the bot started
/// each break like a fresh click, one tick early and with no destroy delay).
// Spec: specs/B6-placement-breaking.md §4.4 (break timing over a chain of breaks)
fn chain_of_breaks(cx: &mut Ctx) -> Outcome {
    cx.scene(&[(0, "iron_axe", 1)]);
    // Two walls of 5 x 3 logs, 2 blocks in front of and behind the bot (it stands at 480, 200, 352).
    let mut list = Vec::new();
    for z in [354, 350] {
        for y in 200..203 {
            for x in 478..483 {
                list.push(pos(x, y, z));
            }
        }
    }
    cx.r("fill 478 200 354 482 202 354 minecraft:oak_log");
    cx.r("fill 478 200 350 482 202 350 minecraft:oak_log");
    cx.wait_block(pos(482, 202, 350), "minecraft:oak_log");
    let n = vanilla_ticks(2.0, 6.0, true);
    let want = ticks_between_packets(n) as u64;
    cx.run("A2", "chain_of_30_breaks", |cx| {
        let rep = cx.call(Op::BreakChain(list.clone()));
        let Info::Chain(log) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        sleep(Duration::from_millis(500));
        let air = list.iter().filter(|p| cx.block_is(**p, "air")).count();
        let bad: Vec<_> = log.iter().filter(|(_, t, o)| *t != Some(want) || *o != BlockOutcome::Done).collect();
        let unlogged: Vec<_> = list.iter().filter(|p| !log.iter().any(|(q, _, _)| q == *p)).collect();
        (
            air == list.len() && log.len() == list.len() && bad.is_empty(),
            format!("{} of {} logs air on the server; {} breaks logged, each want {want} ticks (vanilla {n}) and Done; off: {bad:?}; no result logged for: {unlogged:?}", air, list.len(), log.len()),
        )
    })
}

/// Vanilla break ticks: progress per tick is speed / hardness / (30 if the tool counts, else 100),
/// summed in f32 until it reaches 1. Held-button mining adds nothing in the start tick, so n ticks
/// pass between the start and finish packets (Grim's FastBreak predicts the same); an instant
/// break (n = 1) finishes on the start packet.
pub(crate) fn ticks_between_packets(n: u32) -> u32 {
    if n <= 1 { 0 } else { n }
}

// Spec: specs/B6-placement-breaking.md §4.4 (break time from hardness and tool)
pub(crate) fn vanilla_ticks(hardness: f32, speed: f32, harvest: bool) -> u32 {
    let delta = speed / hardness / if harvest { 30.0 } else { 100.0 };
    let (mut p, mut n) = (0.0f32, 0);
    while p < 1.0 {
        p += delta;
        n += 1;
    }
    n
}

/// (name, block, tool, hardness, tool speed, drops need the tool and it is right)
/// Hardness: oak_log 2.0, stone 1.5. Speed: hand 1, iron axe/pickaxe 6 (iron tier).
const BREAKS: [(&str, &str, Option<&str>, f32, f32); 3] = [
    ("oak_log_by_hand", "oak_log", None, 2.0, 1.0),
    ("oak_log_iron_axe", "oak_log", Some("iron_axe"), 2.0, 6.0),
    ("stone_iron_pickaxe", "stone", Some("iron_pickaxe"), 1.5, 6.0),
];

// Spec: specs/B6-placement-breaking.md §4.4 (break at vanilla speed; measured on the server)
pub fn a2(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    for (name, block, tool, hardness, speed) in BREAKS {
        cx.scene(&tool.map(|t| vec![(0, t, 1)]).unwrap_or_default());
        let target = pos(480, FLOOR_Y + 1, 355);
        // Stone is both the floor and the target; the target sits on the floor.
        cx.r(&format!("setblock {} {} {} minecraft:{block}", target.x, target.y, target.z));
        cx.wait_block(target, if block == "oak_log" { "minecraft:oak_log" } else { "minecraft:stone" });
        out.push(cx.run("A2", name, |cx| {
            let n = vanilla_ticks(hardness, speed, true);
            let air = format!("execute if block {} {} {} minecraft:air run time query gametime", target.x, target.y, target.z);
            cx.ops.send(Op::Break(target)).unwrap();
            // The bot sends Started the tick it issues the break; sample the server clock at once.
            loop {
                match cx.reports.recv_timeout(Duration::from_secs(30)) {
                    Ok(r) if matches!(r.info, Info::Started) => break,
                    Ok(_) => {}
                    Err(e) => return (false, format!("bot never started the break: {e}")),
                }
            };
            let g0 = last_int(&cx.r("time query gametime")).unwrap_or(-1);
            // Tight poll: one RCON command checks the block and reads the clock together.
            let t = Instant::now();
            let mut g1 = None;
            while t.elapsed() < Duration::from_secs(20) && g1.is_none() {
                let reply = cx.r(&air);
                if reply.contains("time is") {
                    g1 = last_int(&reply);
                }
            }
            let fin = loop {
                match cx.reports.recv_timeout(Duration::from_secs(30)) {
                    Ok(r) if !matches!(r.info, Info::Started) => break r,
                    Ok(_) => {}
                    Err(e) => return (false, format!("bot never finished the break: {e}")),
                }
            };
            let Info::Action(a) = fin.info else { return (false, format!("bot reported {:?}", fin.info)) };
            let bot_ticks = a.finish_tick.map(|f| f - a.start_tick);
            let server = g1.map(|g| g - g0);
            let air_now = cx.block_is(target, "air");
            let want = ticks_between_packets(n) as i64;
            let ok = air_now
                && a.outcome == BlockOutcome::Done
                && bot_ticks == Some(want as u64)
                && server.is_some_and(|d| (want - 1..=want + 2).contains(&d));
            (
                ok,
                format!(
                    "{block} with {}: vanilla {n} mining ticks = {want} ticks between start and finish packets; bot {bot_ticks:?} ticks (outcome {:?}, ack read at tick {:?}); server gametime start {g0} -> air seen {g1:?} = {server:?} ticks; block is air on server: {air_now}. Precision: bot ticks are exact (pass needs equality); the server figure is two RCON clock reads, the start read can land a tick before the start packet is handled and the finish read a tick after the stop packet, so it reads want-1..want+2 for a vanilla-speed break (pass window) and catches breaks 2+ ticks too fast or too slow",
                    tool.unwrap_or("hand"),
                    a.outcome,
                    a.ack_tick
                ),
            )
        }));
    }
    // Refusal: the log is 9 blocks away; the server must refuse and the bot must say so.
    cx.scene(&[(0, "iron_axe", 1)]);
    let far = pos(480, FLOOR_Y + 1, 361);
    cx.r("setblock 480 200 361 minecraft:oak_log");
    cx.wait_block(far, "minecraft:oak_log");
    out.push(cx.run("A2", "refused_by_client_break_out_of_reach", |cx| {
        let r = cx.call(Op::Break(far));
        let Info::Action(a) = r.info else { return (false, format!("bot reported {:?}", r.info)) };
        let still = cx.block_is(far, "oak_log");
        (
            a.outcome == BlockOutcome::Refused && a.reason == Some("out of reach") && a.ack_tick.is_none() && still,
            format!("client refusal, nothing sent: bot outcome {:?} reason {:?} (want Refused, \"out of reach\"), ack read at {:?}; oak_log still on the server: {still}", a.outcome, a.reason, a.ack_tick),
        )
    }));
    out.push(chain_of_breaks(cx));
    // Cancel: abort a long break half-way; the block must stay and the bot must say cancelled.
    cx.scene(&[]);
    let target = pos(480, FLOOR_Y + 1, 355);
    cx.r("setblock 480 200 355 minecraft:oak_log");
    cx.wait_block(target, "minecraft:oak_log");
    out.push(cx.run("A2", "cancel_break", |cx| {
        let r = cx.call(Op::BreakThenCancel(target, 20));
        sleep(Duration::from_millis(500));
        let still = cx.block_is(target, "oak_log");
        let Info::Action(a) = r.info else { return (false, format!("bot reported {:?}", r.info)) };
        (
            a.outcome == BlockOutcome::Cancelled && still,
            format!("cancelled after 20 ticks of a 60-tick break: bot outcome {:?}; oak_log still on the server: {still}", a.outcome),
        )
    }));
    out
}

// ---- A3 ------------------------------------------------------------------------------------

// Spec: specs/B6-placement-breaking.md §4.5 (drop collection)
pub fn a3(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene(&[(0, "iron_axe", 1)]);
    let log = pos(480, FLOOR_Y + 1, 355);
    cx.r("setblock 480 200 355 minecraft:oak_log");
    cx.wait_block(log, "minecraft:oak_log");
    vec![cx.run("A3", "pickup_drop", |cx| {
        let before = cx.count("oak_log");
        let b = cx.call(Op::Break(log));
        let Info::Action(a) = b.info else { return (false, format!("break reported {:?}", b.info)) };
        let on_floor = cx.r("execute if entity @e[type=minecraft:item,nbt={Item:{id:\"minecraft:oak_log\"}}]").contains("passed");
        let t = Instant::now();
        cx.call(Op::Goto(log));
        let got = cx.poll(5.0, |cx| cx.count("oak_log") > before);
        let after = cx.count("oak_log");
        let walk_s = t.elapsed().as_secs_f64();
        sleep(Duration::from_millis(300));
        let view = cx.snap().counts.get("minecraft:oak_log").copied().unwrap_or(0);
        (
            a.outcome == BlockOutcome::Done && got.is_some() && after == before + 1 && view == after,
            format!(
                "break outcome {:?}; drop entity on server before pickup: {on_floor}; server oak_log count {before} -> {after} (after {walk_s:.2} s of walking onto it); bot's own inventory view says {view}",
                a.outcome
            ),
        )
    })]
}

// ---- A4 ------------------------------------------------------------------------------------

struct Place {
    name: &'static str,
    item: &'static str,
    floor_block: &'static str,
    /// Clicked block and face; the new block goes to click.offset(face).
    click: (i32, i32, i32),
    face: Face,
    /// Where the bot stands (it must see the face), and where on the face it aims, from the centre.
    at: (f64, f64, f64),
    aim: (f64, f64, f64),
    summon_stand: bool,
    side_pillar: bool,
    expect_placed: bool,
    /// The client must refuse by itself with this reason and send nothing.
    client_reason: Option<&'static str>,
}

const P: Place = Place {
    name: "",
    item: "stone",
    floor_block: "stone",
    click: (480, FLOOR_Y, 355),
    face: Face::Up,
    at: SPAWN,
    aim: (0.0, 0.0, 0.0),
    summon_stand: false,
    side_pillar: false,
    expect_placed: true,
    client_reason: None,
};

const PLACES: [Place; 6] = [
    Place { name: "place_top_face", ..P },
    // East face of the pillar: the bot stands east of it, where that face is visible.
    Place { name: "place_side_face", click: (481, FLOOR_Y + 1, 355), face: Face::East, at: (484.5, 200.0, 355.5), side_pillar: true, ..P },
    Place { name: "sapling_on_grass", item: "oak_sapling", floor_block: "grass_block", ..P },
    // Legal click, refused by the server. The armor stand stands mid-cell, so the bot aims at the
    // face corner to keep the ray off the stand (a vanilla click would hit the stand otherwise).
    Place { name: "refused_by_server_entity_in_the_way", summon_stand: true, aim: (-0.45, 0.0, 0.45), expect_placed: false, ..P },
    // Illegal click (9 blocks away): the client refuses and sends nothing.
    Place { name: "refused_by_client_out_of_reach", click: (480, FLOOR_Y, 361), expect_placed: false, client_reason: Some("out of reach"), ..P },
    Place { name: "refused_by_server_no_support_sapling_on_stone", item: "oak_sapling", expect_placed: false, ..P },
];

// Spec: specs/B6-placement-breaking.md §4.2, §4.6 (place on a chosen face; read the ack)
pub fn a4(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    for p in PLACES {
        cx.scene_at(&[(0, p.item, 8)], p.at);
        let (cx_, cy, cz) = p.click;
        let clicked = pos(cx_, cy, cz);
        let target = clicked.offset(p.face);
        if p.floor_block != "stone" {
            cx.r(&format!("setblock {cx_} {cy} {cz} minecraft:{}", p.floor_block));
        }
        if p.side_pillar {
            cx.r(&format!("setblock {cx_} {cy} {cz} minecraft:stone"));
            cx.wait_block(clicked, "minecraft:stone");
        }
        if p.summon_stand {
            cx.r(&format!("summon minecraft:armor_stand {}.5 {} {}.5 {{NoGravity:1b}}", target.x, target.y, target.z));
        }
        cx.wait_block(target, "minecraft:air");
        out.push(cx.run("A4", p.name, |cx| {
            let before = cx.count(p.item);
            let r = cx.call(Op::Place { pos: clicked, face: p.face, aim: Vec3::new(p.aim.0, p.aim.1, p.aim.2) });
            let Info::Action(a) = r.info else { return (false, format!("bot reported {:?}", r.info)) };
            sleep(Duration::from_millis(200));
            let after = cx.count(p.item);
            let view = match cx.call(Op::Peek(target)).info {
                Info::Block(b) => b,
                _ => None,
            };
            let target_name = if p.item == "stone" { "stone" } else { "oak_sapling" };
            let placed = cx.block_is(target, target_name);
            let air = cx.block_is(target, "air");
            let want = if p.expect_placed { BlockOutcome::Done } else { BlockOutcome::Refused };
            let ticks = a.ack_tick.map(|t| t as i64 - a.start_tick as i64);
            (
                a.outcome == want
                    && placed == p.expect_placed
                    && (after == before - u32::from(p.expect_placed))
                    && (p.expect_placed || air)
                    && a.reason == p.client_reason
                    // a client refusal means nothing was sent, so there is no ack to read
                    && (p.client_reason.is_none() || a.ack_tick.is_none()),
                format!(
                    "{:?} face of {clicked:?}; bot outcome {:?} (want {want:?}, client reason {:?}), ack read {ticks:?} ticks after the click; server: {target_name} at {target:?} = {placed}, air = {air}; bot's view of target {view:?}; {} count {before} -> {after}",
                    p.face, a.outcome, a.reason, p.item
                ),
            )
        }));
    }
    out
}

pub fn report_line(o: &Outcome) -> String {
    format!("{} {} {} — {}", if o.pass { "PASS" } else { "FAIL" }, o.id, o.name, o.evidence)
}
