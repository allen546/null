//! A20: client refusals (fork patch 34). Spec: specs/F5-version-layer.md §4.6; plan:
//! specs/plans/ABILITY_CHECK.md. The bot never starts a movement the fork does not model: it
//! refuses on the client, with a reason, and sends nothing. Rows:
//! - `refuse_elytra_jump_in_air`: an elytra worn, the bot falls and presses jump in the air:
//!   refused, no `PlayerCommand` sent, the server shows no glide, the bot lands;
//! - `broken_elytra_jump_not_refused`: an elytra one point from breaking: the same jump is not
//!   refused (vanilla cannot glide with it) and the server shows no glide;
//! - `refuse_riptide_in_water`: a Riptide trident used standing in water: refused, no `UseItem`
//!   sent, the bot does not move;
//! - `refuse_use_effects_speed`: an item with `use_effects` speed 1.0: refused, no `UseItem`
//!   sent, the item is not eaten;
//! - `refuse_mount_*`: a right-click on a boat, minecart, untamed horse (empty hand), saddled
//!   pig, camel and harnessed happy ghast: refused (`WouldMount`), no `Interact` sent, the
//!   server shows no passenger.
//!
//! A probe in the bot's app records the name of every game packet it sends while a row watches.
//! Server truth is read by RCON; the server log is scanned for warnings and Grim alerts on every
//! row. Area: corner (980, 342), floor y=199.

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
    ecs::prelude::*,
    packet::game::SendGamePacketEvent,
};
use null_bot_api::InputRefusal;

use crate::{
    scenarios::{Ctx, Outcome, last_int},
    scenarios_inventory::inv,
    skill::{Info, Op},
    skill_inventory::{Aim, InvOp},
    skill_refusals::{RefuseInfo, RefuseOp},
};

/// This module's test area (corner, 21x21), apart from the other scenarios' areas.
const AREA: (i32, i32) = (980, 342);
/// The bot stands here, facing south until it aims (the entity is north of it).
const SPAWN: (f64, f64, f64) = (990.5, 200.0, 352.5);
/// The tag every entity of this module carries.
const SEL: &str = "@e[tag=a20,limit=1]";

pub static SENT_ON: AtomicBool = AtomicBool::new(false);
/// Names of the game packets the main account sent while `SENT_ON`.
pub static SENT: Mutex<Vec<String>> = Mutex::new(Vec::new());

// Spec: specs/F5-version-layer.md §4.6 (client refusals send nothing)
/// Adds the packet probe to the bot's app. `main` is the runner's account.
pub struct RefusalProbe {
    pub main: String,
}

impl Plugin for RefusalProbe {
    fn build(&self, app: &mut App) {
        let main = self.main.clone();
        app.add_observer(move |trigger: On<SendGamePacketEvent>, accounts: Query<&Account>| {
            if !SENT_ON.load(Ordering::Relaxed) {
                return;
            }
            if accounts.get(trigger.event().sent_by).is_ok_and(|a| a.username() == main) {
                let text = format!("{:?}", trigger.event().packet);
                SENT.lock().unwrap().push(text.split(['(', ' ', '{']).next().unwrap_or("").to_string());
            }
        });
    }
}

