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
    // Spec: specs/B3-inventory-crafting.md §4.1 (slot management: read all 36 slots)
    /// Main inventory then hotbar as the server numbers them (`Inventory` NBT): 0-8 hotbar,
    /// 9-35 main. Armour and offhand are not included.
    fn inventory(&self) -> &[Option<ItemStack>; 36];
    fn selected_slot(&self) -> u8;
    fn offhand(&self) -> Option<&ItemStack>;
    fn using_item(&self) -> Option<UsingItem>;
    fn entities(&self) -> &[EntitySnapshot];
    fn recent_attackers(&self) -> &[Attacker];
    // Spec: specs/F5-version-layer.md §4.6 (observations for the shield-delay belief)
    /// Hits taken in the last 10 s while a blocking item was in use, oldest first.
    fn shield_hits(&self) -> &[ShieldHit] {
        &[]
    }
    fn is_solid(&self, pos: BlockPos) -> bool;
    fn nav_status(&self) -> NavStatus;
    // Spec: specs/B6-placement-breaking.md §4.6 (read back the block the server left)
    /// Namespaced block name at `pos` ("minecraft:oak_log"); None when the chunk is not loaded.
    fn block_at(&self, pos: BlockPos) -> Option<String>;
    // Spec: specs/B6-placement-breaking.md §4.4, §4.6 (latest break and placement results)
    /// The latest break (B6 §4.4) and the latest placement (B6 §4.6), until replaced by a newer one.
    fn last_break(&self) -> Option<BlockAction>;
    fn last_place(&self) -> Option<BlockAction>;

    // Spec: specs/B3-inventory-crafting.md §4.1 (slot management: count items)
    /// Total count of `item` ("minecraft:stone") over the 36 main slots and the offhand.
    fn count_item(&self, item: &str) -> u32 {
        let main: u32 = self.inventory().iter().flatten().filter(|s| s.item == item).map(|s| s.count as u32).sum();
        main + self.offhand().filter(|s| s.item == item).map_or(0, |s| s.count as u32)
    }
    fn held_item(&self) -> Option<&ItemStack> {
        self.hotbar()[self.selected_slot() as usize % 9].as_ref()
    }
    fn entity(&self, id: EntityId) -> Option<&EntitySnapshot> {
        self.entities().iter().find(|e| e.id == id)
    }
}
