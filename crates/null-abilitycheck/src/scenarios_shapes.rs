//! Blocks whose shape is not a 1x1x1 cube, clicked, placed and broken (milestone SA, A4S and A2S).
//! Plan: specs/plans/ABILITY_CHECK.md. Every row passes only with the server's state, the bot's
//! outcome and zero Grim alerts for this account. Vanilla's answer for each row is written in the
//! row (target cell, block state), worked out from the block's `canBeReplaced` and
//! `getStateForPlacement`, not read from the bot.

use std::{thread::sleep, time::Duration};

use null_bot_api::{BlockOutcome, BlockPos, Face, Vec3};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, pos, ticks_between_packets, vanilla_ticks},
    skill::{Info, Op},
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (500, 342);
/// The bot stands here, facing south; the block under test is `T`, three blocks ahead.
const STAND: (f64, f64, f64) = (510.5, 200.0, 348.5);
const T: (i32, i32, i32) = (510, 200, 351);

/// (dx, dy, dz) from `T`, and a block with its state, as `setblock` and `execute if block` take it.
type Blk = (i32, i32, i32, &'static str);

fn at(d: (i32, i32, i32)) -> BlockPos {
    pos(T.0 + d.0, T.1 + d.1, T.2 + d.2)
}

/// `SHAPES_ONLY=text` runs only the rows whose name contains it (for rerunning a failed row).
fn wanted(name: &str) -> bool {
    std::env::var("SHAPES_ONLY").map_or(true, |t| name.contains(&t))
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// Clears the working volume (floor back to stone, air above, no drops) and the held item.
fn reset(cx: &mut Ctx) {
    let n = cx.name.clone();
    for c in [
        format!("fill 507 {} 346 514 {} 356 minecraft:air", FLOOR_Y + 1, FLOOR_Y + 7),
        format!("fill 507 {FLOOR_Y} 346 514 {FLOOR_Y} 356 minecraft:stone"),
        "kill @e[type=minecraft:item,x=500,y=199,z=342,dx=21,dy=10,dz=21]".to_string(),
        format!("clear {n}"),
    ] {
        cx.r(&c);
    }
}

fn give(cx: &mut Ctx, item: Option<&str>) {
    if let Some(i) = item {
        let n = cx.name.clone();
        cx.r(&format!("item replace entity {n} hotbar.0 with minecraft:{i} 1"));
    }
    // the client must hold the item before it clicks
    sleep(Duration::from_millis(250));
}

/// Sets the blocks in order (supports first) and waits until the bot sees each.
fn set_up(cx: &mut Ctx, setup: &[Blk]) {
    for (dx, dy, dz, b) in setup {
        let p = at((*dx, *dy, *dz));
        cx.r(&format!("setblock {} {} {} minecraft:{b}", p.x, p.y, p.z));
    }
    for (dx, dy, dz, b) in setup {
        cx.wait_block(at((*dx, *dy, *dz)), leak(format!("minecraft:{}", b.split('[').next().unwrap())));
    }
    sleep(Duration::from_millis(150));
}

/// What vanilla does with the click.
#[derive(Clone, Copy)]
enum Res {
    /// The item's block appears (the `blocks` below hold).
    Placed,
    /// A legal click the server refuses or answers with an interaction; nothing is placed.
    ServerRefused,
    /// A click no vanilla client can make: the bot refuses it itself and sends nothing. The
    /// reason text must match when given.
    ClientRefused(Option<&'static str>),
}

/// One click: set up, hold `item`, aim at `face` of the block at `click`, click.
#[derive(Clone, Copy)]
struct Row {
    name: &'static str,
    setup: &'static [Blk],
    item: &'static str,
    click: (i32, i32, i32),
    face: Face,
    sneak: bool,
    /// Aim this far from the face centre (a hit point in the upper half, say); None aims by the
    /// shape (`look_at_block`).
    aim: Option<(f64, f64, f64)>,
    res: Res,
    /// Block states that must hold on the server afterwards.
    blocks: &'static [Blk],
}

const R: Row = Row {
    name: "",
    setup: &[],
    item: "stone",
    click: (0, 0, 0),
    face: Face::Up,
    sneak: false,
    aim: None,
    res: Res::Placed,
    blocks: &[(0, 1, 0, "stone")],
};

/// Stone on the side (north) face; it lands north of the clicked block.
const SIDE: &[Blk] = &[(0, 0, -1, "stone")];
/// Stone replaces the clicked block itself.
const INTO: &[Blk] = &[(0, 0, 0, "stone")];
const GRASS: Blk = (0, -1, 0, "grass_block");

macro_rules! rows {
    ($($r:expr),* $(,)?) => { &[$($r),*] };
}

/// "Click on it": a block is placed against the shape; vanilla's target is in `blocks`.
/// Blocks that use the click (doors, buttons, chests, ...) are clicked while sneaking, as a
/// player does to place against them.
const CLICKS: &[Row] = rows![
    // thin or low
    // A lone dot of wire toggles to a cross on a click (vanilla); a player sneaks to place on it.
    Row { name: "click_redstone_wire_line_top", setup: &[(0, 0, 0, "redstone_wire[north=side,south=side,east=none,west=none]")], ..R },
    Row { name: "click_redstone_wire_dot_sneaking_top", setup: &[(0, 0, 0, "redstone_wire")], sneak: true, ..R },
    Row { name: "click_redstone_wire_dot_toggles_to_cross", setup: &[(0, 0, 0, "redstone_wire")], res: Res::ServerRefused, blocks: &[(0, 0, 0, "redstone_wire[north=side,south=side,east=side,west=side]"), (0, 1, 0, "air")], ..R },
    Row { name: "click_carpet_top", setup: &[(0, 0, 0, "white_carpet")], ..R },
    Row { name: "click_carpet_side", setup: &[(0, 0, 0, "white_carpet")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_rail_top", setup: &[(0, 0, 0, "rail")], ..R },
    Row { name: "click_pressure_plate_top", setup: &[(0, 0, 0, "stone_pressure_plate")], ..R },
    Row { name: "click_lily_pad_on_water_top", setup: &[(0, -1, 0, "water"), (0, 0, 0, "lily_pad")], ..R },
    Row { name: "click_snow_1_replaced", setup: &[(0, 0, 0, "snow[layers=1]")], blocks: INTO, ..R },
    Row { name: "click_snow_3_not_replaced_by_stone", setup: &[(0, 0, 0, "snow[layers=3]")], ..R },
    Row { name: "click_snow_8_top", setup: &[(0, 0, 0, "snow[layers=8]")], ..R },
    Row { name: "click_snow_3_with_snow_adds_a_layer", setup: &[(0, 0, 0, "snow[layers=3]")], item: "snow", blocks: &[(0, 0, 0, "snow[layers=4]")], ..R },
    Row { name: "click_repeater_top", setup: &[(0, 0, 0, "repeater")], sneak: true, ..R },
    Row { name: "click_daylight_detector_top", setup: &[(0, 0, 0, "daylight_detector")], sneak: true, ..R },
    // posts and connectors
    Row { name: "click_fence_top", setup: &[(0, 0, 0, "oak_fence")], ..R },
    Row { name: "click_fence_side", setup: &[(0, 0, 0, "oak_fence")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_wall_top", setup: &[(0, 0, 0, "cobblestone_wall")], ..R },
    Row { name: "click_wall_side", setup: &[(0, 0, 0, "cobblestone_wall")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_iron_bars_top", setup: &[(0, 0, 0, "iron_bars")], ..R },
    Row { name: "click_iron_bars_side", setup: &[(0, 0, 0, "iron_bars")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_glass_pane_top", setup: &[(0, 0, 0, "glass_pane")], ..R },
    Row { name: "click_chain_y_top", setup: &[(0, 0, 0, "iron_chain[axis=y]")], ..R },
    Row { name: "click_chain_x_top", setup: &[(0, 0, 0, "iron_chain[axis=x]")], ..R },
    Row { name: "click_chain_z_top", setup: &[(0, 0, 0, "iron_chain[axis=z]")], ..R },
    Row { name: "click_chain_x_side", setup: &[(0, 0, 0, "iron_chain[axis=x]")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_chain_z_end", setup: &[(0, 0, 0, "iron_chain[axis=z]")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_end_rod_top", setup: &[(0, 0, 0, "end_rod[facing=up]")], ..R },
    Row { name: "click_lightning_rod_top", setup: &[(0, 0, 0, "lightning_rod[facing=up]")], ..R },
    // partial heights
    Row { name: "click_slab_bottom_top", setup: &[(0, 0, 0, "oak_slab[type=bottom]")], ..R },
    Row { name: "click_slab_top_top", setup: &[(0, 0, 0, "oak_slab[type=top]")], ..R },
    Row { name: "click_slab_double_top", setup: &[(0, 0, 0, "oak_slab[type=double]")], ..R },
    Row { name: "click_slab_bottom_with_slab_merges", setup: &[(0, 0, 0, "oak_slab[type=bottom]")], item: "oak_slab", blocks: &[(0, 0, 0, "oak_slab[type=double]")], ..R },
    Row { name: "click_stairs_bottom_top", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=bottom,shape=straight]")], ..R },
    Row { name: "click_stairs_top_half_top", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=top,shape=straight]")], ..R },
    Row { name: "click_stairs_outer_corner_top", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=bottom,shape=outer_left]")], ..R },
    Row { name: "click_stairs_inner_corner_top", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=bottom,shape=inner_right]")], ..R },
    Row { name: "click_stairs_side", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=bottom,shape=straight]")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_farmland_top", setup: &[(0, 0, 0, "farmland")], ..R },
    Row { name: "click_dirt_path_top", setup: &[(0, 0, 0, "dirt_path")], ..R },
    Row { name: "click_soul_sand_top", setup: &[(0, 0, 0, "soul_sand")], ..R },
    Row { name: "click_chest_top", setup: &[(0, 0, 0, "chest[facing=south]")], sneak: true, ..R },
    Row { name: "click_enchanting_table_top", setup: &[(0, 0, 0, "enchanting_table")], sneak: true, ..R },
    // attached
    Row { name: "click_torch_top", setup: &[(0, 0, 0, "torch")], ..R },
    Row { name: "click_wall_torch_top", setup: &[(0, 0, 1, "stone"), (0, 0, 0, "wall_torch[facing=north]")], ..R },
    Row { name: "click_lantern_standing_top", setup: &[(0, 0, 0, "lantern[hanging=false]")], ..R },
    Row { name: "click_lantern_hanging_from_chain_side", setup: &[(0, 2, 0, "stone"), (0, 1, 0, "iron_chain[axis=y]"), (0, 0, 0, "lantern[hanging=true]")], face: Face::North, blocks: SIDE, ..R },
    Row { name: "click_button_on_floor_top", setup: &[(0, 0, 0, "stone_button[face=floor,facing=north]")], sneak: true, ..R },
    Row { name: "click_lever_on_floor_top", setup: &[(0, 0, 0, "lever[face=floor,facing=north]")], sneak: true, ..R },
    Row { name: "click_ladder_top", setup: &[(0, 0, 1, "stone"), (0, 0, 0, "ladder[facing=north]")], ..R },
    Row { name: "click_wall_sign_top", setup: &[(0, 0, 1, "stone"), (0, 0, 0, "oak_wall_sign[facing=north]")], sneak: true, ..R },
    Row { name: "click_trapdoor_bottom_top", setup: &[(0, 0, 0, "oak_trapdoor[half=bottom,open=false,facing=north]")], sneak: true, ..R },
    Row { name: "click_trapdoor_top_half_top", setup: &[(0, 0, 0, "oak_trapdoor[half=top,open=false,facing=north]")], sneak: true, ..R },
    Row { name: "click_trapdoor_without_sneak_opens_it", setup: &[(0, 0, 0, "oak_trapdoor[half=bottom,open=false,facing=north]")], res: Res::ServerRefused, blocks: &[(0, 0, 0, "oak_trapdoor[open=true]")], ..R },
    Row { name: "click_door_lower_side", setup: &[(0, 1, 0, "oak_door[half=upper,facing=north]"), (0, 0, 0, "oak_door[half=lower,facing=north]")], face: Face::North, sneak: true, blocks: SIDE, ..R },
    Row { name: "click_door_upper_top_is_above_the_eye", setup: &[(0, 1, 0, "oak_door[half=upper,facing=north]"), (0, 0, 0, "oak_door[half=lower,facing=north]")], click: (0, 1, 0), res: Res::ClientRefused(Some("face not visible from the eye")), blocks: &[(0, 2, 0, "air")], ..R },
    Row { name: "click_bed_top", setup: &[(0, 0, 1, "white_bed[part=head,facing=south]"), (0, 0, 0, "white_bed[part=foot,facing=south]")], sneak: true, ..R },
    Row { name: "click_banner_top", setup: &[(0, 0, 0, "white_banner[rotation=0]")], ..R },
    Row { name: "click_skull_top", setup: &[(0, 0, 0, "skeleton_skull[rotation=0]")], ..R },
    // odd interaction shapes
    Row { name: "click_cauldron_top", setup: &[(0, 0, 0, "cauldron")], ..R },
    Row { name: "click_water_cauldron_top", setup: &[(0, 0, 0, "water_cauldron[level=3]")], ..R },
    Row { name: "click_hopper_top", setup: &[(0, 0, 0, "hopper[facing=down]")], sneak: true, ..R },
    Row { name: "click_hopper_sideways_top", setup: &[(0, 0, 0, "hopper[facing=east]")], sneak: true, ..R },
    Row { name: "click_hopper_spout_side", setup: &[(0, 0, 0, "hopper[facing=north]")], face: Face::North, sneak: true, blocks: SIDE, ..R },
    Row { name: "click_composter_top", setup: &[(0, 0, 0, "composter[level=0]")], sneak: true, ..R },
    Row { name: "click_scaffolding_top", setup: &[(0, 0, 0, "scaffolding[bottom=false]")], ..R },
    Row { name: "click_anvil_top", setup: &[(0, 0, 0, "anvil[facing=north]")], sneak: true, ..R },
    Row { name: "click_bell_top", setup: &[(0, 0, 0, "bell[attachment=floor,facing=north]")], sneak: true, ..R },
    Row { name: "click_flower_pot_top", setup: &[(0, 0, 0, "flower_pot")], sneak: true, ..R },
    Row { name: "click_campfire_top", setup: &[(0, 0, 0, "campfire[lit=false]")], ..R },
    Row { name: "click_cake_top", setup: &[(0, 0, 0, "cake")], sneak: true, ..R },
    // no collision or replaceable
    Row { name: "click_short_grass_is_replaced", setup: &[GRASS, (0, 0, 0, "short_grass")], blocks: INTO, ..R },
    Row { name: "click_poppy_top", setup: &[GRASS, (0, 0, 0, "poppy")], ..R },
];

/// Pillar to place against: stone at `T`, clicked on its north face (the bot is north of it).
const PILLAR: &[Blk] = &[(0, 0, 0, "stone")];
/// A second pillar three east, whose west face the bot also sees.
const PILLAR_E: &[Blk] = &[(3, 0, 0, "stone")];
/// Floor cell under `T`, clicked on top.
const FLOOR: (i32, i32, i32) = (0, -1, 0);

/// "Place it": the item itself, with the state vanilla gives for this look and hit point. The
/// bot faces south (yaw 0).
const PLACES: &[Row] = rows![
    Row { name: "place_stairs_on_floor", item: "oak_stairs", click: FLOOR, blocks: &[(0, 0, 0, "oak_stairs[facing=south,half=bottom,shape=straight]")], ..R },
    Row { name: "place_stairs_side_upper_hit_is_top_half", setup: PILLAR, item: "oak_stairs", face: Face::North, aim: Some((0.0, 0.3, 0.0)), blocks: &[(0, 0, -1, "oak_stairs[facing=south,half=top]")], ..R },
    Row { name: "place_stairs_side_lower_hit_is_bottom_half", setup: PILLAR, item: "oak_stairs", face: Face::North, aim: Some((0.0, -0.3, 0.0)), blocks: &[(0, 0, -1, "oak_stairs[facing=south,half=bottom]")], ..R },
    Row { name: "place_stairs_outer_corner_shape", setup: &[(0, 0, 1, "oak_stairs[facing=east,half=bottom]")], item: "oak_stairs", click: FLOOR, blocks: &[(0, 0, 0, "oak_stairs[facing=south,half=bottom,shape=outer_left]")], ..R },
    Row { name: "place_slab_on_floor_is_bottom", item: "oak_slab", click: FLOOR, blocks: &[(0, 0, 0, "oak_slab[type=bottom]")], ..R },
    Row { name: "place_slab_side_upper_hit_is_top", setup: PILLAR, item: "oak_slab", face: Face::North, aim: Some((0.0, 0.3, 0.0)), blocks: &[(0, 0, -1, "oak_slab[type=top]")], ..R },
    Row { name: "place_slab_side_lower_hit_is_bottom", setup: PILLAR, item: "oak_slab", face: Face::North, aim: Some((0.0, -0.3, 0.0)), blocks: &[(0, 0, -1, "oak_slab[type=bottom]")], ..R },
    Row { name: "place_fence", item: "oak_fence", click: FLOOR, blocks: &[(0, 0, 0, "oak_fence")], ..R },
    Row { name: "place_wall", item: "cobblestone_wall", click: FLOOR, blocks: &[(0, 0, 0, "cobblestone_wall[up=true]")], ..R },
    Row { name: "place_iron_bars", item: "iron_bars", click: FLOOR, blocks: &[(0, 0, 0, "iron_bars")], ..R },
    Row { name: "place_glass_pane", item: "glass_pane", click: FLOOR, blocks: &[(0, 0, 0, "glass_pane")], ..R },
    Row { name: "place_chain_on_floor_is_axis_y", item: "iron_chain", click: FLOOR, blocks: &[(0, 0, 0, "iron_chain[axis=y]")], ..R },
    Row { name: "place_chain_on_north_face_is_axis_z", setup: PILLAR, item: "iron_chain", face: Face::North, blocks: &[(0, 0, -1, "iron_chain[axis=z]")], ..R },
    Row { name: "place_chain_on_west_face_is_axis_x", setup: PILLAR_E, item: "iron_chain", click: (3, 0, 0), face: Face::West, blocks: &[(2, 0, 0, "iron_chain[axis=x]")], ..R },
    Row { name: "place_end_rod", item: "end_rod", click: FLOOR, blocks: &[(0, 0, 0, "end_rod[facing=up]")], ..R },
    Row { name: "place_lightning_rod", item: "lightning_rod", click: FLOOR, blocks: &[(0, 0, 0, "lightning_rod[facing=up]")], ..R },
    Row { name: "place_redstone_wire_on_stone", item: "redstone", click: FLOOR, blocks: &[(0, 0, 0, "redstone_wire")], ..R },
    Row { name: "place_redstone_wire_on_bottom_slab_refused_by_server", setup: &[(0, 0, 0, "oak_slab[type=bottom]")], item: "redstone", res: Res::ServerRefused, blocks: &[(0, 1, 0, "air")], ..R },
    Row { name: "place_redstone_wire_on_air_refused_by_client", setup: &[(0, -1, 0, "air")], item: "redstone", click: FLOOR, res: Res::ClientRefused(None), blocks: &[(0, 0, 0, "air")], ..R },
    Row { name: "place_carpet", item: "white_carpet", click: FLOOR, blocks: &[(0, 0, 0, "white_carpet")], ..R },
    Row { name: "place_rail", item: "rail", click: FLOOR, blocks: &[(0, 0, 0, "rail")], ..R },
    Row { name: "place_pressure_plate", item: "stone_pressure_plate", click: FLOOR, blocks: &[(0, 0, 0, "stone_pressure_plate")], ..R },
    Row { name: "place_snow_layer", item: "snow", click: FLOOR, blocks: &[(0, 0, 0, "snow[layers=1]")], ..R },
    Row { name: "place_repeater_faces_back_to_the_player", item: "repeater", click: FLOOR, blocks: &[(0, 0, 0, "repeater[facing=north]")], ..R },
    Row { name: "place_daylight_detector", item: "daylight_detector", click: FLOOR, blocks: &[(0, 0, 0, "daylight_detector")], ..R },
    Row { name: "place_chest_faces_the_player", item: "chest", click: FLOOR, blocks: &[(0, 0, 0, "chest[facing=north,type=single]")], ..R },
    Row { name: "place_torch_on_floor", item: "torch", click: FLOOR, blocks: &[(0, 0, 0, "torch")], ..R },
    Row { name: "place_torch_on_wall_is_wall_torch", setup: PILLAR, item: "torch", face: Face::North, blocks: &[(0, 0, -1, "wall_torch[facing=north]")], ..R },
    Row { name: "place_lantern_hanging_under_ceiling", setup: &[(0, 2, 0, "stone")], item: "lantern", click: (0, 2, 0), face: Face::Down, blocks: &[(0, 1, 0, "lantern[hanging=true]")], ..R },
    Row { name: "place_lantern_on_floor", item: "lantern", click: FLOOR, blocks: &[(0, 0, 0, "lantern[hanging=false]")], ..R },
    Row { name: "place_button_on_wall", setup: PILLAR, item: "stone_button", face: Face::North, blocks: &[(0, 0, -1, "stone_button[face=wall,facing=north]")], ..R },
    Row { name: "place_lever_on_wall", setup: PILLAR, item: "lever", face: Face::North, blocks: &[(0, 0, -1, "lever[face=wall,facing=north]")], ..R },
    Row { name: "place_ladder_on_wall", setup: PILLAR, item: "ladder", face: Face::North, blocks: &[(0, 0, -1, "ladder[facing=north]")], ..R },
    Row { name: "place_trapdoor_on_side_upper_hit", setup: PILLAR, item: "oak_trapdoor", face: Face::North, aim: Some((0.0, 0.3, 0.0)), blocks: &[(0, 0, -1, "oak_trapdoor[facing=north,half=top]")], ..R },
    Row { name: "place_door_both_halves", item: "oak_door", click: FLOOR, blocks: &[(0, 0, 0, "oak_door[half=lower,facing=south]"), (0, 1, 0, "oak_door[half=upper]")], ..R },
    Row { name: "place_bed_both_parts", item: "white_bed", click: FLOOR, blocks: &[(0, 0, 0, "white_bed[part=foot,facing=south]"), (0, 0, 1, "white_bed[part=head,facing=south]")], ..R },
    Row { name: "place_banner", item: "white_banner", click: FLOOR, blocks: &[(0, 0, 0, "white_banner")], ..R },
    Row { name: "place_skull_rotation_from_yaw", item: "skeleton_skull", click: FLOOR, blocks: &[(0, 0, 0, "skeleton_skull[rotation=0]")], ..R },
    Row { name: "place_anvil_faces_clockwise_of_the_player", item: "anvil", click: FLOOR, blocks: &[(0, 0, 0, "anvil[facing=west]")], ..R },
    Row { name: "place_cauldron", item: "cauldron", click: FLOOR, blocks: &[(0, 0, 0, "cauldron")], ..R },
    Row { name: "place_hopper", item: "hopper", click: FLOOR, blocks: &[(0, 0, 0, "hopper[facing=down]")], ..R },
    Row { name: "place_composter", item: "composter", click: FLOOR, blocks: &[(0, 0, 0, "composter")], ..R },
    Row { name: "place_scaffolding", item: "scaffolding", click: FLOOR, blocks: &[(0, 0, 0, "scaffolding")], ..R },
    Row { name: "place_campfire", item: "campfire", click: FLOOR, blocks: &[(0, 0, 0, "campfire")], ..R },
    Row { name: "place_cake", item: "cake", click: FLOOR, blocks: &[(0, 0, 0, "cake")], ..R },
    Row { name: "place_flower_pot", item: "flower_pot", click: FLOOR, blocks: &[(0, 0, 0, "flower_pot")], ..R },
    Row { name: "place_enchanting_table", item: "enchanting_table", click: FLOOR, blocks: &[(0, 0, 0, "enchanting_table")], ..R },
    Row { name: "place_bell_on_floor", item: "bell", click: FLOOR, blocks: &[(0, 0, 0, "bell[attachment=floor]")], ..R },
    Row { name: "place_poppy_on_grass", setup: &[(0, -1, 0, "grass_block")], item: "poppy", click: FLOOR, blocks: &[(0, 0, 0, "poppy")], ..R },
];

// Spec: specs/B6-placement-breaking.md §4.2, §4.6 (place on a chosen face against any shape; read the ack)
fn run_row(cx: &mut Ctx, r: &Row) -> Outcome {
    reset(cx);
    set_up(cx, r.setup);
    give(cx, Some(r.item));
    let clicked = at(r.click);
    cx.run("A4S", r.name, |cx| {
        if r.sneak {
            cx.call(Op::Sneak(true));
        }
        let before = cx.count(r.item);
        let op = match r.aim {
            Some((x, y, z)) => Op::Place { pos: clicked, face: r.face, aim: Vec3::new(x, y, z) },
            None => Op::PlaceAim { pos: clicked, face: r.face },
        };
        let rep = cx.call(op);
        if r.sneak {
            cx.call(Op::Sneak(false));
        }
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        sleep(Duration::from_millis(250));
        let after = cx.count(r.item);
        let missing: Vec<String> = r
            .blocks
            .iter()
            .filter(|(dx, dy, dz, b)| !cx.block_is(at((*dx, *dy, *dz)), b))
            .map(|(dx, dy, dz, b)| format!("{b} not at {:?}", at((*dx, *dy, *dz))))
            .collect();
        let (want_out, want_items, reason_ok) = match r.res {
            Res::Placed => (BlockOutcome::Done, before - 1, a.reason.is_none()),
            Res::ServerRefused => (BlockOutcome::Refused, before, a.reason.is_none()),
            Res::ClientRefused(w) => (
                BlockOutcome::Refused,
                before,
                a.reason.is_some() && w.is_none_or(|w| a.reason == Some(w)) && a.ack_tick.is_none(),
            ),
        };
        let ok = a.outcome == want_out && after == want_items && reason_ok && missing.is_empty();
        (
            ok,
            format!(
                "{:?} face of {clicked:?} holding {}{}; bot outcome {:?} (want {want_out:?}), reason {:?}; {} count {before} -> {after}; server: {}",
                r.face,
                r.item,
                if r.sneak { " sneaking" } else { "" },
                a.outcome,
                a.reason,
                r.item,
                if missing.is_empty() { "expected states hold".to_string() } else { missing.join(", ") }
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.2, §4.6 (place against and place every shape family)
pub fn a4s(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let out = CLICKS.iter().chain(PLACES).filter(|r| wanted(r.name)).map(|r| run_row(cx, r)).collect();
    cx.clean_in(AREA);
    out
}

/// One break: the block (and what it needs) is set up, the bot holds `tool`, breaks it by aiming
/// at its real shape. The bot-side tick count must equal vanilla's formula.
struct Brk {
    name: &'static str,
    setup: &'static [Blk],
    tool: Option<&'static str>,
    hardness: f32,
    /// Mining speed of the tool on this block (6 for an iron tool of the right kind, else 1).
    speed: f32,
    /// Whether vanilla counts the tool as right for drops (the /30 rather than /100 divisor).
    harvest: bool,
    /// Run under Haste 255 (mining speed x52.2).
    haste: bool,
}

const B: Brk = Brk { name: "", setup: &[], tool: None, hardness: 0.0, speed: 1.0, harvest: true, haste: false };
const PICK: Option<&str> = Some("iron_pickaxe");
const AXE: Option<&str> = Some("iron_axe");

/// Every row runs for its full vanilla time except the last, which runs under Haste 255 (the
/// instamine path). Before fork patch 10, Grim's FastBreak flagged after about 20 full-length
/// breaks, so most rows ran under Haste; the cause was the bot's timing, not Grim.
const BREAKS: &[Brk] = &[
    // instant at hardness 0: the break goes out on the start packet (BlockBreakSent from the instamine path)
    Brk { name: "break_redstone_wire", setup: &[(0, 0, 0, "redstone_wire")], ..B },
    Brk { name: "break_torch", setup: &[(0, 0, 0, "torch")], ..B },
    Brk { name: "break_wall_torch", setup: &[(0, 0, 1, "stone"), (0, 0, 0, "wall_torch[facing=north]")], ..B },
    Brk { name: "break_short_grass", setup: &[GRASS, (0, 0, 0, "short_grass")], ..B },
    Brk { name: "break_poppy", setup: &[GRASS, (0, 0, 0, "poppy")], ..B },
    Brk { name: "break_lily_pad", setup: &[(0, -1, 0, "water"), (0, 0, 0, "lily_pad")], ..B },
    Brk { name: "break_repeater", setup: &[(0, 0, 0, "repeater")], ..B },
    Brk { name: "break_end_rod", setup: &[(0, 0, 0, "end_rod[facing=up]")], ..B },
    Brk { name: "break_scaffolding", setup: &[(0, 0, 0, "scaffolding[bottom=false]")], ..B },
    Brk { name: "break_flower_pot", setup: &[(0, 0, 0, "flower_pot")], ..B },
    // full time, normal speed
    Brk { name: "break_carpet", setup: &[(0, 0, 0, "white_carpet")], hardness: 0.1, ..B },
    Brk { name: "break_rail", setup: &[(0, 0, 0, "rail")], tool: PICK, hardness: 0.7, speed: 6.0, ..B },
    Brk { name: "break_chain_y", setup: &[(0, 0, 0, "iron_chain[axis=y]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_fence_by_hand", setup: &[(0, 0, 0, "oak_fence")], hardness: 2.0, ..B },
    Brk { name: "break_wall_by_hand_wrong_tool", setup: &[(0, 0, 0, "cobblestone_wall")], hardness: 2.0, harvest: false, ..B },
    Brk { name: "break_glass_pane", setup: &[(0, 0, 0, "glass_pane")], hardness: 0.3, ..B },
    Brk { name: "break_stairs", setup: &[(0, 0, 0, "oak_stairs[facing=east,half=bottom]")], tool: AXE, hardness: 2.0, speed: 6.0, ..B },
    Brk { name: "break_lantern", setup: &[(0, 0, 0, "lantern[hanging=false]")], tool: PICK, hardness: 3.5, speed: 6.0, ..B },
    Brk { name: "break_farmland", setup: &[(0, 0, 0, "farmland")], hardness: 0.6, ..B },
    Brk { name: "break_pressure_plate", setup: &[(0, 0, 0, "stone_pressure_plate")], tool: PICK, hardness: 0.5, speed: 6.0, ..B },
    Brk { name: "break_snow_layer", setup: &[(0, 0, 0, "snow[layers=1]")], tool: Some("iron_shovel"), hardness: 0.1, speed: 6.0, ..B },
    Brk { name: "break_iron_bars", setup: &[(0, 0, 0, "iron_bars")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_chain_x", setup: &[(0, 0, 0, "iron_chain[axis=x]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_chain_z", setup: &[(0, 0, 0, "iron_chain[axis=z]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_lightning_rod", setup: &[(0, 0, 0, "lightning_rod[facing=up]")], tool: PICK, hardness: 3.0, speed: 6.0, ..B },
    Brk { name: "break_slab", setup: &[(0, 0, 0, "oak_slab[type=bottom]")], tool: AXE, hardness: 2.0, speed: 6.0, ..B },
    Brk { name: "break_dirt_path", setup: &[(0, 0, 0, "dirt_path")], hardness: 0.65, ..B },
    Brk { name: "break_soul_sand", setup: &[(0, 0, 0, "soul_sand")], hardness: 0.5, ..B },
    Brk { name: "break_chest", setup: &[(0, 0, 0, "chest[facing=south]")], tool: AXE, hardness: 2.5, speed: 6.0, ..B },
    Brk { name: "break_enchanting_table", setup: &[(0, 0, 0, "enchanting_table")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_ladder", setup: &[(0, 0, 1, "stone"), (0, 0, 0, "ladder[facing=north]")], hardness: 0.4, ..B },
    Brk { name: "break_trapdoor", setup: &[(0, 0, 0, "oak_trapdoor[half=bottom,facing=north]")], tool: AXE, hardness: 3.0, speed: 6.0, ..B },
    Brk { name: "break_door_lower_half", setup: &[(0, 1, 0, "oak_door[half=upper,facing=north]"), (0, 0, 0, "oak_door[half=lower,facing=north]")], tool: AXE, hardness: 3.0, speed: 6.0, ..B },
    Brk { name: "break_bed", setup: &[(0, 0, 1, "white_bed[part=head,facing=south]"), (0, 0, 0, "white_bed[part=foot,facing=south]")], hardness: 0.2, ..B },
    Brk { name: "break_banner", setup: &[(0, 0, 0, "white_banner[rotation=0]")], hardness: 1.0, ..B },
    Brk { name: "break_skull", setup: &[(0, 0, 0, "skeleton_skull[rotation=0]")], hardness: 1.0, ..B },
    Brk { name: "break_cauldron", setup: &[(0, 0, 0, "cauldron")], tool: PICK, hardness: 2.0, speed: 6.0, ..B },
    Brk { name: "break_hopper", setup: &[(0, 0, 0, "hopper[facing=down]")], tool: PICK, hardness: 3.0, speed: 6.0, ..B },
    Brk { name: "break_composter", setup: &[(0, 0, 0, "composter[level=0]")], hardness: 0.6, ..B },
    Brk { name: "break_anvil", setup: &[(0, 0, 0, "anvil[facing=north]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_bell", setup: &[(0, 0, 0, "bell[attachment=floor,facing=north]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B },
    Brk { name: "break_campfire", setup: &[(0, 0, 0, "campfire[lit=false]")], tool: AXE, hardness: 2.0, speed: 6.0, ..B },
    Brk { name: "break_cake", setup: &[(0, 0, 0, "cake")], hardness: 0.5, ..B },
    // Haste 255 with the right tool: instamine (the break goes out on the start packet)
    Brk { name: "break_wall_hasted", setup: &[(0, 0, 0, "cobblestone_wall")], tool: PICK, hardness: 2.0, speed: 6.0, haste: true, ..B },
];

/// The bot-side tick count of a break must equal vanilla's (`ticks_between_packets`).
// Spec: specs/B6-placement-breaking.md §4.4 (break timing against any shape; instamine path)
fn run_break(cx: &mut Ctx, b: &Brk, spam: bool) -> Outcome {
    reset(cx);
    set_up(cx, b.setup);
    give(cx, b.tool);
    let target = at((0, 0, 0));
    let name = if spam { format!("{}_command_every_tick", b.name) } else { b.name.to_string() };
    cx.run("A2S", &name, |cx| {
        let rep = cx.call(if spam { Op::BreakSpam(target) } else { Op::BreakAim(target) });
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        sleep(Duration::from_millis(250));
        let air = cx.block_is(target, "air");
        let speed = if b.haste { b.speed * (1.0f32 + 256.0 * 0.2) } else { b.speed };
        let n = vanilla_ticks(b.hardness, speed, b.harvest);
        let want = ticks_between_packets(n) as u64;
        let ticks = a.finish_tick.map(|f| f - a.start_tick);
        (
            air && a.outcome == BlockOutcome::Done && ticks == Some(want),
            format!(
                "hardness {} with {} (speed {}, right tool {}): vanilla {n} mining ticks = {want} between start and finish; bot {ticks:?} (outcome {:?}, reason {:?}, ack read at tick {:?}); air on server: {air}",
                b.hardness,
                b.tool.unwrap_or("hand"),
                speed,
                b.harvest,
                a.outcome,
                a.reason,
                a.ack_tick
            ),
        )
    })
}

// Spec: specs/B6-placement-breaking.md §4.4 (break any shape at vanilla speed; refusals; repeated command)
pub fn a2s(cx: &mut Ctx) -> Vec<Outcome> {
    cx.scene_in(AREA, &[], STAND);
    let n = cx.name.clone();
    let mut out = Vec::new();
    for b in BREAKS.iter().filter(|b| !b.haste && wanted(b.name)) {
        out.push(run_break(cx, b, false));
    }
    // The same break command every tick still ends with a result (the adapter keeps one record).
    out.push(run_break(cx, &Brk { name: "break_chain_y", setup: &[(0, 0, 0, "iron_chain[axis=y]")], tool: PICK, hardness: 5.0, speed: 6.0, ..B }, true));
    cx.r(&format!("effect give {n} minecraft:haste 1000000 255 true"));
    sleep(Duration::from_millis(500));
    for b in BREAKS.iter().filter(|b| b.haste && wanted(b.name)) {
        out.push(run_break(cx, b, false));
    }
    out.push(run_break(cx, &Brk { name: "break_wall_hasted", setup: &[(0, 0, 0, "cobblestone_wall")], tool: PICK, hardness: 2.0, speed: 6.0, haste: true, ..B }, true));
    cx.r(&format!("effect clear {n}"));
    sleep(Duration::from_millis(300));
    // Adventure mode may not break blocks: refused with a reason, not "cancelled".
    reset(cx);
    set_up(cx, &[(0, 0, 0, "stone")]);
    cx.r(&format!("gamemode adventure {n}"));
    sleep(Duration::from_millis(400));
    out.push(cx.run("A2S", "refused_break_in_adventure_mode", |cx| {
        let rep = cx.call(Op::BreakAim(at((0, 0, 0))));
        let still = cx.block_is(at((0, 0, 0)), "stone");
        let Info::Action(a) = rep.info else { return (false, format!("bot reported {:?}", rep.info)) };
        (
            a.outcome == BlockOutcome::Refused && a.reason == Some("breaking not allowed in this game mode") && still,
            format!("outcome {:?} reason {:?} (want Refused, \"breaking not allowed in this game mode\"); stone still there: {still}", a.outcome, a.reason),
        )
    }));
    cx.r(&format!("gamemode survival {n}"));
    sleep(Duration::from_millis(400));
    cx.clean_in(AREA);
    out
}
