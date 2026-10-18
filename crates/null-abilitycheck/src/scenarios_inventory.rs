//! A9: inventory clicks (pick up, place, split, quick-move, swap, drop, double click) in the
//! player's own window, and a click with each kind of custom item, so a wrong component hash
//! shows as a corrected slot. Plan: specs/plans/ABILITY_CHECK.md. Area: corner (560, 342).
//! Server truth: `data get entity <bot> Inventory` and `equipment`; the server must correct
//! nothing after any click (vanilla sends a slot back only where its result differs from ours).

use std::{thread::sleep, time::Duration};

use null_bot_api::{Click, ItemStack};

use crate::{
    scenarios::{Ctx, Outcome, last_int},
    skill::{Info, Op},
    skill_inventory::{InvInfo, InvOp},
};

const AREA: (i32, i32) = (560, 342);
const SPAWN: (f64, f64, f64) = (570.5, 200.0, 352.5);
const BOX: &str = "x=560,y=199,z=342,dx=21,dy=10,dz=21";

/// Player-window slot of inventory slot `i` (0-8 hotbar, 9-35 main).
const fn w(i: u16) -> Option<u16> {
    Some(if i < 9 { 36 + i } else { i })
}

/// One stack the server holds: inventory slot, item id, count.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Held {
    pub slot: i64,
    pub id: String,
    pub count: i64,
}

/// The quoted string in an RCON reply ("... entity data: \"minecraft:stone\"").
pub(crate) fn quoted(r: &str) -> Option<String> {
    let (_, rest) = r.split_once('"')?;
    Some(rest.split_once('"')?.0.to_string())
}

/// The server's main inventory and hotbar, then armour (36 feet to 39 head) and the offhand (40).
/// Read one field at a time: Paper cuts long `data get` replies short with "...".
pub(crate) fn server_inventory(cx: &mut Ctx) -> Vec<Held> {
    let n = cx.name.clone();
    let mut out = Vec::new();
    for k in 0..41 {
        let path = format!("Inventory[{k}]");
        // An index past the end answers with an error that names the index; stop there.
        let r = cx.r(&format!("data get entity {n} {path}.Slot"));
        let Some(slot) = last_int(&r).filter(|_| r.contains("entity data")) else { break };
        let id = quoted(&cx.r(&format!("data get entity {n} {path}.id"))).unwrap_or_default();
        let count = last_int(&cx.r(&format!("data get entity {n} {path}.count"))).unwrap_or(-1);
        out.push(Held { slot, id, count });
    }
    for (name, slot) in [("feet", 36), ("legs", 37), ("chest", 38), ("head", 39), ("offhand", 40)] {
        let r = cx.r(&format!("data get entity {n} equipment.{name}.id"));
        if let Some(id) = quoted(&r).filter(|_| r.contains("entity data")) {
            let count = last_int(&cx.r(&format!("data get entity {n} equipment.{name}.count"))).unwrap_or(-1);
            out.push(Held { slot, id, count });
        }
    }
    out.sort_by_key(|h| h.slot);
    out
}

pub(crate) fn inv(cx: &mut Ctx, op: InvOp) -> InvInfo {
    match cx.call(Op::Inv(op)).info {
        Info::Inv(i) => i,
        i => panic!("expected inventory info: {i:?}"),
    }
}

pub(crate) fn short(s: &Option<ItemStack>) -> String {
    s.as_ref().map_or("-".into(), |s| format!("{} {}", s.item.trim_start_matches("minecraft:"), s.count))
}

/// The bot's own view as (slot, id, count), in the server's numbering (40 = offhand).
pub(crate) fn view_slots(i: &InvInfo) -> Vec<(i64, String, i64)> {
    let mut v: Vec<(i64, String, i64)> = i
        .inventory
        .iter()
        .enumerate()
        .filter_map(|(k, s)| s.as_ref().map(|s| (k as i64, s.item.clone(), s.count as i64)))
        .collect();
    if let Some(o) = &i.offhand {
        v.push((40, o.item.clone(), o.count as i64));
    }
    v
}

/// What a row expects the server to hold afterwards: (slot, item, count), slots as in
/// `server_inventory` (39 head, 40 offhand).
type Want = &'static [(i64, &'static str, i64)];

struct Case {
    name: &'static str,
    /// (inventory slot, item with components, count), placed with `item replace`.
    give: &'static [(u16, &'static str, u8)],
    clicks: &'static [(Option<u16>, Click)],
    want: Want,
    /// An item entity the clicks should drop: (item, count).
    dropped: Option<(&'static str, i64)>,
}

