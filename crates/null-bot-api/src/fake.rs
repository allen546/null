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
    pub selected_slot: u8,
    pub offhand: Option<ItemStack>,
    pub using_item: Option<UsingItem>,
    pub entities: Vec<EntitySnapshot>,
    pub attackers: Vec<Attacker>,
    /// Solid blocks. Everything with y < `floor_y` is solid too (flat world).
    pub solid: Vec<BlockPos>,
    pub floor_y: i32,
    pub nav_status: NavStatus,
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
            selected_slot: 0,
            offhand: None,
            using_item: None,
            entities: Vec::new(),
            attackers: Vec::new(),
            solid: Vec::new(),
            floor_y: 64,
            nav_status: NavStatus::Idle,
        }
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
    fn is_solid(&self, pos: BlockPos) -> bool {
        pos.y < self.floor_y || self.solid.contains(&pos)
    }
    fn nav_status(&self) -> NavStatus {
        self.nav_status
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
    fn goto(&mut self, target: BlockPos, tolerance: f32) {
        self.push(Command::Goto { target, tolerance });
    }
    fn stop_nav(&mut self) {
        self.push(Command::StopNav);
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
}
