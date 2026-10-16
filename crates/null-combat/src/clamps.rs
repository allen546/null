use crate::{DuelAction, DuelObs};
use null_mechanics::{Mechanic, mechanic};
use std::collections::VecDeque;

/// Counts of what the clamps (and the rule filters) changed. Per step, or summed in `totals`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClampReport {
    pub attacks_dropped_rate: u32,
    pub attacks_dropped_reach: u32,
    /// Attacks moved to a later tick because the shield had to come down first.
    pub attacks_held_for_shield: u32,
    pub looks_limited: u32,
    /// Set by the fight skill's cargo-proximity rule, not by `Clamps`.
    pub attacks_dropped_cargo: u32,
}

impl ClampReport {
    pub fn add(&mut self, o: &ClampReport) {
        self.attacks_dropped_rate += o.attacks_dropped_rate;
        self.attacks_dropped_reach += o.attacks_dropped_reach;
        self.attacks_held_for_shield += o.attacks_held_for_shield;
        self.looks_limited += o.looks_limited;
        self.attacks_dropped_cargo += o.attacks_dropped_cargo;
    }
}

pub fn wrap_deg(a: f32) -> f32 {
    let r = (a + 180.0).rem_euclid(360.0) - 180.0;
    if r == -180.0 { 180.0 } else { r }
}

/// Vanilla limits, applied after the controller. Values carry their mechanics confidence.
pub struct Clamps {
    pub max_clicks_per_second: Mechanic,
    pub max_turn_per_tick: Mechanic,
    pub reach: Mechanic,
    clicks: VecDeque<u64>,
    total: ClampReport,
}

fn lookup(key: &str, value: f64, unit: &'static str) -> Mechanic {
    // Fallback only if the table loses a key; keeps the crate usable, flagged as Claimed.
    mechanic(key).unwrap_or(Mechanic {
        value,
        unit,
        confidence: null_mechanics::Confidence::Claimed,
        source: "fallback default in null-combat",
    })
}

impl Clamps {
    pub fn from_mechanics() -> Self {
        Self::new(
            lookup("legit.max_clicks_per_second", 14.0, "clicks/s"),
            lookup("legit.max_turn_per_tick", 60.0, "degrees/tick"),
            lookup("combat.entity_interaction_range", 3.0, "blocks"),
        )
    }

    pub fn new(cps: Mechanic, turn: Mechanic, reach: Mechanic) -> Self {
        Self {
            max_clicks_per_second: cps,
            max_turn_per_tick: turn,
            reach,
            clicks: VecDeque::new(),
            total: ClampReport::default(),
        }
    }

    pub fn totals(&self) -> ClampReport {
        self.total
    }

    pub fn reset(&mut self) {
        self.clicks.clear();
        self.total = ClampReport::default();
    }

    /// Reach uses `obs.target.reach_distance` (eye to target centre minus 0.3), an approximation
    /// of eye-to-hit-box. Turn limits apply per axis; the yaw takes the short way round.
    pub fn apply(&mut self, tick: u64, obs: &DuelObs, mut a: DuelAction) -> (DuelAction, ClampReport) {
        let mut r = ClampReport::default();
        if a.attack {
            if obs.target.reach_distance > self.reach.value {
                a.attack = false;
                r.attacks_dropped_reach += 1;
            } else {
                while self.clicks.front().is_some_and(|&t| t + 20 <= tick) {
                    self.clicks.pop_front();
                }
                if self.clicks.len() as f64 >= self.max_clicks_per_second.value.floor() {
                    a.attack = false;
                    r.attacks_dropped_rate += 1;
                } else {
                    self.clicks.push_back(tick);
                }
            }
        }
        let max = self.max_turn_per_tick.value as f32;
        let dyaw = wrap_deg(a.yaw - obs.look.yaw);
        let dpitch = a.pitch.clamp(-90.0, 90.0) - obs.look.pitch;
        let (cy, cp) = (dyaw.clamp(-max, max), dpitch.clamp(-max, max));
        if cy != dyaw || cp != dpitch {
            r.looks_limited += 1;
        }
        a.yaw = wrap_deg(obs.look.yaw + cy);
        a.pitch = obs.look.pitch + cp;
        self.total.add(&r);
        (a, r)
    }
}
