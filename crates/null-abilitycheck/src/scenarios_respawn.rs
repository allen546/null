//! A12: die, respawn, and know it happened (milestone SA). Plan: specs/plans/ABILITY_CHECK.md.
//! Area: corner (680, 342), floor y=199. The bot dies at `STAND` and its spawn point is `SPAWN_BLOCK`
//! (set with `/spawnpoint`, far from the death spot so the dropped items are not picked up again).
//! A dead bot keeps its brain but loses its skill at the death tick: the runner times the death with
//! the host's events, waits for the fresh skill after the respawn, and reads the death record from
//! its view; the `DeathProbe` reflex shows what the dead bot's own code saw. Server truth is read
//! over RCON, one NBT field at a time (Paper cuts long replies).

use std::{
    thread::sleep,
    time::{Duration, Instant},
};

use mc_adapter::{BotState, HostEvent};
use null_bot_api::{BlockOutcome, BlockPos, Hand, Life, SkillStatus};

use crate::{
    scenarios::{Ctx, Outcome, last_int, pos, ticks_between_packets, vanilla_ticks},
    skill::{Info, Op},
    skill_inventory::InvOp,
    skill_items::ItemOp,
    skill_respawn::{LifeInfo, LifeOp},
};

const AREA: (i32, i32) = (680, 342);
/// Where the bot dies (mid-area; the chest and the log of the rows are near it).
const STAND: (f64, f64, f64) = (695.5, 200.0, 357.5);
/// The spawn point (block position) and the bot's position after a respawn there.
const SPAWN_BLOCK: (i32, i32, i32) = (684, 200, 346);
const SPAWN: (f64, f64, f64) = (684.5, 200.0, 346.5);
/// The button turns on at the 20th screen tick (`DeathScreen#tick`).
const GATE: u64 = 20;
/// The bot sees the kill packet within about a tick plus the RCON round trip.
const DEAD_WITHIN_MS: u128 = 200;
const GOLDEN_APPLE_FOOD: i64 = 4;

// ---- helpers --------------------------------------------------------------------------------

