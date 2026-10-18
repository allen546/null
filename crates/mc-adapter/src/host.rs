//! The per-tick host: `decide` runs every bot's `Brain` against a borrowed view and buffers its
//! commands; `apply` (chained right after, still in GameTick and before physics) turns them into
//! component writes. No `Update`-schedule messages for input, so input lands in the same tick
//! (spike §3.2). Goto/stop-nav are the exception: Azalea's pathfinder only takes messages.

use azalea::{
    BlockPos as ABlockPos, ItemInUse, Vec3 as AVec3,
    attackers::{RecentAttackers, age_recent_attackers},
    tags::ServerTags,
    attack::{AttackQueued, AttackStrengthScale},
    core::{entity_id::MinecraftEntityId, tick::GameTick},
    entity::{
        Dead, HasClientLoaded, Jumping, LocalEntity, LookDirection, Physics, Position,
        inventory::Inventory,
        metadata::Health,
    },
    interact::{BlockStatePredictionHandler, handle_start_use_item_queued},
    inventory::{
        ClickResults, CloseContainerEvent, ContainerClickEvent, OpenInventoryEvent, OpenScreen,
        ensure_has_sent_carried_item,
        hold_still_while_screen_open,
        operations::{ClickOperation, PickupAllClick, PickupClick, QuickMoveClick, SwapClick, ThrowClick},
    },
    item_use::{ReleaseUseItemQueued, SwapOffhandQueued, UseItemQueued, handle_use_item_queued},
    block_actions::{
        BlockActionResults, BreakBlockQueued, LookAtBlockQueued, PlaceBlockQueued, StopMiningQueued,
        handle_break_block_queued, handle_look_at_block_queued,
    },
    mining::MiningSystems,
    local_player::{Hunger, WorldHolder},
    pathfinder::{
        ExecutingPath, GotoEvent, Pathfinder, PathfinderOpts, StopPathfindingEvent,
        goals::{BlockPosGoal, RadiusGoal},
    },
    physics::{PhysicsSystems, client_movement::{ClientMovementState, WalkDirection}},
    protocol::packets::game::s_interact::InteractionHand,
    world::WorldName,
};
use azalea::app::{App, Plugin};
use azalea::ecs::prelude::*;
use azalea::entity::indexing::EntityIdIndex;
use null_bot_api::{Brain, Command, Hand, Skill, SkillStatus};

use crate::blocks::{self, BlockActions, Issued};
use crate::view::{AzaleaView, OthersQuery, ViewParts};

/// A bot's decision state: reflexes, the active skill and the commands it just issued.
#[derive(Component)]
#[require(HostFlags)]
pub struct BotBrain {
    pub brain: Brain,
    pub skill: Option<Box<dyn Skill>>,
    pending: Vec<Command>,
}

impl BotBrain {
    pub fn new(brain: Brain, skill: Option<Box<dyn Skill>>) -> Self {
        Self { brain, skill, pending: Vec::new() }
    }
}

/// Host-side bookkeeping that is not the decision code's business.
#[derive(Component, Default)]
pub struct HostFlags {
    /// A jump was set last tick and must be released this tick (one physics step).
    jump_pulse: bool,
    /// Host tick of the last goto, until Azalea's pathfinder has picked it up.
    nav_requested: Option<u64>,
    /// Our latest break and place commands (B6 §4.6).
    pub(crate) blocks: BlockActions,
}

/// The host clock; `TicksConnected` is missing in some setups and resets on reconnect.
#[derive(Resource, Default)]
pub struct HostTick(pub u64);

/// Written when a skill returns `Done` or `Failed`.
#[derive(Message)]
pub struct SkillEnded {
    pub entity: Entity,
    pub skill: String,
    pub status: SkillStatus,
    pub tick: u64,
}

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        // The pathfinder messages are registered here too (idempotent) so the host also runs
        // in setups without the pathfinder plugin.
        app.init_resource::<HostTick>()
            .add_message::<SkillEnded>()
            .add_message::<GotoEvent>()
            .add_message::<StopPathfindingEvent>()
            .add_systems(
            GameTick,
            // Before everything that reads what `apply` writes: physics, mining, use-item and the
            // carried-item sync (the ambiguity logger flagged each of these).
            (decide, apply)
                .chain()
                .before(PhysicsSystems)
                .before(MiningSystems)
                .before(handle_break_block_queued)
                .before(handle_look_at_block_queued)
                .before(ensure_has_sent_carried_item)
                .before(handle_start_use_item_queued)
                .before(handle_use_item_queued)
                .before(hold_still_while_screen_open)
                .before(age_recent_attackers),
        );
    }
}

