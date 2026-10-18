//! A11: craft by clicks in the player's 2x2 grid and at a crafting table (no recipe book). Plan:
//! specs/plans/ABILITY_CHECK.md. Area: corner (620, 342). Server truth: the bot's inventory over
//! RCON after the screen closes (the grid itself is not readable), item entities, and for the
//! repair row the item's components.
//!
//! Corrections: a crafting result is computed by the server, so slot 0 may be updated after any
//! click (`Window::computed_slots`; vanilla `CraftingMenu#slotChangedCraftingGrid` sends it after
//! every grid change). A shift-click on the result may also update the grid and the destination:
//! the vanilla client crafts once (`AbstractContainerMenu#doClick` QUICK_MOVE loop stops because
//! its result slot empties; the client never recomputes it), the server loops until the grid no
//! longer matches. Every other slot must stay uncorrected.

use std::{
    collections::BTreeMap,
    sync::atomic::{AtomicBool, Ordering},
    thread::sleep,
    time::Duration,
};

use null_bot_api::Click;

use crate::{
    scenarios::{Ctx, Outcome, pos},
    scenarios_inventory::{inv, server_inventory, short, view_slots},
    skill_inventory::{InvInfo, InvOp},
};

const AREA: (i32, i32) = (620, 342);
const SPAWN: (f64, f64, f64) = (630.5, 200.0, 352.5);
const TABLE: (i32, i32, i32) = (630, 200, 350);
const BOX: &str = "x=620,y=199,z=342,dx=21,dy=10,dz=21";

/// Player-window slot of inventory slot `i` (0-8 hotbar, 9-35 main).
const fn pw(i: u16) -> u16 {
    if i < 9 { 36 + i } else { i }
}

/// Crafting-table-window slot of inventory slot `i`.
const fn tw(i: u16) -> u16 {
    if i < 9 { 37 + i } else { i + 1 }
}

/// Opens the player window.
fn open_player(cx: &mut Ctx, ev: &mut Vec<String>) -> bool {
    let i = inv(cx, InvOp::Open);
    let ok = i.window.as_ref().is_some_and(|w| w.id == 0 && w.computed_slots == [0]) && i.open_refusal.is_none();
    ev.push(format!("open inventory ok={ok}"));
    ok
}

/// Opens the crafting table by right-clicking it.
fn open_table(cx: &mut Ctx, ev: &mut Vec<String>) -> bool {
    let i = inv(cx, InvOp::OpenContainer(pos(TABLE.0, TABLE.1, TABLE.2), None));
    let ok = i.window.as_ref().is_some_and(|w| w.kind == Some("minecraft:crafting") && w.slots.len() == 46 && w.computed_slots == [0]);
    ev.push(format!("open table {:?} ok={ok}", i.open.as_ref().map(|o| &o.outcome)));
    ok
}

/// Set when the server is Paper: it resyncs the whole window and the offhand after a click on
/// the result (CraftBukkit's `CraftItemEvent` path calls `sendAllDataToRemote`, then
/// `sendOffHandSlotChange`). Vanilla does neither, so on vanilla such a click may update only
/// the slots a row allows.
static PAPER: AtomicBool = AtomicBool::new(false);

/// A click that must be sent and may only update the slots in `allow`.
fn click(cx: &mut Ctx, slot: u16, c: Click, allow: &[u16], ev: &mut Vec<String>) -> (bool, InvInfo) {
    let i = inv(cx, InvOp::Click(Some(slot), c));
    let paper_resync = slot == 0 && PAPER.load(Ordering::Relaxed);
    let player_window = i.window.as_ref().is_some_and(|w| w.id == 0);
    let ok = i.click.as_ref().is_some_and(|o| {
        o.refused.is_none()
            && o.slot == Some(slot)
            && (o.full_resends == 0 || paper_resync)
            && o.corrected.iter().all(|s| s.is_some_and(|s| allow.contains(&s) || (paper_resync && player_window && s == 45)))
    });
    let corrected = i.click.as_ref().map_or(vec![], |o| o.corrected.clone());
    let resends = i.click.as_ref().map_or(0, |o| o.full_resends);
    let carry = i.window.as_ref().map_or("-".into(), |w| short(&w.carried));
    ev.push(format!("{c:?}@{slot} {} (server set {corrected:?}, {resends} resend(s), carry {carry})", if ok { "ok" } else { "FAILED" }));
    (ok, i)
}