fn floats(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn bot_pos(cx: &mut Ctx) -> [f64; 3] {
    let n = cx.name.clone();
    let f = floats(&cx.r(&format!("data get entity {n} Pos")));
    let k = f.len().saturating_sub(3);
    [f[k], f[k + 1], f[k + 2]]
}

fn health(cx: &mut Ctx) -> f64 {
    let n = cx.name.clone();
    floats(&cx.r(&format!("data get entity {n} Health"))).last().copied().unwrap_or(f64::NAN)
}

fn food(cx: &mut Ctx) -> i64 {
    let n = cx.name.clone();
    last_int(&cx.r(&format!("data get entity {n} foodLevel"))).unwrap_or(-1)
}

fn dimension(cx: &mut Ctx) -> String {
    let n = cx.name.clone();
    cx.r(&format!("data get entity {n} Dimension")).split('"').nth(1).unwrap_or("?").to_string()
}

fn near_spawn(p: [f64; 3]) -> bool {
    (p[0] - SPAWN.0).abs() < 0.6 && (p[1] - SPAWN.1).abs() < 0.6 && (p[2] - SPAWN.2).abs() < 0.6
}

/// A game rule's value ("Gamerule keep_inventory is currently set to: false").
fn rule(cx: &mut Ctx, name: &str) -> String {
    cx.r(&format!("gamerule {name}")).split_whitespace().last().unwrap_or("?").to_string()
}

fn life(cx: &mut Ctx, op: LifeOp) -> LifeInfo {
    match cx.call(Op::Life(op)).info {
        Info::Life(i) => i,
        i => panic!("expected a life report: {i:?}"),
    }
}

/// `/kill` the bot and wait for the host's `Dead` event; the milliseconds from the command's reply
/// to the event.
fn kill(cx: &mut Ctx) -> Result<u128, String> {
    while cx.events.try_recv().is_ok() {}
    let n = cx.name.clone();
    cx.r(&format!("kill {n}"));
    let t = Instant::now();
    loop {
        match cx.events.recv_timeout(Duration::from_secs(10)) {
            Ok(HostEvent::State(c)) if c.state == BotState::Dead => return Ok(t.elapsed().as_millis()),
            Ok(_) => {}
            Err(e) => return Err(format!("the bot never reported its death: {e}")),
        }
    }
}

/// Waits for the fresh skill after the respawn (the call blocks until it runs) and reads its view.
fn revive(cx: &mut Ctx) -> LifeInfo {
    life(cx, LifeOp::Status)
}

fn items(cx: &mut Ctx, op: ItemOp) -> crate::skill_items::ItemsInfo {
    match cx.call(Op::Items(op)).info {
        Info::Items(i) => i,
        i => panic!("expected an items report: {i:?}"),
    }
}

/// Fresh scene: bot at `STAND` with `hotbar`, spawn point set, `keep_inventory` as asked.
fn fresh(cx: &mut Ctx, hotbar: &[(u8, &str, u8)], keep: bool) {
    cx.scene_in(AREA, hotbar, STAND);
    let n = cx.name.clone();
    cx.r(&format!("spawnpoint {n} {} {} {}", SPAWN_BLOCK.0, SPAWN_BLOCK.1, SPAWN_BLOCK.2));
    cx.r(&format!("gamerule keep_inventory {keep}"));
    cx.r(&format!("effect give {n} minecraft:saturation 1 255 true"));
    sleep(Duration::from_millis(300));
}

/// Item counts on the server for the bot (`clear <bot> <item> 0` counts without removing).
fn server_counts(cx: &mut Ctx, names: &[&str]) -> Vec<(String, u32)> {
    names.iter().map(|i| (i.to_string(), cx.count(i))).collect()
}

/// The bot's snapshot of items at death, summed by item over the main slots, armour and offhand.
fn death_counts(info: &LifeInfo) -> std::collections::BTreeMap<String, u32> {
    let mut m = std::collections::BTreeMap::new();
    if let Some(d) = &info.last_death {
        for s in d.inventory.iter().chain(d.armor.iter()).chain(std::iter::once(&d.offhand)).flatten() {
            *m.entry(s.item.clone()).or_insert(0) += s.count as u32;
        }
    }
    m
}

/// Item entities of `item` with count `n` within 48 blocks of the death spot. The server decides
/// where a drop lands (a random throw, then a fall off the scene's edge; Wind Burst, explosions
/// and water move drops too), so the row checks that the drops exist, not where.
// Spec: specs/B2-survival-upkeep.md §4.7 (the drops are the server's)
fn dropped(cx: &mut Ctx, at: (f64, f64, f64), item: &str, n: u32) -> bool {
    cx.r(&format!(
        "execute positioned {} {} {} if entity @e[type=minecraft:item,distance=..48,nbt={{Item:{{id:\"minecraft:{item}\",count:{n}}}}}]",
        at.0, at.1, at.2
    ))
    .contains("passed")
}

/// Where the item entities of this kind are, anywhere in the world: tells a drop that flew off, one
/// that merged and one that never spawned apart (once a stone stack was not found).
fn find_anywhere(cx: &mut Ctx, at: (f64, f64, f64), item: &str) -> String {
    let sel = format!("@e[type=minecraft:item,nbt={{Item:{{id:\"minecraft:{item}\"}}}},sort=nearest,limit=1]");
    let n = last_int(&cx.r(&format!("execute if entity @e[type=minecraft:item,nbt={{Item:{{id:\"minecraft:{item}\"}}}}]"))).unwrap_or(0);
    if n == 0 {
        return format!("{item}: no item entity anywhere");
    }
    let near = |cx: &mut Ctx, path: &str| cx.r(&format!("execute positioned {} {} {} run data get entity {sel} {path}", at.0, at.1, at.2));
    let (p, c) = (near(cx, "Pos"), near(cx, "Item.count"));
    format!("{item}: {n} item entit(ies); nearest {} count {:?}", p.rsplit(": ").next().unwrap_or(&p), last_int(&c))
}

fn clear_items(cx: &mut Ctx) {
    cx.r("kill @e[type=minecraft:item,x=680,y=199,z=342,dx=21,dy=10,dz=21]");
}

fn dist(a: [f64; 3], b: (f64, f64, f64)) -> f64 {
    ((a[0] - b.0).powi(2) + (a[1] - b.1).powi(2) + (a[2] - b.2).powi(2)).sqrt()
}

/// The checks every normal death shares: the event came within a tick, the record has the position
/// and message, the respawn packet went out exactly 20 screen ticks after the screen opened, and
/// the bot is alive at the spawn point.
fn normal_death_checks(cx: &mut Ctx, lat: u128, info: &LifeInfo, from: (f64, f64, f64)) -> (bool, String) {
    let name = cx.name.clone();
    let d = info.last_death.clone();
    let r = info.last_respawn.clone();
    sleep(Duration::from_millis(1200));
    let (hp, fd, p, dim) = (health(cx), food(cx), bot_pos(cx), dimension(cx));
    let at = d.as_ref().map(|d| (d.position.x, d.position.y, d.position.z));
    let held = match (&d, &r) {
        (Some(d), Some(r)) => Some(r.sent_tick - d.tick),
        _ => None,
    };
    let ok = lat <= DEAD_WITHIN_MS
        && d.as_ref().is_some_and(|d| {
            (d.position.x - from.0).abs() < 0.6
                && (d.position.z - from.2).abs() < 0.6
                && d.message.contains(&name)
                && d.dimension == "minecraft:overworld"
        })
        && r.as_ref().is_some_and(|r| !r.immediate && r.screen_ticks == GATE as u32 && r.completed_tick.is_some())
        && held == Some(GATE)
        && info.life == Life::Alive
        && hp == 20.0
        && fd == 20
        && near_spawn(p)
        && dim == "minecraft:overworld";
    (
        ok,
        format!(
            "bot saw the death {lat} ms after /kill (want at most {DEAD_WITHIN_MS}); record: position {at:?} (died at {from:?}), message {:?}, dimension {:?}; respawn packet {held:?} bot ticks after the screen opened (want exactly {GATE}), screen ticks {:?}, immediate {:?}, answered at tick {:?}; server: Health {hp} (want 20.0), food {fd} (want 20), position {p:?} (spawn point {SPAWN:?}), dimension {dim}",
            d.as_ref().map(|d| d.message.clone()),
            d.as_ref().map(|d| d.dimension.clone()),
            r.as_ref().map(|r| r.screen_ticks),
            r.as_ref().map(|r| r.immediate),
            r.as_ref().and_then(|r| r.completed_tick),
        ),
    )
}

// ---- rows -----------------------------------------------------------------------------------

// Spec: specs/F1-swarm-runtime.md §4.4 (dead, respawning, idle; death reported with position)
fn kill_and_respawn(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[], false);
    let deaths0 = life(cx, LifeOp::Status).deaths;
    cx.run("A12", "kill_and_respawn_twice_in_a_row", |cx| {
        let mut ev = Vec::new();
        let mut ok = true;
        let mut from = STAND;
        let mut last = None;
        for i in 0..2 {
            let lat = match kill(cx) {
                Ok(l) => l,
                Err(e) => return (false, e),
            };
            let info = revive(cx);
            let (o, e) = normal_death_checks(cx, lat, &info, from);
            ev.push(format!("death {}: {e}; death count {} (want {})", i + 1, info.deaths, deaths0 + i + 1));
            ok &= o && info.deaths == deaths0 + i + 1;
            from = SPAWN;
            last = Some(info);
        }
        // Back in play: a short step and a slot change, with no warnings (the log check adds that).
        let p0 = bot_pos(cx);
        items(cx, ItemOp::Walk(10));
        sleep(Duration::from_millis(400));
        let p1 = bot_pos(cx);
        let snap = cx.snap_after(Op::SelectSlot(3));
        let moved = dist(p1, (p0[0], p0[1], p0[2]));
        ok &= moved > 0.3 && snap.selected == 3;
        ev.push(format!("then walked {moved:.2} blocks and selected slot {} (want 3) with the server quiet", snap.selected));
        let _ = last;
        (ok, ev.join(" | "))
    })
}

