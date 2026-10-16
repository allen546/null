//! Engage skill: find a named enemy, close in, fight. Never finishes. Nothing latches: each tick
//! picks the target and the way to move again from what the bot sees (retreating is the
//! `combat.flee_low_hp` reflex's job).

use crate::fight::{drop_ahead, line_blocked, wall_ahead};
use crate::obs::{item_is_axe, item_is_sword};
use crate::retreat::run;
use crate::telemetry::{TickReport, Telemetry};
use crate::{FightRules, FightSkill, MELEE_BASIC_SPACE, MeleeBasic, TargetSelector};
use null_bot_api::{BlockPos, BotCommands, BotView, EntityId, EntitySnapshot, NavStatus, Skill, SkillStatus, Vec3};

/// Beyond this, or with something in the way, the bot never walks straight at an enemy.
const DIRECT_RANGE: f64 = 16.0;
/// Melee hands over to the pathfinder only after the line to the target is blocked this long,
/// so a trunk flickering in and out of the line does not restart the fight every few ticks.
const BLOCKED_TICKS: u32 = 10;
/// A path being followed that has not moved us half a block in this long is stuck.
const STUCK_TICKS: u64 = 40;
/// Azalea searches for 1 to 5 s (`PathfinderOpts` min/max timeout); a new goto restarts the
/// search, so nothing interrupts one until it has had its full time plus a margin.
const SEARCH_TICKS: u64 = 110;
/// Another enemy must be this much closer before the target changes.
const SWITCH_MARGIN: f64 = 3.0;

/// The goto in flight, so a moving goal is only re-planned for when it moved enough.
#[derive(Default)]
pub(crate) struct Goto {
    pub goal: Option<BlockPos>,
    issued: Option<u64>,
    /// Where we were when we last made progress, and when.
    anchor: Option<(Vec3, u64)>,
}

impl Goto {
    pub fn since(&self, now: u64) -> u64 {
        self.issued.map_or(u64::MAX, |i| now - i)
    }

    /// Re-aims often (the goal moves), and asks again when the pathfinder sits idle or has been
    /// calculating forever.
    pub fn update(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands, goal: BlockPos) {
        let now = view.tick();
        let moved = self.goal.is_none_or(|g| (g.x - goal.x).abs().max((g.z - goal.z).abs()) >= 3);
        let (since, nav) = (self.since(now), view.nav_status());
        // A pathfinder that holds a goal but gets us nowhere (stuck on a node, or never finding a
        // path) is force-stopped and asked again.
        let me = view.position();
        let anchor = *self.anchor.get_or_insert((me, now));
        if me.distance(anchor.0) >= 0.5 || nav != NavStatus::Following {
            self.anchor = Some((me, now));
        } else if now - anchor.1 >= STUCK_TICKS {
            cmds.stop();
            self.anchor = Some((me, now));
            self.issued = None;
        }
        let since = if self.issued.is_none() { u64::MAX } else { since };
        // Re-aim at a moving goal only while following a path, never mid-search.
        let reaim = moved && since >= 10 && nav == NavStatus::Following;
        if reaim || nav == NavStatus::Idle && since >= 4 || matches!(nav, NavStatus::Calculating | NavStatus::Stalled) && since >= SEARCH_TICKS || self.issued.is_none() {
            cmds.goto(goal, 2.0);
            self.goal = Some(goal);
            self.issued = Some(now);
        }
    }
}

// Spec: specs/B5-combat.md §4.6 (target selection: a filter, then a priority)
fn valid_enemies<'a>(rules: &'a FightRules, view: &'a dyn BotView) -> impl Iterator<Item = &'a EntitySnapshot> {
    view.entities().iter().filter(|e| rules.is_valid_target(view, e) && e.health.is_none_or(|h| h > 0.0))
}

pub(crate) fn nearest_enemy<'a>(rules: &FightRules, view: &'a dyn BotView) -> Option<&'a EntitySnapshot> {
    let me = view.position();
    view.entities().iter().filter(|e| rules.is_valid_target(view, e) && e.health.is_none_or(|h| h > 0.0)).min_by(|a, b| a.position.distance(me).total_cmp(&b.position.distance(me)))
}

