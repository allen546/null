//! Bot-side ops for A14 (`gather` and the tree policy, B4 §3, §4.10). `RemoteSkill::step` calls
//! `step` for `Op::Gather`; the job itself is `null_skills::fell`, driven through `BotView` /
//! `BotCommands` only. The op times every step of the skill on the live bot (the planning budget
//! was set on a fake view) and records what the view showed on the way.
// Spec: specs/B4-gathering.md §4.10 (gather core and policies)

use std::{
    collections::BTreeSet,
    sync::{Arc, Mutex},
    time::Instant,
};

use null_belief::Belief;
use null_bot_api::{BlockPos, BotCommands, BotView, Skill, SkillStatus};
use null_skills::{Claim, Gather, GatherResult};

use crate::skill::Info;

/// The op gives up after this many ticks (about 17 minutes), before the runner's own wait.
const MAX_TICKS: u64 = 20_000;

#[derive(Clone, Debug)]
pub enum GatherOp {
    /// Fell natural trees of `tree` ("oak") in `claim`, up to `quota` logs. The bot's belief is
    /// read at `probe` afterwards.
    Fell { tree: &'static str, quota: u32, claim: Claim, observer: String, probe: Vec<BlockPos> },
    /// Read cells through the view: name, `is_solid`, `persistent`.
    Probe(Vec<BlockPos>),
}

/// One cell as the view reads it.
#[derive(Clone, Debug)]
pub struct CellRead {
    pub name: Option<String>,
    pub solid: bool,
    pub persistent: Option<String>,
}

/// One cell as belief holds it: block, observer, and what it was before a change.
#[derive(Clone, Debug)]
pub struct BeliefRead {
    pub block: Option<String>,
    pub observer: Option<String>,
    pub changed_from: Option<String>,
}

/// Wall time of the skill's steps, in microseconds.
#[derive(Clone, Debug, Default)]
pub struct StepTimes {
    pub steps: usize,
    pub p50: u128,
    pub p99: u128,
    pub max: u128,
    /// Steps over 2 ms (the limit in BUILD_PLAN S5), and the op age of the slowest.
    pub over_2ms: usize,
    pub max_at_age: u64,
}

#[derive(Clone, Debug, Default)]
pub struct GatherInfo {
    pub status: String,
    pub result: GatherResult,
    pub belief: Vec<BeliefRead>,
    pub times: StepTimes,
    /// Highest feet height the bot reached (a stand on log 1 reads one above the ground).
    pub max_y: f64,
    /// Every entity kind the view listed during the job, and the most item entities at once.
    pub kinds: BTreeSet<String>,
    pub max_items: usize,
    pub reads: Vec<CellRead>,
}

#[derive(Default)]
pub struct GatherState {
    skill: Option<Gather>,
    belief: Arc<Mutex<Belief>>,
    micros: Vec<(u128, u64)>,
    kinds: BTreeSet<String>,
    max_items: usize,
    max_y: f64,
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    sorted.get(((sorted.len() as f64 - 1.0) * p).round() as usize).copied().unwrap_or(0)
}

// Spec: specs/B4-gathering.md §4.10 (planning stays inside a step budget; measured live here)
fn times(st: &GatherState) -> StepTimes {
    let mut sorted: Vec<u128> = st.micros.iter().map(|(m, _)| *m).collect();
    sorted.sort_unstable();
    let (max, at) = st.micros.iter().copied().max_by_key(|(m, _)| *m).unwrap_or((0, 0));
    StepTimes { steps: sorted.len(), p50: percentile(&sorted, 0.5), p99: percentile(&sorted, 0.99), max, over_2ms: sorted.iter().filter(|m| **m > 2000).count(), max_at_age: at }
}

// Spec: specs/B4-gathering.md §4.10 (the op drives one `fell` job to its end and reports it)
pub fn step(op: &GatherOp, age: u64, v: &dyn BotView, c: &mut dyn BotCommands, st: &mut GatherState) -> Option<Info> {
    match op {
        GatherOp::Probe(cells) => Some(Info::Gather(Box::new(GatherInfo {
            reads: cells.iter().map(|p| CellRead { name: v.block_at(*p), solid: v.is_solid(*p), persistent: v.block_property(*p, "persistent") }).collect(),
            ..Default::default()
        }))),
        GatherOp::Fell { tree, quota, claim, observer, probe } => {
            if age == 0 {
                *st = GatherState::default();
                st.skill = Some(null_skills::fell(tree, *quota, false, *claim, observer, st.belief.clone()));
            }
            st.max_y = st.max_y.max(v.position().y);
            st.max_items = st.max_items.max(v.entities().iter().filter(|e| e.kind == "minecraft:item").count());
            st.kinds.extend(v.entities().iter().map(|e| e.kind.clone()));
            let t0 = Instant::now();
            let status = st.skill.as_mut()?.step(v, c);
            st.micros.push((t0.elapsed().as_micros(), age));
            let timed_out = age > MAX_TICKS;
            if status == SkillStatus::Running && !timed_out {
                return None;
            }
            if timed_out {
                c.stop_nav();
            }
            let now = v.tick();
            let b = st.belief.lock().unwrap();
            let belief = probe
                .iter()
                .map(|p| {
                    let seen = b.block(*p, now);
                    BeliefRead { block: seen.as_ref().map(|s| s.entry.block.clone()), observer: seen.as_ref().map(|s| s.entry.observer.clone()), changed_from: seen.and_then(|s| s.entry.changed.map(|ch| ch.from)) }
                })
                .collect();
            drop(b);
            let result = st.skill.as_ref()?.result();
            Some(Info::Gather(Box::new(GatherInfo {
                status: if timed_out { "Timeout".into() } else { format!("{status:?}") },
                result,
                belief,
                times: times(st),
                max_y: st.max_y,
                kinds: st.kinds.clone(),
                max_items: st.max_items,
                reads: Vec::new(),
            })))
        }
    }
}
