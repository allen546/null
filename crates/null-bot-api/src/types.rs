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
    /// The neighbour across `face`.
    pub fn offset(self, face: Face) -> BlockPos {
        let (dx, dy, dz) = face.delta();
        BlockPos::new(self.x + dx, self.y + dy, self.z + dz)
    }
    pub fn center(self) -> Vec3 {
        Vec3::new(self.x as f64 + 0.5, self.y as f64 + 0.5, self.z as f64 + 0.5)
    }
}

/// A block face; the direction it points.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Face {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

impl Face {
    /// Unit step out of the face (north = -Z, east = +X, as in Minecraft).
    pub fn delta(self) -> (i32, i32, i32) {
        match self {
            Face::Down => (0, -1, 0),
            Face::Up => (0, 1, 0),
            Face::North => (0, 0, -1),
            Face::South => (0, 0, 1),
            Face::West => (-1, 0, 0),
            Face::East => (1, 0, 0),
        }
    }
}

/// How far a break or placement has got, as the server told us (B6 §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockOutcome {
    /// Sent; the server's acknowledgement has not been read yet.
    Pending,
    /// Acknowledged and the block is now as intended (air for a break, changed for a place).
    Done,
    /// Not placed or broken: refused by the client's own checks, or acknowledged (or timed out,
    /// reason "no ack") with the block unchanged.
    Refused,
    /// `cancel_break` ended it.
    Cancelled,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    Break,
    Place,
}

/// The latest break or placement. For a place, `pos` is the block that should appear
/// (the clicked block's neighbour), not the clicked block.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockAction {
    pub kind: BlockActionKind,
    pub pos: BlockPos,
    pub outcome: BlockOutcome,
    /// Host tick the command was applied.
    pub start_tick: u64,
    /// Break only: host tick of the last mining tick (when the client sent the finishing packet).
    pub finish_tick: Option<u64>,
    /// Host tick the server's acknowledgement was read, if it was.
    pub ack_tick: Option<u64>,
    /// Why a `Refused` action was refused: the client refused the click itself and sent nothing
    /// ("out of reach", "crosshair does not hit it", ...), or "no ack" (sent, the server never answered).
    pub reason: Option<&'static str>,
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

impl Look {
    /// The look that points from `from` (the eye) at `to`.
    pub fn towards(from: Vec3, to: Vec3) -> Look {
        let d = to.sub(from);
        let flat = (d.x * d.x + d.z * d.z).sqrt();
        Look { yaw: (-d.x).atan2(d.z).to_degrees() as f32, pitch: (-d.y).atan2(flat).to_degrees() as f32 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EntityId(pub u32);

/// Item id is the namespaced name, e.g. "minecraft:iron_sword".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemStack {
    pub item: String,
    pub count: u8,
    /// For an item that blocks attacks (a shield): its raise-to-block delay in server ticks, from
    /// the `blocks_attacks` component the server sent. A prior; see `null_mechanics::belief`.
    pub block_delay_ticks: Option<u32>,
}

impl ItemStack {
    pub fn new(item: &str, count: u8) -> Self {
        Self { item: item.to_string(), count, block_delay_ticks: None }
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
    /// Server ticks of the use, counted from the server's confirmation (to about a tick); None
    /// until the server confirms it.
    pub server_ticks: Option<u32>,
}

/// A hit taken while an item that blocks attacks (`blocks_attacks`) was in use, blocked or not.
#[derive(Clone, Debug, PartialEq)]
pub struct ShieldHit {
    /// Host tick the bot learned of it.
    pub tick: u64,
    /// `UsingItem::server_ticks` when the hit came.
    pub use_ticks: u32,
    pub blocked: bool,
    /// Where the hit came from (the attacker, or the damage's position), if known. None for
    /// blocked hits: the block sound carries no source.
    pub from: Option<Vec3>,
    /// Damage type ("minecraft:player_attack"), if known. None for blocked hits.
    pub damage_type: Option<String>,
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
