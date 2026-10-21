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
    /// A newer action of the same kind replaced the fork's record before the server answered, so
    /// the outcome is unknown (read `BotView::block_at`). Only when the skill sends the next
    /// action before this one is acknowledged (B6 §4.6).
    Superseded,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockActionKind {
    Break,
    Place,
}

/// The latest break or placement. For a place, `pos` is the block that should appear
/// (the clicked block's neighbour), not the clicked block.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    /// What the client's check saw when it refused (B6 §4.1); `None` for other outcomes.
    pub evidence: Option<ClickEvidence>,
}

/// What the crosshair ray hit when a click was checked.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ClickHit {
    Miss,
    Block { pos: BlockPos, face: Face, point: Vec3 },
    Entity { id: EntityId, kind: &'static str },
}

/// The values a refused click was checked with (B6 §4.1): the client's own eye, look and ray hit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClickEvidence {
    /// The client's tick count when the click was checked.
    pub check_tick: u64,
    pub eye: Vec3,
    /// The eye height above the feet the check used (it follows the pose).
    pub eye_height: f64,
    pub yaw: f32,
    pub pitch: f32,
    pub crouching: bool,
    pub hit: ClickHit,
    /// The look changed after the crosshair was last picked, so the hit may predate the look.
    pub look_changed_after_hit: bool,
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
#[derive(Clone, Debug, PartialEq)]
pub struct ItemStack {
    pub item: String,
    pub count: u8,
    /// For an item that blocks attacks (a shield): its `blocks_attacks` component.
    pub blocks_attacks: Option<BlocksAttacks>,
}

impl ItemStack {
    pub fn new(item: &str, count: u8) -> Self {
        Self { item: item.to_string(), count, blocks_attacks: None }
    }

    /// The raise-to-block delay in server ticks, for an item that blocks attacks.
    pub fn block_delay_ticks(&self) -> Option<u32> {
        self.blocks_attacks.as_ref().map(|b| b.delay_ticks)
    }
}

/// The `blocks_attacks` component the server sent with an item (vanilla `BlocksAttacks`).
#[derive(Clone, Debug, PartialEq)]
pub struct BlocksAttacks {
    /// Raise-to-block delay in server ticks (`block_delay_seconds` * 20, rounded). A prior; see
    /// `null_mechanics::belief`.
    pub delay_ticks: u32,
    pub reductions: Vec<DamageReduction>,
    /// Damage types that go through (`bypassed_by`); None if the item names no tag.
    pub bypassed_by: Option<DamageTypes>,
}

/// One entry of `damage_reductions`: a hit within `angle_deg` of the facing, of a listed type,
/// loses `base + factor * damage`, clamped to `0..=damage`.
#[derive(Clone, Debug, PartialEq)]
pub struct DamageReduction {
    pub angle_deg: f32,
    /// None: every damage type.
    pub types: Option<DamageTypes>,
    pub base: f32,
    pub factor: f32,
}

/// Damage type ids ("minecraft:arrow"), resolved with the server's registry and tags.
#[derive(Clone, Debug, PartialEq)]
pub enum DamageTypes {
    Known(Vec<String>),
    /// A tag or id the server never sent us.
    Unknown,
}

impl BlocksAttacks {
    // Spec: specs/F5-version-layer.md §4.6 (the item's own rules, as the server sent them)
    /// Whether the raised item surely blocks a hit above 0 damage of `damage_type` from
    /// `angle_deg` off the holder's facing (vanilla `LivingEntity#applyItemBlocking`). False when
    /// it might not: a set we can't resolve, or a reduction that is 0 for some damage.
    pub fn surely_blocks(&self, damage_type: &str, angle_deg: f64) -> bool {
        let has = |t: &DamageTypes| matches!(t, DamageTypes::Known(v) if v.iter().any(|x| x == damage_type));
        if self.bypassed_by.as_ref().is_some_and(|t| !matches!(t, DamageTypes::Known(_)) || has(t)) {
            return false;
        }
        // Blocked when the summed reduction is above 0; each term is clamped to 0 or more.
        self.reductions.iter().any(|r| {
            angle_deg <= r.angle_deg as f64
                && r.types.as_ref().is_none_or(has)
                && ((r.base >= 0.0 && r.factor > 0.0) || (r.base > 0.0 && r.factor >= 0.0))
        })
    }
}

/// A click on a window slot, as a vanilla client makes them (B3 §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Click {
    /// Pick up the stack, put the carried stack down, or swap them. Outside the window: drop the
    /// carried stack.
    Left,
    /// Pick up half, or put one down. Outside the window: drop one carried item.
    Right,
    /// Shift + left click: move the stack to the other part of the window.
    ShiftLeft,
    /// Number key over the slot: swap it with hotbar slot `0..=8`.
    SwapHotbar(u8),
    /// F over the slot: swap it with the offhand.
    SwapOffhand,
    /// Q over the slot: drop one.
    Drop,
    /// Ctrl + Q over the slot: drop the stack.
    DropStack,
    /// Double click with a carried stack: gather matching items onto it.
    DoubleClick,
}

