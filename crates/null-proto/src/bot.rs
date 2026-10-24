//! Bot messages (squad <-> bot). Spec: specs/K6-squad-leader.md §3
// Spec: specs/K6-squad-leader.md §3

use crate::{BotId, ClaimId, Gt, ItemId, JobId, Loc, Outcome};
use serde::{Deserialize, Serialize};

// Spec: specs/K6-squad-leader.md §3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Life {
    Alive,
    Dead,
    Disconnected,
}

// Spec: specs/K6-squad-leader.md §3
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BotState {
    pub bot: BotId,
    pub life: Life,
    pub at_loc: Option<Loc>,
    pub health: f32,
    pub job: Option<JobId>,
    pub tick: Gt,
}

// Spec: specs/K6-squad-leader.md §3
/// What a bot saw.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Observed {
    Block { at: Loc, block: ItemId },
    Entity { id: u64, kind: String, at: Loc },
}

/// Every observation says who saw it and when (CONVENTIONS §3.4).
// Spec: specs/K6-squad-leader.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Observation {
    pub observer: BotId,
    pub tick: Gt,
    pub what: Observed,
}

// Spec: specs/K6-squad-leader.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobOutcome {
    pub bot: BotId,
    pub outcome: Outcome,
    pub tick: Gt,
    /// The `Job::attempt` this outcome answers.
    #[serde(default)]
    pub attempt: u32,
}

/// The squad renews a job's claim and tells the bot how long it holds. A bot stops its job by itself when
/// `expires_at` is near and no newer lease came (lease rule, K6 §4.8, K5 §4.6).
// Spec: specs/K6-squad-leader.md §4.8
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lease {
    pub bot: BotId,
    pub job: JobId,
    pub attempt: u32,
    pub claim: ClaimId,
    pub expires_at: Gt,
}
