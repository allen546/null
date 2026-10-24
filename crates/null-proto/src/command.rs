//! Commands from the commander (K2 §3) and the intent/summary bodies the bus carries.
// Spec: specs/K2-commander-api.md §3

use crate::*;
use serde::{Deserialize, Serialize};

// Spec: specs/K2-commander-api.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Scope {
    Swarm,
    Server(ServerId),
    Operation(OperationId),
    Squad(SquadId),
    Bot(BotId),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InspectTarget {
    Goal(GoalId),
    Scope(Scope),
}

// Spec: specs/K2-commander-api.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Command {
    Status,
    Inspect { target: InspectTarget },
    SetGoal(GoalSpec),
    CancelGoal { id: GoalId, reason: String },
    /// Halt and recall are never gated (K2 §4.9). They use the command class.
    Halt { scope: Scope },
    Recall { scope: Scope },
    SetReflexes { scope: Scope, settings: Vec<ReflexSetting>, reason: String },
}

impl Command {
    /// Urgent commands go on the command class and are never queued behind other traffic (F6 §4.3).
    pub fn is_urgent(&self) -> bool {
        matches!(self, Command::Halt { .. } | Command::Recall { .. })
    }
}

// Spec: specs/K2-commander-api.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Quota {
    pub item: ItemId,
    pub n: u32,
}

/// Bots HQ gives an operation.
// Spec: specs/K2-commander-api.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allocation {
    pub op: OperationId,
    pub bots: Vec<BotId>,
}

/// Payload of the intent class (at least once; the bus dedupes by the key in [`Intent`]).
// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum IntentBody {
    /// HQ gives an operation its sub-goal.
    Goal { op: OperationId, region: Option<Region>, goal: GoalSpec, quotas: Vec<Quota> },
    Allocation(Allocation),
    JobBatch { squad: SquadId, jobs: Vec<Job> },
    /// A commander write other than halt/recall.
    Command(Command),
    JobOutcome(JobOutcome),
    Observation(Observation),
}

// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Intent {
    /// Idempotency key: receivers apply a key once.
    pub key: String,
    pub body: IntentBody,
}

/// Payload of the summary class (latest wins).
// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SummaryBody {
    Operation(OpSummary),
    Digest(Digest),
    Bot(BotState),
}

/// One line in a shard's append-only log. `data` is JSON text so the log format does not change with the types.
// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogEvent {
    pub at: WallMs,
    pub kind: String,
    pub data: String,
}
