//! `combat.flee_low_hp` reflex: at low health, run to teammates (or away from the threat).
//! Re-reads everything each tick; the only memory is hysteresis and the hopeless timer.

use crate::engage::{nearest_enemy, Goto};
use crate::fight::{drop_ahead, step_ahead, wall_ahead};
use crate::telemetry::{TickReport, Telemetry};
use crate::FightRules;
use null_bot_api::reflex::{Group, Param, Reflex, ReflexSpec};
use null_bot_api::{BotCommands, BotView, EntitySnapshot, NavStatus, Vec3};

/// A retreating bot runs to teammates within this range; close enough to one, it stays put.
const RALLY_RANGE: f64 = 64.0;
const RALLY_REACHED: f64 = 3.0;
/// Enemies within this range are threats to run from.
const THREAT_RANGE: f64 = 12.0;

pub struct Retreat {
    spec: ReflexSpec,
    rules: FightRules,
    retreat_hp: f32,
    resume_health: f32,
    stall_ticks: u64,
    caught_radius: f64,
    /// Fired last tick (hysteresis).
    firing: bool,
    /// Just started firing: the next act releases the shield and plans afresh.
    fresh: bool,
    /// Health only regenerates with food at 18+ and bots cannot eat yet: no rise for
    /// `stall_ticks` means waiting is pointless, so engage fights it out instead.
    hopeless: bool,
    best_hp: f32,
    rose_at: u64,
    nav: Goto,
    rally: Option<String>,
    /// Our own name: in a swarm's shared world a bot can also see a copy of itself.
    me: Option<String>,
    tel: Option<Telemetry>,
}

impl Retreat {
    // Spec: specs/CONVENTIONS.md §4.1 (registry entry: id, group, priority, default, params)
    pub fn new(rules: FightRules) -> Self {
        let (retreat_hp, resume_health, stall_ticks, caught_radius) = (6.0, 14.0, 200.0, 2.5);
        let p = |name, value, unit| Param { name, value, unit };
        Self {
            spec: ReflexSpec {
                id: "combat.flee_low_hp",
                group: Group::Combat,
                priority: 50,
                default_on: true,
                params: vec![
                    p("retreat_hp", retreat_hp, "half-hearts"),
                    p("resume_health", resume_health, "half-hearts"),
                    p("stall_ticks", stall_ticks, "ticks"),
                    p("caught_radius", caught_radius, "blocks"),
                ],
            },
            rules,
            retreat_hp: retreat_hp as f32,
            resume_health: resume_health as f32,
            stall_ticks: stall_ticks as u64,
            caught_radius,
            firing: false,
            fresh: false,
            hopeless: false,
            best_hp: 0.0,
            rose_at: 0,
            nav: Goto::default(),
            rally: None,
            me: None,
            tel: None,
        }
    }

    pub fn with_self_name(mut self, name: &str) -> Self {
        self.me = Some(name.into());
        self
    }

    pub fn with_telemetry(mut self, tel: Telemetry) -> Self {
        self.tel = Some(tel);
        self
    }

    // Spec: specs/B5-combat.md §4.5 (retreat to a rally point, not just away)
    /// The teammate to retreat to: near, with others around it, and not on the far side of the threat.
    fn rally_point<'a>(&self, view: &'a dyn BotView, threat: Option<Vec3>) -> Option<&'a EntitySnapshot> {
        let me = view.position();
        let allies: Vec<&EntitySnapshot> = view
            .entities()
            .iter()
            .filter(|e| e.is_player && e.name.as_ref().is_some_and(|n| self.rules.never_names.contains(n) && self.me.as_ref() != Some(n)))
            // Only healthy teammates: running to one that is retreating too adds no swords, and two
            // hurt bots park together until the hopeless timer ends it.
            .filter(|e| e.health.is_none_or(|h| h >= self.resume_health) && e.position.distance(me) <= RALLY_RANGE)
            .collect();
        let past_threat = |a: &EntitySnapshot| {
            threat.is_some_and(|t| {
                let (ax, az, tx, tz) = (a.position.x - me.x, a.position.z - me.z, t.x - me.x, t.z - me.z);
                let (la, lt) = ((ax * ax + az * az).sqrt(), (tx * tx + tz * tz).sqrt());
                lt < la && la > 0.0 && lt > 0.0 && (ax * tx + az * tz) / (la * lt) > 0.7
            })
        };
        let score = |a: &EntitySnapshot| {
            let friends = allies.iter().filter(|b| b.position.distance(a.position) <= 8.0).count() as f64;
            a.position.distance(me) - 4.0 * friends
        };
        allies.iter().copied().filter(|a| !past_threat(a)).min_by(|a, b| score(a).total_cmp(&score(b)))
    }

    fn threat<'a>(&self, view: &'a dyn BotView) -> Option<&'a EntitySnapshot> {
        // Enter at 12, stay until 14: at a hard edge the bot flips between retreat and seek.
        let range = if self.firing { THREAT_RANGE + 2.0 } else { THREAT_RANGE };
        nearest_enemy(&self.rules, view).filter(|e| e.position.distance(view.position()) <= range)
    }

    // Spec: specs/B5-combat.md §4.5, §4.9 (`combat.flee_low_hp`; "if off: bot fights on" covers caught and hopeless)
    /// Why the reflex is not firing this tick, or None if it fires.
    fn verdict(&mut self, view: &dyn BotView) -> Option<&'static str> {
        let (now, hp) = (view.tick(), view.health());
        // The clock runs whenever we are hurt, firing or not: two hurt bots at the threat edge
        // otherwise take turns stopping it and never commit (seed 11, 2026-10-16).
        if hp > self.resume_health || hp > self.best_hp {
            self.best_hp = hp;
            self.rose_at = now;
            self.hopeless = false;
        } else if now - self.rose_at >= self.stall_ticks {
            self.hopeless = true;
        }
        if !(hp < self.retreat_hp || self.firing && hp <= self.resume_health) {
            return Some("healed");
        }
        if self.hopeless {
            return Some("hopeless");
        }
        let me = view.position();
        let threat = self.threat(view);
        if threat.is_some_and(|e| e.position.distance(me) <= self.caught_radius) {
            return Some("caught");
        }
        // With a teammate to run to we always can; alone we need a threat to run from and somewhere safe to run.
        let rally = self.rally_point(view, threat.map(|t| t.position)).is_some();
        if threat.is_none() && !rally {
            return Some("no_threat");
        }
        if let Some(t) = threat {
            if !rally {
                let (dx, dz) = (me.x - t.position.x, me.z - t.position.z);
                if safe_heading(view, (-dx).atan2(dz).to_degrees() as f32).is_none() {
                    return Some("cornered");
                }
            }
        }
        None
    }
}

