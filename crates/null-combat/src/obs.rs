use null_bot_api::{BotView, EntityId, Hand, Look, Vec3};

/// Order of `DuelObs::to_vec()`. Booleans are 0/1; unknown values are -1.
pub const OBS_FIELDS: &[&str] = &[
    "tick", "health", "cooldown", "on_ground", "in_water", "in_lava", "fall_distance",
    "vel_x", "vel_y", "vel_z", "yaw", "pitch", "sprinting", "held_axe", "held_sword",
    "offhand_shield", "using_shield",
    "t_dx", "t_dy", "t_dz", "t_dist", "t_hdist", "t_reach", "t_vx", "t_vy", "t_vz",
    "t_face_yaw", "t_face_pitch", "t_look_yaw", "t_look_pitch", "t_on_ground",
    "t_blocking", "t_held_axe", "t_held_sword", "t_health",
];

const EYE_HEIGHT: f64 = 1.62;
/// Rough hit-box centre height used for the reach estimate.
const TARGET_CENTRE: f64 = 0.9;
/// Players get their eyes aimed at; other kinds get a lower point (approximation).
const OTHER_AIM: f64 = 1.0;
const HALF_WIDTH: f64 = 0.3;

pub fn item_is_axe(item: Option<&str>) -> bool {
    item.is_some_and(|i| i.ends_with("_axe"))
}
pub fn item_is_sword(item: Option<&str>) -> bool {
    item.is_some_and(|i| i.ends_with("_sword"))
}

#[derive(Clone, Debug, PartialEq)]
pub struct TargetObs {
    pub id: EntityId,
    /// Target feet minus own feet.
    pub offset: Vec3,
    pub distance: f64,
    pub horizontal_distance: f64,
    /// Own eye to target box, approximated: eye-to-centre distance minus the horizontal half-width (0.3).
    pub reach_distance: f64,
    pub velocity: Vec3,
    /// Yaw/pitch that put the crosshair on the target's eyes.
    pub face: Look,
    pub look: Look,
    pub on_ground: bool,
    pub blocking: bool,
    pub held_item: Option<String>,
    pub health: Option<f32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DuelObs {
    pub tick: u64,
    pub health: f32,
    pub cooldown: f32,
    pub on_ground: bool,
    pub in_water: bool,
    pub in_lava: bool,
    pub fall_distance: f32,
    pub velocity: Vec3,
    pub look: Look,
    /// The view does not expose the sprint flag, so `observe` leaves this None.
    pub sprinting: Option<bool>,
    pub held_item: Option<String>,
    pub offhand_shield: bool,
    pub using_shield: bool,
    pub target: TargetObs,
}

fn b(x: bool) -> f32 {
    x as u8 as f32
}

impl DuelObs {
    pub fn to_vec(&self) -> Vec<f32> {
        let t = &self.target;
        let h = self.held_item.as_deref();
        let th = t.held_item.as_deref();
        vec![
            self.tick as f32, self.health, self.cooldown, b(self.on_ground), b(self.in_water),
            b(self.in_lava), self.fall_distance,
            self.velocity.x as f32, self.velocity.y as f32, self.velocity.z as f32,
            self.look.yaw, self.look.pitch,
            self.sprinting.map_or(-1.0, b),
            b(item_is_axe(h)), b(item_is_sword(h)), b(self.offhand_shield), b(self.using_shield),
            t.offset.x as f32, t.offset.y as f32, t.offset.z as f32,
            t.distance as f32, t.horizontal_distance as f32, t.reach_distance as f32,
            t.velocity.x as f32, t.velocity.y as f32, t.velocity.z as f32,
            t.face.yaw, t.face.pitch, t.look.yaw, t.look.pitch,
            b(t.on_ground), b(t.blocking), b(item_is_axe(th)), b(item_is_sword(th)),
            t.health.unwrap_or(-1.0),
        ]
    }
}

/// Pure read of the view. None when the target is not in the entity list.
pub fn observe(view: &dyn BotView, target: EntityId) -> Option<DuelObs> {
    let e = view.entity(target)?;
    let me = view.position();
    let off = e.position.sub(me);
    let hd = off.x.hypot(off.z);
    let aim_h = if e.is_player { EYE_HEIGHT } else { OTHER_AIM };
    let dy = (e.position.y + aim_h) - (me.y + EYE_HEIGHT);
    let own_look = view.look();
    let face = if hd < 1e-6 {
        Look { yaw: own_look.yaw, pitch: if dy > 0.0 { -90.0 } else { 90.0 } }
    } else {
        Look {
            yaw: (-off.x).atan2(off.z).to_degrees() as f32,
            pitch: (-dy.atan2(hd)).to_degrees() as f32,
        }
    };
    let centre = Vec3::new(e.position.x, e.position.y + TARGET_CENTRE, e.position.z);
    let reach = (centre.distance(Vec3::new(me.x, me.y + EYE_HEIGHT, me.z)) - HALF_WIDTH).max(0.0);
    let using_shield = view.using_item().is_some_and(|u| {
        let item = match u.hand {
            Hand::Main => view.held_item(),
            Hand::Off => view.offhand(),
        };
        item.is_some_and(|i| i.item == "minecraft:shield")
    });
    Some(DuelObs {
        tick: view.tick(),
        health: view.health(),
        cooldown: view.attack_cooldown(),
        on_ground: view.on_ground(),
        in_water: view.in_water(),
        in_lava: view.in_lava(),
        fall_distance: view.fall_distance(),
        velocity: view.velocity(),
        look: own_look,
        sprinting: None,
        held_item: view.held_item().map(|i| i.item.clone()),
        offhand_shield: view.offhand().is_some_and(|i| i.item == "minecraft:shield"),
        using_shield,
        target: TargetObs {
            id: target,
            offset: off,
            distance: off.length(),
            horizontal_distance: hd,
            reach_distance: reach,
            velocity: e.velocity,
            face,
            look: e.look,
            on_ground: e.on_ground,
            blocking: e.blocking_with_shield,
            held_item: e.held_item.as_ref().map(|i| i.item.clone()),
            health: e.health,
        },
    })
}