// Spec: specs/B3-inventory-crafting.md §4.1 (each click as vanilla's `AbstractContainerMenu#doClick`)
const CASES: &[Case] = &[
    Case {
        name: "pickup_and_place",
        give: &[(9, "cobblestone", 64)],
        clicks: &[(w(9), Click::Left), (w(20), Click::Left)],
        want: &[(20, "cobblestone", 64)],
        dropped: None,
    },
    Case {
        name: "split_half",
        give: &[(9, "cobblestone", 63)],
        clicks: &[(w(9), Click::Right), (w(10), Click::Left)],
        want: &[(9, "cobblestone", 31), (10, "cobblestone", 32)],
        dropped: None,
    },
    Case {
        name: "place_one_at_a_time",
        give: &[(9, "dirt", 5)],
        clicks: &[(w(9), Click::Left), (w(10), Click::Right), (w(11), Click::Right), (w(9), Click::Left)],
        want: &[(9, "dirt", 3), (10, "dirt", 1), (11, "dirt", 1)],
        dropped: None,
    },
    Case {
        name: "swap_with_carried",
        give: &[(9, "dirt", 10), (10, "stone", 5)],
        clicks: &[(w(9), Click::Left), (w(10), Click::Left), (w(9), Click::Left)],
        want: &[(9, "stone", 5), (10, "dirt", 10)],
        dropped: None,
    },
    Case {
        name: "merge_onto_a_stack",
        give: &[(9, "stone", 40), (10, "stone", 40)],
        clicks: &[(w(9), Click::Left), (w(10), Click::Left), (w(9), Click::Left)],
        want: &[(9, "stone", 16), (10, "stone", 64)],
        dropped: None,
    },
    Case {
        name: "quick_move_main_to_hotbar",
        give: &[(9, "stone", 32), (0, "stone", 40)],
        clicks: &[(w(9), Click::ShiftLeft)],
        want: &[(0, "stone", 64), (1, "stone", 8)],
        dropped: None,
    },
    Case {
        name: "quick_move_hotbar_to_main",
        give: &[(2, "oak_log", 16)],
        clicks: &[(w(2), Click::ShiftLeft)],
        want: &[(9, "oak_log", 16)],
        dropped: None,
    },
    Case {
        name: "quick_move_armour",
        give: &[(15, "iron_helmet", 1)],
        clicks: &[(w(15), Click::ShiftLeft)],
        want: &[(39, "iron_helmet", 1)],
        dropped: None,
    },
    Case {
        name: "swap_hotbar_key",
        give: &[(12, "dirt", 7), (3, "stone", 2)],
        clicks: &[(w(12), Click::SwapHotbar(3))],
        want: &[(3, "dirt", 7), (12, "stone", 2)],
        dropped: None,
    },
    Case {
        name: "swap_offhand_key",
        give: &[(12, "shield", 1)],
        clicks: &[(w(12), Click::SwapOffhand)],
        want: &[(40, "shield", 1)],
        dropped: None,
    },
    Case {
        name: "drop_one",
        give: &[(9, "dirt", 10)],
        clicks: &[(w(9), Click::Drop)],
        want: &[(9, "dirt", 9)],
        dropped: Some(("dirt", 1)),
    },
    Case {
        name: "drop_stack",
        give: &[(9, "dirt", 10)],
        clicks: &[(w(9), Click::DropStack)],
        want: &[],
        dropped: Some(("dirt", 10)),
    },
    Case {
        name: "drop_carried_outside",
        give: &[(9, "dirt", 10)],
        clicks: &[(w(9), Click::Left), (None, Click::Left)],
        want: &[],
        dropped: Some(("dirt", 10)),
    },
    Case {
        name: "drop_one_carried_outside",
        give: &[(9, "dirt", 10)],
        clicks: &[(w(9), Click::Left), (None, Click::Right), (w(9), Click::Left)],
        want: &[(9, "dirt", 9)],
        dropped: Some(("dirt", 1)),
    },
    Case {
        name: "double_click_gathers",
        give: &[(9, "stone", 10), (10, "stone", 20), (11, "stone", 30), (12, "stone", 64)],
        clicks: &[(w(11), Click::Left), (w(11), Click::DoubleClick), (w(11), Click::Left)],
        // vanilla's second pass takes from full stacks too: 30+10+20, then 4 from the 64
        want: &[(11, "stone", 64), (12, "stone", 60)],
        dropped: None,
    },
];

