//! The bus: one trait, six message classes (F6 §3). Decision crate: no azalea, no bevy.
//! `InProcBus` runs on tokio channels for the skeleton; a NATS implementation sits behind `Bus` later (F6 §4.6).

mod classes;
mod inproc;
mod subject;
#[cfg(test)]
mod tests;

pub use classes::*;
pub use inproc::InProcBus;
pub use subject::*;

use null_proto::{ClaimReply, ClaimRequest, Command, Escalation, Intent, LogEvent, SummaryBody, WallMs};
use std::future::Future;

// Spec: specs/F6-bus-deployment.md §3
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BusError {
    /// A claim request got no reply in time.
    Timeout,
    /// The claim server dropped the request without replying.
    NoReply,
    /// `send_command` takes only halt and recall; other commands travel as intents.
    NotUrgent,
    /// The queue of this subject already has its consumer.
    AlreadySubscribed,
    /// The other end is gone.
    Closed,
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for BusError {}

/// Spec: specs/F6-bus-deployment.md §3. Methods are grouped by message class.
/// Queued classes (intent, escalation, claim) have one consumer per subject and buffer until it subscribes.
pub trait Bus: Send + Sync {
    // Intent: at least once, receivers dedupe by key. Waits when the queue is full.
    // Spec: specs/F6-bus-deployment.md §4.3
    fn publish_intent(&self, subject: &Subject, intent: Intent) -> impl Future<Output = Result<(), BusError>> + Send;
    fn subscribe_intents(&self, subject: &Subject) -> Result<IntentRx, BusError>;

    // Summary: latest wins, may drop.
    fn publish_summary(&self, subject: &Subject, body: SummaryBody);
    fn subscribe_summary(&self, subject: &Subject) -> SummaryRx;

    // Escalation: at least once, deduplicated by `Escalation::key`.
    fn publish_escalation(&self, subject: &Subject, e: Escalation) -> impl Future<Output = Result<(), BusError>> + Send;
    fn subscribe_escalations(&self, subject: &Subject) -> Result<EscalationRx, BusError>;

    // Claim request: request and reply with a timeout (wall time).
    fn request_claim(
        &self,
        subject: &Subject,
        req: ClaimRequest,
        timeout: WallMs,
    ) -> impl Future<Output = Result<ClaimReply, BusError>> + Send;
    fn serve_claims(&self, subject: &Subject) -> Result<ClaimServer, BusError>;

    // Event log: durable, append-only. `append` returns the sequence number (from 0); `read` is in order.
    fn append_event(&self, subject: &Subject, ev: LogEvent) -> u64;
    fn read_events(&self, subject: &Subject, from_seq: u64) -> Vec<(u64, LogEvent)>;

    // Command: halt and recall only, on their own channel, never queued behind other traffic.
    fn send_command(&self, subject: &Subject, cmd: Command) -> Result<(), BusError>;
    fn subscribe_commands(&self, subject: &Subject) -> CommandRx;
}
