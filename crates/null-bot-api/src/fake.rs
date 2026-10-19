//! Test doubles. Public so other decision crates can use them in their tests.

use crate::*;

pub struct FakeView {
    pub tick: u64,
    pub position: Vec3,
    pub velocity: Vec3,
    pub look: Look,
    pub on_ground: bool,
    pub in_water: bool,
    pub in_lava: bool,
    pub fall_distance: f32,
    pub health: f32,
    pub food: u32,
    pub attack_cooldown: f32,
    pub hotbar: [Option<ItemStack>; 9],
    /// Main slots 9-35; slots 0-8 of `BotView::inventory` are read from `hotbar`.
    pub main_inventory: [Option<ItemStack>; 27],
    pub selected_slot: u8,
    pub offhand: Option<ItemStack>,
    pub using_item: Option<UsingItem>,
    pub entities: Vec<EntitySnapshot>,
    pub attackers: Vec<Attacker>,
    pub shield_hits: Vec<ShieldHit>,
    /// Solid blocks. Everything with y < `floor_y` is solid too (flat world).
    pub solid: Vec<BlockPos>,
    pub floor_y: i32,
    /// Block names by position; anything else reads as air above the floor and stone below.
    pub blocks: Vec<(BlockPos, String)>,
    pub last_break: Option<BlockAction>,
    pub last_place: Option<BlockAction>,
    pub nav_status: NavStatus,
    inventory_cache: std::cell::OnceCell<[Option<ItemStack>; 36]>,
}

impl Default for FakeView {
    fn default() -> Self {
        Self {
            tick: 0,
            position: Vec3::new(0.0, 64.0, 0.0),
            velocity: Vec3::ZERO,
            look: Look::default(),
            on_ground: true,
            in_water: false,
            in_lava: false,
            fall_distance: 0.0,
            health: 20.0,
            food: 20,
            attack_cooldown: 1.0,
            hotbar: Default::default(),
            main_inventory: Default::default(),
            selected_slot: 0,
            offhand: None,
            using_item: None,
            entities: Vec::new(),
            attackers: Vec::new(),
            shield_hits: Vec::new(),
            solid: Vec::new(),
            floor_y: 64,
            blocks: Vec::new(),
            last_break: None,
            last_place: None,
            nav_status: NavStatus::Idle,
            inventory_cache: Default::default(),
        }
    }
}

impl FakeView {
    /// An item in use in `hand` for `ticks` ticks, as the adapter's view reports it.
    pub fn start_using(&mut self, hand: Hand, ticks: u32) {
        self.using_item = Some(UsingItem { hand, ticks, server_ticks: None });
    }

    /// A hit by `entity` that the bot learned of at the current tick, newest last.
    pub fn hit_by(&mut self, entity: u32) {
        self.attackers.push(Attacker { entity: EntityId(entity), tick: self.tick });
    }
}

impl BotView for FakeView {
    fn tick(&self) -> u64 {
        self.tick
    }
    fn position(&self) -> Vec3 {
        self.position
    }
    fn velocity(&self) -> Vec3 {
        self.velocity
    }
    fn look(&self) -> Look {
        self.look
    }
    fn on_ground(&self) -> bool {
        self.on_ground
    }
    fn in_water(&self) -> bool {
        self.in_water
    }
    fn in_lava(&self) -> bool {
        self.in_lava
    }
    fn fall_distance(&self) -> f32 {
        self.fall_distance
    }
    fn health(&self) -> f32 {
        self.health
    }
    fn food(&self) -> u32 {
        self.food
    }
    fn attack_cooldown(&self) -> f32 {
        self.attack_cooldown
    }
    fn hotbar(&self) -> &[Option<ItemStack>; 9] {
        &self.hotbar
    }
    fn inventory(&self) -> &[Option<ItemStack>; 36] {
        // Built once; tests set the fields before the first read.
        self.inventory_cache.get_or_init(|| {
            std::array::from_fn(|i| if i < 9 { self.hotbar[i].clone() } else { self.main_inventory[i - 9].clone() })
        })
    }
    fn selected_slot(&self) -> u8 {
        self.selected_slot
    }
    fn offhand(&self) -> Option<&ItemStack> {
        self.offhand.as_ref()
    }
    fn using_item(&self) -> Option<UsingItem> {
        self.using_item
    }
    fn entities(&self) -> &[EntitySnapshot] {
        &self.entities
    }
    fn recent_attackers(&self) -> &[Attacker] {
        &self.attackers
    }
    fn shield_hits(&self) -> &[ShieldHit] {
        &self.shield_hits
    }
    fn is_solid(&self, pos: BlockPos) -> bool {
        pos.y < self.floor_y || self.solid.contains(&pos)
    }
    fn nav_status(&self) -> NavStatus {
        self.nav_status
    }
    fn block_at(&self, pos: BlockPos) -> Option<String> {
        if let Some((_, name)) = self.blocks.iter().rev().find(|(p, _)| *p == pos) {
            return Some(name.clone());
        }
        Some(if pos.y < self.floor_y { "minecraft:stone" } else { "minecraft:air" }.to_string())
    }
    fn last_break(&self) -> Option<BlockAction> {
        self.last_break
    }
    fn last_place(&self) -> Option<BlockAction> {
        self.last_place
    }
}

