//! null-abilitycheck: milestone SA (specs/plans/ABILITY_CHECK.md). One bot, driven through
//! `mc-adapter`, does each ability against skeleton-server; RCON sets the scene and reads the
//! server's truth. One line per scenario: `PASS|FAIL <id> <name> — <evidence>`.

mod rcon;
mod scenarios;
mod scenarios_items;
mod scenarios_shapes;
mod skill;
mod skill_items;

use std::{
    path::PathBuf,
    sync::{Arc, Mutex, mpsc},
    time::Duration,
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
[--scenario all|A1|A2|A3|A4|A2S|A4S|A5|A6|A7|A8[,..]] [--rcon-port 25577] [--rcon-password skeleton] [--server-log PATH]";

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
    let account = args.account.clone();
    let server = args.server.clone();
    // A8 needs a second player online to be the attacker: `<account>2`, with no skill.
    let account2 = format!("{}2", args.account);
    let two = args.scenarios.iter().any(|s| s == "ALL" || s == "A8");
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_multi_thread().enable_all().build().unwrap();
        rt.block_on(async move {
            let second = account2.clone();
            let factory = Arc::new(move |name: &str| {
                if name == second {
                    return BotBrain::new(Brain::new(), None);
                }
                let skill = RemoteSkill::new(ops.clone(), rep_tx.clone());
                BotBrain::new(Brain::new(), Some(Box::new(skill)))
            });
            let mut accounts = vec![Account::offline(&account)];
            if two {
                accounts.push(Account::offline(&account2));
            }
            let _ = SwarmBuilder::new_without_plugins()
                .add_plugins((
                    DefaultPlugins,
                    null_bot_plugins(),
                    DefaultSwarmPlugins,
                    HostPlugin,
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

    let mut cx = Ctx { rc, name: args.account.clone(), ops: op_tx, reports: rep_rx, log: args.log };
    let wants = |id: &str| args.scenarios.iter().any(|s| s == "ALL" || s == id);
    let mut results: Vec<Outcome> = Vec::new();
    type Scenario = fn(&mut Ctx) -> Vec<Outcome>;
    let all: [(&str, Scenario); 4] = [("A1", scenarios::a1), ("A2", scenarios::a2), ("A3", scenarios::a3), ("A4", scenarios::a4)];
    let shapes: [(&str, Scenario); 2] = [("A4S", scenarios_shapes::a4s), ("A2S", scenarios_shapes::a2s)];
    let items: [(&str, Scenario); 4] = [
        ("A5", scenarios_items::a5),
        ("A6", scenarios_items::a6),
        ("A7", scenarios_items::a7),
        ("A8", scenarios_items::a8),
    ];
    let mut aborted = false;
    for (id, f) in all.into_iter().chain(shapes).chain(items) {
        if !wants(id) {
            continue;
        }
        // A panic (the bot stopped answering) becomes a failed row; the run then stops, since the
        // bot's state is unknown. Rows already printed stay printed.
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&mut cx))) {
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
                let o = scenarios::Outcome { id: "RUN", name: format!("{id}_aborted"), pass: false, evidence: format!("{id} panicked: {why}; run stopped"), shown: false };
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
    // The bot thread never ends; leaving closes its socket.
    std::process::exit(if failed > 0 || results.is_empty() { 1 } else { 0 });
}
