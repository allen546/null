//! A10E: open storage on entities (chest minecart, hopper minecart, chest boat) by right-clicking
//! the entity, read it, move items by clicks, close it. Plan: specs/plans/ABILITY_CHECK.md. Area:
//! corner (740, 342). Server truth: `data get entity @e[tag=a10e,limit=1] Items[k]` field by field,
//! and the bot's inventory over RCON; the server must correct no slot.

use std::{thread::sleep, time::Duration};

use null_bot_api::{Click, OpenOutcome, OpenTarget};

use crate::{
    scenarios::{Ctx, Outcome},
    scenarios_containers::{Slots, Src, click, close, container_agrees, inventory_agrees, iw, s, server_container, status},
    scenarios_inventory::inv,
    skill::Op,
    skill_inventory::{Aim, InvInfo, InvOp},
};

const AREA: (i32, i32) = (740, 342);
/// The bot stands two blocks south of the entity, feet on the floor, facing south until it aims.
const SPAWN: (f64, f64, f64) = (750.5, 200.0, 352.5);
/// The entity's position: the middle of a block, on the floor.
const E: (f64, f64, f64) = (750.5, 200.0, 350.5);
/// The tag every entity of this module carries, so `@e[tag=a10e,limit=1]` is always the one.
const SEL: &str = "@e[tag=a10e,limit=1]";

/// One kind of storage entity: how to summon it and what its window looks like.
struct Ent {
    /// The summon id and the `BotView` kind.
    id: &'static str,
    menu: &'static str,
    size: u16,
    /// A chest boat opens only for a crouching player: the bot crouches for it.
    auto_crouch: bool,
    /// Contents for the rows: slot, item, count. The second stack is the one the rows take out.
    basic: &'static [(u8, &'static str, u8)],
    /// For the exact-counts row: container slots for stone, apple and diamond; slot for the logs.
    exact: [u8; 3],
    log_slot: u8,
    /// A container slot that is empty in `basic`.
    free: u8,
}

const MINECART: Ent = Ent {
    id: "chest_minecart",
    menu: "minecraft:generic_9x3",
    size: 27,
    auto_crouch: false,
    basic: &[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)],
    exact: [0, 5, 13],
    log_slot: 26,
    free: 9,
};
const HOPPER: Ent = Ent {
    id: "hopper_minecart",
    menu: "minecraft:hopper",
    size: 5,
    auto_crouch: false,
    basic: &[(0, "stone", 5), (2, "diamond", 3), (4, "apple", 1)],
    exact: [0, 1, 3],
    log_slot: 4,
    free: 1,
};
const BOAT: Ent = Ent {
    id: "oak_chest_boat",
    menu: "minecraft:generic_9x3",
    size: 27,
    auto_crouch: true,
    basic: &[(0, "stone", 5), (4, "diamond", 3), (13, "oak_log", 64), (26, "apple", 1)],
    exact: [0, 5, 13],
    log_slot: 26,
    free: 9,
};

impl Ent {
    fn kind(&self) -> &'static str {
        match self.id {
            "chest_minecart" => "minecraft:chest_minecart",
            "hopper_minecart" => "minecraft:hopper_minecart",
            "oak_chest_boat" => "minecraft:oak_chest_boat",
            _ => "minecraft:minecart",
        }
    }

    fn src(&self) -> Src {
        (format!("entity {SEL}"), "Items")
    }

    fn slots(&self) -> Slots {
        self.basic.iter().map(|&(k, i, c)| (k as i64, i.to_string(), c as i64)).collect()
    }
}

/// `Items:[...]` for a summon.
fn items_list(items: &[(u8, &str, u8)]) -> String {
    let parts: Vec<String> =
        items.iter().map(|(k, i, c)| format!("{{Slot:{k}b,id:\"minecraft:{i}\",count:{c}}}")).collect();
    format!("Items:[{}]", parts.join(","))
}