// Spec: specs/B2-survival-upkeep.md §4.7 (inventory snapshot at death; the drops are the server's)
fn inventory_drops(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5), (4, "golden_apple", 3), (8, "iron_axe", 1)], false);
    cx.run("A12", "inventory_drops_with_keep_inventory_off", |cx| {
        let lat = match kill(cx) {
            Ok(l) => l,
            Err(e) => return (false, e),
        };
        let info = revive(cx);
        let snap = death_counts(&info);
        sleep(Duration::from_millis(800));
        let left = server_counts(cx, &["stone", "golden_apple", "iron_axe"]);
        let drops = [("stone", 5), ("golden_apple", 3), ("iron_axe", 1)].map(|(i, n)| (i, n, dropped(cx, STAND, i, n)));
        let want = |i: &str, n: u32| snap.get(&format!("minecraft:{i}")) == Some(&n);
        let ok = lat <= DEAD_WITHIN_MS
            && want("stone", 5)
            && want("golden_apple", 3)
            && want("iron_axe", 1)
            && left.iter().all(|(_, n)| *n == 0)
            && drops.iter().all(|d| d.2);
        let missing: Vec<String> = drops.iter().filter(|d| !d.2).map(|d| find_anywhere(cx, STAND, d.0)).collect();
        clear_items(cx);
        (
            ok,
            format!(
                "death snapshot {snap:?} (want stone 5, golden_apple 3, iron_axe 1); after the respawn the server counts {left:?} in the bot's inventory (want all 0); item entities near the death spot with the same counts: {drops:?}{}",
                if missing.is_empty() { String::new() } else { format!("; missing drops: {}", missing.join(", ")) }
            ),
        )
    })
}

// Spec: specs/B2-survival-upkeep.md §4.7 (keep_inventory: the same items after the respawn)
fn keep_inventory(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5), (4, "golden_apple", 3), (8, "iron_axe", 1)], true);
    let out = cx.run("A12", "keep_inventory_keeps_the_items", |cx| {
        let lat = match kill(cx) {
            Ok(l) => l,
            Err(e) => return (false, e),
        };
        let info = revive(cx);
        let snap = death_counts(&info);
        sleep(Duration::from_millis(800));
        let kept = server_counts(cx, &["stone", "golden_apple", "iron_axe"]);
        let bot = cx.snap();
        let ok = lat <= DEAD_WITHIN_MS
            && kept == vec![("stone".into(), 5), ("golden_apple".into(), 3), ("iron_axe".into(), 1)]
            && bot.counts.get("minecraft:stone") == Some(&5)
            && bot.counts.get("minecraft:golden_apple") == Some(&3)
            && snap.get("minecraft:stone") == Some(&5);
        (
            ok,
            format!(
                "server counts after the respawn {kept:?} (want stone 5, golden_apple 3, iron_axe 1); the bot's own view {:?}; death snapshot {snap:?}",
                bot.counts
            ),
        )
    });
    cx.r("gamerule keep_inventory false");
    out
}

