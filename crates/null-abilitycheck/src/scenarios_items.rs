//! A5-A8: eating, shield, attack with cooldown and crits, `recent_attackers`.
//! Plan: specs/plans/ABILITY_CHECK.md. Area: corner (530, 342), floor y=199, accounts `null_itm`
//! (and `null_itm2` as the attacker in A8). Server truth is read over RCON.

use std::{thread::sleep, time::Duration};

use null_bot_api::Hand;

use crate::{
    scenarios::{Ctx, Outcome, last_int},
    skill::{Info, Op},
    skill_items::{AttackHow, ItemOp, ItemsInfo},
};

const AREA: (i32, i32) = (530, 342);
/// The bot stands mid-area facing +Z (yaw 0), with 12 blocks ahead of it.
const SPAWN: (f64, f64, f64) = (540.5, 200.0, 349.5);
/// Selector bounds for this area.
const BOX: &str = "x=530,y=199,z=342,dx=21,dy=10,dz=21";

// ---- helpers --------------------------------------------------------------------------------

/// Every number in an RCON reply, in order ("[540.5d, 200.0d, 349.5d]" gives three).
fn floats(s: &str) -> Vec<f64> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
        .filter(|t| t.chars().any(|c| c.is_ascii_digit()))
        .filter_map(|t| t.parse().ok())
        .collect()
}

fn health_of(cx: &mut Ctx, who: &str) -> f64 {
    floats(&cx.r(&format!("data get entity {who} Health"))).last().copied().unwrap_or(f64::NAN)
}

fn food(cx: &mut Ctx) -> i64 {
    let n = cx.name.clone();
    last_int(&cx.r(&format!("data get entity {n} foodLevel"))).unwrap_or(-1)
}

fn saturation(cx: &mut Ctx) -> f64 {
    let n = cx.name.clone();
    floats(&cx.r(&format!("data get entity {n} foodSaturationLevel"))).last().copied().unwrap_or(f64::NAN)
}

fn bot_pos(cx: &mut Ctx) -> [f64; 3] {
    let n = cx.name.clone();
    let f = floats(&cx.r(&format!("data get entity {n} Pos")));
    let k = f.len().saturating_sub(3);
    [f[k], f[k + 1], f[k + 2]]
}

fn game_time(cx: &mut Ctx) -> i64 {
    last_int(&cx.r("time query gametime")).unwrap_or(-1)
}

