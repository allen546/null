//! null-node: brings a list of accounts up in one Azalea swarm and runs a demo skill on each.
//! F1's node shell for the skeleton: paced logins, reconnect with backoff, BotState JSON lines.

use std::{
    collections::HashMap,
    io::Write,
    sync::{Arc, mpsc},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use azalea::{
    DefaultPlugins,
    account::Account,
    swarm::{DefaultSwarmPlugins, SwarmBuilder},
};
use mc_adapter::{
    Backoff, BotBrain, BotState, HostEvent, HostPlugin, NodePlugin, null_bot_plugins,
};
use null_bot_api::{Brain, SkillStatus};
use null_bot_api::{BlockPos, Skill};
use null_combat::{Engage, FightRules, Retreat, Telemetry, TargetSelector};
use null_skills_demo::{GotoPoint, Patrol, PatrolConfig};
use serde_json::{Value, json};

struct Args {
    server: String,
    accounts: Vec<String>,
    join_interval_ms: u64,
    state_log: Option<String>,
    exit_after_skills: bool,
    center: Option<(f64, f64)>,
    goto: Option<(i32, i32, i32)>,
    fight: bool,
    no_respawn: bool,
    telemetry: Option<String>,
    enemies: Vec<String>,
}

const USAGE: &str = "usage: null-node --server HOST:PORT --accounts null_b1..null_b20 \
[--join-interval-ms N] [--state-log FILE] [--fight --enemies A..B [--no-respawn] [--telemetry FILE]] [--center X,Z] [--goto X,Y,Z] [--exit-after-skills] [--fight --enemies PREFIXa..PREFIXb]\n  --fight runs the engage skill and retreat reflex against --enemies (the node's own accounts are allies); runs until killed\n  --goto replaces the patrol: every bot walks to that block and finishes
  --accounts takes PREFIXa..PREFIXb (numeric suffix range) or a comma-separated list";

/// `null_b1..null_b20` -> null_b1, null_b2, ... null_b20.
fn expand_accounts(spec: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    for part in spec.split(',').filter(|p| !p.is_empty()) {
        let Some((a, b)) = part.split_once("..") else {
            out.push(part.to_string());
            continue;
        };
        let split = |s: &str| {
            let i = s.trim_end_matches(|c: char| c.is_ascii_digit()).len();
            (s[..i].to_string(), s[i..].parse::<u32>())
        };
        let (pa, na) = split(a);
        let (pb, nb) = split(b);
        match (na, nb) {
            (Ok(na), Ok(nb)) if pa == pb && na <= nb => {
                out.extend((na..=nb).map(|n| format!("{pa}{n}")))
            }
            _ => return Err(format!("bad account range {part:?}")),
        }
    }
    if out.is_empty() {
        return Err("no accounts".into());
    }
    Ok(out)
}

fn parse_args() -> Result<Args, String> {
    let mut a = Args {
        server: String::new(),
        accounts: vec![],
        join_interval_ms: 300,
        state_log: None,
        exit_after_skills: false,
        center: None,
        goto: None,
        fight: false,
        no_respawn: false,
        telemetry: None,
        enemies: vec![],
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or(format!("{flag} needs a value"));
        match flag.as_str() {
            "--server" => a.server = val()?,
            "--accounts" => a.accounts.extend(expand_accounts(&val()?)?),
            "--join-interval-ms" => {
                a.join_interval_ms = val()?.parse().map_err(|e| format!("--join-interval-ms: {e}"))?
            }
            "--state-log" => a.state_log = Some(val()?),
            "--center" => {
                let v = val()?;
                let (x, z) = v.split_once(',').ok_or("--center wants X,Z")?;
                a.center = Some((
                    x.parse().map_err(|e| format!("--center: {e}"))?,
                    z.parse().map_err(|e| format!("--center: {e}"))?,
                ));
            }
            "--goto" => {
                let v = val()?;
                let p: Vec<i32> = v.split(',').map(|n| n.trim().parse()).collect::<Result<_, _>>()
                    .map_err(|e| format!("--goto: {e}"))?;
                let [x, y, z] = p[..] else { return Err("--goto wants X,Y,Z".into()) };
                a.goto = Some((x, y, z));
            }
            "--no-respawn" => a.no_respawn = true,
            "--telemetry" => a.telemetry = Some(val()?),
            "--fight" => a.fight = true,
            "--enemies" => a.enemies.extend(expand_accounts(&val()?)?),
            "--exit-after-skills" => a.exit_after_skills = true,
            "-h" | "--help" => return Err(USAGE.into()),
            other => return Err(format!("unknown argument {other}\n{USAGE}")),
        }
    }
    if a.server.is_empty() || a.accounts.is_empty() {
        return Err(USAGE.into());
    }
    if a.fight && a.enemies.is_empty() {
        return Err("--fight needs --enemies".into());
    }
    Ok(a)
}

fn state_json(s: &BotState) -> Value {
    match s {
        BotState::OnJob(job) => json!({ "job": job }),
        BotState::Kicked { class, reason, retry_in_ms } => {
            json!({ "class": format!("{class:?}"), "reason": reason, "retry_in_ms": retry_in_ms })
        }
        BotState::Offline { reason, retry_in_ms } => {
            json!({ "reason": reason, "retry_in_ms": retry_in_ms })
        }
        _ => Value::Null,
    }
}

/// Writes events as JSON lines until the channel closes. With `exit_after`, ends the process
/// once that many skills have finished (a test convenience).
fn log_events(rx: mpsc::Receiver<HostEvent>, mut out: Box<dyn Write + Send>, exit_after: Option<usize>) {
    let mut finished = 0usize;
    for ev in rx {
        let wall_ms = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64);
        let line = match &ev {
            HostEvent::State(c) => json!({
                "wall_ms": wall_ms, "t_ms": c.at_ms, "bot": c.bot,
                "state": c.state.label(), "detail": state_json(&c.state),
            }),
            HostEvent::SkillEnded { bot, skill, status, tick } => {
                finished += 1;
                let (st, reason) = match status {
                    SkillStatus::Done => ("done", Value::Null),
                    SkillStatus::Failed(r) => ("failed", json!(r)),
                    SkillStatus::Running => ("running", Value::Null),
                };
                json!({ "wall_ms": wall_ms, "event": "skill_ended", "bot": bot, "skill": skill,
                        "status": st, "reason": reason, "tick": tick })
            }
            HostEvent::Died { bot, tick, death } => {
                let at = death.as_ref().map(|d| json!({
                    "x": d.position.x, "y": d.position.y, "z": d.position.z,
                    "dimension": d.dimension, "message": d.message,
                    "stacks": d.inventory.iter().chain(d.armor.iter()).chain(std::iter::once(&d.offhand)).flatten().count(),
                }));
                json!({ "wall_ms": wall_ms, "event": "died", "bot": bot, "tick": tick, "death": at })
            }
            HostEvent::ReflexDisabled { bot, reflex, tick } => {
                json!({ "wall_ms": wall_ms, "event": "reflex_disabled", "bot": bot, "reflex": reflex,
                        "why": "result leak", "tick": tick })
            }
        };
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
        if exit_after.is_some_and(|n| finished >= n) {
            std::thread::sleep(Duration::from_millis(500));
            std::process::exit(0);
        }
    }
}

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(2);
        }
    };

    let out: Box<dyn Write + Send> = match &args.state_log {
        Some(p) => Box::new(std::fs::File::create(p).expect("open --state-log")),
        None => Box::new(std::io::stdout()),
    };
    let (tx, rx) = mpsc::channel();
    let exit_after = (args.exit_after_skills && !args.fight).then_some(args.accounts.len());
    std::thread::spawn(move || log_events(rx, out, exit_after));

    // Close the sockets promptly on Ctrl-C so the server logs a normal leave.
    tokio::spawn(async {
        let _ = tokio::signal::ctrl_c().await;
        std::process::exit(0);
    });

    // Fight telemetry: one JSON line per sample or event, stamped with wall time.
    let tel_tx = args.telemetry.as_ref().map(|p| {
        let mut f = std::io::BufWriter::new(std::fs::File::create(p).expect("open --telemetry"));
        let (ttx, trx) = mpsc::channel::<String>();
        std::thread::spawn(move || {
            for line in trx {
                let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
                // Prepend wall_ms into the object.
                let _ = writeln!(f, "{{\"wall_ms\":{ms},{}", &line[1..]);
                let _ = f.flush();
            }
        });
        ttx
    });
    let tel_tx = std::sync::Mutex::new(tel_tx);

    let slots: HashMap<String, usize> =
        args.accounts.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
    let count = args.accounts.len();
    let (center, goto) = (args.center, args.goto);
    let (fight, enemies, allies) = (args.fight, args.enemies.clone(), args.accounts.clone());
    let factory = Arc::new(move |name: &str| {
        if fight {
            // Naming enemies is the permission to attack first; everyone else is never a target.
            let mut rules = FightRules::new(TargetSelector::PlayerNames(enemies.clone()));
            rules.never_names = allies.clone();
            let seed = slots.get(name).copied().unwrap_or(0) as u64 * 7919;
            // Spec: specs/B5-combat.md §4.9; specs/CONVENTIONS.md §4.1
            // The retreat reflex preempts engage while it fires; both report through one telemetry handle.
            let tel = tel_tx.lock().unwrap().clone().map(|tx| Telemetry::new(name, tx, 10));
            let mut engage = Engage::new(rules.clone(), center.unwrap_or((416.0, 352.0)), seed);
            let mut retreat = Retreat::new(rules).with_self_name(name);
            if let Some(t) = tel {
                engage = engage.with_telemetry(t.clone());
                retreat = retreat.with_telemetry(t);
            }
            let mut brain = Brain::new();
            brain.register(Box::new(retreat));
            return BotBrain::new(brain, Some(Box::new(engage)));
        }
        let skill: Box<dyn Skill> = match goto {
            Some((x, y, z)) => Box::new(GotoPoint::new(BlockPos::new(x, y, z))),
            None => {
                let mut cfg = PatrolConfig::new(slots.get(name).copied().unwrap_or(0), count);
                cfg.center = center;
                Box::new(Patrol::new(cfg))
            }
        };
        BotBrain::new(Brain::new(), Some(skill))
    });

    let accounts: Vec<Account> = args.accounts.iter().map(|n| Account::offline(n)).collect();
    let builder = SwarmBuilder::new_without_plugins()
        .add_plugins((
            DefaultPlugins,
            null_bot_plugins(),
            DefaultSwarmPlugins,
            HostPlugin,
            NodePlugin { factory, events: tx, backoff: Backoff::default(), respawn: !args.no_respawn },
        ))
        // Azalea's fixed-delay reconnect is replaced by NodePlugin's backoff.
        .reconnect_after(None)
        .add_accounts(accounts)
        .join_delay(Duration::from_millis(args.join_interval_ms));
    let _ = builder.start(args.server.as_str()).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_ranges() {
        assert_eq!(expand_accounts("null_b1..null_b3").unwrap(), ["null_b1", "null_b2", "null_b3"]);
        assert_eq!(expand_accounts("a,b").unwrap(), ["a", "b"]);
        assert_eq!(expand_accounts("null_b1..null_b20").unwrap().len(), 20);
        assert!(expand_accounts("x1..y3").is_err());
        assert!(expand_accounts("x3..x1").is_err());
    }
}