// Spec: specs/F1-swarm-runtime.md §4.4 (immediate respawn: no screen, no delay)
fn immediate_respawn(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[], false);
    let was = rule(cx, "immediate_respawn");
    cx.r("gamerule immediate_respawn true");
    sleep(Duration::from_millis(600));
    let out = cx.run("A12", "immediate_respawn_has_no_delay", |cx| {
        let mut ok = true;
        let mut ev = Vec::new();
        // Three deaths: how many ticks the answer takes varies, the rule for the count does not.
        for i in 0..3 {
            let lat = match kill(cx) {
                Ok(l) => l,
                Err(e) => return (false, e),
            };
            let info = revive(cx);
            sleep(Duration::from_millis(1200));
            let (hp, p) = (health(cx), bot_pos(cx));
            let d = info.last_death.clone();
            let r = info.last_respawn.clone();
            let delay = match (&d, &r) {
                (Some(d), Some(r)) => Some(r.sent_tick as i64 - d.tick as i64),
                _ => None,
            };
            // Vanilla: the kill packet's handler sends one, then `Minecraft#tick` sends one every
            // tick until the respawn packet replaces the player.
            let want_sends = match (&d, &r) {
                (Some(d), Some(r)) => r.completed_tick.map(|c| 1 + c.saturating_sub(d.tick) as u32),
                _ => None,
            };
            let sends = r.as_ref().map(|r| r.sends);
            ok &= lat <= DEAD_WITHIN_MS
                && r.as_ref().is_some_and(|r| r.immediate && r.screen_ticks == 0 && r.completed_tick.is_some())
                && delay == Some(0)
                && sends.is_some()
                && sends == want_sends
                && hp == 20.0
                && near_spawn(p);
            ev.push(format!(
                "death {}: {lat} ms to see it; respawn packet {delay:?} ticks after the kill packet (want 0), immediate {:?}, screen ticks {:?}; respawn packets sent {sends:?} (vanilla's rule: 1 from the kill packet + 1 per tick until the answer = {want_sends:?}); server Health {hp}, position {p:?}",
                i + 1,
                r.as_ref().map(|r| r.immediate),
                r.as_ref().map(|r| r.screen_ticks)
            ));
        }
        (ok, format!("rule on (was {was}): {}", ev.join(" | ")))
    });
    cx.r(&format!("gamerule immediate_respawn {was}"));
    sleep(Duration::from_millis(600));
    out
}

// Spec: specs/F1-swarm-runtime.md §4.4 (a respawn asked while alive sends nothing)
fn respawn_refused_alive(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[], false);
    cx.run("A12", "respawn_refused_while_alive", |cx| {
        let before = life(cx, LifeOp::Status);
        let after = life(cx, LifeOp::Respawn);
        sleep(Duration::from_millis(500));
        let hp = health(cx);
        let ok = after.refusal == Some("not dead")
            && after.last_respawn == before.last_respawn
            && after.deaths == before.deaths
            && after.life == Life::Alive
            && hp == 20.0;
        (
            ok,
            format!(
                "refusal {:?} (want \"not dead\"); last respawn record unchanged: {} (so no packet went out; a sent one would also draw Grim BadPacketsM, checked in the log); deaths {} -> {}; server Health {hp}",
                after.refusal,
                after.last_respawn == before.last_respawn,
                before.deaths,
                after.deaths
            ),
        )
    })
}

// Spec: specs/F1-swarm-runtime.md §4.4 (death replaces the open container screen)
fn death_with_open_chest(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5)], true);
    let chest = pos(695, 200, 360);
    cx.r("setblock 695 200 360 minecraft:chest[facing=north]");
    cx.r("item replace block 695 200 360 container.0 with minecraft:cobblestone 7");
    cx.wait_block(chest, "minecraft:chest");
    let out = cx.run("A12", "death_with_open_chest_closes_it", |cx| {
        let open = match cx.call(Op::Inv(InvOp::OpenContainer(chest, None))).info {
            Info::Inv(i) => i,
            i => return (false, format!("bot reported {i:?}")),
        };
        let was = open.window.as_ref().map(|w| (w.id, w.kind));
        if open.window.as_ref().is_none_or(|w| w.id == 0) {
            return (false, format!("the chest did not open: {was:?}, last open {:?}", open.open));
        }
        let lat = match kill(cx) {
            Ok(l) => l,
            Err(e) => return (false, e),
        };
        let info = revive(cx);
        sleep(Duration::from_millis(1000));
        // The player window works: pick up the stone and put it back, with no server correction.
        let opened = cx.call(Op::Inv(InvOp::Open));
        let Info::Inv(_) = opened.info else { return (false, format!("bot reported {:?}", opened.info)) };
        let pick = match cx.call(Op::Inv(InvOp::Click(Some(36), null_bot_api::Click::Left))).info {
            Info::Inv(i) => i,
            i => return (false, format!("bot reported {i:?}")),
        };
        let put = match cx.call(Op::Inv(InvOp::Click(Some(36), null_bot_api::Click::Left))).info {
            Info::Inv(i) => i,
            i => return (false, format!("bot reported {i:?}")),
        };
        let closed = cx.call(Op::Inv(InvOp::Close));
        let _ = closed;
        let clean = |i: &crate::skill_inventory::InvInfo| {
            i.click.as_ref().is_some_and(|c| c.refused.is_none() && c.corrected.is_empty() && c.full_resends == 0)
        };
        let stone = cx.count("stone");
        let ok = lat <= DEAD_WITHIN_MS && info.window.is_none() && clean(&pick) && clean(&put) && stone == 5;
        (
            ok,
            format!(
                "chest open before the kill: window {was:?}; after the respawn the bot has window {:?} (want none); click in the player window picked the stone: {:?}, put it back: {:?} (want no refusal, no correction, no resend); server stone count {stone} (want 5)",
                info.window, pick.click, put.click
            ),
        )
    });
    cx.r("gamerule keep_inventory false");
    out
}

