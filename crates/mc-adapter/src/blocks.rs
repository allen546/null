//! Translation between `null_bot_api` block types and the fork's (`azalea_client::block_actions`,
//! AZALEA_PATCHES row 5): positions and faces, and the block name at a position. No game logic.

use azalea::{BlockPos as ABlockPos, core::direction::Direction};
use null_bot_api::{BlockPos, Face};

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
