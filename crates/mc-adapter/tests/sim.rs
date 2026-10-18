//! Simulator tests: one per `BotCommands` variant, driven through the real host in Azalea's
//! scripted-packet client simulation. A test pushes `Command`s into a queue; a skill replays them
//! on the next host tick, and the test checks the packets and components that result.

use std::sync::{Arc, Mutex};

use azalea::{
    core::{
        entity_id::MinecraftEntityId,
        position::{BlockPos, ChunkPos, Vec3},
    },
    entity::{LookDirection, Position, inventory::Inventory},
    pathfinder::{Pathfinder, PathfinderPlugin},
    protocol::{
        common::movements::{PositionMoveRotation, RelativeMovements},
        packets::{
            ConnectionProtocol,
            game::{
                ClientboundBlockUpdate, ClientboundContainerSetSlot, ClientboundDamageEvent,
                ClientboundPlayerPosition, ClientboundSetChunkCacheCenter, ClientboundSetHealth,
                c_damage_event::OptionalEntityId,
                ServerboundGamePacket, s_interact::ActionType, s_player_action::Action,
            },
        },
    },
    registry::builtin::{BlockKind, EntityKind},
    test_utils::prelude::*,
};
use mc_adapter::{BotBrain, HostPlugin};
use null_bot_api::{
    BotCommands, BotView, Brain, Command, EntityId, Hand, NavStatus, Skill, SkillStatus,
};

type Queue = Arc<Mutex<Vec<Command>>>;

#[derive(Default, Clone, Debug)]
struct Seen {
    ticks: Vec<u64>,
    position: Option<null_bot_api::Vec3>,
    on_ground: bool,
    selected_slot: u8,
    health: f32,
    food: u32,
    nav: Vec<NavStatus>,
    entity_ids: Vec<u32>,
    solid_below: bool,
    solid_at_feet: bool,
    using: Option<null_bot_api::UsingItem>,
    attackers: Vec<null_bot_api::Attacker>,
}

/// Replays queued commands and records what the view showed that tick.
struct Replay {
    queue: Queue,
    seen: Arc<Mutex<Seen>>,
}

impl Skill for Replay {
    fn id(&self) -> &str {
        "replay"
    }
    fn step(&mut self, v: &dyn BotView, c: &mut dyn BotCommands) -> SkillStatus {
        {
            let mut s = self.seen.lock().unwrap();
            s.ticks.push(v.tick());
            s.position = Some(v.position());
            s.on_ground = v.on_ground();
            s.selected_slot = v.selected_slot();
            s.health = v.health();
            s.food = v.food();
            s.nav.push(v.nav_status());
            s.entity_ids = v.entities().iter().map(|e| e.id.0).collect();
            let p = v.position().block_pos();
            s.solid_below = v.is_solid(null_bot_api::BlockPos::new(p.x, p.y - 1, p.z));
            s.solid_at_feet = v.is_solid(p);
            s.using = v.using_item();
            s.attackers = v.recent_attackers().to_vec();
        }
        for cmd in std::mem::take(&mut *self.queue.lock().unwrap()) {
            match cmd {
                Command::Walk { forward, strafe } => c.walk(forward, strafe),
                Command::Sprint(b) => c.sprint(b),
                Command::Jump => c.jump(),
                Command::Sneak(b) => c.sneak(b),
                Command::Look { yaw, pitch } => c.look(yaw, pitch),
                Command::SelectSlot(s) => c.select_slot(s),
                Command::Attack(e) => c.attack(e),
                Command::UseItem(h) => c.use_item(h),
                Command::ReleaseUse => c.release_use(),
                Command::SwapOffhand => c.swap_offhand(),
                Command::Stop => c.stop(),
                Command::Goto { target, tolerance } => c.goto(target, tolerance),
                Command::StopNav => c.stop_nav(),
                Command::BreakBlock(p) => c.break_block(p),
                Command::CancelBreak => c.cancel_break(),
                Command::PlaceBlock { pos, face, auto_crouch } => c.place_block(pos, face, auto_crouch),
                Command::LookAtBlock { pos, face } => c.look_at_block(pos, face),
                Command::OpenInventory => c.open_inventory(),
                Command::OpenContainer { pos, face } => c.open_container(pos, face),
                Command::CloseWindow => c.close_window(),
                Command::ClickSlot { slot, click } => c.click_slot(slot, click),
            }
        }
        SkillStatus::Running
    }
}

struct Rig {
    sim: Simulation,
    sent: SentPackets,
    queue: Queue,
    seen: Arc<Mutex<Seen>>,
}

