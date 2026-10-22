//! A21: pathfinder walls for blocks whose effect on motion the fork does not model (fork patch 36).
//! Plan: specs/plans/PATCH_34_BRIEF.md, specs/plans/PATCH_30_TIER1.md §2.7, specs/B1-movement.md
//! §4.3. Each row builds a wall across the straight line from the start to the goal, with a gap of
//! open floor at both ends, and sends `goto`. The bot must arrive by a path round the wall and its
//! box must never touch the wall block (or, for a piston, the cell in front of its face; for
//! honey, its side). Touching would be a `Movement` entry in patch 30's gate, which disconnects
//! the bot: so the rows also check that the account is still connected and has no disconnect
//! record. One border row sets the world border so the goal lies inside the 3-block margin; the
//! goal cannot be reached and the bot must stay outside the margin.
//! Area: corner (1010, 342), floor y=199; the wall runs along x at z = 351, the bot walks +z.

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use crate::{
    scenarios::{Ctx, FLOOR_Y, Outcome, pos},
    scenarios_block_effects::bot_pos,
    scenarios_envelope::seen,
    skill::{Info, Op},
};

const AREA: (i32, i32) = (1010, 342);
const START: (f64, f64, f64) = (1020.5, (FLOOR_Y + 1) as f64, 345.5);
const GOAL: (i32, i32) = (1020, 357);
/// The wall: cells x in `WALL_X`, z = `WALL_Z`. The area spans x 1010..1030, so the gaps at the
/// ends are 5 and 4 cells wide.
const WALL_X: (i32, i32) = (1015, 1025);
const WALL_Z: i32 = 351;
const HALF_WIDTH: f64 = 0.3;
const BOT_HEIGHT: f64 = 1.8;
/// The world border's margin for the planner (patch 36): 2 blocks for a player's box plus 1.
const BORDER_MARGIN: f64 = 3.0;

/// A box of blocks the bot must not touch: x, y, z ranges as (min, max).
type Danger = ((f64, f64), (f64, f64), (f64, f64));

fn touches(p: [f64; 3], d: &Danger) -> bool {
    const E: f64 = 1e-3;
    p[0] + HALF_WIDTH > d.0.0 + E
        && p[0] - HALF_WIDTH < d.0.1 - E
        && p[1] + BOT_HEIGHT > d.1.0 + E
        && p[1] < d.1.1 - E
        && p[2] + HALF_WIDTH > d.2.0 + E
        && p[2] - HALF_WIDTH < d.2.1 - E
}

fn fill(y0: i32, y1: i32, z0: i32, z1: i32, block: &str) -> String {
    format!("fill {} {y0} {z0} {} {y1} {z1} minecraft:{block}", WALL_X.0, WALL_X.1)
}

/// The wall box over the row's cells, grown by `grow` blocks on every side but down.
fn wall_box(y0: f64, y1: f64, z0: i32, z1: i32, grow: f64) -> Danger {
    (
        (WALL_X.0 as f64 - grow, (WALL_X.1 + 1) as f64 + grow),
        (y0, y1),
        (z0 as f64 - grow, (z1 + 1) as f64 + grow),
    )
}

