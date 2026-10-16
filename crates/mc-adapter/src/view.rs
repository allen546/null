//! `BotView` over Azalea components. Scalars are read straight from the borrowed components;
//! lists (hotbar, entities) are built on first use each tick, so a skill that never asks costs 0.

use std::cell::OnceCell;

use azalea::{
    core::entity_id::MinecraftEntityId,
    entity::{
        Dead, EntityKindComponent, LookDirection, Physics, Position, inventory::Inventory,
        metadata::{AbstractLivingUsingItem, Health},
    },
    local_player::{Hunger, WorldHolder},
    pathfinder::{ExecutingPath, Pathfinder},
    player::GameProfileComponent,
    world::WorldName,
};
use azalea::ecs::prelude::*;
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
    pub using_item: Option<&'a AbstractLivingUsingItem>,
    pub attack_scale: f32,
    pub inventory: &'a Inventory,
    pub world: &'a WorldHolder,
    pub world_name: &'a WorldName,
    pub pathfinder: &'a Pathfinder,
    pub executing: Option<&'a ExecutingPath>,
    /// A goto was issued in the last tick or two and Azalea has not picked it up yet.
    pub nav_pending: bool,
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
    offhand: OnceCell<Option<api::ItemStack>>,
    entities: OnceCell<Vec<api::EntitySnapshot>>,
}

fn v3(v: azalea::Vec3) -> api::Vec3 {
    api::Vec3::new(v.x, v.y, v.z)
}

fn stack(s: &azalea::inventory::ItemStack) -> Option<api::ItemStack> {
    match s {
        azalea::inventory::ItemStack::Empty => None,
        azalea::inventory::ItemStack::Present(d) => {
            Some(api::ItemStack { item: d.kind.to_str().to_string(), count: d.count.clamp(0, 255) as u8 })
        }
    }
}

/// How far around the bot `entities()` looks. The node's bots share one world, so this includes
/// what teammates see; it is the team's shared picture of the area.
const ENTITY_RADIUS: f64 = 256.0;

impl<'a, 'w, 's> AzaleaView<'a, 'w, 's> {
    pub fn new(p: ViewParts<'a>, others: &'a OthersQuery<'w, 's>) -> Self {
        Self {
            p,
            others,
            hotbar: OnceCell::new(),
            offhand: OnceCell::new(),
            entities: OnceCell::new(),
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
            std::array::from_fn(|i| menu.slot(start + i).and_then(stack))
        })
    }
    fn selected_slot(&self) -> u8 {
        self.p.inventory.selected_hotbar_slot
    }
    fn offhand(&self) -> Option<&api::ItemStack> {
        // Player menu slot 45.
        self.offhand
            .get_or_init(|| self.p.inventory.inventory_menu.slot(45).and_then(stack))
            .as_ref()
    }
    fn using_item(&self) -> Option<api::UsingItem> {
        // Azalea tracks only the flag, not the hand or the tick count.
        self.p.using_item.filter(|u| u.0).map(|_| api::UsingItem { hand: api::Hand::Main, ticks: 0 })
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
    fn recent_attackers(&self) -> &[api::Attacker] {
        // Azalea does not track who hit us; the damage-source observer comes with F2.
        &[]
    }
    fn is_solid(&self, pos: api::BlockPos) -> bool {
        let w = self.p.world.shared.read();
        w.chunks
            .get_block_state(azalea::BlockPos::new(pos.x, pos.y, pos.z))
            .is_some_and(azalea::pathfinder::world::is_block_state_solid)
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
