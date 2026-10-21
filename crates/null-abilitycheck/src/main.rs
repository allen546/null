//! null-abilitycheck: milestone SA (specs/plans/ABILITY_CHECK.md). One bot, driven through
//! `mc-adapter`, does each ability against skeleton-server; RCON sets the scene and reads the
//! server's truth. One line per scenario: `PASS|FAIL <id> <name> — <evidence>`.

mod rcon;
mod scenarios;
mod scenarios_block_effects;
mod scenarios_containers;
mod scenarios_crafting;
mod scenarios_crouch;
mod scenarios_envelope;
mod scenarios_entity_containers;
mod scenarios_inventory;
mod scenarios_items;
mod scenarios_gather;
mod scenarios_mailbox;
mod scenarios_respawn;
mod scenarios_shapes;
mod scenarios_server_attributes;
mod scenarios_tick;
mod skill;
mod skill_inventory;
mod skill_items;
mod skill_gather;
mod skill_mailbox;
mod skill_respawn;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

use azalea::{
    DefaultPlugins,
    account::Account,
    swarm::{DefaultSwarmPlugins, SwarmBuilder},
};
use mc_adapter::{Backoff, BotBrain, BotState, HostEvent, HostPlugin, NodePlugin, null_bot_plugins};
use null_bot_api::Brain;

use scenarios::{Ctx, Outcome};
use skill::RemoteSkill;

const USAGE: &str = "usage: null-abilitycheck [--server 127.0.0.1:25567] [--account null_check] \
[--scenario all|A1|A2|A3|A4|A2S|A4S|A4C|A5|A6|A7|A8|A9|A10|A10E|A11|A12|A13|A14|A15|A16|A17|AT[,..]] [--rcon-port 25577] [--rcon-password skeleton] [--server-log PATH]";

struct Args {
    server: String,
    account: String,
    scenarios: Vec<String>,
    rcon_port: u16,
    rcon_pass: String,
    log: PathBuf,
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        server: "127.0.0.1:25567".into(),
        account: "null_check".into(),
        scenarios: vec!["all".into()],
        rcon_port: 25577,
        rcon_pass: "skeleton".into(),
        log: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../harness/skeleton-server/server/server.log"),
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--server" => a.server = val()?,
            "--account" => a.account = val()?,
            "--scenario" => a.scenarios = val()?.split(',').map(|s| s.trim().to_uppercase()).collect(),
            "--rcon-port" => a.rcon_port = val()?.parse().map_err(|e| format!("--rcon-port: {e}"))?,
            "--rcon-password" => a.rcon_pass = val()?,
            "--server-log" => a.log = PathBuf::from(val()?),
            _ => return Err(USAGE.into()),
        }
    }
    Ok(a)
}

/// A16 ends the runner's skill on purpose (an unmodelled input stops it); this puts a fresh one in
/// when the row asks for it, so later rows still run. Only the runner's own account gets it: the
/// A8 attacker (`<account>2`) has no skill, and a second `RemoteSkill` would take ops from the
/// shared queue.
struct ReviveSkill {
    account: String,
    make: Arc<dyn Fn() -> Box<dyn null_bot_api::Skill> + Send + Sync>,
}

impl azalea::app::Plugin for ReviveSkill {
    fn build(&self, app: &mut azalea::app::App) {
        use azalea::ecs::prelude::IntoScheduleConfigs;
        let (make, account) = (self.make.clone(), self.account.clone());
        app.add_systems(azalea::app::Update, (move |mut bots: azalea::ecs::prelude::Query<(&mut BotBrain, &azalea::player::GameProfileComponent)>| {
            if scenarios_server_attributes::REVIVE.load(std::sync::atomic::Ordering::Relaxed) {
                for (mut bb, profile) in &mut bots {
                    if profile.0.name == account && !bb.has_skill() {
                        bb.install_skill(make());
                        scenarios_server_attributes::REVIVE.store(false, std::sync::atomic::Ordering::Relaxed);
                    }
                }
            }
        }).ambiguous_with_all());
    }
}