impl Rig {
    fn new() -> Rig {
        let mut sim = Simulation::new(ConnectionProtocol::Game);
        let sent = SentPackets::new(&mut sim);
        sim.app.add_plugins(HostPlugin);
        // The host's query needs the `Pathfinder` component, as in the node.
        sim.app.add_plugins((azalea::bot::BotPlugin, PathfinderPlugin));
        // Plugin::finish registers the path executor; the simulation never calls it.
        sim.app.finish();
        sim.app.cleanup();
        sim.receive_packet(default_login_packet());
        sim.tick();
        sim.receive_packet(ClientboundSetChunkCacheCenter { x: 0, z: 0 });
        sim.receive_packet(make_basic_empty_chunk(ChunkPos::new(0, 0), (384 + 64) / 16));
        // A stone floor at y=63 over x,z in 0..12.
        for x in 0..12 {
            for z in 0..12 {
                sim.receive_packet(ClientboundBlockUpdate {
                    pos: BlockPos::new(x, 63, z),
                    block_state: BlockKind::Stone.into(),
                });
            }
        }
        sim.receive_packet(ClientboundPlayerPosition {
            id: 1,
            change: PositionMoveRotation {
                pos: Vec3::new(2.5, 64., 2.5),
                delta: Vec3::ZERO,
                look_direction: LookDirection::default(),
            },
            relative: RelativeMovements::all_absolute(),
        });
        for _ in 0..3 {
            sim.tick();
        }
        let queue: Queue = Default::default();
        let seen: Arc<Mutex<Seen>> = Default::default();
        let skill = Replay { queue: queue.clone(), seen: seen.clone() };
        let e = sim.entity;
        sim.app
            .world_mut()
            .entity_mut(e)
            .insert(BotBrain::new(Brain::new(), Some(Box::new(skill))));
        // Let the host run once with nothing queued, then forget the setup packets.
        sim.tick();
        sent.clear();
        Rig { sim, sent, queue, seen }
    }

    fn push(&self, c: Command) {
        self.queue.lock().unwrap().push(c);
    }

    /// Issue `c` and run the tick that replays it, and the one that applies it.
    /// Returns the packets sent in those two ticks.
    fn run(&mut self, c: Command) -> Vec<ServerboundGamePacket> {
        self.push(c);
        self.sent.clear();
        // Tick A: the skill's step issues the command and `apply` writes it, before physics.
        self.sim.tick();
        let mut out = drain(&self.sent);
        // Tick B: the next host tick sees the result in its view.
        self.sim.tick();
        out.extend(drain(&self.sent));
        out
    }

    fn pos(&self) -> Vec3 {
        **self.sim.app.world().get::<Position>(self.sim.entity).unwrap()
    }
}

fn drain(s: &SentPackets) -> Vec<ServerboundGamePacket> {
    std::iter::from_fn(|| s.next()).collect()
}

fn input(p: &ServerboundGamePacket) -> Option<&azalea::protocol::packets::game::ServerboundPlayerInput> {
    if let ServerboundGamePacket::PlayerInput(i) = p { Some(i) } else { None }
}

#[test]
fn view_reads_basics() {
    let _lock = init();
    let rig = Rig::new();
    let s = rig.seen.lock().unwrap().clone();
    let p = s.position.unwrap();
    assert!((p.x - 2.5).abs() < 1e-6 && (p.z - 2.5).abs() < 1e-6, "{p:?}");
    assert!(s.on_ground);
    assert!(s.solid_below, "stone floor at y=63");
    assert!(!s.solid_at_feet);
    assert_eq!(s.nav.last(), Some(&NavStatus::Idle));
    assert!(s.health > 0.0 && s.food == 20);
    assert!(s.ticks.windows(2).all(|w| w[1] == w[0] + 1), "host clock counts every tick: {:?}", s.ticks);
}

#[test]
fn walk_sends_input_in_the_same_tick_and_moves() {
    let _lock = init();
    let mut rig = Rig::new();
    let z0 = rig.pos().z;
    rig.push(Command::Walk { forward: 1, strafe: 0 });
    rig.sent.clear();
    rig.sim.tick(); // one tick: the host writes, physics runs, input packet goes out
    let sent = drain(&rig.sent);
    assert!(
        sent.iter().any(|p| input(p).is_some_and(|i| i.forward)),
        "forward input in the same tick, got {sent:?}"
    );
    for _ in 0..10 {
        rig.sim.tick();
    }
    assert!(rig.pos().z > z0 + 0.3, "yaw 0 walks toward +Z: {} -> {}", z0, rig.pos().z);
    // Diagonal and backward mappings.
    let sent = rig.run(Command::Walk { forward: -1, strafe: 1 });
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| i.backward && i.right && !i.forward)));
    let sent = rig.run(Command::Walk { forward: 0, strafe: 0 });
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| !i.forward && !i.backward && !i.left && !i.right)));
}

