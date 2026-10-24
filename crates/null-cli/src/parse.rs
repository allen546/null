//! Command-line text to K2 commands, and replies back to text.
use null_hq::Reply;
use null_proto::*;

/// One line of input.
// Spec: specs/K4-human-interface.md §4.1
#[derive(Clone, Debug, PartialEq)]
pub enum Line {
    /// A K2 command and the reason given for it.
    Command { cmd: Command, reason: String },
    /// Advance the simulation by this many ticks.
    Run(u64),
    /// Advance until no operation is running, at most this many ticks.
    RunDone(u64),
    /// Print the usage text.
    Help,
}

/// The usage text. Every write (`set-goal`, `halt`, `recall`) needs `--reason`; without it the command is refused (K2 §4.3).
pub const HELP: &str = "commands:
  set-goal obtain <item> <n> [--priority P] --reason \"text\"   start a goal
  halt [scope] --reason \"text\"       stop work now (scope: swarm, op1, sq1, bot3, op:<id>, ...)
  recall [scope] --reason \"text\"     stop new work
  status | inspect <op|goal>          read-only, no reason needed
  run N | run-done [N]                advance the simulation
A write without --reason is refused and audited. Quote a reason of several words.";

/// Split on spaces; a double-quoted part is one word (so `--reason \"need logs\"` works).
// Spec: specs/K4-human-interface.md §4.1
fn tokenize(line: &str) -> Result<Vec<String>, String> {
    let (mut out, mut cur, mut quoted, mut any) = (vec![], String::new(), false, false);
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                any = true;
            }
            c if c.is_whitespace() && !quoted => {
                if any {
                    out.push(std::mem::take(&mut cur));
                    any = false;
                }
            }
            c => {
                cur.push(c);
                any = true;
            }
        }
    }
    if quoted {
        return Err("unclosed quote".into());
    }
    if any {
        out.push(cur);
    }
    Ok(out)
}

/// A scope word: `swarm`, `op:<id>`, `squad:<id>`, `bot:<id>`, `server:<id>`, or a bare id (`op1`, `sq1`, `bot3`).
// Spec: specs/K4-human-interface.md §4.1
fn scope(w: &str) -> Result<Scope, String> {
    let (kind, id) = match w.split_once(':') {
        Some((k, i)) => (k, i),
        None if w == "swarm" => return Ok(Scope::Swarm),
        None if w.starts_with("op") => ("op", w),
        None if w.starts_with("sq") => ("squad", w),
        None => ("bot", w),
    };
    Ok(match kind {
        "swarm" => Scope::Swarm,
        "op" => Scope::Operation(OperationId::new(id)),
        "squad" => Scope::Squad(SquadId::new(id)),
        "bot" => Scope::Bot(BotId::new(id)),
        "server" => Scope::Server(ServerId::new(id)),
        _ => return Err(format!("unknown scope `{w}`")),
    })
}

/// Parse `set-goal obtain oak_log 64 [--priority P] --reason R`, `status`, `inspect <op|goal>`,
/// `halt [scope] --reason R`, `recall [scope] --reason R`, `run N`, `run-done [N]`, `help`. `next_goal` names a new
/// goal (`g1`, `g2`, ...). A missing reason stays empty: the caller refuses and audits the write (K2 §4.3).
// Spec: specs/K4-human-interface.md §4.1
pub fn parse_line(line: &str, next_goal: u32) -> Result<Line, String> {
    let owned = tokenize(line)?;
    let mut words: Vec<&str> = owned.iter().map(String::as_str).collect();
    let mut reason = String::new();
    let mut priority = Priority(1);
    while let Some(i) = words.iter().position(|w| w.starts_with("--")) {
        let flag = words.remove(i);
        if i >= words.len() {
            // A bare trailing `--reason` is a write with an empty reason: the caller refuses and audits it (K2 §4.3).
            if flag == "--reason" {
                continue;
            }
            return Err(format!("{flag} needs a value"));
        }
        let v = words.remove(i);
        match flag {
            "--reason" => reason = v.to_string(),
            "--priority" => priority = Priority(v.parse().map_err(|_| format!("bad priority `{v}`"))?),
            _ => return Err(format!("unknown flag {flag}")),
        }
    }
    let num = |w: Option<&&str>, what: &str| -> Result<u64, String> {
        w.ok_or(format!("{what} is missing"))?.parse().map_err(|_| format!("{what} must be a number"))
    };
    match words.as_slice() {
        ["set-goal", "obtain", item, n] => {
            let item = if item.contains(':') { item.to_string() } else { format!("minecraft:{item}") };
            let n = num(Some(n), "quantity")? as u32;
            let spec = GoalSpec {
                id: GoalId::new(format!("g{next_goal}")),
                goal: Goal::Obtain { item: ItemId::new(item), n },
                priority,
                reason: reason.clone(),
            };
            Ok(Line::Command { cmd: Command::SetGoal(spec), reason })
        }
        ["set-goal", other, ..] => Err(format!("goal kind `{other}` is not supported yet; use `obtain`")),
        ["status"] => Ok(Line::Command { cmd: Command::Status, reason }),
        ["inspect", what] => {
            let target = if what.starts_with('g') {
                InspectTarget::Goal(GoalId::new(*what))
            } else {
                InspectTarget::Scope(scope(what)?)
            };
            Ok(Line::Command { cmd: Command::Inspect { target }, reason })
        }
        ["halt"] => Ok(Line::Command { cmd: Command::Halt { scope: Scope::Swarm }, reason }),
        ["halt", s] => Ok(Line::Command { cmd: Command::Halt { scope: scope(s)? }, reason }),
        ["recall"] => Ok(Line::Command { cmd: Command::Recall { scope: Scope::Swarm }, reason }),
        ["recall", s] => Ok(Line::Command { cmd: Command::Recall { scope: scope(s)? }, reason }),
        ["help"] => Ok(Line::Help),
        ["run", n] => Ok(Line::Run(num(Some(n), "ticks")?)),
        ["run-done"] => Ok(Line::RunDone(20_000)),
        ["run-done", n] => Ok(Line::RunDone(num(Some(n), "ticks")?)),
        [] => Err("empty command".into()),
        _ => Err(format!("unknown command `{line}`")),
    }
}

/// Text for a person.
// Spec: specs/K4-human-interface.md §4.1
pub fn render(r: &Reply) -> String {
    match r {
        Reply::Accepted { effect } => effect.clone(),
        Reply::Rejected { why } => format!("refused: {why}"),
        Reply::Status(d) => {
            let mut out = format!("digest #{} at tick {}\n", d.generation, d.at.0);
            for row in &d.rows {
                out += &format!(
                    "  {} goal {} {:?} bots {}/{} progress {}/{}{}{}\n",
                    row.op,
                    row.goal,
                    row.state,
                    row.headcount.alive,
                    row.headcount.assigned,
                    row.progress.done,
                    row.progress.target,
                    if row.stale { " STALE" } else { "" },
                    row.top_issue.as_ref().map(|t| format!(" issue: {t}")).unwrap_or_default(),
                );
            }
            for w in &d.warnings {
                out += &format!("  warning: {w}\n");
            }
            out.trim_end().to_string()
        }
        Reply::Inspect(i) => serde_json::to_string_pretty(i).expect("report serializes"),
    }
}