/// Prints total wall time, time per module (slowest first), the 10 slowest rows and the RCON wait.
/// Fixed sleeps are not summed: they sit at about 150 call sites, so only RCON is counted.
// Spec: specs/plans/ABILITY_CHECK.md §How to run (Timing)
fn print_timing(total: f64, modules: &[(&str, f64)], results: &[Outcome]) {
    let mut by_module = modules.to_vec();
    by_module.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut rows: Vec<&Outcome> = results.iter().collect();
    rows.sort_by(|a, b| b.secs.total_cmp(&a.secs));
    let rcon = rcon::WAIT_NS.load(std::sync::atomic::Ordering::Relaxed) as f64 / 1e9;
    println!("TIMING total {total:.1} s");
    println!("TIMING modules, slowest first: {}", by_module.iter().map(|(id, t)| format!("{id} {t:.1} s")).collect::<Vec<_>>().join(", "));
    println!("TIMING slowest rows:");
    for o in rows.iter().take(10) {
        println!("TIMING   {:.1} s {} {}", o.secs, o.id, o.name);
    }
    println!("TIMING RCON waits {rcon:.1} s ({:.0}% of total); fixed sleeps are not measured", rcon / total.max(0.001) * 100.0);
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };
    let rc = rcon::Rcon::connect(args.rcon_port, &args.rcon_pass).unwrap_or_else(|e| {
        eprintln!("cannot reach RCON on port {} (is skeleton-server up?): {e}", args.rcon_port);
        std::process::exit(2);
    });

    let (op_tx, op_rx) = mpsc::channel();
    let (rep_tx, rep_rx) = mpsc::channel();
    let (ev_tx, ev_rx) = mpsc::channel();
    let ops = Arc::new(Mutex::new(op_rx));
    let probe: skill_respawn::SharedProbe = Arc::default();
    let bot_probe = probe.clone();
    let account = args.account.clone();
    let server = args.server.clone();
    // A8 needs a second player online to be the attacker: `<account>2`, with no skill.
    let account2 = format!("{}2", args.account);
    let two = args.scenarios.iter().any(|s| s == "ALL" || s == "A8");
    // A16 and A17 end some rows in a disconnect: each such row has its own idle account.
    let spares = args.scenarios.iter().any(|s| s == "ALL" || s == "A16" || s == "A17");
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let second = account2.clone();
            let probe_main = account.clone();
            let (revive_ops, revive_rep) = (ops.clone(), rep_tx.clone());
            let revive_account = account.clone();
            let factory = Arc::new(move |name: &str| {
                if name == second || name.starts_with("null_d") {
                    return BotBrain::new(Brain::new(), None);
                }
                let skill = RemoteSkill::new(ops.clone(), rep_tx.clone());
                let mut brain = Brain::new();
                brain.register(Box::new(skill_respawn::DeathProbe::new(bot_probe.clone())));
                BotBrain::new(brain, Some(Box::new(skill)))
            });
            let mut accounts = vec![Account::offline(&account)];
            if two {
                accounts.push(Account::offline(&account2));
            }
            if spares {
                for k in 1..=scenarios_envelope::SPARES {
                    accounts.push(Account::offline(&scenarios_envelope::d_name(k)));
                }
            }
            let _ = SwarmBuilder::new_without_plugins()
                .add_plugins((
                    DefaultPlugins,
                    null_bot_plugins(),
                    DefaultSwarmPlugins,
                    HostPlugin,
                    scenarios_envelope::EnvelopeProbe { main: probe_main },
                    ReviveSkill { account: revive_account, make: Arc::new(move || Box::new(skill::RemoteSkill::new(revive_ops.clone(), revive_rep.clone()))) },
                    NodePlugin { factory, events: ev_tx, backoff: Backoff::default(), respawn: true },
                ))
                .reconnect_after(None)
                .add_accounts(accounts)
                .start(server.as_str())
                .await;
        });
    });

    // Wait for the skill to be installed (the bot is spawned and loaded).
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        match ev_rx.recv_timeout(Duration::from_secs(1)) {
            Ok(HostEvent::State(c)) if matches!(c.state, BotState::OnJob(_)) => break,
            Ok(HostEvent::State(c)) if matches!(c.state, BotState::Kicked { .. } | BotState::Offline { .. } | BotState::Banned) => {
                eprintln!("bot could not join: {:?}", c.state);
                std::process::exit(2);
            }
            _ if std::time::Instant::now() > deadline => {
                eprintln!("bot did not spawn within 60 s");
                std::process::exit(2);
            }
            _ => {}
        }
    }

    let mut cx = Ctx { rc, name: args.account.clone(), ops: op_tx, reports: rep_rx, seq: 0, log: args.log, events: ev_rx, probe };
    let wants = |id: &str| args.scenarios.iter().any(|s| s == "ALL" || s == id);
    let mut results: Vec<Outcome> = Vec::new();
    type Scenario = fn(&mut Ctx) -> Vec<Outcome>;
    let all: [(&str, Scenario); 4] = [("A1", scenarios::a1), ("A2", scenarios::a2), ("A3", scenarios::a3), ("A4", scenarios::a4)];
    let shapes: [(&str, Scenario); 3] = [("A4S", scenarios_shapes::a4s), ("A2S", scenarios_shapes::a2s), ("A4C", scenarios_crouch::a4c)];
    let items: [(&str, Scenario); 15] = [
        ("A5", scenarios_items::a5),
        ("A6", scenarios_items::a6),
        ("A7", scenarios_items::a7),
        ("A8", scenarios_items::a8),
        ("A9", scenarios_inventory::a9),
        ("A10", scenarios_containers::a10),
        ("A10E", scenarios_entity_containers::a10e),
        ("A11", scenarios_crafting::a11),
        ("A12", scenarios_respawn::a12),
        ("A13", scenarios_mailbox::a13),
        ("A14", scenarios_gather::a14),
        ("A15", scenarios_block_effects::a15),
        ("A16", scenarios_server_attributes::a16),
        ("A17", scenarios_envelope::a17),
        ("AT", scenarios_tick::at),
    ];
    let mut aborted = false;
    let run_start = Instant::now();
    let mut modules: Vec<(&str, f64)> = Vec::new();
    for (id, f) in all.into_iter().chain(shapes).chain(items) {
        if !wants(id) {
            continue;
        }
        // A panic (the bot stopped answering) becomes a failed row; the run then stops, since the
        // bot's state is unknown. Rows already printed stay printed.
        let module_start = Instant::now();
        let ran = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut cx)));
        modules.push((id, module_start.elapsed().as_secs_f64()));
        match ran {
            Ok(rows) => {
                for o in rows {
                    if !o.shown {
                        println!("{}", scenarios::report_line(&o));
                    }
                    results.push(o);
                }
            }
            Err(e) => {
                let why = e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default();
                let o = scenarios::Outcome { id: "RUN", name: format!("{id}_aborted"), pass: false, evidence: format!("{id} panicked: {why}; run stopped"), shown: false, secs: 0.0 };
                println!("{}", scenarios::report_line(&o));
                results.push(o);
                aborted = true;
                break;
            }
        }
    }
    if !aborted {
        cx.clean();
    }
    let failed = results.iter().filter(|o| !o.pass).count();
    println!("{} scenario(s), {} passed, {failed} failed", results.len(), results.len() - failed);
    print_timing(run_start.elapsed().as_secs_f64(), &modules, &results);
    // The bot thread never ends; leaving closes its socket.
    std::process::exit(if failed > 0 || results.is_empty() { 1 } else { 0 });
}
