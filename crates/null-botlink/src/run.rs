//! One `fell` run on a real bot: the skill the host steps, and the handle the bus side watches.
use null_belief::Belief;
use null_bot_api::{BlockPos, BotCommands, BotView, Skill, SkillStatus};
use null_proto::Pos;
use null_skills::{Claim, Gather};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

// Spec: specs/B4-gathering.md §3
/// What a `fell` job asks of the bot: the box is the only place it may break.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FellSpec {
    pub tree_type: String,
    pub quota: u32,
    pub min: Pos,
    pub max: Pos,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunStatus {
    Done,
    Failed(String),
    /// The bus side asked for a stop (halt, lease, budget) and the skill stopped.
    Stopped,
}

// Spec: specs/CONVENTIONS.md §3.1
/// How a run ended. `items` are the inventory gains over the run, by item name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunEnd {
    pub status: RunStatus,
    pub items: BTreeMap<String, u32>,
    pub gathered: u32,
    /// Targets left standing, and item drops left on the ground.
    pub left: u32,
    pub drops_left: u32,
}

#[derive(Default)]
struct Shared {
    stop: bool,
    end: Option<RunEnd>,
}

/// Shared between the skill (on the bot's tick) and the bus side. Short locks only: neither side waits on the other.
#[derive(Clone, Default)]
pub struct RunHandle(Arc<Mutex<Shared>>);

impl RunHandle {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn request_stop(&self) {
        self.0.lock().unwrap().stop = true;
    }
    pub fn stop_requested(&self) -> bool {
        self.0.lock().unwrap().stop
    }
    /// The first end wins: a skill the host cut short must not overwrite what the skill reported.
    pub fn finish(&self, end: RunEnd) {
        self.0.lock().unwrap().end.get_or_insert(end);
    }
    pub fn take_end(&self) -> Option<RunEnd> {
        self.0.lock().unwrap().end.take()
    }
}

fn counts(view: &dyn BotView) -> HashMap<String, u32> {
    let mut m = HashMap::new();
    for s in view.inventory().iter().flatten().chain(view.offhand()) {
        *m.entry(s.item.clone()).or_insert(0) += s.count as u32;
    }
    m
}

// Spec: specs/B4-gathering.md §3 (fell is gather with the tree policy; a stop ends it and reports what it took)
/// `fell` plus a stop switch. The skill id is "fell".
pub struct FellRun {
    gather: Gather,
    handle: RunHandle,
    start: Option<HashMap<String, u32>>,
}

impl FellRun {
    pub fn new(spec: &FellSpec, observer: &str, handle: RunHandle) -> Self {
        let b = |p: Pos| BlockPos::new(p.x, p.y, p.z);
        let claim = Claim { min: b(spec.min), max: b(spec.max) };
        let gather = null_skills::fell(&spec.tree_type, spec.quota, false, claim, observer, Arc::new(Mutex::new(Belief::default())));
        Self { gather, handle, start: None }
    }

    fn end(&self, view: &dyn BotView, status: RunStatus) -> RunEnd {
        let r = self.gather.result();
        // A stop reads the gains itself: `gather` fills its yield only when it ends.
        let items = if status == RunStatus::Stopped {
            let start = self.start.clone().unwrap_or_default();
            counts(view).into_iter().filter_map(|(k, v)| v.checked_sub(*start.get(&k).unwrap_or(&0)).filter(|d| *d > 0).map(|d| (k, d))).collect()
        } else {
            r.yield_items.clone()
        };
        RunEnd { status, items, gathered: r.gathered, left: r.left.len() as u32, drops_left: r.drops_left.len() as u32 }
    }
}

impl Skill for FellRun {
    fn id(&self) -> &str {
        "fell"
    }

    // Spec: specs/B4-gathering.md §3
    fn step(&mut self, view: &dyn BotView, cmds: &mut dyn BotCommands) -> SkillStatus {
        self.start.get_or_insert_with(|| counts(view));
        if self.handle.stop_requested() {
            cmds.stop_nav();
            // A break in progress would finish after the skill ends, in a box the bot no longer holds (S6a live run 2026-10-28).
            cmds.cancel_break();
            let end = self.end(view, RunStatus::Stopped);
            self.handle.finish(end);
            return SkillStatus::Done;
        }
        let st = self.gather.step(view, cmds);
        match &st {
            SkillStatus::Running => {}
            SkillStatus::Done => self.handle.finish(self.end(view, RunStatus::Done)),
            SkillStatus::Failed(why) => self.handle.finish(self.end(view, RunStatus::Failed(why.clone()))),
        }
        st
    }
}