// Spec: specs/F1-swarm-runtime.md §4.4 (a fresh player: not crouching, not using an item)
fn death_mid_action(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "golden_apple", 3)], true);
    let out = cx.run("A12", "death_while_crouching_and_eating", |cx| {
        let n = cx.name.clone();
        cx.call(Op::Sneak(true));
        // Eating: golden apples can be eaten at any food level.
        let seq = cx.send(Op::Items(ItemOp::Use(Hand::Main)));
        if let Err(e) = cx.recv(seq, Duration::from_secs(30)) {
            return (false, format!("bot never started eating: {e}"));
        }
        // The op reports two ticks after the start; the use runs 32 ticks. Kill about 10 ticks in.
        if let Err(e) = cx.finish_op(seq, Duration::from_secs(30)) {
            return (false, format!("bot never reported the first use: {e}"));
        }
        sleep(Duration::from_millis(300));
        let lat = match kill(cx) {
            Ok(l) => l,
            Err(e) => return (false, e),
        };
        let info = revive(cx);
        sleep(Duration::from_millis(1200));
        let apples = cx.count("golden_apple");
        // Eat one to the end after the respawn: lower food first so the +4 shows (as A5).
        cx.r(&format!("effect give {n} minecraft:hunger 30 255 true"));
        cx.poll(15.0, |cx| food(cx) <= 12);
        cx.r(&format!("effect clear {n} minecraft:hunger"));
        sleep(Duration::from_millis(500));
        let f0 = food(cx);
        let seq = cx.send(Op::Items(ItemOp::Use(Hand::Main)));
        if let Err(e) = cx.recv(seq, Duration::from_secs(30)) {
            return (false, format!("bot never started the second use: {e}"));
        }
        cx.poll(4.0, |cx| food(cx) > f0);
        if let Err(e) = cx.finish_op(seq, Duration::from_secs(30)) {
            return (false, format!("bot never finished the second use: {e}"));
        }
        sleep(Duration::from_millis(300));
        let (f1, apples1) = (food(cx), cx.count("golden_apple"));
        let after = life(cx, LifeOp::Status);
        let want = (f0 + GOLDEN_APPLE_FOOD).min(20);
        let ok = lat <= DEAD_WITHIN_MS
            && !info.using_item
            && !info.sneaking
            && apples == 3
            && f0 <= 12
            && f1 == want
            && apples1 == 2
            && !after.using_item;
        (
            ok,
            format!(
                "killed about 10 ticks into eating while crouching; after the respawn using an item: {} (want false), crouching: {} (want false), apples kept {apples}; then ate one to the end: server food {f0} -> {f1} (want {want}), apples {apples} -> {apples1} (want 2), bot using afterwards: {}",
                info.using_item, info.sneaking, after.using_item
            ),
        )
    });
    cx.r("gamerule keep_inventory false");
    out
}

// Spec: specs/F1-swarm-runtime.md §4.4 (respawn in another dimension; the world works after)
fn death_in_nether(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[], false);
    let n = cx.name.clone();
    for c in [
        "execute in minecraft:the_nether run forceload add 690 350 700 360".to_string(),
        "execute in minecraft:the_nether run fill 690 70 350 700 70 360 minecraft:stone".to_string(),
        "execute in minecraft:the_nether run fill 690 71 350 700 76 360 minecraft:air".to_string(),
        format!("execute in minecraft:the_nether run tp {n} 695.5 71 355.5"),
    ] {
        cx.r(&c);
    }
    sleep(Duration::from_millis(2500));
    let log: BlockPos = pos(684, 200, 349);
    let out = cx.run("A12", "death_in_the_nether_respawns_in_the_overworld", |cx| {
        let in_nether = dimension(cx);
        let lat = match kill(cx) {
            Ok(l) => l,
            Err(e) => return (false, e),
        };
        let info = revive(cx);
        sleep(Duration::from_millis(1500));
        let (dim, p, hp) = (dimension(cx), bot_pos(cx), health(cx));
        // A block in the test area breaks at vanilla time.
        cx.r("setblock 684 200 349 minecraft:oak_log");
        cx.wait_block(log, "minecraft:oak_log");
        let want = ticks_between_packets(vanilla_ticks(2.0, 1.0, true)) as u64;
        let r = cx.call(Op::Break(log));
        let Info::Action(a) = r.info else { return (false, format!("bot reported {:?}", r.info)) };
        sleep(Duration::from_millis(400));
        let air = cx.block_is(log, "air");
        let ticks = a.finish_tick.map(|f| f - a.start_tick);
        let d = info.last_death.clone();
        let ok = in_nether == "minecraft:the_nether"
            && lat <= DEAD_WITHIN_MS
            && d.as_ref().is_some_and(|d| d.dimension == "minecraft:the_nether")
            && dim == "minecraft:overworld"
            && near_spawn(p)
            && hp == 20.0
            && a.outcome == BlockOutcome::Done
            && ticks == Some(want)
            && air;
        (
            ok,
            format!(
                "bot was in {in_nether} when killed; death record dimension {:?}, position {:?}; after the respawn the server says {dim} at {p:?} (spawn point {SPAWN:?}), Health {hp}; oak log by hand: outcome {:?}, {ticks:?} ticks between start and finish (want {want}), air on server {air}",
                d.as_ref().map(|d| d.dimension.clone()),
                d.as_ref().map(|d| (d.position.x, d.position.y, d.position.z)),
                a.outcome
            ),
        )
    });
    for c in [
        "execute in minecraft:the_nether run fill 690 70 350 700 76 360 minecraft:air".to_string(),
        "execute in minecraft:the_nether run forceload remove 690 350 700 360".to_string(),
    ] {
        cx.r(&c);
    }
    out
}


