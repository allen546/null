use crate::{DuelAction, DuelObs};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DoneReason {
    TargetDead,
    TargetLost,
    Retreat,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DuelStep {
    Act(DuelAction),
    Done(DoneReason),
}

/// Every technique, written or learned, sits behind this one seam (AD-B5.1).
pub trait DuelController {
    fn id(&self) -> &str;
    /// Clears all state; the same seed must give the same behaviour.
    fn reset(&mut self, seed: u64);
    fn step(&mut self, obs: &DuelObs) -> DuelStep;
}
