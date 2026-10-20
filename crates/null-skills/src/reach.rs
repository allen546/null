//! Reach model and break-order search for `gather`. Pure functions of the view.
//!
//! The model matches the fork's `block_actions.rs` (`check_click`, `aim_at_block`): the cube
//! distance from the eye to the block must be within the interaction range, and the crosshair
//! must hit a visible face point within that range with nothing in between.

use std::collections::{HashMap, HashSet};


use null_bot_api::{BlockPos, BotView, Vec3};

use crate::policy::Claim;

/// Vanilla block interaction range (the fork reads the player attribute; 4.5 by default).
pub const REACH: f64 = 4.5;
/// The planner is stricter than the client by this much (B6 §4.1).
pub const MARGIN: f64 = 0.05;
/// How far from the stand's centre the bot may end up; reach is also checked at these corners.
pub const STAND_SLACK: f64 = 0.3;
/// Standing eye height above the feet (B6 §4.1).
pub const EYE_HEIGHT: f64 = 1.62;
/// States kept per depth of the search; sets of up to this many break orders are searched whole.
pub const LAYER_CAP: usize = 256;

const RANGE: f64 = REACH - MARGIN;
/// A sight line that needs more blocks cleared than this is not used.
const MAX_CLEARED_PER_LINE: usize = 6;

/// One break and the stand (feet cell) to do it from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Task {
    pub stand: BlockPos,
    pub target: BlockPos,
    pub is_clear: bool,
}

pub fn eye_at(x: f64, feet_y: f64, z: f64) -> Vec3 {
    Vec3::new(x, feet_y + EYE_HEIGHT, z)
}

fn stand_eye(s: BlockPos) -> Vec3 {
    eye_at(s.x as f64 + 0.5, s.y as f64, s.z as f64 + 0.5)
}

// Spec: specs/B6-placement-breaking.md §4.1 (reach: eye to the nearest point of the block's cube; fork `distance_to_block`)
/// Distance from the eye to the nearest point of the block's unit cube.
pub fn cube_distance(eye: Vec3, pos: BlockPos) -> f64 {
    let d = |e: f64, lo: i32| (lo as f64 - e).max(e - (lo as f64 + 1.0)).max(0.0);
    (d(eye.x, pos.x).powi(2) + d(eye.y, pos.y).powi(2) + d(eye.z, pos.z).powi(2)).sqrt()
}

// Spec: specs/B6-placement-breaking.md §4.2 (aim at a visible point; the same spots as the fork's `aim_at_block`)
/// Points on the faces of the block that the eye sees and can reach, in the fork's try order.
fn aim_points(eye: Vec3, pos: BlockPos) -> Vec<Vec3> {
    const SPOTS: [(f64, f64); 5] = [(0.5, 0.5), (0.3, 0.3), (0.7, 0.3), (0.3, 0.7), (0.7, 0.7)];
    const IN: f64 = 0.005;
    let (x, y, z) = (pos.x as f64, pos.y as f64, pos.z as f64);
    // (visible from the eye, point for fractions u and v)
    type Face = (bool, fn(f64, f64, f64, f64, f64) -> Vec3);
    let faces: [Face; 6] = [
        (eye.y > y + 1.0, |x, y, z, u, v| Vec3::new(x + u, y + 1.0 - IN, z + v)),
        (eye.z < z, |x, y, z, u, v| Vec3::new(x + u, y + v, z + IN)),
        (eye.z > z + 1.0, |x, y, z, u, v| Vec3::new(x + u, y + v, z + 1.0 - IN)),
        (eye.x > x + 1.0, |x, y, z, u, v| Vec3::new(x + 1.0 - IN, y + v, z + u)),
        (eye.x < x, |x, y, z, u, v| Vec3::new(x + IN, y + v, z + u)),
        (eye.y < y, |x, y, z, u, v| Vec3::new(x + u, y + IN, z + v)),
    ];
    let mut out = Vec::new();
    for (u, v) in SPOTS {
        for (visible, f) in &faces {
            let p = f(x, y, z, u, v);
            if *visible && eye.distance(p) <= RANGE {
                out.push(p);
            }
        }
    }
    out
}

