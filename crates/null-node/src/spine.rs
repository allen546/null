//! The live spine (S6a): HQ, the operation, the squad and one `BotLink` per real bot, in one process.
//! The spine runs on its own thread at one step per game tick (50 ms). The Bevy bots never wait on it: they take
//! skills from an inbox and report through shared state, each with a short lock.
//! Spec: specs/plans/S6_BRIEF.md (task S6a); the bot side of the protocol is `null-botlink`.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
    time::Duration,
};

use azalea::{
    account::Account,
    app::{App, Plugin, PostUpdate},
    entity::Position,
};
use azalea::ecs::prelude::*;
use mc_adapter::{BotBrain, BotState, HostEvent};
use null_bot_api::{Skill, SkillStatus};
use null_botlink::{BotLink, FellRun, FellSpec, Host, HostStatus, RunEnd, RunHandle, RunStatus};
use null_bus::InProcBus;
use null_cli::{Spine, SpineBot, SpineConfig};
use null_proto::{BotId, Dimension, Gt, Life, Loc, Pos, ServerId};
use serde::Deserialize;

/// One tree the operation may fell, from the run script's `--trees` file: its lowest log and its log count.
#[derive(Deserialize)]
struct TreeRow {
    x: i32,
    y: i32,
    z: i32,
    logs: u32,
}

/// Reads the tree list the run script wrote after planting by RCON.
// Spec: specs/plans/S6_BRIEF.md (trees come from the server, set up by RCON)
pub fn read_trees(path: &str, server: &ServerId) -> Result<Vec<(Loc, u32)>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("--trees {path}: {e}"))?;
    let rows: Vec<TreeRow> = serde_json::from_str(&text).map_err(|e| format!("--trees {path}: {e}"))?;
    Ok(rows
        .into_iter()
        .map(|r| (Loc { server: server.clone(), dim: Dimension::Overworld, pos: Pos::new(r.x, r.y, r.z) }, r.logs))
        .collect())
}

#[derive(Clone)]
struct BotRow {
    life: Life,
    pos: Option<Pos>,
}

/// State shared between the Bevy bots and the spine thread.
#[derive(Default)]
pub struct Shared {
    /// Skills the spine wants started, per bot name.
    inbox: Mutex<HashMap<String, VecDeque<Box<dyn Skill>>>>,
    runs: Mutex<HashMap<String, RunHandle>>,
    rows: Mutex<HashMap<String, BotRow>>,
}

impl Shared {
    /// Keeps the shared view in step with what the lifecycle reports. A bot that is not alive loses its pending start.
    // Spec: specs/F1-swarm-runtime.md §4.4
    pub fn on_event(&self, ev: &HostEvent) {
        let (bot, life) = match ev {
            HostEvent::State(c) => {
                let life = match c.state {
                    BotState::Spawned | BotState::Idle | BotState::OnJob(_) => Life::Alive,
                    BotState::Dead | BotState::Respawning => Life::Dead,
                    _ => Life::Disconnected,
                };
                (c.bot.clone(), life)
            }
            HostEvent::Died { bot, .. } => (bot.clone(), Life::Dead),
            HostEvent::UnmodelledDisconnect { bot, .. } => (bot.clone(), Life::Disconnected),
            HostEvent::SkillEnded { bot, skill, status, .. } if skill == "fell" => {
                // A skill the host ended (leak guard, unmodelled input) never reported itself.
                if let (SkillStatus::Failed(why), Some(h)) = (status, self.runs.lock().unwrap().get(bot)) {
                    h.finish(RunEnd { status: RunStatus::Failed(why.clone()), items: Default::default(), gathered: 0, left: 0, drops_left: 0 });
                }
                return;
            }
            _ => return,
        };
        if life != Life::Alive {
            self.inbox.lock().unwrap().remove(&bot);
            self.runs.lock().unwrap().remove(&bot);
        }
        self.rows.lock().unwrap().entry(bot).and_modify(|r| r.life = life).or_insert(BotRow { life, pos: None });
    }
}

/// The Bevy side: installs a started skill when the bot has none, and reports the bot's block position.
/// Does nothing without `--spine`.
pub struct SpineLink(pub Option<Arc<Shared>>);

impl Plugin for SpineLink {
    fn build(&self, app: &mut App) {
        if let Some(shared) = &self.0 {
            app.insert_resource(Link(shared.clone())).add_systems(PostUpdate, deliver);
        }
    }
}

