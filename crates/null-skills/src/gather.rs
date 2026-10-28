//! `gather(policy, quota, claim)` (B4 §3): find targets the policy accepts inside the claim, plan
//! stands and a break order, break through the mailbox, pick up drops, report.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use null_belief::Belief;
use null_bot_api::{ActionId, ActionRecord, ActionState, BlockOutcome, BlockPos, BotCommands, BotView, EntityId, NavStatus, Skill, SkillStatus};

use crate::policy::{Claim, Policy, Reason, Verdict};
use crate::reach::{self, Planner, Search, Task};

/// Ticks to wait for drops to spawn before looking for them.
const SETTLE_TICKS: u64 = 10;
/// Ticks between aiming and breaking (B6 §4.4: leave two ticks for the look to land).
const AIM_TICKS: u64 = 2;
/// A walk (to a stand or an item) that takes longer than this fails.
const WALK_TIMEOUT: u64 = 1200;
const ITEM_TIMEOUT: u64 = 200;
/// Drops count when they lie within this many blocks of a block broken this round (B4 §4.9).
const DROP_RADIUS: f64 = 6.0;
/// Targets planned in one round.
const ROUND_CAP: usize = 40;
/// The bot counts as on its stand within this distance of the stand's centre: exactly the slack
/// the planner allows for (reach is checked at the stand's corners), so an arrival the planner
/// did not allow for never fails the reach gate.
const ARRIVE: f64 = reach::STAND_SLACK;
/// Item entities near a block broken this job and still on the ground at the end are reported.
const LEFT_RADIUS: f64 = 2.0 * DROP_RADIUS;
/// Work one `step` may do while planning, in units (about one block read or one stand check each),
/// so a step takes a bounded time whatever the world (the tick loop must not stall). Measured
/// 2026-10-20: 6000 units cost 0.9 ms on a hashed fake view but 3-14 ms on the live bot (A14, p99
/// 3-10 ms); 1000 aims at about 2 ms live. One target's candidate stands (`GEN_UNITS`) are built in
/// one piece, so a step may still run past this.
pub const STEP_UNITS: u32 = 1000;
/// Units charged for asking the policy about one non-air cell (a tree is judged at its first log).
const POLICY_UNITS: u32 = 20;

pub struct GatherConfig {
    /// Blocks to gather; the skill stops at the quota.
    pub quota: u32,
    pub claim: Claim,
    /// This bot's name, recorded as the observer in belief.
    pub observer: String,
}

/// A target left standing, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    pub at: BlockPos,
    pub reason: Reason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Changed {
    pub pos: BlockPos,
    pub before: String,
    pub after: String,
}

/// What `gather` reports (B4 §3). Tool wear is not tracked: the view carries no durability yet.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GatherResult {
    /// Items gained over the job, by name.
    pub yield_items: BTreeMap<String, u32>,
    /// Targets broken.
    pub gathered: u32,
    pub left: Vec<Left>,
    /// Targets and cleared blocks, with before and after.
    pub changed: Vec<Changed>,
    pub ticks: u64,
    /// Done with fewer targets than the quota.
    pub short_of_quota: bool,
    pub failure: Option<String>,
    /// Item entities still on the ground near what was broken at the end (inventory full, out of
    /// reach or out of `DROP_RADIUS`; B4 §4.9). The view gives no stack info, so only positions.
    pub drops_left: Vec<BlockPos>,
}

enum Sub {
    Go { issued: Option<u64> },
    Aim { since: Option<u64> },
    Wait { id: ActionId },
}

struct Job {
    tasks: Vec<Task>,
    /// The block name at each task's cell when the plan was made.
    planned: Vec<Option<String>>,
    i: usize,
    sub: Sub,
    broken: Vec<BlockPos>,
}

/// Claim read in progress: the next cell and the targets found so far.
#[derive(Default)]
struct Scanner {
    i: u64,
    targets: Vec<BlockPos>,
}

