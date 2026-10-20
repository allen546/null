//! What `gather` may break: a policy only filters blocks (B4 §4.1, §4.8). It plans nothing.

use std::collections::{HashSet, VecDeque};

use null_bot_api::{BlockPos, BotView};

/// The box the skill may work in, corners inclusive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Claim {
    pub min: BlockPos,
    pub max: BlockPos,
}

impl Claim {
    pub fn contains(&self, p: BlockPos) -> bool {
        (self.min.x..=self.max.x).contains(&p.x) && (self.min.y..=self.max.y).contains(&p.y) && (self.min.z..=self.max.z).contains(&p.z)
    }
}

/// Why a target stays where it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Reason {
    /// The policy does not handle it (2x2 trunk, too big).
    Unsupported(String),
    OutsideClaim(BlockPos),
    NotNatural(String),
    /// No stand reaches it, however the other breaks are ordered.
    Unreachable,
    /// The quota was met first.
    Quota,
    /// A break came back refused, cancelled or without an ack.
    BreakFailed(String),
}

/// A policy's answer for one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Ignore,
    Target,
    /// Not to be gathered; reported once for the whole `group` (all cells it covers).
    Reject { reason: Reason, group: Vec<BlockPos> },
}

// Spec: specs/B4-gathering.md §4.8 (ownership check before breaking: a filter on blocks, nothing more)
pub trait Policy: Send + Sync {
    /// Called before each scan of the claim, so a policy may drop what it cached.
    fn begin_scan(&mut self) {}
    /// Is the block at `pos` (inside the claim) a block to gather?
    fn target(&mut self, view: &dyn BotView, pos: BlockPos) -> Verdict;
    /// May the block at `pos` be broken because it is in the way?
    fn may_clear(&mut self, view: &dyn BotView, pos: BlockPos) -> bool;
    /// Is the planned target at `pos` still one to break, read fresh just before the break (the
    /// world may change during a walk)? Defaults to asking `target` again.
    fn still_target(&mut self, view: &dyn BotView, pos: BlockPos) -> bool {
        self.begin_scan();
        matches!(self.target(view, pos), Verdict::Target)
    }
    /// The tool family for the targets ("axe" matches `<tier>_axe`).
    fn tool(&self) -> &'static str;
    /// A reason to refuse the whole job before it starts.
    fn refuses(&self) -> Option<String> {
        None
    }
}

fn short(n: &str) -> &str {
    n.strip_prefix("minecraft:").unwrap_or(n)
}

fn neighbours(p: BlockPos) -> impl Iterator<Item = BlockPos> {
    (-1..=1).flat_map(move |dx| (-1..=1).flat_map(move |dy| (-1..=1).map(move |dz| (dx, dy, dz)))).filter(|d| *d != (0, 0, 0)).map(move |(dx, dy, dz)| BlockPos::new(p.x + dx, p.y + dy, p.z + dz))
}

// Spec: specs/B4-gathering.md §4.8 (break only what is clearly natural: an allowlist of what may touch a tree)
/// Blocks that may sit against a natural tree. Anything else (planks, torches, signs, other logs)
/// reads as a build.
fn natural_neighbour(name: &str, log: &str) -> bool {
    let n = short(name);
    matches!(n, "air" | "cave_air" | "void_air")
        || n == short(log)
        || n.ends_with("_leaves")
        || matches!(
            n,
            "water" | "snow" | "grass_block" | "dirt" | "coarse_dirt" | "podzol" | "rooted_dirt" | "mycelium" | "mud" | "moss_block" | "moss_carpet" | "stone" | "deepslate" | "sand" | "red_sand" | "gravel" | "clay" | "andesite" | "granite" | "diorite" | "tuff" | "short_grass" | "tall_grass" | "fern" | "large_fern" | "dead_bush" | "vine" | "bee_nest" | "brown_mushroom" | "red_mushroom" | "dandelion" | "poppy" | "azure_bluet" | "cornflower" | "oxeye_daisy" | "lily_of_the_valley" | "allium" | "blue_orchid" | "lilac" | "rose_bush" | "peony" | "sunflower" | "sweet_berry_bush" | "bush"
        )
}

const MAX_LOGS: usize = 120;

/// Logs of natural single-trunk trees of one type inside the claim. Leaves with `persistent=false`
/// in the claim may be cleared.
pub struct TreePolicy {
    log: String,
    claim: Claim,
    refusal: Option<String>,
    judged: std::collections::HashMap<BlockPos, Verdict>,
}

impl TreePolicy {
    pub fn new(tree_type: &str, claim: Claim) -> Self {
        let t = short(tree_type);
        // Mangroves and 2x2 giants have their own patterns (B4 §4.3).
        let refusal = matches!(t, "dark_oak" | "mangrove" | "crimson" | "warped").then(|| format!("unsupported tree type {t}"));
        Self { log: format!("minecraft:{t}_log"), claim, refusal, judged: Default::default() }
    }