/// One pathfinder row. `setup` builds the wall; `present` is a block of it that must still be
/// there when the goto starts (a wall that vanished tests nothing); `dangers` are the boxes the
/// bot must not touch. The bot must arrive at the goal.
// Spec: specs/B1-movement.md §4.3 (walls for unmodelled blocks)
fn wall_row(cx: &mut Ctx, name: &str, setup: &[String], present: (i32, i32, i32, &str), dangers: &[Danger]) -> Outcome {
    cx.scene_in(AREA, &[], START);
    for c in setup {
        cx.r(c);
    }
    sleep(Duration::from_millis(1000));
    let goal = pos(GOAL.0, FLOOR_Y + 1, GOAL.1);
    cx.run("A21", name, |cx| {
        let here = cx.block_is(pos(present.0, present.1, present.2), present.3);
        if !here {
            return (false, format!("the wall block {} at ({}, {}, {}) is not in place before the goto, so the row tests nothing", present.3, present.0, present.1, present.2));
        }
        let seq = cx.send(Op::Goto(goal));
        let t = Instant::now();
        let (mut samples, mut touched, mut closest) = (0, 0, f64::MAX);
        let mut first_touch: Option<String> = None;
        let mut near: Vec<String> = Vec::new();
        loop {
            if cx.try_recv(seq).is_some_and(|r| !matches!(r.info, Info::Started)) {
                break;
            }
            if t.elapsed() > Duration::from_secs(40) {
                return (false, "the goto did not finish in 40 s".into());
            }
            let p = bot_pos(cx);
            samples += 1;
            for d in dangers {
                if touches(p, d) {
                    touched += 1;
                    first_touch.get_or_insert_with(|| format!("sample {samples} at ({:.2}, {:.2}, {:.2})", p[0], p[1], p[2]));
                }
                // distance in the xz plane from the bot's centre to the danger box
                let dx = (d.0.0 - p[0]).max(p[0] - d.0.1).max(0.0);
                let dz = (d.2.0 - p[2]).max(p[2] - d.2.1).max(0.0);
                let dist = dx.hypot(dz);
                closest = closest.min(dist);
                if dist < 1.0 && near.len() < 60 {
                    near.push(format!("({:.2},{:.2},{:.2})", p[0], p[1], p[2]));
                }
            }
        }
        sleep(Duration::from_millis(300));
        let end = bot_pos(cx);
        let arrived = (end[0] - goal.x as f64 - 0.5).abs() < 1.0 && (end[2] - goal.z as f64 - 0.5).abs() < 1.0;
        let s = seen(&cx.name);
        let connected = s.as_ref().is_some_and(|s| s.connected);
        let disconnect = s.and_then(|s| s.disconnect);
        let still_there = cx.block_is(pos(present.0, present.1, present.2), present.3);
        (
            arrived && touched == 0 && connected && disconnect.is_none() && still_there,
            format!(
                "goto ({}, {}, {}) from z {}: ended at ({:.2}, {:.2}, {:.2}) (arrived: {arrived}); {samples} position samples, {touched} touching the wall (want 0; first: {}), closest approach {closest:.2} blocks; connected: {connected}; disconnect record: {disconnect:?}; wall still there: {still_there}",
                goal.x, goal.y, goal.z, START.2, end[0], end[1], end[2],
                first_touch.unwrap_or_else(|| "none".into()),
            ) + &format!("; samples within 1 block of the wall: {}", near.join(" ")),
        )
    })
}

// Spec: specs/B1-movement.md §4.3 (BubbleColumnBlock#entityInside: bubble columns are walls)
fn bubble_column(cx: &mut Ctx) -> Outcome {
    // A column needs magma (or soul sand) under it and a water source above its top, or the server
    // turns it back into water. A bubble column is a water source: it floods every open cell
    // beside it (the first live run drowned the whole area), so the strip is sealed in stone on
    // both long sides, both ends and above the water. The planner still has to route round the
    // sealed strip, and the bot must never reach the column's cells.
    let y = FLOOR_Y;
    let (x0, x1) = (WALL_X.0 - 1, WALL_X.1 + 1);
    let setup = vec![
        fill(y, y, WALL_Z, WALL_Z, "magma_block"),
        fill(y + 1, y + 2, WALL_Z, WALL_Z, "bubble_column[drag=false]"),
        fill(y + 3, y + 3, WALL_Z, WALL_Z, "water"),
        format!("fill {x0} {} {} {x1} {} {} minecraft:stone", y + 1, WALL_Z - 1, y + 4, WALL_Z - 1),
        format!("fill {x0} {} {} {x1} {} {} minecraft:stone", y + 1, WALL_Z + 1, y + 4, WALL_Z + 1),
        format!("fill {x0} {} {WALL_Z} {x0} {} {WALL_Z} minecraft:stone", y + 1, y + 4),
        format!("fill {x1} {} {WALL_Z} {x1} {} {WALL_Z} minecraft:stone", y + 1, y + 4),
        fill(y + 4, y + 4, WALL_Z, WALL_Z, "stone"),
    ];
    wall_row(
        cx,
        "goto_across_a_bubble_column_goes_around_it",
        &setup,
        (WALL_X.0 + 5, y + 1, WALL_Z, "bubble_column"),
        &[wall_box((y + 1) as f64, (y + 5) as f64, WALL_Z - 1, WALL_Z + 1, 0.0)],
    )
}

