//! Bot-side ops for A12 (death and respawn). `RemoteSkill::step` calls `step` for `Op::Life`;
//! everything goes through `BotView` / `BotCommands` only.
//! A dead bot keeps its brain but has no skill (it ends with `Failed("died")` at the death tick),
//! so ops cannot reach it; `DeathProbe`, a reflex, is the bot's own code that sees the dead view
//! and, when armed, tries every action once. The runner reads what the probe saw.

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

use null_bot_api::{
    ActionId, BotCommands, BotView, Click, DeadRefusal, Death, EntityId, Face, Group, Hand, Life, Reflex,
    ReflexSpec, RespawnSent, SkillStatus,
};

use crate::skill::Info;

#[derive(Clone, Debug)]
pub enum LifeOp {
    Status,
    /// Ask to respawn (`BotCommands::respawn`), then report.
    Respawn,
}

#[derive(Clone, Debug)]
pub struct LifeInfo {
    pub life: Life,
    pub deaths: u32,
    pub last_death: Option<Death>,
    pub last_respawn: Option<RespawnSent>,
    pub refusal: Option<&'static str>,
    /// The open window's id and kind, if any.
    pub window: Option<(i32, Option<&'static str>)>,
    pub sneaking: bool,
    pub using_item: bool,
}

// Spec: specs/F1-swarm-runtime.md §4.4 (the bot's record of its deaths and respawns)
pub fn step(
    op: &LifeOp,
    age: u64,
    v: &dyn BotView,
    c: &mut dyn BotCommands,
    l: &mut crate::skill_inventory::Latest,
) -> Option<Info> {
    let wait = match op {
        LifeOp::Status => 0,
        LifeOp::Respawn => {
            if age == 0 {
                let id = c.respawn();
                crate::skill_inventory::Latest::set(&mut l.respawn, id, c);
            }
            3
        }
    };
    (age >= wait).then(|| {
        Info::Life(LifeInfo {
            life: v.life(),
            deaths: v.deaths(),
            last_death: v.last_death().cloned(),
            // The respawn that ended the latest death, whoever asked (a command's own result is
            // `refusal`).
            last_respawn: v.last_death().and_then(|d| d.respawn.clone()),
            refusal: l.respawn.and_then(|id| v.action(id).respawn().and_then(|r| r.refused)),
            window: v.window().map(|w| (w.id, w.kind)),
            sneaking: v.sneaking(),
            using_item: v.using_item().is_some(),
        })
    })
}

/// What the probe saw, shared with the runner.
#[derive(Default)]
pub struct ProbeData {
    /// Host tick and life of every tick the view was not alive, oldest first.
    pub ticks: Vec<(u64, Life)>,
    /// Try every action once, this many dead ticks after the first.
    pub script_at: Option<u64>,
    /// The block the scripted commands aim at.
    pub block: Option<null_bot_api::BlockPos>,
    /// Ticks the script ran at and the commands it issued (names).
    pub fired: Option<u64>,
    pub issued: Vec<&'static str>,
    /// The attack target and the entities the view listed (kind, id).
    pub target: Option<(u32, Vec<(String, u32)>)>,
    /// The latest dead view's refusals, packets sent and click and open refusals.
    pub refused: Vec<DeadRefusal>,
    pub sent: Vec<(String, u32)>,
    pub open_refusal: Option<&'static str>,
    pub click_refused: Option<&'static str>,
    /// The ids of the probe's own `open_inventory` and `click_slot`, read each dead tick.
    pub open_id: Option<ActionId>,
    pub click_id: Option<ActionId>,
    /// The probe stays quiet (so a kept skill is stepped while dead instead).
    pub passive: bool,
}

impl ProbeData {
    /// Forget the last death and arm (or disarm) the script.
    pub fn reset(&mut self, script_at: Option<u64>, block: Option<null_bot_api::BlockPos>) {
        *self = ProbeData { script_at, block, ..Default::default() };
    }
}

pub type SharedProbe = Arc<Mutex<ProbeData>>;

/// A reflex that fires while the bot is dead: it records the view each tick and, once, tries every
/// action (F1 §4.4: a dead bot sends nothing a death screen cannot).
// Spec: specs/F1-swarm-runtime.md §4.4
pub struct DeathProbe {
    spec: ReflexSpec,
    data: SharedProbe,
}

impl DeathProbe {
    pub fn new(data: SharedProbe) -> Self {
        Self {
            spec: ReflexSpec { id: "death_probe", group: Group::Survival, priority: 1000, default_on: true, params: vec![] },
            data,
        }
    }
}

impl Reflex for DeathProbe {
    fn spec(&self) -> &ReflexSpec {
        &self.spec
    }
    fn trigger(&mut self, view: &dyn BotView) -> bool {
        view.life() != Life::Alive && !self.data.lock().unwrap().passive
    }
    fn act(&mut self, v: &dyn BotView, c: &mut dyn BotCommands) {
        let mut d = self.data.lock().unwrap();
        d.ticks.push((v.tick(), v.life()));
        let n = d.ticks.len() as u64 - 1;
        if d.script_at == Some(n) && d.fired.is_none() {
            let b = d.block.unwrap_or(null_bot_api::BlockPos { x: 0, y: 0, z: 0 });
            let me = v.position();
            let d2 = |p: null_bot_api::Vec3| (p.x - me.x).powi(2) + (p.y - me.y).powi(2) + (p.z - me.z).powi(2);
            // The nearest pig: the row's own, a few blocks away.
            let target = v
                .entities()
                .iter()
                .filter(|e| e.kind == "minecraft:pig")
                .min_by(|a, b| d2(a.position).total_cmp(&d2(b.position)))
                .map(|e| e.id)
                .unwrap_or(EntityId(0));
            d.target = Some((target.0, v.entities().iter().map(|e| (e.kind.clone(), e.id.0)).collect()));
            c.walk(1, 0);
            c.sprint(true);
            c.sneak(true);
            c.jump();
            c.look(90.0, 10.0);
            c.select_slot(5);
            c.swap_offhand();
            c.attack(target);
            c.use_item(Hand::Main);
            c.release_use();
            c.look_at_block(b, None);
            c.break_block(b);
            c.cancel_break();
            c.place_block(b, Face::Up, false);
            c.open_container(b, None);
            d.open_id = Some(c.open_inventory());
            d.click_id = Some(c.click_slot(Some(36), Click::Left));
            c.close_window();
            c.goto(null_bot_api::BlockPos { x: b.x + 10, y: b.y, z: b.z }, 0.0);
            d.fired = Some(n);
            d.issued = vec![
                "walk", "sprint", "sneak", "jump", "look", "select_slot", "swap_offhand", "attack", "use_item",
                "release_use", "look_at_block", "break_block", "cancel_break", "place_block", "open_container",
                "open_inventory", "click_slot", "close_window", "goto",
            ];
        }
        d.refused = v.refused_while_dead();
        d.sent = v.sent_while_dead();
        d.open_refusal = d.open_id.and_then(|id| v.action(id).open().and_then(|o| o.refused));
        d.click_refused = d.click_id.and_then(|id| v.action(id).click().and_then(|c| c.refused));
    }
}

/// `RemoteSkill::on_death` answers `Keep` while this is set (row `skill_on_death_keep`).
pub static KEEP_ARMED: AtomicBool = AtomicBool::new(false);

/// One step of the kept skill: its own step number, the host tick and what the view said.
#[derive(Clone, Debug)]
pub struct KeepStep {
    pub step: u64,
    pub tick: u64,
    pub life: Life,
}

#[derive(Default)]
pub struct KeepLog {
    pub steps: Vec<KeepStep>,
    /// The fork's refusals as the latest dead step saw them.
    pub refused: Vec<DeadRefusal>,
}

pub static KEEP_LOG: Mutex<KeepLog> = Mutex::new(KeepLog { steps: Vec::new(), refused: Vec::new() });

/// Per-instance counters of the kept skill (a fresh instance would start again at 0).
#[derive(Default)]
pub struct KeepState {
    steps: u64,
    dead_steps: u64,
    alive_steps: u64,
    died: bool,
}

// Spec: specs/F1-swarm-runtime.md §4.4 (a kept skill is stepped while dead and carries on after)
/// While armed the skill does only this: it counts its steps, tries a slot change while dead
/// (refused), changes slot after the respawn (the server reads it back) and ends `Done`.
pub fn keep_step(st: &mut KeepState, v: &dyn BotView, c: &mut dyn BotCommands) -> Option<SkillStatus> {
    if !KEEP_ARMED.load(Ordering::Relaxed) {
        return None;
    }
    st.steps += 1;
    let life = v.life();
    let mut log = KEEP_LOG.lock().unwrap();
    log.steps.push(KeepStep { step: st.steps, tick: v.tick(), life });
    if life != Life::Alive {
        st.died = true;
        st.dead_steps += 1;
        if st.dead_steps == 2 {
            c.select_slot(5);
        }
        log.refused = v.refused_while_dead();
    } else if st.died {
        st.alive_steps += 1;
        if st.alive_steps == 3 {
            c.select_slot(6);
        } else if st.alive_steps >= 10 {
            KEEP_ARMED.store(false, Ordering::Relaxed);
            return Some(SkillStatus::Done);
        }
    }
    Some(SkillStatus::Running)
}
