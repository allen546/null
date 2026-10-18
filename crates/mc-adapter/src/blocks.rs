//! Translation between `null_bot_api` block actions and the fork's block-action API
//! (`azalea_client::block_actions`, AZALEA_PATCHES row 5). No game logic: the fork places, breaks
//! and judges the server's answer; this only maps types and remembers which result is ours.

use azalea::{
    BlockPos as ABlockPos,
    block_actions::{BlockActionOutcome, BlockActionResult, BlockActionResults},
    core::direction::Direction,
};
use azalea::block_actions::{UseBlockOutcome, UseBlockResult};
use null_bot_api::{OpenOutcome, OpenResult, BlockAction, BlockActionKind, BlockOutcome, BlockPos, Face};

/// A command we issued, kept so a result left over from an earlier action is not mistaken for it.
#[derive(Clone, Copy)]
pub struct Issued {
    pub tick: u64,
    pub target: BlockPos,
    /// The id of the fork's result at the time of the command; ours has a different one.
    prev_id: u32,
}

impl Issued {
    pub fn new_open(tick: u64, target: BlockPos, prev_id: Option<u32>) -> Self {
        Self { tick, target, prev_id: prev_id.unwrap_or(0) }
    }
    pub fn new(tick: u64, target: BlockPos, prev: Option<&BlockActionResult>) -> Self {
        Self { tick, target, prev_id: prev.map_or(0, |r| r.id) }
    }
}

// Spec: specs/B6-placement-breaking.md §4.4 (one result per break; a repeated break command keeps it)
/// A repeated break of the block already being broken: its result is still open (or not opened
/// yet), so the earlier command's record stays.
pub fn same_break_pending(issued: Option<Issued>, pos: BlockPos, cur: Option<&BlockActionResult>) -> bool {
    let Some(i) = issued.filter(|i| i.target == pos) else { return false };
    match cur {
        Some(r) if r.id != i.prev_id => r.outcome == BlockActionOutcome::Pending,
        _ => true,
    }
}

/// The host's record of the last break and place commands.
#[derive(Default)]
pub struct BlockActions {
    pub last_break: Option<Issued>,
    pub last_place: Option<Issued>,
    pub last_open: Option<Issued>,
}

// Spec: specs/B6-placement-breaking.md §4.2 (click face as the fork's Direction)
pub fn face_to_dir(f: Face) -> Direction {
    match f {
        Face::Down => Direction::Down,
        Face::Up => Direction::Up,
        Face::North => Direction::North,
        Face::South => Direction::South,
        Face::West => Direction::West,
        Face::East => Direction::East,
    }
}

// Spec: specs/B6-placement-breaking.md §4.2 (api block position as the fork's)
pub fn block_pos(p: BlockPos) -> ABlockPos {
    ABlockPos::new(p.x, p.y, p.z)
}

// Spec: specs/B6-placement-breaking.md §4.6 (read back the block the server left)
pub fn block_name(w: &azalea::world::World, pos: BlockPos) -> Option<String> {
    use azalea::registry::builtin::BlockKind;
    let st = w.chunks.get_block_state(block_pos(pos))?;
    Some(BlockKind::from(st).to_str().to_string())
}

// Spec: specs/B6-placement-breaking.md §4.6 (acknowledgement and readback, as the api's BlockAction)
/// The api's view of our latest action of `kind`; Pending until the fork opens its result.
pub fn translate(
    kind: BlockActionKind,
    issued: Option<Issued>,
    results: Option<&BlockActionResults>,
) -> Option<BlockAction> {
    let i = issued?;
    let r = results.and_then(|r| match kind {
        BlockActionKind::Break => r.last_break,
        BlockActionKind::Place => r.last_place,
    });
    let mut a = BlockAction {
        kind,
        pos: i.target,
        outcome: BlockOutcome::Pending,
        start_tick: i.tick,
        finish_tick: None,
        ack_tick: None,
        reason: None,
    };
    if let Some(r) = r.filter(|r| r.id != i.prev_id) {
        a.outcome = match r.outcome {
            BlockActionOutcome::Pending => BlockOutcome::Pending,
            BlockActionOutcome::Accepted => BlockOutcome::Done,
            BlockActionOutcome::Refused => BlockOutcome::Refused,
            BlockActionOutcome::Cancelled => BlockOutcome::Cancelled,
        };
        a.reason = r.refusal.map(|w| w.as_str());
        // The fork knows the real target (a replaceable clicked block is placed into).
        a.pos = BlockPos::new(r.position.x, r.position.y, r.position.z);
        a.finish_tick = r.finish_ticks.map(|t| i.tick + t as u64);
        // For a break the fork counts the ack from the finishing packet.
        a.ack_tick = r.ack_ticks.map(|t| i.tick + r.finish_ticks.unwrap_or(0) as u64 + t as u64);
    }
    Some(a)
}

// Spec: specs/B3-inventory-crafting.md §4.7 (the latest container open, as the api's OpenResult)
/// The api's view of our latest `open_container`; Pending until the fork opens its result.
pub fn translate_open(issued: Option<Issued>, results: Option<&BlockActionResults>) -> Option<OpenResult> {
    let i = issued?;
    let mut o = OpenResult { target: null_bot_api::OpenTarget::Block(i.target), outcome: OpenOutcome::Pending, refused: None, ticks: 0 };
    if let Some(r) = results.and_then(|r| r.last_use).filter(|r: &UseBlockResult| r.id != i.prev_id) {
        o.outcome = match r.outcome {
            UseBlockOutcome::Pending => OpenOutcome::Pending,
            UseBlockOutcome::Opened { window_id, menu } => OpenOutcome::Opened { window_id, kind: menu.to_str() },
            UseBlockOutcome::NoScreen => OpenOutcome::NoScreen,
            UseBlockOutcome::Refused => OpenOutcome::Refused,
        };
        o.refused = r.refusal.map(|w| w.as_str());
        o.ticks = r.elapsed_ticks;
    }
    Some(o)
}
