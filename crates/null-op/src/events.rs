//! The operation's event payloads and the append-only log (K5 §4.4, §4.9).
//! A `LogEvent`'s `data` is the JSON of an [`OpEvent`]; the bus log and an optional JSONL file hold the same lines.
use null_bus::{Bus, Subject};
use null_proto::*;
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::claims::ClaimEvent;

/// Every change to the operation's state is one of these. Replaying them in order rebuilds the state.
// Spec: specs/K5-operation-service.md §4.9
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum OpEvent {
    GoalSet {
        goal: GoalSpec,
    },
    Allocated {
        bots: Vec<BotId>,
    },
    /// One `fell` job for one tree, with the logs the operation believes it holds.
    JobCreated {
        job: Job,
        tree: Loc,
        logs: u32,
    },
    Claim(ClaimEvent),
    /// The `attempt`-th dispatch of the job. Its claim is `claim_id(job, attempt)`.
    Dispatched {
        job: JobId,
        attempt: u32,
    },
    /// `key` identifies the outcome by (job, attempt), not by tick, so a resend counts once.
    Outcome {
        key: String,
        bot: BotId,
        attempt: u32,
        outcome: Outcome,
    },
    /// An outcome that was not counted or acted on (unknown job, unnumbered or never-dispatched attempt, unknown bot).
    Ignored {
        job: JobId,
        bot: BotId,
        attempt: u32,
        reason: String,
    },
    JobDone {
        job: JobId,
    },
    Requeued {
        job: JobId,
        reason: String,
    },
    JobDead {
        job: JobId,
        reason: String,
    },
    State {
        state: OpState,
    },
}

impl OpEvent {
    // Spec: specs/K5-operation-service.md §4.4
    pub fn kind(&self) -> &'static str {
        match self {
            OpEvent::GoalSet { .. } => "goal_set",
            OpEvent::Allocated { .. } => "allocated",
            OpEvent::JobCreated { .. } => "job_created",
            OpEvent::Claim(ClaimEvent::Granted { .. }) => "claim_granted",
            OpEvent::Claim(ClaimEvent::Renewed { .. }) => "claim_renewed",
            OpEvent::Claim(ClaimEvent::Released { .. }) => "claim_released",
            OpEvent::Claim(ClaimEvent::Expired { .. }) => "claim_expired",
            OpEvent::Claim(ClaimEvent::Preempted { .. }) => "claim_preempted",
            OpEvent::Dispatched { .. } => "dispatched",
            OpEvent::Outcome { .. } => "outcome",
            OpEvent::Ignored { .. } => "ignored",
            OpEvent::JobDone { .. } => "job_done",
            OpEvent::Requeued { .. } => "requeued",
            OpEvent::JobDead { .. } => "job_dead",
            OpEvent::State { .. } => "state",
        }
    }
}

/// Wall time of an event, derived from the game tick (50 ms each) so a seeded run writes the same log twice.
// Spec: specs/F3-test-harness.md §4.5
pub fn wall_of(now: Gt) -> WallMs {
    WallMs(now.0 * 50)
}

/// Append-only log on the bus, mirrored to a JSONL file when a path is given.
// Spec: specs/K5-operation-service.md §4.4
pub struct EventLog<B: Bus> {
    bus: B,
    subject: Subject,
    file: Option<std::fs::File>,
}

impl<B: Bus> EventLog<B> {
    pub fn new(bus: B, subject: Subject, jsonl: Option<PathBuf>) -> std::io::Result<Self> {
        let file = match jsonl {
            Some(p) => Some(
                std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(p)?,
            ),
            None => None,
        };
        Ok(Self { bus, subject, file })
    }
    pub fn append(&mut self, now: Gt, ev: &OpEvent) {
        let le = LogEvent {
            at: wall_of(now),
            kind: ev.kind().into(),
            data: serde_json::to_string(ev).expect("event serializes"),
        };
        if let Some(f) = &mut self.file {
            let _ = writeln!(
                f,
                "{}",
                serde_json::to_string(&le).expect("log event serializes")
            );
        }
        self.bus.append_event(&self.subject, le);
    }
    /// Everything logged so far, in order.
    pub fn read_all(&self) -> Vec<LogEvent> {
        self.bus
            .read_events(&self.subject, 0)
            .into_iter()
            .map(|(_, e)| e)
            .collect()
    }
}

/// Read a JSONL log file (one `LogEvent` per line).
// Spec: specs/K5-operation-service.md §4.9
pub fn load_jsonl(path: &Path) -> std::io::Result<Vec<LogEvent>> {
    std::fs::read_to_string(path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(std::io::Error::other))
        .collect()
}

/// Decode a log into events.
// Spec: specs/K5-operation-service.md §4.9
pub fn decode(events: &[LogEvent]) -> Vec<OpEvent> {
    events
        .iter()
        .filter_map(|e| serde_json::from_str(&e.data).ok())
        .collect()
}
