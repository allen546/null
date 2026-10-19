//! A10: open a container by right-clicking it, read it, move items by clicks, close it. Plan:
//! specs/plans/ABILITY_CHECK.md. Area: corner (590, 342). Server truth: `data get block <pos>
//! Items[k]` field by field, and the bot's inventory over RCON; the server must correct no slot.

use std::{thread::sleep, time::Duration};

use null_bot_api::{Click, Face, OpenOutcome};

use crate::{
    scenarios::{Ctx, Outcome, last_int, pos},
    scenarios_inventory::{inv, quoted, server_inventory, short, view_slots},
    skill::Op,
    skill_inventory::{InvInfo, InvOp},
};

const AREA: (i32, i32) = (590, 342);
const SPAWN: (f64, f64, f64) = (600.5, 200.0, 352.5);
/// The container block (the bot stands two blocks south of it, facing north).
const C: (i32, i32, i32) = (600, 200, 350);

pub(crate) type Slots = Vec<(i64, String, i64)>;
/// Where a container's items live for `data get`: ("block x y z" or "entity <bot>", "Items" or "EnderItems").
pub(crate) type Src = (String, &'static str);

fn blk(x: i32) -> Src {
    (format!("block {}", at(x)), "Items")
}

fn at(x: i32) -> String {
    format!("{x} {} {}", C.1, C.2)
}

/// SNBT of a chest's `Items`: (slot, item with optional `[components]` after the id, count).
fn items_nbt(items: &[(u8, &str, u8)]) -> String {
    let parts: Vec<String> = items
        .iter()
        .map(|(s, i, c)| match i.split_once('|') {
            Some((id, comps)) => format!("{{Slot:{s}b,id:\"minecraft:{id}\",count:{c},components:{comps}}}"),
            None => format!("{{Slot:{s}b,id:\"minecraft:{i}\",count:{c}}}"),
        })
        .collect();
    format!("{{Items:[{}]}}", parts.join(","))
}

/// The block entity's items as (slot, item, count), read one field at a time (Paper cuts long
/// replies short).
pub(crate) fn server_container(cx: &mut Ctx, src: &Src) -> Slots {
    let mut out = Vec::new();
    for k in 0..54 {
        let p = format!("data get {} {}[{k}]", src.0, src.1);
        let r = cx.r(&format!("{p}.Slot"));
        if !r.contains("has the following") {
            break;
        }
        let slot = last_int(&r).unwrap_or(-1);
        let id = quoted(&cx.r(&format!("{p}.id"))).unwrap_or_default();
        let count = last_int(&cx.r(&format!("{p}.count"))).unwrap_or(-1);
        out.push((slot, id.trim_start_matches("minecraft:").to_string(), count));
    }
    out.sort();
    out
}

/// The bot's reading of the container's own slots (the window minus the 36 player slots).
pub(crate) fn view_container(i: &InvInfo) -> Slots {
    let Some(w) = &i.window else { return vec![] };
    let n = w.slots.len().saturating_sub(36);
    w.slots[..n]
        .iter()
        .enumerate()
        .filter_map(|(k, s)| s.as_ref().map(|s| (k as i64, s.item.trim_start_matches("minecraft:").to_string(), s.count as i64)))
        .collect()
}

/// The window slot that shows inventory slot `i` (0-8 hotbar, 9-35 main) in a container of `size`.
pub(crate) fn iw(size: u16, i: u16) -> Option<u16> {
    Some(if i < 9 { size + 27 + i } else { size + i - 9 })
}

pub(crate) fn short_slots(s: &Slots) -> String {
    s.iter().map(|(k, i, c)| format!("{k}:{i}x{c}")).collect::<Vec<_>>().join(" ")
}

/// A scene with the container block(s) placed by RCON.
fn scene(cx: &mut Ctx, blocks: &[(i32, &str, &[(u8, &str, u8)])], give: &[(u16, &str, u8)]) {
    cx.scene_in(AREA, &[], SPAWN);
    let n = cx.name.clone();
    for (x, block, items) in blocks {
        cx.r(&format!("setblock {} minecraft:{block}{}", at(*x), items_nbt(items)));
    }
    for (slot, item, count) in give {
        cx.r(&format!("item replace entity {n} container.{slot} with minecraft:{item} {count}"));
    }
    sleep(Duration::from_millis(500));
}

fn open(cx: &mut Ctx) -> InvInfo {
    open_on(cx, None)
}

fn open_on(cx: &mut Ctx, face: Option<Face>) -> InvInfo {
    inv(cx, InvOp::OpenContainer(pos(C.0, C.1, C.2), face))
}

/// The player's ender chest slots are the player's, not the block's: set them by `item replace`.
fn set_ender(cx: &mut Ctx, items: &[(u8, &str, u8)]) {
    let n = cx.name.clone();
    for k in 0..27 {
        cx.r(&format!("item replace entity {n} enderchest.{k} with minecraft:air"));
    }
    for (k, item, count) in items {
        cx.r(&format!("item replace entity {n} enderchest.{k} with minecraft:{item} {count}"));
    }
    sleep(Duration::from_millis(300));
}

fn ender(cx: &Ctx) -> Src {
    (format!("entity {}", cx.name), "EnderItems")
}

fn lit(cx: &mut Ctx, v: bool) -> bool {
    cx.r(&format!("execute if block {} {} {} minecraft:redstone_lamp[lit={v}]", C.0 - 1, C.1, C.2)).contains("passed")
}

/// Open, take one stack by click, put another by click, close; the whole row's server check.
fn take_and_put(cx: &mut Ctx, kind: &str, size: u16, src: &Src, want: &Slots, ev: &mut Vec<String>) -> bool {
    let (mut ok, _) = open_expect(cx, kind, size as usize, ev);
    ok &= click(cx, Some(0), Click::Left, ev);
    ok &= click(cx, iw(size, 1), Click::Left, ev);
    ok &= click(cx, iw(size, 9), Click::Left, ev);
    ok &= click(cx, Some(size - 1), Click::Left, ev);
    let i = status(cx);
    ok &= container_agrees(cx, std::slice::from_ref(src), want, &i, ev);
    ok &= close(cx, ev);
    ok & inventory_agrees(cx, ev)
}

pub(crate) fn click(cx: &mut Ctx, slot: Option<u16>, c: Click, ev: &mut Vec<String>) -> bool {
    let i = inv(cx, InvOp::Click(slot, c));
    let clean = i.click.as_ref().is_some_and(|o| o.uncorrected() && o.slot == slot);
    ev.push(match &i.click {
        Some(o) if clean => format!("{c:?}@{slot:?} ok (carry {})", i.window.as_ref().map_or("-".into(), |w| short(&w.carried))),
        Some(o) => format!("{c:?}@{slot:?} refused {:?}, corrected {:?}, {} resend(s)", o.refused, o.corrected, o.full_resends),
        None => format!("{c:?}@{slot:?}: no click result"),
    });
    clean
}

/// As `click`, but a whole-window resend is allowed: Grim's own inventory model has no
/// `ShulkerBoxSlot#mayPlace`, so on Paper it resyncs after a refused shift-click (same contents).
fn click_resync_ok(cx: &mut Ctx, slot: Option<u16>, c: Click, ev: &mut Vec<String>) -> bool {
    let i = inv(cx, InvOp::Click(slot, c));
    let ok = i.click.as_ref().is_some_and(|o| o.refused.is_none() && o.corrected.is_empty() && o.slot == slot);
    ev.push(format!("{c:?}@{slot:?} {} ({} resend(s) of the whole window)", if ok { "ok" } else { "FAILED" }, i.click.as_ref().map_or(0, |o| o.full_resends)));
    ok
}

/// Opens the container; fails the row unless the menu kind and size are as expected.
fn open_expect(cx: &mut Ctx, kind: &str, size: usize, ev: &mut Vec<String>) -> (bool, InvInfo) {
    let i = open(cx);
    let w = i.window.clone();
    let opened = matches!(i.open.as_ref().map(|o| &o.outcome), Some(OpenOutcome::Opened { kind: k, window_id }) if *k == kind && Some(*window_id) == w.as_ref().map(|w| w.id));
    let sized = w.as_ref().is_some_and(|w| w.kind == Some(kind) && w.id != 0 && w.slots.len() == size + 36);
    ev.push(format!("open {:?}, window {:?}/{} slots", i.open.as_ref().map(|o| &o.outcome), w.as_ref().and_then(|w| w.kind), w.as_ref().map_or(0, |w| w.slots.len())));
    (opened && sized, i)
}

pub(crate) fn close(cx: &mut Ctx, ev: &mut Vec<String>) -> bool {
    let c = inv(cx, InvOp::Close);
    ev.push("closed".into());
    c.window.is_none()
}

/// The bot's inventory view against the server's (slots 0-35 and the offhand).
pub(crate) fn inventory_agrees(cx: &mut Ctx, ev: &mut Vec<String>) -> bool {
    sleep(Duration::from_millis(200));
    let keep = |s: i64| s <= 35 || s == 40;
    let server: Slots = server_inventory(cx)
        .into_iter()
        .filter(|h| keep(h.slot))
        .map(|h| (h.slot, h.id.trim_start_matches("minecraft:").to_string(), h.count))
        .collect();
    let view: Slots = view_slots(&inv(cx, InvOp::Status))
        .into_iter()
        .map(|(s, i, c)| (s, i.trim_start_matches("minecraft:").to_string(), c))
        .filter(|(s, ..)| keep(*s))
        .collect();
    ev.push(format!("inventory {}", short_slots(&server)));
    if view != server {
        ev.push(format!("BOT INVENTORY {}", short_slots(&view)));
    }
    view == server
}

/// Server truth of the container(s) at `xs` (first block's slots first) against `want` and the bot.
pub(crate) fn container_agrees(cx: &mut Ctx, xs: &[Src], want: &Slots, bot: &InvInfo, ev: &mut Vec<String>) -> bool {
    sleep(Duration::from_millis(200));
    let mut server = Slots::new();
    for (k, x) in xs.iter().enumerate() {
        server.extend(server_container(cx, x).into_iter().map(|(s, i, c)| (s + 27 * k as i64, i, c)));
    }
    ev.push(format!("container {}", short_slots(&server)));
    let view = view_container(bot);
    if &server != want {
        ev.push(format!("WANTED {}", short_slots(want)));
    }
    if view != server {
        ev.push(format!("BOT CONTAINER {}", short_slots(&view)));
    }
    &server == want && view == server
}

pub(crate) fn s(v: &[(i64, &str, i64)]) -> Slots {
    v.iter().map(|&(a, b, c)| (a, b.to_string(), c)).collect()
}

pub(crate) fn status(cx: &mut Ctx) -> InvInfo {
    inv(cx, InvOp::Status)
}

fn row(
    cx: &mut Ctx,
    name: &str,
    blocks: &[(i32, &str, &[(u8, &str, u8)])],
    give: &[(u16, &str, u8)],
    body: impl FnOnce(&mut Ctx, &mut Vec<String>) -> bool,
) -> Outcome {
    scene(cx, blocks, give);
    cx.run("A10", name, |cx| {
        let mut ev = Vec::new();
        let ok = body(cx, &mut ev);
        (ok, ev.join("; "))
    })
}

const BASIC: &[(u8, &str, u8)] = &[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)];