struct PlanJob {
    clusters: Vec<Vec<BlockPos>>,
    ci: usize,
    need_left: usize,
    tasks: Vec<Task>,
    cur: BlockPos,
    removed: HashSet<BlockPos>,
    search: Option<Search>,
    depth: usize,
    targets: HashSet<BlockPos>,
    clear: HashSet<BlockPos>,
}

enum Sel {
    Scan(Scanner),
    Clear { clusters: Vec<Vec<BlockPos>>, need: usize, lo: BlockPos, hi: BlockPos, i: u64, clear: HashSet<BlockPos> },
    Plan(PlanJob),
}

enum Phase {
    Start,
    Select(Sel),
    /// The quota is met: read the claim once more to report what is left.
    Ending(Scanner),
    Run(Job),
    Settle { until: u64, broken: Vec<BlockPos> },
    Pick { broken: Vec<BlockPos>, visited: HashSet<EntityId>, current: Option<(EntityId, u64)> },
    Finished,
}

pub struct Gather {
    cfg: GatherConfig,
    policy: Box<dyn Policy>,
    belief: Arc<Mutex<Belief>>,
    refusal: Option<String>,
    phase: Phase,
    result: Arc<Mutex<GatherResult>>,
    start_tick: u64,
    start_items: HashMap<String, u32>,
    /// Cells already judged or reported, never offered again.
    done: HashSet<BlockPos>,
    /// Every cell broken in this job, for the drops report.
    broken_all: Vec<BlockPos>,
    /// Units spent in the current step, and the most in any step (for the budget test).
    units: u32,
    pub(crate) max_units: u32,
}

fn items(view: &dyn BotView) -> HashMap<String, u32> {
    let mut m = HashMap::new();
    for s in view.inventory().iter().flatten().chain(view.offhand()) {
        *m.entry(s.item.clone()).or_insert(0) += s.count as u32;
    }
    m
}

fn near(a: BlockPos, b: BlockPos) -> bool {
    (a.x - b.x).abs() <= 1 && (a.y - b.y).abs() <= 1 && (a.z - b.z).abs() <= 1
}

/// Groups of touching cells, as `Vec`s sorted bottom first.
fn clusters(cells: &[BlockPos]) -> Vec<Vec<BlockPos>> {
    let mut left: Vec<BlockPos> = cells.to_vec();
    let mut out = Vec::new();
    while let Some(seed) = left.pop() {
        let mut group = vec![seed];
        let mut i = 0;
        while i < group.len() {
            let (touch, rest): (Vec<_>, Vec<_>) = left.iter().partition(|c| near(group[i], **c));
            group.extend(touch);
            left = rest;
            i += 1;
        }
        group.sort_by_key(|p| (p.y, p.x, p.z));
        out.push(group);
    }
    out
}

impl Gather {
    // Spec: specs/B4-gathering.md §4.8 (a claim the skill cannot read is refused before any break)
    pub fn new(cfg: GatherConfig, policy: Box<dyn Policy>, belief: Arc<Mutex<Belief>>) -> Self {
        let c = cfg.claim;
        let valid = c.min.x <= c.max.x && c.min.y <= c.max.y && c.min.z <= c.max.z;
        let refusal = policy.refuses().or_else(|| (!valid).then(|| format!("invalid claim: min {:?} is past max {:?}", c.min, c.max)));
        Self { cfg, policy, belief, refusal, phase: Phase::Start, result: Default::default(), start_tick: 0, start_items: HashMap::new(), done: HashSet::new(), broken_all: Vec::new(), units: 0, max_units: 0 }
    }

    /// Refuse the job before it starts, with a reason (the skill ends `Failed`).
    pub fn refuse(mut self, why: &str) -> Self {
        self.refusal.get_or_insert_with(|| why.to_string());
        self
    }

    /// A live handle to the result, filled in as the skill runs and final when it ends.
    pub fn result_handle(&self) -> Arc<Mutex<GatherResult>> {
        self.result.clone()
    }