/// A fresh scene with one entity summoned at `E` by RCON (fixed position and rotation, no gravity,
/// nothing above it) and the bot seeing it.
fn scene(cx: &mut Ctx, id: &str, kind: &'static str, items: &[(u8, &str, u8)], give: &[(u16, &str, u8)]) {
    cx.r(&format!("kill @e[tag=a10e]"));
    cx.scene_in(AREA, &[], SPAWN);
    let n = cx.name.clone();
    for (slot, item, count) in give {
        cx.r(&format!("item replace entity {n} container.{slot} with minecraft:{item} {count}"));
    }
    cx.r(&format!(
        "summon minecraft:{id} {} {} {} {{NoGravity:1b,Tags:[\"a10e\"],Rotation:[90f,0f],{}}}",
        E.0,
        E.1,
        E.2,
        items_list(items)
    ));
    let seen = inv(cx, InvOp::WaitEntity(kind)).seen;
    if seen.is_none() {
        eprintln!("warning: the bot never saw the {kind}");
    }
    sleep(Duration::from_millis(500));
}

fn open_entity(cx: &mut Ctx, e: &Ent, aim: Aim) -> InvInfo {
    inv(cx, InvOp::OpenEntity { kind: e.kind(), auto_crouch: e.auto_crouch, aim })
}

/// Opens the entity; fails the row unless the menu kind and size are as expected and the result
/// names the entity the bot saw.
fn open_expect(cx: &mut Ctx, e: &Ent, ev: &mut Vec<String>) -> (bool, InvInfo) {
    let i = open_entity(cx, e, Aim::Entity);
    let w = i.window.clone();
    let target = i.open.as_ref().map(|o| o.target);
    let opened = matches!(i.open.as_ref().map(|o| &o.outcome), Some(OpenOutcome::Opened { kind: k, window_id }) if *k == e.menu && Some(*window_id) == w.as_ref().map(|w| w.id));
    let named = matches!((target, i.seen), (Some(OpenTarget::Entity(a)), Some((b, _))) if a == b);
    let sized = w.as_ref().is_some_and(|w| w.kind == Some(e.menu) && w.id != 0 && w.slots.len() == e.size as usize + 36);
    ev.push(format!(
        "open {:?} on {:?}, window {:?}/{} slots, standing again {}",
        i.open.as_ref().map(|o| &o.outcome),
        i.seen.map(|(id, _)| id.0),
        w.as_ref().and_then(|w| w.kind),
        w.as_ref().map_or(0, |w| w.slots.len()),
        !i.sneaking
    ));
    (opened && named && sized && !i.sneaking, i)
}

fn row(
    cx: &mut Ctx,
    name: &str,
    e: &Ent,
    items: &[(u8, &str, u8)],
    give: &[(u16, &str, u8)],
    body: impl FnOnce(&mut Ctx, &mut Vec<String>) -> bool,
) -> Outcome {
    scene(cx, e.id, e.kind(), items, give);
    cx.run("A10E", name, |cx| {
        let mut ev = Vec::new();
        let ok = body(cx, &mut ev);
        (ok, ev.join("; "))
    })
}

/// Whether the entity has a passenger (a standing click on a boat would have mounted it).
fn has_passenger(cx: &mut Ctx) -> bool {
    cx.r(&format!("data get entity {SEL} Passengers")).contains("has the following")
}

fn entity_alive(cx: &mut Ctx) -> bool {
    cx.r(&format!("data get entity {SEL} Pos")).contains("has the following")
}

/// A refused open: the result says so with a reason containing `why`, no window opened, and the
/// entity is unchanged on the server (nothing reached it).
fn refused(cx: &mut Ctx, e: &Ent, i: &InvInfo, why: &str, ev: &mut Vec<String>) -> bool {
    let o = i.open.clone();
    ev.push(format!("{o:?}"));
    let said = o.as_ref().is_some_and(|o| o.outcome == OpenOutcome::Refused && o.refused.is_some_and(|r| r.contains(why)));
    let same = server_container(cx, &e.src()) == e.slots() && !has_passenger(cx);
    said && i.window.is_none() && same
}

fn go(cx: &mut Ctx, at: &str) {
    let n = cx.name.clone();
    cx.r(&format!("tp {n} {at}"));
    sleep(Duration::from_millis(800));
}

