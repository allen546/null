//! `fell` against a small simulated world behind `FakeView`. The sim answers each command the
//! way the adapter would (goto arrives, a break lands one tick later) and checks reach on every break.

use super::*;
use std::collections::{HashMap, HashSet};

use null_bot_api::{ActionId, ActionRecord, ActionState, BlockOutcome, BlockPos, BotView, Skill, SkillStatus};
use null_bot_api::fake::{FakeCommands, FakeView};
use null_bot_api::{BlockAction, BlockActionKind, Command, ItemStack, Vec3};

const LOG: &str = "minecraft:oak_log";
const LEAF: &str = "minecraft:oak_leaves";
const AIR: &str = "minecraft:air";

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Ok,
    /// The nth break (1-based) comes back with this outcome and reason.
    FailAt(u32, BlockOutcome, &'static str),
    /// The nth break's result reads Expired instead of Finished.
    ExpireAt(u32),
}

struct Sim {
    tick: u64,
    pos: Vec3,
    blocks: HashMap<BlockPos, String>,
    props: Vec<(BlockPos, String, String)>,
    items: Vec<(u32, Vec3)>,
    inv: u32,
    hotbar: [Option<ItemStack>; 9],
    results: HashMap<ActionId, ActionState>,
    due: Vec<(ActionId, ActionState)>,
    breaks: Vec<BlockPos>,
    leaves_broken: Vec<BlockPos>,
    issued: Vec<ActionId>,
    released: Vec<ActionId>,
    selected: Vec<u8>,
    stand_y: Vec<f64>,
    mode: Mode,
    evidence: Option<null_bot_api::ClickEvidence>,
    next_item: u32,
    max_step: std::time::Duration,
    fast: bool,
    max_at: u64,
    /// Where a goto to a stand really ends, off the centre by this much on each axis.
    offset: f64,
    /// A goto to a stand never moves the bot.
    stuck: bool,
    /// Drops are never picked up (a full inventory).
    full: bool,
    /// Goto targets with tolerance 0 (the stands).
    stands: Vec<BlockPos>,
    /// Applied once, at the first goto to a stand: the world changes while the bot walks.
    swap: Option<(BlockPos, &'static str)>,
    persist_on_walk: bool,
}

impl Sim {
    fn new() -> Self {
        Self { tick: 100, pos: Vec3::new(8.5, 64.0, 0.5), blocks: HashMap::new(), props: vec![], items: vec![], inv: 0, hotbar: Default::default(), results: HashMap::new(), due: vec![], breaks: vec![], leaves_broken: vec![], issued: vec![], released: vec![], selected: vec![], stand_y: vec![], mode: Mode::Ok, evidence: None, next_item: 1, max_step: Default::default(), fast: false, max_at: 0, offset: 0.0, stuck: false, full: false, stands: vec![], swap: None, persist_on_walk: false }
    }
    fn set(&mut self, x: i32, y: i32, z: i32, n: &str) {
        let p = BlockPos::new(x, y, z);
        self.blocks.insert(p, n.to_string());
        if n.ends_with("_leaves") {
            self.props.push((p, "persistent".into(), "false".into()));
        }
    }
    /// A trunk of `h` logs at (x, z) with a canopy of natural leaves around the top.
    fn oak(&mut self, x: i32, z: i32, h: i32) {
        for y in 0..h {
            self.set(x, 64 + y, z, LOG);
        }
        for y in h - 2..=h {
            let r = if y == h { 1 } else { 2 };
            for dx in -r..=r {
                for dz in -r..=r {
                    if !self.blocks.contains_key(&BlockPos::new(x + dx, 64 + y, z + dz)) {
                        self.set(x + dx, 64 + y, z + dz, LEAF);
                    }
                }
            }
        }
    }
    fn view(&self) -> FakeView {
        let mut v = FakeView::default();
        (v.tick, v.position, v.hotbar, v.results, v.properties) = (self.tick, self.pos, self.hotbar.clone(), self.results.clone(), self.props.clone());
        for (p, n) in &self.blocks {
            v.blocks.push((*p, n.clone()));
            if !null_belief::is_air(n) {
                v.solid.push(*p);
            }
        }
        if self.inv > 0 {
            v.main_inventory[0] = Some(ItemStack::new(LOG, self.inv as u8));
        }
        for (id, at) in &self.items {
            v.entities.push(null_bot_api::EntitySnapshot::new(*id, "minecraft:item", *at));
        }
        v
    }
    fn apply(&mut self, cmds: &FakeCommands) {
        for (_, c) in &cmds.log {
            match *c {
                Command::Goto { target, tolerance } => {
                    if tolerance == 0.0 {
                        self.stands.push(target);
                        if let Some((p, n)) = self.swap.take() {
                            self.blocks.insert(p, n.to_string());
                        }
                        if std::mem::take(&mut self.persist_on_walk) {
                            for (_, _, v) in self.props.iter_mut().filter(|(_, n, _)| n == "persistent") {
                                *v = "true".into();
                            }
                        }
                        if self.stuck {
                            continue;
                        }
                    }
                    self.pos = target.center();
                    self.pos.y = target.y as f64;
                    if tolerance == 0.0 {
                        self.pos.x += self.offset;
                        self.pos.z += self.offset;
                    }
                    let me = self.pos;
                    if !self.full {
                        let (near, far): (Vec<_>, Vec<_>) = self.items.iter().partition(|(_, at)| at.distance(me) <= 1.5);
                        self.inv += near.len() as u32;
                        self.items = far;
                    }
                }
                Command::SelectSlot(s) => self.selected.push(s),
                Command::BreakBlock(p) => {
                    let eye = Vec3::new(self.pos.x, self.pos.y + 1.62, self.pos.z);
                    assert!(reach::cube_distance(eye, p) <= 4.5, "break out of reach: {p:?} from {:?}", self.pos);
                    // Every break is a planned target or a clearable leaf, inside the claim.
                    let name = self.blocks.get(&p).cloned().unwrap_or_default();
                    let natural_leaf = name == LEAF && self.props.iter().any(|(q, n, v)| *q == p && n == "persistent" && v == "false");
                    assert!(claim().contains(p), "break outside the claim: {p:?}");
                    assert!(name == LOG || name == "minecraft:stone" || natural_leaf, "break of {name} at {p:?}");
                    self.breaks.push(p);
                    self.stand_y.push(self.pos.y);
                    let n = self.breaks.len() as u32;
                    let id = cmds.issued.iter().find(|(_, c)| *c == Command::BreakBlock(p)).unwrap().0;
                    self.issued.push(id);
                    let (outcome, reason) = match self.mode {
                        Mode::FailAt(k, o, r) if k == n => (o, Some(r)),
                        _ => (BlockOutcome::Done, None),
                    };
                    if outcome == BlockOutcome::Done {
                        let was = self.blocks.insert(p, AIR.to_string()).unwrap();
                        if was == LOG {
                            self.items.push((self.next_item, p.center()));
                            self.next_item += 1;
                        } else {
                            self.leaves_broken.push(p);
                        }
                    }
                    let rec = BlockAction { kind: BlockActionKind::Break, pos: p, outcome, start_tick: self.tick, finish_tick: Some(self.tick), ack_tick: Some(self.tick + 1), reason, evidence: if outcome == BlockOutcome::Refused { self.evidence } else { None } };
                    self.results.insert(id, ActionState::Pending(None));
                    let landed = if matches!(self.mode, Mode::ExpireAt(k) if k == n) { ActionState::Expired } else { ActionState::Finished(ActionRecord::Block(rec)) };
                    self.due.push((id, landed));
                }
                _ => {}
            }
        }
        self.released.extend(cmds.released.iter().copied());
        for r in &cmds.released {
            self.results.insert(*r, ActionState::Expired);
        }
    }
    fn run(&mut self, skill: &mut Gather) -> SkillStatus {
        for _ in 0..4000 {
            let v = if self.fast { Box::new(self.fast_view()) as Box<dyn BotView> } else { Box::new(self.view()) };
            let mut c = FakeCommands::default();
            c.set_tick(self.tick);
            let t0 = std::time::Instant::now();
            let st = skill.step(&*v, &mut c);
            let el = t0.elapsed();
            if el > self.max_step {
                (self.max_step, self.max_at) = (el, self.tick - 100);
            }
            self.apply(&c);
            // Results land after the command's tick: the id reads pending in the step that got it.
            for (id, s) in std::mem::take(&mut self.due) {
                self.results.insert(id, s);
            }
            self.tick += 1;
            if st != SkillStatus::Running {
                return st;
            }
        }
        panic!("skill did not finish");
    }
}

fn claim() -> Claim {
    Claim { min: BlockPos::new(-10, 60, -10), max: BlockPos::new(10, 90, 10) }
}

fn fell_job(quota: u32, replant: bool) -> (Gather, Arc<Mutex<Belief>>) {
    let b = Arc::new(Mutex::new(Belief::new()));
    (fell("oak", quota, replant, claim(), "Bot1", b.clone()), b)
}

fn logs_left(s: &Sim) -> usize {
    s.blocks.values().filter(|n| *n == LOG).count()
}

#[test]
fn fells_a_natural_oak_and_belief_shows_the_cells_as_air() {
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    s.hotbar[2] = Some(ItemStack::new("minecraft:stone_axe", 1));
    s.hotbar[5] = Some(ItemStack::new("minecraft:iron_axe", 1));
    let before = s.blocks.clone();
    let (mut f, belief) = fell_job(5, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert_eq!(logs_left(&s), 0);
    assert!(s.leaves_broken.is_empty(), "no leaf is in the way of a trunk");
    // One break at a time: every id read, released once.
    assert_eq!(s.issued.len(), s.breaks.len());
    assert_eq!(s.released, s.issued);
    assert_eq!(s.selected.first(), Some(&5), "the iron axe is chosen over the stone one");
    let r = f.result();
    assert_eq!(r.gathered, 5);
    assert_eq!(r.yield_items.get(LOG), Some(&5), "all five drops picked up");
    assert!(r.left.is_empty() && !r.short_of_quota && r.ticks > 0);
    assert_eq!(r.changed.len(), 5);
    assert!(r.changed.iter().all(|c| c.after == AIR && before[&c.pos] == c.before));
    let b = belief.lock().unwrap();
    for y in 64..69 {
        let e = b.block(BlockPos::new(0, y, 0), s.tick).unwrap().entry;
        assert_eq!((e.block.as_str(), e.observer.as_str()), (AIR, "Bot1"));
        assert_eq!(e.changed.unwrap().from, LOG);
    }
}

#[test]
fn a_seven_log_trunk_is_reached_whole_from_the_ground_or_the_column() {
    let mut s = Sim::new();
    s.oak(0, 0, 7);
    let (mut f, _) = fell_job(7, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert_eq!(f.result().gathered, 7);
    assert_eq!(logs_left(&s), 0);
    assert!(s.stand_y.iter().all(|y| *y == 64.0), "no stand above the ground is needed");
}

#[test]
fn on_an_eight_log_trunk_the_bot_stands_on_the_first_log_and_breaks_it_last() {
    let mut s = Sim::new();
    s.oak(0, 0, 8);
    let (mut f, _) = fell_job(8, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    let r = f.result();
    assert_eq!((r.gathered, logs_left(&s)), (8, 0));
    assert_eq!(*s.breaks.last().unwrap(), BlockPos::new(0, 64, 0), "the log under the bot goes last");
    assert_eq!(*s.stand_y.last().unwrap(), 65.0);
    let top = s.breaks.iter().position(|p| *p == BlockPos::new(0, 71, 0)).unwrap();
    assert_eq!(s.stand_y[top], 65.0, "the top log is only in reach from on top of the first");
    // Logs 2 and 3 are cleared from the ground first, to make room for the body.
    let first_high = s.stand_y.iter().position(|y| *y == 65.0).unwrap();
    for y in [65, 66] {
        assert!(s.breaks[..first_high].contains(&BlockPos::new(0, y, 0)));
    }
}

#[test]
fn a_nine_log_trunk_leaves_the_top_log_unreachable_and_gathers_the_rest() {
    let mut s = Sim::new();
    s.oak(0, 0, 9);
    let (mut f, _) = fell_job(9, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    let r = f.result();
    assert_eq!(r.gathered, 8);
    assert_eq!(r.left, vec![Left { at: BlockPos::new(0, 72, 0), reason: Reason::Unreachable }]);
    assert!(r.short_of_quota);
    assert_eq!(logs_left(&s), 1);
}

#[test]
fn leaves_that_block_a_log_are_broken_first_and_others_stay() {
    let mut s = Sim::new();
    // Trunk of two logs, a branch log diagonal to the top, and a dense natural canopy from y 65 up.
    s.set(0, 64, 0, LOG);
    s.set(0, 65, 0, LOG);
    s.set(1, 66, 0, LOG);
    for x in -2..=3 {
        for z in -2..=2 {
            for y in 65..=68 {
                if !s.blocks.contains_key(&BlockPos::new(x, y, z)) {
                    s.set(x, y, z, LEAF);
                }
            }
        }
    }
    let leaves_before = s.blocks.values().filter(|n| *n == LEAF).count();
    let (mut f, _) = fell_job(3, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert_eq!(logs_left(&s), 0);
    let first_leaf = s.breaks.iter().position(|p| s.leaves_broken.contains(p)).expect("a leaf had to go");
    let last_log = s.breaks.iter().rposition(|p| !s.leaves_broken.contains(p)).unwrap();
    assert!(first_leaf < last_log, "a leaf goes before the log it blocks");
    let left = s.blocks.values().filter(|n| *n == LEAF).count();
    assert!(left > leaves_before / 2, "most leaves are left to decay ({left} of {leaves_before})");
    assert_eq!(f.result().changed.len(), 3 + s.leaves_broken.len(), "broken leaves count as changed cells");
    assert_eq!(f.result().gathered, 3, "leaves are not targets");
}

#[test]
fn a_hut_and_other_non_natural_trees_are_rejected_and_reported_with_nothing_broken() {
    // Logs under a roof of persistent leaves (placed by a player).
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    s.props.clear();
    for (p, n) in s.blocks.clone() {
        if n == LEAF {
            s.props.push((p, "persistent".into(), "true".into()));
        }
    }
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(s.breaks.is_empty());
    assert!(matches!(f.result().left[0].reason, Reason::NotNatural(_)));

    // A natural tree with a plank wall against its trunk.
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    s.set(1, 64, 0, "minecraft:oak_planks");
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(s.breaks.is_empty());
    assert!(matches!(&f.result().left[0].reason, Reason::NotNatural(m) if m.contains("oak_planks")));

    // Logs with no leaves, and a torch against a natural tree.
    let mut s = Sim::new();
    for y in 64..67 {
        s.set(0, y, 0, LOG);
    }
    s.oak(5, 5, 4);
    s.set(6, 65, 5, "minecraft:torch");
    let (mut f, _) = fell_job(7, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(s.breaks.is_empty());
    let r = f.result();
    assert_eq!(r.left.len(), 2, "one entry per rejected tree");
    assert!(r.short_of_quota && r.gathered == 0);
}

#[test]
fn a_tree_partly_outside_the_claim_is_rejected() {
    let mut s = Sim::new();
    s.oak(10, 0, 4);
    s.set(11, 66, 0, LOG); // a branch log one block past the claim edge
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(s.breaks.is_empty());
    assert_eq!(f.result().left[0].reason, Reason::OutsideClaim(BlockPos::new(11, 66, 0)));
}

#[test]
fn a_two_by_two_trunk_is_unsupported_and_a_dark_oak_job_is_refused() {
    let mut s = Sim::new();
    for (dx, dz) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        for y in 64..68 {
            s.set(dx, y, dz, LOG);
        }
    }
    for x in -2..=3 {
        for z in -2..=3 {
            if !s.blocks.contains_key(&BlockPos::new(x, 68, z)) {
                s.set(x, 68, z, LEAF);
            }
        }
    }
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(s.breaks.is_empty());
    assert!(matches!(&f.result().left[0].reason, Reason::Unsupported(m) if m.contains("2x2")));
    let mut d = fell("dark_oak", 1, false, claim(), "Bot1", Default::default());
    assert!(matches!(Sim::new().run(&mut d), SkillStatus::Failed(_)));
}

#[test]
fn a_refused_or_unacknowledged_break_fails_the_job_and_stops() {
    for (outcome, reason) in [(BlockOutcome::Refused, "crosshair does not hit it"), (BlockOutcome::Refused, "no ack"), (BlockOutcome::Cancelled, "too long")] {
        let mut s = Sim::new();
        s.oak(0, 0, 5);
        s.mode = Mode::FailAt(3, outcome, reason);
        let (mut f, belief) = fell_job(5, false);
        let st = s.run(&mut f);
        assert!(matches!(&st, SkillStatus::Failed(m) if m.contains(reason)), "{st:?}");
        assert_eq!(s.breaks.len(), 3, "no break after the failed one");
        assert_eq!(s.released, s.issued, "the failed id is read and released too");
        let r = f.result();
        assert_eq!(r.gathered, 2);
        assert!(matches!(r.left[0].reason, Reason::BreakFailed(_)) && r.failure.is_some());
        let b = belief.lock().unwrap();
        assert_eq!(b.block(s.breaks[2], s.tick).unwrap().entry.block, LOG, "the cell that did not break is not recorded as air");
        assert!(matches!(f.step(&s.view(), &mut FakeCommands::default()), SkillStatus::Failed(_)));
    }
}

#[test]
fn a_refused_break_reports_what_the_client_saw() {
    use null_bot_api::{ClickEvidence, ClickHit};
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    s.mode = Mode::FailAt(1, BlockOutcome::Refused, "crosshair does not hit it");
    s.evidence = Some(ClickEvidence { check_tick: 7, eye: Vec3::new(1.5, 65.62, 2.5), eye_height: 1.62, yaw: 10.0, pitch: 20.0, crouching: false, hit: ClickHit::Miss, look_changed_after_hit: false });
    let (mut f, _) = fell_job(5, false);
    let st = s.run(&mut f);
    assert!(matches!(&st, SkillStatus::Failed(m) if m.contains("ray hit nothing") && m.contains("height 1.62")), "{st:?}");
}

#[test]
fn replant_is_refused() {
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    let (mut f, _) = fell_job(4, true);
    assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("replant")));
    assert!(s.breaks.is_empty());
}

#[test]
fn the_quota_stops_the_job_and_reports_what_it_left() {
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    s.oak(0, 6, 4);
    let (mut f, _) = fell_job(3, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    let r = f.result();
    assert_eq!((r.gathered, r.short_of_quota), (3, false));
    assert_eq!(logs_left(&s), 5);
    assert!(r.left.iter().all(|l| l.reason == Reason::Quota) && !r.left.is_empty());
}

#[test]
fn fewer_trees_than_the_quota_is_done_with_a_short_report() {
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    s.oak(0, 6, 4);
    let (mut f, _) = fell_job(20, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    let r = f.result();
    assert_eq!((r.gathered, r.short_of_quota), (8, true));
    assert_eq!(r.yield_items.get(LOG), Some(&8));
}

/// A second policy, to show the skill is generic: every stone block in the claim above the floor.
struct StonePolicy;
impl Policy for StonePolicy {
    fn target(&mut self, view: &dyn BotView, pos: BlockPos) -> Verdict {
        if pos.y >= 64 && view.block_at(pos).as_deref() == Some("minecraft:stone") { Verdict::Target } else { Verdict::Ignore }
    }
    fn may_clear(&mut self, _: &dyn BotView, _: BlockPos) -> bool {
        false
    }
    fn tool(&self) -> &'static str {
        "pickaxe"
    }
}

#[test]
fn any_policy_works() {
    let mut s = Sim::new();
    for y in 64..67 {
        s.set(2, y, 0, "minecraft:stone");
    }
    let cfg = GatherConfig { quota: 2, claim: claim(), observer: "Bot1".into() };
    let mut g = Gather::new(cfg, Box::new(StonePolicy), Default::default());
    assert_eq!(s.run(&mut g), SkillStatus::Done);
    assert_eq!(g.result().gathered, 2);
    assert_eq!(s.breaks.len(), 2);
}

#[test]
fn planning_is_sliced_within_the_step_budget_and_slicing_does_not_change_the_plan() {
    use crate::reach::{Planner, Search};
    let mut s = Sim::new();
    s.oak(0, 0, 8);
    let (mut f, _) = fell_job(8, false);
    s.run(&mut f);
    // Counted in block reads and stand checks, not time: the claim alone is 13k cells.
    assert!(f.max_units <= STEP_UNITS + MAX_PAIR_UNITS, "{}", f.max_units);
    assert!(f.max_units >= STEP_UNITS, "the work really was cut at the budget");
    let s = {
        let mut s = Sim::new();
        s.oak(0, 0, 8);
        s
    };
    let v = s.view();
    let targets: HashSet<BlockPos> = s.blocks.iter().filter(|(_, n)| *n == LOG).map(|(p, _)| *p).collect();
    let clear: HashSet<BlockPos> = s.blocks.iter().filter(|(_, n)| *n == LEAF).map(|(p, _)| *p).collect();
    let mut want: Vec<BlockPos> = targets.iter().copied().collect();
    want.sort_by_key(|p| p.y);
    let c = claim();
    let plan_with = |budget: u32| {
        let mut se = Search::new(want.clone(), BlockPos::new(8, 64, 0), HashSet::new(), 8);
        let mut calls = 0;
        loop {
            calls += 1;
            let pl = Planner { view: &v, targets: &targets, clear: &clear, claim: &c };
            if let (Some(found), _) = se.advance(&pl, budget) {
                return (found.tasks, calls);
            }
        }
    };
    let (small, calls_small) = plan_with(200);
    let (big, calls_big) = plan_with(u32::MAX / 2);
    assert_eq!(small, big, "the same world gives the same plan however the work is sliced");
    assert!(calls_small > 10 * calls_big && calls_big == 1);
}

/// `FakeView` with hashed block reads, like the adapter's chunk lookups, so timings are not
/// dominated by the fake's linear searches.
struct Fast {
    v: FakeView,
    blocks: HashMap<BlockPos, String>,
    solid: HashSet<BlockPos>,
    props: HashMap<BlockPos, String>,
}

impl BotView for Fast {
    fn tick(&self) -> u64 { self.v.tick() }
    fn position(&self) -> Vec3 { self.v.position() }
    fn velocity(&self) -> Vec3 { self.v.velocity() }
    fn look(&self) -> null_bot_api::Look { self.v.look() }
    fn on_ground(&self) -> bool { true }
    fn in_water(&self) -> bool { false }
    fn in_lava(&self) -> bool { false }
    fn fall_distance(&self) -> f32 { 0.0 }
    fn health(&self) -> f32 { 20.0 }
    fn food(&self) -> u32 { 20 }
    fn attack_cooldown(&self) -> f32 { 1.0 }
    fn hotbar(&self) -> &[Option<ItemStack>; 9] { self.v.hotbar() }
    fn inventory(&self) -> &[Option<ItemStack>; 36] { self.v.inventory() }
    fn selected_slot(&self) -> u8 { 0 }
    fn offhand(&self) -> Option<&ItemStack> { None }
    fn using_item(&self) -> Option<null_bot_api::UsingItem> { None }
    fn entities(&self) -> &[null_bot_api::EntitySnapshot] { self.v.entities() }
    fn recent_attackers(&self) -> &[null_bot_api::Attacker] { &[] }
    fn is_solid(&self, p: BlockPos) -> bool { p.y < 64 || self.solid.contains(&p) }
    fn nav_status(&self) -> null_bot_api::NavStatus { self.v.nav_status() }
    fn block_at(&self, p: BlockPos) -> Option<String> {
        Some(self.blocks.get(&p).cloned().unwrap_or_else(|| if p.y < 64 { "minecraft:stone" } else { AIR }.to_string()))
    }
    fn block_property(&self, p: BlockPos, _: &str) -> Option<String> { self.props.get(&p).cloned() }
    fn action(&self, id: ActionId) -> ActionState { self.v.action(id) }
}

impl Sim {
    fn fast_view(&self) -> Fast {
        Fast {
            v: self.view(),
            blocks: self.blocks.clone(),
            solid: self.blocks.iter().filter(|(_, n)| !null_belief::is_air(n)).map(|(p, _)| *p).collect(),
            props: self.props.iter().map(|(p, _, v)| (*p, v.clone())).collect(),
        }
    }
}

/// Run with `--release --ignored --nocapture`: the longest single step per scene.
#[test]
#[ignore]
fn timings() {
    for h in [5, 7, 8, 9] {
        let mut s = Sim::new();
        s.fast = true;
        s.oak(0, 0, h);
        let (mut f, _) = fell_job(h as u32, false);
        s.run(&mut f);
        println!("trunk {h}: longest step {:?} at tick {}, max units {}, ticks {}", s.max_step, s.max_at, f.max_units, f.result().ticks);
    }
    // Near the 40-target cap: eight 5-log trees.
    let mut s = Sim::new();
    s.fast = true;
    for i in 0..8 {
        s.oak(-9 + 2 * i, -8 + 2 * (i % 2) * 8, 5);
    }
    let (mut f, _) = fell_job(40, false);
    s.run(&mut f);
    println!("8 trees: longest step {:?} at tick {}, max units {}, ticks {}, gathered {}", s.max_step, s.max_at, f.max_units, f.result().ticks, f.result().gathered);
}

// A stand's centre is within the slack of where the bot ends up: the planner checked reach at those corners.
#[test]
fn an_off_centre_arrival_within_the_planned_slack_still_breaks() {
    let mut s = Sim::new();
    s.oak(0, 0, 7);
    s.offset = reach::STAND_SLACK - 0.01;
    let (mut f, _) = fell_job(7, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert_eq!((f.result().gathered, logs_left(&s)), (7, 0));
}

#[test]
fn an_arrival_beyond_the_slack_fails_the_job_and_breaks_nothing() {
    let mut s = Sim::new();
    s.oak(0, 0, 7);
    s.offset = reach::STAND_SLACK + 0.05;
    let (mut f, _) = fell_job(7, false);
    assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("could not reach")));
    assert!(s.breaks.is_empty());
}

#[test]
fn a_log_replaced_by_a_chest_during_the_walk_is_not_broken() {
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    let chest = BlockPos::new(0, 66, 0);
    s.swap = Some((chest, "minecraft:chest"));
    let (mut f, _) = fell_job(5, false);
    let st = s.run(&mut f);
    assert!(matches!(&st, SkillStatus::Failed(m) if m.contains("changed")), "{st:?}");
    assert!(!s.breaks.contains(&chest) && s.blocks[&chest] == "minecraft:chest");
    let r = f.result();
    // The chest against its neighbours also stops them: the first break refused is next to it.
    assert!(matches!(&r.left[0], Left { at, reason: Reason::BreakFailed(_) } if at.x == 0 && (65..=67).contains(&at.y)));
    assert_eq!(s.released, s.issued, "every id sent was read and released");
}

#[test]
fn a_plank_wall_built_against_the_trunk_during_the_walk_stops_the_break() {
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    s.swap = Some((BlockPos::new(1, 65, 0), "minecraft:oak_planks"));
    let (mut f, _) = fell_job(5, false);
    assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("changed")));
    assert!(!s.breaks.contains(&BlockPos::new(0, 65, 0)), "no break of a log with a build against it");
}

#[test]
fn leaves_that_turn_persistent_during_the_walk_are_not_cleared() {
    // The dense canopy of the leaf test: a leaf is cleared first; a player's block update marks them kept.
    let mut s = Sim::new();
    s.set(0, 64, 0, LOG);
    s.set(0, 65, 0, LOG);
    s.set(1, 66, 0, LOG);
    for x in -2..=3 {
        for z in -2..=2 {
            for y in 65..=68 {
                if !s.blocks.contains_key(&BlockPos::new(x, y, z)) {
                    s.set(x, y, z, LEAF);
                }
            }
        }
    }
    s.persist_on_walk = true;
    let (mut f, _) = fell_job(3, false);
    let st = s.run(&mut f);
    assert!(matches!(&st, SkillStatus::Failed(m) if m.contains("changed")), "{st:?}");
    assert!(s.leaves_broken.is_empty(), "no leaf is broken once it is persistent");
}

#[test]
fn leaves_outside_the_claim_are_never_cleared() {
    // A log on the claim's east edge under a dense canopy that reaches past it; the sim asserts
    // on every break that it lies in the claim.
    let mut s = Sim::new();
    s.set(10, 64, 0, LOG);
    s.set(10, 65, 0, LOG);
    s.set(9, 66, 0, LOG);
    for x in 7..=13 {
        for z in -2..=2 {
            for y in 65..=68 {
                if !s.blocks.contains_key(&BlockPos::new(x, y, z)) {
                    s.set(x, y, z, LEAF);
                }
            }
        }
    }
    let outside = s.blocks.iter().filter(|(p, n)| p.x > 10 && **n == LEAF).count();
    let (mut f, _) = fell_job(3, false);
    let st = s.run(&mut f);
    assert!(matches!(st, SkillStatus::Done | SkillStatus::Failed(_)));
    assert!(s.leaves_broken.iter().all(|p| claim().contains(*p)));
    assert_eq!(s.blocks.iter().filter(|(p, n)| p.x > 10 && **n == LEAF).count(), outside);
}

#[test]
fn no_stand_lies_outside_the_claim() {
    let mut s = Sim::new();
    s.oak(9, 0, 6);
    s.pos = Vec3::new(5.5, 64.0, 5.5);
    let (mut f, _) = fell_job(6, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert_eq!(f.result().gathered, 6);
    assert!(!s.stands.is_empty() && s.stands.iter().all(|p| claim().contains(*p)), "{:?}", s.stands);
}

#[test]
fn a_result_that_expired_ends_the_job() {
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    s.mode = Mode::ExpireAt(2);
    let (mut f, _) = fell_job(5, false);
    assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("expired")));
    assert_eq!(s.breaks.len(), 2, "no break after the expired one");
    assert!(matches!(f.result().left[0].reason, Reason::BreakFailed(_)));
}

#[test]
fn a_walk_that_goes_nowhere_ends_the_job_without_a_break() {
    let mut s = Sim::new();
    s.oak(0, 0, 5);
    s.stuck = true;
    let (mut f, _) = fell_job(5, false);
    assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("could not reach")));
    assert!(s.breaks.is_empty());
}

