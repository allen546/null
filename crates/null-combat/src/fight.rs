use crate::obs::item_is_axe;
use crate::{
    ActionApplier, ClampReport, Clamps, DoneReason, DuelAction, DuelController, DuelStep, TraceRecord,
    observe,
};
use null_bot_api::{BlockPos, Vec3, BotCommands, BotView, EntityId, EntitySnapshot, Skill, SkillStatus};

#[derive(Clone, Debug, PartialEq)]
pub enum TargetSelector {
    Entities(Vec<EntityId>),
    /// Namespaced kinds, e.g. "minecraft:zombie".
    Kinds(Vec<String>),
    /// Players by username. Naming them is the person's permission to attack first (AD-K2.2).
    PlayerNames(Vec<String>),
}

#[derive(Clone, Debug)]
pub struct FightRules {
    pub allowed: TargetSelector,
    pub never_cargo: Vec<EntityId>,
    pub never_allies: Vec<EntityId>,
    /// Player names that are never targets, even if also selected.
    pub never_names: Vec<String>,
    /// AD-K2.2: off by default; a person may enable it once.
    pub attack_player_first_allowed: bool,
    /// A player counts as an attacker for this many ticks after its last hit on us.
    pub recent_attacker_ticks: u64,
    /// Within this distance of cargo (bot or target side) only axe attacks go through.
    pub cargo_radius: f64,
}

impl FightRules {
    pub fn new(allowed: TargetSelector) -> Self {
        Self {
            allowed,
            never_cargo: Vec::new(),
            never_allies: Vec::new(),
            never_names: Vec::new(),
            attack_player_first_allowed: false,
            recent_attacker_ticks: 200,
            cargo_radius: 8.0,
        }
    }

    pub fn is_valid_target(&self, view: &dyn BotView, e: &EntitySnapshot) -> bool {
        if self.never_cargo.contains(&e.id) || self.never_allies.contains(&e.id) {
            return false;
        }
        if e.health.is_some_and(|h| h <= 0.0) {
            return false;
        }
        if e.name.as_ref().is_some_and(|n| self.never_names.contains(n)) {
            return false;
        }
        let selected = match &self.allowed {
            TargetSelector::Entities(ids) => ids.contains(&e.id),
            TargetSelector::Kinds(kinds) => kinds.iter().any(|k| *k == e.kind),
            TargetSelector::PlayerNames(names) => {
                // Named players only; a nameless snapshot never matches.
                return e.is_player && e.name.as_ref().is_some_and(|n| names.contains(n));
            }
        };
        if !selected {
            return false;
        }
        if e.is_player && !self.attack_player_first_allowed {
            let now = view.tick();
            return view
                .recent_attackers()
                .iter()
                .any(|a| a.entity == e.id && now.saturating_sub(a.tick) <= self.recent_attacker_ticks);
        }
        true
    }
}

pub struct FightSkill {
    rules: FightRules,
    controller: Box<dyn DuelController + Send + Sync>,
    applier: ActionApplier,
    clamps: Clamps,
    target: Option<EntityId>,
    last_target_health: Option<f32>,
    trace: Vec<TraceRecord>,
    /// Why the last step ended the fight, if it did.
    pub last_done: Option<DoneReason>,
    /// What the last step did, for telemetry: the clamped action and whether an attack went out.
    pub last_action: Option<DuelAction>,
    pub last_attack_sent: bool,
    /// The controller's walk (forward, strafe) before the guards, and which guards fired, for telemetry.
    pub last_ctrl_walk: (i8, i8),
    pub last_drop_guard: bool,
    pub last_step_jump: bool,
    /// Walked off an edge on purpose toward a target below (see `drop_worth`).
    pub last_drop_in: bool,
}

impl FightSkill {
    pub fn new(rules: FightRules, mut controller: Box<dyn DuelController + Send + Sync>, seed: u64) -> Self {
        controller.reset(seed);
        Self {
            rules,
            controller,
            applier: ActionApplier::default(),
            clamps: Clamps::from_mechanics(),
            target: None,
            last_target_health: None,
            trace: Vec::new(),
            last_done: None,
            last_action: None,
            last_attack_sent: false,
            last_ctrl_walk: (0, 0),
            last_drop_guard: false,
            last_step_jump: false,
            last_drop_in: false,
        }
    }

    pub fn trace(&self) -> &[TraceRecord] {
        &self.trace
    }
    pub fn take_trace(&mut self) -> Vec<TraceRecord> {
        std::mem::take(&mut self.trace)
    }
    pub fn clamp_totals(&self) -> ClampReport {
        self.clamps.totals()
    }
    pub fn blocking(&self) -> bool {
        self.applier.blocking()
    }

    pub fn target(&self) -> Option<EntityId> {
        self.target
    }