/// The open window: the player's inventory or a container (B3 §4.1, §4.8).
#[derive(Clone, Debug, PartialEq)]
pub struct Window {
    /// The server's window id; 0 for the player's own inventory.
    pub id: i32,
    /// The menu kind of a container ("minecraft:generic_9x3", "minecraft:hopper"); None for the
    /// player's own inventory.
    pub kind: Option<&'static str>,
    /// Every slot in the window's own numbering (the player inventory window: 0 crafting result,
    /// 1-4 crafting grid, 5-8 armour head to feet, 9-35 main, 36-44 hotbar, 45 offhand). A
    /// container lists its own slots, then the main inventory, then the hotbar.
    pub slots: Vec<Option<ItemStack>>,
    /// Window slots whose content the server computes (a crafting result), so an update there
    /// after a click is not a correction. Empty for storage blocks.
    pub computed_slots: Vec<u16>,
    /// The stack on the cursor.
    pub carried: Option<ItemStack>,
}

impl Window {
    pub fn is_player_inventory(&self) -> bool {
        self.id == 0
    }

    // Spec: specs/B3-inventory-crafting.md §4.1 (slot numbering)
    /// The window slot that shows inventory slot `i` (`BotView::inventory` numbering: 0-8
    /// hotbar, 9-35 main).
    pub fn slot_of_inventory(&self, i: usize) -> Option<u16> {
        if i > 35 {
            return None;
        }
        let start = if self.is_player_inventory() { 9 } else { self.slots.len().checked_sub(36)? };
        Some((if i < 9 { start + 27 + i } else { start + i - 9 }) as u16)
    }
}

/// What came of the latest `open_container` or `open_entity` (B3 §4.7).
#[derive(Clone, Debug, PartialEq)]
pub struct OpenResult {
    pub target: OpenTarget,
    pub outcome: OpenOutcome,
    /// Why the client refused it (nothing was sent), if it did.
    pub refused: Option<&'static str>,
    /// What the client's check saw when it refused after casting the ray (B6 §4.1).
    pub evidence: Option<ClickEvidence>,
    /// Ticks since the click was sent.
    pub ticks: u32,
}

/// What was right-clicked to open a window: a block, or an entity container (chest minecart,
/// hopper minecart, chest boat; B3 §4.7).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OpenTarget {
    Block(BlockPos),
    Entity(EntityId),
    /// The player's own inventory (`open_inventory`).
    Inventory,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpenOutcome {
    /// Sent; the server has not answered.
    Pending,
    /// The server opened this window.
    Opened { window_id: i32, kind: &'static str },
    /// Sent and acknowledged, but no screen came (a chest with a solid block on top).
    NoScreen,
    Refused,
    /// A newer open replaced the fork's record before the server answered; the outcome is unknown
    /// (read `BotView::window`).
    Superseded,
}

/// What came of the latest click (B3 §4.1).
#[derive(Clone, Debug, PartialEq)]
pub struct ClickOutcome {
    /// The clicked window slot; None outside the window.
    pub slot: Option<u16>,
    /// Why the client refused it (nothing was sent), if it did.
    pub refused: Option<&'static str>,
    /// Ticks since the click.
    pub ticks: u32,
    /// Window slots the server set after the click (`None` = the cursor). Vanilla sends a slot
    /// after a click only where its result differs from ours, so these are corrections (or the
    /// server's own changes, such as a crafting result).
    pub corrected: Vec<Option<u16>>,
    /// Times the server resent the whole window after the click.
    pub full_resends: u32,
    /// A later click came before this one settled, so `corrected` and `full_resends` hold only
    /// what arrived before it (the fork attributes later updates to the later click).
    pub superseded: bool,
}

impl ClickOutcome {
    /// Sent, and the server has changed nothing since.
    pub fn uncorrected(&self) -> bool {
        self.refused.is_none() && self.corrected.is_empty() && self.full_resends == 0
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

// Spec: specs/F1-swarm-runtime.md §4.4 (dead, then respawning)
/// Whether the bot is alive. The host keeps the bot's brain and view through a death, so
/// reflexes see `Dead` from the death tick on (the running skill ends with `Failed("died")` that
/// tick). A dead bot can send nothing but the respawn (`BotView::refused_while_dead`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Life {
    Alive,
    /// The death screen is up and no respawn request has gone out; `since_tick` is the host tick
    /// it opened.
    Dead { since_tick: u64 },
    /// The respawn request went out (`since_tick`) and the server's answer has not come.
    RespawnPending { since_tick: u64 },
}

// Spec: specs/B2-survival-upkeep.md §4.7 (the death report: position and inventory snapshot)
/// The latest death, as seen when the kill packet arrived (before the server's drops).
#[derive(Clone, Debug, PartialEq)]
pub struct Death {
    /// Host tick of the death (same clock as `BotView::tick`).
    pub tick: u64,
    pub position: Vec3,
    pub dimension: String,
    /// The server's death message as plain text (empty with `showDeathMessages` off).
    pub message: String,
    /// The 36 main slots as `BotView::inventory` numbers them, then armour (head to feet) and
    /// the offhand.
    pub inventory: [Option<ItemStack>; 36],
    pub armor: [Option<ItemStack>; 4],
    pub offhand: Option<ItemStack>,
    /// The respawn that ended this death, once its request went out (whoever asked: a skill, a
    /// reflex or the host). A command's own result is `BotView::action`.
    pub respawn: Option<RespawnSent>,
}

/// A respawn request: what went out, or why nothing did.
#[derive(Clone, Debug, PartialEq)]
pub struct RespawnSent {
    /// Host tick the packet went out.
    pub sent_tick: u64,
    /// Ticks the death screen had been up (20 at the earliest; 0 with immediate respawn).
    pub screen_ticks: u32,
    pub immediate: bool,
    /// Host tick the server's respawn packet arrived.
    pub completed_tick: Option<u64>,
    /// Respawn packets sent for this death: one with the death screen; with immediate respawn
    /// one from the kill packet and one per tick until the answer, as a vanilla client does.
    pub sends: u32,
    /// Why the client refused the request (nothing was sent: "not dead", hardcore), or "no answer"
    /// (sent, but the server's respawn packet did not come in time). The other fields are 0/false
    /// for a refusal.
    pub refused: Option<&'static str>,
}

// Spec: specs/F1-swarm-runtime.md §4.4 (a dead bot sends nothing a death screen cannot)
/// An action the fork refused on the client because the bot is dead, with how many times since
/// the death began.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeadRefusal {
    pub action: String,
    pub reason: &'static str,
    pub count: u32,
}

// Spec: specs/F5-version-layer.md §4.6 (inputs of the client's own computations that Null does not model)
/// What an unmodelled input changes. `Movement` moves the bot even with no input, so stopping the
/// skill is not enough: the fork disconnects the bot at once and the host reports it
/// ([`UnmodelledDisconnect`]). `Action` changes only what an action or a plan computes (reach,
/// break time, the gravity a skill plans with): the skill stops and the bot stays connected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnmodelledKind {
    Movement,
    Action,
}