/// The result slot the bot sees, as (item, count).
fn result_of(i: &InvInfo) -> Option<(String, i64)> {
    let s = i.window.as_ref()?.slots.first()?.as_ref()?;
    Some((s.item.trim_start_matches("minecraft:").to_string(), s.count as i64))
}

fn want_result(i: &InvInfo, item: &str, count: i64, ev: &mut Vec<String>) -> bool {
    let got = result_of(i);
    let ok = got == Some((item.to_string(), count));
    ev.push(format!("result {got:?}{}", if ok { String::new() } else { format!(" WANTED {item} x{count}") }));
    ok
}

fn want_no_result(i: &InvInfo, ev: &mut Vec<String>) -> bool {
    let got = result_of(i);
    ev.push(format!("result {got:?}"));
    got.is_none()
}

/// What the bot's grid slot `slot` shows.
fn grid_is(i: &InvInfo, slot: usize, want: Option<(&str, i64)>) -> bool {
    let got = i.window.as_ref().and_then(|w| w.slots.get(slot)).and_then(|s| s.as_ref()).map(|s| (s.item.trim_start_matches("minecraft:").to_string(), s.count as i64));
    got == want.map(|(a, b)| (a.to_string(), b))
}

/// Closes the screen, then checks the server's items against `want` (item -> total), the bot's
/// inventory against the server's slot by slot, and that nothing dropped.
fn finish(cx: &mut Ctx, want: &[(&str, i64)], ev: &mut Vec<String>) -> bool {
    inv(cx, InvOp::Close);
    sleep(Duration::from_millis(700));
    let held = server_inventory(cx);
    let mut totals: BTreeMap<String, i64> = BTreeMap::new();
    for h in &held {
        *totals.entry(h.id.trim_start_matches("minecraft:").to_string()).or_default() += h.count;
    }
    let want_map: BTreeMap<String, i64> = want.iter().map(|&(k, v)| (k.to_string(), v)).collect();
    let keep = |s: i64| s <= 35 || s == 40;
    let server: Vec<(i64, String, i64)> = held.iter().filter(|h| keep(h.slot)).map(|h| (h.slot, h.id.clone(), h.count)).collect();
    let status = inv(cx, InvOp::Status);
    let view: Vec<(i64, String, i64)> = view_slots(&status);
    let dropped = cx.r(&format!("execute if entity @e[type=minecraft:item,{BOX}]")).contains("passed");
    ev.push(format!("server items {totals:?}"));
    let mut ok = totals == want_map;
    if !ok {
        ev.push(format!("WANTED {want_map:?}"));
    }
    if view != server {
        ev.push(format!("BOT INVENTORY {view:?} vs SERVER {server:?}"));
        ok = false;
    }
    if dropped {
        ev.push("an item was dropped".into());
        ok = false;
    }
    ok && status.window.is_none()
}

/// A scene with the stock in main inventory slots 9... (`give` is (inventory slot, item, count)).
fn scene(cx: &mut Ctx, table: bool, give: &[(u16, &str, u8)]) {
    cx.scene_in(AREA, &[], SPAWN);
    let n = cx.name.clone();
    if table {
        cx.r(&format!("setblock {} {} {} minecraft:crafting_table", TABLE.0, TABLE.1, TABLE.2));
    }
    for (slot, item, count) in give {
        cx.r(&format!("item replace entity {n} container.{slot} with minecraft:{item} {count}"));
    }
    sleep(Duration::from_millis(500));
}