// Spec: specs/B1-movement.md §4.3 (SlimeBlock#stepOn: no path stands on slime)
fn slime_floor(cx: &mut Ctx) -> Outcome {
    let y = FLOOR_Y;
    // The floor itself is slime across the line (3 cells deep); the bot may not stand on it.
    let setup = vec![fill(y, y, WALL_Z - 1, WALL_Z + 1, "slime_block")];
    wall_row(
        cx,
        "goto_across_a_slime_floor_goes_around_it",
        &setup,
        (WALL_X.0 + 5, y, WALL_Z, "slime_block"),
        // from the floor up to 0.2 above its top: a bot standing on it has its feet at y + 1
        &[wall_box(y as f64, y as f64 + 1.2, WALL_Z - 1, WALL_Z + 1, 0.0)],
    )
}

// Spec: specs/B1-movement.md §4.3 (BedBlock#updateEntityMovementAfterFallOn: beds are walls)
fn bed(cx: &mut Ctx) -> Outcome {
    let y = FLOOR_Y + 1;
    // Beds lie along z: foot at WALL_Z, head one cell toward the start.
    let mut setup = vec![];
    for x in WALL_X.0..=WALL_X.1 {
        setup.push(format!("setblock {x} {y} {WALL_Z} minecraft:red_bed[facing=north,part=foot]"));
        setup.push(format!("setblock {x} {y} {} minecraft:red_bed[facing=north,part=head]", WALL_Z - 1));
    }
    wall_row(
        cx,
        "goto_across_beds_goes_around_them",
        &setup,
        (WALL_X.0 + 5, y, WALL_Z, "red_bed"),
        &[wall_box(y as f64, y as f64 + 1.5, WALL_Z - 1, WALL_Z, 0.0)],
    )
}

// Spec: specs/B1-movement.md §4.3 (HoneyBlock#entityInside: the sides of honey at body height)
fn honey_wall(cx: &mut Ctx) -> Outcome {
    let y = FLOOR_Y + 1;
    let setup = vec![fill(y, y + 1, WALL_Z, WALL_Z, "honey_block")];
    wall_row(
        cx,
        "goto_across_a_honey_wall_goes_around_it",
        &setup,
        (WALL_X.0 + 5, y, WALL_Z, "honey_block"),
        // any approach within 1 cm of a side counts as touching it
        &[wall_box(y as f64, y as f64 + 2.0, WALL_Z, WALL_Z, 0.01)],
    )
}

// Spec: specs/B1-movement.md §4.3 (ScaffoldingBlock#getCollisionShape: scaffolding is a wall)
fn scaffolding(cx: &mut Ctx) -> Outcome {
    let y = FLOOR_Y + 1;
    let setup = vec![fill(y, y + 1, WALL_Z, WALL_Z, "scaffolding[bottom=false,distance=0]")];
    wall_row(
        cx,
        "goto_across_scaffolding_goes_around_it",
        &setup,
        (WALL_X.0 + 5, y, WALL_Z, "scaffolding"),
        &[wall_box(y as f64, y as f64 + 2.0, WALL_Z, WALL_Z, 0.0)],
    )
}

