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

// Spec: specs/B6-placement-breaking.md §4.1 (refusal evidence: the face the ray hit)
pub fn dir_to_face(d: Direction) -> Face {
    match d {
        Direction::Down => Face::Down,
        Direction::Up => Face::Up,
        Direction::North => Face::North,
        Direction::South => Face::South,
        Direction::West => Face::West,
        Direction::East => Face::East,
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

// Spec: specs/B4-gathering.md §4.8 (natural check: a block-state property as text)
/// One property of a block state ("persistent" -> "false"); None when the block has no such property.
pub fn state_property(st: azalea::block::BlockState, name: &str) -> Option<String> {
    st.to_trait().get_property(name).map(str::to_string)
}

// Spec: specs/B4-gathering.md §4.8 (natural check: leaves' `persistent` property)
/// A property of the block at `pos`; None when the chunk is not loaded or the block lacks it.
pub fn block_property(w: &azalea::world::World, pos: BlockPos, name: &str) -> Option<String> {
    state_property(w.chunks.get_block_state(block_pos(pos))?, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use azalea::block::{BlockTrait, blocks::{OakLeaves, OakLog, Stone}, properties};

    #[test]
    fn leaves_persistent_reads_true_or_false_and_other_blocks_none() {
        let leaves = |persistent| OakLeaves { distance: properties::OakLeavesDistance::_1, persistent, waterlogged: false }.as_block_state();
        assert_eq!(state_property(leaves(true), "persistent").as_deref(), Some("true"));
        assert_eq!(state_property(leaves(false), "persistent").as_deref(), Some("false"));
        assert_eq!(state_property(leaves(false), "distance").as_deref(), Some("1"));
        assert_eq!(state_property(Stone {}.as_block_state(), "persistent"), None);
        assert_eq!(state_property(OakLog { axis: properties::Axis::Y }.as_block_state(), "persistent"), None);
    }
}