fn flat(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// Hunger 255 burns saturation to 0, then food; stop at 12 or less, so an apple's +4 shows. Needs
/// a non-Peaceful server (Peaceful never lowers food).
fn lower_food(cx: &mut Ctx) {
    let n = cx.name.clone();
    cx.r(&format!("effect give {n} minecraft:hunger 30 255 true"));
    cx.poll(15.0, |cx| food(cx) <= 12);
    cx.r(&format!("effect clear {n} minecraft:hunger"));
    sleep(Duration::from_millis(500));
}

fn difficulty(cx: &mut Ctx) -> String {
    cx.r("difficulty")
}

fn heal(cx: &mut Ctx) {
    let n = cx.name.clone();
    cx.r(&format!("effect give {n} minecraft:instant_health 1 10 true"));
    sleep(Duration::from_millis(300));
}

/// Sends an op and waits for its `Started` (the command was just issued).
fn start(cx: &mut Ctx, op: ItemOp) -> Result<(), String> {
    let seq = cx.send(Op::Items(op));
    match cx.recv(seq, Duration::from_secs(30)) {
        Ok(r) if matches!(r.info, Info::Started) => Ok(()),
        Ok(r) => Err(format!("bot reported {:?} before the op started", r.info)),
        Err(e) => Err(format!("bot never started the op: {e}")),
    }
}

/// The final report of the op that `start` began (the last one sent).
fn finish(cx: &mut Ctx) -> ItemsInfo {
    match cx.finish_op(cx.seq, Duration::from_secs(30)) {
        Ok(r) => match r.info {
            Info::Items(i) => i,
            i => panic!("expected an items report: {i:?}"),
        },
        Err(e) => panic!("bot did not answer: {e}"),
    }
}

fn items(cx: &mut Ctx, op: ItemOp) -> ItemsInfo {
    match cx.call(Op::Items(op)).info {
        Info::Items(i) => i,
        i => panic!("expected an items report: {i:?}"),
    }
}

fn scene(cx: &mut Ctx, hotbar: &[(u8, &str, u8)]) {
    cx.scene_in(AREA, hotbar, SPAWN);
    // Full food (Saturation 255 for one tick), then no effects: a golden apple leaves absorption
    // hearts that would soak up the damage tests.
    let n = cx.name.clone();
    cx.r(&format!("effect give {n} minecraft:saturation 1 255 true"));
    sleep(Duration::from_millis(200));
    cx.r(&format!("effect clear {n}"));
    heal(cx);
    items(cx, ItemOp::Face { yaw: 0.0, pitch: 0.0 });
}

fn using(i: &ItemsInfo) -> String {
    i.using.map_or("none".into(), |u| format!("{:?} for {} ticks", u.hand, u.ticks))
}

// ---- A5 eating ------------------------------------------------------------------------------

/// Golden apple: nutrition 4, saturation 9.6, can always be eaten; 32 ticks. Walking 20 ticks from
/// rest covers about 4.3 blocks; slowed to 20% about 0.9.
const APPLE_SATURATION: f64 = 9.6;
const APPLE_FOOD: i64 = 4;

// Spec: specs/B2-survival-upkeep.md §4.1 (eating takes 32 ticks, release cancels, movement slowed)
pub fn a5(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();

    scene(cx, &[(0, "golden_apple", 3)]);
    lower_food(cx);
    out.push(cx.run("A5", "eat_takes_32_ticks", |cx| {
        let apples0 = cx.count("golden_apple");
        let (f0, s0) = (food(cx), saturation(cx));
        let diff = difficulty(cx);
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        let g0 = game_time(cx);
        let mut g1 = None;
        cx.poll(4.0, |cx| {
            let rose = food(cx) > f0;
            if rose {
                g1 = Some(game_time(cx));
            }
            rose
        });
        let during = finish(cx);
        let s1 = saturation(cx);
        let f1 = food(cx);
        sleep(Duration::from_millis(300));
        let after = items(cx, ItemOp::Status);
        let apples1 = cx.count("golden_apple");
        let gap = g1.map(|g| g - g0);
        // FoodData.eat: food = min(food + nutrition, 20), saturation = min(saturation + value, food).
        let want_f = (f0 + APPLE_FOOD).min(20);
        let want_s = (want_f as f64).min(s0 + APPLE_SATURATION);
        (
            f0 <= 12
                && f1 == want_f
                && (s1 - want_s).abs() < 0.05
                && apples1 + 1 == apples0
                && gap.is_some_and(|g| (30..=35).contains(&g))
                && during.using.is_some_and(|u| u.hand == Hand::Main)
                && after.using.is_none(),
            format!(
                "{diff}; golden apple (nutrition 4, saturation 9.6): server food {f0} -> {f1} (want {want_f}; food lowered to 12 or less first), saturation {s0} -> {s1} (want {want_s}), apples {apples0} -> {apples1}; food rose {gap:?} server ticks after the use started (vanilla 32, window 30-35: two RCON clock reads); bot view 2 ticks in: {}, after: {}",
                using(&during),
                using(&after)
            ),
        )
    }));

    scene(cx, &[(0, "golden_apple", 3)]);
    lower_food(cx);
    out.push(cx.run("A5", "release_early_eats_nothing", |cx| {
        let apples0 = cx.count("golden_apple");
        let (f0, s0) = (food(cx), saturation(cx));
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        finish(cx);
        sleep(Duration::from_millis(400));
        let released = items(cx, ItemOp::Release);
        sleep(Duration::from_millis(2000));
        let (f1, s1) = (food(cx), saturation(cx));
        let apples1 = cx.count("golden_apple");
        let view = items(cx, ItemOp::Status);
        (
            f0 < 20 && f1 == f0 && (s1 - s0).abs() < 0.01 && apples1 == apples0 && released.using.is_none() && view.using.is_none(),
            format!(
                "released about 8 ticks into the 32: server food {f0} -> {f1}, saturation {s0} -> {s1} (want both unchanged; an apple would add 4 and 9.6), apples {apples0} -> {apples1} (want unchanged); bot view after release: {}, 2 s later: {}",
                using(&released),
                using(&view)
            ),
        )
    }));

    scene(cx, &[(0, "golden_apple", 3)]);
    out.push(cx.run("A5", "movement_slowed_while_eating", |cx| {
        let n = cx.name.clone();
        let walk = |cx: &mut Ctx| {
            let p0 = bot_pos(cx);
            items(cx, ItemOp::Walk(20));
            sleep(Duration::from_millis(400));
            (p0, bot_pos(cx))
        };
        let (a0, a1) = walk(cx);
        let free = flat(a0, a1);
        cx.r(&format!("tp {n} {} {} {} 0 0", SPAWN.0, SPAWN.1, SPAWN.2));
        sleep(Duration::from_millis(700));
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        let started = finish(cx);
        let (b0, b1) = walk(cx);
        let slowed = flat(b0, b1);
        let view = items(cx, ItemOp::Status);
        items(cx, ItemOp::Release);
        (
            free > 2.0 && slowed < 0.35 * free && started.using.is_some(),
            format!(
                "server positions after 20 ticks of walking: {free:.2} blocks free, {slowed:.2} blocks while eating (want about 0.2x, so under 0.35x = {:.2}); bot view while eating: {} (still eating after the walk: {})",
                0.35 * free,
                using(&started),
                view.using.is_some()
            ),
        )
    }));

    // Bread needs hunger; at full food the server refuses, and a vanilla client does not slow down.
    scene(cx, &[(0, "bread", 3)]);
    out.push(cx.run("A5", "full_food_bread_is_refused", |cx| {
        let bread0 = cx.count("bread");
        let (f0, s0) = (food(cx), saturation(cx));
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        let started = finish(cx);
        sleep(Duration::from_millis(2000));
        let (bread1, f1, s1) = (cx.count("bread"), food(cx), saturation(cx));
        (
            f0 == 20 && bread1 == bread0 && f1 == 20 && (s1 - s0).abs() < 0.01 && started.using.is_none(),
            format!("bread at full food: food {f0} -> {f1} (want 20 -> 20), bread {bread0} -> {bread1}, saturation {s0} -> {s1} (want both unchanged); bot view 2 ticks in: {} (want none: not slowed)", using(&started)),
        )
    }));

    cx.clean_in(AREA);
    out
}

// ---- A6 shield ------------------------------------------------------------------------------

// Spec: specs/B5-combat.md §4.5 (shield: 5-tick raise, blocks from the front only, movement slowed)
pub fn a6(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    let n = cx.name.clone();
    // (name, wait after the raise in ms, where the hit comes from (z offset), damage expected, why)
    let cases: [(&str, u64, f64, f64, &str); 3] = [
        ("blocks_from_the_front", 700, 4.0, 0.0, "raised for 14 ticks (the 5-tick delay is over), hit from the front"),
        ("hurts_from_behind", 700, -4.0, 6.0, "raised for 14 ticks, hit from behind"),
        ("raise_delay_lets_a_hit_through", 0, 4.0, 6.0, "hit from the front in the 5-tick raise delay (RCON a few ms after the use packet, so inside it)"),
    ];
    // Natural regeneration off for the damage rows (restored below), so the health drop is exact.
    let regen = cx.r("gamerule natural_health_regeneration");
    let regen_was = if regen.contains("false") { "false" } else { "true" };
    cx.r("gamerule natural_health_regeneration false");
    for (name, wait, dz, want_dmg, why) in cases {
        scene(cx, &[(0, "shield", 1)]);
        out.push(cx.run("A6", name, |cx| {
            let h0 = health_of(cx, &n);
            let absorb = floats(&cx.r(&format!("data get entity {n} AbsorptionAmount"))).last().copied();
            if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
                return (false, e);
            }
            sleep(Duration::from_millis(wait));
            let reply = cx.r(&format!("damage {n} 6 minecraft:player_attack at {} {} {}", SPAWN.0, SPAWN.1, SPAWN.2 + dz));
            let h1 = health_of(cx, &n);
            let raised = finish(cx);
            items(cx, ItemOp::Release);
            let dmg = h0 - h1;
            let applied = reply.contains("Applied");
            (
                if want_dmg == 0.0 { !applied && dmg <= 0.01 } else { applied && (dmg - want_dmg).abs() < 0.01 },
                format!("{why}: server health {h0} (absorption {absorb:?}) -> {h1}, damage {dmg} (want {want_dmg}; natural regeneration off); the server's reply to the damage command: {reply:?} (\"Applied\" = hurt, \"invulnerable\" = fully blocked); bot view: {}", using(&raised)),
            )
        }));
    }
    // (row, item, delay in ticks the item says, waits after the server confirms the raise in ms)
    let sweeps: [(&str, &str, u32, &[u64]); 2] = [
        ("delay_belief_default_shield", "shield", 5, &[0, 0, 50, 100, 200, 400]),
        ("delay_belief_shield_with_1s_delay", SLOW_SHIELD, 20, &[600, 850, 950, 1050, 1150, 1400]),
    ];
    for (name, item, want, waits) in sweeps {
        scene(cx, &[(0, item, 1)]);
        out.push(cx.run("A6", name, |cx| shield_sweep(cx, want, waits)));
    }
    cx.r(&format!("gamerule natural_health_regeneration {regen_was}"));

    scene(cx, &[(0, "shield", 1)]);
    out.push(cx.run("A6", "movement_slowed_while_raised", |cx| {
        let walk = |cx: &mut Ctx| {
            let p0 = bot_pos(cx);
            items(cx, ItemOp::Walk(20));
            sleep(Duration::from_millis(400));
            (p0, bot_pos(cx))
        };
        let (a0, a1) = walk(cx);
        let free = flat(a0, a1);
        cx.r(&format!("tp {n} {} {} {} 0 0", SPAWN.0, SPAWN.1, SPAWN.2));
        sleep(Duration::from_millis(700));
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        let raised = finish(cx);
        let (b0, b1) = walk(cx);
        let slowed = flat(b0, b1);
        items(cx, ItemOp::Release);
        (
            free > 2.0 && slowed < 0.35 * free && raised.using.is_some(),
            format!(
                "server positions after 20 ticks of walking: {free:.2} blocks free, {slowed:.2} blocks with the shield up (want under 0.35x = {:.2}); bot view: {}",
                0.35 * free,
                using(&raised)
            ),
        )
    }));

    cx.clean_in(AREA);
    out
}