#[test]
fn sprint_sets_the_sprint_input() {
    let _lock = init();
    let mut rig = Rig::new();
    rig.push(Command::Walk { forward: 1, strafe: 0 });
    rig.push(Command::Sprint(true));
    rig.sent.clear();
    rig.sim.tick();
    let sent = drain(&rig.sent);
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| i.forward && i.sprint)), "{sent:?}");
    let sent = rig.run(Command::Sprint(false));
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| !i.sprint)));
}

#[test]
fn sneak_sets_the_shift_input() {
    let _lock = init();
    let mut rig = Rig::new();
    let sent = rig.run(Command::Sneak(true));
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| i.shift)), "{sent:?}");
    let sent = rig.run(Command::Sneak(false));
    assert!(sent.iter().any(|p| input(p).is_some_and(|i| !i.shift)));
}

#[test]
fn jump_lasts_one_tick() {
    let _lock = init();
    let mut rig = Rig::new();
    rig.push(Command::Jump);
    rig.sent.clear();
    rig.sim.tick();
    let first = drain(&rig.sent);
    assert!(first.iter().any(|p| input(p).is_some_and(|i| i.jump)), "{first:?}");
    rig.sim.tick();
    let second = drain(&rig.sent);
    assert!(second.iter().any(|p| input(p).is_some_and(|i| !i.jump)), "released: {second:?}");
    assert!(rig.pos().y > 64.0, "left the ground: {}", rig.pos().y);
}

#[test]
fn look_sets_the_direction() {
    let _lock = init();
    let mut rig = Rig::new();
    let sent = rig.run(Command::Look { yaw: 90.0, pitch: 20.0 });
    let look = *rig.sim.app.world().get::<LookDirection>(rig.sim.entity).unwrap();
    assert!((look.y_rot() - 90.0).abs() < 1.0 && (look.x_rot() - 20.0).abs() < 1.0, "{look:?}");
    assert!(
        sent.iter().any(|p| matches!(p, ServerboundGamePacket::MovePlayerRot(_) | ServerboundGamePacket::MovePlayerPosRot(_))),
        "rotation packet: {sent:?}"
    );
}

#[test]
fn select_slot_writes_the_slot_and_sends_it() {
    let _lock = init();
    let mut rig = Rig::new();
    let sent = rig.run(Command::SelectSlot(4));
    assert_eq!(rig.sim.app.world().get::<Inventory>(rig.sim.entity).unwrap().selected_hotbar_slot, 4);
    assert!(sent.iter().any(|p| matches!(p, ServerboundGamePacket::SetCarriedItem(s) if s.slot == 4)), "{sent:?}");
    assert_eq!(rig.seen.lock().unwrap().selected_slot, 4, "view reflects it next tick");
}

#[test]
fn attack_queues_an_interact_packet() {
    let _lock = init();
    let mut rig = Rig::new();
    rig.sim.receive_packet(make_basic_add_entity(EntityKind::Zombie, 123, (4.5, 64., 2.5)));
    rig.sim.tick();
    rig.sim.tick();
    assert!(rig.seen.lock().unwrap().entity_ids.contains(&123), "entity visible in the view");
    let sent = rig.run(Command::Attack(EntityId(123)));
    assert!(
        sent.iter().any(|p| matches!(p, ServerboundGamePacket::Interact(i)
            if i.entity_id == MinecraftEntityId(123) && matches!(i.action, ActionType::Attack))),
        "{sent:?}"
    );
}

/// Bread in hotbar slot 0 and a hungry player, so a use starts.
fn give_bread(rig: &mut Rig, food: u32) {
    rig.sim.receive_packet(ClientboundContainerSetSlot {
        container_id: 0,
        state_id: 1,
        slot: *azalea::inventory::Player::HOTBAR_SLOTS.start() as u16,
        item_stack: azalea::inventory::ItemStack::from(azalea::registry::builtin::ItemKind::Bread),
    });
    rig.sim.receive_packet(ClientboundSetHealth { health: 20., food, saturation: 0. });
    rig.sim.tick();
}

#[test]
fn use_item_sends_use_item_and_the_view_reports_hand_and_ticks() {
    let _lock = init();
    let mut rig = Rig::new();
    give_bread(&mut rig, 10);
    let sent = rig.run(Command::UseItem(Hand::Main));
    assert!(sent.iter().any(|p| matches!(p, ServerboundGamePacket::UseItem(_))), "{sent:?}");
    let u = rig.seen.lock().unwrap().using.expect("view reports the use");
    assert_eq!(u.hand, Hand::Main);
    assert!(u.ticks >= 1, "{u:?}");
    // A slot change ends it.
    rig.run(Command::SelectSlot(1));
    rig.sim.tick();
    assert_eq!(rig.seen.lock().unwrap().using, None);
}