/// An attribute, status effect, vehicle or surrounding the server has set that changes what a
/// vanilla client computes for itself, and that Null does not model (or only models in the
/// physics, not in the plans). While any is in force the host ends the running skill and nothing
/// moves or breaks.
#[derive(Clone, Debug, PartialEq)]
pub enum UnmodelledInput {
    /// Attribute id ("minecraft:scale") and its current value.
    Attribute { id: String, value: f64, kind: UnmodelledKind },
    /// Effect id ("minecraft:levitation") and its amplifier (0 is level I).
    Effect { id: String, amplifier: i32, kind: UnmodelledKind },
    /// The bot is a passenger of a vehicle.
    Passenger { kind: UnmodelledKind },
    /// The bot's box touches a block the fork does not model (id "minecraft:bubble_column").
    Block { pos: BlockPos, id: String, kind: UnmodelledKind },
    /// The bot's box is outside or at the world border.
    WorldBorder { kind: UnmodelledKind },
}

impl std::fmt::Display for UnmodelledInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnmodelledInput::Attribute { id, value, .. } => write!(f, "attribute {id}={value} ({:?})", self.kind()),
            UnmodelledInput::Effect { id, amplifier, .. } => write!(f, "effect {id}={amplifier} ({:?})", self.kind()),
            UnmodelledInput::Passenger { .. } => write!(f, "passenger ({:?})", self.kind()),
            UnmodelledInput::Block { pos, id, .. } => write!(f, "block {id} at {},{},{} ({:?})", pos.x, pos.y, pos.z, self.kind()),
            UnmodelledInput::WorldBorder { .. } => write!(f, "world border ({:?})", self.kind()),
        }
    }
}

/// The fork closed the bot's connection because a `Movement` input was in force (AD-F5.5). The bot
/// sends nothing more; staying off is the default.
#[derive(Clone, Debug, PartialEq)]
pub struct UnmodelledDisconnect {
    /// The client's tick count when the connection was closed.
    pub tick: u64,
    /// The `Movement` entries that caused it.
    pub inputs: Vec<UnmodelledInput>,
    /// Packets the client tried to send after the trigger (dropped, never written). Must be 0.
    pub packets_after: u32,
}

impl UnmodelledInput {
    pub fn kind(&self) -> UnmodelledKind {
        match self {
            UnmodelledInput::Attribute { kind, .. }
            | UnmodelledInput::Effect { kind, .. }
            | UnmodelledInput::Passenger { kind }
            | UnmodelledInput::Block { kind, .. }
            | UnmodelledInput::WorldBorder { kind } => *kind,
        }
    }
}