    fn pick_target(&self, view: &dyn BotView) -> Option<EntityId> {
        let me = view.position();
        view.entities()
            .iter()
            .filter(|e| self.rules.is_valid_target(view, e))
            .min_by(|a, b| a.position.distance(me).total_cmp(&b.position.distance(me)))
            .map(|e| e.id)
    }

    fn near_cargo(&self, view: &dyn BotView, target: &EntitySnapshot) -> bool {
        let me = view.position();
        view.entities().iter().filter(|e| self.rules.never_cargo.contains(&e.id)).any(|c| {
            c.position.distance(me) <= self.rules.cargo_radius
                || c.position.distance(target.position) <= self.rules.cargo_radius
        })
    }

    /// Ends the fight from outside (the caller dropped it): lowers the shield and stops input.
    pub fn abort(&mut self, cmds: &mut dyn BotCommands) {
        self.applier.release_all(cmds);
    }

    fn finish(&mut self, cmds: &mut dyn BotCommands, why: DoneReason) -> SkillStatus {
        self.applier.release_all(cmds);
        self.last_done = Some(why);
        match why {
            DoneReason::TargetDead => SkillStatus::Done,
            DoneReason::TargetLost => SkillStatus::Failed("target lost".into()),
            DoneReason::Retreat => SkillStatus::Failed("retreat".into()),
        }
    }
}

impl Skill for FightSkill {
    fn id(&self) -> &str {
        "fight"
    }

    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        self.last_attack_sent = false;
        // Filter first, outside the controller: a target that stops being valid is dropped.
        let current = self.target.and_then(|id| view.entity(id).map(|e| (id, self.rules.is_valid_target(view, e))));
        let id = match (self.target, current) {
            (_, Some((id, true))) => id,
            (Some(_), None) => {
                let dead = self.last_target_health.is_some_and(|h| h <= 0.0);
                return self.finish(cmds, if dead { DoneReason::TargetDead } else { DoneReason::TargetLost });
            }
            (Some(t), Some((_, false))) => {
                let dead = view.entity(t).is_some_and(|e| e.health.is_some_and(|h| h <= 0.0));
                return self.finish(cmds, if dead { DoneReason::TargetDead } else { DoneReason::TargetLost });
            }
            (None, _) => match self.pick_target(view) {
                Some(id) => {
                    self.target = Some(id);
                    id
                }
                None => return SkillStatus::Failed("no valid target".into()),
            },
        };
        let Some(obs) = observe(view, id) else {
            return self.finish(cmds, DoneReason::TargetLost);
        };
        self.last_target_health = obs.target.health;
        let mut action = match self.controller.step(&obs) {
            DuelStep::Done(why) => return self.finish(cmds, why),
            DuelStep::Act(a) => a,
        };

        // Cargo-proximity rule (B5 §4.6): near cargo, only axe attacks (no sweep).
        let mut report = ClampReport::default();
        let target_entity = view.entity(id).expect("observed");
        if action.attack && self.near_cargo(view, target_entity) && !item_is_axe(obs.held_item.as_deref()) {
            action.attack = false;
            report.attacks_dropped_cargo += 1;
        }
        let (mut action, clamp_report) = self.clamps.apply(obs.tick, &obs, action);
        self.last_ctrl_walk = (action.forward, action.strafe);
        // A target below and out of reach: drop onto it when the fall leaves us ahead. No MLG
        // bucket here: placing it costs the time for the enemy to land one or two hits.
        let t = &obs.target;
        self.last_drop_in = t.offset.y < -2.0
            && t.reach_distance > 3.0
            && drop_depth(view, 1, 0, t.face.yaw)
                .is_some_and(|d| drop_worth(obs.health, t.health, d + obs.fall_distance.ceil() as i32)); // mid-air: what we already fell counts
        if self.last_drop_in {
            action.forward = 1;
            action.strafe = 0;
            action.yaw = t.face.yaw;
            action.jump = false;
        }
        // Never walk off a drop the controller cannot see (it only knows the target).
        self.last_drop_guard = !self.last_drop_in && drop_ahead(view, action.forward, action.strafe, action.yaw);
        if self.last_drop_guard {
            action.forward = 0;
            action.strafe = 0;
            action.sprint = false;
        }
        // The controllers only jump for crits; a full block in the way must be jumped too.
        self.last_step_jump = !action.jump && view.on_ground() && step_ahead(view, action.forward, action.strafe, action.yaw);
        if self.last_step_jump {
            action.jump = true;
        }
        report.add(&clamp_report);

        // Our own shield state, not the server's echo of it, which lags (see ActionApplier).
        let sent = self.applier.apply(cmds, &action, id);
        if !sent {
            report.attacks_held_for_shield += 1;
        }
        self.last_attack_sent = sent && action.attack;
        self.last_action = Some(action);
        self.trace.push(TraceRecord { tick: obs.tick, obs: obs.to_vec(), action: action.to_vec(), clamped: report });
        SkillStatus::Running
    }
}

