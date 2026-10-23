//! One bot's belief about its shield's raise-to-block delay, fed from its view each tick.

use null_bot_api::{BotView, Hand, ItemStack, ShieldHit};
use null_mechanics::BlockDelayBelief;

/// A piercing arrow goes through any raised item (vanilla `LivingEntity#applyItemBlocking`), and
/// we can't tell a piercing one, so arrow hits are never evidence. A 0-damage hit (snowball, egg)
/// never reaches us: the server drops it before any packet (vanilla `Player#hurtServer`; Paper
/// keeps that).
const NEVER_EVIDENCE: &[&str] = &["minecraft:arrow"];

/// How far the attacker's position, as we see it, may be off in angle: a hit through the item
/// counts only from this far inside its blocking arc.
const ANGLE_MARGIN_DEG: f64 = 45.0;

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
        if let Some(p) = blocking_item(v).and_then(ItemStack::block_delay_ticks)
            && self.belief.is_none_or(|b| b.prior() != p)
        {
            self.belief = Some(BlockDelayBelief::new(p));
            self.seen = v.shield_hits().last().map_or(self.seen, |h| h.tick.max(self.seen));
            return;
        }
        let Some(b) = self.belief.as_mut() else { return };
        let rules = blocking_item(v).and_then(|s| s.blocks_attacks.as_ref());
        for h in v.shield_hits().iter().filter(|h| h.tick > self.seen) {
            if h.blocked || rules.is_some_and(|r| unblocked_counts(v, r, h)) {
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
        None => v.held_item().filter(|s| s.blocks_attacks.is_some()).or(v.offhand()),
    }
}

/// A hit through the item says "not up yet" only if the raised item, by its own rules, would
/// surely have blocked it.
fn unblocked_counts(v: &dyn BotView, rules: &null_bot_api::BlocksAttacks, h: &ShieldHit) -> bool {
    let (Some(from), Some(kind)) = (h.from, h.damage_type.as_deref()) else { return false };
    if NEVER_EVIDENCE.contains(&kind) {
        return false;
    }
    // Vanilla view vector at yaw: (-sin, cos).
    let yaw = (v.look().yaw as f64).to_radians();
    let (fx, fz) = (-yaw.sin(), yaw.cos());
    let p = v.position();
    let (dx, dz) = (from.x - p.x, from.z - p.z);
    let len = (dx * dx + dz * dz).sqrt();
    len > 1e-6 && rules.surely_blocks(kind, ((fx * dx + fz * dz) / len).acos().to_degrees() + ANGLE_MARGIN_DEG)
}

#[cfg(test)]
mod tests {
    use super::*;
    use null_bot_api::{BlocksAttacks, DamageReduction, DamageTypes, Vec3, fake::FakeView};

    fn hit(tick: u64, k: u32, blocked: bool, from_z: f64, kind: &str) -> ShieldHit {
        ShieldHit {
            tick,
            use_ticks: k,
            blocked,
            from: (!blocked).then(|| Vec3::new(0.0, 0.0, from_z)),
            damage_type: (!blocked).then(|| kind.to_string()),
        }
    }

    /// The vanilla shield's rules, with a short stand-in for `#bypasses_shield`.
    fn vanilla_rules(delay: u32) -> BlocksAttacks {
        BlocksAttacks {
            delay_ticks: delay,
            reductions: vec![DamageReduction { angle_deg: 90.0, types: None, base: 0.0, factor: 1.0 }],
            bypassed_by: Some(DamageTypes::Known(vec!["minecraft:magic".into(), "minecraft:lava".into()])),
        }
    }

    fn view_with(rules: BlocksAttacks) -> FakeView {
        let mut v = FakeView::default();
        let mut s = ItemStack::new("minecraft:shield", 1);
        s.blocks_attacks = Some(rules);
        v.hotbar[0] = Some(s);
        v.start_using(Hand::Main, 1);
        v
    }

    fn shield_view(delay: u32) -> FakeView {
        view_with(vanilla_rules(delay))
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
        assert_eq!((b.ticks(), b.bounds(), b.observed()), (24, (24, Some(26)), 2));
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
    fn a_custom_item_is_judged_by_its_own_rules() {
        let front = |k| hit(1, k, false, 3.0, "minecraft:player_attack");
        let off_20_deg = |k| ShieldHit { from: Some(Vec3::new(3.0 * 20f64.to_radians().tan(), 0.0, 3.0)), ..front(k) };
        let observed = |rules: BlocksAttacks, h: ShieldHit| {
            let mut v = view_with(rules);
            let mut sd = ShieldDelay::default();
            sd.update(&v);
            v.shield_hits = vec![h];
            sd.update(&v);
            sd.belief().unwrap().observed()
        };
        assert_eq!(observed(vanilla_rules(5), off_20_deg(30)), 1, "vanilla shield: 20° + margin is inside 90°");
        let mut narrow = vanilla_rules(5);
        narrow.reductions[0].angle_deg = 30.0;
        assert_eq!(observed(narrow, off_20_deg(30)), 0, "20° + margin is outside a 30° arc");
        let mut unknown_tag = vanilla_rules(5);
        unknown_tag.bypassed_by = Some(DamageTypes::Unknown);
        assert_eq!(observed(unknown_tag, front(30)), 0, "can't tell what bypasses it");
        let mut fire_only = vanilla_rules(5);
        fire_only.reductions[0].types = Some(DamageTypes::Known(vec!["minecraft:in_fire".into()]));
        assert_eq!(observed(fire_only, front(30)), 0, "it only reduces fire");
        let mut negative = vanilla_rules(5);
        negative.reductions[0].base = -2.0;
        assert_eq!(observed(negative, front(30)), 0, "small hits are not reduced at all");
    }

    #[test]
    fn a_new_item_value_starts_over() {
        let mut v = shield_view(5);
        let mut sd = ShieldDelay::default();
        sd.update(&v);
        v.shield_hits = vec![hit(1, 9, true, 0.0, "")];
        sd.update(&v);
        assert_eq!(sd.belief().unwrap().observed(), 1);
        v.hotbar[0].as_mut().unwrap().blocks_attacks.as_mut().unwrap().delay_ticks = 20;
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
