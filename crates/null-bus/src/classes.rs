//! Receiver handles for each class. They wrap tokio channels, so another transport can feed them.
// Spec: specs/F6-bus-deployment.md §3

use null_proto::{ClaimReply, ClaimRequest, Command, Escalation, Intent, SummaryBody};
use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot, watch};

/// Queue receiver that drops a message whose key it has seen: at-least-once in, applied once out.
// Spec: specs/F6-bus-deployment.md §4.3
pub struct KeyedRx<T> {
    pub(crate) rx: mpsc::Receiver<T>,
    // S2 keeps `seen` for the receiver's life; a bounded window is a later choice (F6 §4.3).
    pub(crate) seen: HashSet<String>,
    pub(crate) key: fn(&T) -> &str,
}

impl<T> KeyedRx<T> {
    pub async fn recv(&mut self) -> Option<T> {
        loop {
            let m = self.rx.recv().await?;
            if self.seen.insert((self.key)(&m).to_owned()) {
                return Some(m);
            }
        }
    }
    /// Next new message if one is queued now.
    pub fn try_recv(&mut self) -> Option<T> {
        while let Ok(m) = self.rx.try_recv() {
            if self.seen.insert((self.key)(&m).to_owned()) {
                return Some(m);
            }
        }
        None
    }
}

pub type IntentRx = KeyedRx<Intent>;
pub type EscalationRx = KeyedRx<Escalation>;

/// Holds only the newest summary on a subject; older ones are overwritten.
// Spec: specs/F6-bus-deployment.md §3
pub struct SummaryRx {
    pub(crate) rx: watch::Receiver<Option<SummaryBody>>,
}

impl SummaryRx {
    pub fn latest(&self) -> Option<SummaryBody> {
        self.rx.borrow().clone()
    }
    /// Waits for a summary newer than the last one seen, then returns the newest.
    pub async fn changed(&mut self) -> Option<SummaryBody> {
        self.rx.changed().await.ok()?;
        self.rx.borrow_and_update().clone()
    }
}

/// One claim request with the channel for its reply.
// Spec: specs/F6-bus-deployment.md §3
pub struct ClaimCall {
    pub req: ClaimRequest,
    pub(crate) reply: oneshot::Sender<ClaimReply>,
}

impl ClaimCall {
    pub fn reply(self, r: ClaimReply) {
        let _ = self.reply.send(r);
    }
}

// Spec: specs/F6-bus-deployment.md §3
pub struct ClaimServer {
    pub(crate) rx: mpsc::Receiver<ClaimCall>,
}

impl ClaimServer {
    pub async fn next(&mut self) -> Option<ClaimCall> {
        self.rx.recv().await
    }
}

/// Halt and recall arrive here, apart from the intent queue.
// Spec: specs/F6-bus-deployment.md §4.3
pub struct CommandRx {
    pub(crate) rx: mpsc::UnboundedReceiver<Command>,
}

impl CommandRx {
    pub async fn recv(&mut self) -> Option<Command> {
        self.rx.recv().await
    }
    pub fn try_recv(&mut self) -> Option<Command> {
        self.rx.try_recv().ok()
    }
}