fn walk_dir(forward: i8, strafe: i8, yaw_deg: f32) -> Option<(f64, f64)> {
    if forward == 0 && strafe == 0 {
        return None;
    }
    let (f, l) = (forward as f64, -(strafe as f64));
    let (sin, cos) = (yaw_deg as f64).to_radians().sin_cos();
    let (dx, dz) = (-f * sin + l * cos, f * cos + l * sin);
    let n = (dx * dx + dz * dz).sqrt();
    Some((dx / n, dz / n))
}

/// More than 3 blocks of air below a point about one block ahead: walking on falls (B5 has no
/// fall-damage reflex yet). Unknown chunks count as a drop.
pub fn drop_ahead(view: &dyn BotView, forward: i8, strafe: i8, yaw_deg: f32) -> bool {
    let Some((dx, dz)) = walk_dir(forward, strafe, yaw_deg) else { return false };
    let me = view.position();
    let y = (me.y + 0.01).floor() as i32;
    [0.8, 1.6].iter().any(|d| {
        let (x, z) = ((me.x + dx * d).floor() as i32, (me.z + dz * d).floor() as i32);
        !(-4..=0).any(|dy| view.is_solid(BlockPos::new(x, y + dy, z)))
    })
}

/// How far down the ground is about one block ahead, as the fall distance a step off the edge
/// gives (0 on flat ground). None when no ground is found within 24 blocks (void, unloaded).
pub fn drop_depth(view: &dyn BotView, forward: i8, strafe: i8, yaw_deg: f32) -> Option<i32> {
    let (dx, dz) = walk_dir(forward, strafe, yaw_deg)?;
    let me = view.position();
    let y = (me.y + 0.01).floor() as i32;
    [0.8, 1.6]
        .iter()
        .map(|d| {
            let (x, z) = ((me.x + dx * d).floor() as i32, (me.z + dz * d).floor() as i32);
            (0..=24).find(|k| view.is_solid(BlockPos::new(x, y - k, z))).map(|k| (k - 1).max(0))
        })
        .try_fold(0, |deepest, d| d.map(|d| deepest.max(d)))
}

// Spec: specs/B5-combat.md §4.5 (dropping onto an enemy)
/// Allen, 2026-10-16: drop onto an enemy only if we survive the fall and land still healthier
/// than it. An unknown enemy health never qualifies.
pub fn drop_worth(our_hp: f32, enemy_hp: Option<f32>, fall: i32) -> bool {
    let safe = null_mechanics::mechanic("movement.safe_fall_distance").map_or(3.0, |m| m.value);
    let after = our_hp - (fall as f64 - safe).max(0.0).ceil() as f32;
    after > 0.0 && enemy_hp.is_some_and(|e| e < after)
}

/// A full block at feet height just ahead in the walking direction, with room to stand on it.
/// Lower steps (slabs, paths) are climbed by physics without a jump.
pub fn step_ahead(view: &dyn BotView, forward: i8, strafe: i8, yaw_deg: f32) -> bool {
    if forward == 0 && strafe == 0 {
        return false;
    }
    let (f, l) = (forward as f64, -(strafe as f64));
    let (sin, cos) = (yaw_deg as f64).to_radians().sin_cos();
    let (dx, dz) = (-f * sin + l * cos, f * cos + l * sin);
    let n = (dx * dx + dz * dz).sqrt();
    let me = view.position();
    let (x, z) = (me.x + dx / n * 0.6, me.z + dz / n * 0.6);
    let y = (me.y + 0.01).floor() as i32;
    let at = |dy: i32| BlockPos::new(x.floor() as i32, y + dy, z.floor() as i32);
    view.is_solid(at(0)) && !view.is_solid(at(1)) && !view.is_solid(at(2))
}

/// A wall at head height about a block ahead: neither walking nor a jump gets past it.
pub fn wall_ahead(view: &dyn BotView, yaw_deg: f32) -> bool {
    let Some((dx, dz)) = walk_dir(1, 0, yaw_deg) else { return false };
    let me = view.position();
    let y = (me.y + 0.01).floor() as i32 + 1;
    view.is_solid(BlockPos::new((me.x + dx * 0.8).floor() as i32, y, (me.z + dz * 0.8).floor() as i32))
}

/// Something at head height on the straight line between two points (trees, walls): walking
/// straight at the target would get stuck, so the pathfinder has to go around.
pub fn line_blocked(view: &dyn BotView, from: Vec3, to: Vec3) -> bool {
    let (dx, dz) = (to.x - from.x, to.z - from.z);
    let len = (dx * dx + dz * dz).sqrt();
    let steps = (len / 0.5).floor() as i32;
    let y = (from.y + 0.01).floor() as i32 + 1;
    (1..steps).any(|i| {
        let t = i as f64 * 0.5 / len;
        view.is_solid(BlockPos::new((from.x + dx * t).floor() as i32, y, (from.z + dz * t).floor() as i32))
    })
}
