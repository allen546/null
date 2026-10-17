//! Bot-side ops for A5-A8 (item use, attack, attackers). `RemoteSkill::step` calls `step` for
//! `Op::Items`; everything goes through `BotView` / `BotCommands` only.

use null_bot_api::{Attacker, BotCommands, BotView, Hand, UsingItem, Vec3};

use crate::skill::{Info, look_at};

/// Two ticks between a look and the click that needs it (the packet order puts the click first).
const AIM_TICKS: u64 = 3;

#[derive(Clone, Copy, Debug)]
pub enum AttackHow {
    /// On the ground at full cooldown.
    Full,
    /// Jump, then hit while falling at full cooldown.
    Crit,
    /// On the ground as soon as the cooldown reads at least this much.
    AtCooldown(f32),
}

#[derive(Clone, Debug)]
pub enum ItemOp {
    /// Start using `hand` (reports `Started` when the command is issued).
    Use(Hand),
    Release,
    /// Walk forward this many ticks, then stop.
    Walk(u32),
    Face { yaw: f32, pitch: f32 },
    Status,
    /// Says `Started`, then waits for a new entry in `recent_attackers`.
    WatchAttackers,
    /// Attack the nearest entity of this kind, aiming at its middle.
    Attack { kind: &'static str, how: AttackHow },
}

#[derive(Clone, Debug, Default)]
pub struct ItemsInfo {
    pub tick: u64,
    pub using: Option<UsingItem>,
    /// The attackers with the name the bot's entity list gives them.
    pub attackers: Vec<(Attacker, Option<String>)>,
    /// Ticks the op waited (WatchAttackers).
    pub waited: u64,
    /// Attack facts at the tick the command was issued.
    pub note: String,
}

/// Per-op scratch, reset when an op starts.
#[derive(Default)]
pub struct ItemState {
    attack_tick: Option<u64>,
    note: String,
    n0: usize,
}

fn info(v: &dyn BotView, st: &ItemState, waited: u64) -> Info {
    let attackers = v
        .recent_attackers()
        .iter()
        .map(|a| (*a, v.entity(a.entity).and_then(|e| e.name.clone())))
        .collect();
    Info::Items(ItemsInfo {
        tick: v.tick(),
        using: v.using_item(),
        attackers,
        waited,
        note: st.note.clone(),
    })
}

// Spec: specs/plans/ABILITY_CHECK.md (A5-A8 ops: use/release, walk, attack with cooldown and crit, attackers)
pub fn step(
    op: &ItemOp,
    age: u64,
    st: &mut ItemState,
    v: &dyn BotView,
    c: &mut dyn BotCommands,
    say: &dyn Fn(Info),
) -> Option<Info> {
    if age == 0 {
        *st = ItemState { n0: v.recent_attackers().len(), ..Default::default() };
    }
    match op {
        ItemOp::Use(hand) => {
            if age == 0 {
                c.use_item(*hand);
                say(Info::Started);
            }
            (age >= 2).then(|| info(v, st, age))
        }
        ItemOp::Release => {
            if age == 0 {
                c.release_use();
                say(Info::Started);
            }
            (age >= 2).then(|| info(v, st, age))
        }
        ItemOp::Walk(n) => {
            let n = *n as u64;
            if age == 0 {
                c.walk(1, 0);
            } else if age == n {
                c.walk(0, 0);
            }
            (age >= n + 2).then(|| info(v, st, age))
        }
        ItemOp::Face { yaw, pitch } => {
            if age == 0 {
                c.look(*yaw, *pitch);
            }
            (age >= AIM_TICKS).then(|| info(v, st, age))
        }
        ItemOp::Status => Some(info(v, st, age)),
        ItemOp::WatchAttackers => {
            if age == 0 {
                say(Info::Started);
            }
            (v.recent_attackers().len() > st.n0 || age > 100).then(|| info(v, st, age))
        }
        ItemOp::Attack { kind, how } => {
            let Some(t) = v.entities().iter().filter(|e| e.kind == *kind).min_by(|a, b| {
                let d = |e: &null_bot_api::EntitySnapshot| e.position.distance(v.position());
                d(a).total_cmp(&d(b))
            }) else {
                st.note = format!("no {kind} in view");
                return Some(info(v, st, age));
            };
            // The middle of a golem-sized target; the aim is set once, a jump does not re-aim.
            if age == 0 {
                look_at(v, c, Vec3::new(t.position.x, t.position.y + 1.4, t.position.z));
            }
            let cooldown = v.attack_cooldown();
            let ready = age >= AIM_TICKS
                && st.attack_tick.is_none()
                && match how {
                    AttackHow::Full => cooldown >= 1.0 && v.on_ground(),
                    AttackHow::AtCooldown(f) => cooldown >= *f && v.on_ground(),
                    AttackHow::Crit => !v.on_ground() && v.fall_distance() > 0.3 && v.velocity().y < 0.0 && cooldown >= 1.0,
                };
            if matches!(how, AttackHow::Crit) && age == AIM_TICKS && v.attack_cooldown() >= 1.0 {
                c.jump();
            }
            if ready {
                c.attack(t.id);
                st.attack_tick = Some(v.tick());
                st.note = format!(
                    "cooldown {cooldown:.3}, on_ground {}, fall_distance {:.2}, velocity.y {:.2}",
                    v.on_ground(),
                    v.fall_distance(),
                    v.velocity().y
                );
            }
            match st.attack_tick {
                Some(t0) if v.tick() >= t0 + 2 => Some(info(v, st, age)),
                None if age > 80 => {
                    st.note = format!("never ready: cooldown {cooldown:.3}, on_ground {}", v.on_ground());
                    Some(info(v, st, age))
                }
                _ => None,
            }
        }
    }
}