// Spec: specs/B1-movement.md §4.3 (PistonBaseBlock#triggerEvent: the cell in front of a piston face)
fn piston_face(cx: &mut Ctx) -> Outcome {
    let y = FLOOR_Y + 1;
    // Pistons face the start (north), so the cell in front of each face is one cell toward it.
    let setup = vec![fill(y, y, WALL_Z, WALL_Z, "piston[facing=north,extended=false]")];
    wall_row(
        cx,
        "goto_across_piston_faces_goes_around_them",
        &setup,
        (WALL_X.0 + 5, y, WALL_Z, "piston"),
        // the piston row and the row of cells in front of its faces
        &[wall_box(y as f64, y as f64 + 2.0, WALL_Z - 1, WALL_Z, 0.0)],
    )
}

// Spec: specs/B1-movement.md §4.3 (WorldBorder#isInsideCloseToBorder: the border margin)
fn world_border(cx: &mut Ctx) -> Outcome {
    // A border 20 wide centred in the area puts its edges on the area's rim: x 1010..1030 and
    // z 342..362. The goal cell (z 360) is inside the 3-block margin, so it cannot be planned.
    let (cx0, cz0) = (AREA.0 as f64 + 10.0, AREA.1 as f64 + 10.0);
    cx.scene_in(AREA, &[], (START.0, START.1, START.2 + 1.0));
    // west of the path, inside the border (x 1012 to 1015, z 344 to 349)
    crate::scenarios_envelope::bring_others_inside(cx, (AREA.0 + 2, AREA.1 + 2));
    cx.r(&format!("worldborder center {cx0} {cz0}"));
    cx.r("worldborder set 20");
    sleep(Duration::from_millis(1500));
    let edge_z = cz0 + 10.0;
    let goal = pos(1020, FLOOR_Y + 1, (edge_z as i32) - 2);
    let out = cx.run("A21", "goto_inside_the_world_border_margin_stays_outside_it", |cx| {
        let seq = cx.send(Op::Goto(goal));
        let t = Instant::now();
        let (mut samples, mut max_z) = (0, f64::MIN);
        loop {
            if cx.try_recv(seq).is_some_and(|r| !matches!(r.info, Info::Started)) {
                break;
            }
            if t.elapsed() > Duration::from_secs(40) {
                return (false, "the goto did not finish in 40 s".into());
            }
            max_z = max_z.max(bot_pos(cx)[2]);
            samples += 1;
        }
        sleep(Duration::from_millis(300));
        let end = bot_pos(cx);
        max_z = max_z.max(end[2]);
        // The last cell the planner allows is the one whose far edge is the margin away.
        let limit = edge_z - BORDER_MARGIN;
        let s = seen(&cx.name);
        let connected = s.as_ref().is_some_and(|s| s.connected);
        let disconnect = s.and_then(|s| s.disconnect);
        (
            max_z < limit + HALF_WIDTH && connected && disconnect.is_none() && samples >= 3,
            format!(
                "goto ({}, {}, {}) with the border's edge at z {edge_z}: ended at ({:.2}, {:.2}, {:.2}), furthest z {max_z:.2} over {samples} samples (margin starts at z {limit}); connected: {connected}; disconnect record: {disconnect:?}",
                goal.x, goal.y, goal.z, end[0], end[1], end[2]
            ),
        )
    });
    // The default border, restored whatever the row did.
    cx.r("worldborder set 59999968");
    cx.r("worldborder center 0 0");
    out
}

// Spec: specs/B1-movement.md §4.3
pub fn a21(cx: &mut Ctx) -> Vec<Outcome> {
    let out = vec![
        bubble_column(cx),
        slime_floor(cx),
        bed(cx),
        honey_wall(cx),
        scaffolding(cx),
        piston_face(cx),
        world_border(cx),
    ];
    cx.clean_in(AREA);
    out
}
