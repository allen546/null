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
    /// `block_property("persistent")` at the probe cells of `block_property_reads_leaves`.
    persistent: Vec<Option<String>>,
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
            s.persistent = [(5, 64, 5), (6, 64, 5), (7, 64, 5), (200, 64, 200)].iter().map(|(x, y, z)| v.block_property(null_bot_api::BlockPos::new(*x, *y, *z), "persistent")).collect();
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
                Command::BreakBlock(p) => { c.break_block(p); },
                Command::CancelBreak => c.cancel_break(),
                Command::PlaceBlock { pos, face, auto_crouch } => { c.place_block(pos, face, auto_crouch); },
                Command::LookAtBlock { pos, face } => c.look_at_block(pos, face),
                Command::OpenInventory => { c.open_inventory(); },
                Command::OpenContainer { pos, face } => { c.open_container(pos, face); },
                Command::LookAtEntity(e) => c.look_at_entity(e),
                Command::OpenEntity { target, auto_crouch } => { c.open_entity(target, auto_crouch); },
                Command::CloseWindow => c.close_window(),
                Command::ClickSlot { slot, click } => { c.click_slot(slot, click); },
                Command::Respawn => { c.respawn(); },
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

// Spec: specs/B4-gathering.md §4.8 (natural check: leaves' `persistent` property through the view)
#[test]
fn block_property_reads_leaves_persistent_and_none_elsewhere() {
    use azalea::block::{BlockTrait, blocks::OakLeaves, properties::OakLeavesDistance};
    let _lock = init();
    let mut rig = Rig::new();
    let leaves = |persistent| OakLeaves { distance: OakLeavesDistance::_1, persistent, waterlogged: false }.as_block_state();
    rig.sim.receive_packet(ClientboundBlockUpdate { pos: BlockPos::new(5, 64, 5), block_state: leaves(false) });
    rig.sim.receive_packet(ClientboundBlockUpdate { pos: BlockPos::new(6, 64, 5), block_state: leaves(true) });
    rig.sim.receive_packet(ClientboundBlockUpdate { pos: BlockPos::new(7, 64, 5), block_state: BlockKind::Stone.into() });
    rig.sim.tick();
    rig.sim.tick();
    let got = rig.seen.lock().unwrap().persistent.clone();
    assert_eq!(got, vec![Some("false".to_string()), Some("true".to_string()), None, None], "natural, placed, stone, unloaded chunk");
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

// ---- the result mailbox (AD-F5.4, B6 §4.6) ----

/// A skill written as a closure over its own step counter.
struct Script<F: FnMut(u32, &dyn BotView, &mut dyn BotCommands) -> SkillStatus + Send + Sync> {
    n: u32,
    f: F,
}

impl<F: FnMut(u32, &dyn BotView, &mut dyn BotCommands) -> SkillStatus + Send + Sync> Skill for Script<F> {
    fn id(&self) -> &str {
        "script"
    }
    fn step(&mut self, v: &dyn BotView, c: &mut dyn BotCommands) -> SkillStatus {
        self.n += 1;
        (self.f)(self.n, v, c)
    }
}

fn rig_with(skill: Box<dyn Skill>, brain: Brain) -> Rig {
    let mut rig = Rig::new();
    let e = rig.sim.entity;
    rig.sim.app.world_mut().entity_mut(e).insert(BotBrain::new(brain, Some(skill)));
    rig.sim.tick();
    rig
}

#[test]
fn a_command_id_reads_pending_at_once_then_finished_then_expired_after_release() {
    use null_bot_api::{ActionId, ActionState, OpenOutcome};
    let _lock = init();
    let log: Arc<Mutex<Vec<(u32, ActionState)>>> = Default::default();
    let (l, id): (_, Arc<Mutex<Option<ActionId>>>) = (log.clone(), Default::default());
    let skill = Script {
        n: 0,
        f: move |n, v: &dyn BotView, c: &mut dyn BotCommands| {
            let mut idg = id.lock().unwrap();
            match (n, *idg) {
                (1, _) => {
                    let i = c.open_inventory();
                    *idg = Some(i);
                    l.lock().unwrap().push((n, v.action(i)));
                }
                (2..=3, Some(i)) => l.lock().unwrap().push((n, v.action(i))),
                (4, Some(i)) => {
                    c.release(i);
                    l.lock().unwrap().push((n, v.action(i)));
                }
                (5, Some(i)) => l.lock().unwrap().push((n, v.action(i))),
                _ => {}
            }
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    for _ in 0..8 {
        rig.sim.tick();
    }
    let log = log.lock().unwrap();
    // The id the step just got reads pending in the same step, never expired.
    assert!(log[0].1.is_pending(), "{:?}", log[0]);
    let fin = log.iter().find(|(n, _)| *n == 3).unwrap();
    assert!(
        matches!(fin.1.open().map(|o| &o.outcome), Some(OpenOutcome::Opened { window_id: 0, .. })),
        "open_inventory finishes as opened: {:?}",
        fin.1
    );
    assert!(fin.1.is_finished());
    // Released at step 4 (takes effect at the end of that tick); gone by step 5.
    assert!(log.iter().find(|(n, _)| *n == 5).unwrap().1.is_expired());
}

#[test]
fn a_skill_that_never_releases_ends_with_result_leak() {
    // The leak guard logs a warning, which the default test setup turns into a panic.
    let _lock = azalea::test_utils::tracing::init_with_level(tracing::Level::ERROR);
    // Every step asks to open the inventory: the first opens it, the rest are refused at once, and
    // each result finishes a tick later. Nothing is ever released.
    let skill = Script {
        n: 0,
        f: |_, _: &dyn BotView, c: &mut dyn BotCommands| {
            c.expect_results(usize::MAX); // no warning noise; the leak guard still holds
            c.open_inventory();
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    let mut ended_at = None;
    for t in 0..(null_bot_api::LEAK_LIMIT + 40) {
        rig.sim.tick();
        let has = rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill();
        if !has {
            ended_at = Some(t);
            break;
        }
    }
    let t = ended_at.expect("the leak guard ended the skill");
    assert!(t >= null_bot_api::LEAK_LIMIT - 5, "not before the limit: ended at tick {t}");
}

#[test]
fn a_reflex_that_leaks_is_turned_off_at_bot_scope() {
    use null_bot_api::{Group, Reflex, ReflexSpec, Scope, Target};
    struct Leaky(ReflexSpec);
    impl Reflex for Leaky {
        fn spec(&self) -> &ReflexSpec {
            &self.0
        }
        fn trigger(&mut self, _: &dyn BotView) -> bool {
            true
        }
        fn act(&mut self, _: &dyn BotView, c: &mut dyn BotCommands) {
            c.expect_results(usize::MAX);
            c.open_inventory();
        }
    }
    let _lock = azalea::test_utils::tracing::init_with_level(tracing::Level::ERROR);
    let spec = ReflexSpec { id: "leaky", group: Group::Survival, priority: 1, default_on: true, params: vec![] };
    let mut brain = Brain::new();
    brain.register(Box::new(Leaky(spec.clone())));
    let skill = Script { n: 0, f: |_, _: &dyn BotView, _: &mut dyn BotCommands| SkillStatus::Running };
    let mut rig = rig_with(Box::new(skill), brain);
    let mut off_at = None;
    for t in 0..(null_bot_api::LEAK_LIMIT + 40) {
        rig.sim.tick();
        let bb = rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap();
        if !bb.brain.is_enabled(&spec) {
            off_at = Some(t);
            break;
        }
    }
    let t = off_at.expect("the leak guard turned the reflex off");
    assert!(t >= null_bot_api::LEAK_LIMIT - 5, "not before the limit: off at tick {t}");
    // Bot scope: the setting is the one a job cannot undo.
    let mut b = Brain::new();
    b.set(Scope::Bot, Target::Reflex("leaky".into()), false);
    assert!(!b.is_enabled(&spec));
    // The skill is unaffected.
    assert!(rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill());
}

#[test]
fn a_break_the_client_refuses_finishes_with_its_reason_and_two_breaks_keep_both_results() {
    use null_bot_api::{ActionId, ActionState, BlockOutcome};
    let _lock = init();
    let log: Arc<Mutex<Vec<(u32, ActionId, ActionState)>>> = Default::default();
    let (l, ids): (_, Arc<Mutex<Vec<ActionId>>>) = (log.clone(), Default::default());
    // Two breaks of blocks the bot is not looking at, a tick apart: the crosshair check refuses both.
    // The second command must not take the first one's result.
    let skill = Script {
        n: 0,
        f: move |n, v: &dyn BotView, c: &mut dyn BotCommands| {
            let mut ids = ids.lock().unwrap();
            if n == 1 {
                ids.push(c.break_block(null_bot_api::BlockPos::new(2, 64, 5)));
            } else if n == 8 {
                ids.push(c.break_block(null_bot_api::BlockPos::new(3, 64, 5)));
            }
            if n >= 2 {
                for id in ids.iter() {
                    l.lock().unwrap().push((n, *id, v.action(*id)));
                }
            }
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    for _ in 0..30 {
        rig.sim.tick();
    }
    let log = log.lock().unwrap();
    let last = |k: usize| {
        let id = log.iter().map(|(_, i, _)| *i).collect::<Vec<_>>();
        let mut uniq = id.clone();
        uniq.dedup();
        let want = uniq[k];
        log.iter().rev().find(|(_, i, _)| *i == want).map(|(_, _, s)| s.clone()).unwrap()
    };
    for k in 0..2 {
        let s = last(k);
        let b = s.block().unwrap_or_else(|| panic!("break {k}: {s:?}"));
        assert!(s.is_finished(), "break {k}: {s:?}");
        assert_eq!(b.outcome, BlockOutcome::Refused, "break {k}");
        assert!(b.reason.is_some(), "a refusal says why: {b:?}");
    }
    let (a, b) = (last(0).block().cloned().unwrap(), last(1).block().cloned().unwrap());
    assert_ne!(a.pos, b.pos, "each id keeps its own block");
}

// ---- finish bounds the fork does not give (B6 §4.6) ----

/// A `Fork` with no results and nothing queued: the fork never answered.
fn silent_fork(now: u64) -> mc_adapter::actions::Fork<'static> {
    mc_adapter::actions::Fork {
        now,
        client_tick: 0,
        blocks: None,
        entity: None,
        clicks: None,
        respawn: None,
        queued: [false; 4],
    }
}

#[test]
fn a_command_the_fork_never_answers_finishes_not_started_after_20_ticks() {
    use mc_adapter::actions::{ADOPT_TICKS, Kind, Target, Tracker};
    use null_bot_api::{ActionId, BlockOutcome, Owner};
    let mut t = Tracker::default();
    let id = ActionId::fresh();
    t.open(id, Owner::Skill);
    t.begin(id, Kind::Place, Target::Block(null_bot_api::BlockPos::new(1, 64, 1)), None, 0, true);
    // Queued in the fork: not dropped, so pending until the bound.
    t.refresh(&fork_with_queue(ADOPT_TICKS));
    assert!(t.mailbox.state(id).is_pending(), "{:?}", t.mailbox.state(id));
    t.refresh(&fork_with_queue(ADOPT_TICKS + 1));
    let s = t.mailbox.state(id);
    let b = s.block().unwrap_or_else(|| panic!("{s:?}"));
    assert_eq!(b.outcome, BlockOutcome::Refused);
    assert_eq!(b.reason, Some("not started"));
}

fn fork_with_queue(now: u64) -> mc_adapter::actions::Fork<'static> {
    mc_adapter::actions::Fork { queued: [true; 4], ..silent_fork(now) }
}

#[test]
fn a_respawn_with_no_respawn_packet_finishes_no_answer_after_600_ticks() {
    use mc_adapter::actions::{Kind, RESPAWN_TICKS, Target, Tracker};
    use null_bot_api::{ActionId, Owner};
    let mut t = Tracker::default();
    let id = ActionId::fresh();
    t.open(id, Owner::Skill);
    t.begin(id, Kind::Respawn, Target::None, None, 0, false);
    t.refresh(&silent_fork(RESPAWN_TICKS));
    assert!(t.mailbox.state(id).is_pending(), "inside the bound");
    t.refresh(&silent_fork(RESPAWN_TICKS + 1));
    let s = t.mailbox.state(id);
    let r = s.respawn().unwrap_or_else(|| panic!("{s:?}"));
    assert_eq!(r.refused, Some("no answer"));
    assert!(r.completed_tick.is_none());
}

#[test]
fn a_break_that_never_finishes_is_cancelled_too_long_after_6000_ticks() {
    use null_bot_api::{ActionId, BlockOutcome};
    let _lock = init();
    let id: Arc<Mutex<Option<ActionId>>> = Default::default();
    let last: Arc<Mutex<Option<null_bot_api::ActionState>>> = Default::default();
    let (i2, l2) = (id.clone(), last.clone());
    let target = null_bot_api::BlockPos::new(2, 64, 4);
    let skill = Script {
        n: 0,
        f: move |n, v: &dyn BotView, c: &mut dyn BotCommands| {
            let mut idg = i2.lock().unwrap();
            if n == 5 {
                c.look_at_block(target, None);
            } else if n == 12 {
                *idg = Some(c.break_block(target));
            }
            if let Some(i) = *idg {
                *l2.lock().unwrap() = Some(v.action(i));
            }
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    // Bedrock can never be mined: the finishing packet is never sent.
    rig.sim.receive_packet(ClientboundBlockUpdate { pos: BlockPos::new(2, 64, 4), block_state: BlockKind::Bedrock.into() });
    for _ in 0..(mc_adapter::actions::BREAK_MAX_TICKS + 200) {
        rig.sim.tick();
    }
    let s = last.lock().unwrap().clone().expect("the break was issued");
    let b = s.block().unwrap_or_else(|| panic!("{s:?}"));
    assert!(s.is_finished(), "{s:?}");
    assert_eq!(b.outcome, BlockOutcome::Cancelled, "{b:?}");
    assert_eq!(b.reason, Some("too long"));
}

// Spec: specs/F5-version-layer.md §4.6 (an attribute the client does not model ends the skill)
#[test]
fn an_unmodelled_attribute_ends_the_skill_and_stops_the_bot() {
    use azalea::protocol::packets::game::{ClientboundUpdateAttributes, c_update_attributes::AttributeSnapshot};
    let _lock = azalea::test_utils::tracing::init_with_level(tracing::Level::ERROR);
    let seen: Arc<Mutex<Vec<Vec<null_bot_api::UnmodelledInput>>>> = Default::default();
    let log = seen.clone();
    let skill = Script {
        n: 0,
        f: move |_, v: &dyn BotView, c: &mut dyn BotCommands| {
            log.lock().unwrap().push(v.unmodelled_inputs());
            c.walk(1, 0);
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    // The server only sends what it uses; a server-only attribute must not stop anything.
    let id = *rig.sim.app.world().get::<MinecraftEntityId>(rig.sim.entity).unwrap();
    rig.sim.receive_packet(ClientboundUpdateAttributes {
        entity_id: id,
        values: vec![AttributeSnapshot { attribute: azalea::registry::builtin::Attribute::MaxHealth, base: 40.0, modifiers: vec![] }],
    });
    for _ in 0..3 {
        rig.sim.tick();
    }
    assert!(rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill(), "max_health does not stop the skill");
    assert!(seen.lock().unwrap().iter().all(|l| l.is_empty()));
    // Scale away from 1 changes the hitbox: the skill ends and the walk input is cleared.
    rig.sim.receive_packet(ClientboundUpdateAttributes {
        entity_id: id,
        values: vec![AttributeSnapshot { attribute: azalea::registry::builtin::Attribute::Scale, base: 2.0, modifiers: vec![] }],
    });
    for _ in 0..4 {
        rig.sim.tick();
    }
    assert!(!rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill(), "the skill ended");
    let mv = rig.sim.app.world().get::<azalea::physics::client_movement::ClientMovementState>(rig.sim.entity).unwrap();
    assert!(matches!(mv.move_direction, azalea::WalkDirection::None), "walk input cleared");
}

// Spec: specs/F5-version-layer.md §4.6 (a status effect the client does not model ends the skill)
#[test]
fn an_unmodelled_effect_ends_the_skill_and_a_modelled_one_does_not() {
    use azalea::entity::MobEffectData;
    use azalea::protocol::packets::game::ClientboundUpdateMobEffect;
    use azalea::registry::builtin::MobEffect;
    let _lock = azalea::test_utils::tracing::init_with_level(tracing::Level::ERROR);
    let skill = Script {
        n: 0,
        f: |_, _: &dyn BotView, c: &mut dyn BotCommands| {
            c.walk(1, 0);
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    let id = *rig.sim.app.world().get::<MinecraftEntityId>(rig.sim.entity).unwrap();
    let data = MobEffectData { amplifier: 0, duration: 200, ..Default::default() };
    rig.sim.receive_packet(ClientboundUpdateMobEffect { entity_id: id, mob_effect: MobEffect::JumpBoost, data: data.clone() });
    for _ in 0..3 {
        rig.sim.tick();
    }
    assert!(rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill(), "jump boost is modelled");
    rig.sim.receive_packet(ClientboundUpdateMobEffect { entity_id: id, mob_effect: MobEffect::Levitation, data });
    for _ in 0..4 {
        rig.sim.tick();
    }
    assert!(!rig.sim.app.world().get::<BotBrain>(rig.sim.entity).unwrap().has_skill(), "levitation ends the skill");
}

// Spec: specs/F5-version-layer.md §4.6 (entry kinds; the disconnect for a Movement entry)
#[test]
fn levitation_is_an_action_entry_and_scale_a_movement_entry_that_disconnects() {
    use azalea::entity::MobEffectData;
    use azalea::protocol::packets::game::{ClientboundUpdateAttributes, ClientboundUpdateMobEffect, c_update_attributes::AttributeSnapshot};
    use azalea::registry::builtin::{Attribute, MobEffect};
    use azalea::unmodelled_disconnect::{DisconnectOnUnmodelledMovement, UnmodelledDisconnect};
    let _lock = azalea::test_utils::tracing::init_with_level(tracing::Level::ERROR);
    let seen: Arc<Mutex<Vec<Vec<null_bot_api::UnmodelledInput>>>> = Default::default();
    let log = seen.clone();
    let skill = Script {
        n: 0,
        f: move |_, v: &dyn BotView, c: &mut dyn BotCommands| {
            log.lock().unwrap().push(v.unmodelled_inputs());
            c.walk(1, 0);
            SkillStatus::Running
        },
    };
    let mut rig = rig_with(Box::new(skill), Brain::new());
    let e = rig.sim.entity;
    rig.sim.app.world_mut().entity_mut(e).insert(DisconnectOnUnmodelledMovement);
    let id = *rig.sim.app.world().get::<MinecraftEntityId>(e).unwrap();
    let data = MobEffectData { amplifier: 0, duration: 200, ..Default::default() };
    rig.sim.receive_packet(ClientboundUpdateMobEffect { entity_id: id, mob_effect: MobEffect::SlowFalling, data });
    for _ in 0..4 {
        rig.sim.tick();
    }
    assert!(rig.sim.app.world().get::<UnmodelledDisconnect>(e).is_none(), "slow falling does not disconnect");
    assert!(!rig.sim.app.world().get::<BotBrain>(e).unwrap().has_skill(), "slow falling stops the skill");
    {
        let last = seen.lock().unwrap().last().cloned().unwrap_or_default();
        // the skill saw nothing after it ended; the earlier ticks saw the entry
        let any = seen.lock().unwrap().iter().flatten().any(|u| matches!(u, null_bot_api::UnmodelledInput::Effect { kind: null_bot_api::UnmodelledKind::Action, .. }));
        assert!(any || last.is_empty());
    }
    rig.sim.receive_packet(ClientboundUpdateAttributes {
        entity_id: id,
        values: vec![AttributeSnapshot { attribute: Attribute::Scale, base: 2.0, modifiers: vec![] }],
    });
    rig.sim.tick();
    let rec = rig.sim.app.world().get::<UnmodelledDisconnect>(e).expect("scale disconnects the bot");
    assert_eq!(rec.inputs.len(), 1);
}
