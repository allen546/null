use crate::types::*;

/// Per-tick reads. The host owns the clock, so `tick` never depends on the game library.
pub trait BotView {
    fn tick(&self) -> u64;
    fn position(&self) -> Vec3;
    fn velocity(&self) -> Vec3;
    fn look(&self) -> Look;
    fn on_ground(&self) -> bool;
    fn in_water(&self) -> bool;
    fn in_lava(&self) -> bool;
    fn fall_distance(&self) -> f32;
    fn health(&self) -> f32;
    fn food(&self) -> u32;
    /// Attack cooldown progress, 0.0 (just swung) to 1.0 (full charge).
    fn attack_cooldown(&self) -> f32;
    fn hotbar(&self) -> &[Option<ItemStack>; 9];
    fn selected_slot(&self) -> u8;
    fn offhand(&self) -> Option<&ItemStack>;
    fn using_item(&self) -> Option<UsingItem>;
    fn entities(&self) -> &[EntitySnapshot];
    fn recent_attackers(&self) -> &[Attacker];
    fn is_solid(&self, pos: BlockPos) -> bool;
    fn nav_status(&self) -> NavStatus;

    fn held_item(&self) -> Option<&ItemStack> {
        self.hotbar()[self.selected_slot() as usize % 9].as_ref()
    }
    fn entity(&self, id: EntityId) -> Option<&EntitySnapshot> {
        self.entities().iter().find(|e| e.id == id)
    }
}