// Spec: specs/B3-inventory-crafting.md §4.7, §4.8 (open, move by clicks, read exact contents, close)
pub fn a10(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    let chest: &[(i32, &str, &[(u8, &str, u8)])] = &[(C.0, "chest", BASIC)];

    out.push(row(cx, "open_single_chest", chest, &[(9, "cobblestone", 7)], |cx, ev| {
        let (ok, i) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let a = container_agrees(cx, &[blk(C.0)], &s(&[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)]), &i, ev);
        let c = close(cx, ev);
        ok & a & c & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "take_by_clicks", &[(C.0, "chest", &[(3, "stone", 32), (5, "stick", 9)])], &[], |cx, ev| {
        let (mut ok, i) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let _ = i;
        ok &= click(cx, Some(3), Click::Left, ev);
        ok &= click(cx, iw(27, 0), Click::Left, ev);
        ok &= click(cx, Some(5), Click::Right, ev);
        ok &= click(cx, iw(27, 10), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(5, "stick", 4)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "put_by_clicks", chest, &[(9, "cobblestone", 20)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        ok &= click(cx, Some(10), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "stone", 5), (4, "diamond", 3), (10, "cobblestone", 20), (13, "oak_log", 64), (26, "apple", 1)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // Vanilla `ChestMenu#quickMoveStack`: container to player fills from the last slot backwards.
    out.push(row(cx, "quick_move_out", &[(C.0, "chest", &[(2, "iron_ingot", 40), (6, "dirt", 64)])], &[(8, "iron_ingot", 30)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        ok &= click(cx, Some(2), Click::ShiftLeft, ev);
        ok &= click(cx, Some(6), Click::ShiftLeft, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // Player to container fills forward, merging into existing stacks first.
    out.push(row(cx, "quick_move_in", &[(C.0, "chest", &[(0, "stone", 60)])], &[(0, "stone", 10), (9, "dirt", 5)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        ok &= click(cx, iw(27, 0), Click::ShiftLeft, ev);
        ok &= click(cx, iw(27, 9), Click::ShiftLeft, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "stone", 64), (1, "stone", 6), (2, "dirt", 5)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // The right half of a double chest is the first 27 slots (vanilla `ChestBlock#getBlockType`).
    let right: &[(u8, &str, u8)] = &[(1, "stone", 3), (20, "gold_ingot", 12)];
    let left: &[(u8, &str, u8)] = &[(0, "emerald", 2), (26, "coal", 64)];
    let double: &[(i32, &str, &[(u8, &str, u8)])] =
        &[(C.0, "chest[facing=north,type=left]", left), (C.0 + 1, "chest[facing=north,type=right]", right)];
    out.push(row(cx, "double_chest", double, &[], |cx, ev| {
        let (mut ok, i) = open_expect(cx, "minecraft:generic_9x6", 54, ev);
        ok &= container_agrees(cx, &[blk(C.0 + 1), blk(C.0)], &s(&[(1, "stone", 3), (20, "gold_ingot", 12), (27, "emerald", 2), (53, "coal", 64)]), &i, ev);
        ok &= click(cx, Some(53), Click::ShiftLeft, ev);
        ok &= click(cx, iw(54, 8), Click::Left, ev);
        ok &= click(cx, Some(40), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0 + 1), blk(C.0)], &s(&[(1, "stone", 3), (20, "gold_ingot", 12), (27, "emerald", 2), (40, "coal", 64)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "barrel_open_property", &[(C.0, "barrel[facing=up,open=false]", BASIC)], &[], |cx, ev| {
        let state = |cx: &mut Ctx, v: &str| cx.r(&format!("execute if block {} minecraft:barrel[open={v}]", at(C.0))).contains("passed");
        let before = state(cx, "false");
        let (mut ok, i) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let while_open = state(cx, "true");
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)]), &i, ev);
        ok &= close(cx, ev);
        // The barrel re-checks its viewers every 5 ticks.
        let closed = cx.poll(3.0, |cx| state(cx, "false")).is_some();
        ev.push(format!("open=false before {before}, open=true while open {while_open}, open=false after close {closed}"));
        ok & before & while_open & closed
    }));

    out.push(row(cx, "hopper_menu", &[(C.0, "hopper[facing=down]", &[(0, "iron_nugget", 4), (2, "redstone", 9), (4, "clay_ball", 1)])], &[], |cx, ev| {
        let (mut ok, i) = open_expect(cx, "minecraft:hopper", 5, ev);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "iron_nugget", 4), (2, "redstone", 9), (4, "clay_ball", 1)]), &i, ev);
        ok &= click(cx, Some(2), Click::ShiftLeft, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "iron_nugget", 4), (4, "clay_ball", 1)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "dispenser_menu", &[(C.0, "dispenser[facing=up]", &[(0, "arrow", 12), (8, "bucket", 1)])], &[(9, "arrow", 8)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_3x3", 9, ev);
        ok &= click(cx, iw(9, 9), Click::ShiftLeft, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "arrow", 20), (8, "bucket", 1)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // B3 §4.8: exact counts in exact slots, read back from the bot and from the server.
    out.push(row(cx, "exact_contents", &[(C.0, "chest", &[])], &[(9, "stone", 64), (10, "apple", 13), (11, "oak_log", 7), (12, "diamond", 1)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        for (from, to, c) in [(9, 0, Click::Left), (10, 5, Click::Left), (12, 13, Click::Left)] {
            ok &= click(cx, iw(27, from), c, ev);
            ok &= click(cx, Some(to), Click::Left, ev);
        }
        ok &= click(cx, iw(27, 11), Click::Left, ev);
        for _ in 0..3 {
            ok &= click(cx, Some(26), Click::Right, ev);
        }
        ok &= click(cx, iw(27, 11), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(0, "stone", 64), (5, "apple", 13), (13, "diamond", 1), (26, "oak_log", 3)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "server_change_while_open", chest, &[], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        cx.r(&format!("item replace block {} container.7 with minecraft:gold_ingot 9", at(C.0)));
        cx.r(&format!("item replace block {} container.0 with minecraft:air", at(C.0)));
        sleep(Duration::from_millis(600));
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[(4, "diamond", 3), (7, "gold_ingot", 9), (13, "oak_log", 64), (26, "apple", 1)]), &i, ev);
        ok &= close(cx, ev);
        ok
    }));

    // One open-take-put-close row per storage kind. Slot 0 holds 12 of an item; slot 9 of the
    // inventory holds a different one, put into the last container slot.
    let two: &[(u8, &str, u8)] = &[(0, "iron_ingot", 12)];
    let put = |size: i64| s(&[(size - 1, "cobblestone", 20)]);
    out.push(row(cx, "trapped_chest_powers_while_open", &[(C.0, "trapped_chest", two)], &[(9, "cobblestone", 20)], |cx, ev| {
        cx.r(&format!("setblock {} {} {} minecraft:redstone_lamp", C.0 - 1, C.1, C.2));
        sleep(Duration::from_millis(300));
        let off = lit(cx, false);
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let on = cx.poll(2.0, |cx| lit(cx, true)).is_some();
        ok &= click(cx, Some(0), Click::Left, ev);
        ok &= click(cx, iw(27, 1), Click::Left, ev);
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        ok &= click(cx, Some(26), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &put(27), &i, ev);
        ok &= close(cx, ev);
        let off_again = cx.poll(3.0, |cx| lit(cx, false)).is_some();
        ev.push(format!("lamp unlit before {off}, lit while open {on}, unlit after close {off_again}"));
        ok & off & on & off_again & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "shulker_box", &[(C.0, "shulker_box[facing=up]", two)], &[(9, "cobblestone", 20)], |cx, ev| {
        take_and_put(cx, "minecraft:shulker_box", 27, &blk(C.0), &put(27), ev)
    }));

    // Vanilla `ShulkerBoxSlot#mayPlace`: the click is sent, and the server refuses it as we do.
    out.push(row(cx, "shulker_refuses_shulker_pickup", &[(C.0, "shulker_box[facing=up]", &[])], &[(9, "red_shulker_box", 1)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:shulker_box", 27, ev);
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        ok &= click(cx, Some(3), Click::Left, ev);
        let carried = inv(cx, InvOp::Status).window.and_then(|w| w.carried);
        ev.push(format!("still carrying {}", short(&carried)));
        ok &= carried.is_some_and(|c| c.item == "minecraft:red_shulker_box");
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));
    out.push(row(cx, "shulker_refuses_shulker_quick_move", &[(C.0, "shulker_box[facing=up]", &[])], &[(10, "blue_shulker_box", 1)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:shulker_box", 27, ev);
        ok &= click_resync_ok(cx, iw(27, 10), Click::ShiftLeft, ev);
        sleep(Duration::from_millis(300));
        let i = status(cx);
        ok &= container_agrees(cx, &[blk(C.0)], &s(&[]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "ender_chest_is_the_players", &[(C.0, "ender_chest[facing=south]", &[])], &[(9, "cobblestone", 20)], |cx, ev| {
        set_ender(cx, &[(0, "iron_ingot", 12), (5, "emerald", 3)]);
        let src = ender(cx);
        let (mut ok, i) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        ok &= container_agrees(cx, std::slice::from_ref(&src), &s(&[(0, "iron_ingot", 12), (5, "emerald", 3)]), &i, ev);
        ok &= click(cx, Some(0), Click::Left, ev);
        ok &= click(cx, iw(27, 1), Click::Left, ev);
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        ok &= click(cx, Some(26), Click::Left, ev);
        let i = status(cx);
        ok &= container_agrees(cx, std::slice::from_ref(&src), &s(&[(5, "emerald", 3), (26, "cobblestone", 20)]), &i, ev);
        ok &= close(cx, ev);
        let r = ok & inventory_agrees(cx, ev);
        set_ender(cx, &[]);
        r
    }));

    // Hopper shapes (fork patch 11): click the bowl's top, then a sideways hopper's spout side.
    out.push(row(cx, "hopper_top_face", &[(C.0, "hopper[facing=down]", two)], &[(9, "cobblestone", 20)], |cx, ev| {
        let i = open_on(cx, Some(Face::Up));
        ev.push(format!("{:?}", i.open.as_ref().map(|o| &o.outcome)));
        let opened = i.window.as_ref().is_some_and(|w| w.kind == Some("minecraft:hopper"));
        let c = !opened || close(cx, ev);
        opened & c
    }));
    out.push(row(cx, "hopper_sideways_faces", &[(C.0, "hopper[facing=east]", two)], &[], |cx, ev| {
        let mut ok = true;
        // seen from the south (the bot's spot), then from the east where the spout points
        for (face, from) in [(Face::South, None), (Face::East, Some("603.5 200 350.5 90 0"))] {
            if let Some(at) = from {
                cx.r(&format!("tp {} {at}", cx.name));
                sleep(Duration::from_millis(800));
            }
            let i = open_on(cx, Some(face));
            ev.push(format!("{face:?}: {:?} {:?}", i.open.as_ref().map(|o| &o.outcome), i.open.as_ref().and_then(|o| o.refused)));
            let opened = i.window.as_ref().is_some_and(|w| w.kind == Some("minecraft:hopper"));
            ok &= opened;
            if opened {
                ok &= close(cx, ev);
            }
            sleep(Duration::from_millis(300));
        }
        ok
    }));

    out.push(row(cx, "dropper_menu", &[(C.0, "dropper[facing=up]", two)], &[(9, "cobblestone", 20)], |cx, ev| {
        take_and_put(cx, "minecraft:generic_3x3", 9, &blk(C.0), &put(9), ev)
    }));

    // Refusals and failures.
    out.push(row(cx, "out_of_reach_refused", chest, &[], |cx, ev| {
        cx.r(&format!("tp {} 600.5 200 358.5 0 0", cx.name));
        sleep(Duration::from_millis(800));
        let i = open(cx);
        let o = i.open.clone();
        ev.push(format!("{o:?}"));
        let refused = o.as_ref().is_some_and(|o| o.outcome == OpenOutcome::Refused && o.refused == Some("out of reach"));
        // nothing reached the server: the chest is as it was and no window opened
        refused && i.window.is_none() && server_container(cx, &blk(C.0)) == s(&[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)])
    }));

    out.push(row(cx, "blocked_chest_no_screen", chest, &[], |cx, ev| {
        cx.r(&format!("setblock {} {} {} minecraft:stone", C.0, C.1 + 1, C.2));
        sleep(Duration::from_millis(300));
        let i = open(cx);
        ev.push(format!("{:?}", i.open));
        // Vanilla's client sends the click; the server declines (`ChestBlock#isChestBlockedAt`).
        i.open.as_ref().is_some_and(|o| o.outcome == OpenOutcome::NoScreen && o.refused.is_none()) && i.window.is_none()
    }));

    // Sent, no screen: `EnderChestBlock` asks `ChestBlock#isChestBlockedAt`; a shulker box needs
    // room for its lid (`ShulkerBoxBlock#canOpen`).
    for (name, block, cover) in [
        ("blocked_ender_chest_no_screen", "ender_chest[facing=south]", (C.0, C.1 + 1, C.2)),
        ("blocked_shulker_lid_no_screen", "shulker_box[facing=up]", (C.0, C.1 + 1, C.2)),
    ] {
        out.push(row(cx, name, &[(C.0, block, &[])], &[], |cx, ev| {
            cx.r(&format!("setblock {} {} {} minecraft:stone", cover.0, cover.1, cover.2));
            sleep(Duration::from_millis(300));
            let i = open(cx);
            ev.push(format!("{:?}", i.open));
            i.open.as_ref().is_some_and(|o| o.outcome == OpenOutcome::NoScreen && o.refused.is_none()) && i.window.is_none()
        }));
    }

    out.push(row(cx, "crouching_with_item_refused", chest, &[], |cx, ev| {
        cx.r(&format!("item replace entity {} hotbar.0 with minecraft:stone 3", cx.name));
        cx.call(Op::Sneak(true));
        sleep(Duration::from_millis(500));
        let i = open(cx);
        let o = i.open.clone();
        ev.push(format!("{o:?}"));
        let refused = o.as_ref().is_some_and(|o| o.outcome == OpenOutcome::Refused && o.refused.is_some_and(|r| r.contains("crouching")));
        cx.call(Op::Sneak(false));
        sleep(Duration::from_millis(500));
        // standing again, the same click works
        let (ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let c = close(cx, ev);
        refused && i.window.is_none() && ok & c
    }));

    out.push(row(cx, "crouching_empty_hands_opens", chest, &[], |cx, ev| {
        cx.call(Op::Sneak(true));
        sleep(Duration::from_millis(500));
        let (ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        let c = close(cx, ev);
        cx.call(Op::Sneak(false));
        ok & c
    }));

    // Items with components move through a container window without a corrected slot.
    let book = "enchanted_book|{\"minecraft:stored_enchantments\":{\"minecraft:mending\":1}}";
    let named = "stick|{\"minecraft:custom_name\":{text:\"Renamed\",color:\"red\"}}";
    out.push(row(cx, "component_items", &[(C.0, "chest", &[(0, book, 1), (1, named, 1)])], &[(9, "diamond_sword[damage=50,enchantments={\"minecraft:sharpness\":3}]", 1)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, "minecraft:generic_9x3", 27, ev);
        ok &= click(cx, Some(0), Click::Left, ev);
        ok &= click(cx, iw(27, 0), Click::Left, ev);
        ok &= click(cx, Some(1), Click::Left, ev);
        ok &= click(cx, Some(8), Click::Left, ev);
        ok &= click(cx, iw(27, 9), Click::Left, ev);
        ok &= click(cx, Some(20), Click::Left, ev);
        ok &= close(cx, ev);
        sleep(Duration::from_millis(200));
        let n = cx.name.clone();
        let chk = |cx: &mut Ctx, target: String, item: &str| cx.r(&format!("execute if items {target} {item}")).contains("passed");
        let b = chk(cx, format!("entity {n} hotbar.0"), "minecraft:enchanted_book[stored_enchantments={\"minecraft:mending\":1}]");
        let r = chk(cx, format!("block {} container.8", at(C.0)), "minecraft:stick[custom_name={text:\"Renamed\",color:\"red\"}]");
        let d = chk(cx, format!("block {} container.20", at(C.0)), "minecraft:diamond_sword[damage=50,enchantments={\"minecraft:sharpness\":3}]");
        ev.push(format!("book in hotbar {b}, renamed stick in chest slot 8 {r}, damaged sword in chest slot 20 {d}"));
        ok & b & r & d & inventory_agrees(cx, ev)
    }));

    cx.clean_in(AREA);
    out
}

