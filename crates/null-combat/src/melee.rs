use crate::rng::Rng;
use crate::{DoneReason, Doctrine, DuelAction, DuelController, DuelObs, DuelStep, ParamSpace, ParamSpec, Params};

const fn spec(name: &'static str, min: f64, max: f64, default: f64, unit: &'static str) -> ParamSpec {
    ParamSpec { name, min, max, default, unit }
}

pub const MELEE_BASIC_SPACE: ParamSpace = ParamSpace(&[
    spec("spacing", 1.5, 3.5, 2.6, "blocks (horizontal)"),
    spec("strafe_period", 4.0, 40.0, 12.0, "ticks"),
    spec("strafe_jitter", 0.0, 1.0, 0.25, "fraction of period"),
    spec("attack_at_cooldown", 0.5, 1.0, 0.95, "cooldown fraction"),
    spec("crit", 0.0, 1.0, 1.0, "switch (>= 0.5 is on)"),
    spec("wtap_ticks", 0.0, 6.0, 2.0, "ticks"),
    spec("shield_react", 0.0, 1.0, 1.0, "switch (>= 0.5 is on)"),
    spec("retreat_hp", 0.0, 20.0, 6.0, "half-hearts"),
    spec("crit_wait_ticks", 0.0, 40.0, 6.0, "ticks"),
]);

const LOST_RANGE: f64 = 32.0;
/// Margin under the vanilla reach so lag and rounding do not turn swings into misses.
const REACH_MARGIN: f64 = 0.1;
/// Jump for a crit once the cooldown is this far along, so we land near full charge.
const CRIT_JUMP_COOLDOWN: f32 = 0.4;

pub struct MeleeBasic {
    spacing: f64,
    strafe_period: f64,
    strafe_jitter: f64,
    attack_at: f32,
    crit: bool,
    wtap_ticks: u32,
    shield_react: bool,
    retreat_hp: f32,
    crit_wait: u32,
    reach: f64,
    rng: Rng,
    started: bool,
    strafe_dir: i8,
    strafe_left: u32,
    prev_cooldown: f32,
    wtap_left: u32,
    last_sprint: bool,
    /// Ticks spent ready to swing while no valid crit came.
    ready_wait: u32,
}

impl MeleeBasic {
    pub fn new(p: &Params) -> Result<Self, String> {
        MELEE_BASIC_SPACE.validate(p)?;
        let g = |n: &str| p.get(n).unwrap_or(0.0);
        let reach = null_mechanics::mechanic("combat.entity_interaction_range").map_or(3.0, |m| m.value);
        let mut c = Self {
            spacing: g("spacing"),
            strafe_period: g("strafe_period"),
            strafe_jitter: g("strafe_jitter"),
            attack_at: g("attack_at_cooldown") as f32,
            crit: g("crit") >= 0.5,
            wtap_ticks: g("wtap_ticks").round() as u32,
            shield_react: g("shield_react") >= 0.5,
            retreat_hp: g("retreat_hp") as f32,
            crit_wait: g("crit_wait_ticks").round() as u32,
            reach: reach - REACH_MARGIN,
            rng: Rng::new(0),
            started: false,
            strafe_dir: 1,
            strafe_left: 0,
            prev_cooldown: 1.0,
            wtap_left: 0,
            last_sprint: false,
            ready_wait: 0,
        };
        c.reset(0);
        Ok(c)
    }

    pub fn from_doctrine(d: &Doctrine) -> Result<Self, String> {
        if d.technique != "melee_basic" {
            return Err(format!("not a melee_basic doctrine: {}", d.technique));
        }
        Self::new(&d.params)
    }

    fn next_period(&mut self) -> u32 {
        let j = 1.0 + self.strafe_jitter * (self.rng.next_f64() * 2.0 - 1.0);
        ((self.strafe_period * j).round() as i64).max(1) as u32
    }
}

impl Default for MeleeBasic {
    fn default() -> Self {
        Self::new(&MELEE_BASIC_SPACE.defaults()).expect("defaults are valid")
    }
}

impl DuelController for MeleeBasic {
    fn id(&self) -> &str {
        "melee_basic"
    }

    fn reset(&mut self, seed: u64) {
        self.rng = Rng::new(seed);
        self.started = false;
        self.strafe_left = 0;
        self.prev_cooldown = 1.0;
        self.wtap_left = 0;
        self.last_sprint = false;
        self.ready_wait = 0;
    }

    fn step(&mut self, o: &DuelObs) -> DuelStep {
        let t = &o.target;
        if t.health.is_some_and(|h| h <= 0.0) {
            return DuelStep::Done(DoneReason::TargetDead);
        }
        if t.horizontal_distance > LOST_RANGE {
            return DuelStep::Done(DoneReason::TargetLost);
        }
        if o.health < self.retreat_hp {
            return DuelStep::Done(DoneReason::Retreat);
        }

        // A cooldown drop means we swung; clamps may have eaten our attack, so look at the result.
        if self.prev_cooldown - o.cooldown > 0.5 && self.wtap_ticks > 0 {
            self.wtap_left = self.wtap_ticks;
        }
        self.prev_cooldown = o.cooldown;

        if !self.started {
            self.started = true;
            self.strafe_dir = if self.rng.next_u64() & 1 == 0 { 1 } else { -1 };
            self.strafe_left = self.next_period();
        } else if self.strafe_left == 0 {
            self.strafe_dir = -self.strafe_dir;
            self.strafe_left = self.next_period();
        }
        self.strafe_left -= 1;

        let hd = t.horizontal_distance;
        let forward: i8 = if hd > self.spacing + 0.1 {
            1
        } else if hd < self.spacing - 0.4 {
            -1
        } else {
            0
        };
        let strafe = if hd < self.spacing + 3.0 { self.strafe_dir } else { 0 };
        let mut sprint = forward >= 0;

        let safe = !o.in_water && !o.in_lava;
        let crit_mode = self.crit && safe;
        if crit_mode && !o.on_ground {
            sprint = false; // a crit needs us not sprinting
        }
        if self.wtap_left > 0 {
            sprint = false;
            self.wtap_left -= 1;
        }

        let in_reach = t.reach_distance <= self.reach;
        let ready = o.cooldown >= self.attack_at && in_reach;
        let crit_valid = safe
            && !o.on_ground
            && o.fall_distance > 0.0
            && o.sprinting != Some(true)
            && !self.last_sprint;
        // Crits are opportunistic: after crit_wait ticks ready without one, swing anyway.
        self.ready_wait = if ready { self.ready_wait + 1 } else { 0 };
        let attack = ready && (!crit_mode || crit_valid || self.ready_wait > self.crit_wait);
        let jump = crit_mode
            && o.on_ground
            && o.cooldown >= CRIT_JUMP_COOLDOWN
            && t.reach_distance <= self.reach + 1.0;

        let block = !attack
            && self.shield_react
            && o.offhand_shield
            && o.cooldown < 0.5
            && t.reach_distance <= self.reach + 0.5
            && !t.blocking;

        self.last_sprint = sprint;
        DuelStep::Act(DuelAction {
            forward,
            strafe,
            sprint,
            jump,
            sneak: false,
            yaw: t.face.yaw,
            pitch: t.face.pitch,
            attack,
            block_with_shield: block,
            select_slot: None,
        })
    }
}