#[test]
fn damage_event_fills_recent_attackers_on_the_host_clock() {
    let _lock = init();
    let mut rig = Rig::new();
    let me = rig.sim.minecraft_entity_id();
    rig.sim.receive_packet(ClientboundDamageEvent {
        entity_id: me,
        source_type_id: 1,
        source_cause_id: OptionalEntityId(Some(42)),
        source_direct_id: OptionalEntityId(Some(42)),
        source_position: None,
    });
    rig.sim.tick();
    let seen = rig.seen.lock().unwrap().clone();
    let a = seen.attackers.first().expect("attacker listed");
    assert_eq!(a.entity, EntityId(42));
    assert_eq!(Some(&a.tick), seen.ticks.last(), "stamped with the tick the bot first saw it");
}

#[test]
fn release_use_and_swap_offhand_send_player_actions() {
    let _lock = init();
    let mut rig = Rig::new();
    give_bread(&mut rig, 10);
    rig.run(Command::UseItem(Hand::Main));
    let sent = rig.run(Command::ReleaseUse);
    assert!(sent.iter().any(|p| matches!(p, ServerboundGamePacket::PlayerAction(a) if a.action == Action::ReleaseUseItem)), "{sent:?}");
    let sent = rig.run(Command::SwapOffhand);
    assert!(sent.iter().any(|p| matches!(p, ServerboundGamePacket::PlayerAction(a) if a.action == Action::SwapItemWithOffhand)), "{sent:?}");
}

#[test]
fn stop_clears_all_input() {
    let _lock = init();
    let mut rig = Rig::new();
    rig.push(Command::Walk { forward: 1, strafe: 0 });
    rig.push(Command::Sprint(true));
    rig.push(Command::Sneak(true));
    rig.sim.tick();
    let sent = rig.run(Command::Stop);
    assert!(
        sent.iter().any(|p| input(p).is_some_and(|i| !i.forward && !i.sprint && !i.shift)),
        "{sent:?}"
    );
}

#[test]
fn goto_walks_to_the_target_and_stop_nav_cancels() {
    // The test logger panics on WARN, and a debug build of the pathfinder warns once.
    let _lock = init_with_level(tracing::Level::ERROR);
    let mut rig = Rig::new();
    let target = null_bot_api::BlockPos::new(9, 64, 2);
    rig.push(Command::Goto { target, tolerance: 0.0 });
    // The goto is a message for Azalea's pathfinder, so it starts one Update later; the view
    // must show `Calculating` meanwhile instead of a misleading `Idle`.
    rig.sim.tick();
    rig.sim.tick();
    assert_eq!(rig.seen.lock().unwrap().nav.last(), Some(&NavStatus::Calculating));
    let mut arrived = false;
    for _ in 0..400 {
        std::thread::sleep(std::time::Duration::from_millis(2)); // A* runs on a task pool
        rig.sim.tick();
        let p = rig.pos();
        if (p.x - 9.5).abs() < 0.8 && (p.z - 2.5).abs() < 0.8 {
            arrived = true;
            break;
        }
    }
    assert!(arrived, "bot did not reach the target, at {:?}", rig.pos());
    let nav = rig.seen.lock().unwrap().nav.clone();
    assert!(nav.contains(&NavStatus::Following), "view showed path following: {nav:?}");
    for _ in 0..5 {
        rig.sim.tick();
    }
    assert_eq!(rig.seen.lock().unwrap().nav.last(), Some(&NavStatus::Idle), "idle after arrival");

    // stop_nav: start a new goto, then cancel it.
    rig.push(Command::Goto { target: null_bot_api::BlockPos::new(2, 64, 9), tolerance: 0.0 });
    rig.sim.tick();
    rig.sim.tick();
    rig.push(Command::StopNav);
    rig.sim.tick();
    rig.sim.tick();
    rig.sim.tick();
    let goal = rig.sim.app.world().get::<Pathfinder>(rig.sim.entity).unwrap().goal.is_some();
    assert!(!goal, "pathfinder goal cleared");
}

#[test]
fn a_dead_or_unloaded_bot_is_skipped() {
    // No BotBrain, no host work: ticking a bare client must not panic.
    let _lock = init();
    let mut sim = Simulation::new(ConnectionProtocol::Game);
    sim.app.add_plugins(HostPlugin);
    sim.receive_packet(default_login_packet());
    for _ in 0..3 {
        sim.tick();
    }
}