// Spec: specs/B3-inventory-crafting.md §4.7, §4.8 (open, move by clicks, read exact contents, close)
fn rows_for(cx: &mut Ctx, e: &Ent, tag: &str, out: &mut Vec<Outcome>) {
    let n = |s: &str| format!("{tag}_{s}");
    let size = e.size;
    let basic = e.basic;

    out.push(row(cx, &n("open_and_read"), e, basic, &[(9, "cobblestone", 7)], |cx, ev| {
        let (ok, i) = open_expect(cx, e, ev);
        let a = container_agrees(cx, &[e.src()], &e.slots(), &i, ev);
        let c = close(cx, ev);
        ok & a & c & inventory_agrees(cx, ev)
    }));

    // Take the first stack by clicks: pick it up, put it in a hotbar slot.
    out.push(row(cx, &n("take_by_clicks"), e, basic, &[], |cx, ev| {
        let (mut ok, _) = open_expect(cx, e, ev);
        ok &= click(cx, Some(0), Click::Left, ev);
        ok &= click(cx, iw(size, 0), Click::Left, ev);
        let i = status(cx);
        let want: Slots = e.slots().into_iter().filter(|(k, ..)| *k != 0).collect();
        ok &= container_agrees(cx, &[e.src()], &want, &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, &n("put_by_clicks"), e, basic, &[(9, "cobblestone", 20)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, e, ev);
        ok &= click(cx, iw(size, 9), Click::Left, ev);
        ok &= click(cx, Some(e.free as u16), Click::Left, ev);
        let i = status(cx);
        let mut want = e.slots();
        want.push((e.free as i64, "cobblestone".into(), 20));
        want.sort();
        ok &= container_agrees(cx, &[e.src()], &want, &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // `ChestMenu#quickMoveStack` / `HopperMenu#quickMoveStack`: out fills the player's slots from the
    // last backwards, in fills the container from the front, after merging into same-item stacks.
    out.push(row(cx, &n("quick_move_out_and_in"), e, basic, &[(10, "dirt", 8)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, e, ev);
        let second = basic[1].0 as u16;
        ok &= click(cx, Some(second), Click::ShiftLeft, ev);
        ok &= click(cx, iw(size, 10), Click::ShiftLeft, ev);
        let i = status(cx);
        let mut want: Slots = e.slots().into_iter().filter(|(k, ..)| *k != second as i64).collect();
        want.push((1, "dirt".into(), 8));
        want.sort();
        ok &= container_agrees(cx, &[e.src()], &want, &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // B3 §4.8: exact counts in exact slots, read back from the bot and from the server.
    out.push(row(cx, &n("exact_counts_in_exact_slots"), e, &[], &[(9, "stone", 64), (10, "apple", 13), (11, "oak_log", 7), (12, "diamond", 1)], |cx, ev| {
        let (mut ok, _) = open_expect(cx, e, ev);
        for (from, to) in [(9u16, e.exact[0]), (10, e.exact[1]), (12, e.exact[2])] {
            ok &= click(cx, iw(size, from), Click::Left, ev);
            ok &= click(cx, Some(to as u16), Click::Left, ev);
        }
        ok &= click(cx, iw(size, 11), Click::Left, ev);
        for _ in 0..3 {
            ok &= click(cx, Some(e.log_slot as u16), Click::Right, ev);
        }
        ok &= click(cx, iw(size, 11), Click::Left, ev);
        let i = status(cx);
        let mut want = s(&[
            (e.exact[0] as i64, "stone", 64),
            (e.exact[1] as i64, "apple", 13),
            (e.exact[2] as i64, "diamond", 1),
            (e.log_slot as i64, "oak_log", 3),
        ]);
        want.sort();
        ok &= container_agrees(cx, &[e.src()], &want, &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // Reach is measured to the hitbox, so a click from far away is refused on the client and
    // nothing is sent: the server's entity is as it was.
    out.push(row(cx, &n("out_of_reach_refused"), e, basic, &[], |cx, ev| {
        go(cx, "750.5 200 358.5 0 0");
        let i = open_entity(cx, e, Aim::Entity);
        refused(cx, e, &i, "out of reach", ev)
    }));
}

// Spec: specs/B3-inventory-crafting.md §4.7 (open a container on an entity by right-clicking it)
pub fn a10e(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    rows_for(cx, &MINECART, "chest_minecart", &mut out);
    rows_for(cx, &HOPPER, "hopper_minecart", &mut out);
    rows_for(cx, &BOAT, "chest_boat", &mut out);

    // The hitbox of a chest minecart (0.98 x 0.7) at the entity's position: its near face is 0.49
    // closer than the position. From 2.7 blocks of the face the eye is 2.85 from the hitbox but 3.58
    // from the position: a click that is in reach only when measured to the hitbox.
    let near = |cx: &mut Ctx, dz: f64, ev: &mut Vec<String>| {
        let z = E.2 + 0.49 + dz;
        go(cx, &format!("{} 200 {z} 0 0", E.0));
        let i = open_entity(cx, &MINECART, Aim::Entity);
        let eye = (E.0, 201.62, z);
        let to_pos = ((eye.0 - E.0).powi(2) + (eye.1 - E.1).powi(2) + (eye.2 - E.2).powi(2)).sqrt();
        let to_box = ((eye.1 - (E.1 + 0.7)).powi(2) + (eye.2 - (E.2 + 0.49)).powi(2)).sqrt();
        ev.push(format!("eye to the entity's position {to_pos:.2}, to its hitbox {to_box:.2}"));
        i
    };
    out.push(row(cx, "chest_minecart_reach_to_the_hitbox_opens", &MINECART, MINECART.basic, &[], |cx, ev| {
        let i = near(cx, 2.7, ev);
        let w = i.window.clone();
        let opened = matches!(i.open.as_ref().map(|o| &o.outcome), Some(OpenOutcome::Opened { .. }));
        ev.push(format!("{:?}, window {:?}", i.open.as_ref().map(|o| &o.outcome), w.as_ref().and_then(|w| w.kind)));
        let a = container_agrees(cx, &[MINECART.src()], &MINECART.slots(), &i, ev);
        let c = close(cx, ev);
        opened & a & c & inventory_agrees(cx, ev)
    }));
    out.push(row(cx, "chest_minecart_just_past_reach_refused", &MINECART, MINECART.basic, &[], |cx, ev| {
        let i = near(cx, 3.1, ev);
        refused(cx, &MINECART, &i, "out of reach", ev)
    }));

    // A wall two blocks high between the bot and the entity: the crosshair ray hits the wall.
    out.push(row(cx, "chest_minecart_behind_a_block_refused", &MINECART, MINECART.basic, &[], |cx, ev| {
        cx.r("fill 749 200 351 751 201 351 minecraft:stone");
        sleep(Duration::from_millis(600));
        // from the eye (750.5, 201.62, 352.5) to the hitbox centre (750.5, 200.35, 350.5)
        let i = open_entity(cx, &MINECART, Aim::Look(180.0, 32.4));
        let r = refused(cx, &MINECART, &i, "block is in the way", ev);
        // `look_at_entity` finds no point either: the wall hides the whole box
        let j = open_entity(cx, &MINECART, Aim::Entity);
        let again = j.open.as_ref().is_some_and(|o| o.outcome == OpenOutcome::Refused);
        ev.push(format!("aimed by the bot: {:?}", j.open.as_ref().and_then(|o| o.refused)));
        cx.r("fill 749 200 351 751 201 351 minecraft:air");
        sleep(Duration::from_millis(600));
        // with the wall gone the same click opens it
        let (ok, _) = open_expect(cx, &MINECART, ev);
        let c = close(cx, ev);
        r & again & ok & c & inventory_agrees(cx, ev)
    }));

    out.push(row(cx, "chest_minecart_crosshair_elsewhere_refused", &MINECART, MINECART.basic, &[], |cx, ev| {
        // the bot faces south, away from the entity, and does not aim
        let i = open_entity(cx, &MINECART, Aim::Keep);
        refused(cx, &MINECART, &i, "crosshair does not hit it", ev)
    }));

    // Vanilla `AbstractBoat#interact`: a standing click rides the boat. Null does not ride.
    let boat_standing = Ent { auto_crouch: false, ..BOAT };
    out.push(row(cx, "chest_boat_standing_click_refused", &BOAT, BOAT.basic, &[], |cx, ev| {
        let i = open_entity(cx, &boat_standing, Aim::Entity);
        let r = refused(cx, &BOAT, &i, "ride", ev);
        // and the same click with auto_crouch opens it, and the bot stands again
        let (ok, _) = open_expect(cx, &BOAT, ev);
        let st = status(cx);
        let a = container_agrees(cx, &[BOAT.src()], &BOAT.slots(), &st, ev);
        let c = close(cx, ev);
        let seated = has_passenger(cx);
        ev.push(format!("passenger on the boat: {seated}"));
        r & ok & a & c & !seated & inventory_agrees(cx, ev)
    }));

    // Crouching already (empty hands): no auto_crouch needed, and it stays crouched.
    out.push(row(cx, "chest_boat_already_crouching_opens", &BOAT, BOAT.basic, &[], |cx, ev| {
        cx.call(Op::Sneak(true));
        sleep(Duration::from_millis(500));
        let i = open_entity(cx, &boat_standing, Aim::Entity);
        let opened = matches!(i.open.as_ref().map(|o| &o.outcome), Some(OpenOutcome::Opened { .. }));
        ev.push(format!("{:?}, crouching {}", i.open.as_ref().map(|o| &o.outcome), i.sneaking));
        let c = close(cx, ev);
        cx.call(Op::Sneak(false));
        sleep(Duration::from_millis(500));
        opened & c & !has_passenger(cx) & inventory_agrees(cx, ev)
    }));

    // A plain minecart is no storage: a standing click rides it, which Null never sends.
    out.push(row(cx, "plain_minecart_standing_click_refused", &MINECART, &[], &[], |cx, ev| {
        cx.r("kill @e[tag=a10e]");
        cx.r(&format!("summon minecraft:minecart {} {} {} {{NoGravity:1b,Tags:[\"a10e\"]}}", E.0, E.1, E.2));
        let seen = inv(cx, InvOp::WaitEntity("minecraft:minecart")).seen.is_some();
        sleep(Duration::from_millis(400));
        let i = inv(cx, InvOp::OpenEntity { kind: "minecraft:minecart", auto_crouch: false, aim: Aim::Entity });
        let o = i.open.clone();
        ev.push(format!("{o:?}"));
        let said = o.as_ref().is_some_and(|o| o.outcome == OpenOutcome::Refused && o.refused.is_some_and(|r| r.contains("ride")));
        seen & said & i.window.is_none() & !has_passenger(cx)
    }));

    // The server changes the contents while the window is open: the bot's window follows.
    out.push(row(cx, "chest_minecart_server_change_while_open", &MINECART, MINECART.basic, &[], |cx, ev| {
        let (mut ok, _) = open_expect(cx, &MINECART, ev);
        cx.r(&format!("item replace entity {SEL} container.7 with minecraft:gold_ingot 9"));
        cx.r(&format!("item replace entity {SEL} container.0 with minecraft:air"));
        sleep(Duration::from_millis(600));
        let i = status(cx);
        ok &= container_agrees(cx, &[MINECART.src()], &s(&[(4, "diamond", 3), (7, "gold_ingot", 9), (13, "oak_log", 64), (26, "apple", 1)]), &i, ev);
        ok &= close(cx, ev);
        ok & inventory_agrees(cx, ev)
    }));

    // The entity dies while the window is open: the server closes the window
    // (`AbstractContainerMenu#stillValid`) and the bot's window goes with it.
    for (name, e) in [("chest_minecart_killed_while_open", &MINECART), ("chest_boat_killed_while_open", &BOAT)] {
        out.push(row(cx, name, e, e.basic, &[], |cx, ev| {
            let (ok, _) = open_expect(cx, e, ev);
            cx.r(&format!("kill {SEL}"));
            sleep(Duration::from_millis(800));
            let i = status(cx);
            let gone = !entity_alive(cx);
            ev.push(format!("entity gone {gone}, bot window after the kill {:?}", i.window.as_ref().map(|w| (w.id, w.kind))));
            // the player window (id 0) is back; a click now is refused as a click in a closed window
            ok & gone & i.window.is_none() & inventory_agrees(cx, ev)
        }));
    }

    cx.r("kill @e[tag=a10e]");
    cx.clean_in(AREA);
    out
}