    pub fn result(&self) -> GatherResult {
        self.result.lock().unwrap().clone()
    }

    fn gathered(&self) -> u32 {
        self.result.lock().unwrap().gathered
    }

    fn leave(&mut self, at: BlockPos, reason: Reason) {
        self.result.lock().unwrap().left.push(Left { at, reason });
    }

    // Spec: specs/B4-gathering.md §3, §4.9 (yield, time, changed cells, targets left, drops left)
    fn end(&mut self, view: &dyn BotView, failure: Option<String>) -> SkillStatus {
        let now = items(view);
        let mut r = self.result.lock().unwrap();
        r.yield_items = now.into_iter().filter_map(|(k, v)| v.checked_sub(*self.start_items.get(&k).unwrap_or(&0)).filter(|d| *d > 0).map(|d| (k, d))).collect();
        r.ticks = view.tick().saturating_sub(self.start_tick);
        r.short_of_quota = r.gathered < self.cfg.quota;
        r.failure = failure.clone();
        r.drops_left = view.entities().iter().filter(|e| e.kind == "minecraft:item" && self.broken_all.iter().any(|b| e.position.distance(b.center()) <= LEFT_RADIUS)).map(|e| e.position.block_pos()).collect();
        self.phase = Phase::Finished;
        match failure {
            Some(f) => SkillStatus::Failed(f),
            None => SkillStatus::Done,
        }
    }

    fn spend(&mut self, n: u32) {
        self.units += n;
    }

    // Spec: specs/B4-gathering.md §4.8 (the policy judges every block; rejects are reported once per group)
    /// Reads part of the claim into belief and asks the policy about each non-air cell, within this
    /// step's unit budget. True when the whole claim has been read.
    fn scan_step(&mut self, view: &dyn BotView, sc: &mut Scanner) -> bool {
        let c = self.cfg.claim;
        let (dy, dz) = ((c.max.y - c.min.y + 1) as u64, (c.max.z - c.min.z + 1) as u64);
        let total = (c.max.x - c.min.x + 1) as u64 * dy * dz;
        if sc.i == 0 {
            self.policy.begin_scan();
        }
        let mut seen = Vec::new();
        while sc.i < total && self.units < STEP_UNITS {
            let i = sc.i;
            sc.i += 1;
            self.spend(1);
            let p = BlockPos::new(c.min.x + (i / (dy * dz)) as i32, c.min.y + ((i / dz) % dy) as i32, c.min.z + (i % dz) as i32);
            let Some(n) = view.block_at(p) else { continue };
            seen.push((p, n));
            if self.done.contains(&p) || null_belief::is_air(&seen.last().unwrap().1) {
                continue;
            }
            self.spend(POLICY_UNITS);
            match self.policy.target(view, p) {
                Verdict::Ignore => {}
                Verdict::Target => sc.targets.push(p),
                Verdict::Reject { reason, group } => {
                    let at = group.iter().copied().min_by_key(|g| (g.y, g.x, g.z)).unwrap_or(p);
                    self.done.extend(group);
                    self.done.insert(p);
                    self.leave(at, reason);
                }
            }
        }
        self.belief.lock().unwrap().observe_batch(&self.cfg.observer, view.tick(), seen.iter().map(|(p, n)| (*p, n.as_str())));
        sc.i >= total
    }

