//! Per-bot test telemetry shared by the engage skill and the retreat reflex: JSON lines
//! (hand-written; the crate has no dependencies). Exactly one of the two acts each tick.

use null_bot_api::{BotView, EntitySnapshot};
use std::sync::{mpsc::Sender, Arc, Mutex};

struct Inner {
    bot: String,
    tx: Sender<String>,
    every: u64,
    last_hp: Option<f32>,
    last_blocking: bool,
    last_phase: &'static str,
    last_target: Option<String>,
    /// Why the previous actor stopped, for the phase event of the one that takes over.
    why: Option<&'static str>,
}

/// What one actor did this tick.
pub struct TickReport {
    /// "fight", "seek" or "recover".
    pub phase: &'static str,
    pub ended: Option<&'static str>,
    pub target: Option<EntitySnapshot>,
    pub attack: bool,
    pub blocking: bool,
    /// Phase-specific JSON fields, without braces.
    pub detail: String,
}

#[derive(Clone)]
pub struct Telemetry(Arc<Mutex<Inner>>);

impl Telemetry {
    /// A sample every `every` ticks plus events, sent to `tx`.
    pub fn new(bot: &str, tx: Sender<String>, every: u64) -> Self {
        Self(Arc::new(Mutex::new(Inner {
            bot: bot.into(), tx, every: every.max(1), last_hp: None, last_blocking: false,
            last_phase: "seek", last_target: None, why: None,
        })))
    }

    /// The reason the actor that just stopped gives, used by the next phase event.
    pub fn note(&self, why: &'static str) {
        self.0.lock().unwrap().why = Some(why);
    }

    // Spec: specs/plans/COMBAT_SCAFFOLD.md "Swarm duel test" (match telemetry for duel_analyze.py)
    /// Emits events (phase, target, hurt, shield, attack) and the periodic sample.
    pub fn report(&self, view: &dyn BotView, r: TickReport) {
        let mut t = self.0.lock().unwrap();
        let (now, me, hp) = (view.tick(), view.position(), view.health());
        let head = format!("{{\"bot\":\"{}\",\"tick\":{now}", t.bot);
        let tgt = |e: &Option<EntitySnapshot>| match e {
            Some(e) => format!(
                "\"target\":\"{}\",\"target_dist\":{:.2},\"target_hp\":{}",
                e.name.as_deref().unwrap_or("?"),
                e.position.distance(me),
                e.health.map_or("null".into(), |h| format!("{h:.1}"))
            ),
            None => "\"target\":null".into(),
        };
        let phase = r.phase;
        let mut out = Vec::new();
        if phase != t.last_phase {
            // A reason the reflex noted when it stopped beats the next actor's own.
            let why = t.why.take().or(r.ended);
            let why = why.map_or("null".into(), |w| format!("\"{w}\""));
            out.push(format!("{head},\"ev\":\"phase\",\"from\":\"{}\",\"to\":\"{phase}\",\"why\":{why},{}}}", t.last_phase, tgt(&r.target)));
            t.last_phase = phase;
        }
        t.why = None;
        let name = r.target.as_ref().and_then(|e| e.name.clone());
        if name.is_some() && name != t.last_target {
            out.push(format!("{head},\"ev\":\"target\",{}}}", tgt(&r.target)));
        }
        if phase != "fight" || name.is_some() {
            t.last_target = name;
        }
        if let Some(prev) = t.last_hp {
            if hp < prev {
                out.push(format!("{head},\"ev\":\"hurt\",\"dmg\":{:.1},\"hp\":{hp:.1},\"blocking\":{}}}", prev - hp, r.blocking));
            }
        }
        t.last_hp = Some(hp);
        if r.blocking != t.last_blocking {
            out.push(format!("{head},\"ev\":\"shield\",\"up\":{}}}", r.blocking));
            t.last_blocking = r.blocking;
        }
        if r.attack {
            let crit = !view.on_ground() && view.fall_distance() > 0.0;
            out.push(format!("{head},\"ev\":\"attack\",\"crit_try\":{crit},\"cooldown\":{:.2},{}}}", view.attack_cooldown(), tgt(&r.target)));
        }
        if now % t.every == 0 {
            out.push(format!(
                "{head},\"ev\":\"sample\",\"phase\":\"{phase}\",\"x\":{:.2},\"y\":{:.2},\"z\":{:.2},\"hp\":{hp:.1},\"cooldown\":{},\"blocking\":{},\"nav\":\"{:?}\",{},{}}}",
                me.x, me.y, me.z,
                if phase == "fight" { format!("{:.2}", view.attack_cooldown()) } else { "null".into() },
                r.blocking,
                view.nav_status(),
                tgt(&r.target),
                r.detail
            ));
        }
        for l in out {
            let _ = t.tx.send(l);
        }
    }
}
