//! Location keys. Every one carries `server` because one HQ spans servers (AD-K1.1).
// Spec: specs/DECISIONS.md AD-K1.1

use crate::ServerId;
use serde::{Deserialize, Serialize};

// Spec: specs/DECISIONS.md AD-K1.1
/// Block position. `null-bot-api::BlockPos` has no serde, so the wire form is its own type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Pos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl Pos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }
}

// Spec: specs/DECISIONS.md AD-K1.1
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Dimension {
    Overworld,
    Nether,
    End,
    Other(String),
}

// Spec: specs/DECISIONS.md AD-K1.1
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Loc {
    pub server: ServerId,
    pub dim: Dimension,
    pub pos: Pos,
}

// Spec: specs/DECISIONS.md AD-K1.1
/// Box with inclusive corners.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Region {
    pub server: ServerId,
    pub dim: Dimension,
    pub min: Pos,
    pub max: Pos,
}

impl Region {
    pub fn contains(&self, l: &Loc) -> bool {
        l.server == self.server
            && l.dim == self.dim
            && (self.min.x..=self.max.x).contains(&l.pos.x)
            && (self.min.y..=self.max.y).contains(&l.pos.y)
            && (self.min.z..=self.max.z).contains(&l.pos.z)
    }
    /// True when the boxes share a block (same server and dimension only).
    pub fn overlaps(&self, o: &Region) -> bool {
        self.server == o.server
            && self.dim == o.dim
            && self.min.x <= o.max.x && o.min.x <= self.max.x
            && self.min.y <= o.max.y && o.min.y <= self.max.y
            && self.min.z <= o.max.z && o.min.z <= self.max.z
    }
}
