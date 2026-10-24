//! The audit log: every command with who, what, why and the effect (K2 §4.3). Append-only JSONL.
use null_hq::Reply;
use null_proto::{Command, Gt, WallMs};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

// Spec: specs/K2-commander-api.md §4.3
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AuditEntry {
    pub at: WallMs,
    pub tick: Gt,
    pub principal: String,
    pub command: Command,
    pub reason: String,
    pub effect: String,
    pub accepted: bool,
}

// Spec: specs/K2-commander-api.md §4.3
#[derive(Default)]
pub struct Audit {
    file: Option<std::fs::File>,
    pub entries: Vec<AuditEntry>,
}

impl Audit {
    /// Opens the file for appending: K2 §4.3 says the log is append-only and kept long, so a new run adds to it.
    pub fn to_file(path: &Path) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self { file: Some(file), entries: vec![] })
    }

    /// Record a command and what came of it. The wall time derives from the tick, so a seeded run writes the same lines every time.
    pub fn record(&mut self, tick: Gt, principal: &str, command: &Command, reason: &str, reply: &Reply) {
        let (effect, accepted) = match reply {
            Reply::Accepted { effect } => (effect.clone(), true),
            Reply::Rejected { why } => (why.clone(), false),
            Reply::Status(d) => (format!("digest generation {}", d.generation), true),
            Reply::Inspect(_) => ("inspect report".into(), true),
        };
        let e = AuditEntry {
            at: WallMs(tick.0 * 50),
            tick,
            principal: principal.into(),
            command: command.clone(),
            reason: reason.into(),
            effect,
            accepted,
        };
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{}", serde_json::to_string(&e).expect("audit entry serializes"));
        }
        self.entries.push(e);
    }
}