/// Cells a segment crosses, from the eye's cell to the end point's cell (voxel traversal).
fn cells_between(from: Vec3, to: Vec3) -> Vec<BlockPos> {
    let (mut cell, end) = (from.block_pos(), to.block_pos());
    let d = [to.x - from.x, to.y - from.y, to.z - from.z];
    let f = [from.x, from.y, from.z];
    let mut c = [cell.x, cell.y, cell.z];
    let mut tmax = [f64::INFINITY; 3];
    let mut tdelta = [f64::INFINITY; 3];
    for i in 0..3 {
        if d[i] > 0.0 {
            tmax[i] = ((c[i] + 1) as f64 - f[i]) / d[i];
            tdelta[i] = 1.0 / d[i];
        } else if d[i] < 0.0 {
            tmax[i] = (f[i] - c[i] as f64) / -d[i];
            tdelta[i] = -1.0 / d[i];
        }
    }
    let mut out = vec![cell];
    while cell != end && out.len() < 64 {
        let i = (0..3).min_by(|a, b| tmax[*a].total_cmp(&tmax[*b])).unwrap();
        if tmax[i] > 1.0 {
            break;
        }
        c[i] += if d[i] > 0.0 { 1 } else { -1 };
        tmax[i] += tdelta[i];
        cell = BlockPos::new(c[0], c[1], c[2]);
        out.push(cell);
    }
    out
}

/// Blocks as the planner sees them: what is to be gathered, what may be cleared, and the world.
pub struct Planner<'a> {
    pub view: &'a dyn BotView,
    pub targets: &'a HashSet<BlockPos>,
    pub clear: &'a HashSet<BlockPos>,
    pub claim: &'a Claim,
}

impl Planner<'_> {
    /// Free to stand in or look through: broken already in this plan, or neither ours nor solid.
    fn passable(&self, c: BlockPos, removed: &HashSet<BlockPos>) -> bool {
        removed.contains(&c) || (!self.targets.contains(&c) && !self.clear.contains(&c) && !self.view.is_solid(c))
    }

    fn body_fits(&self, s: BlockPos, removed: &HashSet<BlockPos>) -> bool {
        let below = BlockPos::new(s.x, s.y - 1, s.z);
        self.claim.contains(s)
            && self.passable(s, removed)
            && self.passable(BlockPos::new(s.x, s.y + 1, s.z), removed)
            && self.view.is_solid(below)
            && !removed.contains(&below)
            && !self.clear.contains(&below)
    }

    // Spec: specs/B4-gathering.md §4.3 (stand on the ground, in the trunk column, or on a target broken last)
    /// A stand needs room for the body and a floor. The floor may be a target that is still there;
    /// it is then broken after every break planned from this stand. A stand on a target must be
    /// one step up from a ground stand beside it (a player cannot climb higher without placing).
    pub fn valid_stand(&self, s: BlockPos, removed: &HashSet<BlockPos>) -> bool {
        if !self.body_fits(s, removed) {
            return false;
        }
        let below = BlockPos::new(s.x, s.y - 1, s.z);
        if !self.targets.contains(&below) {
            return true;
        }
        [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dz)| {
            let n = BlockPos::new(s.x + dx, s.y - 1, s.z + dz);
            self.body_fits(n, removed) && !self.targets.contains(&BlockPos::new(n.x, n.y - 1, n.z)) && self.passable(BlockPos::new(n.x, n.y + 2, n.z), removed)
        })
    }

    fn sees_through(&self, eye: Vec3, p: Vec3, pos: BlockPos, removed: &HashSet<BlockPos>, extra: &[BlockPos]) -> Vec<BlockPos> {
        cells_between(eye, p).into_iter().filter(|c| *c != pos && !self.passable(*c, removed) && !extra.contains(c)).collect()
    }

    // Spec: specs/B6-placement-breaking.md §4.1 (reach and visibility as the fork checks them, with a margin)
    /// The cleared blocks to break first (nearest first) so the crosshair reaches `pos` from `stand`,
    /// or None when it cannot. `rays` are the static sight lines from `rays_for`.
    fn aim(&self, stand: BlockPos, rays: &[Vec<BlockPos>], removed: &HashSet<BlockPos>, work: &mut u32) -> Option<Vec<BlockPos>> {
        let eye = stand_eye(stand);
        let mut best: Option<Vec<BlockPos>> = None;
        for cells in rays {
            let blockers: Vec<BlockPos> = cells.iter().copied().filter(|c| !self.passable(*c, removed)).collect();
            if blockers.len() > MAX_CLEARED_PER_LINE || best.as_ref().is_some_and(|b| b.len() <= blockers.len()) {
                continue;
            }
            let mut out: Vec<BlockPos> = Vec::new();
            let ok = blockers.iter().all(|b| {
                // Checking a blocker means a fresh sight search of its own.
                *work += 25;
                let fine = self.clear.contains(b) && self.claim.contains(*b) && aim_points(eye, *b).iter().any(|q| self.sees_through(eye, *q, *b, removed, &out).is_empty());
                if fine {
                    out.push(*b);
                }
                fine
            });
            if ok {
                best = Some(out);
                if best.as_ref().is_some_and(Vec::is_empty) {
                    break;
                }
            }
        }
        best
    }
}