/// Custom items for the hash check: each is picked up and put down, and the server must not
/// resend its slot. Values a survival player meets, plus the components fork patch 15 fixed.
const HASH_ITEMS: &[(&str, &str)] = &[
    ("shield_slow_blocks_attacks", "shield[minecraft:blocks_attacks={block_delay_seconds:1.0f,\
block_sound:\"minecraft:item.shield.block\",disabled_sound:\"minecraft:item.shield.break\",\
bypassed_by:\"#minecraft:bypasses_shield\",item_damage:{threshold:3.0f,base:1.0f,factor:1.0f}}]"),
    ("pickaxe_custom_tool", "iron_pickaxe[tool={rules:[{blocks:\"#minecraft:mineable/pickaxe\",speed:20f,\
correct_for_drops:true}],default_mining_speed:2f,damage_per_block:2}]"),
    ("sword_damaged_enchanted", "diamond_sword[damage=50,enchantments={\"minecraft:sharpness\":3,\"minecraft:unbreaking\":2}]"),
    ("named_with_lore", "stick[custom_name={text:\"Nul\",color:\"red\"},lore=[\"first line\",{text:\"second\",italic:false}]]"),
    ("repair_cost", "iron_sword[repair_cost=3,custom_name=\"Blade\"]"),
    ("damage_resistant_tag", "stick[damage_resistant={types:\"#minecraft:is_fire\"}]"),
    ("repairable_tag", "stick[repairable={items:\"#minecraft:wooden_tool_materials\"}]"),
    ("potion", "potion[potion_contents={potion:\"minecraft:strength\"}]"),
    ("potion_custom_effects", "potion[potion_contents={custom_color:16711680,custom_effects:[{id:\"minecraft:speed\",amplifier:1,duration:200}]}]"),
    ("tipped_arrow", "tipped_arrow[potion_contents={potion:\"minecraft:poison\"}]"),
    ("dyed_leather", "leather_helmet[dyed_color=16711680]"),
    ("enchanted_book", "enchanted_book[stored_enchantments={\"minecraft:mending\":1}]"),
    ("trimmed_armour", "iron_chestplate[trim={material:\"minecraft:gold\",pattern:\"minecraft:sentry\"}]"),
    ("firework", "firework_rocket[fireworks={flight_duration:2,explosions:[{shape:\"star\",colors:[I;16711680],has_trail:true}]}]"),
    ("written_book", "written_book[written_book_content={title:\"t\",author:\"a\",pages:[\"page one\"]}]"),
    ("writable_book", "writable_book[writable_book_content={pages:[\"draft\"]}]"),
    ("shulker_with_contents", "shulker_box[container=[{slot:0,item:{id:\"minecraft:stone\",count:3}}]]"),
    ("banner_patterns", "white_banner[banner_patterns=[{pattern:\"minecraft:stripe_top\",color:\"red\"}]]"),
    ("shield_with_patterns", "shield[banner_patterns=[{pattern:\"minecraft:cross\",color:\"blue\"}],base_color=\"white\"]"),
    ("attribute_modifiers", "stick[attribute_modifiers=[{type:\"minecraft:attack_damage\",amount:3.0,operation:\"add_value\",\
id:\"null:test\",slot:\"mainhand\"}]]"),
    ("unbreakable", "iron_pickaxe[unbreakable={}]"),
    ("custom_data", "stick[custom_data={null:1b,tag:\"x\"}]"),
    ("food_and_consumable", "stick[food={nutrition:3,saturation:1.2f},consumable={consume_seconds:2.0f}]"),
    ("max_stack_size", "stick[max_stack_size=16]"),
    ("glint_and_rarity", "stick[enchantment_glint_override=true,rarity=\"epic\"]"),
    ("suspicious_stew", "suspicious_stew[suspicious_stew_effects=[{id:\"minecraft:night_vision\",duration:100}]]"),
    ("goat_horn", "goat_horn[instrument=\"minecraft:ponder_goat_horn\"]"),
    ("lodestone_compass", "compass[lodestone_tracker={target:{pos:[I;1,64,1],dimension:\"minecraft:overworld\"},tracked:false}]"),
    ("charged_crossbow", "crossbow[charged_projectiles=[{id:\"minecraft:arrow\",count:1}]]"),
    ("can_break", "iron_pickaxe[can_break={blocks:\"minecraft:stone\"}]"),
    ("tooltip_display", "stick[tooltip_display={hidden_components:[\"minecraft:enchantments\"]}]"),
];

/// Item entities in the area as (item, count).
fn dropped(cx: &mut Ctx) -> Vec<(String, i64)> {
    let mut out = Vec::new();
    for _ in 0..8 {
        let e = format!("@e[type=minecraft:item,{BOX},limit=1]");
        let r = cx.r(&format!("data get entity {e} Item.id"));
        let Some(id) = quoted(&r).filter(|_| r.contains("entity data")) else { break };
        let count = last_int(&cx.r(&format!("data get entity {e} Item.count"))).unwrap_or(-1);
        out.push((id.trim_start_matches("minecraft:").to_string(), count));
        cx.r(&format!("kill {e}"));
    }
    out
}

