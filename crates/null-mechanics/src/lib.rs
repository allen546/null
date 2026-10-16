//! Mechanics table stub (F5 §3, §4.3). Each value says where it came from and how sure we are.
//! Claimed means "believed, not checked"; M2 clamps carry the confidence along.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Confidence {
    /// Read from a primary source (game data, decompiled code, official docs).
    Sourced,
    /// Measured on a real server by us.
    Measured,
    /// From memory or secondary sources; not verified.
    Claimed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mechanic {
    pub value: f64,
    pub unit: &'static str,
    pub confidence: Confidence,
    pub source: &'static str,
}

const WIKI: &str = "Minecraft Wiki, Java attack speed/damage tables, from memory";
const WIKI_AXE: &str = "Minecraft Wiki, axe attack speed by tier, from memory";

use Confidence::*;

const fn m(value: f64, unit: &'static str, confidence: Confidence, source: &'static str) -> Mechanic {
    Mechanic { value, unit, confidence, source }
}

static TABLE: &[(&str, Mechanic)] = &[
    ("combat.entity_interaction_range", m(3.0, "blocks", Claimed, "player attribute default since 1.20.5; not yet checked against 1.21.11 data")),
    ("combat.attack_speed.fist", m(4.0, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.wooden_sword", m(1.6, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.stone_sword", m(1.6, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.iron_sword", m(1.6, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.diamond_sword", m(1.6, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.netherite_sword", m(1.6, "attacks/s", Claimed, WIKI)),
    ("combat.attack_speed.wooden_axe", m(0.8, "attacks/s", Claimed, WIKI_AXE)),
    ("combat.attack_speed.stone_axe", m(0.8, "attacks/s", Claimed, WIKI_AXE)),
    ("combat.attack_speed.iron_axe", m(0.9, "attacks/s", Claimed, WIKI_AXE)),
    ("combat.attack_speed.diamond_axe", m(1.0, "attacks/s", Claimed, WIKI_AXE)),
    ("combat.attack_speed.netherite_axe", m(1.0, "attacks/s", Claimed, WIKI_AXE)),
    ("combat.crit_damage_multiplier", m(1.5, "x", Claimed, "Minecraft Wiki, critical hit; needs falling, off ground, not sprinting, not in water")),
    ("combat.shield_disable_ticks.axe", m(100.0, "ticks", Claimed, "Minecraft Wiki, shield; 5 s cooldown after an axe hit, version drift possible")),
    ("combat.sprint_knockback_bonus", m(1.0, "knockback levels", Claimed, "Minecraft Wiki, knockback; a sprinting hit acts like Knockback I and ends the sprint")),
    ("movement.safe_fall_distance", m(3.0, "blocks", Claimed, "player attribute default since 1.20.5; fall damage is 1 per block beyond it, armor does not reduce it")),
    ("player.max_health", m(20.0, "half-hearts", Claimed, "player attribute default (10 hearts)")),
    ("legit.max_clicks_per_second", m(14.0, "clicks/s", Claimed, "our guess at a sane upper bound for human play, not an anticheat figure")),
    ("legit.max_turn_per_tick", m(60.0, "degrees/tick", Claimed, "our guess; fast human flicks are large but finite")),
];

pub fn mechanic(key: &str) -> Option<Mechanic> {
    TABLE.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

/// All keys, for tools and docs.
pub fn keys() -> impl Iterator<Item = &'static str> {
    TABLE.iter().map(|(k, _)| *k)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_hits_and_misses() {
        let r = mechanic("combat.entity_interaction_range").unwrap();
        assert_eq!(r.value, 3.0);
        assert_eq!(r.unit, "blocks");
        assert!(mechanic("combat.nope").is_none());
    }

    #[test]
    fn every_entry_has_unit_and_source_and_unique_key() {
        let mut seen = std::collections::HashSet::new();
        for (k, v) in TABLE {
            assert!(seen.insert(*k), "duplicate {k}");
            assert!(!v.unit.is_empty() && !v.source.is_empty(), "{k}");
            assert!(v.value.is_finite());
        }
    }

    #[test]
    fn needed_combat_keys_exist() {
        for t in ["fist", "wooden_sword", "stone_sword", "iron_sword", "diamond_sword", "netherite_sword", "wooden_axe", "stone_axe", "iron_axe", "diamond_axe", "netherite_axe"] {
            assert!(mechanic(&format!("combat.attack_speed.{t}")).is_some(), "{t}");
        }
        for k in ["combat.crit_damage_multiplier", "combat.shield_disable_ticks.axe", "combat.sprint_knockback_bonus", "player.max_health", "legit.max_clicks_per_second", "legit.max_turn_per_tick"] {
            assert!(mechanic(k).is_some(), "{k}");
        }
    }

    #[test]
    fn axes_are_slower_than_swords() {
        let s = mechanic("combat.attack_speed.iron_sword").unwrap().value;
        let a = mechanic("combat.attack_speed.iron_axe").unwrap().value;
        assert!(a < s);
    }
}