/// A shield whose `blocks_attacks` says 1.0 s (20 ticks); the rest as vanilla's shield.
const SLOW_SHIELD: &str = "shield[minecraft:blocks_attacks={block_delay_seconds:1.0f,\
block_sound:\"minecraft:item.shield.block\",disabled_sound:\"minecraft:item.shield.break\",\
bypassed_by:\"#minecraft:bypasses_shield\",item_damage:{threshold:3.0f,base:1.0f,factor:1.0f}}]";

// Spec: specs/F5-version-layer.md §4.6 (the shield delay is a belief: prior from the item, refined by hits)
/// Raise, wait `waits[i]` ms after the server confirms, hit from the front, lower. The server's
/// reply says whether each hit was blocked; the bot's own reading must agree. The server's delay
/// is then bracketed by the use ticks of its unblocked and blocked hits, and must hold the item's
/// value `want`; the bot's belief must start from `want` and end on it.
fn shield_sweep(cx: &mut Ctx, want: u32, waits: &[u64]) -> (bool, String) {
    let n = cx.name.clone();
    let mut ok = true;
    let mut lines = Vec::new();
    let (mut lo, mut hi) = (0u32, u32::MAX);
    let mut hurt = 0;
    let mut prior_seen = None;
    // The belief lives across rows: count this sweep's observations only.
    let observed0 = items(cx, ItemOp::Status).shield.map_or(0, |b| b.observed());
    for &wait in waits {
        // Each hit that gets through knocks the bot back; start every hit from the spawn point.
        cx.r(&format!("tp {n} {} {} {} 0 0", SPAWN.0, SPAWN.1, SPAWN.2));
        sleep(Duration::from_millis(400));
        if let Err(e) = start(cx, ItemOp::Use(Hand::Main)) {
            return (false, e);
        }
        let mut st = finish(cx);
        for _ in 0..20 {
            if st.using.is_some_and(|u| u.server_ticks.is_some()) {
                break;
            }
            st = items(cx, ItemOp::Status);
        }
        prior_seen = prior_seen.or(st.shield.map(|b| b.prior()));
        let t0 = st.tick;
        sleep(Duration::from_millis(wait));
        let reply = cx.r(&format!("damage {n} 6 minecraft:player_attack at {} {} {}", SPAWN.0, SPAWN.1, SPAWN.2 + 4.0));
        let server_blocked = !reply.contains("Applied");
        sleep(Duration::from_millis(200));
        let after = items(cx, ItemOp::Status);
        items(cx, ItemOp::Release);
        let seen: Vec<_> = after.shield_hits.iter().filter(|h| h.tick > t0).collect();
        match seen.as_slice() {
            [h] => {
                if h.blocked != server_blocked {
                    ok = false;
                }
                if server_blocked {
                    hi = hi.min(h.use_ticks + 1);
                } else {
                    lo = lo.max(h.use_ticks);
                    hurt += 1;
                }
                lines.push(format!(
                    "wait {wait} ms: server {}, bot {} at use tick {}",
                    if server_blocked { "blocked" } else { "hurt" },
                    if h.blocked { "blocked" } else { "hurt" },
                    h.use_ticks
                ));
            }
            other => {
                ok = false;
                lines.push(format!("wait {wait} ms: server reply {reply:?}; bot saw {} hits: {other:?}", other.len()));
            }
        }
        // 20 ticks of hurt immunity follow any hit, blocked or not; then full health again.
        sleep(Duration::from_millis(1100));
        heal(cx);
    }
    let end = items(cx, ItemOp::Status);
    let b = end.shield;
    let belief_ok = b.is_some_and(|b| {
        b.prior() == want && b.ticks() == want && b.observed().saturating_sub(observed0) == waits.len() as u32
    });
    // Both sides of the delay seen, or the bracket proves little.
    let both = hurt > 0 && hi != u32::MAX;
    let bracket_ok = lo <= want && want <= hi && lo <= hi;
    (
        ok && both && bracket_ok && belief_ok && prior_seen == Some(want) && end.held_delay == Some(want),
        format!(
            "item says {want} ticks (bot reads {:?}); server delay from its replies in {lo}..={} (must hold {want}); bot belief {:?} (prior at the first raise {prior_seen:?}); {}",
            end.held_delay,
            if hi == u32::MAX { "?".into() } else { hi.to_string() },
            b.map(|b| (b.ticks(), b.bounds(), b.observed() - observed0, b.ignored())),
            lines.join("; ")
        ),
    )
}