// Spec: specs/B6-placement-breaking.md §4.1 (sight lines per stand; every corner of the stand cell must be in range)
/// The sight lines from `stand` to `pos`: for each visible face point in range, the cells between
/// (not `pos`). None when the stand is out of range at any corner of its cell. Static: they do not
/// depend on what has been broken, so the search builds them once.
fn rays_for(stand: BlockPos, pos: BlockPos) -> Option<Vec<Vec<BlockPos>>> {
    let eye = stand_eye(stand);
    // The bot may stand off-centre: every corner must still be in range.
    for (dx, dz) in [(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
        if cube_distance(eye_at(eye.x + dx * STAND_SLACK, stand.y as f64, eye.z + dz * STAND_SLACK), pos) > RANGE {
            return None;
        }
    }
    Some(aim_points(eye, pos).into_iter().map(|p| cells_between(eye, p).into_iter().filter(|c| *c != pos).collect()).collect())
}

#[derive(Clone)]
struct State {
    removed: HashSet<BlockPos>,
    tasks: Vec<Task>,
    cur: BlockPos,
    /// (cleared blocks, stand changes, blocks walked): lower is better.
    cost: (u32, u32, i32),
}

type Key = Vec<(i32, i32, i32)>;

fn key(s: &State) -> Key {
    let mut k: Vec<_> = s.removed.iter().map(|p| (p.x, p.y, p.z)).collect();
    k.sort_unstable();
    k
}

/// States kept per depth: a small set of targets is searched whole, a big one by beam.
fn layer_cap(targets: usize) -> usize {
    match targets {
        0..=8 => LAYER_CAP,
        9..=16 => 32,
        _ => 8,
    }
}

/// What a finished search hands back.
pub struct Found {
    pub tasks: Vec<Task>,
    /// Every block the plan breaks, targets and cleared ones.
    pub removed: HashSet<BlockPos>,
    /// Targets the plan breaks.
    pub count: usize,
}

// Spec: specs/B4-gathering.md §4.3 (search break orders; stand on a target and break it last)
/// A resumable search of orders of breaking `want` targets (up to `depth` of them), layer by layer:
/// a layer holds every distinct set of broken blocks, kept to `layer_cap` best by cost. Greedy
/// lowest-first would break the block a later stand needs. `advance` does a bounded amount of work
/// counted in stand checks, never in time, so the result does not depend on how it is sliced.
pub struct Search {
    want: Vec<BlockPos>,
    depth: usize,
    cap: usize,
    layer: Vec<State>,
    best: State,
    next: HashMap<Key, State>,
    level: usize,
    si: usize,
    ti: usize,
    stands: HashMap<BlockPos, Vec<BlockPos>>,
    rays: HashMap<(BlockPos, BlockPos), Option<Vec<Vec<BlockPos>>>>,
    valid: HashMap<BlockPos, bool>,
    /// The target in hand: next candidate, best so far.
    pi: usize,
    pbest: Option<(BlockPos, Vec<BlockPos>)>,
    pkey: PKey,
}

type PKey = (u32, bool, i32, i32, i32, i32);
const PKEY0: PKey = (u32::MAX, true, i32::MAX, 0, 0, 0);

impl Search {
    pub fn new(want: Vec<BlockPos>, start: BlockPos, removed: HashSet<BlockPos>, depth: usize) -> Self {
        let first = State { removed, tasks: Vec::new(), cur: start, cost: (0, 0, 0) };
        let cap = layer_cap(want.len());
        Self { want, depth, cap, layer: vec![first.clone()], best: first, next: HashMap::new(), level: 0, si: 0, ti: 0, stands: HashMap::new(), rays: HashMap::new(), valid: HashMap::new(), pi: 0, pbest: None, pkey: PKEY0 }
    }

    // Spec: specs/B4-gathering.md §4.3 (candidate stands around a target: in range, floor and room can exist)
    /// Stands worth testing for `t`: in range, with a floor that can exist and room that can exist.
    fn gen_stands(pl: &Planner, t: BlockPos) -> Vec<BlockPos> {
        let may_open = |c: BlockPos| !pl.view.is_solid(c) || pl.targets.contains(&c) || pl.clear.contains(&c);
        let mut v = Vec::new();
        for x in t.x - 5..=t.x + 5 {
            for z in t.z - 5..=t.z + 5 {
                for y in t.y - 6..=t.y + 3 {
                    let s = BlockPos::new(x, y, z);
                    let below = BlockPos::new(x, y - 1, z);
                    if cube_distance(stand_eye(s), t) <= RANGE && pl.view.is_solid(below) && !pl.clear.contains(&below) && may_open(s) && may_open(BlockPos::new(x, y + 1, z)) {
                        v.push(s);
                    }
                }
            }
        }
        v
    }

    // Spec: specs/B4-gathering.md §4.3 (the break-order search, resumable and bounded per step)
    /// Does up to about `budget` units of work (one unit is one stand check) and returns the plan
    /// when the search is over. One target's candidate list is always finished, so a call may
    /// overshoot by `MAX_PAIR_UNITS`.
    pub fn advance(&mut self, pl: &Planner, budget: u32) -> (Option<Found>, u32) {
        let mut spent = 0u32;
        while spent < budget {
            if self.level == self.depth || (self.si >= self.layer.len() && self.next.is_empty()) {
                return (Some(self.finish()), spent);
            }
            if self.si >= self.layer.len() {
                let mut v: Vec<(Key, State)> = std::mem::take(&mut self.next).into_iter().collect();
                spent += 4 * v.len() as u32;
                v.sort_by(|a, b| a.1.cost.cmp(&b.1.cost).then(a.0.cmp(&b.0)));
                v.truncate(self.cap);
                self.layer = v.into_iter().map(|(_, s)| s).collect();
                self.best = self.layer[0].clone();
                (self.level, self.si, self.ti) = (self.level + 1, 0, 0);
                self.valid.clear();
                continue;
            }
            if self.ti >= self.want.len() {
                (self.si, self.ti) = (self.si + 1, 0);
                self.valid.clear();
                continue;
            }
            let t = self.want[self.ti];
            if self.layer[self.si].removed.contains(&t) {
                self.ti += 1;
                continue;
            }
            if let std::collections::hash_map::Entry::Vacant(e) = self.stands.entry(t) {
                e.insert(Self::gen_stands(pl, t));
                spent += GEN_UNITS;
            }
            let st = &self.layer[self.si];
            let stands = &self.stands[&t];
            // The candidates are checked one by one so a step can stop between any two of them.
            while self.pi < stands.len() {
                if spent >= budget {
                    return (None, spent);
                }
                let s = &stands[self.pi];
                self.pi += 1;
                spent += 1;
                let ok = *self.valid.entry(*s).or_insert_with(|| pl.valid_stand(*s, &st.removed));
                if !ok {
                    continue;
                }
                let fresh = !self.rays.contains_key(&(*s, t));
                let rays = self.rays.entry((*s, t)).or_insert_with(|| rays_for(*s, t));
                // Building sight lines is the dear part; checking a built one is cheap.
                spent += if fresh { 30 } else { 2 };
                let Some(rays) = rays else { continue };
                let mut work = 0;
                let found = pl.aim(*s, rays, &st.removed, &mut work);
                spent += work;
                let Some(l) = found else { continue };
                let k = (l.len() as u32, *s != st.cur, (s.x - st.cur.x).abs() + (s.z - st.cur.z).abs(), s.x, s.y, s.z);
                if k < self.pkey {
                    self.pkey = k;
                    self.pbest = Some((*s, l));
                }
            }
            (self.ti, self.pi, self.pkey) = (self.ti + 1, 0, PKEY0);
            let best = self.pbest.take();
            let st = &self.layer[self.si];
            let Some((stand, leaves)) = best else { continue };
            let mut n = st.clone();
            for l in &leaves {
                n.tasks.push(Task { stand, target: *l, is_clear: true });
                n.removed.insert(*l);
            }
            n.tasks.push(Task { stand, target: t, is_clear: false });
            n.removed.insert(t);
            n.cost = (st.cost.0 + leaves.len() as u32, st.cost.1 + (stand != st.cur) as u32, st.cost.2 + (stand.x - st.cur.x).abs() + (stand.z - st.cur.z).abs());
            n.cur = stand;
            spent += 10;
            let k = key(&n);
            if self.next.get(&k).is_none_or(|old| n.cost < old.cost) {
                self.next.insert(k, n);
            }
        }
        (None, spent)
    }

    fn finish(&mut self) -> Found {
        let s = &self.best;
        Found { tasks: s.tasks.clone(), removed: s.removed.clone(), count: s.tasks.iter().filter(|t| !t.is_clear).count() }
    }
}

/// Units charged for building one target's candidate stands (a window of about 1,200 cells).
const GEN_UNITS: u32 = 900;
/// The most a step can run past its budget: one target's candidate list built, or one candidate
/// checked against every sight line.
pub const MAX_PAIR_UNITS: u32 = GEN_UNITS + 33 + 30 * (MAX_CLEARED_PER_LINE as u32) * 25;