/// What one death's events looked like to the runner.
struct DeathEvents {
    /// Milliseconds from `/kill`'s reply to each event.
    dead_ms: Option<u128>,
    died_ms: Option<u128>,
    ended_ms: Option<u128>,
    ended: Option<(String, SkillStatus, u64)>,
    died: Option<(u64, Option<null_bot_api::Death>)>,
}

/// `/kill` the bot and collect the host's events about the death.
fn kill_events(cx: &mut Ctx) -> DeathEvents {
    while cx.events.try_recv().is_ok() {}
    let n = cx.name.clone();
    cx.r(&format!("kill {n}"));
    let t = Instant::now();
    let mut e = DeathEvents { dead_ms: None, died_ms: None, ended_ms: None, ended: None, died: None };
    while t.elapsed() < Duration::from_secs(5) && (e.dead_ms.is_none() || e.died_ms.is_none() || e.ended_ms.is_none()) {
        match cx.events.recv_timeout(Duration::from_millis(200)) {
            Ok(HostEvent::State(c)) if c.state == BotState::Dead => e.dead_ms = Some(t.elapsed().as_millis()),
            Ok(HostEvent::SkillEnded { skill, status, tick, .. }) => {
                e.ended_ms = Some(t.elapsed().as_millis());
                e.ended = Some((skill, status, tick));
            }
            Ok(HostEvent::Died { tick, death, .. }) => {
                e.died_ms = Some(t.elapsed().as_millis());
                e.died = Some((tick, death));
            }
            _ => {}
        }
    }
    e
}

// Spec: specs/F1-swarm-runtime.md §4.4 (the death tick: the job ends, the death is reported, reflexes see it)
fn skill_on_death_end(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5)], false);
    cx.run("A12", "skill_on_death_end", |cx| {
        cx.probe.lock().unwrap().reset(None, None);
        // A long walk is the running skill; the kill comes while it runs.
        cx.send(Op::Items(ItemOp::Walk(400)));
        sleep(Duration::from_millis(500));
        let before = bot_pos(cx);
        let e = kill_events(cx);
        let info = revive(cx);
        let probe = std::mem::take(&mut cx.probe.lock().unwrap().ticks);
        let (Some((skill, status, ended_tick)), Some((died_tick, Some(death)))) = (e.ended.clone(), e.died.clone()) else {
            return (false, format!("events missing: skill ended {:?}, died {:?}, dead state after {:?} ms", e.ended, e.died.as_ref().map(|d| d.0), e.dead_ms));
        };
        let kill_tick = death.tick;
        let stone: u32 = death.inventory.iter().flatten().filter(|s| s.item == "minecraft:stone").map(|s| s.count as u32).sum();
        let consecutive = probe.windows(2).all(|w| w[1].0 == w[0].0 + 1);
        let dead_n = probe.iter().filter(|(_, l)| matches!(l, Life::Dead { .. })).count();
        let pending_n = probe.iter().filter(|(_, l)| matches!(l, Life::RespawnPending { .. })).count();
        let ordered = probe.iter().position(|(_, l)| matches!(l, Life::RespawnPending { .. })).is_none_or(|i| {
            probe[..i].iter().all(|(_, l)| matches!(l, Life::Dead { .. })) && probe[i..].iter().all(|(_, l)| matches!(l, Life::RespawnPending { .. }))
        });
        let first = probe.first().copied();
        let last = probe.last().map(|x| x.0);
        let completed = info.last_respawn.as_ref().and_then(|r| r.completed_tick);
        let first_dead_since = match first {
            Some((_, Life::Dead { since_tick })) => Some(since_tick),
            _ => None,
        };
        let near = dist(
            before,
            (death.position.x, death.position.y, death.position.z),
        );
        let ok = skill == "ability-check"
            && status == SkillStatus::Failed("died".into())
            && ended_tick == died_tick
            && ended_tick >= kill_tick
            && ended_tick - kill_tick <= 1
            && e.dead_ms.is_some_and(|m| m <= DEAD_WITHIN_MS)
            && e.died_ms.is_some_and(|m| m <= DEAD_WITHIN_MS + 100)
            && e.ended_ms.is_some_and(|m| m <= DEAD_WITHIN_MS + 100)
            && near < 3.0
            && stone == 5
            && death.dimension == "minecraft:overworld"
            && first.is_some_and(|f| f.0 == ended_tick)
            && first_dead_since.is_some_and(|s| s + 1 >= kill_tick && s <= ended_tick)
            && consecutive
            && ordered
            && dead_n >= 15
            && pending_n >= 1
            && completed.is_some_and(|c| last == Some(c))
            && info.life == Life::Alive;
        (
            ok,
            format!(
                "skill {skill:?} ended {status:?} at host tick {ended_tick}; death event at tick {died_tick} (want the same); kill packet at tick {kill_tick} (so {} tick(s) later, want at most 1); wall time after /kill: dead state {:?} ms, death event {:?} ms, skill end {:?} ms (want at most {DEAD_WITHIN_MS}); record: position {:?} is {near:.2} blocks from the server's {before:?} just before, stone {stone} (want 5), dimension {:?}; the bot's own reflex saw {} dead ticks from tick {:?} to {last:?} (consecutive {consecutive}): Dead x{dead_n} then RespawnPending x{pending_n} (in that order {ordered}); first Dead since {first_dead_since:?}; the respawn arrived at tick {completed:?} (want the last dead tick); alive afterwards {:?}",
                ended_tick.saturating_sub(kill_tick),
                e.dead_ms, e.died_ms, e.ended_ms,
                (death.position.x, death.position.y, death.position.z),
                death.dimension,
                probe.len(),
                first.map(|f| f.0),
                info.life
            ),
        )
    })
}

