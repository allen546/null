use crate::types::*;

/// Intent-shaped output. Walk/strafe are held until changed; `stop` clears all of them.
pub trait BotCommands {
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
    /// Path to within `tolerance` blocks of `target` (0 = stand on that block). Asynchronous:
    /// read progress from `BotView::nav_status`. A new goto replaces the current one.
    fn goto(&mut self, target: BlockPos, tolerance: f32);
    /// Cancel navigation only; held walk/sprint input is left alone.
    fn stop_nav(&mut self);
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
    Goto { target: BlockPos, tolerance: f32 },
    StopNav,
}
