//! The adapter's side of the result mailbox (AD-F5.4): ties each command's `ActionId` to the
//! fork's result for it. The fork keeps every in-flight placement, break and block use by id and
//! settles them by the server's cumulative ack (fork patch 25), so this reads each result by id.
//! Entity uses and clicks have no sequence number: the fork keeps one record of each, and this
//! copies it every tick and keeps the copy when a newer action takes the record.
//! No game logic: the fork judges; this maps and bounds.
// Spec: specs/B6-placement-breaking.md §4.6 (the result mailbox)

use azalea::{
    block_actions::{
        BlockActionOutcome, BlockActionResult, BlockActionResults, UseBlockOutcome, UseBlockResult,
    },
    entity_actions::{EntityUseResults, UseEntityOutcome, UseEntityResult},
    inventory::ClickResults,
    respawn::RespawnLog,
};
use null_bot_api::{
    ActionId, ActionRecord, BlockAction, BlockActionKind, BlockOutcome, BlockPos, ClickOutcome, EntityId,
    Mailbox, OpenOutcome, OpenResult, OpenTarget, Owner, RespawnSent,
};

/// A command whose fork result has not appeared this many ticks after it was applied is judged
/// "not started" (the fork dropped it, or its input never arrived).
pub const ADOPT_TICKS: u64 = 20;
/// A command with no result and nothing queued for this many refreshes in a row was dropped by
/// the fork.
const UNQUEUED_REFRESHES: u8 = 4;
/// A click settles this many ticks after it was sent: the server's corrections come within a few
/// (the ability check waits the same for them).
pub const CLICK_SETTLE_TICKS: u32 = 8;
/// A respawn request with no respawn packet from the server after this many ticks (30 s) is
/// finished as "no answer".
pub const RESPAWN_TICKS: u64 = 600;
/// A break that has not sent its finishing packet after this many ticks (5 minutes, past the
/// longest vanilla hand-break, obsidian at about 250 s) is cancelled, as `cancel_break` would.
pub const BREAK_MAX_TICKS: u64 = 6000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Break,
    Place,
    UseBlock,
    UseEntity,
    Click,
    Inventory,
    Respawn,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Block(BlockPos),
    Entity(EntityId),
    None,
}

/// What the fork shows this tick: its result records and whether each queued command is still
/// waiting in its component.
pub struct Fork<'a> {
    pub now: u64,
    pub client_tick: u64,
    pub blocks: Option<&'a BlockActionResults>,
    pub entity: Option<&'a EntityUseResults>,
    pub clicks: Option<&'a ClickResults>,
    pub respawn: Option<&'a RespawnLog>,
    /// `Has<BreakBlockQueued>`, `Has<PlaceBlockQueued>`, `Has<UseBlockQueued>`, `Has<UseEntityQueued>`.
    pub queued: [bool; 4],
}

impl Fork<'_> {
    fn queued(&self, kind: Kind) -> bool {
        match kind {
            Kind::Break => self.queued[0],
            Kind::Place => self.queued[1],
            Kind::UseBlock => self.queued[2],
            Kind::UseEntity => self.queued[3],
            _ => false,
        }
    }
    /// The ids of the fork's results of `kind` above `after`, oldest first.
    fn ids_after(&self, kind: Kind, after: u32) -> Vec<u32> {
        match kind {
            Kind::Break | Kind::Place => self
                .blocks
                .map(|b| b.blocks_after(kind == Kind::Break, after).iter().map(|r| r.id).collect())
                .unwrap_or_default(),
            Kind::UseBlock => {
                self.blocks.map(|b| b.uses_after(after).iter().map(|r| r.id).collect()).unwrap_or_default()
            }
            Kind::UseEntity => {
                self.entity.and_then(|e| e.last_use).filter(|r| r.id > after).map(|r| vec![r.id]).unwrap_or_default()
            }
            _ => Vec::new(),
        }
    }
    /// The highest id of `kind` now (0: none yet).
    fn current_id(&self, kind: Kind) -> u32 {
        self.ids_after(kind, 0).last().copied().unwrap_or(0)
    }
    /// A fork tick on the host clock (both advance once per game tick).
    fn host_tick(&self, client: u64) -> u64 {
        (self.now + client).saturating_sub(self.client_tick)
    }
}

