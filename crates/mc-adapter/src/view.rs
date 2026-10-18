//! `BotView` over Azalea components. Scalars are read straight from the borrowed components;
//! lists (hotbar, entities) are built on first use each tick, so a skill that never asks costs 0.

use std::cell::OnceCell;

use azalea::{
    core::entity_id::MinecraftEntityId,
    entity::{
        Dead, EntityKindComponent, LookDirection, Physics, Position, inventory::Inventory,
        metadata::Health,
    },
    local_player::{Hunger, WorldHolder},
    pathfinder::{ExecutingPath, Pathfinder},
    player::GameProfileComponent,
    protocol::packets::game::s_interact::InteractionHand,
    world::WorldName,
};
use azalea::ecs::prelude::*;
use azalea::registry::{DataRegistry, HolderSet};
use null_bot_api as api;
use null_bot_api::BotView;

/// Everything the view reads for one bot (a bundle of borrows from one query row).
pub struct ViewParts<'a> {
    pub entity: Entity,
    pub tick: u64,
    pub position: &'a Position,
    pub physics: &'a Physics,
    pub look: &'a LookDirection,
    pub health: Option<&'a Health>,
    pub hunger: Option<&'a Hunger>,
    pub using_item: Option<&'a azalea::ItemInUse>,
    pub attack_scale: f32,
    pub inventory: &'a Inventory,
    pub world: &'a WorldHolder,
    pub world_name: &'a WorldName,
    pub pathfinder: &'a Pathfinder,
    pub executing: Option<&'a ExecutingPath>,
    /// A goto was issued in the last tick or two and Azalea has not picked it up yet.
    pub nav_pending: bool,
    pub blocks: Option<&'a crate::blocks::BlockActions>,
    pub results: Option<&'a azalea::block_actions::BlockActionResults>,
    pub attackers: Option<&'a azalea::attackers::RecentAttackers>,
    pub tags: Option<&'a azalea::tags::ServerTags>,
    /// A window is open (the fork's `OpenScreen`).
    pub screen_open: bool,
    /// The open container's menu kind.
    pub screen_kind: Option<azalea::registry::builtin::MenuKind>,
    pub clicks: Option<&'a azalea::inventory::ClickResults>,
    /// The bot is holding the crouch key (the input the server is sent that tick).
    pub sneaking: bool,
}

/// Other entities, for `BotView::entities`.
pub type OthersQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static MinecraftEntityId,
        &'static EntityKindComponent,
        &'static Position,
        &'static Physics,
        &'static LookDirection,
        &'static WorldName,
        Option<&'static GameProfileComponent>,
        Option<&'static Health>,
        Has<Dead>,
    ),
>;

pub struct AzaleaView<'a, 'w, 's> {
    p: ViewParts<'a>,
    others: &'a OthersQuery<'w, 's>,
    hotbar: OnceCell<[Option<api::ItemStack>; 9]>,
    inventory: OnceCell<[Option<api::ItemStack>; 36]>,
    offhand: OnceCell<Option<api::ItemStack>>,
    entities: OnceCell<Vec<api::EntitySnapshot>>,
    attackers: OnceCell<Vec<api::Attacker>>,
    shield_hits: OnceCell<Vec<api::ShieldHit>>,
    window: OnceCell<Option<api::Window>>,
}

fn v3(v: azalea::Vec3) -> api::Vec3 {
    api::Vec3::new(v.x, v.y, v.z)
}

const DAMAGE_TYPE: &str = "minecraft:damage_type";

