//! HQ (K1, skeleton): goal tree, one operation per goal, headcount allocation, digest, halt and recall.
//! Decision crate: no azalea, no bevy. HQ never assigns a job to a bot (K1 §1).

mod hq;
#[cfg(test)]
mod tests;

pub use hq::{Hq, HqConfig, InspectReport, OpPlan, Reply, hq_digest};