fn row(cx: &mut Ctx, name: &str, table: bool, give: &[(u16, &str, u8)], body: impl FnOnce(&mut Ctx, &mut Vec<String>) -> bool) -> Outcome {
    scene(cx, table, give);
    cx.run("A11", name, |cx| {
        let mut ev = Vec::new();
        let ok = body(cx, &mut ev);
        (ok, ev.join("; "))
    })
}

// Spec: specs/B3-inventory-crafting.md §4.2 (crafting by clicks; the result is the server's)
pub fn a11(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    // vanilla answers "Unknown or incomplete command" to Paper's own command
    let paper = !cx.r("paper version").contains("Unknown");
    eprintln!("A11: server is {}", if paper { "Paper" } else { "vanilla" });
    PAPER.store(paper, Ordering::Relaxed);

    out.push(row(cx, "planks_from_log_2x2", false, &[(9, "oak_log", 3)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Right, &[0], ev).0;
        let (c, i) = click(cx, pw(9), Click::Left, &[0], ev);
        ok &= c & want_result(&i, "oak_planks", 4, ev);
        ok &= click(cx, 0, Click::Left, &[0], ev).0;
        ok &= click(cx, pw(10), Click::Left, &[], ev).0;
        ok & finish(cx, &[("oak_log", 2), ("oak_planks", 4)], ev)
    }));

    // One plank at a time with right-clicks, stacked down the left column.
    out.push(row(cx, "sticks_2x2_right_clicks", false, &[(9, "oak_planks", 4)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Right, &[0], ev).0;
        let (c, i) = click(cx, 3, Click::Right, &[0], ev);
        ok &= c & want_result(&i, "stick", 4, ev);
        ok &= click(cx, pw(9), Click::Left, &[0], ev).0;
        ok &= click(cx, 0, Click::Left, &[0], ev).0;
        ok &= click(cx, pw(10), Click::Left, &[], ev).0;
        ok & finish(cx, &[("oak_planks", 2), ("stick", 4)], ev)
    }));

    out.push(row(cx, "crafting_table_from_planks", false, &[(9, "oak_planks", 4)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        let mut last = None;
        for g in 1..=4 {
            let (c, i) = click(cx, g, Click::Right, &[0], ev);
            ok &= c;
            last = Some(i);
        }
        ok &= want_result(&last.unwrap(), "crafting_table", 1, ev);
        ok &= click(cx, 0, Click::Left, &[0], ev).0;
        ok &= click(cx, pw(10), Click::Left, &[], ev).0;
        ok & finish(cx, &[("crafting_table", 1)], ev)
    }));

    // 3x3 shape: planks across the top, sticks down the middle, one at a time.
    out.push(row(cx, "wooden_pickaxe_at_table", true, &[(9, "oak_planks", 3), (10, "stick", 2)], |cx, ev| {
        let mut ok = open_table(cx, ev);
        ok &= click(cx, tw(9), Click::Left, &[], ev).0;
        for g in 1..=3 {
            ok &= click(cx, g, Click::Right, &[0], ev).0;
        }
        ok &= click(cx, tw(10), Click::Left, &[0], ev).0;
        ok &= click(cx, 5, Click::Right, &[0], ev).0;
        let (c, i) = click(cx, 8, Click::Right, &[0], ev);
        ok &= c & want_result(&i, "wooden_pickaxe", 1, ev);
        ok &= click(cx, 0, Click::Left, &[0], ev).0;
        ok &= click(cx, tw(11), Click::Left, &[], ev).0;
        ok & finish(cx, &[("wooden_pickaxe", 1)], ev)
    }));

    // The client crafts once; the server crafts until the grid is used up, so it also sets the
    // grid slot and the slot that received the planks (vanilla QUICK_MOVE loop, see the header).
    out.push(row(cx, "shift_click_crafts_all", false, &[(9, "oak_log", 3)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Left, &[0], ev).0;
        let (c, i) = click(cx, 0, Click::ShiftLeft, &[0, 1, pw(8)], ev);
        ok &= c & want_no_result(&i, ev) & grid_is(&i, 1, None);
        ok & finish(cx, &[("oak_planks", 12)], ev)
    }));

    // One craft only: the client's prediction is the server's, so only the result is updated.
    out.push(row(cx, "shift_click_one_craft", false, &[(9, "oak_log", 1)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Left, &[0], ev).0;
        let (c, _) = click(cx, 0, Click::ShiftLeft, &[0], ev);
        ok &= c;
        ok & finish(cx, &[("oak_planks", 4)], ev)
    }));

    out.push(row(cx, "table_shift_click_crafts_all", true, &[(9, "oak_planks", 3), (10, "oak_planks", 3)], |cx, ev| {
        let mut ok = open_table(cx, ev);
        ok &= click(cx, tw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 2, Click::Left, &[0], ev).0;
        ok &= click(cx, tw(10), Click::Left, &[0], ev).0;
        ok &= click(cx, 5, Click::Left, &[0], ev).0;
        let (c, i) = click(cx, 0, Click::ShiftLeft, &[0, 2, 5, 45], ev);
        ok &= c & want_no_result(&i, ev);
        ok & finish(cx, &[("stick", 12)], ev)
    }));

    // Shift-click from the inventory puts the stack in the table's grid (CraftingMenu#quickMoveStack).
    out.push(row(cx, "table_shift_click_into_grid", true, &[(9, "oak_log", 5)], |cx, ev| {
        let mut ok = open_table(cx, ev);
        let (c, i) = click(cx, tw(9), Click::ShiftLeft, &[0], ev);
        ok &= c & grid_is(&i, 1, Some(("oak_log", 5))) & want_result(&i, "oak_planks", 4, ev);
        ok & finish(cx, &[("oak_log", 5)], ev)
    }));

    // Two planks on a diagonal make nothing (side by side they make a pressure plate); a click on the empty result changes nothing.
    out.push(row(cx, "wrong_shape_and_empty_result", false, &[(9, "oak_planks", 2)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Right, &[0], ev).0;
        let (c, i) = click(cx, 4, Click::Right, &[0], ev);
        ok &= c & want_no_result(&i, ev);
        let (c, i) = click(cx, 0, Click::Left, &[], ev);
        ok &= c & want_no_result(&i, ev) & grid_is(&i, 1, Some(("oak_planks", 1))) & grid_is(&i, 4, Some(("oak_planks", 1)));
        ok & finish(cx, &[("oak_planks", 2)], ev)
    }));

    // A cake: the three milk buckets leave three empty buckets in their cells (ResultSlot#onTake
    // with the item defaults); the client predicts them, the server agrees.
    let cake: &[(u16, &str, u8)] = &[(9, "milk_bucket", 1), (10, "milk_bucket", 1), (11, "milk_bucket", 1), (12, "sugar", 2), (13, "egg", 1), (14, "wheat", 3)];
    out.push(row(cx, "cake_leaves_buckets", true, cake, |cx, ev| {
        let mut ok = open_table(cx, ev);
        // milk buckets, sugar and wheat come as stacks or singles: pick up, place one at a time
        for k in 0..3 {
            ok &= click(cx, tw(9 + k), Click::Left, &[0], ev).0;
            ok &= click(cx, 1 + k, Click::Left, &[0], ev).0;
        }
        ok &= click(cx, tw(12), Click::Left, &[0], ev).0;
        ok &= click(cx, 4, Click::Right, &[0], ev).0;
        ok &= click(cx, 6, Click::Right, &[0], ev).0;
        ok &= click(cx, tw(13), Click::Left, &[0], ev).0;
        ok &= click(cx, 5, Click::Left, &[0], ev).0;
        ok &= click(cx, tw(14), Click::Left, &[0], ev).0;
        ok &= click(cx, 7, Click::Right, &[0], ev).0;
        ok &= click(cx, 8, Click::Right, &[0], ev).0;
        let (c, last) = click(cx, 9, Click::Right, &[0], ev);
        ok &= c & want_result(&last, "cake", 1, ev);
        let (c, i) = click(cx, 0, Click::Left, &[0], ev);
        ok &= c;
        ok &= (1..=3).all(|g| grid_is(&i, g, Some(("bucket", 1)))) & (4..=9).all(|g| grid_is(&i, g, None));
        ok &= click(cx, tw(20), Click::Left, &[], ev).0;
        ok & finish(cx, &[("cake", 1), ("bucket", 3)], ev)
    }));

    // Closing with items in the grid returns them (AbstractContainerMenu#clearContainer).
    out.push(row(cx, "close_returns_grid_player", false, &[(9, "oak_planks", 2)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Right, &[0], ev).0;
        ok &= click(cx, 2, Click::Right, &[0], ev).0;
        ok & finish(cx, &[("oak_planks", 2)], ev)
    }));

    out.push(row(cx, "close_returns_grid_table", true, &[(9, "oak_planks", 8)], |cx, ev| {
        let mut ok = open_table(cx, ev);
        ok &= click(cx, tw(9), Click::Left, &[], ev).0;
        ok &= click(cx, 1, Click::Right, &[0], ev).0;
        ok &= click(cx, 5, Click::Right, &[0], ev).0;
        ok &= click(cx, 9, Click::Right, &[0], ev).0;
        // the rest of the stack stays in hand: the server puts it back too
        ok & finish(cx, &[("oak_planks", 8)], ev)
    }));

    // An ingredient with components: the recipe ignores them, and the slot hash must match.
    out.push(row(cx, "renamed_ingredient", false, &[(9, "oak_log[custom_name=\"Fancy\",lore=[\"x\"]]", 1)], |cx, ev| {
        let mut ok = open_player(cx, ev);
        ok &= click(cx, pw(9), Click::Left, &[], ev).0;
        let (c, i) = click(cx, 1, Click::Left, &[0], ev);
        ok &= c & want_result(&i, "oak_planks", 4, ev);
        ok &= click(cx, 0, Click::Left, &[0], ev).0;
        ok &= click(cx, pw(10), Click::Left, &[], ev).0;
        ok & finish(cx, &[("oak_planks", 4)], ev)
    }));

    // Two damaged pickaxes combine (RepairItemRecipe): the result carries `damage`, so moving
    // it checks that component's hash. 59 max; (59-50) + (59-55) + 59*5/100 = 15 left -> damage 44.
    out.push(row(
        cx,
        "repair_damaged_pickaxes",
        false,
        &[(9, "wooden_pickaxe[damage=50]", 1), (10, "wooden_pickaxe[damage=55]", 1)],
        |cx, ev| {
            let mut ok = open_player(cx, ev);
            ok &= click(cx, pw(9), Click::Left, &[], ev).0;
            ok &= click(cx, 1, Click::Left, &[0], ev).0;
            ok &= click(cx, pw(10), Click::Left, &[0], ev).0;
            let (c, i) = click(cx, 2, Click::Left, &[0], ev);
            ok &= c & want_result(&i, "wooden_pickaxe", 1, ev);
            ok &= click(cx, 0, Click::ShiftLeft, &[0], ev).0;
            ok &= finish(cx, &[("wooden_pickaxe", 1)], ev);
            let n = cx.name.clone();
            let has = cx.r(&format!("execute if items entity {n} container.8 minecraft:wooden_pickaxe[minecraft:damage=44]")).contains("passed");
            ev.push(format!("pickaxe with damage 44 in the last hotbar slot: {has}"));
            ok & has
        },
    ));

    cx.clean_in(AREA);
    out
}
