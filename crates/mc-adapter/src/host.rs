//! The per-tick host: `decide` runs every bot's `Brain` against a borrowed view and buffers its
//! commands; `apply` (chained right after, still in GameTick and before physics) turns them into
//! component writes. No `Update`-schedule messages for input, so input lands in the same tick
//! (spike §3.2). Goto/stop-nav are the exception: Azalea's pathfinder only takes messages.

use azalea::{
    BlockPos as ABlockPos, ItemInUse, Vec3 as AVec3,
    attack::{AttackQueued, AttackStrengthScale},
    core::{entity_id::MinecraftEntityId, tick::GameTick},
    entity::{
        Dead, HasClientLoaded, Jumping, LocalEntity, LookDirection, Physics, Position,
        inventory::Inventory,
        metadata::{AbstractLivingUsingItem, Health},
    },
    interact::{BlockStatePredictionHandler, handle_start_use_item_queued},
    inventory::ensure_has_sent_carried_item,
    mining::MiningSystems,
    local_player::{Hunger, WorldHolder},
    packet::game::SendGamePacketEvent,
    pathfinder::{
        ExecutingPath, GotoEvent, Pathfinder, PathfinderOpts, StopPathfindingEvent,
        goals::{BlockPosGoal, RadiusGoal},
    },
    physics::{PhysicsSystems, client_movement::{ClientMovementState, WalkDirection}},
    protocol::packets::game::{
        ServerboundPlayerAction, ServerboundUseItem, s_interact::InteractionHand,
        s_player_action::Action,
    },
    world::WorldName,
};
use azalea::app::{App, Plugin};
use azalea::ecs::prelude::*;
use azalea::entity::indexing::EntityIdIndex;
use null_bot_api::{Brain, Command, Hand, Skill, SkillStatus};

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
                .before(ensure_has_sent_carried_item)
                .before(handle_start_use_item_queued),
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
            Option<&AbstractLivingUsingItem>,
            Option<&AttackStrengthScale>,
            &Inventory,
            &WorldHolder,
            &WorldName,
            &Pathfinder,
            Option<&ExecutingPath>,
            Option<&HostFlags>,
        ),
        (With<LocalEntity>, With<HasClientLoaded>, Without<Dead>),
    >,
    others: OthersQuery,
    mut ended: MessageWriter<SkillEnded>,
) {
    tick.0 += 1;
    let now = tick.0;
    for (entity, mut bb, position, physics, look, health, hunger, using, atk, inv, world, wname, pf, exec, flags) in
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
    )>,
    mut goto: MessageWriter<GotoEvent>,
    mut stop_nav: MessageWriter<StopPathfindingEvent>,
) {
    for (entity, mut bb, mut flags, mut mv, mut look, mut jumping, mut inv, mut pred, ids) in &mut bots {
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
                    let s = s.min(8);
                    // A slot change ends any use, as in vanilla.
                    if inv.selected_hotbar_slot != s {
                        commands.entity(entity).remove::<ItemInUse>();
                    }
                    inv.selected_hotbar_slot = s;
                }
                Command::Attack(id) => {
                    if let Some(target) = ids.get_by_minecraft_entity(MinecraftEntityId(id.0 as i32)) {
                        commands.entity(entity).insert(AttackQueued { target });
                    }
                }
                Command::UseItem(h) => {
                    // Raw UseItem so it works with a block under the crosshair (spike §6.3).
                    commands.trigger(SendGamePacketEvent::new(
                        entity,
                        ServerboundUseItem {
                            hand: hand(h),
                            seq: pred.start_predicting(),
                            x_rot: look.x_rot(),
                            y_rot: look.y_rot(),
                        },
                    ));
                    // Slows our own physics to vanilla's using-item speed.
                    commands.entity(entity).insert(ItemInUse);
                }
                Command::ReleaseUse => {
                    release(&mut commands, entity, Action::ReleaseUseItem);
                    commands.entity(entity).remove::<ItemInUse>();
                }
                Command::SwapOffhand => {
                    release(&mut commands, entity, Action::SwapItemWithOffhand);
                    commands.entity(entity).remove::<ItemInUse>();
                }
                Command::Stop => {
                    mv.move_direction = WalkDirection::None;
                    mv.trying_to_sprint = false;
                    mv.trying_to_crouch = false;
                    stop_nav.write(StopPathfindingEvent { entity, force: true });
                    flags.nav_requested = None;
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
            }
        }
    }
}

fn release(commands: &mut Commands, entity: Entity, action: Action) {
    commands.trigger(SendGamePacketEvent::new(
        entity,
        ServerboundPlayerAction {
            action,
            pos: ABlockPos::new(0, 0, 0),
            direction: azalea::core::direction::Direction::Down,
            seq: 0,
        },
    ));
}
