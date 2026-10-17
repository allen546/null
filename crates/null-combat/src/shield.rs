//! One bot's belief about its shield's raise-to-block delay, fed from its view each tick.

use null_bot_api::{BotView, Hand, ItemStack, ShieldHit};
use null_mechanics::BlockDelayBelief;

/// Damage types a raised shield blocks from the front: not in vanilla 1.21.11's
/// `#minecraft:bypasses_shield` (which includes `#bypasses_armor`). Left out: arrows (a piercing
/// arrow goes through, vanilla `LivingEntity#applyItemBlocking`, and we can't tell) and `thrown`
/// (snowballs and eggs do 0 damage, and a 0-damage hit is never blocked but still reported).
const BLOCKABLE: &[&str] = &[
    "minecraft:player_attack",
    "minecraft:mob_attack",
    "minecraft:mob_attack_no_aggro",
    "minecraft:spear",
    "minecraft:mace_smash",
    "minecraft:trident",
    "minecraft:mob_projectile",
    "minecraft:spit",
    "minecraft:sting",
];

/// An unblocked hit counts only from well inside the shield's front arc (vanilla default 90°).
const FRONT_DEG: f64 = 45.0;

#[derive(Clone, Debug, Default)]
pub struct ShieldDelay {
    belief: Option<BlockDelayBelief>,
    /// Newest hit tick already used.
    seen: u64,
}

impl ShieldDelay {
    pub fn belief(&self) -> Option<&BlockDelayBelief> {
        self.belief.as_ref()
    }

    // Spec: specs/F5-version-layer.md §4.6 (prior from the item, refined by observation)
    /// The prior comes from the blocking item in use (else the one held); a different value means
    /// a different item, so the belief starts over. Then new hits taken in use narrow it. Hits
    /// from before the belief started (another item, a past life) are not evidence.
    pub fn update(&mut self, v: &dyn BotView) {
        if let Some(p) = blocking_item(v).and_then(|s| s.block_delay_ticks)
            && self.belief.is_none_or(|b| b.prior() != p)
        {
            self.belief = Some(BlockDelayBelief::new(p));
            self.seen = v.shield_hits().last().map_or(self.seen, |h| h.tick.max(self.seen));
            return;
        }
        let Some(b) = self.belief.as_mut() else { return };
        for h in v.shield_hits().iter().filter(|h| h.tick > self.seen) {
            if h.blocked || unblocked_counts(v, h) {
                b.observe(h.use_ticks, h.blocked);
            }
        }
        self.seen = v.shield_hits().last().map_or(self.seen, |h| h.tick.max(self.seen));
    }
}

fn blocking_item(v: &dyn BotView) -> Option<&ItemStack> {
    match v.using_item().map(|u| u.hand) {
        Some(Hand::Main) => v.held_item(),
        Some(Hand::Off) => v.offhand(),
        None => v.held_item().filter(|s| s.block_delay_ticks.is_some()).or(v.offhand()),
    }
}

/// A hit through the shield says "not up yet" only if a raised shield would have blocked it.
fn unblocked_counts(v: &dyn BotView, h: &ShieldHit) -> bool {
    let (Some(from), Some(kind)) = (h.from, h.damage_type.as_deref()) else { return false };
    if !BLOCKABLE.contains(&kind) {
        return false;
    }
    // Vanilla view vector at yaw: (-sin, cos).
    let yaw = (v.look().yaw as f64).to_radians();
    let (fx, fz) = (-yaw.sin(), yaw.cos());
    let p = v.position();
    let (dx, dz) = (from.x - p.x, from.z - p.z);
    let len = (dx * dx + dz * dz).sqrt();
    len > 1e-6 && ((fx * dx + fz * dz) / len).acos().to_degrees() <= FRONT_DEG
}

#[cfg(test)]
mod tests {
    use super::*;
    use null_bot_api::{Vec3, fake::FakeView};

    fn hit(tick: u64, k: u32, blocked: bool, from_z: f64, kind: &str) -> ShieldHit {
        ShieldHit {
            tick,
            use_ticks: k,
            blocked,
            from: (!blocked).then(|| Vec3::new(0.0, 0.0, from_z)),
            damage_type: (!blocked).then(|| kind.to_string()),
        }
    }

    fn shield_view(delay: u32) -> FakeView {
        let mut v = FakeView::default();
        let mut s = ItemStack::new("minecraft:shield", 1);
        s.block_delay_ticks = Some(delay);
        v.hotbar[0] = Some(s);
        v.start_using(Hand::Main, 1);
        v
    }

    #[test]
    fn prior_from_the_item_then_narrowed() {
        // Yaw 0 faces +z; the bot stands at the origin.
        let mut v = shield_view(20);
        let mut sd = ShieldDelay::default();
        sd.update(&v);
        assert_eq!(sd.belief().unwrap().ticks(), 20);
        v.tick = 30;
        v.shield_hits = vec![hit(10, 22, false, 3.0, "minecraft:player_attack"), hit(30, 25, true, 0.0, "")];
        sd.update(&v);
        let b = sd.belief().unwrap();
        assert_eq!((b.ticks(), b.bounds(), b.observed()), (22, (22, Some(26)), 2));
        sd.update(&v);
        assert_eq!(sd.belief().unwrap().observed(), 2, "each hit is used once");
    }

    #[test]
    fn hits_a_shield_would_not_block_are_not_evidence() {
        let mut v = shield_view(5);
        let mut sd = ShieldDelay::default();
        sd.update(&v);
        v.shield_hits = vec![
            hit(1, 30, false, -3.0, "minecraft:player_attack"), // from behind
            hit(2, 30, false, 3.0, "minecraft:magic"),          // bypasses shields
            hit(3, 30, false, 3.0, "minecraft:arrow"),          // might pierce
        ];
        sd.update(&v);
        assert_eq!((sd.belief().unwrap().observed(), sd.belief().unwrap().ticks()), (0, 5));
    }

    #[test]
    fn a_new_item_value_starts_over() {
        let mut v = shield_view(5);
        let mut sd = ShieldDelay::default();
        sd.update(&v);
        v.shield_hits = vec![hit(1, 9, true, 0.0, "")];
        sd.update(&v);
        assert_eq!(sd.belief().unwrap().observed(), 1);
        v.hotbar[0].as_mut().unwrap().block_delay_ticks = Some(20);
        sd.update(&v);
        sd.update(&v);
        assert_eq!((sd.belief().unwrap().prior(), sd.belief().unwrap().observed()), (20, 0));
    }

    #[test]
    fn hits_from_before_the_belief_are_skipped() {
        // A fresh tracker (after a respawn) still sees the last 10 s of hits.
        let mut v = shield_view(20);
        v.shield_hits = vec![hit(5, 6, true, 0.0, "")];
        let mut sd = ShieldDelay::default();
        sd.update(&v);
        sd.update(&v);
        assert_eq!((sd.belief().unwrap().observed(), sd.belief().unwrap().ticks()), (0, 20));
    }
}
