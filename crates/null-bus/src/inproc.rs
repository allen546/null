//! In-process bus on tokio channels (F6 §4.6: same types, same trait as the NATS bus will have).
// Spec: specs/F6-bus-deployment.md §4.6

use crate::*;
use null_proto::{ClaimReply, ClaimRequest, Command, Escalation, Intent, LogEvent, SummaryBody, WallMs};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use tokio::sync::{mpsc, oneshot, watch};

/// A queue whose receiver waits for its consumer; messages sent before that stay buffered.
// Spec: specs/F6-bus-deployment.md §4.6
struct Queue<T> {
    tx: mpsc::Sender<T>,
    rx: Option<mpsc::Receiver<T>>,
}

impl<T> Queue<T> {
    fn new(cap: usize) -> Self {
        let (tx, rx) = mpsc::channel(cap);
        Self { tx, rx: Some(rx) }
    }
}

// Spec: specs/F6-bus-deployment.md §4.6
#[derive(Default)]
struct State {
    intents: HashMap<Subject, Queue<Intent>>,
    escalations: HashMap<Subject, Queue<Escalation>>,
    claims: HashMap<Subject, Queue<ClaimCall>>,
    summaries: HashMap<Subject, watch::Sender<Option<SummaryBody>>>,
    logs: HashMap<Subject, Vec<LogEvent>>,
    commands: HashMap<Subject, Vec<mpsc::UnboundedSender<Command>>>,
    /// Commands sent while a subject had no subscriber; the first subscriber gets them.
    held_commands: HashMap<Subject, Vec<Command>>,
}

// Spec: specs/F6-bus-deployment.md §4.6
#[derive(Clone)]
pub struct InProcBus {
    cap: usize,
    state: Arc<Mutex<State>>,
}

impl Default for InProcBus {
    fn default() -> Self {
        Self::with_capacity(1024)
    }
}

impl InProcBus {
    /// `cap` is the depth of each queued class. A full intent queue makes publishers wait; commands are unaffected.
    pub fn with_capacity(cap: usize) -> Self {
        Self { cap, state: Arc::default() }
    }

    fn queue_tx<T>(&self, subject: &Subject, pick: impl FnOnce(&mut State) -> &mut HashMap<Subject, Queue<T>>) -> mpsc::Sender<T> {
        let cap = self.cap;
        let mut st = self.state.lock().unwrap();
        pick(&mut st).entry(subject.clone()).or_insert_with(|| Queue::new(cap)).tx.clone()
    }

    fn queue_rx<T>(
        &self,
        subject: &Subject,
        pick: impl FnOnce(&mut State) -> &mut HashMap<Subject, Queue<T>>,
    ) -> Result<mpsc::Receiver<T>, BusError> {
        let cap = self.cap;
        let mut st = self.state.lock().unwrap();
        pick(&mut st).entry(subject.clone()).or_insert_with(|| Queue::new(cap)).rx.take().ok_or(BusError::AlreadySubscribed)
    }

    fn summary_tx(&self, subject: &Subject) -> watch::Sender<Option<SummaryBody>> {
        let mut st = self.state.lock().unwrap();
        st.summaries.entry(subject.clone()).or_insert_with(|| watch::channel(None).0).clone()
    }
}

// Spec: specs/F6-bus-deployment.md §3
impl Bus for InProcBus {
    async fn publish_intent(&self, subject: &Subject, intent: Intent) -> Result<(), BusError> {
        let tx = self.queue_tx(subject, |s| &mut s.intents);
        tx.send(intent).await.map_err(|_| BusError::Closed)
    }

    fn subscribe_intents(&self, subject: &Subject) -> Result<IntentRx, BusError> {
        let rx = self.queue_rx(subject, |s| &mut s.intents)?;
        Ok(KeyedRx { rx, seen: HashSet::new(), key: |i| &i.key })
    }

    fn publish_summary(&self, subject: &Subject, body: SummaryBody) {
        self.summary_tx(subject).send_replace(Some(body));
    }

    fn subscribe_summary(&self, subject: &Subject) -> SummaryRx {
        SummaryRx { rx: self.summary_tx(subject).subscribe() }
    }

    async fn publish_escalation(&self, subject: &Subject, e: Escalation) -> Result<(), BusError> {
        let tx = self.queue_tx(subject, |s| &mut s.escalations);
        tx.send(e).await.map_err(|_| BusError::Closed)
    }

    fn subscribe_escalations(&self, subject: &Subject) -> Result<EscalationRx, BusError> {
        let rx = self.queue_rx(subject, |s| &mut s.escalations)?;
        Ok(KeyedRx { rx, seen: HashSet::new(), key: |e| &e.key })
    }

    async fn request_claim(&self, subject: &Subject, req: ClaimRequest, timeout: WallMs) -> Result<ClaimReply, BusError> {
        let tx = self.queue_tx(subject, |s| &mut s.claims);
        let (reply, rx) = oneshot::channel();
        let call = async {
            tx.send(ClaimCall { req, reply }).await.map_err(|_| BusError::Closed)?;
            rx.await.map_err(|_| BusError::NoReply)
        };
        tokio::time::timeout(timeout.as_duration(), call).await.map_err(|_| BusError::Timeout)?
    }

    fn serve_claims(&self, subject: &Subject) -> Result<ClaimServer, BusError> {
        Ok(ClaimServer { rx: self.queue_rx(subject, |s| &mut s.claims)? })
    }

    fn append_event(&self, subject: &Subject, ev: LogEvent) -> u64 {
        let mut st = self.state.lock().unwrap();
        let log = st.logs.entry(subject.clone()).or_default();
        log.push(ev);
        log.len() as u64 - 1
    }

    fn read_events(&self, subject: &Subject, from_seq: u64) -> Vec<(u64, LogEvent)> {
        let st = self.state.lock().unwrap();
        let Some(log) = st.logs.get(subject) else { return vec![] };
        log.iter().enumerate().skip(from_seq as usize).map(|(i, e)| (i as u64, e.clone())).collect()
    }

    fn send_command(&self, subject: &Subject, cmd: Command) -> Result<(), BusError> {
        if !cmd.is_urgent() {
            return Err(BusError::NotUrgent);
        }
        let mut st = self.state.lock().unwrap();
        let delivered = match st.commands.get_mut(subject) {
            Some(subs) => {
                subs.retain(|s| s.send(cmd.clone()).is_ok());
                !subs.is_empty()
            }
            None => false,
        };
        // Commands are at least once (F6 §3): a halt sent before anyone listens must not vanish.
        // The first subscriber gets the held ones; simpler than a per-subscriber replay log.
        if !delivered {
            st.held_commands.entry(subject.clone()).or_default().push(cmd);
        }
        Ok(())
    }

    fn subscribe_commands(&self, subject: &Subject) -> CommandRx {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut st = self.state.lock().unwrap();
        for cmd in st.held_commands.remove(subject).unwrap_or_default() {
            let _ = tx.send(cmd);
        }
        st.commands.entry(subject.clone()).or_default().push(tx);
        CommandRx { rx }
    }
}