    // Spec: specs/B4-gathering.md §4.3 (single-trunk trees only)
    fn read_logs(&self, view: &dyn BotView, seed: BlockPos) -> (Vec<BlockPos>, bool) {
        let is_log = |p: BlockPos| view.block_at(p).is_some_and(|n| short(&n) == short(&self.log));
        let mut seen: HashSet<BlockPos> = HashSet::from([seed]);
        let mut queue = VecDeque::from([seed]);
        while let Some(p) = queue.pop_front() {
            for q in neighbours(p) {
                if !seen.contains(&q) && is_log(q) {
                    seen.insert(q);
                    queue.push_back(q);
                }
            }
            if seen.len() > MAX_LOGS {
                break;
            }
        }
        let big = seen.len() > MAX_LOGS;
        let giant = seen.iter().any(|p| [(1, 0), (0, 1), (1, 1)].iter().all(|(dx, dz)| seen.contains(&BlockPos::new(p.x + dx, p.y, p.z + dz))));
        let mut logs: Vec<BlockPos> = seen.into_iter().collect();
        logs.sort_by_key(|p| (p.y, p.x, p.z));
        (logs, big || giant)
    }

    // Spec: specs/B4-gathering.md §4.8 (a log attached to leaves with persistent=false is a natural tree; asymmetric rule)
    fn judge(&self, view: &dyn BotView, seed: BlockPos) -> (Vec<BlockPos>, Result<(), Reason>) {
        let (logs, unsupported) = self.read_logs(view, seed);
        if unsupported {
            let why = if logs.len() > MAX_LOGS { "too many connected logs" } else { "2x2 trunk" };
            return (logs, Err(Reason::Unsupported(why.into())));
        }
        if let Some(p) = logs.iter().find(|p| !self.claim.contains(**p)) {
            return (logs.clone(), Err(Reason::OutsideClaim(*p)));
        }
        let set: HashSet<BlockPos> = logs.iter().copied().collect();
        let mut touching = 0;
        for &l in &logs {
            for q in neighbours(l).filter(|q| !set.contains(q)) {
                let Some(n) = view.block_at(q) else { return (logs.clone(), Err(Reason::NotNatural(format!("unloaded cell at {q:?}")))) };
                if n.ends_with("_leaves") {
                    if view.block_property(q, "persistent").as_deref() != Some("false") {
                        return (logs.clone(), Err(Reason::NotNatural(format!("leaves at {q:?} are not known to be natural"))));
                    }
                    touching += 1;
                } else if !natural_neighbour(&n, &self.log) {
                    return (logs.clone(), Err(Reason::NotNatural(format!("{n} at {q:?} next to the tree"))));
                }
            }
        }
        if touching == 0 {
            return (logs, Err(Reason::NotNatural("no leaves on the logs".into())));
        }
        (logs, Ok(()))
    }
}

impl Policy for TreePolicy {
    fn begin_scan(&mut self) {
        self.judged.clear();
    }

    // Spec: specs/B4-gathering.md §4.8 (the tree policy's verdict for one log, cached per scan)
    fn target(&mut self, view: &dyn BotView, pos: BlockPos) -> Verdict {
        if view.block_at(pos).is_none_or(|n| short(&n) != short(&self.log)) {
            return Verdict::Ignore;
        }
        if let Some(v) = self.judged.get(&pos) {
            return v.clone();
        }
        let (logs, res) = self.judge(view, pos);
        for &l in &logs {
            let v = match &res {
                Ok(()) => Verdict::Target,
                Err(r) => Verdict::Reject { reason: r.clone(), group: logs.clone() },
            };
            self.judged.insert(l, v);
        }
        self.judged[&pos].clone()
    }

    // Spec: specs/B4-gathering.md §4.8 (re-read before the break: still the log, nothing built against it)
    /// A local check, not a new tree judgement: leaves cleared around the trunk would make the full
    /// judgement ("no leaves on the logs") fail for a tree that was fine when planned.
    fn still_target(&mut self, view: &dyn BotView, pos: BlockPos) -> bool {
        self.claim.contains(pos)
            && view.block_at(pos).is_some_and(|n| short(&n) == short(&self.log))
            && neighbours(pos).all(|q| view.block_at(q).is_some_and(|n| natural_neighbour(&n, &self.log) && (!n.ends_with("_leaves") || view.block_property(q, "persistent").as_deref() == Some("false"))))
    }

    // Spec: specs/B4-gathering.md §4.3 (clear only natural leaves, inside the claim)
    fn may_clear(&mut self, view: &dyn BotView, pos: BlockPos) -> bool {
        self.claim.contains(pos) && view.block_at(pos).is_some_and(|n| n.ends_with("_leaves")) && view.block_property(pos, "persistent").as_deref() == Some("false")
    }

    fn tool(&self) -> &'static str {
        "axe"
    }

    fn refuses(&self) -> Option<String> {
        self.refusal.clone()
    }
}
