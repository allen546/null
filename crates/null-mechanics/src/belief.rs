//! Timings a server or an item can change: a prior, narrowed by what the bot observes.

/// Packet arrival vs server tick: an observed tick count can be one off either way.
const SLACK: u32 = 1;

// Spec: specs/F5-version-layer.md §4.6 (server-changeable timings are beliefs)
/// The raise-to-block delay `d` of a `blocks_attacks` item, in server ticks: a hit `k` server
/// ticks into the use is blocked when `k >= d` (vanilla `LivingEntity#getItemBlockingWith`).
/// The prior is the held item's `block_delay_seconds` (vanilla `BlocksAttacks#blockDelayTicks`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlockDelayBelief {
    prior: u32,
    /// `d` is at least this.
    lo: u32,
    /// `d` is at most this, once a blocked hit was seen.
    hi: Option<u32>,
    observed: u32,
    /// Unblocked hits that contradicted a blocked one and were ignored.
    ignored: u32,
}

impl BlockDelayBelief {
    pub fn new(prior: u32) -> Self {
        Self { prior, lo: 0, hi: None, observed: 0, ignored: 0 }
    }

    pub fn prior(&self) -> u32 {
        self.prior
    }

    /// A hit that this item blocks once raised (from the front, a blockable damage type),
    /// `k` server ticks into the use.
    ///
    /// A blocked hit is proof the item was up, so it wins a conflict; an unblocked one can have a
    /// cause we can't see (a piercing arrow, a plugin), so it is ignored on conflict.
    pub fn observe(&mut self, k: u32, blocked: bool) {
        self.observed += 1;
        if blocked {
            let hi = k + SLACK;
            if hi < self.lo {
                self.lo = 0;
            }
            self.hi = Some(self.hi.map_or(hi, |h| h.min(hi)));
        } else {
            let lo = k;
            if self.hi.is_some_and(|h| lo > h) {
                self.ignored += 1;
                return;
            }
            self.lo = self.lo.max(lo);
        }
    }

    /// Best estimate: the prior when the observations allow it, else the nearest bound.
    pub fn ticks(&self) -> u32 {
        let p = self.prior.max(self.lo);
        self.hi.map_or(p, |h| p.min(h))
    }

    /// What the observations allow: `d` in `lo..=hi` (`hi` None: no blocked hit yet).
    pub fn bounds(&self) -> (u32, Option<u32>) {
        (self.lo, self.hi)
    }

    pub fn observed(&self) -> u32 {
        self.observed
    }

    pub fn ignored(&self) -> u32 {
        self.ignored
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
        assert_eq!((b.ticks(), b.bounds()), (5, (3, Some(8))));
    }

    #[test]
    fn hits_through_after_the_prior_raise_the_estimate() {
        // The item says 5; the server lets hits through until 19: a different delay.
        let mut b = BlockDelayBelief::new(5);
        b.observe(12, false);
        assert_eq!(b.ticks(), 12);
        b.observe(19, false);
        b.observe(21, true);
        assert_eq!((b.ticks(), b.bounds()), (19, (19, Some(22))));
    }

    #[test]
    fn a_block_before_the_prior_lowers_the_estimate() {
        let mut b = BlockDelayBelief::new(20);
        b.observe(6, true);
        assert_eq!(b.ticks(), 7);
    }

    #[test]
    fn a_blocked_hit_wins_a_conflict() {
        let mut b = BlockDelayBelief::new(5);
        b.observe(30, false); // something we can't see let it through
        b.observe(6, true);
        assert_eq!((b.ticks(), b.bounds()), (5, (0, Some(7))));
        b.observe(40, false);
        assert_eq!((b.ignored(), b.ticks()), (1, 5));
    }
}
