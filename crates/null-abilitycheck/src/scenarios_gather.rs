//! Milestone S5, A14: `gather` with the tree policy (`fell`, B4 §3, §4.3, §4.8, §4.10) on the live
//! bot. RCON builds every tree and hut from explicit block states, so the shape is known; the bot
//! fells through `null_skills::fell`; RCON reads the server's truth back (log cells, the log count
//! in the inventory, the hut intact) and the bot's belief is read at the felled cells. The rows
//! also check what the live view gives the skill: `is_solid` and `persistent` for logs and
//! leaves, item entities listed as `minecraft:item`, the step cost of planning, and a stand on
//! log 1 (the pathfinder's one-block step). Plan: specs/plans/BUILD_PLAN.md S5.

use std::{thread::sleep, time::Duration};

use null_bot_api::BlockPos;
use null_skills::{Claim, Reason};

use crate::{
    scenarios::{Ctx, Outcome, pos},
    skill::{Info, Op, Report},
    skill_gather::{GatherInfo, GatherOp},
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (800, 342);
const STAND: (f64, f64, f64) = (804.5, 200.0, 352.5);
const FLOOR_Y: i32 = crate::scenarios::FLOOR_Y;
/// Highest y a row builds to (a 9-log trunk and its canopy); `scene_in` clears only 8 rows.
const TOP_Y: i32 = 215;
/// Every row clears up to here, which covers the hut's copy 20 blocks above it.
const CLEAR_Y: i32 = FLOOR_Y + 30;
const OAK_X: i32 = 810;
const OAK_Z: i32 = 352;

fn claim(max_x: i32) -> Claim {
    Claim { min: pos(AREA.0, FLOOR_Y + 1, AREA.1), max: pos(max_x, TOP_Y, AREA.1 + 20) }
}

/// A job can run for minutes (by hand a log takes 10 s), so wait longer than `Ctx::call`.
fn call_long(cx: &mut Ctx, op: Op) -> Report {
    let seq = cx.send(op);
    cx.finish_op(seq, Duration::from_secs(1200)).unwrap_or_else(|e| panic!("bot did not finish the job in 20 min: {e}"))
}

/// Fresh scene with the full height cleared, and the bot at `STAND`.
fn reset(cx: &mut Ctx, hotbar: &[(u8, &str, u8)]) {
    cx.scene_in(AREA, hotbar, STAND);
    let (x1, z1) = (AREA.0 + 20, AREA.1 + 20);
    cx.r(&format!("fill {} {} {} {x1} {CLEAR_Y} {z1} minecraft:air", AREA.0, FLOOR_Y + 1, AREA.1));
}

fn fill(cx: &mut Ctx, a: (i32, i32, i32), b: (i32, i32, i32), block: &str) {
    cx.r(&format!("fill {} {} {} {} {} {} {block}", a.0, a.1, a.2, b.0, b.1, b.2));
}

/// A natural tree of `kind` ("oak"): `h` logs from y = floor + 1, a canopy of leaves with
/// `persistent=false` around the top (they count as natural, B4 §4.8). Returns the trunk cells,
/// bottom first. The leaves go in first and the trunk over them.
fn tree(cx: &mut Ctx, kind: &str, x: i32, z: i32, h: i32) -> Vec<BlockPos> {
    let base = FLOOR_Y + 1;
    let top = base + h - 1;
    let leaves = format!("minecraft:{kind}_leaves[persistent=false,distance=1]");
    fill(cx, (x - 2, top - 1, z - 2), (x + 2, top, z + 2), &leaves);
    fill(cx, (x - 1, top + 1, z - 1), (x + 1, top + 2, z + 1), &leaves);
    fill(cx, (x, base, z), (x, top, z), &format!("minecraft:{kind}_log"));
    (base..=top).map(|y| pos(x, y, z)).collect()
}

/// Whether every cell reads as `name` on the server.
fn all_are(cx: &mut Ctx, cells: &[BlockPos], name: &str) -> bool {
    cells.iter().all(|p| cx.block_is(*p, name))
}

/// Belief at the cells: air, seen by this bot, changed from `from`.
fn belief_air(info: &GatherInfo, who: &str, from: &str) -> bool {
    info.belief.iter().all(|b| b.block.as_deref() == Some("minecraft:air") && b.observer.as_deref() == Some(who) && b.changed_from.as_deref() == Some(from))
}

fn gather_info(rep: Report) -> Result<GatherInfo, String> {
    match rep.info {
        Info::Gather(g) => Ok(*g),
        i => Err(format!("bot reported {i:?}")),
    }
}

fn times_line(g: &GatherInfo) -> String {
    let t = &g.times;
    format!("{} steps, p50 {} us, p99 {} us, max {} us (at op tick {}), {} over 2 ms", t.steps, t.p50, t.p99, t.max, t.max_at_age, t.over_2ms)
}

/// Where the server holds the item entities still on the ground near the trees, and the bot, for a
/// short row: a drop that stays needs its server position next to the bot's (the view lags it).
// Spec: specs/B4-gathering.md §4.9 (drops: walk to item entities near what was broken)
fn server_drops(cx: &mut Ctx) -> String {
    let n = cx.name.clone();
    let tail = |s: String| s.trim().rsplit(':').next().unwrap_or("").trim().to_string();
    let bot = tail(cx.r(&format!("data get entity {n} Pos")));
    let mut seen = Vec::new();
    for _ in 0..8 {
        let sel = format!("@e[type=minecraft:item,tag=!a14seen,x={OAK_X},y={},z={OAK_Z},distance=..14,limit=1,sort=nearest]", FLOOR_Y + 2);
        let at = cx.r(&format!("data get entity {sel} Pos"));
        if at.contains("No entity") {
            break;
        }
        // Paper cuts a long `Item` reply short, so read the two fields apart.
        let id = entity_data(&cx.r(&format!("data get entity {sel} Item.id")));
        let count = tail(cx.r(&format!("data get entity {sel} Item.count")));
        let delay = cx.r(&format!("data get entity {sel} PickupDelay"));
        cx.r(&format!("tag {sel} add a14seen"));
        seen.push(format!("{} {id} x{count} delay {}", tail(at), tail(delay)));
    }
    format!("server: bot at {bot}, items on the ground {seen:?}")
}

/// One item entity the server holds near the tree: its item, count, and whether the block under it
/// is a leaf block (the drop rests on the canopy).
#[derive(Debug, Clone, PartialEq)]
struct ServerDrop {
    id: String,
    count: u32,
    on_leaves: bool,
}

/// Whether a row that ended short of `want` logs may pass: `gather` reports a drop that rests on
/// leaves out of reach as left (it does not chase or clear for it; the leaves decay, then a later
/// pass picks it up). Accepted only with server proof: every drop on the server is this log, rests
/// on a leaf block, and the inventory plus those drops adds up to `want`.
// Spec: specs/B4-gathering.md §4.9 (a drop on leaves out of reach is reported as left, Allen 2026-10-24)
fn left_on_leaves_ok(want: u32, inv: u32, log_id: &str, drops: &[ServerDrop]) -> bool {
    !drops.is_empty() && drops.iter().all(|d| d.id == log_id && d.on_leaves) && inv + drops.iter().map(|d| d.count).sum::<u32>() == want
}

/// The value in a `data get entity` reply, quotes removed. Split on the reply's own words, not on
/// the last colon: an id such as `minecraft:birch_log` holds one.
fn entity_data(reply: &str) -> String {
    reply.trim().split(" has the following entity data: ").last().unwrap_or("").trim().trim_matches('"').to_string()
}

/// The item entities of `log_id` the server holds near the tree, read field by field (Paper cuts a
/// long `Item` reply short), each with a leaf test on the block under it.
// Spec: specs/B4-gathering.md §4.9 (server proof for a drop left on leaves)
fn server_log_drops(cx: &mut Ctx) -> Vec<ServerDrop> {
    let tail = |s: String| entity_data(&s);
    let mut out = Vec::new();
    for _ in 0..16 {
        let sel = format!("@e[type=minecraft:item,tag=!a14left,x={OAK_X},y={},z={OAK_Z},distance=..14,limit=1,sort=nearest]", FLOOR_Y + 2);
        if cx.r(&format!("data get entity {sel} Pos")).contains("No entity") {
            break;
        }
        let id = tail(cx.r(&format!("data get entity {sel} Item.id")));
        let count = tail(cx.r(&format!("data get entity {sel} Item.count"))).trim_end_matches(|c: char| !c.is_ascii_digit()).parse().unwrap_or(0);
        // The item rests on the block's top face, so look 0.2 below its feet.
        let on_leaves = cx.r(&format!("execute as {sel} at @s if block ~ ~-0.2 ~ #minecraft:leaves")).contains("passed");
        cx.r(&format!("tag {sel} add a14left"));
        out.push(ServerDrop { id, count, on_leaves });
    }
    out
}

/// Whether the log count is right: all `want` in the inventory, or (proven on the server) the
/// rest left on leaves and reported in `drops_left`. Returns the verdict and a detail note.
// Spec: specs/B4-gathering.md §4.9
fn logs_accounted(cx: &mut Ctx, want: u32, got: u32, log: &str, view_left: usize) -> (bool, String) {
    if got == want {
        return (true, String::new());
    }
    let drops = server_log_drops(cx);
    let ok = view_left == drops.len() && left_on_leaves_ok(want, got, &format!("minecraft:{log}"), &drops);
    (ok, if ok { format!("; PASSED WITH A DROP LEFT ON LEAVES (inventory {got}, server drops {drops:?})") } else { format!("; short of {want}, server drops {drops:?}") })
}

fn result_line(g: &GatherInfo) -> String {
    let r = &g.result;
    let leaves = r.changed.len() as u32 - r.gathered;
    format!(
        "status {}, gathered {}, yield {:?}, {} leaves cleared, left {:?}, {} ticks, drops left {}, bot reached y {:.2}, entity kinds seen {:?} (most items at once {}); step time: {}",
        g.status, r.gathered, r.yield_items, leaves, r.left, r.ticks, r.drops_left.len(), g.max_y, g.kinds, g.max_items, times_line(g)
    )
}

/// Fell one tree of `h` logs with `quota`; a trunk that needs more than a stand on log 1 leaves
/// its top log (8 are reachable, see the tall row).
// Spec: specs/B4-gathering.md §4.3, §4.10 (fell a natural tree completely and pick up the logs)
fn fell_tree(cx: &mut Ctx, name: &str, kind: &'static str, h: i32, hotbar: &[(u8, &str, u8)]) -> Outcome {
    reset(cx, hotbar);
    let logs = tree(cx, kind, OAK_X, OAK_Z, h);
    cx.wait_block(*logs.last().unwrap(), if kind == "oak" { "minecraft:oak_log" } else { "minecraft:birch_log" });
    let log = format!("{kind}_log");
    let shape_ok = all_are(cx, &logs, &log);
    let want = h as u32;
    cx.run("A14", name, |cx| {
        if !shape_ok {
            return (false, "the trunk did not build as planned".into());
        }
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: kind, quota: want, claim: claim(AREA.0 + 20), observer: who.clone(), probe: logs.clone() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let air = all_are(cx, &logs, "air");
        let got = cx.count(&log);
        let r = &g.result;
        let drops = if got < want || !r.drops_left.is_empty() { format!("; {}", server_drops(cx)) } else { String::new() };
        let (logs_ok, left_note) = logs_accounted(cx, want, got, &log, r.drops_left.len());
        let ok = g.status == "Done" && r.gathered == want && r.failure.is_none() && air && logs_ok && r.left.is_empty() && !r.short_of_quota && belief_air(&g, &who, &format!("minecraft:{log}")) && g.kinds.contains("minecraft:item");
        (ok, format!("{h}-log {kind}; server: all {h} log cells air {air}, {log} in the inventory {got} (want {want}); belief at the cells air, observed by {who}, changed from {log}: {}; {}", belief_air(&g, &who, &format!("minecraft:{log}")), result_line(&g) + &drops + &left_note))
    })
}

// Spec: specs/B4-gathering.md §4.3 (the tool by policy: the family's best tier)
fn oak_with_axe(cx: &mut Ctx) -> Outcome {
    fell_tree(cx, "oak_felled_with_an_axe", "oak", 5, &[(0, "iron_axe", 1)])
}

// Spec: specs/B4-gathering.md §4.3 (no axe: the empty hand)
fn oak_by_hand(cx: &mut Ctx) -> Outcome {
    fell_tree(cx, "oak_felled_by_hand", "oak", 4, &[])
}

// Spec: specs/B4-gathering.md §4.3 (any single-trunk tree type)
fn birch(cx: &mut Ctx) -> Outcome {
    fell_tree(cx, "birch_felled_trunk_read_back_first", "birch", 6, &[(0, "iron_axe", 1)])
}

// Spec: specs/B4-gathering.md §4.3, §4.10 (leaves that block a log are cleared first; the others stay)
fn log_behind_natural_leaves(cx: &mut Ctx) -> Outcome {
    reset(cx, &[(0, "iron_axe", 1)]);
    // A trunk of two logs and a branch log diagonal to the top one, inside a dense natural canopy:
    // no straight line reaches the branch without passing a leaf.
    let y = FLOOR_Y + 1;
    let (x, z) = (OAK_X, OAK_Z);
    fill(cx, (x - 2, y + 1, z - 2), (x + 3, y + 4, z + 2), "minecraft:oak_leaves[persistent=false,distance=1]");
    fill(cx, (x, y, z), (x, y + 1, z), "minecraft:oak_log");
    let branch = pos(x + 1, y + 2, z);
    cx.r(&format!("setblock {} {} {} minecraft:oak_log", branch.x, branch.y, branch.z));
    let logs = vec![pos(x, y, z), pos(x, y + 1, z), branch];
    let canopy = 6 * 5 * 4 - 3;
    cx.wait_block(branch, "minecraft:oak_log");
    cx.run("A14", "log_behind_natural_leaves_leaves_cleared_then_logs_felled", |cx| {
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: "oak", quota: 3, claim: claim(AREA.0 + 20), observer: who.clone(), probe: logs.clone() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let air = all_are(cx, &logs, "air");
        let got = cx.count("oak_log");
        let r = &g.result;
        let cleared: Vec<_> = r.changed.iter().filter(|c| c.before.ends_with("_leaves")).map(|c| c.pos).collect();
        let cleared_air = all_are(cx, &cleared, "air");
        // Most of the canopy is left to decay on its own.
        let few = !cleared.is_empty() && cleared.len() < canopy / 2;
        let (logs_ok, left_note) = logs_accounted(cx, 3, got, "oak_log", r.drops_left.len());
        let drops = if got < 3 || !r.drops_left.is_empty() { format!("; view drops {:?}; {}{left_note}", r.drops_left, server_drops(cx)) } else { String::new() };
        (
            g.status == "Done" && r.gathered == 3 && air && logs_ok && cleared_air && few && r.left.is_empty() && belief_air(&g, &who, "minecraft:oak_log"),
            format!("server: 3 log cells air {air}, oak logs in the inventory {got} (want 3), the {} cleared leaf cells air {cleared_air} (want 1 or more, under {}: {few}); {}", cleared.len(), canopy / 2, result_line(&g) + &drops),
        )
    })
}

// Spec: specs/B4-gathering.md §4.8 (a build is left intact and reported)
fn hut(cx: &mut Ctx) -> Outcome {
    reset(cx, &[(0, "iron_axe", 1)]);
    // Planks walls, log corner posts, a roof of persistent leaves (placed leaves keep `persistent=true`).
    let (x0, x1, z0, z1) = (812, 814, 356, 358);
    fill(cx, (x0, FLOOR_Y + 1, z0), (x1, FLOOR_Y + 3, z1), "minecraft:oak_planks hollow");
    for (x, z) in [(x0, z0), (x0, z1), (x1, z0), (x1, z1)] {
        fill(cx, (x, FLOOR_Y + 1, z), (x, FLOOR_Y + 3, z), "minecraft:oak_log");
    }
    fill(cx, (x0 - 1, FLOOR_Y + 4, z0 - 1), (x1 + 1, FLOOR_Y + 4, z1 + 1), "minecraft:oak_leaves[persistent=true,distance=1]");
    // A copy 20 blocks up (outside the claim) to compare against afterwards.
    let (lo, hi) = ((x0 - 1, FLOOR_Y + 1, z0 - 1), (x1 + 1, FLOOR_Y + 4, z1 + 1));
    cx.r(&format!("clone {} {} {} {} {} {} {} {} {}", lo.0, lo.1, lo.2, hi.0, hi.1, hi.2, lo.0, lo.1 + 20, lo.2));
    cx.wait_block(pos(x1, FLOOR_Y + 3, z1), "minecraft:oak_log");
    let compare = format!("execute if blocks {} {} {} {} {} {} {} {} {} all", lo.0, lo.1, lo.2, hi.0, hi.1, hi.2, lo.0, lo.1 + 20, lo.2);
    let posts: Vec<BlockPos> = [(x0, z0), (x0, z1), (x1, z0), (x1, z1)].iter().flat_map(|(x, z)| (1..=3).map(move |dy| pos(*x, FLOOR_Y + dy, *z))).collect();
    let before = cx.r(&compare).contains("passed");
    cx.run("A14", "hut_in_the_claim_left_intact_and_reported", |cx| {
        if !before {
            return (false, "the copy of the hut did not match before the job".into());
        }
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: "oak", quota: 4, claim: claim(AREA.0 + 20), observer: who, probe: posts.clone() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let intact = cx.r(&compare).contains("passed");
        let logs_in_hand = cx.count("oak_log");
        let r = &g.result;
        let not_natural = !r.left.is_empty() && r.left.iter().all(|l| matches!(l.reason, Reason::NotNatural(_)));
        // The posts stay in belief as logs (nothing was broken).
        let belief_logs = g.belief.iter().all(|b| b.block.as_deref() == Some("minecraft:oak_log"));
        (
            g.status == "Done" && r.gathered == 0 && r.changed.is_empty() && intact && logs_in_hand == 0 && not_natural && belief_logs,
            format!("server: hut identical to its copy {intact}, oak logs in the inventory {logs_in_hand} (want 0); every reported reason NotNatural {not_natural}; belief still shows the post logs {belief_logs}; {}", result_line(&g)),
        )
    })
}

// Spec: specs/B4-gathering.md §4.8 (the claim is a hard boundary: outside is never touched)
fn outside_and_across_the_edge(cx: &mut Ctx) -> Outcome {
    reset(cx, &[(0, "iron_axe", 1)]);
    // The claim ends at x = 812. One tree stands wholly outside it. The other has its trunk on the
    // last column and a branch log one block past the edge.
    let edge_x = AREA.0 + 12;
    let mut cells = tree(cx, "oak", AREA.0 + 17, 346, 4);
    let outside = cells.len();
    cells.extend(tree(cx, "oak", edge_x, 358, 4));
    let branch = pos(edge_x + 1, FLOOR_Y + 3, 358);
    cx.r(&format!("setblock {} {} {} minecraft:oak_log", branch.x, branch.y, branch.z));
    cells.push(branch);
    cx.wait_block(branch, "minecraft:oak_log");
    cx.run("A14", "tree_outside_the_claim_and_one_across_its_edge_left_and_reported", |cx| {
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: "oak", quota: 8, claim: claim(edge_x), observer: who, probe: Vec::new() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let standing = all_are(cx, &cells, "oak_log");
        let logs_in_hand = cx.count("oak_log");
        let r = &g.result;
        let edge_reported = r.left.iter().any(|l| l.reason == Reason::OutsideClaim(branch));
        (
            g.status == "Done" && r.gathered == 0 && r.changed.is_empty() && standing && logs_in_hand == 0 && edge_reported && r.left.len() == 1,
            format!(
                "server: all {} logs of both trees (and the branch) still standing {standing}, oak logs in the inventory {logs_in_hand} (want 0); the edge tree reported OutsideClaim at its branch {edge_reported}; the tree wholly outside ({outside} logs) is never read, so it is not listed: {}",
                cells.len(),
                result_line(&g)
            ),
        )
    })
}

// Spec: specs/B4-gathering.md §3 (a quota stops the job; what is left is reported)
fn quota_stops_after_one_tree(cx: &mut Ctx) -> Outcome {
    reset(cx, &[(0, "iron_axe", 1)]);
    let near = tree(cx, "oak", 809, 350, 4);
    let far = tree(cx, "oak", 816, 358, 4);
    cx.wait_block(*far.last().unwrap(), "minecraft:oak_log");
    cx.run("A14", "quota_of_four_stops_after_one_tree", |cx| {
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: "oak", quota: 4, claim: claim(AREA.0 + 20), observer: who.clone(), probe: near.clone() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let near_air = all_are(cx, &near, "air");
        let far_standing = all_are(cx, &far, "oak_log");
        let got = cx.count("oak_log");
        let r = &g.result;
        let quota_left = !r.left.is_empty() && r.left.iter().all(|l| l.reason == Reason::Quota);
        (
            g.status == "Done" && r.gathered == 4 && !r.short_of_quota && near_air && far_standing && got == 4 && quota_left && belief_air(&g, &who, "minecraft:oak_log"),
            format!("server: nearest tree's 4 log cells air {near_air}, the other tree still standing {far_standing}, oak logs in the inventory {got} (want 4); the other tree reported Quota {quota_left}; {}", result_line(&g)),
        )
    })
}

// Spec: specs/B4-gathering.md §4.3, §4.10 (a log out of reach is left and reported; a stand on a target, broken last)
fn tall_trunk(cx: &mut Ctx) -> Outcome {
    reset(cx, &[(0, "iron_axe", 1)]);
    let logs = tree(cx, "oak", OAK_X, OAK_Z, 9);
    cx.wait_block(*logs.last().unwrap(), "minecraft:oak_log");
    let reachable = logs[..8].to_vec();
    let top = logs[8];
    cx.run("A14", "tall_trunk_top_log_left_unreachable", |cx| {
        let who = cx.name.clone();
        let op = GatherOp::Fell { tree: "oak", quota: 9, claim: claim(AREA.0 + 20), observer: who.clone(), probe: reachable.clone() };
        let g = match gather_info(call_long(cx, Op::Gather(op))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        sleep(Duration::from_millis(500));
        let air = all_are(cx, &reachable, "air");
        let top_stands = cx.block_is(top, "oak_log");
        let got = cx.count("oak_log");
        let r = &g.result;
        let left_ok = r.left.len() == 1 && r.left[0].reason == Reason::Unreachable && r.left[0].at == top;
        // Log 8 is out of reach from the ground (5.4 above the eye): the bot stood on log 1.
        let stepped = g.max_y >= (FLOOR_Y + 2) as f64 - 0.01;
        (
            g.status == "Done" && r.gathered == 8 && r.short_of_quota && air && top_stands && got == 8 && left_ok && stepped && belief_air(&g, &who, "minecraft:oak_log"),
            format!("9-log trunk; server: the 8 reachable log cells air {air}, the top log still standing {top_stands}, oak logs in the inventory {got} (want 8); left is exactly the top log, Unreachable: {left_ok}; the pathfinder's step onto log 1 (bot reached y >= {}): {stepped}; {}", FLOOR_Y + 2, result_line(&g)),
        )
    })
}

// Spec: specs/B4-gathering.md §4.8 (the view reads `persistent` for leaves; `is_solid` for logs and leaves)
fn view_reads(cx: &mut Ctx) -> Outcome {
    reset(cx, &[]);
    let y = FLOOR_Y + 1;
    let cells = [pos(806, y, 346), pos(806, y, 347), pos(806, y, 348), pos(806, y, 349), pos(806, y - 1, 346)];
    cx.r("setblock 806 200 346 minecraft:oak_log");
    cx.r("setblock 806 200 347 minecraft:oak_leaves[persistent=false,distance=1]");
    cx.r("setblock 806 200 348 minecraft:oak_leaves[persistent=true,distance=1]");
    cx.wait_block(cells[2], "minecraft:oak_leaves");
    cx.run("A14", "view_reads_logs_and_leaves_solid_and_persistent", |cx| {
        let g = match gather_info(cx.call(Op::Gather(GatherOp::Probe(cells.to_vec())))) {
            Ok(g) => g,
            Err(e) => return (false, e),
        };
        let r = &g.reads;
        let want: [(&str, bool, Option<&str>); 5] = [("minecraft:oak_log", true, None), ("minecraft:oak_leaves", true, Some("false")), ("minecraft:oak_leaves", true, Some("true")), ("minecraft:air", false, None), ("minecraft:stone", true, None)];
        let ok = r.len() == want.len() && r.iter().zip(want).all(|(c, (n, s, p))| c.name.as_deref() == Some(n) && c.solid == s && c.persistent.as_deref() == p);
        (ok, format!("(name, is_solid, persistent) as the view reads them: {:?}; want {want:?}", r.iter().map(|c| (c.name.clone().unwrap_or_default(), c.solid, c.persistent.clone())).collect::<Vec<_>>()))
    })
}

// Spec: specs/B4-gathering.md §3, §4.3, §4.8, §4.10
pub fn a14(cx: &mut Ctx) -> Vec<Outcome> {
    // `A14_ONLY=<text>` runs only the rows whose function name contains the text.
    let only = std::env::var("A14_ONLY").unwrap_or_default();
    type Row = fn(&mut Ctx) -> Outcome;
    let rows: [(&str, Row); 9] = [
        ("view_reads", view_reads),
        ("oak_with_axe", oak_with_axe),
        ("oak_by_hand", oak_by_hand),
        ("birch", birch),
        ("log_behind_natural_leaves", log_behind_natural_leaves),
        ("hut", hut),
        ("outside_and_across_the_edge", outside_and_across_the_edge),
        ("quota_stops_after_one_tree", quota_stops_after_one_tree),
        ("tall_trunk", tall_trunk),
    ];
    let out = rows.iter().filter(|(n, _)| n.contains(&only)).map(|(_, f)| f(cx)).collect();
    // `clean_in` clears 8 rows; the tall trunk and the clone went higher.
    cx.r(&format!("fill {} {} {} {} {CLEAR_Y} {} minecraft:air", AREA.0, FLOOR_Y + 1, AREA.1, AREA.0 + 20, AREA.1 + 20));
    cx.clean_in(AREA);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop(id: &str, count: u32, on_leaves: bool) -> ServerDrop {
        ServerDrop { id: id.into(), count, on_leaves }
    }

    #[test]
    fn q1_case_passes() {
        assert!(left_on_leaves_ok(6, 5, "minecraft:birch_log", &[drop("minecraft:birch_log", 1, true)]));
    }

    #[test]
    fn drop_missing_on_server_fails() {
        assert!(!left_on_leaves_ok(6, 5, "minecraft:birch_log", &[]));
    }

    #[test]
    fn drop_on_a_non_leaf_block_fails() {
        assert!(!left_on_leaves_ok(6, 5, "minecraft:birch_log", &[drop("minecraft:birch_log", 1, false)]));
    }

    #[test]
    fn count_that_does_not_add_up_fails() {
        assert!(!left_on_leaves_ok(6, 4, "minecraft:birch_log", &[drop("minecraft:birch_log", 1, true)]));
        assert!(!left_on_leaves_ok(6, 5, "minecraft:birch_log", &[drop("minecraft:birch_log", 2, true)]));
    }

    #[test]
    fn other_item_fails_and_split_stacks_add_up() {
        assert!(!left_on_leaves_ok(6, 5, "minecraft:birch_log", &[drop("minecraft:stick", 1, true)]));
        assert!(left_on_leaves_ok(6, 3, "minecraft:birch_log", &[drop("minecraft:birch_log", 2, true), drop("minecraft:birch_log", 1, true)]));
    }

    #[test]
    fn entity_data_keeps_the_namespace() {
        assert_eq!(entity_data("Item has the following entity data: \"minecraft:birch_log\""), "minecraft:birch_log");
        assert_eq!(entity_data("Item has the following entity data: 1"), "1");
    }
}
