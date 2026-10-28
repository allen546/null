//! Subjects: `null.<server>.<level>.<id>.<kind>`; the id is left out for singletons like `null.smp1.hq.digest`.
// Spec: specs/F6-bus-deployment.md §3

use null_proto::{BotId, ServerId};
use std::fmt;

// Spec: specs/F6-bus-deployment.md §4.2
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    Hq,
    Op,
    Squad,
    Node,
    Bot,
}

impl Level {
    pub fn as_str(self) -> &'static str {
        match self {
            Level::Hq => "hq",
            Level::Op => "op",
            Level::Squad => "squad",
            Level::Node => "node",
            Level::Bot => "bot",
        }
    }
    fn parse(s: &str) -> Option<Level> {
        [Level::Hq, Level::Op, Level::Squad, Level::Node, Level::Bot].into_iter().find(|l| l.as_str() == s)
    }
}

/// Kind names used by the skeleton.
pub mod kind {
    pub const JOBS: &str = "jobs";
    pub const SUMMARY: &str = "summary";
    pub const DIGEST: &str = "digest";
    pub const ESCALATE: &str = "escalate";
    pub const CLAIMS: &str = "claims";
    pub const EVENTS: &str = "events";
    pub const CMD: &str = "cmd";
}

// Spec: specs/F6-bus-deployment.md §4.2
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Subject {
    pub server: ServerId,
    pub level: Level,
    pub id: Option<String>,
    pub kind: String,
}

impl Subject {
    pub fn new(server: &ServerId, level: Level, id: Option<&str>, kind: &str) -> Self {
        Self { server: server.clone(), level, id: id.map(str::to_owned), kind: kind.to_owned() }
    }

    pub fn parse(s: &str) -> Option<Subject> {
        let parts: Vec<&str> = s.split('.').collect();
        let (server, level, id, kind) = match parts.as_slice() {
            ["null", server, level, kind] => (server, level, None, kind),
            ["null", server, level, id, kind] => (server, level, Some(*id), kind),
            _ => return None,
        };
        if [server, level, kind].iter().any(|p| p.is_empty()) || id == Some("") {
            return None;
        }
        Some(Subject::new(&ServerId::new(*server), Level::parse(level)?, id, kind))
    }
}

impl fmt::Display for Subject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "null.{}.{}", self.server, self.level.as_str())?;
        if let Some(id) = &self.id {
            write!(f, ".{id}")?;
        }
        write!(f, ".{}", self.kind)
    }
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot receives `IntentBody::JobBatch`.
pub fn bot_jobs(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::JOBS)
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot publishes `IntentBody::JobOutcome` (its squad consumes it).
pub fn bot_up(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), "up")
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot publishes `SummaryBody::Bot`.
pub fn bot_summary(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::SUMMARY)
}

// Spec: specs/F6-bus-deployment.md §4.2
/// Where a bot receives `Command::Halt`.
pub fn bot_cmd(server: &ServerId, bot: &BotId) -> Subject {
    Subject::new(server, Level::Bot, Some(bot.as_str()), kind::CMD)
}