/// What the host must do after a refresh.
#[derive(Default)]
pub struct Refresh {
    /// A break outlived `BREAK_MAX_TICKS`: send `StopMiningQueued`.
    pub cancel_break: bool,
}

struct Track {
    kind: Kind,
    /// The ids that name this one action: repeated commands for the same break or respawn alias.
    ids: Vec<ActionId>,
    issued: u64,
    target: Target,
    /// The fork's result id once found (blocks, uses).
    fork_id: Option<u32>,
    adopted_tick: u64,
    /// Click: the slot asked for. Respawn: the fork tick of the request it matched.
    slot: Option<u16>,
    respawn_sent: Option<u64>,
    /// The packet that the server's ack settles has gone out (so a replaced record is unknown, not
    /// cancelled).
    sent: bool,
    /// The latest pending record, kept for when the fork replaces its own.
    last: Option<ActionRecord>,
    cancelling: bool,
    /// Refreshes in a row that found no result and no queued command. A break passes through
    /// several fork stages (queued, held for the destroy delay, mining queued), so one gap is not
    /// a drop.
    unqueued: u8,
}

#[derive(Default)]
struct Seen {
    init: bool,
    place: u32,
    brk: u32,
    use_block: u32,
    use_entity: u32,
    click_ticks: Option<u32>,
    respawn_sent: Option<u64>,
}

/// A bot's mailbox and the tracks that feed it.
#[derive(Default)]
pub struct Tracker {
    pub mailbox: Mailbox,
    tracks: Vec<Track>,
    seen: Seen,
}

enum Step {
    Keep(Option<ActionRecord>),
    Finish(ActionRecord),
}

// Spec: specs/B6-placement-breaking.md §4.6 (outcomes as the api's BlockAction)
fn block_record(
    kind: BlockActionKind,
    issued: u64,
    target: BlockPos,
    r: Option<&BlockActionResult>,
) -> BlockAction {
    let mut a = BlockAction {
        kind,
        pos: target,
        outcome: BlockOutcome::Pending,
        start_tick: issued,
        finish_tick: None,
        ack_tick: None,
        reason: None,
    };
    if let Some(r) = r {
        a.outcome = match r.outcome {
            BlockActionOutcome::Pending => BlockOutcome::Pending,
            BlockActionOutcome::Accepted => BlockOutcome::Done,
            BlockActionOutcome::Refused => BlockOutcome::Refused,
            BlockActionOutcome::Cancelled => BlockOutcome::Cancelled,
        };
        a.reason = r.refusal.map(|w| w.as_str());
        // The fork knows the real target (a replaceable clicked block is placed into).
        a.pos = BlockPos::new(r.position.x, r.position.y, r.position.z);
        a.finish_tick = r.finish_ticks.map(|t| issued + t as u64);
        // For a break the fork counts the ack from the finishing packet.
        a.ack_tick = r.ack_ticks.map(|t| issued + r.finish_ticks.unwrap_or(0) as u64 + t as u64);
    }
    a
}

fn open_from_block(pos: BlockPos, r: Option<&UseBlockResult>) -> OpenResult {
    let mut o = OpenResult { target: OpenTarget::Block(pos), outcome: OpenOutcome::Pending, refused: None, ticks: 0 };
    if let Some(r) = r {
        o.outcome = match r.outcome {
            UseBlockOutcome::Pending => OpenOutcome::Pending,
            UseBlockOutcome::Opened { window_id, menu } => OpenOutcome::Opened { window_id, kind: menu.to_str() },
            UseBlockOutcome::NoScreen => OpenOutcome::NoScreen,
            UseBlockOutcome::Refused => OpenOutcome::Refused,
        };
        o.refused = r.refusal.map(|w| w.as_str());
        o.ticks = r.elapsed_ticks;
    }
    o
}