impl Reflex for Retreat {
    fn spec(&self) -> &ReflexSpec {
        &self.spec
    }

    // Spec: specs/B5-combat.md §4.9 (`combat.flee_low_hp` row); specs/CONVENTIONS.md §4.3 (trigger evaluated every tick)
    fn trigger(&mut self, view: &dyn BotView) -> bool {
        let why = self.verdict(view);
        let fire = why.is_none();
        if self.firing && !fire {
            self.nav = Goto::default();
            self.rally = None;
            if let Some(t) = &self.tel {
                t.note(why.unwrap_or("healed"));
            }
        }
        self.fresh |= fire && !self.firing;
        self.firing = fire;
        fire
    }

    // Spec: specs/B5-combat.md §4.9 (`combat.flee_low_hp`: disengage to the rally point); §4.5
    fn act(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) {
        let me = view.position();
        let threat = self.threat(view).map(|e| e.position);
        if self.fresh {
            self.fresh = false;
            // The fight may have left the shield up.
            cmds.release_use();
        }
        match self.rally_point(view, threat).map(|a| (a.position, a.name.clone())) {
            Some((pos, name)) => {
                // Run to teammates: a chaser then meets several swords. Until the pathfinder has
                // a path, keep running away from the threat.
                self.rally = name;
                if pos.distance(me) <= RALLY_REACHED {
                    if self.nav.goal.is_some() {
                        cmds.stop();
                        self.nav = Goto::default();
                    }
                } else {
                    self.nav.update(view, cmds, pos.block_pos());
                    if let (Some(t), false) = (threat, view.nav_status() == NavStatus::Following) {
                        if let Some(yaw) = safe_heading(view, away_yaw(me, t)) {
                            run(view, cmds, yaw);
                        }
                    }
                }
            }
            None => {
                self.rally = None;
                // No rally point: stop any path still running (ours or engage's), or it keeps
                // driving the bot and fights the run below.
                if self.nav.goal.is_some() || view.nav_status() != NavStatus::Idle {
                    cmds.stop();
                    self.nav = Goto::default();
                }
                // Nobody to run to: keep running while anyone is close; standing still gets us killed.
                match threat.and_then(|t| safe_heading(view, away_yaw(me, t))) {
                    Some(yaw) => run(view, cmds, yaw),
                    None => cmds.stop(),
                }
            }
        }
        if let Some(t) = &self.tel {
            t.report(view, TickReport {
                phase: "recover",
                ended: Some("retreat"),
                target: None,
                attack: false,
                blocking: false,
                detail: format!(
                    "\"best_hp\":{:.1},\"rose_at\":{},\"desperate\":{},\"rally\":{}",
                    self.best_hp, self.rose_at, self.hopeless,
                    self.rally.as_deref().map_or("null".into(), |n| format!("\"{n}\""))
                ),
            });
        }
    }
}

fn away_yaw(me: Vec3, threat: Vec3) -> f32 {
    let (dx, dz) = (me.x - threat.x, me.z - threat.z);
    (-dx).atan2(dz).to_degrees() as f32
}

/// Away from the threat if possible, else turned up to 135 degrees; never off a drop or into a wall.
pub(crate) fn safe_heading(view: &dyn BotView, away: f32) -> Option<f32> {
    [0.0, 45.0, -45.0, 90.0, -90.0, 135.0, -135.0]
        .iter()
        .map(|o| away + o)
        .find(|&y| !drop_ahead(view, 1, 0, y) && !wall_ahead(view, y))
}

pub(crate) fn run(view: &dyn BotView, cmds: &mut dyn BotCommands, yaw: f32) {
    cmds.look(yaw, 0.0);
    cmds.sprint(true);
    cmds.walk(1, 0);
    if view.on_ground() && step_ahead(view, 1, 0, yaw) {
        cmds.jump();
    }
}