    // Spec: specs/B4-gathering.md §4.3, §4.8 (plan stands and the break order for what the policy accepts)
    /// One slice of a round: scan the claim, find what may be cleared around the chosen targets,
    /// search the break orders. Each call stays within `STEP_UNITS` (plus one target's evaluation),
    /// so the bot's tick is never held up; the bot stands still meanwhile.
    fn select(&mut self, view: &dyn BotView, mut sel: Sel) -> Option<SkillStatus> {
        loop {
            if self.units >= STEP_UNITS {
                self.phase = Phase::Select(sel);
                return None;
            }
            match sel {
                Sel::Scan(mut sc) => {
                    if !self.scan_step(view, &mut sc) {
                        sel = Sel::Scan(sc);
                        continue;
                    }
                    let need = self.cfg.quota.saturating_sub(self.gathered()) as usize;
                    if need == 0 {
                        self.phase = Phase::Ending(Scanner::default());
                        return None;
                    }
                    if sc.targets.is_empty() {
                        return Some(self.end(view, None));
                    }
                    let me = view.position();
                    let mut groups = clusters(&sc.targets);
                    groups.sort_by(|a, b| {
                        let d = |g: &Vec<BlockPos>| g.iter().map(|p| me.distance(p.center())).fold(f64::MAX, f64::min);
                        d(a).total_cmp(&d(b)).then((a[0].y, a[0].x, a[0].z).cmp(&(b[0].y, b[0].x, b[0].z)))
                    });
                    let (mut chosen, mut total): (Vec<Vec<BlockPos>>, usize) = (Vec::new(), 0);
                    for mut g in groups {
                        g.truncate(ROUND_CAP - total);
                        total += g.len();
                        chosen.push(g);
                        if total >= need || total >= ROUND_CAP {
                            break;
                        }
                    }
                    let (lo, hi) = bounds(&chosen.concat());
                    let (lo, hi) = (BlockPos::new(lo.x - 6, lo.y - 3, lo.z - 6), BlockPos::new(hi.x + 6, hi.y + 5, hi.z + 6));
                    sel = Sel::Clear { clusters: chosen, need, lo, hi, i: 0, clear: HashSet::new() };
                }
                Sel::Clear { clusters, need, lo, hi, mut i, mut clear } => {
                    // Everything the policy lets us clear around the chosen targets.
                    let (dy, dz) = ((hi.y - lo.y + 1) as u64, (hi.z - lo.z + 1) as u64);
                    let total = (hi.x - lo.x + 1) as u64 * dy * dz;
                    let targets: HashSet<BlockPos> = clusters.concat().into_iter().collect();
                    while i < total && self.units < STEP_UNITS {
                        let p = BlockPos::new(lo.x + (i / (dy * dz)) as i32, lo.y + ((i / dz) % dy) as i32, lo.z + (i % dz) as i32);
                        i += 1;
                        self.spend(3);
                        if self.cfg.claim.contains(p) && !targets.contains(&p) && self.policy.may_clear(view, p) {
                            clear.insert(p);
                        }
                    }
                    sel = if i < total {
                        Sel::Clear { clusters, need, lo, hi, i, clear }
                    } else {
                        let cur = view.position().block_pos();
                        Sel::Plan(PlanJob { clusters, ci: 0, need_left: need, tasks: Vec::new(), cur, removed: HashSet::new(), search: None, depth: 0, targets, clear })
                    };
                }
                Sel::Plan(mut job) => {
                    if !self.plan_step(view, &mut job) {
                        sel = Sel::Plan(job);
                        continue;
                    }
                    if job.tasks.is_empty() {
                        self.phase = Phase::Select(Sel::Scan(Scanner::default()));
                    } else {
                        self.phase = Phase::Run(Job { planned: job.tasks.iter().map(|t| view.block_at(t.target)).collect(), tasks: job.tasks, i: 0, sub: Sub::Go { issued: None }, broken: Vec::new() });
                    }
                    return None;
                }
            }
        }
    }