fn open_from_entity(id: EntityId, r: Option<&UseEntityResult>) -> OpenResult {
    let mut o = OpenResult { target: OpenTarget::Entity(id), outcome: OpenOutcome::Pending, refused: None, ticks: 0 };
    if let Some(r) = r {
        o.outcome = match r.outcome {
            UseEntityOutcome::Pending => OpenOutcome::Pending,
            UseEntityOutcome::Opened { window_id, menu } => OpenOutcome::Opened { window_id, kind: menu.to_str() },
            UseEntityOutcome::NoScreen => OpenOutcome::NoScreen,
            UseEntityOutcome::Refused => OpenOutcome::Refused,
        };
        o.refused = r.refusal.map(|w| w.as_str());
        o.ticks = r.elapsed_ticks;
    }
    o
}

fn refused_open(target: OpenTarget, why: &'static str) -> ActionRecord {
    ActionRecord::Open(OpenResult { target, outcome: OpenOutcome::Refused, refused: Some(why), ticks: 0 })
}

fn click_record(slot: Option<u16>, c: &azalea::inventory::ClickResult, superseded: bool) -> ClickOutcome {
    let s = |s: i16| u16::try_from(s).ok();
    ClickOutcome {
        slot: s(c.slot_num).or(slot),
        refused: c.refusal.map(|r| r.reason()),
        ticks: c.ticks,
        corrected: c.corrected_slots.iter().map(|&x| s(x)).collect(),
        full_resends: c.full_resends,
        superseded,
    }
}

fn respawn_record(f: &Fork, r: &azalea::respawn::RespawnRecord) -> RespawnSent {
    RespawnSent {
        sent_tick: f.host_tick(r.sent_tick),
        screen_ticks: r.screen_ticks,
        immediate: r.immediate,
        completed_tick: r.completed_tick.map(|t| f.host_tick(t)),
        sends: r.sends,
        refused: None,
    }
}

fn no_respawn(why: &'static str) -> RespawnSent {
    RespawnSent { sent_tick: 0, screen_ticks: 0, immediate: false, completed_tick: None, sends: 0, refused: Some(why) }
}