fn watch() {
    SENT.lock().unwrap().clear();
    SENT_ON.store(true, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
}

fn unwatch() -> Vec<String> {
    SENT_ON.store(false, Ordering::Relaxed);
    sleep(Duration::from_millis(100));
    std::mem::take(&mut *SENT.lock().unwrap())
}

fn sent_count(sent: &[String], names: &[&str]) -> usize {
    sent.iter().filter(|n| names.contains(&n.as_str())).count()
}

fn refuse(cx: &mut Ctx, op: RefuseOp) -> RefuseInfo {
    match cx.call(Op::Refuse(op)).info {
        Info::Refuse(i) => i,
        i => panic!("expected refusal info: {i:?}"),
    }
}

fn count_of(i: &RefuseInfo, kind: &str) -> u32 {
    i.refused.iter().find(|r: &&InputRefusal| r.kind == kind).map_or(0, |r| r.count)
}

/// Whether the server holds the bot as gliding (`EntityFlagsPredicate#isFlying` is
/// `Entity#isFallFlying`).
fn server_gliding(cx: &mut Ctx) -> bool {
    let n = cx.name.clone();
    cx.r(&format!(
        "execute as {n} if predicate {{condition:\"minecraft:entity_properties\",entity:\"this\",predicate:{{flags:{{is_flying:true}}}}}}"
    ))
    .contains("passed")
}

fn on_ground(cx: &mut Ctx) -> bool {
    let n = cx.name.clone();
    last_int(&cx.r(&format!("data get entity {n} OnGround"))) == Some(1)
}

fn bot_pos(cx: &mut Ctx) -> (f64, f64, f64) {
    let s = cx.snap();
    (s.pos.x, s.pos.y, s.pos.z)
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: elytra)
fn elytra_row(cx: &mut Ctx, name: &'static str, chest: &str, expect_refused: bool) -> Outcome {
    cx.scene_in(AREA, &[], SPAWN);
    let n = cx.name.clone();
    cx.r(&format!("item replace entity {n} armor.chest with {chest}"));
    sleep(Duration::from_millis(400));
    let before = refuse(cx, RefuseOp::Status);
    cx.run("A20", name, |cx| {
        let n = cx.name.clone();
        watch();
        // A fall of four blocks: the bot is in the air for about nine ticks.
        cx.r(&format!("execute in minecraft:overworld run tp {n} {} 204.0 {} 0 0", SPAWN.0, SPAWN.2));
        let i = refuse(cx, RefuseOp::AirJump);
        let gliding = server_gliding(cx);
        let landed = cx.poll(10.0, |cx| on_ground(cx)).is_some();
        let sent = unwatch();
        let delta = count_of(&i, "elytra_start") - count_of(&before, "elytra_start");
        let commands = sent_count(&sent, &["PlayerCommand"]);
        let ok = i.issued
            && delta == if expect_refused { 1 } else { 0 }
            && commands == 0
            && !gliding
            && landed;
        (
            ok,
            format!(
                "jump pressed {} ticks into the fall: elytra_start refusals +{delta} (want {}), reason {:?}; PlayerCommand packets sent {commands} (want 0); server gliding {gliding} (want false); landed {landed}; bot at y {:.2} (on ground {}) when the op ended; packets seen {}",
                i.air_ticks,
                u32::from(expect_refused),
                i.refused.iter().find(|r| r.kind == "elytra_start").filter(|_| expect_refused).map(|r| r.reason),
                i.pos.y,
                i.on_ground,
                sent.len()
            ),
        )
    })
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: elytra)
fn refuse_elytra_jump_in_air(cx: &mut Ctx) -> Outcome {
    elytra_row(cx, "refuse_elytra_jump_in_air", "minecraft:elytra", true)
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: elytra, a broken glider)
fn broken_elytra_jump_not_refused(cx: &mut Ctx) -> Outcome {
    elytra_row(cx, "broken_elytra_jump_not_refused", "minecraft:elytra[damage=431]", false)
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: riptide)
fn refuse_riptide_in_water(cx: &mut Ctx) -> Outcome {
    // The companion (companion mode) stands on an inner block of the pool (all four neighbours water), not at its default spot on the
    // west edge: at the edge it sits in water that is still spreading, and Grim's flow model differs
    // from vanilla's for a flowing block with water above it (proof: F5 §4.6, "Identical to vanilla means
    // identical Grim alerts"), so it would draw `Simulation` alerts that a vanilla client draws too. The row is about the main bot.
    cx.scene_in_spot(AREA, &[], SPAWN, Some((-1.0, -1.0)));
    let n = cx.name.clone();
    cx.r("fill 988 200 350 992 203 354 minecraft:water");
    cx.r(&format!("execute in minecraft:overworld run tp {n} {} 200.0 {} 0 0", SPAWN.0, SPAWN.2));
    cx.r(&format!("item replace entity {n} hotbar.0 with minecraft:trident[enchantments={{riptide:3}}]"));
    sleep(Duration::from_millis(1500));
    let before = refuse(cx, RefuseOp::Status);
    let out = cx.run("A20", "refuse_riptide_in_water", |cx| {
        let start = bot_pos(cx);
        watch();
        let i = refuse(cx, RefuseOp::UseMain);
        sleep(Duration::from_millis(500));
        let end = bot_pos(cx);
        let sent = unwatch();
        let delta = count_of(&i, "riptide") - count_of(&before, "riptide");
        let uses = sent_count(&sent, &["UseItem", "UseItemOn", "PlayerAction"]);
        let moved = ((end.0 - start.0).powi(2) + (end.2 - start.2).powi(2)).sqrt();
        let ok = delta == 1 && uses == 0 && moved < 0.5;
        (
            ok,
            format!(
                "riptide refusals +{delta} (want 1), reason {:?}; UseItem/UseItemOn/PlayerAction packets sent {uses} (want 0); horizontal move {moved:.2} blocks (want under 0.5), y {:.2} -> {:.2}; packets seen {}",
                i.refused.iter().find(|r| r.kind == "riptide").map(|r| r.reason),
                start.1,
                end.1,
                sent.len()
            ),
        )
    });
    cx.r("fill 988 200 350 992 203 354 minecraft:air");
    out
}

// Spec: specs/F5-version-layer.md §4.6 (client refusals: item use speed)
fn refuse_use_effects_speed(cx: &mut Ctx) -> Outcome {
    cx.scene_in(AREA, &[], SPAWN);
    let n = cx.name.clone();
    cx.r(&format!("item replace entity {n} hotbar.0 with minecraft:golden_apple[use_effects={{speed_multiplier:1.0}}]"));
    sleep(Duration::from_millis(500));
    let before = refuse(cx, RefuseOp::Status);
    cx.run("A20", "refuse_use_effects_speed", |cx| {
        watch();
        let i = refuse(cx, RefuseOp::UseMain);
        // Eating takes 32 ticks; wait past it.
        sleep(Duration::from_millis(2200));
        let apples = cx.count("golden_apple");
        let sent = unwatch();
        let delta = count_of(&i, "use_effects") - count_of(&before, "use_effects");
        let uses = sent_count(&sent, &["UseItem", "UseItemOn", "PlayerAction"]);
        let ok = delta == 1 && uses == 0 && apples == 1;
        (
            ok,
            format!(
                "use_effects refusals +{delta} (want 1), reason {:?}; UseItem/UseItemOn/PlayerAction packets sent {uses} (want 0); golden apples left {apples} (want 1: not eaten)",
                i.refused.iter().find(|r| r.kind == "use_effects").map(|r| r.reason)
            ),
        )
    })
}

/// One mount family: the row name, the summon (id and extra NBT) and the `BotView` kind.
struct Mount {
    row: &'static str,
    id: &'static str,
    nbt: &'static str,
    kind: &'static str,
    /// Distance north of the bot, in blocks (the happy ghast's box is 4 wide, so its edge is 2 blocks nearer: reach is measured to the box).
    dz: f64,
}

const MOUNTS: [Mount; 6] = [
    Mount { row: "refuse_mount_boat", id: "oak_boat", nbt: "", kind: "minecraft:oak_boat", dz: 3.0 },
    Mount { row: "refuse_mount_minecart", id: "minecart", nbt: "", kind: "minecraft:minecart", dz: 3.0 },
    Mount { row: "refuse_mount_horse", id: "horse", nbt: "", kind: "minecraft:horse", dz: 3.0 },
    Mount { row: "refuse_mount_pig", id: "pig", nbt: ",equipment:{saddle:{id:\"minecraft:saddle\",count:1}}", kind: "minecraft:pig", dz: 3.0 },
    Mount { row: "refuse_mount_camel", id: "camel", nbt: "", kind: "minecraft:camel", dz: 3.0 },
    Mount { row: "refuse_mount_happy_ghast", id: "happy_ghast", nbt: ",equipment:{body:{id:\"minecraft:white_harness\",count:1}}", kind: "minecraft:happy_ghast", dz: 4.0 },
];

// Spec: specs/F5-version-layer.md §4.6 (client refusals: mounting on a right-click)
fn refuse_mount(cx: &mut Ctx, m: &Mount) -> Outcome {
    cx.r("kill @e[tag=a20]");
    cx.scene_in(AREA, &[], SPAWN);
    cx.r(&format!(
        "summon minecraft:{} {} {} {} {{NoAI:1b,Silent:1b,PersistenceRequired:1b,Tags:[\"a20\"],Rotation:[0f,0f]{}}}",
        m.id,
        SPAWN.0,
        SPAWN.1,
        SPAWN.2 - m.dz,
        m.nbt
    ));
    let seen = inv(cx, InvOp::WaitEntity(m.kind)).seen;
    if seen.is_none() {
        eprintln!("warning: the bot never saw the {}", m.kind);
    }
    sleep(Duration::from_millis(500));
    let out = cx.run("A20", m.row, |cx| {
        let n = cx.name.clone();
        watch();
        let i = inv(cx, InvOp::OpenEntity { kind: m.kind, auto_crouch: false, aim: Aim::Entity });
        sleep(Duration::from_millis(500));
        let sent = unwatch();
        let passengers = cx.r(&format!("data get entity {SEL} Passengers"));
        let vehicle = cx.r(&format!("data get entity {n} RootVehicle"));
        let reason = i.open.as_ref().and_then(|o| o.refused);
        let interacts = sent_count(&sent, &["Interact"]);
        let ok = i.seen.is_some()
            && reason.is_some_and(|r| r.contains("would ride"))
            && interacts == 0
            && !passengers.contains("entity data")
            && !vehicle.contains("entity data");
        (
            ok,
            format!(
                "{} seen {}; click refusal {reason:?} (want \"would ride\"); Interact packets sent {interacts} (want 0); server Passengers {passengers:?}, bot RootVehicle {vehicle:?} (want no data); packets seen {}",
                m.kind,
                i.seen.is_some(),
                sent.len()
            ),
        )
    });
    cx.r("kill @e[tag=a20]");
    out
}

// Spec: specs/plans/ABILITY_CHECK.md (A20, client refusals)
pub fn a20(cx: &mut Ctx) -> Vec<Outcome> {
    let mut rows = vec![
        refuse_elytra_jump_in_air(cx),
        broken_elytra_jump_not_refused(cx),
        refuse_riptide_in_water(cx),
        refuse_use_effects_speed(cx),
    ];
    for m in &MOUNTS {
        rows.push(refuse_mount(cx, m));
    }
    let n = cx.name.clone();
    cx.r(&format!("clear {n}"));
    cx.clean_in(AREA);
    rows
}
