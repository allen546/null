//! Upward messages: summary, exceptions, escalations, digest (K5 §3, K1, F6 §3).
// Spec: specs/K5-operation-service.md §3

use crate::{BotId, GoalId, Gt, ItemCount, OperationId, ServerId, SquadId};
use serde::{Deserialize, Serialize};

// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpState {
    Planning,
    Running,
    Blocked,
    Done,
    Failed,
    Halted,
}

// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Headcount {
    pub assigned: u32,
    pub alive: u32,
}

// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Progress {
    pub done: u32,
    pub target: u32,
}

// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Health {
    Ok,
    Degraded,
    Down,
}

/// Same kind and subject within a window merge into one entry with a count.
// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Exception {
    pub kind: String,
    pub subject: String,
    pub count: u32,
    pub first_seen: Gt,
    pub last_seen: Gt,
}

/// Most exceptions an `OpSummary` carries, so the summary stays fixed-size (K5 §3).
pub const MAX_EXCEPTIONS: usize = 8;

/// Add an occurrence to a list, merging by (kind, subject). A full list drops the entry with the oldest `last_seen`.
// Spec: specs/K5-operation-service.md §3
pub fn merge_exception(list: &mut Vec<Exception>, kind: &str, subject: &str, at: Gt) {
    match list.iter_mut().find(|e| e.kind == kind && e.subject == subject) {
        Some(e) => {
            e.count += 1;
            e.last_seen = at;
        }
        None => {
            if list.len() >= MAX_EXCEPTIONS {
                if let Some(oldest) = (0..list.len()).min_by_key(|&i| list[i].last_seen) {
                    list.remove(oldest);
                }
            }
            list.push(Exception { kind: kind.into(), subject: subject.into(), count: 1, first_seen: at, last_seen: at });
        }
    }
}

/// Operation to HQ. Fixed fields, so its size does not grow with bot count. Latest wins.
// Spec: specs/K5-operation-service.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpSummary {
    pub op: OperationId,
    pub server: ServerId,
    pub goal: GoalId,
    pub state: OpState,
    pub headcount: Headcount,
    pub progress: Progress,
    pub stock: Vec<ItemCount>,
    pub health: Health,
    pub top_issue: Option<String>,
    pub exceptions: Vec<Exception>,
    pub at: Gt,
}

// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Remedy {
    pub action: String,
    pub result: String,
}

/// Needs a decision from above. `key` names the problem, so repeats collapse (F6 §3).
// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Escalation {
    pub key: String,
    pub from_op: OperationId,
    pub from_squad: Option<SquadId>,
    pub from_bot: Option<BotId>,
    pub kind: String,
    pub detail: String,
    pub remedies_tried: Vec<Remedy>,
    pub at: Gt,
}

/// One row per operation (K1).
// Spec: specs/K1-hq.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DigestRow {
    pub op: OperationId,
    pub server: ServerId,
    pub goal: GoalId,
    pub state: OpState,
    pub headcount: Headcount,
    pub progress: Progress,
    pub top_issue: Option<String>,
    /// True when the latest summary is overdue (K1 failure table).
    pub stale: bool,
}

// Spec: specs/K1-hq.md §3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Digest {
    /// Grows with each digest, so a write based on an old one is detectable (K2 §4.2).
    pub generation: u64,
    pub at: Gt,
    pub rows: Vec<DigestRow>,
    /// One line each, e.g. survival reflexes off somewhere (K2 §4.5).
    pub warnings: Vec<String>,
}
