//! Claims (CONVENTIONS §3.3): request and reply types. The registry itself is S3's (`null-op`).
// Spec: specs/CONVENTIONS.md §3.3

use crate::{BotId, ClaimId, Gt, ItemId, JobId, Loc, OperationId, Priority, Region, SquadId};
use serde::{Deserialize, Serialize};

// Spec: specs/CONVENTIONS.md §3.3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimKind {
    Region(Region),
    BlockSet(Vec<Loc>),
    Container(Loc),
    Items { container: Loc, item: ItemId, n: u32 },
    Entity(u64),
    Bot(BotId),
}

// Spec: specs/CONVENTIONS.md §3.3
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ClaimOwner {
    Operation(OperationId),
    Squad(SquadId),
    Bot(BotId),
    Job(JobId),
}

/// Take, renew and release. `ttl` counts game ticks from the registry's clock: a claim frees itself.
// Spec: specs/CONVENTIONS.md §3.3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimRequest {
    Take { id: ClaimId, kind: ClaimKind, owner: ClaimOwner, priority: Priority, ttl: Gt },
    Renew { id: ClaimId, owner: ClaimOwner, ttl: Gt },
    Release { id: ClaimId, owner: ClaimOwner },
}

impl ClaimRequest {
    pub fn id(&self) -> &ClaimId {
        match self {
            ClaimRequest::Take { id, .. } | ClaimRequest::Renew { id, .. } | ClaimRequest::Release { id, .. } => id,
        }
    }
}

// Spec: specs/CONVENTIONS.md §3.3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenyReason {
    /// A holder of equal or higher priority has it.
    Held { by: ClaimOwner },
    /// Renew or release of a claim that is not held (never taken, or already expired).
    NotHeld,
    /// Renew or release by someone who is not the owner.
    NotOwner,
}

// Spec: specs/CONVENTIONS.md §3.3
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClaimReply {
    /// `token` grows with every grant so a cut-off holder cannot act on a lost claim (F6 §4.7).
    Granted { id: ClaimId, token: u64, expires_at: Gt },
    Denied { id: ClaimId, reason: DenyReason },
    /// The claim was taken away by a higher priority. Sent to the old owner, or as the reply to its renew.
    Preempt { id: ClaimId, by: ClaimOwner },
    Released { id: ClaimId },
}