// ---- A7 attack ------------------------------------------------------------------------------

/// Player.attack: damage = base * (0.2 + s^2 * 0.8) with s = (ticks since the last attack + 0.5) /
/// (20 / attack speed), at most 1; a critical hit (falling, s > 0.9) multiplies by 1.5.
fn expected_damage(base: f64, attack_speed: f64, ticks_since: f64, crit: bool) -> f64 {
    let s = ((ticks_since + 0.5) / (20.0 / attack_speed)).min(1.0);
    base * (0.2 + s * s * 0.8) * if crit && s > 0.9 { 1.5 } else { 1.0 }
}

const GOLEM: &str = "@e[type=minecraft:iron_golem,tag=itm_target,x=530,y=199,z=342,dx=21,dy=10,dz=21,limit=1]";

// Spec: specs/B5-combat.md §4.1 (attack cooldown and critical hits)
pub fn a7(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    // Iron sword: 1 base + 5 = 6 damage, attack speed 4 - 2.4 = 1.6. An iron golem has 100 health,
    // no armor and full knockback resistance; NoAI keeps it where it is.
    const SWORD: f64 = 6.0;
    const SPEED: f64 = 1.6;
    scene(cx, &[(0, "iron_sword", 1)]);
    cx.r(&format!("summon minecraft:iron_golem {} {} {} {{NoAI:1b,Silent:1b,PersistenceRequired:1b,Tags:[\"itm_target\"]}}", SPAWN.0, SPAWN.1, SPAWN.2 + 2.6));
    sleep(Duration::from_millis(1500));
    let hit = |cx: &mut Ctx, how: AttackHow| {
        let h0 = health_of(cx, GOLEM);
        let info = items(cx, ItemOp::Attack { kind: "minecraft:iron_golem", how });
        sleep(Duration::from_millis(400));
        (h0, health_of(cx, GOLEM), info)
    };

    out.push(cx.run("A7", "full_cooldown_hit", |cx| {
        let want = expected_damage(SWORD, SPEED, 99.0, false);
        let (h0, h1, info) = hit(cx, AttackHow::Full);
        let dmg = h0 - h1;
        (
            (dmg - want).abs() < 0.01,
            format!("expected: {SWORD} * (0.2 + 1.0^2 * 0.8) = {want}; measured on the server: golem health {h0} -> {h1} = {dmg}; bot at the attack: {}", info.note),
        )
    }));

    sleep(Duration::from_millis(1500));
    out.push(cx.run("A7", "critical_hit", |cx| {
        let want = expected_damage(SWORD, SPEED, 99.0, true);
        let (h0, h1, info) = hit(cx, AttackHow::Crit);
        let dmg = h0 - h1;
        (
            (dmg - want).abs() < 0.01,
            format!("expected: {SWORD} * (0.2 + 1.0^2 * 0.8) * 1.5 crit = {want}; measured on the server: golem health {h0} -> {h1} = {dmg}; bot at the attack: {}", info.note),
        )
    }));

    sleep(Duration::from_millis(1500));
    out.push(cx.run("A7", "partial_cooldown_hit", |cx| {
        // A full hit, then a second one when the bot's cooldown first reads at least 0.88: 11 ticks
        // later (s = 11.5 / 12.5 = 0.92). The server counts 10 to 12 ticks between the two packets.
        let h0 = health_of(cx, GOLEM);
        items(cx, ItemOp::Attack { kind: "minecraft:iron_golem", how: AttackHow::Full });
        let second = items(cx, ItemOp::Attack { kind: "minecraft:iron_golem", how: AttackHow::AtCooldown(0.88) });
        sleep(Duration::from_millis(400));
        let h2 = health_of(cx, GOLEM);
        let total = h0 - h2;
        let d = |t: f64| expected_damage(SWORD, SPEED, t, false);
        let second_dmg = total - SWORD;
        let matched: Vec<i32> = (10..=12).filter(|t| (second_dmg - d(*t as f64)).abs() < 0.02).collect();
        (
            !matched.is_empty(),
            format!(
                "expected: first hit {SWORD}; second hit about {} ticks later at s = (t + 0.5) / 12.5: t=10 {:.3}, t=11 {:.3}, t=12 {:.3} (from {SWORD} * (0.2 + s^2 * 0.8), full at 12); measured on the server: golem health {h0} -> {h2} = {total}, so the second hit did {second_dmg:.3}, matching t = {matched:?}; bot at the second attack: {}",
                11,
                d(10.0),
                d(11.0),
                d(12.0),
                second.note
            ),
        )
    }));

    cx.r(&format!("kill {GOLEM}"));
    cx.clean_in(AREA);
    out
}