/// Records every command with the tick it was issued in.
#[derive(Default)]
pub struct FakeCommands {
    pub tick: u64,
    pub log: Vec<(u64, Command)>,
}

impl FakeCommands {
    pub fn set_tick(&mut self, tick: u64) {
        self.tick = tick;
    }
    pub fn all(&self) -> Vec<Command> {
        self.log.iter().map(|(_, c)| *c).collect()
    }
    pub fn at_tick(&self, tick: u64) -> Vec<Command> {
        self.log.iter().filter(|(t, _)| *t == tick).map(|(_, c)| *c).collect()
    }
    fn push(&mut self, c: Command) {
        self.log.push((self.tick, c));
    }
}

impl BotCommands for FakeCommands {
    fn walk(&mut self, forward: i8, strafe: i8) {
        self.push(Command::Walk { forward: forward.clamp(-1, 1), strafe: strafe.clamp(-1, 1) });
    }
    fn sprint(&mut self, on: bool) {
        self.push(Command::Sprint(on));
    }
    fn jump(&mut self) {
        self.push(Command::Jump);
    }
    fn sneak(&mut self, on: bool) {
        self.push(Command::Sneak(on));
    }
    fn look(&mut self, yaw: f32, pitch: f32) {
        self.push(Command::Look { yaw, pitch });
    }
    fn select_slot(&mut self, slot: u8) {
        self.push(Command::SelectSlot(slot));
    }
    fn attack(&mut self, target: EntityId) {
        self.push(Command::Attack(target));
    }
    fn use_item(&mut self, hand: Hand) {
        self.push(Command::UseItem(hand));
    }
    fn release_use(&mut self) {
        self.push(Command::ReleaseUse);
    }
    fn swap_offhand(&mut self) {
        self.push(Command::SwapOffhand);
    }
    fn stop(&mut self) {
        self.push(Command::Stop);
    }
    fn break_block(&mut self, pos: BlockPos) {
        self.push(Command::BreakBlock(pos));
    }
    fn cancel_break(&mut self) {
        self.push(Command::CancelBreak);
    }
    fn place_block(&mut self, pos: BlockPos, face: Face, auto_crouch: bool) {
        self.push(Command::PlaceBlock { pos, face, auto_crouch });
    }
    fn look_at_block(&mut self, pos: BlockPos, face: Option<Face>) {
        self.push(Command::LookAtBlock { pos, face });
    }
    fn goto(&mut self, target: BlockPos, tolerance: f32) {
        self.push(Command::Goto { target, tolerance });
    }
    fn stop_nav(&mut self) {
        self.push(Command::StopNav);
    }
    fn open_inventory(&mut self) {
        self.push(Command::OpenInventory);
    }
    fn open_container(&mut self, pos: BlockPos, face: Option<Face>) {
        self.push(Command::OpenContainer { pos, face });
    }
    fn close_window(&mut self) {
        self.push(Command::CloseWindow);
    }
    fn click_slot(&mut self, slot: Option<u16>, click: Click) {
        self.push(Command::ClickSlot { slot, click });
    }
    fn respawn(&mut self) {
        self.push(Command::Respawn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_recorded_per_tick() {
        let mut c = FakeCommands::default();
        c.walk(1, 0);
        c.set_tick(3);
        c.attack(EntityId(7));
        c.walk(5, -9); // clamped like a real adapter would
        assert_eq!(c.at_tick(0), vec![Command::Walk { forward: 1, strafe: 0 }]);
        assert_eq!(
            c.at_tick(3),
            vec![Command::Attack(EntityId(7)), Command::Walk { forward: 1, strafe: -1 }]
        );
        assert_eq!(c.log.len(), 3);
    }

    #[test]
    fn using_item_and_attackers_are_readable() {
        let mut v = FakeView::default();
        assert!(v.using_item().is_none() && v.recent_attackers().is_empty());
        v.start_using(Hand::Off, 7);
        assert_eq!(v.using_item(), Some(UsingItem { hand: Hand::Off, ticks: 7, server_ticks: None }));
        v.tick = 40;
        v.hit_by(9);
        v.tick = 43;
        v.hit_by(12);
        let a = v.recent_attackers();
        assert_eq!(a.len(), 2);
        assert_eq!((a[1].entity, a[1].tick), (EntityId(12), 43));
        // A skill's "recent" window is its own business: here, the last 5 ticks.
        let now = v.tick();
        assert_eq!(a.iter().filter(|x| now - x.tick <= 5).count(), 2);
        assert_eq!(a.iter().filter(|x| now - x.tick <= 2).count(), 1);
    }

    #[test]
    fn view_defaults_and_helpers() {
        let mut v = FakeView::default();
        v.hotbar[2] = Some(ItemStack::new("minecraft:iron_sword", 1));
        assert!(v.held_item().is_none());
        v.selected_slot = 2;
        assert_eq!(v.held_item().unwrap().item, "minecraft:iron_sword");
        v.entities.push(EntitySnapshot::new(4, "minecraft:player", Vec3::new(1.0, 64.0, 0.0)));
        assert!(v.entity(EntityId(4)).unwrap().is_player);
        assert!(v.entity(EntityId(5)).is_none());
        assert!(v.is_solid(BlockPos::new(0, 63, 0)));
        assert!(!v.is_solid(BlockPos::new(0, 64, 0)));
        v.solid.push(BlockPos::new(0, 64, 0));
        assert!(v.is_solid(BlockPos::new(0, 64, 0)));
        assert!((Vec3::new(3.0, 0.0, 4.0).length() - 5.0).abs() < 1e-9);
    }

    #[test]
    fn block_commands_and_reads() {
        let mut c = FakeCommands::default();
        c.break_block(BlockPos::new(1, 64, 2));
        c.place_block(BlockPos::new(1, 63, 2), Face::Up, false);
        c.cancel_break();
        assert_eq!(
            c.all(),
            vec![
                Command::BreakBlock(BlockPos::new(1, 64, 2)),
                Command::PlaceBlock { pos: BlockPos::new(1, 63, 2), face: Face::Up, auto_crouch: false },
                Command::CancelBreak
            ]
        );
        let mut v = FakeView::default();
        v.blocks.push((BlockPos::new(0, 64, 0), "minecraft:oak_log".into()));
        assert_eq!(v.block_at(BlockPos::new(0, 64, 0)).as_deref(), Some("minecraft:oak_log"));
        assert_eq!(v.block_at(BlockPos::new(0, 65, 0)).as_deref(), Some("minecraft:air"));
        assert_eq!(v.block_at(BlockPos::new(0, 10, 0)).as_deref(), Some("minecraft:stone"));
    }

    #[test]
    fn inventory_covers_hotbar_main_and_offhand() {
        let mut v = FakeView::default();
        v.hotbar[0] = Some(ItemStack::new("minecraft:stone", 10));
        v.main_inventory[0] = Some(ItemStack::new("minecraft:stone", 5)); // slot 9
        v.offhand = Some(ItemStack::new("minecraft:stone", 1));
        assert_eq!(v.inventory()[9].as_ref().unwrap().count, 5);
        assert_eq!(v.count_item("minecraft:stone"), 16);
        assert_eq!(v.count_item("minecraft:dirt"), 0);
    }

    #[test]
    fn faces_offset_blocks_and_looks_aim() {
        assert_eq!(BlockPos::new(0, 64, 0).offset(Face::North), BlockPos::new(0, 64, -1));
        assert_eq!(BlockPos::new(0, 64, 0).offset(Face::Up), BlockPos::new(0, 65, 0));
        let eye = Vec3::new(0.0, 0.0, 0.0);
        let south = Look::towards(eye, Vec3::new(0.0, 0.0, 5.0));
        assert!(south.yaw.abs() < 1e-4 && south.pitch.abs() < 1e-4);
        let east = Look::towards(eye, Vec3::new(5.0, 0.0, 0.0));
        assert!((east.yaw + 90.0).abs() < 1e-4);
        assert!((Look::towards(eye, Vec3::new(0.0, -3.0, 0.0)).pitch - 90.0).abs() < 1e-4);
    }
}