impl AzaleaView<'_, '_, '_> {
    fn stack(&self, s: &azalea::inventory::ItemStack) -> Option<api::ItemStack> {
        match s {
            azalea::inventory::ItemStack::Empty => None,
            azalea::inventory::ItemStack::Present(d) => Some(api::ItemStack {
                item: d.kind.to_str().to_string(),
                count: d.count.clamp(0, 255) as u8,
                blocks_attacks: d
                    .get_component::<azalea::inventory::components::BlocksAttacks>()
                    .map(|b| self.blocks_attacks(&b)),
            }),
        }
    }

    // Spec: specs/F5-version-layer.md §4.6 (the item's own rules, as the server sent them)
    fn blocks_attacks(&self, b: &azalea::inventory::components::BlocksAttacks) -> api::BlocksAttacks {
        api::BlocksAttacks {
            // Vanilla `BlocksAttacks#blockDelayTicks`: seconds * 20, rounded.
            delay_ticks: (b.block_delay_seconds * 20.0).round().max(0.0) as u32,
            reductions: b
                .damage_reductions
                .iter()
                .map(|r| api::DamageReduction {
                    angle_deg: r.horizontal_blocking_angle,
                    types: r.kind.as_ref().map(|k| match k {
                        HolderSet::Direct { contents } => self.damage_types(contents.iter().map(|d| d.protocol_id())),
                        HolderSet::Named { key, .. } => self.damage_tag(key),
                    }),
                    base: r.base,
                    factor: r.factor,
                })
                .collect(),
            bypassed_by: b.bypassed_by.as_ref().map(|t| self.damage_tag(t)),
        }
    }

    /// A damage type tag, resolved with the tags the server sent.
    fn damage_tag(&self, tag: &azalea::Identifier) -> api::DamageTypes {
        let ids = self.p.tags.and_then(|t| t.get(&azalea::Identifier::new(DAMAGE_TYPE), tag));
        match ids {
            Some(ids) => self.damage_types(ids.iter().copied()),
            None => api::DamageTypes::Unknown,
        }
    }

    /// Damage type protocol ids to names, through the server's synced registry.
    fn damage_types(&self, ids: impl Iterator<Item = u32>) -> api::DamageTypes {
        let w = self.p.world.shared.read();
        let Some(reg) = w.registries.extra.get(&azalea::Identifier::new(DAMAGE_TYPE)) else {
            return api::DamageTypes::Unknown;
        };
        let names: Option<Vec<String>> =
            ids.map(|i| reg.map.get_index(i as usize).map(|(id, _)| id.to_string())).collect();
        names.map_or(api::DamageTypes::Unknown, api::DamageTypes::Known)
    }
}

/// How long a hit stays in `recent_attackers()`: 10 s.
const RECENT_ATTACKER_TICKS: u64 = 200;

/// How far around the bot `entities()` looks. The node's bots share one world, so this includes
/// what teammates see; it is the team's shared picture of the area.
const ENTITY_RADIUS: f64 = 256.0;

impl<'a, 'w, 's> AzaleaView<'a, 'w, 's> {
    pub fn new(p: ViewParts<'a>, others: &'a OthersQuery<'w, 's>) -> Self {
        Self {
            p,
            others,
            hotbar: OnceCell::new(),
            inventory: OnceCell::new(),
            offhand: OnceCell::new(),
            entities: OnceCell::new(),
            attackers: OnceCell::new(),
            shield_hits: OnceCell::new(),
            window: OnceCell::new(),
        }
    }
}

