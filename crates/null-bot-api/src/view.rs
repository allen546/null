use crate::action::{ActionId, ActionState};
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
    // Spec: specs/B4-gathering.md §4.8 (natural check: leaves' `persistent` property)
    /// One block-state property of the block at `pos` as text (`name` "persistent" -> "false").
    /// None when the block lacks it, is unknown or is not loaded.
    fn block_property(&self, _pos: BlockPos, _name: &str) -> Option<String> {
        None
    }
    // Spec: specs/F5-version-layer.md §4.6 (inputs the client does not model)
    /// Attributes and status effects the server has set that change what a vanilla client
    /// computes for itself but that Null does not model yet, such as `scale` or levitation.
    /// While this is not empty the host ends the running skill and sends nothing that moves or
    /// breaks. Empty when the client's predictions hold.
    fn unmodelled_inputs(&self) -> Vec<UnmodelledInput> {
        Vec::new()
    }
    // Spec: specs/F5-version-layer.md §4.6 (the disconnect for a Movement entry)
    /// Set when the fork closed the connection for a `Movement` entry (AD-F5.5). The host also
    /// reports it as an event; the bot stays off.
    fn unmodelled_disconnect(&self) -> Option<UnmodelledDisconnect> {
        None
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (crouch handling)
    /// The bot is crouching (the crouch input it sends).
    fn sneaking(&self) -> bool {
        false
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (crouch handling: does a click on this block need a crouch)
    /// Whether a right-click on the block at `pos` runs the block's own action, so a placing click
    /// on it needs a crouch (generated per version from the game's sources). False when unknown.
    fn block_uses_click(&self, _pos: BlockPos) -> bool {
        false
    }
    // Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox)
    /// The state of a command's result: pending (with the partial record), finished, or expired
    /// (released, pruned or never issued; never reads as pending). The same id reads the same
    /// until it finishes; a finished result stays until its owner releases it (B6 §4.6).
    fn action(&self, id: ActionId) -> ActionState;

    // Spec: specs/B3-inventory-crafting.md §4.1 (the open window)
    /// The open window, if any.
    fn window(&self) -> Option<&Window> {
        None
    }

    // Spec: specs/F1-swarm-runtime.md §4.4 (death and respawn, as the bot knows them)
    fn life(&self) -> Life {
        Life::Alive
    }
    /// The latest death and how many there were (B2 §4.7), with the respawn that ended it. Kept
    /// across respawns.
    fn last_death(&self) -> Option<&Death> {
        None
    }
    fn deaths(&self) -> u32 {
        0
    }
    // Spec: specs/F1-swarm-runtime.md §4.4 (nothing while dead)
    /// What the fork refused since the latest death began (the death screen has no keys or mouse).
    fn refused_while_dead(&self) -> Vec<DeadRefusal> {
        Vec::new()
    }
    /// Game packets the client sent since the latest death began, by packet name, with counts.
    fn sent_while_dead(&self) -> Vec<(String, u32)> {
        Vec::new()
    }

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
