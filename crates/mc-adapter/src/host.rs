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
        HasClientLoaded, Jumping, LocalEntity, LookDirection, Physics, Position,
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
        BlockActionResults, BreakBlockQueued, LookAtBlockQueued, PlaceBlockQueued, StopMiningQueued, UseBlockQueued,
        handle_break_block_queued, handle_look_at_block_queued,
    },
    entity_actions::{
        EntityUseResults, LookAtEntityQueued, UseEntityQueued, handle_look_at_entity_queued,
        handle_use_entity_queued,
    },
    mining::MiningSystems,
    respawn::{DeathScreen, IsHardcore, PerformRespawnEvent, RespawnLog, check_respawn_request},
    tick_counter::TicksConnected,
    local_player::{Hunger, WorldHolder},
    pathfinder::{
        ExecutingPath, GotoEvent, Pathfinder, PathfinderOpts, StopPathfindingEvent,
        goals::{BlockPosGoal, RadiusGoal},
    },
    physics::{PhysicsSystems, client_movement::{ClientMovementState, WalkDirection}},
    protocol::packets::game::s_interact::InteractionHand,
    world::WorldName,
};
use std::cell::RefCell;

use azalea::app::{App, Plugin};
use azalea::ecs::prelude::*;
use azalea::entity::indexing::EntityIdIndex;
use null_bot_api::{
    ActionId, AfterDeath, Brain, BotView, Command, Hand, MailboxEvent, Owner, Scope, Skill, SkillStatus,
    Target,
};

use crate::actions::{self, Fork, Kind, Tracker};
use crate::blocks;
use crate::view::{AzaleaView, OthersQuery, ViewParts};

/// A bot's decision state: reflexes, the active skill and the commands it just issued.
#[derive(Component)]
#[require(HostFlags)]
pub struct BotBrain {
    pub brain: Brain,
    pub(crate) skill: Option<Box<dyn Skill>>,
    /// The commands the last step issued, with the id of each that has a result.
    pending: Vec<(Option<ActionId>, Command)>,
    /// Deaths already announced by this brain (`BotDied`).
    died_seen: u32,
    /// The result mailbox and the tracks that feed it (AD-F5.4).
    pub(crate) actions: Tracker,
}