/// Opens the window, makes the clicks, closes it; returns the evidence and whether every click
/// went out uncorrected (and the window opened and closed cleanly).
fn click_through(cx: &mut Ctx, clicks: &[(Option<u16>, Click)]) -> (bool, Vec<String>) {
    let mut ev = Vec::new();
    let open = inv(cx, InvOp::Open);
    let mut ok = open.window.is_some() && open.open_refusal.is_none();
    if !ok {
        ev.push(format!("open failed (window {:?}, refusal {:?})", open.window.as_ref().map(|w| w.id), open.open_refusal));
    }
    for (slot, click) in clicks {
        let i = inv(cx, InvOp::Click(*slot, *click));
        let outcome = i.click.clone();
        let clean = outcome.as_ref().is_some_and(|o| o.uncorrected() && o.slot == *slot);
        ok &= clean;
        let carried = i.window.as_ref().map_or("-".into(), |w| short(&w.carried));
        ev.push(match outcome {
            Some(_) if clean => format!("{click:?}@{slot:?} ok (carry {carried})"),
            Some(o) => format!(
                "{click:?}@{slot:?} refused {:?}, corrected {:?}, {} full resend(s)",
                o.refused, o.corrected, o.full_resends
            ),
            None => format!("{click:?}@{slot:?}: no click result"),
        });
    }
    let closed = inv(cx, InvOp::Close);
    if closed.window.is_some() {
        ok = false;
        ev.push("window still open after close".into());
    }
    (ok, ev)
}

// Spec: specs/B3-inventory-crafting.md §4.1 (move items in the inventory, proven by the server)
pub fn a9(cx: &mut Ctx) -> Vec<Outcome> {
    let n = cx.name.clone();
    let mut out = Vec::new();
    for case in CASES {
        cx.scene_in(AREA, &[], SPAWN);
        for (slot, item, count) in case.give {
            cx.r(&format!("item replace entity {n} container.{slot} with minecraft:{item} {count}"));
        }
        sleep(Duration::from_millis(400));
        out.push(cx.run("A9", case.name, |cx| {
            let (clicks_ok, mut ev) = click_through(cx, case.clicks);
            sleep(Duration::from_millis(200));
            let server: Vec<(i64, String, i64)> = server_inventory(cx)
                .into_iter()
                .map(|h| (h.slot, h.id.trim_start_matches("minecraft:").to_string(), h.count))
                .collect();
            let want: Vec<(i64, String, i64)> = case.want.iter().map(|&(s, i, c)| (s, i.to_string(), c)).collect();
            let view: Vec<(i64, String, i64)> = view_slots(&inv(cx, InvOp::Status))
                .into_iter()
                .map(|(s, i, c)| (s, i.trim_start_matches("minecraft:").to_string(), c))
                .filter(|(s, ..)| *s <= 35 || *s == 40)
                .collect();
            let server_main: Vec<_> = server.iter().filter(|(s, ..)| *s <= 35 || *s == 40).cloned().collect();
            let drops = dropped(cx);
            let drop_ok = match case.dropped {
                Some((item, count)) => drops == vec![(item.to_string(), count)],
                None => drops.is_empty(),
            };
            let ok = clicks_ok && server == want && view == server_main && drop_ok;
            ev.push(format!("server {server:?}"));
            if server != want {
                ev.push(format!("WANTED {want:?}"));
            }
            if view != server_main {
                ev.push(format!("BOT VIEW {view:?}"));
            }
            ev.push(format!("dropped {drops:?}"));
            (ok, ev.join("; "))
        }));
    }
    for (name, item) in HASH_ITEMS {
        cx.scene_in(AREA, &[], SPAWN);
        let given = cx.r(&format!("item replace entity {n} container.9 with minecraft:{item} 1"));
        sleep(Duration::from_millis(400));
        out.push(cx.run("A9", &format!("hash_{name}"), |cx| {
            // `if items` with the give string matches those components exactly (vanilla item
            // predicates), so the item must arrive in slot 20 unchanged.
            let has = |cx: &mut Ctx, slot: u16| cx.r(&format!("execute if items entity {n} container.{slot} minecraft:{item}")).contains("passed");
            if !has(cx, 9) {
                return (false, format!("RCON gave nothing: {given}"));
            }
            let (clicks_ok, mut ev) = click_through(cx, &[(w(9), Click::Left), (w(20), Click::Left)]);
            sleep(Duration::from_millis(200));
            let after = server_inventory(cx);
            let same = has(cx, 20) && after.len() == 1;
            if !same {
                ev.push(format!("server after: {after:?}"));
            }
            (clicks_ok && same, ev.join("; "))
        }));
    }
    cx.clean_in(AREA);
    out
}
