//! Timings a server or an item can change: a prior, narrowed by what the bot observes.

/// Packet arrival vs server tick: an observed tick count can be one off either way.
const SLACK: u32 = 1;

// Spec: specs/F5-version-layer.md §4.6 (server-changeable timings are beliefs; err long, never short)
/// The raise-to-block delay `d` of a `blocks_attacks` item, in server ticks: a hit `k` server
/// ticks into the use is blocked when `k >= d` (vanilla `LivingEntity#getItemBlockingWith`).
/// The prior is the held item's `block_delay_seconds` (vanilla `BlocksAttacks#blockDelayTicks`).
///
/// A belief shorter than the real delay is unsafe (the bot expects a block, the shield is not up
/// yet). The observed `k` can be off by a tick or more (the fork counts its own ticks), so
/// `ticks()` is never below the prior and never below what unblocked hits prove; blocked hits
/// only narrow `bounds()`, and a conflict keeps the longer estimate (Allen, 2026-10-23).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockDelayBelief {
    prior: u32,
    /// `d` is at least this.
    lo: u32,
    /// `d` is at most this, once a blocked hit was seen.
    hi: Option<u32>,
    observed: u32,
    /// Hits that contradicted an earlier hit of the other kind (diagnostics).
    conflicts: u32,
}

impl BlockDelayBelief {
    pub fn new(prior: u32) -> Self {
        Self { prior, lo: 0, hi: None, observed: 0, conflicts: 0 }
    }

    pub fn prior(&self) -> u32 {
        self.prior
    }

    /// A hit that this item blocks once raised (from the front, a blockable damage type),
    /// `k` server ticks into the use.
    ///
    /// A conflict (an unblocked hit later in the use than a blocked one) keeps the longer
    /// estimate: either hit's tick count can be off, and a belief may err long, never short.
    pub fn observe(&mut self, k: u32, blocked: bool) {
        self.observed += 1;
        if blocked {
            let hi = k + SLACK;
            // `lo` carries the unblocked margin (`SLACK + 1`); compare the raw reading, so only
            // a real contradiction counts.
            if hi < self.lo.saturating_sub(SLACK + 1) {
                self.conflicts += 1;
            }
            self.hi = Some(self.hi.map_or(hi, |h| h.min(hi)));
        } else {
            // Conflict on the raw reading (`d >= k` at the least), without the `lo` margin.
            if self.hi.is_some_and(|h| k > h) {
                self.conflicts += 1;
            }
            // The true count is at most `k + SLACK` and the delay is above it: the strongest
            // floor the reading can mean, so a low reading never leaves `lo` short.
            self.lo = self.lo.max(k + SLACK + 1);
        }
    }

    /// Safe estimate: the larger of the prior and what unblocked hits prove. An unblocked hit read
    /// at `k` means the delay is above the true tick count, which is at most `k + SLACK`, so `lo` is
    /// `k + SLACK + 1` and can only make this longer (the allowed direction). `hi` never lowers it.
    pub fn ticks(&self) -> u32 {
        self.prior.max(self.lo)
    }

    /// What the observations allow: `d` in `lo..=hi` (`hi` None: no blocked hit yet). For
    /// diagnostics: `ticks()` does not follow `hi`.
    pub fn bounds(&self) -> (u32, Option<u32>) {
        (self.lo, self.hi)
    }

    pub fn observed(&self) -> u32 {
        self.observed
    }

    pub fn conflicts(&self) -> u32 {
        self.conflicts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prior_stands_when_hits_agree() {
        let mut b = BlockDelayBelief::new(5);
        b.observe(3, false);
        b.observe(7, true);
        assert_eq!((b.ticks(), b.bounds()), (5, (5, Some(8))));
    }

    #[test]
    fn hits_through_after_the_prior_raise_the_estimate() {
        // The item says 5; the server lets hits through until 19: a different delay.
        let mut b = BlockDelayBelief::new(5);
        b.observe(12, false);
        assert_eq!(b.ticks(), 14);
        b.observe(19, false);
        b.observe(21, true);
        assert_eq!((b.ticks(), b.bounds()), (21, (21, Some(22))));
    }

    #[test]
    fn a_block_before_the_prior_does_not_lower_the_estimate() {
        let mut b = BlockDelayBelief::new(20);
        b.observe(6, true);
        assert_eq!((b.ticks(), b.bounds()), (20, (0, Some(7))));
    }

    #[test]
    fn a_low_tick_count_on_a_loaded_host_keeps_the_prior() {
        // Pi case: the real delay is 5; the count reads 3 at a blocked hit.
        let mut b = BlockDelayBelief::new(5);
        b.observe(3, true);
        assert_eq!((b.ticks(), b.bounds()), (5, (0, Some(4))));
    }

    #[test]
    fn an_unblocked_hit_read_one_tick_low_is_not_short() {
        // Real delay 10; the hit came at true tick 9 but read 8. The estimate stays >= 10.
        let mut b = BlockDelayBelief::new(5);
        b.observe(8, false);
        assert!(b.ticks() >= 10, "{}", b.ticks());
    }

    #[test]
    fn a_conflict_keeps_the_longer_estimate() {
        let mut b = BlockDelayBelief::new(5);
        b.observe(30, false); // something we can't see let it through
        b.observe(6, true);
        assert_eq!((b.ticks(), b.bounds(), b.conflicts()), (32, (32, Some(7)), 1));
        b.observe(40, false);
        assert_eq!((b.conflicts(), b.ticks()), (2, 42));
    }
}
