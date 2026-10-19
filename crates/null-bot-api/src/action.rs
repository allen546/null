//! Action ids and the result mailbox (AD-F5.4). Pure: no game library, so the adapter and the
//! fake share it and the rules are unit-tested here.
// Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox)

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::types::{BlockAction, ClickOutcome, OpenResult, RespawnSent};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Names one result-bearing command. Ids come from one counter for the whole process, so an id
/// from before a reconnect never aliases a new one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(u64);

impl ActionId {
    /// A new id, never issued before in this process.
    pub fn fresh() -> Self {
        ActionId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
    pub fn raw(self) -> u64 {
        self.0
    }
}

/// Who issued a command: the running skill, or a reflex. The host stamps every command with it
/// (`Brain::tick` says which one is stepping); results are kept and pruned per owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Owner {
    Skill,
    Reflex(&'static str),
}

/// What a result holds, one variant per record type.
#[derive(Clone, Debug, PartialEq)]
pub enum ActionRecord {
    /// `break_block` or `place_block` (`BlockAction::kind` tells which).
    Block(BlockAction),
    /// `open_inventory`, `open_container` or `open_entity`.
    Open(OpenResult),
    Click(ClickOutcome),
    Respawn(RespawnSent),
}

/// What `BotView::action` returns for an id.
#[derive(Clone, Debug, PartialEq)]
pub enum ActionState {
    /// Not finished. Holds the partial record when there is one (a break in progress). Every kind
    /// finishes within a bounded time (B6 §4.6).
    Pending(Option<ActionRecord>),
    Finished(ActionRecord),
    /// Released, pruned (the owner ended, a disconnect) or never issued. Never reads as pending.
    Expired,
}

impl ActionState {
    pub fn is_pending(&self) -> bool {
        matches!(self, ActionState::Pending(_))
    }
    pub fn is_finished(&self) -> bool {
        matches!(self, ActionState::Finished(_))
    }
    pub fn is_expired(&self) -> bool {
        matches!(self, ActionState::Expired)
    }
    /// The record, partial or final.
    pub fn record(&self) -> Option<&ActionRecord> {
        match self {
            ActionState::Pending(r) => r.as_ref(),
            ActionState::Finished(r) => Some(r),
            ActionState::Expired => None,
        }
    }
    /// The final record.
    pub fn finished(&self) -> Option<&ActionRecord> {
        match self {
            ActionState::Finished(r) => Some(r),
            _ => None,
        }
    }
    /// The break or placement record, partial or final.
    pub fn block(&self) -> Option<&BlockAction> {
        match self.record() {
            Some(ActionRecord::Block(b)) => Some(b),
            _ => None,
        }
    }
    pub fn open(&self) -> Option<&OpenResult> {
        match self.record() {
            Some(ActionRecord::Open(o)) => Some(o),
            _ => None,
        }
    }
    pub fn click(&self) -> Option<&ClickOutcome> {
        match self.record() {
            Some(ActionRecord::Click(c)) => Some(c),
            _ => None,
        }
    }
    pub fn respawn(&self) -> Option<&RespawnSent> {
        match self.record() {
            Some(ActionRecord::Respawn(r)) => Some(r),
            _ => None,
        }
    }
}

/// Finished results an owner may hold before the first warning (`BotCommands::expect_results`).
pub const DEFAULT_EXPECTED: usize = 64;
/// Unreleased finished results that end the owner (a leak, AD-F5.4).
pub const LEAK_LIMIT: usize = 4096;

/// What the mailbox reports to the host; each is raised once per owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MailboxEvent {
    /// The owner holds more finished results than it expected. Nothing is pruned.
    OverExpected { owner: Owner, finished: usize, expected: usize },
    /// The owner holds `LEAK_LIMIT` unreleased finished results: end it.
    Leak { owner: Owner },
}

#[derive(Clone, Debug)]
enum Slot {
    Pending(Option<ActionRecord>),
    Finished(ActionRecord),
}

#[derive(Clone, Debug)]
struct Entry {
    owner: Owner,
    slot: Slot,
    /// The owner released it while pending: drop it the moment it finishes.
    release_on_finish: bool,
}

#[derive(Clone, Debug)]
struct OwnerInfo {
    /// Finished, unreleased results.
    finished: usize,
    expected: usize,
    warned: bool,
    leaked: bool,
}