impl BotView for AzaleaView<'_, '_, '_> {
    fn tick(&self) -> u64 {
        self.p.tick
    }
    fn position(&self) -> api::Vec3 {
        v3(**self.p.position)
    }
    fn velocity(&self) -> api::Vec3 {
        v3(self.p.physics.velocity)
    }
    fn look(&self) -> api::Look {
        api::Look { yaw: self.p.look.y_rot(), pitch: self.p.look.x_rot() }
    }
    fn on_ground(&self) -> bool {
        self.p.physics.on_ground()
    }
    fn in_water(&self) -> bool {
        self.p.physics.is_in_water()
    }
    fn in_lava(&self) -> bool {
        self.p.physics.is_in_lava()
    }
    fn fall_distance(&self) -> f32 {
        self.p.physics.fall_distance as f32
    }
    fn health(&self) -> f32 {
        self.p.health.map_or(20.0, |h| h.0)
    }
    fn food(&self) -> u32 {
        self.p.hunger.map_or(20, |h| h.food)
    }
    fn attack_cooldown(&self) -> f32 {
        self.p.attack_scale
    }
    fn hotbar(&self) -> &[Option<api::ItemStack>; 9] {
        self.hotbar.get_or_init(|| {
            let menu = &self.p.inventory.inventory_menu;
            let start = *menu.hotbar_slots_range().start();
            std::array::from_fn(|i| menu.slot(start + i).and_then(|s| self.stack(s)))
        })
    }
    // Spec: specs/B3-inventory-crafting.md §4.1 (slot management: read all 36 slots)
    fn inventory(&self) -> &[Option<api::ItemStack>; 36] {
        // Player menu: slots 9-35 main, 36-44 hotbar; the server's numbering puts the hotbar first.
        self.inventory.get_or_init(|| {
            let menu = &self.p.inventory.inventory_menu;
            std::array::from_fn(|i| {
                let slot = if i < 9 { 36 + i } else { i };
                menu.slot(slot).and_then(|s| self.stack(s))
            })
        })
    }
    fn selected_slot(&self) -> u8 {
        self.p.inventory.selected_hotbar_slot
    }
    fn offhand(&self) -> Option<&api::ItemStack> {
        // Player menu slot 45.
        self.offhand
            .get_or_init(|| self.p.inventory.inventory_menu.slot(45).and_then(|s| self.stack(s)))
            .as_ref()
    }
    // Spec: specs/B2-survival-upkeep.md §4.1 (the fork's ItemInUse: hand and ticks)
    fn using_item(&self) -> Option<api::UsingItem> {
        let hand = match self.p.using_item?.hand {
            InteractionHand::MainHand => api::Hand::Main,
            InteractionHand::OffHand => api::Hand::Off,
        };
        let u = self.p.using_item?;
        Some(api::UsingItem { hand, ticks: u.ticks, server_ticks: u.server_ticks() })
    }
    fn entities(&self) -> &[api::EntitySnapshot] {
        self.entities.get_or_init(|| {
            let me = v3(**self.p.position);
            let mut out = Vec::new();
            for (e, id, kind, pos, phys, look, wn, profile, hp, dead) in self.others.iter() {
                if e == self.p.entity || wn != self.p.world_name {
                    continue;
                }
                let pv = v3(**pos);
                if (pv.x - me.x).abs() > ENTITY_RADIUS || (pv.z - me.z).abs() > ENTITY_RADIUS {
                    continue;
                }
                let name = kind.0.to_str();
                let mut s = api::EntitySnapshot::new(id.0 as u32, name, pv);
                s.velocity = v3(phys.velocity);
                s.look = api::Look { yaw: look.y_rot(), pitch: look.x_rot() };
                s.on_ground = phys.on_ground();
                // Dead stays visible with health 0: a fight then ends as "target dead", and
                // targeting skips it on health.
                s.health = if dead { Some(0.0) } else { hp.map(|h| h.0) };
                s.name = profile.map(|p| p.name.clone());
                out.push(s);
            }
            out
        })
    }
    // Spec: specs/B5-combat.md §4.7 (who hit us; the fork's RecentAttackers, ticks on the host clock)
    // Spec: specs/F5-version-layer.md §4.6 (observations for the shield-delay belief)
    fn shield_hits(&self) -> &[api::ShieldHit] {
        self.shield_hits.get_or_init(|| {
            let Some(r) = self.p.attackers else { return Vec::new() };
            let host_tick = |t: u64| self.p.tick.saturating_sub(r.now.saturating_sub(t));
            let cutoff = r.now.saturating_sub(RECENT_ATTACKER_TICKS);
            let w = self.p.world.shared.read();
            let types = w.registries.extra.get(&azalea::Identifier::new("minecraft:damage_type"));
            let mut out: Vec<api::ShieldHit> = Vec::new();
            for h in r.hits.iter().filter(|h| h.tick >= cutoff) {
                let Some(use_ticks) = h.use_ticks else { continue };
                // Vanilla `DamageSource#getSourcePosition`: the given position, else the direct entity's.
                let from = h.position.map(v3).or_else(|| {
                    let id = h.direct?;
                    self.others.iter().find(|o| *o.1 == id).map(|o| v3(**o.3))
                });
                out.push(api::ShieldHit {
                    tick: host_tick(h.tick),
                    use_ticks,
                    blocked: false,
                    from,
                    damage_type: types
                        .and_then(|t| t.map.get_index(h.source_type as usize))
                        .map(|(id, _)| id.to_string()),
                });
            }
            for b in r.blocked.iter().filter(|b| b.tick >= cutoff) {
                let Some(use_ticks) = b.use_ticks else { continue };
                out.push(api::ShieldHit { tick: host_tick(b.tick), use_ticks, blocked: true, from: None, damage_type: None });
            }
            out.sort_by_key(|h| h.tick);
            out
        })
    }
    fn recent_attackers(&self) -> &[api::Attacker] {
        self.attackers.get_or_init(|| {
            let Some(r) = self.p.attackers else { return Vec::new() };
            r.within(RECENT_ATTACKER_TICKS)
                .filter_map(|h| {
                    Some(api::Attacker {
                        entity: api::EntityId(h.attacker()?.0 as u32),
                        tick: self.p.tick.saturating_sub(r.now.saturating_sub(h.tick)),
                    })
                })
                .collect()
        })
    }
    fn is_solid(&self, pos: api::BlockPos) -> bool {
        let w = self.p.world.shared.read();
        w.chunks
            .get_block_state(azalea::BlockPos::new(pos.x, pos.y, pos.z))
            .is_some_and(azalea::pathfinder::world::is_block_state_solid)
    }
    // Spec: specs/B6-placement-breaking.md §4.6 (read back the block the server left)
    fn block_at(&self, pos: api::BlockPos) -> Option<String> {
        crate::blocks::block_name(&self.p.world.shared.read(), pos)
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (crouch handling)
    fn sneaking(&self) -> bool {
        self.p.sneaking
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (crouch handling)
    fn block_uses_click(&self, pos: api::BlockPos) -> bool {
        self.p
            .world
            .shared
            .read()
            .chunks
            .get_block_state(azalea::BlockPos::new(pos.x, pos.y, pos.z))
            .is_some_and(azalea::block::state_uses_click)
    }
    // Spec: specs/B6-placement-breaking.md §4.4 (break timing and result)
    fn last_break(&self) -> Option<api::BlockAction> {
        crate::blocks::translate(api::BlockActionKind::Break, self.p.blocks?.last_break, self.p.results)
    }
    // Spec: specs/B6-placement-breaking.md §4.6 (acknowledgement and readback)
    fn last_place(&self) -> Option<api::BlockAction> {
        crate::blocks::translate(api::BlockActionKind::Place, self.p.blocks?.last_place, self.p.results)
    }
    // Spec: specs/B3-inventory-crafting.md §4.1 (the open window, in its own numbering)
    fn window(&self) -> Option<&api::Window> {
        self.window
            .get_or_init(|| {
                if !self.p.screen_open {
                    return None;
                }
                let inv = self.p.inventory;
                let kind = self.p.screen_kind.map(|k| k.to_str());
                Some(api::Window {
                    id: inv.id,
                    kind,
                    slots: inv.menu().slots().iter().map(|s| self.stack(s)).collect(),
                    // Crafting results are the server's (the client has no recipes); storage has none.
                    computed_slots: if inv.id == 0 || kind == Some("minecraft:crafting") { vec![0] } else { vec![] },
                    carried: self.stack(&inv.carried),
                })
            })
            .as_ref()
    }
    // Spec: specs/B3-inventory-crafting.md §4.1 (what the server said after our click)
    fn last_click(&self) -> Option<api::ClickOutcome> {
        let c = self.p.clicks?.last.as_ref()?;
        let slot = |s: i16| u16::try_from(s).ok();
        Some(api::ClickOutcome {
            slot: slot(c.slot_num),
            refused: c.refusal.map(|r| r.reason()),
            ticks: c.ticks,
            corrected: c.corrected_slots.iter().map(|&s| slot(s)).collect(),
            full_resends: c.full_resends,
        })
    }
    // Spec: specs/B3-inventory-crafting.md §4.7 (the latest container open)
    fn last_open(&self) -> Option<api::OpenResult> {
        crate::blocks::translate_open(self.p.blocks?.last_open, self.p.results)
    }
    fn open_refusal(&self) -> Option<&'static str> {
        self.p.clicks?.open_refusal.map(|r| r.reason())
    }
    fn nav_status(&self) -> api::NavStatus {
        if self.p.pathfinder.is_calculating || self.p.nav_pending {
            api::NavStatus::Calculating
        } else if self.p.executing.is_some() {
            api::NavStatus::Following
        } else if self.p.pathfinder.goal.is_some() {
            api::NavStatus::Stalled
        } else {
            api::NavStatus::Idle
        }
    }
}