impl BotBrain {
    pub fn new(brain: Brain, skill: Option<Box<dyn Skill>>) -> Self {
        Self { brain, skill, pending: Vec::new(), died_seen: 0, actions: Tracker::default() }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (a skill's results go when it is replaced)
    /// Replaces the skill. The old one's results are released.
    pub fn set_skill(&mut self, skill: Option<Box<dyn Skill>>) {
        self.actions.mailbox.end_owner(Owner::Skill);
        self.skill = skill;
    }

    pub fn has_skill(&self) -> bool {
        self.skill.is_some()
    }

    // Spec: specs/F1-swarm-runtime.md §4.4, specs/B6-placement-breaking.md §4.6 (a kept skill and
    // its results cross the death)
    /// Takes the skill of the brain that is being replaced, with its results (a skill that chose
    /// `Keep`). Does nothing when the old brain has none.
    pub(crate) fn inherit_skill(&mut self, old: &mut BotBrain) {
        if let Some(kept) = old.skill.take() {
            self.actions.mailbox.end_owner(Owner::Skill);
            self.skill = Some(kept);
            self.actions.adopt_skill_from(&mut old.actions);
        }
    }

    /// Ends the running skill and releases its results.
    fn end_skill(&mut self) -> Option<Box<dyn Skill>> {
        self.actions.mailbox.end_owner(Owner::Skill);
        self.brain.end_job();
        self.skill.take()
    }
}

/// The reason of the `SkillStatus::Failed` that a death writes (F1 §4.4: a job does not survive
/// a death).
pub const SKILL_DIED: &str = "died";

/// The reason of the `SkillStatus::Failed` that the leak guard writes (AD-F5.4): the skill held
/// 4,096 unreleased finished results.
pub const SKILL_LEAK: &str = "result leak";

/// Host-side bookkeeping that is not the decision code's business.
#[derive(Component, Default)]
pub struct HostFlags {
    /// A jump was set last tick and must be released this tick (one physics step).
    jump_pulse: bool,
    /// Host tick of the last goto, until Azalea's pathfinder has picked it up.
    nav_requested: Option<u64>,
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

/// Written when the leak guard turns a reflex off: it held 4,096 unreleased finished results
/// (AD-F5.4). A reflex cannot end, so it is switched off at bot scope.
#[derive(Message)]
pub struct ReflexDisabled {
    pub entity: Entity,
    pub reflex: &'static str,
    pub tick: u64,
}

/// Written the first host tick that sees a death, with the death record (F1 §4.4).
#[derive(Message)]
pub struct BotDied {
    pub entity: Entity,
    pub tick: u64,
    pub death: Option<null_bot_api::Death>,
}

pub struct HostPlugin;

impl Plugin for HostPlugin {
    fn build(&self, app: &mut App) {
        // The pathfinder messages are registered here too (idempotent) so the host also runs
        // in setups without the pathfinder plugin.
        app.init_resource::<HostTick>()
            .add_message::<SkillEnded>()
            .add_message::<BotDied>()
            .add_message::<ReflexDisabled>()
            .add_message::<GotoEvent>()
            .add_message::<StopPathfindingEvent>()
            .add_systems(
            GameTick,
            // Before everything that reads what `apply` writes: physics, mining, use-item and the
            // carried-item sync (the ambiguity logger flagged each of these).
            (advance_clock, refresh_actions, decide, apply)
                .chain()
                .before(PhysicsSystems)
                .before(MiningSystems)
                .before(handle_break_block_queued)
                .before(handle_look_at_block_queued)
                .before(handle_look_at_entity_queued)
                .before(handle_use_entity_queued)
                .before(ensure_has_sent_carried_item)
                .before(handle_start_use_item_queued)
                .before(handle_use_item_queued)
                .before(hold_still_while_screen_open)
                .before(age_recent_attackers)
                // A dead bot's commands are refused by the fork's guards, which run after us.
                .before(azalea::respawn::refuse_input_while_dead)
                .before(azalea::block_actions::refuse_block_actions_while_dead)
                .before(azalea::entity_actions::refuse_entity_actions_while_dead)
                // The view reads the fork's tick count and death record, so they settle first.
                .after(azalea::tick_counter::increment_counter)
                .after(azalea::respawn::tick_death_screen),
        );
    }
}

/// One step's output. Each command is stamped with the owner `Brain::tick` set (Spec:
/// specs/B6-placement-breaking.md §4.6); the mailbox is touched after the step, when the view is
/// no longer borrowed.
struct CommandBuffer<'a> {
    owner: Owner,
    cmds: Vec<(Option<ActionId>, Command)>,
    /// Shared with the view, so a skill reading the id it just got sees it pending, not expired.
    opened: &'a RefCell<Vec<(ActionId, Owner)>>,
    released: Vec<(Owner, ActionId)>,
    expected: Vec<(Owner, usize)>,
}

impl<'a> CommandBuffer<'a> {
    fn new(opened: &'a RefCell<Vec<(ActionId, Owner)>>) -> Self {
        Self { owner: Owner::Skill, cmds: Vec::new(), opened, released: Vec::new(), expected: Vec::new() }
    }
}

impl CommandBuffer<'_> {
    fn push(&mut self, c: Command) {
        self.cmds.push((None, c));
    }
    /// A command with a result: it gets an id now, and an entry in the owner's mailbox after the step.
    fn push_id(&mut self, c: Command) -> ActionId {
        let id = ActionId::fresh();
        self.cmds.push((Some(id), c));
        self.opened.borrow_mut().push((id, self.owner));
        id
    }
}

impl null_bot_api::BotCommands for CommandBuffer<'_> {
    fn set_owner(&mut self, owner: Owner) {
        self.owner = owner;
    }
    fn release(&mut self, id: ActionId) {
        self.released.push((self.owner, id));
    }
    fn expect_results(&mut self, n: usize) {
        self.expected.push((self.owner, n));
    }
    fn walk(&mut self, forward: i8, strafe: i8) {
        self.push(Command::Walk { forward: forward.clamp(-1, 1), strafe: strafe.clamp(-1, 1) });
    }
    fn sprint(&mut self, on: bool) {
        self.push(Command::Sprint(on));
    }
    fn jump(&mut self) {
        self.push(Command::Jump);
    }
    fn sneak(&mut self, on: bool) {
        self.push(Command::Sneak(on));
    }
    fn look(&mut self, yaw: f32, pitch: f32) {
        self.push(Command::Look { yaw, pitch });
    }
    fn select_slot(&mut self, slot: u8) {
        self.push(Command::SelectSlot(slot));
    }
    fn attack(&mut self, target: null_bot_api::EntityId) {
        self.push(Command::Attack(target));
    }
    fn use_item(&mut self, hand: Hand) {
        self.push(Command::UseItem(hand));
    }
    fn release_use(&mut self) {
        self.push(Command::ReleaseUse);
    }
    fn swap_offhand(&mut self) {
        self.push(Command::SwapOffhand);
    }
    fn stop(&mut self) {
        self.push(Command::Stop);
    }
    fn goto(&mut self, target: null_bot_api::BlockPos, tolerance: f32) {
        self.push(Command::Goto { target, tolerance });
    }
    fn stop_nav(&mut self) {
        self.push(Command::StopNav);
    }
    // Spec: specs/B6-placement-breaking.md §4.4 (break timing)
    fn break_block(&mut self, pos: null_bot_api::BlockPos) -> ActionId {
        self.push_id(Command::BreakBlock(pos))
    }
    // Spec: specs/B6-placement-breaking.md §4.4 (abort a break)
    fn cancel_break(&mut self) {
        self.push(Command::CancelBreak);
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (place on a chosen face)
    fn place_block(&mut self, pos: null_bot_api::BlockPos, face: null_bot_api::Face, auto_crouch: bool) -> ActionId {
        self.push_id(Command::PlaceBlock { pos, face, auto_crouch })
    }
    // Spec: specs/B6-placement-breaking.md §4.2 (click choice: aim at a visible point)
    fn look_at_block(&mut self, pos: null_bot_api::BlockPos, face: Option<null_bot_api::Face>) {
        self.push(Command::LookAtBlock { pos, face });
    }
    // Spec: specs/B3-inventory-crafting.md §4.1 (open, close and click windows)
    fn open_inventory(&mut self) -> ActionId {
        self.push_id(Command::OpenInventory)
    }
    // Spec: specs/B3-inventory-crafting.md §4.7 (open a container by right-clicking it)
    fn open_container(&mut self, pos: null_bot_api::BlockPos, face: Option<null_bot_api::Face>) -> ActionId {
        self.push_id(Command::OpenContainer { pos, face })
    }
    // Spec: specs/B3-inventory-crafting.md §4.7 (aim at a point of an entity's hitbox)
    fn look_at_entity(&mut self, target: null_bot_api::EntityId) {
        self.push(Command::LookAtEntity(target));
    }
    // Spec: specs/B3-inventory-crafting.md §4.7 (open a container on an entity by right-clicking it)
    fn open_entity(&mut self, target: null_bot_api::EntityId, auto_crouch: bool) -> ActionId {
        self.push_id(Command::OpenEntity { target, auto_crouch })
    }
    fn close_window(&mut self) {
        self.push(Command::CloseWindow);
    }
    fn click_slot(&mut self, slot: Option<u16>, click: null_bot_api::Click) -> ActionId {
        self.push_id(Command::ClickSlot { slot, click })
    }
    // Spec: specs/F1-swarm-runtime.md §4.4 (respawn only as the death screen's button can)
    fn respawn(&mut self) -> ActionId {
        self.push_id(Command::Respawn)
    }
}

// Spec: specs/B6-placement-breaking.md §4.6 (one host tick per game tick, before the refresh)
/// The host clock advances first, so the refresh and the step of one tick agree on the number.
fn advance_clock(mut tick: ResMut<HostTick>) {
    tick.0 += 1;
}

// Spec: specs/B6-placement-breaking.md §4.6 (the fork's records are copied into the mailbox)
#[allow(clippy::type_complexity)]
fn refresh_actions(
    mut commands: Commands,
    tick: Res<HostTick>,
    mut bots: Query<(
        Entity,
        &mut BotBrain,
        Option<&BlockActionResults>,
        Option<&EntityUseResults>,
        Option<&ClickResults>,
        Option<&RespawnLog>,
        Option<&TicksConnected>,
        (Has<BreakBlockQueued>, Has<PlaceBlockQueued>, Has<UseBlockQueued>, Has<UseEntityQueued>),
    )>,
) {
    for (entity, mut bb, blocks, entity_results, clicks, respawn_log, connected, (qb, qp, qu, qe)) in &mut bots {
        let fork = Fork {
            now: tick.0,
            client_tick: connected.map_or(0, |t| t.0),
            blocks,
            entity: entity_results,
            clicks,
            respawn: respawn_log,
            queued: [qb, qp, qu, qe],
        };
        if bb.actions.refresh(&fork).cancel_break {
            commands.entity(entity).insert(StopMiningQueued);
        }
    }
}

#[allow(clippy::type_complexity)]
fn decide(
    tick: Res<HostTick>,
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
                Option<&RecentAttackers>,
                Option<&ServerTags>,
                Option<&OpenScreen>,
                Option<&ClientMovementState>,
                Option<&RespawnLog>,
                Option<&TicksConnected>,
                Option<&DeathScreen>,
            ),
        ),
        (With<LocalEntity>, With<HasClientLoaded>),
    >,
    others: OthersQuery,
    mut ended: MessageWriter<SkillEnded>,
    mut died: MessageWriter<BotDied>,
    mut reflex_off: MessageWriter<ReflexDisabled>,
) {
    let now = tick.0;
    for (entity, mut bb, position, physics, look, health, hunger, using, atk, inv, world, wname, pf, exec, (flags, attackers, tags, screen, movement, respawn_log, connected, death_screen)) in
        &mut bots
    {
        let bb = &mut *bb;
        // Spec: specs/B6-placement-breaking.md §4.6 (warn once over the expected count; the leak
        // guard ends a skill and turns a reflex off)
        for ev in bb.actions.mailbox.take_events() {
            match ev {
                MailboxEvent::OverExpected { owner, finished, expected } => {
                    tracing::warn!(?entity, ?owner, finished, expected, "owner holds more finished results than it expected; release them");
                }
                MailboxEvent::Leak { owner: Owner::Skill } => {
                    tracing::warn!(?entity, "skill leaked results: ending it");
                    if let Some(skill) = bb.end_skill() {
                        ended.write(SkillEnded {
                            entity,
                            skill: skill.id().to_string(),
                            status: SkillStatus::Failed(SKILL_LEAK.to_string()),
                            tick: now,
                        });
                    }
                }
                MailboxEvent::Leak { owner: Owner::Reflex(id) } => {
                    tracing::warn!(?entity, reflex = id, "reflex leaked results: turning it off at bot scope");
                    bb.brain.set(Scope::Bot, Target::Reflex(id.to_string()), false);
                    bb.actions.mailbox.end_owner(Owner::Reflex(id));
                    reflex_off.write(ReflexDisabled { entity, reflex: id, tick: now });
                }
            }
        }
        let nav_pending = flags.and_then(|f| f.nav_requested).is_some_and(|t| now - t <= 2);
        let opened = RefCell::new(Vec::new());
        let view = AzaleaView::new(
            ViewParts {
                entity,
                issued: &opened,
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
                mailbox: &bb.actions.mailbox,
                attackers,
                tags,
                screen_open: screen.is_some(),
                screen_kind: screen.and_then(|s| s.kind),
                sneaking: movement.is_some_and(|m| m.trying_to_crouch),
                respawn_log,
                death_screen,
                client_tick: connected.map_or(0, |t| t.0),
            },
            &others,
        );
        let mut buf = CommandBuffer::new(&opened);
        // Spec: specs/F1-swarm-runtime.md §4.4 (the death tick: the skill chooses End or Keep, the
        // death is reported, reflexes and a kept skill keep their view until the respawn)
        let mut ended_by_death: Option<Box<dyn Skill>> = None;
        if death_screen.is_some() && view.deaths() > bb.died_seen {
            bb.died_seen = view.deaths();
            let death = view.last_death().cloned();
            if let (Some(skill), Some(d)) = (bb.skill.as_mut(), death.as_ref())
                && skill.on_death(&view, d) == AfterDeath::End
            {
                ended_by_death = bb.skill.take();
            }
            died.write(BotDied { entity, tick: now, death });
        }
        // Written out so the trait-object lifetime shortens (Option does not coerce).
        let skill: Option<&mut dyn Skill> = match bb.skill.as_mut() {
            Some(s) => Some(&mut **s),
            None => None,
        };
        let outcome = bb.brain.tick(&view, &mut buf, skill);
        drop(view);
        let CommandBuffer { cmds, released, expected, .. } = buf;
        // The step's commands, ids and releases enter the mailbox before the skill can end, so an
        // ending skill takes its own results with it.
        for (id, owner) in opened.into_inner() {
            bb.actions.open(id, owner);
        }
        for (owner, id) in released {
            bb.actions.mailbox.release(owner, id);
        }
        for (owner, n) in expected {
            bb.actions.mailbox.expect(owner, n);
        }
        bb.pending = cmds;
        if let Some(skill) = ended_by_death {
            bb.actions.mailbox.end_owner(Owner::Skill);
            bb.brain.end_job();
            ended.write(SkillEnded {
                entity,
                skill: skill.id().to_string(),
                status: SkillStatus::Failed(SKILL_DIED.to_string()),
                tick: now,
            });
        }
        if let Some(status @ (SkillStatus::Done | SkillStatus::Failed(_))) = outcome.skill {
            let id = bb.skill.as_ref().map(|s| s.id().to_string()).unwrap_or_default();
            bb.end_skill();
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
        (
            Option<&DeathScreen>,
            Option<&IsHardcore>,
            Option<&RespawnLog>,
            (Has<BreakBlockQueued>, Has<PlaceBlockQueued>, Has<UseBlockQueued>, Has<UseEntityQueued>),
        ),
    )>,
    mut goto: MessageWriter<GotoEvent>,
    mut stop_nav: MessageWriter<StopPathfindingEvent>,
    mut respawn: MessageWriter<PerformRespawnEvent>,
) {
    for (entity, mut bb, mut flags, mut mv, mut look, mut jumping, mut inv, _pred, ids, (screen, hardcore, rlog, (qb, qp, qu, qe))) in &mut bots
    {
        if flags.jump_pulse {
            **jumping = false;
            flags.jump_pulse = false;
        }
        for (id, cmd) in std::mem::take(&mut bb.pending) {
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
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::Break, actions::Target::Block(pos), None, tick.0, qb);
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
                Command::PlaceBlock { pos, face, auto_crouch } => {
                    commands.entity(entity).insert(PlaceBlockQueued {
                        position: blocks::block_pos(pos),
                        direction: blocks::face_to_dir(face),
                        auto_crouch,
                    });
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::Place, actions::Target::Block(pos.offset(face)), None, tick.0, qp);
                    }
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
                Command::OpenInventory => {
                    commands.trigger(OpenInventoryEvent { entity });
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::Inventory, actions::Target::None, None, tick.0, false);
                    }
                }
                Command::OpenContainer { pos, face } => {
                    commands.entity(entity).insert(UseBlockQueued {
                        position: blocks::block_pos(pos),
                        direction: face.map(blocks::face_to_dir),
                    });
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::UseBlock, actions::Target::Block(pos), None, tick.0, qu);
                    }
                }
                // Spec: specs/B3-inventory-crafting.md §4.7 (aim at, then right-click, an entity)
                Command::LookAtEntity(id) => {
                    commands.entity(entity).insert(LookAtEntityQueued { entity_id: id.0 as i32 });
                }
                Command::OpenEntity { target, auto_crouch } => {
                    commands.entity(entity).insert(UseEntityQueued { entity_id: target.0 as i32, auto_crouch });
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::UseEntity, actions::Target::Entity(target), None, tick.0, qe);
                    }
                }
                Command::CloseWindow => commands.trigger(CloseContainerEvent { entity, id: inv.id }),
                // Spec: specs/F1-swarm-runtime.md §4.4, specs/B6-placement-breaking.md §4.6 (a refused
                // request is finished now; one already sent names that request)
                Command::Respawn => {
                    respawn.write(PerformRespawnEvent { entity });
                    if let Some(id) = id {
                        match check_respawn_request(screen, hardcore.is_some_and(|h| h.0)) {
                            Err(azalea::respawn::RespawnRefusal::AlreadySent) => {
                                let sent = rlog.and_then(|l| l.last_respawn.as_ref()).map(|r| r.sent_tick);
                                match sent {
                                    Some(t) => bb.actions.begin_respawn_sent(id, tick.0, t),
                                    None => bb.actions.begin(id, Kind::Respawn, actions::Target::None, None, tick.0, false),
                                }
                            }
                            Err(why) => bb.actions.refuse_respawn(id, why.reason()),
                            Ok(_) => bb.actions.begin(id, Kind::Respawn, actions::Target::None, None, tick.0, false),
                        }
                    }
                }
                Command::ClickSlot { slot, click } => {
                    commands.trigger(ContainerClickEvent { entity, window_id: inv.id, operation: click_op(slot, click) });
                    if let Some(id) = id {
                        bb.actions.begin(id, Kind::Click, actions::Target::None, slot, tick.0, false);
                    }
                }
            }
        }
    }
}

