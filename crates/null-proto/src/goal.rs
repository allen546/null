//! Goals (ARCHITECTURE §3.6).
// Spec: specs/ARCHITECTURE.md §3.6

use crate::{GoalId, ItemId, Loc, Region};
use serde::{Deserialize, Serialize};

// Spec: specs/ARCHITECTURE.md §3.6
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Goal {
    Obtain { item: ItemId, n: u32 },
    Build { schematic: String, site: Loc },
    Defend { region: Region },
    KillDragon,
    Explore { area: Region },
    Hold { position: Loc },
}

/// A local error, not a wire message, so it has no serde derives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unsupported {
    pub kind: &'static str,
}

impl Goal {
    pub fn kind(&self) -> &'static str {
        match self {
            Goal::Obtain { .. } => "obtain",
            Goal::Build { .. } => "build",
            Goal::Defend { .. } => "defend",
            Goal::KillDragon => "kill_dragon",
            Goal::Explore { .. } => "explore",
            Goal::Hold { .. } => "hold",
        }
    }
    /// The skeleton runs `Obtain` only; HQ rejects the rest with this error.
    pub fn check_supported(&self) -> Result<(), Unsupported> {
        match self {
            Goal::Obtain { .. } => Ok(()),
            other => Err(Unsupported { kind: other.kind() }),
        }
    }
}

// Spec: specs/ARCHITECTURE.md §3.6
/// A goal as set by the commander (K2 `set_goal`). Higher priority wins when bots are short.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalSpec {
    pub id: GoalId,
    pub goal: Goal,
    pub priority: Priority,
    /// K2 §4.3: every write states a reason.
    pub reason: String,
}

// Spec: specs/ARCHITECTURE.md §3.6
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Priority(pub u8);