    // Spec: specs/B4-gathering.md §4.3 (one cluster at a time; what the search cannot reach is reported)
    /// Plans cluster after cluster within the unit budget. True when every chosen cluster is planned.
    fn plan_step(&mut self, view: &dyn BotView, job: &mut PlanJob) -> bool {
        while self.units < STEP_UNITS {
            if job.search.is_none() {
                if job.ci >= job.clusters.len() || job.need_left == 0 {
                    return true;
                }
                let c = &job.clusters[job.ci];
                job.depth = c.len().min(job.need_left);
                job.search = Some(Search::new(c.clone(), job.cur, job.removed.clone(), job.depth));
            }
            let pl = Planner { view, targets: &job.targets, clear: &job.clear, claim: &self.cfg.claim };
            let (found, spent) = job.search.as_mut().unwrap().advance(&pl, STEP_UNITS - self.units);
            self.spend(spent);
            if let Some(f) = found {
                // Fewer than asked for means the search ran out of reachable blocks, not of quota.
                let got: HashSet<BlockPos> = f.tasks.iter().filter(|t| !t.is_clear).map(|t| t.target).collect();
                if f.count < job.depth {
                    for t in job.clusters[job.ci].clone().into_iter().filter(|t| !got.contains(t)) {
                        self.done.insert(t);
                        self.leave(t, Reason::Unreachable);
                    }
                }
                job.cur = f.tasks.last().map_or(job.cur, |t| t.stand);
                job.need_left -= f.count;
                job.tasks.extend(f.tasks);
                job.removed = f.removed;
                job.search = None;
                job.ci += 1;
            }
        }
        false
    }

    // Spec: specs/B4-gathering.md §4.3, specs/B6-placement-breaking.md §4.4 (one break at a time; the ack decides)
    fn run(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands, mut job: Job) -> Option<SkillStatus> {
        let now = view.tick();
        let task = job.tasks[job.i];
        match job.sub {
            Sub::Go { issued } => {
                let c = task.stand.center();
                let p = view.position();
                let arrived = (p.x - c.x).abs() <= ARRIVE && (p.z - c.z).abs() <= ARRIVE && (p.y - task.stand.y as f64).abs() < 1.0;
                if arrived {
                    cmds.stop_nav();
                    job.sub = Sub::Aim { since: None };
                } else if let Some(t) = issued {
                    let idle = matches!(view.nav_status(), NavStatus::Idle | NavStatus::Stalled);
                    if (idle && now >= t + 2) || now > t + WALK_TIMEOUT {
                        return Some(self.fail(view, task.target, "could not reach the stand".into()));
                    }
                } else {
                    cmds.goto(task.stand, 0.0);
                    job.sub = Sub::Go { issued: Some(now) };
                }
            }
            Sub::Aim { since: None } => {
                if let Err(why) = self.recheck(view, task, &job.planned[job.i]) {
                    return Some(self.fail(view, task.target, why));
                }
                // The client would refuse a click out of range: refuse here and send nothing.
                let p = view.position();
                if reach::cube_distance(reach::eye_at(p.x, p.y, p.z), task.target) > reach::REACH {
                    return Some(self.fail(view, task.target, "out of reach at the stand".into()));
                }
                let slot = best_tool_slot(view, self.policy.tool());
                if slot != view.selected_slot() {
                    cmds.select_slot(slot);
                }
                cmds.look_at_block(task.target, None);
                job.sub = Sub::Aim { since: Some(now) };
            }
            Sub::Aim { since: Some(t) } => {
                if now >= t + AIM_TICKS {
                    // The look took ticks too: read the target once more right before the break.
                    if let Err(why) = self.recheck(view, task, &job.planned[job.i]) {
                        return Some(self.fail(view, task.target, why));
                    }
                    job.sub = Sub::Wait { id: cmds.break_block(task.target) };
                }
            }
            Sub::Wait { id } => match view.action(id) {
                ActionState::Pending(_) => {}
                ActionState::Expired => return Some(self.fail(view, task.target, "result expired".into())),
                ActionState::Finished(rec) => {
                    cmds.release(id);
                    let ok = matches!(&rec, ActionRecord::Block(b) if b.outcome == BlockOutcome::Done && b.pos == task.target);
                    let after = view.block_at(task.target).unwrap_or_default();
                    if !ok || !null_belief::is_air(&after) {
                        let why = match &rec {
                            ActionRecord::Block(b) => {
                                let why = b.reason.map(str::to_string).unwrap_or_else(|| format!("{:?}", b.outcome));
                                // A refusal carries the client's own eye, look and ray; keep them in the report.
                                match &b.evidence {
                                    Some(e) => format!("{why} ({})", describe_evidence(e)),
                                    None => why,
                                }
                            }
                            _ => "unexpected record".into(),
                        };
                        return Some(self.fail(view, task.target, format!("break at {:?}: {why}", task.target)));
                    }
                    self.record_change(view, task.target);
                    job.broken.push(task.target);
                    if !task.is_clear {
                        self.result.lock().unwrap().gathered += 1;
                    }
                    job.i += 1;
                    if job.i == job.tasks.len() || self.gathered() >= self.cfg.quota {
                        self.phase = Phase::Settle { until: now + SETTLE_TICKS, broken: job.broken };
                        return None;
                    }
                    job.sub = Sub::Go { issued: None };
                }
            },
        }
        self.phase = Phase::Run(job);
        None
    }

