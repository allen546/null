use crate::action::{ActionId, Owner};
use crate::types::*;

/// Intent-shaped output. Walk/strafe are held until changed; `stop` clears all of them.
///
/// Commands that have a result (`break_block`, `place_block`, `open_*`, `click_slot`, `respawn`)
/// return an [`ActionId`]; read it with `BotView::action(id)` as often as needed, and `release`
/// it when done. Waiting for the result before the next command is the skill's choice.
pub trait BotCommands {
    // Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox: owners)
    /// The host calls this before each skill step or reflex act, so every command is stamped with
    /// who sent it. Skills and reflexes never call it.
    fn set_owner(&mut self, _owner: Owner) {}
    // Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox: release)
    /// Let go of a result of this owner. A finished one is dropped (at the end of the tick); a
    /// pending one is dropped when it finishes. Ignored for another owner's id.
    fn release(&mut self, _id: ActionId) {}
    // Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox: expected count)
    /// How many finished results this owner expects to hold at once (default 64). Going over
    /// logs one warning; nothing is pruned.
    fn expect_results(&mut self, _n: usize) {}
    /// `forward` and `strafe` are each -1, 0 or 1 (forward +1 = ahead, strafe +1 = right).
    fn walk(&mut self, forward: i8, strafe: i8);
    fn sprint(&mut self, on: bool);
    fn jump(&mut self);
    fn sneak(&mut self, on: bool);
    fn look(&mut self, yaw: f32, pitch: f32);
    fn select_slot(&mut self, slot: u8);
    fn attack(&mut self, target: EntityId);
    fn use_item(&mut self, hand: Hand);
    fn release_use(&mut self);
    fn swap_offhand(&mut self);
    fn stop(&mut self);
    // Spec: specs/B6-placement-breaking.md §4.4 (break timing)
    /// Hold the break on the block at `pos` until it breaks (B6 §4.4). Look at the block first and
    /// leave a tick for the look to land; read progress from `BotView::action` (B6 §4.6).
    fn break_block(&mut self, pos: BlockPos) -> ActionId;
    // Spec: specs/B6-placement-breaking.md §4.4 (abort a break)
    /// Abort the break in progress.
    fn cancel_break(&mut self);
    // Spec: specs/B6-placement-breaking.md §4.2 (place on a chosen face)
    /// Right-click the `face` of the block at `pos` with the held item (B6 §4.2). The new block
    /// would be `pos.offset(face)`; read the outcome from `BotView::action`. A block that uses
    /// the click (`BotView::block_uses_click`) takes a placing click only while crouching. If the
    /// bot stands: with `auto_crouch` off the click is refused with a reason and nothing is sent;
    /// with it on the bot crouches, clicks on the next tick and stands again (for one-off
    /// placements; a sequence holds the crouch with `sneak`).
    fn place_block(&mut self, pos: BlockPos, face: Face, auto_crouch: bool) -> ActionId;
    // Spec: specs/B6-placement-breaking.md §4.2 (click choice: aim at a visible point)
    /// Turn to a visible point of the block's real shape on `face` (any face when `None`), so the
    /// crosshair is on it (B6 §4.2). Leave two ticks before the click; if no such point is visible
    /// the look does not change and the click is refused.
    fn look_at_block(&mut self, pos: BlockPos, face: Option<Face>);
    /// Path to within `tolerance` blocks of `target` (0 = stand on that block). Asynchronous:
    /// read progress from `BotView::nav_status`. A new goto replaces the current one.
    fn goto(&mut self, target: BlockPos, tolerance: f32);
    /// Cancel navigation only; held walk/sprint input is left alone.
    fn stop_nav(&mut self);
    // Spec: specs/B3-inventory-crafting.md §4.1 (open the inventory as a vanilla client does)
    /// Open the player's inventory (vanilla's E). While a window is open the bot holds still;
    /// refused while mining or using an item (the result says why). Click from the next tick.
    fn open_inventory(&mut self) -> ActionId;
    // Spec: specs/B3-inventory-crafting.md §4.7 (open a container by right-clicking it)
    /// Right-click the block at `pos` (on `face`, or any visible face) with the main hand, as a
    /// vanilla client does. Look at it first (`look_at_block`, two ticks). Refused with a reason,
    /// sending nothing, when out of reach, crouching with an item in hand, or a screen is open.
    /// Read the result with `BotView::action`.
    fn open_container(&mut self, pos: BlockPos, face: Option<Face>) -> ActionId;
    // Spec: specs/B3-inventory-crafting.md §4.7 (aim at a point of an entity's hitbox)
    /// Turn to a point of the entity's real hitbox where the crosshair hits it (blocks in the way
    /// and the entity reach included). Leave two ticks before the click; if no such point is
    /// reachable the look does not change and the click is refused.
    fn look_at_entity(&mut self, target: EntityId);
    // Spec: specs/B3-inventory-crafting.md §4.7 (open a container on an entity by right-clicking it)
    /// Right-click a chest minecart, hopper minecart or chest boat with the main hand, as a
    /// vanilla client does. Look at it first (`look_at_entity`, two ticks). Refused with a reason,
    /// sending nothing, when out of reach (measured to the hitbox), behind a block, not under the
    /// crosshair, or a screen is open. A chest boat opens only for a crouching player (a standing
    /// click would ride it, which Null does not do): standing, it is refused unless `auto_crouch`
    /// is set; then the bot crouches, clicks one tick later and stands again. Read the result with
    /// `BotView::action`.
    fn open_entity(&mut self, target: EntityId, auto_crouch: bool) -> ActionId;
    // Spec: specs/B3-inventory-crafting.md §4.1 (close the open window)
    fn close_window(&mut self);
    // Spec: specs/B3-inventory-crafting.md §4.1 (click a slot of the open window)
    /// Click `slot` of the open window (`None` = outside it, for dropping the carried stack).
    /// Read the result with `BotView::action`; it finishes after the server's corrections had time
    /// to arrive (B3 §4.1).
    fn click_slot(&mut self, slot: Option<u16>, click: Click) -> ActionId;
    // Spec: specs/F1-swarm-runtime.md §4.4 (respawn like the death screen's button)
    /// Ask to respawn. Held until the death screen's button is active (20 ticks after it opened),
    /// then sent. Refused with a reason, sending nothing, when not dead or in hardcore
    /// (the result says why).
    fn respawn(&mut self) -> ActionId;
}

/// One command as data; the fake records these and adapters may queue them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Walk { forward: i8, strafe: i8 },
    Sprint(bool),
    Jump,
    Sneak(bool),
    Look { yaw: f32, pitch: f32 },
    SelectSlot(u8),
    Attack(EntityId),
    UseItem(Hand),
    ReleaseUse,
    SwapOffhand,
    Stop,
    BreakBlock(BlockPos),
    CancelBreak,
    PlaceBlock { pos: BlockPos, face: Face, auto_crouch: bool },
    LookAtBlock { pos: BlockPos, face: Option<Face> },
    Goto { target: BlockPos, tolerance: f32 },
    StopNav,
    OpenInventory,
    OpenContainer { pos: BlockPos, face: Option<Face> },
    LookAtEntity(EntityId),
    OpenEntity { target: EntityId, auto_crouch: bool },
    CloseWindow,
    ClickSlot { slot: Option<u16>, click: Click },
    Respawn,
}