// ---- A8 attackers ---------------------------------------------------------------------------

// Spec: specs/B5-combat.md §4.7, specs/K2-commander-api.md AD-K2.2 (who hit us; defend or retaliate only against the attacker)
pub fn a8(cx: &mut Ctx) -> Vec<Outcome> {
    let mut out = Vec::new();
    let n = cx.name.clone();
    let other = format!("{n}2");
    scene(cx, &[(0, "iron_sword", 1)]);
    let online = cx.poll(20.0, |cx| cx.r(&format!("execute if entity {other}")).contains("passed"));
    if online.is_none() {
        out.push(Outcome { id: "A8", name: "setup".into(), pass: false, evidence: format!("{other} never came online"), shown: false, secs: 0.0 });
        return out;
    }
    for c in [
        format!("gamemode survival {other}"),
        format!("tp {other} 540.5 200 345.5 0 0"),
        "summon minecraft:pig 543.5 200 349.5 {NoAI:1b,Silent:1b,PersistenceRequired:1b,Tags:[\"itm_target\"]}".into(),
    ] {
        cx.r(&c);
    }
    sleep(Duration::from_millis(2000));

    out.push(cx.run("A8", "damage_without_an_entity_names_nobody", |cx| {
        let before = items(cx, ItemOp::Status).attackers.len();
        let reply = cx.r(&format!("damage {n} 1 minecraft:generic"));
        sleep(Duration::from_millis(600));
        let after = items(cx, ItemOp::Status).attackers.len();
        (after == before, format!("RCON `damage generic` (no source entity), reply {reply:?}: bot's attackers {before} -> {after} (want unchanged)"))
    }));

    // (name, RCON command, what it should name)
    let cases = [
        ("melee_by_named_player", format!("damage {n} 2 minecraft:player_attack by {other}"), "melee: cause and direct entity are the player"),
        (
            "projectile_names_the_shooter",
            format!("damage {n} 2 minecraft:indirect_magic by @e[type=minecraft:pig,tag=itm_target,{BOX},limit=1] from {other}"),
            "thrown potion: the direct entity is a pig, the cause is the player",
        ),
    ];
    for (name, cmd, what) in cases {
        sleep(Duration::from_millis(800));
        out.push(cx.run("A8", name, |cx| {
            let h0 = health_of(cx, &n);
            if let Err(e) = start(cx, ItemOp::WatchAttackers) {
                return (false, e);
            }
            let reply = cx.r(&cmd);
            let seen = finish(cx);
            let h1 = health_of(cx, &n);
            let last = seen.attackers.last();
            let named = last.and_then(|(_, name)| name.clone());
            let lag = last.map(|(a, _)| seen.tick.saturating_sub(a.tick));
            (
                named.as_deref() == Some(other.as_str()) && seen.waited <= 2 && lag.is_some_and(|l| l <= 1) && h1 < h0,
                format!(
                    "{what}; RCON reply {reply:?}; server health {h0} -> {h1}; the bot listed {:?} after {} ticks (want {other} within 2), stamped {lag:?} ticks before it read the list (want at most 1)",
                    named, seen.waited
                ),
            )
        }));
    }

    cx.r(&format!("kill @e[type=minecraft:pig,tag=itm_target,{BOX}]"));
    cx.clean_in(AREA);
    out
}