/// Packet names a dead vanilla client still sends: the position packets, the tick end, answers to
/// the server and the respawn (`LocalPlayer#tick`, `Minecraft#tick`, `DeathScreen`).
fn allowed_while_dead(name: &str) -> bool {
    name.starts_with("MovePlayer")
        || matches!(
            name,
            "ClientTickEnd" | "KeepAlive" | "Pong" | "ChunkBatchReceived" | "AcceptTeleportation" | "ClientCommand"
        )
}

fn rotation(cx: &mut Ctx) -> Vec<f64> {
    let n = cx.name.clone();
    floats(&cx.r(&format!("data get entity {n} Rotation")))
}

fn selected_slot(cx: &mut Ctx) -> i64 {
    let n = cx.name.clone();
    last_int(&cx.r(&format!("data get entity {n} SelectedItemSlot"))).unwrap_or(-1)
}

// Spec: specs/F1-swarm-runtime.md §4.4 (a dead bot sends only what a death screen sends)
fn nothing_sent_while_dead(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5), (2, "iron_axe", 1)], true);
    let target = pos(695, 199, 358);
    cx.r("summon minecraft:pig 697.5 200 357.5 {NoAI:1b,Silent:1b,PersistenceRequired:1b}");
    sleep(Duration::from_millis(500));
    let out = cx.run("A12", "nothing_sent_while_dead", |cx| {
        let snap = cx.snap_after(Op::SelectSlot(2));
        sleep(Duration::from_millis(300));
        let (slot0, rot0) = (selected_slot(cx), rotation(cx));
        cx.probe.lock().unwrap().reset(Some(2), Some(target));
        let e = kill_events(cx);
        // The script fires at the second dead tick; the 20-tick gate is a second away.
        sleep(Duration::from_millis(400));
        let (slot1, rot1) = (selected_slot(cx), rotation(cx));
        let above = cx.block_is(pos(695, 200, 358), "air");
        let floor = cx.block_is(target, "stone");
        let pig = floats(&cx.r("data get entity @e[type=minecraft:pig,x=680,y=199,z=342,dx=21,dy=10,dz=21,limit=1] Health")).last().copied();
        let axe = cx.count("iron_axe");
        let info = revive(cx);
        let d = std::mem::take(&mut *cx.probe.lock().unwrap());
        let refused = |name: &str| d.refused.iter().find(|r| r.action == name).map(|r| (r.count, r.reason));
        let missing: Vec<&str> = [
            "walk", "sprint", "sneak", "jump", "look", "select_slot", "swap_offhand", "attack", "use_item",
            "release_use", "look_at_block", "break_block", "stop_break", "place_block", "use_block", "goto",
        ]
        .into_iter()
        .filter(|a| refused(a).is_none_or(|(n, why)| n == 0 || why.is_empty()))
        .collect();
        let bad_packets: Vec<&(String, u32)> = d.sent.iter().filter(|(n, _)| !allowed_while_dead(n)).collect();
        let input_leak: u32 = d.sent.iter().filter(|(n, _)| n == "PlayerInput" || n == "PlayerCommand").map(|(_, c)| *c).sum();
        let ok = snap.selected == 2
            && e.dead_ms.is_some()
            && d.fired.is_some()
            && missing.is_empty()
            && d.open_refusal == Some("dead")
            && d.click_refused == Some("dead")
            && bad_packets.is_empty()
            && input_leak == 0
            && d.sent.iter().any(|(n, _)| n == "ClientTickEnd")
            && slot0 == 2
            && slot1 == 2
            && rot0.len() >= 2
            && rot1.len() >= 2
            && rot0.iter().zip(&rot1).all(|(a, b)| (a - b).abs() < 0.01)
            && above
            && floor
            && pig == Some(10.0)
            && axe == 1
            && info.life == Life::Alive;
        (
            ok,
            format!(
                "dead bot tried {} commands at dead tick {:?}; refused with a reason: all of {:?} except {missing:?}; open_inventory refusal {:?}, click refusal {:?} (want \"dead\" both); attack target and entities in view {:?}; packets it sent while dead {:?} (allowed: position packets, tick end, keep-alive, pong, teleport answer, respawn; not allowed found: {bad_packets:?}; input/command packets {input_leak}, want 0); server while dead: SelectedItemSlot {slot0} -> {slot1} (want 2), Rotation {rot0:?} -> {rot1:?} (want equal); block above the floor is air {above}, floor block still stone {floor}, pig Health {pig:?} (want 10), iron axe kept {axe}",
                d.issued.len(),
                d.fired,
                d.issued,
                d.open_refusal,
                d.click_refused,
                d.target,
                d.sent
            ),
        )
    });
    cx.r("kill @e[type=minecraft:pig,x=680,y=199,z=342,dx=21,dy=10,dz=21]");
    cx.r("gamerule keep_inventory false");
    out
}