impl Default for OwnerInfo {
    fn default() -> Self {
        Self { finished: 0, expected: DEFAULT_EXPECTED, warned: false, leaked: false }
    }
}

/// One bot's results. Never pruned by time: pending results stay until they finish, and finished
/// ones stay until their owner releases them, ends, or the bot disconnects.
#[derive(Clone, Debug)]
pub struct Mailbox {
    entries: HashMap<ActionId, Entry>,
    owners: HashMap<Owner, OwnerInfo>,
    events: Vec<MailboxEvent>,
    leak_limit: usize,
}

impl Default for Mailbox {
    fn default() -> Self {
        Self::with_leak_limit(LEAK_LIMIT)
    }
}

impl Mailbox {
    pub fn new() -> Self {
        Self::default()
    }
    /// A smaller leak limit, for tests.
    pub fn with_leak_limit(leak_limit: usize) -> Self {
        Self { entries: HashMap::new(), owners: HashMap::new(), events: Vec::new(), leak_limit }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (pending results are never refused)
    /// Registers a pending result for `owner`. Never refused.
    pub fn open(&mut self, id: ActionId, owner: Owner) {
        self.owners.entry(owner).or_default();
        self.entries.insert(id, Entry { owner, slot: Slot::Pending(None), release_on_finish: false });
    }

    pub fn contains(&self, id: ActionId) -> bool {
        self.entries.contains_key(&id)
    }

    /// Sets the partial record of a pending result. Ignored for a finished or unknown id.
    pub fn update(&mut self, id: ActionId, partial: ActionRecord) {
        if let Some(Entry { slot: slot @ Slot::Pending(_), .. }) = self.entries.get_mut(&id) {
            *slot = Slot::Pending(Some(partial));
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (expected count and leak guard)
    /// Finishes a pending result. Ignored for a finished or unknown id.
    pub fn finish(&mut self, id: ActionId, record: ActionRecord) {
        let Some(e) = self.entries.get_mut(&id) else { return };
        if matches!(e.slot, Slot::Finished(_)) {
            return;
        }
        if e.release_on_finish {
            self.entries.remove(&id);
            return;
        }
        e.slot = Slot::Finished(record);
        let owner = e.owner;
        let info = self.owners.entry(owner).or_default();
        info.finished += 1;
        if info.finished > info.expected && !info.warned {
            info.warned = true;
            self.events.push(MailboxEvent::OverExpected { owner, finished: info.finished, expected: info.expected });
        }
        if info.finished >= self.leak_limit && !info.leaked {
            info.leaked = true;
            self.events.push(MailboxEvent::Leak { owner });
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (expired is never pending)
    pub fn state(&self, id: ActionId) -> ActionState {
        match self.entries.get(&id).map(|e| &e.slot) {
            None => ActionState::Expired,
            Some(Slot::Pending(r)) => ActionState::Pending(r.clone()),
            Some(Slot::Finished(r)) => ActionState::Finished(r.clone()),
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (the owner releases a result after reading it)
    /// The owner lets go of `id`: a finished result is dropped now, a pending one when it
    /// finishes. Ignored when `owner` did not issue it.
    pub fn release(&mut self, owner: Owner, id: ActionId) {
        let Some(e) = self.entries.get_mut(&id).filter(|e| e.owner == owner) else { return };
        match e.slot {
            Slot::Pending(_) => e.release_on_finish = true,
            Slot::Finished(_) => {
                self.entries.remove(&id);
                if let Some(i) = self.owners.get_mut(&owner) {
                    i.finished = i.finished.saturating_sub(1);
                }
            }
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (expected count)
    /// How many finished results `owner` expects to hold before the warning.
    pub fn expect(&mut self, owner: Owner, n: usize) {
        self.owners.entry(owner).or_default().expected = n;
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (a skill's results go when it ends)
    /// The owner ended: all its results go, pending ones too, and its counters start over.
    pub fn end_owner(&mut self, owner: Owner) {
        self.entries.retain(|_, e| e.owner != owner);
        self.owners.remove(&owner);
        self.events.retain(|ev| match ev {
            MailboxEvent::OverExpected { owner: o, .. } | MailboxEvent::Leak { owner: o } => *o != owner,
        });
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (all results clear at a disconnect)
    /// A disconnect: everything goes.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.owners.clear();
        self.events.clear();
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (a kept skill keeps its results across a death)
    /// Moves everything `owner` holds into `to` (a kept skill carries its results across a death).
    pub fn move_owner(&mut self, owner: Owner, to: &mut Mailbox) {
        let ids: Vec<ActionId> = self.entries.iter().filter(|(_, e)| e.owner == owner).map(|(i, _)| *i).collect();
        for id in ids {
            if let Some(e) = self.entries.remove(&id) {
                to.entries.insert(id, e);
            }
        }
        if let Some(info) = self.owners.remove(&owner) {
            to.owners.insert(owner, info);
        }
    }

    /// Ids held by `owner`, any state.
    pub fn ids_of(&self, owner: Owner) -> Vec<ActionId> {
        self.entries.iter().filter(|(_, e)| e.owner == owner).map(|(i, _)| *i).collect()
    }

    /// Finished, unreleased results `owner` holds.
    pub fn finished_count(&self, owner: Owner) -> usize {
        self.owners.get(&owner).map_or(0, |i| i.finished)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Events raised since the last call.
    pub fn take_events(&mut self) -> Vec<MailboxEvent> {
        std::mem::take(&mut self.events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{BlockActionKind, BlockOutcome, BlockPos};

    fn rec(pos: i32) -> ActionRecord {
        ActionRecord::Block(BlockAction {
            kind: BlockActionKind::Break,
            pos: BlockPos::new(pos, 64, 0),
            outcome: BlockOutcome::Done,
            start_tick: 1,
            finish_tick: Some(5),
            ack_tick: Some(6),
            reason: None,
        })
    }

    fn open(m: &mut Mailbox, owner: Owner) -> ActionId {
        let id = ActionId::fresh();
        m.open(id, owner);
        id
    }

    #[test]
    fn ids_are_never_reused() {
        let a = ActionId::fresh();
        let b = ActionId::fresh();
        assert_ne!(a, b);
        assert!(b > a);
    }

    #[test]
    fn pending_then_finished_then_released() {
        let mut m = Mailbox::new();
        let id = open(&mut m, Owner::Skill);
        assert_eq!(m.state(id), ActionState::Pending(None));
        m.update(id, rec(1));
        assert_eq!(m.state(id), ActionState::Pending(Some(rec(1))));
        m.finish(id, rec(2));
        assert_eq!(m.state(id), ActionState::Finished(rec(2)));
        assert_eq!(m.finished_count(Owner::Skill), 1);
        m.release(Owner::Skill, id);
        assert!(m.state(id).is_expired());
        assert_eq!(m.finished_count(Owner::Skill), 0);
    }

    #[test]
    fn a_finished_result_is_not_changed_by_later_updates() {
        let mut m = Mailbox::new();
        let id = open(&mut m, Owner::Skill);
        m.finish(id, rec(1));
        m.update(id, rec(2));
        m.finish(id, rec(3));
        assert_eq!(m.state(id), ActionState::Finished(rec(1)));
        assert_eq!(m.finished_count(Owner::Skill), 1);
    }

    #[test]
    fn release_by_a_stranger_is_ignored() {
        let mut m = Mailbox::new();
        let id = open(&mut m, Owner::Skill);
        m.finish(id, rec(1));
        m.release(Owner::Reflex("r"), id);
        assert!(m.state(id).is_finished());
    }

    #[test]
    fn releasing_a_pending_result_drops_it_when_it_finishes() {
        let mut m = Mailbox::new();
        let id = open(&mut m, Owner::Skill);
        m.release(Owner::Skill, id);
        assert!(m.state(id).is_pending(), "still pending until the server answers");
        m.finish(id, rec(1));
        assert!(m.state(id).is_expired());
        assert_eq!(m.finished_count(Owner::Skill), 0);
    }

    #[test]
    fn unknown_and_pruned_ids_read_expired_never_pending() {
        let mut m = Mailbox::new();
        assert!(m.state(ActionId::fresh()).is_expired());
        let id = open(&mut m, Owner::Skill);
        m.end_owner(Owner::Skill);
        assert!(m.state(id).is_expired());
        assert!(!m.state(id).is_pending());
    }

    #[test]
    fn pending_results_are_never_pruned_and_never_refuse() {
        let mut m = Mailbox::with_leak_limit(8);
        let ids: Vec<_> = (0..10_000).map(|_| open(&mut m, Owner::Skill)).collect();
        assert!(ids.iter().all(|i| m.state(*i) == ActionState::Pending(None)));
        assert!(m.take_events().is_empty(), "pending results raise no warning and no leak");
    }

    #[test]
    fn ending_a_skill_drops_all_its_results_and_only_its_own() {
        let mut m = Mailbox::new();
        let (a, b) = (open(&mut m, Owner::Skill), open(&mut m, Owner::Skill));
        let r = open(&mut m, Owner::Reflex("flee"));
        m.finish(a, rec(1));
        m.finish(r, rec(2));
        m.end_owner(Owner::Skill);
        assert!(m.state(a).is_expired() && m.state(b).is_expired());
        assert!(m.state(r).is_finished());
        assert_eq!(m.finished_count(Owner::Skill), 0);
    }

    #[test]
    fn a_kept_skill_keeps_its_results_across_a_death() {
        let mut old = Mailbox::new();
        let (a, b) = (open(&mut old, Owner::Skill), open(&mut old, Owner::Skill));
        let r = open(&mut old, Owner::Reflex("flee"));
        old.finish(a, rec(1));
        let mut new = Mailbox::new();
        old.move_owner(Owner::Skill, &mut new);
        assert_eq!(new.state(a), ActionState::Finished(rec(1)));
        assert_eq!(new.state(b), ActionState::Pending(None));
        assert_eq!(new.finished_count(Owner::Skill), 1);
        assert!(old.state(a).is_expired());
        assert!(old.state(r).is_pending(), "the reflex's result stays behind with the old brain");
        assert!(new.state(r).is_expired());
    }

    #[test]
    fn a_disconnect_clears_everything() {
        let mut m = Mailbox::new();
        let a = open(&mut m, Owner::Skill);
        let r = open(&mut m, Owner::Reflex("x"));
        m.finish(a, rec(1));
        m.clear();
        assert!(m.state(a).is_expired() && m.state(r).is_expired());
        assert!(m.is_empty());
        // An id from before the clear never aliases a new one.
        assert_ne!(open(&mut m, Owner::Skill), a);
    }

    #[test]
    fn going_over_the_expected_count_warns_once_per_owner_and_prunes_nothing() {
        let mut m = Mailbox::new();
        m.expect(Owner::Skill, 3);
        let ids: Vec<_> = (0..6).map(|_| open(&mut m, Owner::Skill)).collect();
        for id in &ids {
            m.finish(*id, rec(0));
        }
        assert_eq!(
            m.take_events(),
            vec![MailboxEvent::OverExpected { owner: Owner::Skill, finished: 4, expected: 3 }]
        );
        assert!(ids.iter().all(|i| m.state(*i).is_finished()), "nothing pruned");
        // Another owner warns on its own default.
        let r: Vec<_> = (0..=DEFAULT_EXPECTED).map(|_| open(&mut m, Owner::Reflex("a"))).collect();
        for id in &r {
            m.finish(*id, rec(0));
        }
        assert_eq!(m.take_events().len(), 1);
        // A new skill instance may warn again.
        m.end_owner(Owner::Skill);
        m.expect(Owner::Skill, 0);
        let id = open(&mut m, Owner::Skill);
        m.finish(id, rec(0));
        assert_eq!(m.take_events().len(), 1);
    }

    #[test]
    fn the_leak_guard_fires_at_the_limit_once() {
        let mut m = Mailbox::new();
        m.expect(Owner::Skill, usize::MAX);
        for _ in 0..LEAK_LIMIT - 1 {
            let id = open(&mut m, Owner::Skill);
            m.finish(id, rec(0));
        }
        assert!(m.take_events().is_empty());
        let id = open(&mut m, Owner::Skill);
        m.finish(id, rec(0));
        assert_eq!(m.take_events(), vec![MailboxEvent::Leak { owner: Owner::Skill }]);
        let id = open(&mut m, Owner::Skill);
        m.finish(id, rec(0));
        assert!(m.take_events().is_empty(), "raised once");
    }

    #[test]
    fn released_results_do_not_count_toward_the_leak() {
        let mut m = Mailbox::with_leak_limit(5);
        for _ in 0..100 {
            let id = open(&mut m, Owner::Skill);
            m.finish(id, rec(0));
            m.release(Owner::Skill, id);
        }
        assert!(m.take_events().is_empty());
        assert!(m.is_empty());
    }

    #[test]
    fn state_helpers_read_the_record() {
        let s = ActionState::Pending(Some(rec(1)));
        assert!(s.block().is_some() && s.finished().is_none() && s.open().is_none());
        let f = ActionState::Finished(rec(1));
        assert_eq!(f.finished(), f.record());
    }
}