#[derive(Default)]
struct CommandBuffer(Vec<Command>);

impl null_bot_api::BotCommands for CommandBuffer {
    fn walk(&mut self, forward: i8, strafe: i8) {
        self.0.push(Command::Walk { forward: forward.clamp(-1, 1), strafe: strafe.clamp(-1, 1) });
    }
    fn sprint(&mut self, on: bool) {
        self.0.push(Command::Sprint(on));
    }
    fn jump(&mut self) {
        self.0.push(Command::Jump);
    }
    fn sneak(&mut self, on: bool) {
        self.0.push(Command::Sneak(on));
    }
    fn look(&mut self, yaw: f32, pitch: f32) {
        self.0.push(Command::Look { yaw, pitch });
    }
    fn select_slot(&mut self, slot: u8) {
        self.0.push(Command::SelectSlot(slot));
    }
    fn attack(&mut self, target: null_bot_api::EntityId) {
        self.0.push(Command::Attack(target));
    }
    fn use_item(&mut self, hand: Hand) {
        self.0.push(Command::UseItem(hand));
    }
    fn release_use(&mut self) {
        self.0.push(Command::ReleaseUse);
    }
    fn swap_offhand(&mut self) {
        self.0.push(Command::SwapOffhand);
    }
    fn stop(&mut self) {
        self.0.push(Command::Stop);
    }
    fn goto(&mut self, target: null_bot_api::BlockPos, tolerance: f32) {
        self.0.push(Command::Goto { target, tolerance });
    }
    fn stop_nav(&mut self) {
        self.0.push(Command::StopNav);
    }
    // Spec: specs/B6-placement-breaking.md §4.4 (break timing)
    fn break_block(&mut self, pos: null_bot_api::BlockPos) {
        self.0.push(Command::BreakBlock(pos));
    }
    // Spec: specs/B6-placement-breaking.md §4.4 (abort a break)
    fn cancel_break(&mut self) {
        self.0.push(Command::CancelBreak);
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (place on a chosen face)
    fn place_block(&mut self, pos: null_bot_api::BlockPos, face: null_bot_api::Face) {
        self.0.push(Command::PlaceBlock { pos, face });
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (click choice: aim at a visible point)
    fn look_at_block(&mut self, pos: null_bot_api::BlockPos, face: Option<null_bot_api::Face>) {
        self.0.push(Command::LookAtBlock { pos, face });
    }
    // Spec: specs/B3-inventory-crafting.md §4.1 (open, close and click windows)
    fn open_inventory(&mut self) {
        self.0.push(Command::OpenInventory);
    }
    fn close_window(&mut self) {
        self.0.push(Command::CloseWindow);
    }
    fn click_slot(&mut self, slot: Option<u16>, click: null_bot_api::Click) {
        self.0.push(Command::ClickSlot { slot, click });
    }
}

#[allow(clippy::type_complexity)]
fn decide(
    mut tick: ResMut<HostTick>,
    mut bots: Query<
        (
            Entity,
            &mut BotBrain,
            &Position,
            &Physics,
            &LookDirection,
            Option<&Health>,
            Option<&Hunger>,
            Option<&ItemInUse>,
            Option<&AttackStrengthScale>,
            &Inventory,
            &WorldHolder,
            &WorldName,
            &Pathfinder,
            Option<&ExecutingPath>,
            (
                Option<&HostFlags>,
                Option<&BlockActionResults>,
                Option<&RecentAttackers>,
                Option<&ServerTags>,
                Option<&OpenScreen>,
                Option<&ClickResults>,
            ),
        ),
        (With<LocalEntity>, With<HasClientLoaded>, Without<Dead>),
    >,
    others: OthersQuery,
    mut ended: MessageWriter<SkillEnded>,
) {
    tick.0 += 1;
    let now = tick.0;
    for (entity, mut bb, position, physics, look, health, hunger, using, atk, inv, world, wname, pf, exec, (flags, results, attackers, tags, screen, clicks)) in
        &mut bots
    {
        let nav_pending = flags.and_then(|f| f.nav_requested).is_some_and(|t| now - t <= 2);
        let view = AzaleaView::new(
            ViewParts {
                entity,
                tick: now,
                position,
                physics,
                look,
                health,
                hunger,
                using_item: using,
                attack_scale: atk.map_or(1.0, |a| a.0),
                inventory: inv,
                world,
                world_name: wname,
                pathfinder: pf,
                executing: exec,
                nav_pending,
                blocks: flags.map(|f| &f.blocks),
                results,
                attackers,
                tags,
                screen_open: screen.is_some(),
                clicks,
            },
            &others,
        );
        let mut buf = CommandBuffer::default();
        let bb = &mut *bb;
        // Written out so the trait-object lifetime shortens (Option does not coerce).
        let skill: Option<&mut dyn Skill> = match bb.skill.as_mut() {
            Some(s) => Some(&mut **s),
            None => None,
        };
        let outcome = bb.brain.tick(&view, &mut buf, skill);
        bb.pending = buf.0;
        if let Some(status @ (SkillStatus::Done | SkillStatus::Failed(_))) = outcome.skill {
            let id = bb.skill.as_ref().map(|s| s.id().to_string()).unwrap_or_default();
            bb.skill = None;
            bb.brain.end_job();
            ended.write(SkillEnded { entity, skill: id, status, tick: now });
        }
    }
}

fn walk_direction(forward: i8, strafe: i8) -> WalkDirection {
    match (forward, strafe) {
        (1, 0) => WalkDirection::Forward,
        (-1, 0) => WalkDirection::Backward,
        (0, 1) => WalkDirection::Right,
        (0, -1) => WalkDirection::Left,
        (1, 1) => WalkDirection::ForwardRight,
        (1, -1) => WalkDirection::ForwardLeft,
        (-1, 1) => WalkDirection::BackwardRight,
        (-1, -1) => WalkDirection::BackwardLeft,
        _ => WalkDirection::None,
    }
}

// Spec: specs/B3-inventory-crafting.md §4.1 (clicks as a vanilla client makes them)
fn click_op(slot: Option<u16>, click: null_bot_api::Click) -> ClickOperation {
    use null_bot_api::Click;
    match (slot, click) {
        (None, Click::Right) => PickupClick::RightOutside.into(),
        (None, _) => PickupClick::LeftOutside.into(),
        (Some(slot), Click::Left) => PickupClick::Left { slot: Some(slot) }.into(),
        (Some(slot), Click::Right) => PickupClick::Right { slot: Some(slot) }.into(),
        (Some(slot), Click::ShiftLeft) => QuickMoveClick::Left { slot }.into(),
        (Some(slot), Click::SwapHotbar(n)) => SwapClick { source_slot: slot, target_slot: n.min(8) }.into(),
        (Some(slot), Click::SwapOffhand) => SwapClick { source_slot: slot, target_slot: 40 }.into(),
        (Some(slot), Click::Drop) => ThrowClick::Single { slot }.into(),
        (Some(slot), Click::DropStack) => ThrowClick::All { slot }.into(),
        (Some(slot), Click::DoubleClick) => PickupAllClick { slot, reversed: false }.into(),
    }
}

fn hand(h: Hand) -> InteractionHand {
    match h {
        Hand::Main => InteractionHand::MainHand,
        Hand::Off => InteractionHand::OffHand,
    }
}

#[allow(clippy::type_complexity)]
fn apply(
    mut commands: Commands,
    tick: Res<HostTick>,
    mut bots: Query<(
        Entity,
        &mut BotBrain,
        &mut HostFlags,
        &mut ClientMovementState,
        &mut LookDirection,
        &mut Jumping,
        &mut Inventory,
        &mut BlockStatePredictionHandler,
        &EntityIdIndex,
        &BlockActionResults,
    )>,
    mut goto: MessageWriter<GotoEvent>,
    mut stop_nav: MessageWriter<StopPathfindingEvent>,
) {
    for (entity, mut bb, mut flags, mut mv, mut look, mut jumping, mut inv, _pred, ids, results) in &mut bots
    {
        if flags.jump_pulse {
            **jumping = false;
            flags.jump_pulse = false;
        }
        for cmd in std::mem::take(&mut bb.pending) {
            match cmd {
                Command::Walk { forward, strafe } => mv.move_direction = walk_direction(forward, strafe),
                Command::Sprint(on) => mv.trying_to_sprint = on,
                Command::Jump => {
                    **jumping = true;
                    flags.jump_pulse = true;
                }
                Command::Sneak(on) => mv.trying_to_crouch = on,
                Command::Look { yaw, pitch } => look.update(LookDirection::new(yaw, pitch)),
                Command::SelectSlot(s) => {
                    // The fork ends any item use on a slot change, as vanilla does.
                    inv.selected_hotbar_slot = s.min(8);
                }
                Command::Attack(id) => {
                    if let Some(target) = ids.get_by_minecraft_entity(MinecraftEntityId(id.0 as i32)) {
                        commands.entity(entity).insert(AttackQueued { target });
                    }
                }
                // Spec: specs/B2-survival-upkeep.md §4.1 (use, release and offhand swap are fork components)
                Command::UseItem(h) => {
                    commands.entity(entity).insert(UseItemQueued { hand: hand(h) });
                }
                Command::ReleaseUse => {
                    commands.entity(entity).insert(ReleaseUseItemQueued);
                }
                Command::SwapOffhand => {
                    commands.entity(entity).insert(SwapOffhandQueued);
                }
                Command::Stop => {
                    mv.move_direction = WalkDirection::None;
                    mv.trying_to_sprint = false;
                    mv.trying_to_crouch = false;
                    stop_nav.write(StopPathfindingEvent { entity, force: true });
                    flags.nav_requested = None;
                }
                // Spec: specs/B6-placement-breaking.md §4.4 (break; command lands in this GameTick, F5 §4.1)
                Command::BreakBlock(pos) => {
                    commands.entity(entity).insert(BreakBlockQueued { position: blocks::block_pos(pos) });
                    // The fork keeps one result while the same block is still being broken.
                    if !blocks::same_break_pending(flags.blocks.last_break, pos, results.last_break.as_ref()) {
                        flags.blocks.last_break = Some(Issued::new(tick.0, pos, results.last_break.as_ref()));
                    }
                }
                // Spec: specs/B6-placement-breaking.md §4.2 (aim at a visible point of the shape)
                Command::LookAtBlock { pos, face } => {
                    commands.entity(entity).insert(LookAtBlockQueued {
                        position: blocks::block_pos(pos),
                        direction: face.map(blocks::face_to_dir),
                    });
                }
                Command::CancelBreak => {
                    commands.entity(entity).insert(StopMiningQueued);
                }
                // Spec: specs/B6-placement-breaking.md §4.2, §4.6 (place on a chosen face)
                Command::PlaceBlock { pos, face } => {
                    commands.entity(entity).insert(PlaceBlockQueued {
                        position: blocks::block_pos(pos),
                        direction: blocks::face_to_dir(face),
                    });
                    flags.blocks.last_place =
                        Some(Issued::new(tick.0, pos.offset(face), results.last_place.as_ref()));
                }
                Command::Goto { target, tolerance } => {
                    // Mining and placing stay off: B1's allow_mining is a job decision, not a default.
                    // An unreachable goal ends (Idle) instead of searching forever.
                    let opts = PathfinderOpts::new().allow_mining(false).retry_on_no_path(false);
                    let p = ABlockPos::new(target.x, target.y, target.z);
                    let ev = if tolerance <= 0.5 {
                        GotoEvent::new(entity, BlockPosGoal(p), opts)
                    } else {
                        GotoEvent::new(entity, RadiusGoal::new(AVec3::from(p.center()), tolerance), opts)
                    };
                    goto.write(ev);
                    flags.nav_requested = Some(tick.0);
                }
                Command::StopNav => {
                    stop_nav.write(StopPathfindingEvent { entity, force: false });
                    flags.nav_requested = None;
                }
                // Spec: specs/B3-inventory-crafting.md §4.1 (windows are fork events; the fork
                // refuses clicks a vanilla client couldn't make)
                Command::OpenInventory => commands.trigger(OpenInventoryEvent { entity }),
                Command::CloseWindow => commands.trigger(CloseContainerEvent { entity, id: inv.id }),
                Command::ClickSlot { slot, click } => {
                    commands.trigger(ContainerClickEvent { entity, window_id: inv.id, operation: click_op(slot, click) });
                }
            }
        }
    }
}