// Spec: specs/F1-swarm-runtime.md §4.4 (a skill that returns Keep goes on after the respawn)
fn skill_on_death_keep(cx: &mut Ctx) -> Outcome {
    fresh(cx, &[(0, "stone", 5)], true);
    let out = cx.run("A12", "skill_on_death_keep", |cx| {
        *crate::skill_respawn::KEEP_LOG.lock().unwrap() = Default::default();
        cx.probe.lock().unwrap().reset(None, None);
        cx.probe.lock().unwrap().passive = true;
        let slot0 = selected_slot(cx);
        crate::skill_respawn::KEEP_ARMED.store(true, std::sync::atomic::Ordering::Relaxed);
        sleep(Duration::from_millis(400));
        while cx.events.try_recv().is_ok() {}
        let n = cx.name.clone();
        cx.r(&format!("kill {n}"));
        let t = Instant::now();
        let (mut died, mut ended, mut done_ms) = (None, None, None);
        while t.elapsed() < Duration::from_secs(10) && done_ms.is_none() {
            match cx.events.recv_timeout(Duration::from_millis(200)) {
                Ok(HostEvent::Died { tick, death, .. }) => died = Some((tick, death.is_some(), t.elapsed().as_millis())),
                Ok(HostEvent::SkillEnded { status, tick, .. }) => {
                    if status == SkillStatus::Done {
                        done_ms = Some(t.elapsed().as_millis());
                    }
                    ended = Some((status, tick));
                }
                _ => {}
            }
        }
        // Read the server a little after the Done: slot 6 chosen after the respawn.
        sleep(Duration::from_millis(300));
        let slot1 = selected_slot(cx);
        crate::skill_respawn::KEEP_ARMED.store(false, std::sync::atomic::Ordering::Relaxed);
        let log = std::mem::take(&mut *crate::skill_respawn::KEEP_LOG.lock().unwrap());
        let steps = &log.steps;
        let dead_ticks: Vec<u64> = steps.iter().filter(|s| s.life != Life::Alive).map(|s| s.tick).collect();
        let first_dead = steps.iter().find(|s| s.life != Life::Alive);
        let numbered = steps.iter().enumerate().all(|(i, s)| s.step == i as u64 + 1);
        // Ticks are consecutive while dead; after the respawn the bot loads the new world for a few
        // ticks (no view, no step) before the skill steps on.
        let consecutive = steps.windows(2).filter(|w| w[0].life != Life::Alive && w[1].life != Life::Alive).all(|w| w[1].tick == w[0].tick + 1);
        let load_gap = steps.windows(2).find(|w| w[0].life != Life::Alive && w[1].life == Life::Alive).map(|w| w[1].tick - w[0].tick);
        let alive_after = steps.iter().rev().take_while(|s| s.life == Life::Alive).count();
        let select_refused = log.refused.iter().any(|r| r.action == "select_slot" && r.count > 0);
        // A fresh instance would restart at step 1 after the respawn: the numbering never does.
        let ok = died.as_ref().is_some_and(|d| d.1 && d.2 <= DEAD_WITHIN_MS + 100)
            && ended.as_ref().is_some_and(|e| e.0 == SkillStatus::Done)
            && first_dead.is_some_and(|f| Some(f.tick) == died.as_ref().map(|d| d.0) && matches!(f.life, Life::Dead { .. }))
            && dead_ticks.len() >= 15
            && numbered
            && consecutive
            && alive_after >= 10
            && select_refused
            && slot0 == 0
            && slot1 == 6;
        (
            ok,
            format!(
                "the skill answered Keep: death event {died:?} (tick, record present, ms); the skill then ended {ended:?}; it was stepped {} times, on consecutive ticks while dead ({consecutive}) and {load_gap:?} ticks after the last dead one once alive (the new world loading), with its own step numbers running 1..{} without a restart ({numbered}), {} of them dead (from tick {:?}, first life {:?}) and {alive_after} alive after the respawn; its select_slot while dead was refused ({select_refused}); server SelectedItemSlot {slot0} before, {slot1} after the skill's own change post-respawn (want 0 then 6; the slot 5 it tried while dead never reached the server)",
                steps.len(),
                steps.len(),
                dead_ticks.len(),
                dead_ticks.first(),
                first_dead.map(|f| f.life)
            ),
        )
    });
    // The finished skill is gone; one more death gives the bot the factory's skill again.
    let _ = kill(cx);
    let _ = revive(cx);
    cx.r("gamerule keep_inventory false");
    out
}

// Spec: specs/plans/ABILITY_CHECK.md (A12: the bot is alive at spawn after /kill, and its view says it died)
pub fn a12(cx: &mut Ctx) -> Vec<Outcome> {
    let out = vec![
        kill_and_respawn(cx),
        inventory_drops(cx),
        keep_inventory(cx),
        immediate_respawn(cx),
        respawn_refused_alive(cx),
        death_with_open_chest(cx),
        death_mid_action(cx),
        death_in_nether(cx),
        skill_on_death_end(cx),
        skill_on_death_keep(cx),
        nothing_sent_while_dead(cx),
    ];
    cx.r("gamerule keep_inventory false");
    cx.clean_in(AREA);
    out
}