    // Spec: specs/B4-gathering.md §4.8 (ownership check before breaking: the plan is only as old as the last read)
    /// The plan was judged rounds ago and a walk takes up to `WALK_TIMEOUT` ticks: before a break,
    /// the cell must still be the planned block and the policy must still accept it.
    fn recheck(&mut self, view: &dyn BotView, task: Task, planned: &Option<String>) -> Result<(), String> {
        let now = view.block_at(task.target);
        let same = now.is_some() && now == *planned;
        let ok = same && if task.is_clear { self.policy.may_clear(view, task.target) } else { self.policy.still_target(view, task.target) };
        if ok { Ok(()) } else { Err(format!("{:?} changed since it was planned (was {planned:?}, now {now:?})", task.target)) }
    }

    // Spec: specs/B4-gathering.md §3 (a broken cell: belief gets air, the result gets before and after)
    fn record_change(&mut self, view: &dyn BotView, pos: BlockPos) {
        let mut b = self.belief.lock().unwrap();
        let before = b.block(pos, view.tick()).map(|s| s.entry.block).unwrap_or_default();
        b.observe(&self.cfg.observer, pos, "minecraft:air", view.tick());
        drop(b);
        self.broken_all.push(pos);
        self.result.lock().unwrap().changed.push(Changed { pos, before, after: "minecraft:air".into() });
    }

    // Spec: specs/B4-gathering.md §4.8 (a refused, changed or unconfirmed break ends the job with a report)
    /// A refused or unconfirmed break ends the job: no retry on a live server (CLAUDE.md, builder rule).
    fn fail(&mut self, view: &dyn BotView, at: BlockPos, why: String) -> SkillStatus {
        self.leave(at, Reason::BreakFailed(why.clone()));
        self.end(view, Some(why))
    }

    // Spec: specs/B4-gathering.md §4.9 (drops: walk to item entities near what was broken)
    fn pick(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands, broken: Vec<BlockPos>, mut visited: HashSet<EntityId>, mut current: Option<(EntityId, u64)>) {
        let now = view.tick();
        if let Some((id, t)) = current {
            let idle = matches!(view.nav_status(), NavStatus::Idle | NavStatus::Stalled) && now >= t + 2;
            if view.entity(id).is_none() || idle || now > t + ITEM_TIMEOUT {
                visited.insert(id);
                current = None;
            }
        }
        if current.is_none() {
            let me = view.position();
            let next = view
                .entities()
                .iter()
                .filter(|e| e.kind == "minecraft:item" && !visited.contains(&e.id) && broken.iter().any(|b| e.position.distance(b.center()) <= DROP_RADIUS))
                .min_by(|a, b| me.distance(a.position).total_cmp(&me.distance(b.position)).then(a.id.0.cmp(&b.id.0)));
            match next {
                Some(e) => {
                    cmds.goto(e.position.block_pos(), 0.8);
                    current = Some((e.id, now));
                }
                None => {
                    self.phase = Phase::Select(Sel::Scan(Scanner::default()));
                    return;
                }
            }
        }
        self.phase = Phase::Pick { broken, visited, current };
    }
}