pub struct Engage {
    rules: FightRules,
    center: (f64, f64),
    /// Within this distance of a clear line the fight skill takes over from goto.
    handoff_radius: f64,
    /// Melee goes back to goto if the target gets this far away.
    drop_radius: f64,
    seed: u64,
    target: Option<EntityId>,
    /// Kept across melee ticks so strafe and crit timing survive; new only on a target change.
    fight: Option<FightSkill>,
    /// Smoothing, not a latch: the inputs are re-read every tick.
    melee: bool,
    blocked_ticks: u32,
    nav: Goto,
    /// Closest approach to the target, since when, and the end of a straight-walk burst.
    best: f64,
    best_since: u64,
    direct_until: u64,
    direct_active: bool,
    last_tick: Option<u64>,
    /// Fights started, for logs and tests.
    pub fights: u32,
    tel: Option<Telemetry>,
}

impl Engage {
    pub fn new(rules: FightRules, center: (f64, f64), seed: u64) -> Self {
        Self {
            rules, center, handoff_radius: 8.0, drop_radius: 14.0, seed, target: None, fight: None,
            melee: false, blocked_ticks: 0, nav: Goto::default(), best: f64::MAX, best_since: 0,
            direct_until: 0, direct_active: false, last_tick: None, fights: 0, tel: None,
        }
    }

    pub fn with_telemetry(mut self, tel: Telemetry) -> Self {
        self.tel = Some(tel);
        self
    }

    fn new_fight(&mut self, id: EntityId) -> FightSkill {
        self.seed = self.seed.wrapping_add(1);
        let mut p = MELEE_BASIC_SPACE.defaults();
        p.set("retreat_hp", 0.0);
        let c = MeleeBasic::new(&p).expect("default params");
        // The fight skill fights the target chosen here; it was validated by the real rules.
        let mut rules = self.rules.clone();
        rules.allowed = TargetSelector::Entities(vec![id]);
        rules.attack_player_first_allowed = true;
        FightSkill::new(rules, Box::new(c), self.seed)
    }

    // Spec: specs/B5-combat.md §4.6 (priority: nearest, with a switch margin)
    /// Nearest valid enemy; the current one stays unless another is `SWITCH_MARGIN` closer.
    fn pick_target(&self, view: &dyn BotView) -> Option<EntityId> {
        let me = view.position();
        let near = nearest_enemy(&self.rules, view)?;
        let dist = |e: &EntitySnapshot| e.position.distance(me);
        let cur = self.target.and_then(|id| valid_enemies(&self.rules, view).find(|e| e.id == id));
        Some(match cur {
            Some(c) if dist(c) - dist(near) <= SWITCH_MARGIN => c.id,
            _ => near.id,
        })
    }

    /// Ground block at (x, z): solid with 2 free blocks above and a solid one in the 4 below,
    /// which rejects canopies.
    fn ground(view: &dyn BotView, x: i32, z: i32, around_y: i32) -> Option<BlockPos> {
        (around_y - 30..=around_y + 30).rev().find_map(|y| {
            let ok = view.is_solid(BlockPos::new(x, y, z))
                && !view.is_solid(BlockPos::new(x, y + 1, z))
                && !view.is_solid(BlockPos::new(x, y + 2, z))
                && (1..=4).all(|d| view.is_solid(BlockPos::new(x, y - d, z)));
            ok.then(|| BlockPos::new(x, y + 1, z))
        })
    }

    fn lose_direct(&mut self, cmds: &mut dyn BotCommands) {
        if self.direct_active {
            cmds.stop();
            self.direct_active = false;
        }
    }