#[test]
fn an_invalid_claim_is_refused_before_any_break() {
    for (min, max) in [(BlockPos::new(5, 60, -10), BlockPos::new(-5, 90, 10)), (BlockPos::new(-10, 91, -10), BlockPos::new(10, 60, 10)), (BlockPos::new(-10, 60, 3), BlockPos::new(10, 90, 2))] {
        let mut s = Sim::new();
        s.oak(0, 0, 4);
        let mut f = fell("oak", 4, false, Claim { min, max }, "Bot1", Default::default());
        assert!(matches!(s.run(&mut f), SkillStatus::Failed(m) if m.contains("invalid claim")));
        assert!(s.breaks.is_empty());
    }
}

#[test]
fn drops_that_were_not_picked_up_are_reported() {
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    s.full = true;
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    let r = f.result();
    assert_eq!(r.gathered, 4);
    assert_eq!(r.drops_left.len(), 4, "{:?}", r.drops_left);
    assert!(r.yield_items.is_empty());

    // With room, nothing is left.
    let mut s = Sim::new();
    s.oak(0, 0, 4);
    let (mut f, _) = fell_job(4, false);
    assert_eq!(s.run(&mut f), SkillStatus::Done);
    assert!(f.result().drops_left.is_empty());
}

#[test]
fn a_target_replaced_during_the_walk_is_not_broken_under_any_policy() {
    // The default re-check asks the policy again: the stone is now a chest, so the policy says no.
    let mut s = Sim::new();
    for y in 64..67 {
        s.set(2, y, 0, "minecraft:stone");
    }
    let chest = BlockPos::new(2, 65, 0);
    s.swap = Some((chest, "minecraft:chest"));
    let cfg = GatherConfig { quota: 3, claim: claim(), observer: "Bot1".into() };
    let mut g = Gather::new(cfg, Box::new(StonePolicy), Default::default());
    assert!(matches!(s.run(&mut g), SkillStatus::Failed(m) if m.contains("changed")));
    assert!(!s.breaks.contains(&chest) && s.blocks[&chest] == "minecraft:chest");
    assert!(matches!(&g.result().left[0], Left { at, .. } if *at == chest));
}