fn bounds(cells: &[BlockPos]) -> (BlockPos, BlockPos) {
    let f = |g: fn(&BlockPos) -> i32, mn: bool| cells.iter().map(g).reduce(|a, b| if mn { a.min(b) } else { a.max(b) }).unwrap_or(0);
    (BlockPos::new(f(|p| p.x, true), f(|p| p.y, true), f(|p| p.z, true)), BlockPos::new(f(|p| p.x, false), f(|p| p.y, false), f(|p| p.z, false)))
}

// Spec: specs/B6-placement-breaking.md §4.4 (choose the tool by policy: the family's best tier, else the hand)
fn best_tool_slot(view: &dyn BotView, family: &str) -> u8 {
    let h = view.hotbar();
    for tier in ["netherite", "diamond", "iron", "stone", "golden", "wooden"] {
        let name = format!("{tier}_{family}");
        if let Some(i) = h.iter().position(|s| s.as_ref().is_some_and(|s| s.item.strip_prefix("minecraft:") == Some(name.as_str()))) {
            return i as u8;
        }
    }
    h.iter().position(Option::is_none).map_or(view.selected_slot(), |i| i as u8)
}

impl Skill for Gather {
    fn id(&self) -> &str {
        "gather"
    }

    // Spec: specs/B4-gathering.md §3 (gather: find, plan, break, pick up, report)
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        // The phase moves out so each handler can set the next one.
        self.units = 0;
        let out = self.step_inner(view, cmds);
        self.max_units = self.max_units.max(self.units);
        out
    }
}

impl Gather {
    // Spec: specs/B4-gathering.md §3 (the phases of gather: select, run, settle, pick up, report)
    fn step_inner(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        match std::mem::replace(&mut self.phase, Phase::Finished) {
            Phase::Start => {
                self.start_tick = view.tick();
                self.start_items = items(view);
                if let Some(why) = self.refusal.clone() {
                    return self.end(view, Some(why));
                }
                self.phase = Phase::Select(Sel::Scan(Scanner::default()));
            }
            Phase::Select(sel) => {
                if let Some(s) = self.select(view, sel) {
                    return s;
                }
            }
            Phase::Ending(mut sc) => {
                if self.scan_step(view, &mut sc) {
                    for c in clusters(&sc.targets) {
                        self.leave(c[0], Reason::Quota);
                    }
                    return self.end(view, None);
                }
                self.phase = Phase::Ending(sc);
            }
            Phase::Run(job) => {
                if let Some(s) = self.run(view, cmds, job) {
                    return s;
                }
            }
            Phase::Settle { until, broken } => {
                self.phase = if view.tick() >= until { Phase::Pick { broken, visited: HashSet::new(), current: None } } else { Phase::Settle { until, broken } };
            }
            Phase::Pick { broken, visited, current } => self.pick(view, cmds, broken, visited, current),
            Phase::Finished => {
                self.phase = Phase::Finished;
                let r = self.result.lock().unwrap();
                return r.failure.clone().map_or(SkillStatus::Done, SkillStatus::Failed);
            }
        }
        SkillStatus::Running
    }
}

// Spec: specs/B6-placement-breaking.md §4.1 (refusals carry their evidence)
/// One line of what the client saw when it refused a click: the ray's hit, the eye and the look.
fn describe_evidence(e: &null_bot_api::ClickEvidence) -> String {
    use null_bot_api::ClickHit;
    let hit = match e.hit {
        ClickHit::Miss => "ray hit nothing".to_string(),
        ClickHit::Block { pos, face, .. } => format!("ray hit block {pos:?} face {face:?}"),
        ClickHit::Entity { id, kind } => format!("ray hit entity {} ({kind})", id.0),
    };
    format!(
        "{hit}; eye {:.2},{:.2},{:.2} height {:.2} crouching {} yaw {:.1} pitch {:.1} look changed after ray {}",
        e.eye.x, e.eye.y, e.eye.z, e.eye_height, e.crouching, e.yaw, e.pitch, e.look_changed_after_hit
    )
}
