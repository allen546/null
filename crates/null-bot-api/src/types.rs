//! Plain value types shared by the view and the commands.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3::new(0.0, 0.0, 0.0);

    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
    pub fn sub(self, o: Vec3) -> Vec3 {
        Vec3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn length(self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
    pub fn distance(self, o: Vec3) -> f64 {
        self.sub(o).length()
    }
    pub fn block_pos(self) -> BlockPos {
        BlockPos::new(self.x.floor() as i32, self.y.floor() as i32, self.z.floor() as i32)
    }
}

/// Minecraft convention: yaw 0 = +Z (south), degrees; pitch -90 up, +90 down.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Look {
    pub yaw: f32,
    pub pitch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// Item id is the namespaced name, e.g. "minecraft:iron_sword".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: String,
    pub count: u8,
}

impl ItemStack {
    pub fn new(item: &str, count: u8) -> Self {
        Self { item: item.to_string(), count }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Main,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UsingItem {
    pub hand: Hand,
    /// Ticks the item has been in use.
    pub ticks: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attacker {
    pub entity: EntityId,
    pub tick: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntitySnapshot {
    pub id: EntityId,
    /// Namespaced kind, e.g. "minecraft:player".
    pub kind: String,
    pub is_player: bool,
    pub position: Vec3,
    pub velocity: Vec3,
    pub look: Look,
    pub on_ground: bool,
    /// None when the server has not told us.
    pub health: Option<f32>,
    pub held_item: Option<ItemStack>,
    pub blocking_with_shield: bool,
    /// Username, players only (None if not yet known).
    pub name: Option<String>,
}

impl EntitySnapshot {
    pub fn new(id: u32, kind: &str, position: Vec3) -> Self {
        Self {
            id: EntityId(id),
            kind: kind.to_string(),
            is_player: kind == "minecraft:player",
            position,
            velocity: Vec3::ZERO,
            look: Look::default(),
            on_ground: true,
            health: None,
            held_item: None,
            blocking_with_shield: false,
            name: None,
        }
    }
}

/// Pathfinder state as seen by decision code. `Idle` also covers "arrived" and "gave up":
/// compare the position with the target to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum NavStatus {
    #[default]
    Idle,
    Calculating,
    Following,
    /// A goal is held but no path is running and no search is underway: Azalea keeps the goal
    /// when a path ends short of it. Nothing will happen until a new goto.
    Stalled,
}
