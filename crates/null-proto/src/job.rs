//! Job contracts and outcomes (CONVENTIONS §3.1, §4.4).
// Spec: specs/CONVENTIONS.md §3.1

use crate::{ClaimId, Gt, ItemId, JobId, Loc, Priority, Region};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemCount {
    pub item: ItemId,
    pub n: u32,
}

// Spec: specs/CONVENTIONS.md §3.1
/// A job parameter. Durations are typed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParamValue {
    Int(i64),
    Text(String),
    Item(ItemId),
    Loc(Loc),
    Locs(Vec<Loc>),
    /// A box the job may touch (the claim a bot hands its skill).
    Region(Region),
    Ticks(Gt),
}

/// A test the bot can check from what it sees. Used for preconditions and the success test.
// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Condition {
    Always,
    HaveItems { item: ItemId, n: u32 },
    BlockAbsent { at: Loc },
    All(Vec<Condition>),
}

/// Budget for in-game work, so it is in game ticks.
// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Budget {
    pub ticks: Gt,
    pub attempts: u32,
}

// Spec: specs/CONVENTIONS.md §4.4
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflexGroup {
    Movement,
    Survival,
    Combat,
    Mob,
}

// Spec: specs/CONVENTIONS.md §4.4
/// Mirror of `null-bot-api::reflex::Target` (that crate has no serde).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReflexTarget {
    All,
    Group(ReflexGroup),
    Reflex(String),
}

// Spec: specs/CONVENTIONS.md §4.4
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReflexSetting {
    pub target: ReflexTarget,
    pub on: bool,
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Checkpoint {
    None,
    Resumable,
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Job {
    pub id: JobId,
    /// Skill name, e.g. "fell".
    pub kind: String,
    pub params: BTreeMap<String, ParamValue>,
    pub items: Vec<ItemCount>,
    /// Claims the job runs under; the scheduler takes them before it hands the job out.
    pub claims: Vec<ClaimId>,
    pub preconditions: Vec<Condition>,
    pub success: Condition,
    pub budget: Budget,
    pub priority: Priority,
    /// Reflex settings applied while the job runs (CONVENTIONS §4.4).
    pub reflexes: Vec<ReflexSetting>,
    pub checkpoint: Checkpoint,
    /// Minimum crew that cannot be preempted (cargo jobs only).
    pub preemption_floor: Option<u32>,
    /// Which dispatch of this job this is (1, 2, ...); 0 means unnumbered. The bot echoes it in its outcome so
    /// the operation can tell a late report of an old attempt from the current one (K5 §4.6).
    #[serde(default)]
    pub attempt: u32,
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailReason {
    NoPath,
    MissingTool,
    TargetGone,
    BudgetExhausted,
    BotDied,
    Other(String),
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockedOn {
    Claim(ClaimId),
    Items(Vec<ItemCount>),
    /// A reflex policy conflicts with the job (CONVENTIONS §4.4), e.g. "upkeep.eat off".
    ReflexPolicy(String),
    Other(String),
}

// Spec: specs/CONVENTIONS.md §3.1
/// The world did something the job did not expect; K5 places the job elsewhere.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Incident {
    pub kind: String,
    pub at: Option<Loc>,
    pub detail: String,
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutcomeKind {
    Succeeded,
    Failed(FailReason),
    Blocked(BlockedOn),
    Preempted,
    Interrupted(Incident),
}

/// What happened, in fields the ledger can count. Every outcome carries one.
// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Report {
    pub elapsed: Gt,
    /// Items gained (positive for the ledger; losses go in `items_lost`).
    pub items_gained: Vec<ItemCount>,
    pub items_lost: Vec<ItemCount>,
    pub at: Option<Loc>,
    pub note: String,
}

// Spec: specs/CONVENTIONS.md §3.1
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Outcome {
    pub job: JobId,
    pub kind: OutcomeKind,
    pub report: Report,
}