#[derive(Resource)]
struct Link(Arc<Shared>);

// Spec: specs/plans/S6_BRIEF.md (the bot's tick never waits on the bus)
fn deliver(link: Res<Link>, mut bots: Query<(&Account, &mut BotBrain, &Position)>) {
    for (account, mut brain, pos) in &mut bots {
        let name = account.username();
        let here = Pos::new(pos.x.floor() as i32, pos.y.floor() as i32, pos.z.floor() as i32);
        if let Some(r) = link.0.rows.lock().unwrap().get_mut(name) {
            r.pos = Some(here);
        }
        if brain.has_skill() {
            continue;
        }
        let next = link.0.inbox.lock().unwrap().get_mut(name).and_then(VecDeque::pop_front);
        if let Some(skill) = next {
            brain.install_skill(skill);
        }
    }
}

/// A real bot as the spine sees it.
struct NodeHost {
    name: String,
    shared: Arc<Shared>,
}

impl Host for NodeHost {
    // Spec: specs/B4-gathering.md §3
    fn start(&mut self, spec: FellSpec) -> RunHandle {
        let handle = RunHandle::new();
        let skill = FellRun::new(&spec, &self.name, handle.clone());
        self.shared.runs.lock().unwrap().insert(self.name.clone(), handle.clone());
        self.shared.inbox.lock().unwrap().entry(self.name.clone()).or_default().push_back(Box::new(skill));
        handle
    }
    fn status(&mut self) -> HostStatus {
        let row = self.shared.rows.lock().unwrap().get(&self.name).cloned();
        match row {
            Some(r) => HostStatus { life: r.life, pos: r.pos, health: 20.0 },
            None => HostStatus { life: Life::Disconnected, pos: None, health: 0.0 },
        }
    }
}

struct NodeBot {
    link: BotLink<InProcBus>,
    host: NodeHost,
}

impl SpineBot for NodeBot {
    async fn tick(&mut self, now: Gt) {
        self.link.tick(now, &mut self.host).await;
    }
}

/// What the spine thread needs to start.
pub struct SpineArgs {
    pub accounts: Vec<String>,
    pub trees: Vec<(Loc, u32)>,
    pub audit: Option<String>,
    pub op_log_dir: Option<String>,
    pub job_budget: u64,
}

const SERVER: &str = "live1";

/// Runs the spine until the process ends: one step per 50 ms, command lines from `lines`.
// Spec: specs/plans/S6_BRIEF.md (null-node --spine reads the same command lines as null-cli)
pub fn run(shared: Arc<Shared>, args: SpineArgs, mut lines: tokio::sync::mpsc::UnboundedReceiver<String>) {
    let rt = tokio::runtime::Builder::new_current_thread().enable_time().build().expect("spine runtime");
    rt.block_on(async move {
        let server = ServerId::new(SERVER);
        let bus = InProcBus::default();
        let names: Vec<BotId> = args.accounts.iter().map(BotId::new).collect();
        let bots = args
            .accounts
            .iter()
            .map(|n| NodeBot { link: BotLink::new(&bus, &server, n), host: NodeHost { name: n.clone(), shared: shared.clone() } })
            .collect();
        let mut cfg = SpineConfig::new(1, args.accounts.len());
        cfg.server = SERVER.into();
        cfg.tree_box = (null_op::TREE_BOX_RADIUS, null_op::TREE_BOX_HEAD);
        cfg.job_budget = Gt(args.job_budget);
        cfg.jsonl_dir = args.op_log_dir.as_ref().map(Into::into);
        let trees = args.trees;
        let mut spine = Spine::with_bots(&cfg, bus, names, bots, Box::new(move || trees.clone()));
        if let Some(p) = &args.audit {
            spine.audit = null_cli::Audit::to_file(p.as_ref()).expect("audit file opens");
        }
        let mut tick = tokio::time::interval(Duration::from_millis(50));
        let mut reported = false;
        loop {
            tokio::select! {
                _ = tick.tick() => {
                    spine.step().await;
                    if spine.finished() && !reported {
                        reported = true;
                        println!("spine idle at tick {}", spine.now);
                    }
                }
                Some(line) = lines.recv() => {
                    if line.trim_start().starts_with("run") {
                        println!("error: run and run-done are for the simulation; the live spine runs in real time");
                    } else {
                        println!("{}", spine.exec(&line).await);
                    }
                    reported &= !line.trim_start().starts_with("set-goal");
                }
            }
        }
    });
}