impl Tracker {
    // Spec: specs/B6-placement-breaking.md §4.6 (every command gets an id and an owner)
    /// Registers a command's id (pending, no record). Called when the host takes the commands of a
    /// step, before the skill can end.
    pub fn open(&mut self, id: ActionId, owner: Owner) {
        self.mailbox.open(id, owner);
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (the command is applied: a track, or a result now)
    /// A command with a result was applied at host tick `now`. `queued`: the fork still holds an
    /// earlier command of this kind in its queued component (a new one replaces it).
    pub fn begin(&mut self, id: ActionId, kind: Kind, target: Target, slot: Option<u16>, now: u64, queued: bool) {
        if !self.mailbox.contains(id) {
            return; // the owner ended in the tick it sent this
        }
        // A repeated command for work already under way names the same action.
        let same = |t: &Track| match kind {
            Kind::Break => t.kind == Kind::Break && t.target == target,
            Kind::Respawn => t.kind == Kind::Respawn,
            _ => false,
        };
        if let Some(t) = self.tracks.iter_mut().find(|t| same(t)) {
            t.ids.push(id);
            if let Some(r) = t.last.clone() {
                self.mailbox.update(id, r);
            }
            return;
        }
        // The fork keeps one queued command per kind: a newer one replaces an older that has not
        // started, which then never gets a result of its own.
        if matches!(kind, Kind::Break | Kind::Place | Kind::UseBlock | Kind::UseEntity)
            && let Some(i) =
                self.tracks.iter().position(|t| t.kind == kind && t.fork_id.is_none() && (t.issued == now || queued))
        {
            let t = self.tracks.remove(i);
            let rec = match kind {
                Kind::Break => ActionRecord::Block(BlockAction {
                    outcome: BlockOutcome::Cancelled,
                    reason: Some("replaced by a later break before it started"),
                    ..block_record(BlockActionKind::Break, t.issued, block_of(t.target), None)
                }),
                Kind::Place => ActionRecord::Block(BlockAction {
                    outcome: BlockOutcome::Refused,
                    reason: Some("replaced by a later place before it started"),
                    ..block_record(BlockActionKind::Place, t.issued, block_of(t.target), None)
                }),
                Kind::UseBlock => refused_open(OpenTarget::Block(block_of(t.target)), "replaced by a later open before it started"),
                _ => refused_open(OpenTarget::Entity(entity_of(t.target)), "replaced by a later open before it started"),
            };
            self.finish_ids(&t.ids, rec);
        }
        if kind == Kind::Click
            && let Some(i) = self.tracks.iter().position(|t| t.kind == Kind::Click && t.fork_id.is_none() && t.issued == now)
        {
            // Two clicks in one tick: the fork keeps the later one's record.
            let t = self.tracks.remove(i);
            let c = ClickOutcome {
                slot: t.slot,
                refused: None,
                ticks: 0,
                corrected: Vec::new(),
                full_resends: 0,
                superseded: true,
            };
            self.finish_ids(&t.ids, ActionRecord::Click(c));
        }
        let pending = match kind {
            Kind::Break => Some(ActionRecord::Block(block_record(BlockActionKind::Break, now, block_of(target), None))),
            Kind::Place => Some(ActionRecord::Block(block_record(BlockActionKind::Place, now, block_of(target), None))),
            Kind::UseBlock => Some(ActionRecord::Open(open_from_block(block_of(target), None))),
            Kind::UseEntity => Some(ActionRecord::Open(open_from_entity(entity_of(target), None))),
            Kind::Inventory => Some(ActionRecord::Open(OpenResult {
                target: OpenTarget::Inventory,
                outcome: OpenOutcome::Pending,
                refused: None,
                ticks: 0,
            })),
            Kind::Click | Kind::Respawn => None,
        };
        if let Some(p) = &pending {
            self.mailbox.update(id, p.clone());
        }
        self.tracks.push(Track {
            kind,
            ids: vec![id],
            issued: now,
            target,
            fork_id: None,
            adopted_tick: 0,
            slot,
            respawn_sent: None,
            sent: false,
            last: pending,
            cancelling: false,
            unqueued: 0,
        });
    }

    // Spec: specs/F1-swarm-runtime.md §4.4 (a refused respawn is finished at once)
    /// A respawn the client refused at once (nothing sent): finished now.
    pub fn refuse_respawn(&mut self, id: ActionId, why: &'static str) {
        self.mailbox.finish(id, ActionRecord::Respawn(no_respawn(why)));
    }

    // Spec: specs/F1-swarm-runtime.md §4.4 (one respawn request per death)
    /// A respawn already requested for this death: names the request that went out.
    pub fn begin_respawn_sent(&mut self, id: ActionId, now: u64, sent_tick: u64) {
        self.begin(id, Kind::Respawn, Target::None, None, now, false);
        if let Some(t) = self.tracks.iter_mut().find(|t| t.kind == Kind::Respawn && t.ids.contains(&id)) {
            t.respawn_sent = Some(sent_tick);
            t.adopted_tick = now.max(1);
        }
    }

    fn finish_ids(&mut self, ids: &[ActionId], rec: ActionRecord) {
        for id in ids {
            self.mailbox.finish(*id, rec.clone());
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (a kept skill keeps its results across a death)
    /// A kept skill carries its results into the brain made at the respawn.
    pub fn adopt_skill_from(&mut self, old: &mut Tracker) {
        old.mailbox.move_owner(Owner::Skill, &mut self.mailbox);
        self.tracks = std::mem::take(&mut old.tracks);
        self.seen = std::mem::take(&mut old.seen);
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (copy the fork's records into the mailbox each tick)
    /// Runs at the start of each host tick: finds the fork's result for each new command, copies
    /// pending records, and finishes what settled, was replaced, or ran out of its time bound.
    pub fn refresh(&mut self, f: &Fork) -> Refresh {
        let mut out = Refresh::default();
        // Tracks of owners that ended are gone.
        let mb = &self.mailbox;
        self.tracks.retain_mut(|t| {
            t.ids.retain(|i| mb.contains(*i));
            !t.ids.is_empty()
        });
        if !self.seen.init {
            self.seen = Seen {
                init: true,
                place: f.current_id(Kind::Place),
                brk: f.current_id(Kind::Break),
                use_block: f.current_id(Kind::UseBlock),
                use_entity: f.current_id(Kind::UseEntity),
                click_ticks: f.clicks.and_then(|c| c.last.as_ref()).map(|c| c.ticks),
                respawn_sent: f.respawn.and_then(|l| l.last_respawn.as_ref()).map(|r| r.sent_tick),
            };
        }
        self.adopt(f);

        let mut tracks = std::mem::take(&mut self.tracks);
        let mut keep = Vec::with_capacity(tracks.len());
        for mut t in tracks.drain(..) {
            match Self::step(&mut t, f, &mut out) {
                Step::Keep(rec) => {
                    if let Some(r) = rec {
                        for id in &t.ids {
                            self.mailbox.update(*id, r.clone());
                        }
                        t.last = Some(r);
                    }
                    keep.push(t);
                }
                Step::Finish(rec) => self.finish_ids(&t.ids, rec),
            }
        }
        self.tracks = keep;
        out
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (matching a command to the fork's result)
    /// Hands each new fork result to the oldest command of its kind that has none.
    fn adopt(&mut self, f: &Fork) {
        for kind in [Kind::Break, Kind::Place, Kind::UseBlock, Kind::UseEntity] {
            let seen = match kind {
                Kind::Break => &mut self.seen.brk,
                Kind::Place => &mut self.seen.place,
                Kind::UseBlock => &mut self.seen.use_block,
                _ => &mut self.seen.use_entity,
            };
            // Several results of a kind can appear between two refreshes (a catch-up of ticks):
            // the oldest command gets the oldest result.
            for id in f.ids_after(kind, *seen) {
                *seen = id;
                if let Some(t) = self.tracks.iter_mut().find(|t| t.kind == kind && t.fork_id.is_none()) {
                    t.fork_id = Some(id);
                    t.adopted_tick = f.now.max(1);
                }
            }
        }
        // Clicks have no id: a new click is one whose tick count did not advance by one.
        let cur = f.clicks.and_then(|c| c.last.as_ref()).map(|c| c.ticks);
        let new_click = match (self.seen.click_ticks, cur) {
            (None, Some(_)) => true,
            (Some(p), Some(c)) => c <= p,
            _ => false,
        };
        self.seen.click_ticks = cur;
        if new_click {
            // The click before it is cut short: later corrections are the new click's.
            for t in self.tracks.iter_mut().filter(|t| t.kind == Kind::Click && t.fork_id == Some(0)) {
                t.fork_id = Some(1); // marks "replaced": step() finishes it from its last snapshot
            }
            if let Some(t) = self.tracks.iter_mut().find(|t| t.kind == Kind::Click && t.fork_id.is_none()) {
                t.fork_id = Some(0);
                t.adopted_tick = f.now.max(1);
            }
        }
        // Respawn: a new request on the fork's record.
        let cur = f.respawn.and_then(|l| l.last_respawn.as_ref()).map(|r| r.sent_tick);
        if cur.is_some() && cur != self.seen.respawn_sent {
            self.seen.respawn_sent = cur;
            if let Some(t) = self.tracks.iter_mut().find(|t| t.kind == Kind::Respawn && t.respawn_sent.is_none()) {
                t.respawn_sent = cur;
                t.adopted_tick = f.now.max(1);
            }
        }
    }

    // Spec: specs/B6-placement-breaking.md §4.6 (finish bounds per kind)
    fn step(t: &mut Track, f: &Fork, out: &mut Refresh) -> Step {
        let age = f.now.saturating_sub(t.issued);
        match t.kind {
            Kind::Break | Kind::Place => {
                let bk = if t.kind == Kind::Break { BlockActionKind::Break } else { BlockActionKind::Place };
                let target = block_of(t.target);
                let Some(fid) = t.fork_id else {
                    // Dropped by the fork: nothing is queued any more, and no result came.
                    t.unqueued = if f.queued(t.kind) { 0 } else { t.unqueued.saturating_add(1) };
                    if age > ADOPT_TICKS || t.unqueued >= UNQUEUED_REFRESHES {
                        let mut a = block_record(bk, t.issued, target, None);
                        a.outcome = BlockOutcome::Refused;
                        a.reason = Some("not started");
                        return Step::Finish(ActionRecord::Block(a));
                    }
                    return Step::Keep(None);
                };
                match f.blocks.and_then(|b| b.block_result(fid)) {
                    Some(r) => {
                        let mut a = block_record(bk, t.issued, target, Some(&r));
                        t.sent = r.seq.is_some();
                        if r.outcome == BlockActionOutcome::Pending {
                            if t.kind == Kind::Break
                                && !t.sent
                                && !t.cancelling
                                && f.now.saturating_sub(t.adopted_tick) > BREAK_MAX_TICKS
                            {
                                t.cancelling = true;
                                out.cancel_break = true;
                            }
                            return Step::Keep(Some(ActionRecord::Block(a)));
                        }
                        if t.cancelling && a.outcome == BlockOutcome::Cancelled {
                            a.reason = Some("too long");
                        }
                        Step::Finish(ActionRecord::Block(a))
                    }
                    // The fork no longer holds the result (its history is bounded, or the component
                    // was reset): the outcome is unknown. Not reached while a reader polls each tick.
                    None => {
                        let mut a = match &t.last {
                            Some(ActionRecord::Block(a)) => *a,
                            _ => block_record(bk, t.issued, target, None),
                        };
                        if t.sent {
                            a.outcome = BlockOutcome::Superseded;
                        } else {
                            a.outcome = BlockOutcome::Cancelled;
                            a.reason = Some("replaced by a newer action before it started");
                        }
                        Step::Finish(ActionRecord::Block(a))
                    }
                }
            }
            Kind::UseBlock | Kind::UseEntity => {
                let Some(fid) = t.fork_id else {
                    t.unqueued = if f.queued(t.kind) { 0 } else { t.unqueued.saturating_add(1) };
                    if age > ADOPT_TICKS || t.unqueued >= UNQUEUED_REFRESHES {
                        let tgt = if t.kind == Kind::UseBlock {
                            OpenTarget::Block(block_of(t.target))
                        } else {
                            OpenTarget::Entity(entity_of(t.target))
                        };
                        return Step::Finish(refused_open(tgt, "not started"));
                    }
                    return Step::Keep(None);
                };
                let o = if t.kind == Kind::UseBlock {
                    f.blocks.and_then(|b| b.use_result(fid)).map(|r| open_from_block(block_of(t.target), Some(&r)))
                } else {
                    f.entity.and_then(|e| e.last_use).filter(|r| r.id == fid).map(|r| open_from_entity(entity_of(t.target), Some(&r)))
                };
                match o {
                    Some(o) if o.outcome == OpenOutcome::Pending => Step::Keep(Some(ActionRecord::Open(o))),
                    Some(o) => Step::Finish(ActionRecord::Open(o)),
                    None => {
                        let mut o = match &t.last {
                            Some(ActionRecord::Open(o)) => o.clone(),
                            _ => open_from_block(block_of(t.target), None),
                        };
                        o.outcome = OpenOutcome::Superseded;
                        Step::Finish(ActionRecord::Open(o))
                    }
                }
            }
            Kind::Inventory => {
                // `open_inventory` is local: the fork answers in the tick it is applied.
                if age == 0 {
                    return Step::Keep(None);
                }
                let refusal = f.clicks.and_then(|c| c.open_refusal).map(|r| r.reason());
                Step::Finish(ActionRecord::Open(OpenResult {
                    target: OpenTarget::Inventory,
                    outcome: if refusal.is_some() {
                        OpenOutcome::Refused
                    } else {
                        OpenOutcome::Opened { window_id: 0, kind: "minecraft:inventory" }
                    },
                    refused: refusal,
                    ticks: 0,
                }))
            }
            Kind::Click => {
                let cur = f.clicks.and_then(|c| c.last.as_ref());
                match (t.fork_id, cur) {
                    (None, _) if age > ADOPT_TICKS => Step::Finish(ActionRecord::Click(ClickOutcome {
                        slot: t.slot,
                        refused: Some("no result"),
                        ticks: 0,
                        corrected: Vec::new(),
                        full_resends: 0,
                        superseded: false,
                    })),
                    (None, _) => Step::Keep(None),
                    // A later click took over the fork's record: keep what we saw.
                    (Some(1), _) | (Some(_), None) => {
                        let mut c = match &t.last {
                            Some(ActionRecord::Click(c)) => c.clone(),
                            _ => click_record_empty(t.slot),
                        };
                        c.superseded = true;
                        Step::Finish(ActionRecord::Click(c))
                    }
                    (Some(_), Some(c)) => {
                        let rec = click_record(t.slot, c, false);
                        if rec.refused.is_some() || rec.ticks >= CLICK_SETTLE_TICKS {
                            Step::Finish(ActionRecord::Click(rec))
                        } else {
                            Step::Keep(Some(ActionRecord::Click(rec)))
                        }
                    }
                }
            }
            Kind::Respawn => {
                let rec = f.respawn.and_then(|l| l.last_respawn.as_ref());
                match (t.respawn_sent, rec) {
                    (Some(sent), Some(r)) if r.sent_tick == sent => {
                        let rec = respawn_record(f, r);
                        if rec.completed_tick.is_some() {
                            Step::Finish(ActionRecord::Respawn(rec))
                        } else if f.now.saturating_sub(t.adopted_tick) > RESPAWN_TICKS {
                            Step::Finish(ActionRecord::Respawn(RespawnSent { refused: Some("no answer"), ..rec }))
                        } else {
                            Step::Keep(Some(ActionRecord::Respawn(rec)))
                        }
                    }
                    // A newer death's request replaced the record; ours completed before that.
                    (Some(_), _) => Step::Finish(ActionRecord::Respawn(match &t.last {
                        Some(ActionRecord::Respawn(r)) => r.clone(),
                        _ => no_respawn("no answer"),
                    })),
                    (None, _) if age > RESPAWN_TICKS => Step::Finish(ActionRecord::Respawn(no_respawn("no answer"))),
                    (None, _) => Step::Keep(None),
                }
            }
        }
    }
}

fn click_record_empty(slot: Option<u16>) -> ClickOutcome {
    ClickOutcome { slot, refused: None, ticks: 0, corrected: Vec::new(), full_resends: 0, superseded: false }
}

fn block_of(t: Target) -> BlockPos {
    match t {
        Target::Block(p) => p,
        _ => BlockPos::new(0, 0, 0),
    }
}

fn entity_of(t: Target) -> EntityId {
    match t {
        Target::Entity(e) => e,
        _ => EntityId(0),
    }
}
