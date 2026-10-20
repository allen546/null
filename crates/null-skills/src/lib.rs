//! Null's first real skill, `gather` (B4 §3), and its tree policy `fell`. Written against
//! `BotView`/`BotCommands` and the belief store only, so it builds on stable.
//! A policy filters blocks (`policy`); the reach model and break-order search are in `reach`.

mod gather;
mod policy;
mod reach;
#[cfg(test)]
mod tests;

use std::sync::{Arc, Mutex};

use null_belief::Belief;

pub use gather::{Changed, Gather, GatherConfig, GatherResult, Left};
pub use policy::{Claim, Policy, Reason, TreePolicy, Verdict};
pub use gather::STEP_UNITS;
pub use reach::{LAYER_CAP, MARGIN, MAX_PAIR_UNITS, REACH, Task};

// Spec: specs/B4-gathering.md §3 (fell(tree_type, quota, replant) as a policy of gather)
/// Fell natural single-trunk trees of `tree_type` inside the claim, up to `quota` logs.
/// `replant` is refused for now: the job ends `Failed` before any break.
pub fn fell(tree_type: &str, quota: u32, replant: bool, claim: Claim, observer: &str, belief: Arc<Mutex<Belief>>) -> Gather {
    let g = Gather::new(GatherConfig { quota, claim, observer: observer.to_string() }, Box::new(TreePolicy::new(tree_type, claim)), belief);
    if replant { g.refuse("replant is not supported yet") } else { g }
}