    // Spec: specs/B5-combat.md §4.2 (one `engage`), AD-B5.1 (technique behind a duel controller); specs/CONVENTIONS.md §4.4
    /// One tick; returns whether melee ran and why a mode or target ended, if it did.
    fn tick(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> (bool, Option<&'static str>) {
        let now = view.tick();
        // A reflex acted in between (or this is the first tick): its goto and inputs are not ours.
        if self.last_tick.is_none_or(|t| now > t + 1) {
            self.nav = Goto::default();
            self.direct_active = false;
            self.melee = false;
            self.blocked_ticks = 0;
            if let Some(f) = self.fight.as_mut() {
                f.abort(cmds);
            }
        }
        self.last_tick = Some(now);
        // Hold still until the kit arrives, so both teams start armed.
        if !view.hotbar().iter().flatten().any(|i| item_is_sword(Some(&i.item)) || item_is_axe(Some(&i.item))) {
            return (false, None);
        }
        let mut ended = None;
        let picked = self.pick_target(view);
        if picked != self.target {
            ended = self.target.map(|old| match view.entity(old) {
                None => "target_lost",
                Some(e) if e.health.is_some_and(|h| h <= 0.0) => "target_dead",
                Some(_) if picked.is_some() => "target_switch",
                Some(_) => "target_lost",
            });
            self.target = picked;
            self.fight = None;
            self.best = f64::MAX;
        }
        let Some(id) = picked else {
            self.leave_melee(cmds);
            self.lose_direct(cmds);
            self.wander(view, cmds);
            return (false, ended);
        };
        let t = view.entity(id).expect("picked from view");
        let (me, d) = (view.position(), t.position.distance(view.position()));
        let blocked_now = d > 3.0 && line_blocked(view, me, t.position);
        self.blocked_ticks = if self.melee && blocked_now { self.blocked_ticks + 1 } else { 0 };
        let stay = d <= self.drop_radius && self.blocked_ticks < BLOCKED_TICKS;
        let enter = d <= self.handoff_radius && !line_blocked(view, me, t.position);
        let melee = if self.melee { stay } else { enter };
        if self.melee && !melee {
            self.leave_melee(cmds);
            ended = Some(if d > self.drop_radius { "target_far" } else { "blocked" });
        } else if !self.melee && melee {
            cmds.stop_nav();
            self.direct_active = false;
            self.fights += 1;
        }
        self.melee = melee;
        if melee {
            let mut fight = match self.fight.take() {
                Some(f) => f,
                None => self.new_fight(id),
            };
            let st = fight.step(view, cmds);
            // Keep the trace bounded; telemetry reports what matters.
            if fight.trace().len() > 200 {
                fight.take_trace();
            }
            if st == SkillStatus::Running {
                self.fight = Some(fight);
            } else {
                // Target dead or gone: next tick picks again.
                use crate::DoneReason::*;
                ended = Some(match fight.last_done {
                    Some(TargetDead) => "target_dead",
                    Some(TargetLost) => "target_lost",
                    Some(Retreat) => "retreat",
                    None => "no_target",
                });
                self.fight = None;
                self.melee = false;
                self.target = None;
            }
            return (true, ended);
        }
        self.path(view, cmds, t);
        (false, ended)
    }

    /// Melee hands over to the pathfinder: release the shield and inputs, plan afresh.
    fn leave_melee(&mut self, cmds: &mut dyn BotCommands) {
        if self.melee {
            if let Some(f) = self.fight.as_mut() {
                f.take_trace();
                f.abort(cmds);
            }
            // The pathfinder goes around what the controller would walk into.
            self.nav = Goto::default();
            self.melee = false;
            self.blocked_ticks = 0;
        }
    }

    // Spec: specs/B1-movement.md AD-B1.1 (Azalea pathfinder), §4.5 (getting unstuck)
    /// Path toward a target out of melee: goto, or a straight push for a target in a canopy.
    fn path(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands, t: &EntitySnapshot) {
        let now = view.tick();
        let (me, d) = (view.position(), t.position.distance(view.position()));
        // The pathfinder gives up on targets in canopies or behind trees; then push straight at them.
        if self.direct_active && now >= self.direct_until {
            self.lose_direct(cmds);
        }
        if self.direct_active {
            let (dx, dz) = (t.position.x - me.x, t.position.z - me.z);
            run(view, cmds, (-dx).atan2(dz).to_degrees() as f32);
            return;
        }
        if d < self.best - 1.0 || matches!(view.nav_status(), NavStatus::Calculating | NavStatus::Stalled) {
            // A search in progress is not a lack of progress.
            self.best = self.best.min(d);
            self.best_since = now;
        } else if now - self.best_since > 20 {
            self.best = d;
            self.best_since = now;
            // A straight push only gets past what stops the pathfinder near the target (canopies);
            // far away or behind a wall it walks into the wall, so plan again instead.
            if d <= DIRECT_RANGE && !line_blocked(view, me, t.position) {
                cmds.stop_nav();
                self.direct_active = true;
                self.direct_until = now + 50;
                return;
            }
            // Force: a path stuck on a node survives a plain re-goto.
            cmds.stop();
            self.nav = Goto::default();
        }
        self.nav.update(view, cmds, t.position.block_pos());
        // A search can take Azalea up to 5 s with many bots searching at once (20 v 20 on
        // 2026-10-16: chasers stood still): head for the target meanwhile, if a heading is safe.
        if matches!(view.nav_status(), NavStatus::Calculating | NavStatus::Stalled) {
            let to = (-(t.position.x - me.x)).atan2(t.position.z - me.z).to_degrees() as f32;
            let yaw = [0.0, 30.0, -30.0, 60.0, -60.0].iter().map(|o| to + o).find(|&y| !drop_ahead(view, 1, 0, y) && !wall_ahead(view, y));
            if let Some(yaw) = yaw {
                run(view, cmds, yaw);
            }
        }
    }

    // Spec: specs/plans/COMBAT_SCAFFOLD.md "Swarm duel test" (test driver: wander to the centre)
    /// No enemy known: head for the centre, wandering a little once there.
    fn wander(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) {
        let (now, me) = (view.tick(), view.position());
        let since = self.nav.since(now);
        if since >= 40 || (view.nav_status() == NavStatus::Idle && since >= 10) {
            let (cx, cz) = (self.center.0.floor() as i32, self.center.1.floor() as i32);
            let dist = ((me.x - self.center.0).powi(2) + (me.z - self.center.1).powi(2)).sqrt();
            let (tx, tz) = if dist < 4.0 { (cx + ((now / 40) % 9) as i32 - 4, cz + ((now / 60) % 9) as i32 - 4) } else { (cx, cz) };
            let g = Self::ground(view, tx, tz, me.y as i32).unwrap_or(BlockPos::new(tx, me.y as i32, tz));
            self.nav = Goto::default();
            self.nav.update(view, cmds, g);
        }
    }

    fn report(&mut self, view: &dyn BotView, melee: bool, ended: Option<&'static str>) {
        let Some(tel) = self.tel.clone() else { return };
        let me = view.position();
        let (target, attack, blocking, detail) = match (&self.fight, melee) {
            (Some(f), true) => {
                let a = f.last_action;
                let detail = format!(
                    "\"ctrl\":[{},{}],\"act\":[{},{}],\"jump\":{},\"drop\":{},\"drop_in\":{},\"step\":{},\"yaw\":{:.0},\"ground\":{}",
                    f.last_ctrl_walk.0, f.last_ctrl_walk.1,
                    a.map_or(0, |a| a.forward), a.map_or(0, |a| a.strafe), a.is_some_and(|a| a.jump),
                    f.last_drop_guard, f.last_drop_in, f.last_step_jump, a.map_or(0.0, |a| a.yaw), view.on_ground()
                );
                (f.target().and_then(|id| view.entity(id)).cloned(), f.last_attack_sent, f.blocking(), detail)
            }
            _ => {
                let valid = valid_enemies(&self.rules, view).count();
                let players = view.entities().iter().filter(|e| e.is_player).count();
                let e = nearest_enemy(&self.rules, view);
                let detail = format!(
                    "\"players\":{players},\"valid\":{valid},\"enemy\":{},\"enemy_dist\":{},\"direct\":{},\"goal\":{}",
                    e.and_then(|e| e.name.as_deref()).map_or("null".into(), |n| format!("\"{n}\"")),
                    e.map_or("null".into(), |e| format!("{:.2}", e.position.distance(me))),
                    self.direct_active,
                    self.nav.goal.map_or("null".into(), |g| format!("[{},{},{}]", g.x, g.y, g.z))
                );
                (None, false, false, detail)
            }
        };
        tel.report(view, TickReport { phase: if melee { "fight" } else { "seek" }, ended, target, attack, blocking, detail });
    }
}

impl Skill for Engage {
    fn id(&self) -> &str {
        "engage"
    }

    // Spec: specs/B5-combat.md §4.2; specs/CONVENTIONS.md §4.4
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        let (melee, ended) = self.tick(view, cmds);
        self.report(view, melee, ended);
        SkillStatus::Running
    }
}
