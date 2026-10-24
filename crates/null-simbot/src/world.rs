//! The toy world: a flat grid with trees at known positions. Spec: specs/F3-test-harness.md §4.5
use null_proto::{Dimension, Loc, Pos, ServerId};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

/// splitmix64: tiny and seedable, so every choice replays exactly (F3 §4.5).
// Spec: specs/F3-test-harness.md §4.5
#[derive(Clone, Debug)]
pub struct SimRng(u64);

impl SimRng {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// A number in `lo..=hi`.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next_u64() % u64::from(hi - lo + 1)) as u32
    }
}

// Spec: specs/F3-test-harness.md §4.3
pub struct SimWorld {
    pub server: ServerId,
    trees: BTreeMap<Pos, u32>,
}

pub type SharedWorld = Arc<Mutex<SimWorld>>;

impl SimWorld {
    /// `count` trees on distinct cells of a square of side `extent`, with 4 to 6 logs each.
    // Spec: specs/F3-test-harness.md §4.3
    pub fn generate(seed: u64, server: ServerId, count: usize, extent: i32) -> Self {
        let mut rng = SimRng::new(seed);
        let mut trees = BTreeMap::new();
        while trees.len() < count {
            let p = Pos::new(
                rng.range(0, extent as u32 - 1) as i32,
                64,
                rng.range(0, extent as u32 - 1) as i32,
            );
            trees.entry(p).or_insert_with(|| rng.range(4, 6));
        }
        Self { server, trees }
    }
    pub fn shared(self) -> SharedWorld {
        Arc::new(Mutex::new(self))
    }
    pub fn loc(&self, pos: Pos) -> Loc {
        Loc {
            server: self.server.clone(),
            dim: Dimension::Overworld,
            pos,
        }
    }
    /// What an operation believes about the trees: (location, logs), in position order.
    // Spec: specs/F3-test-harness.md §4.3
    pub fn tree_list(&self) -> Vec<(Loc, u32)> {
        self.trees.iter().map(|(p, n)| (self.loc(*p), *n)).collect()
    }
    // Spec: specs/F3-test-harness.md §4.3
    pub fn logs_at(&self, pos: Pos) -> Option<u32> {
        self.trees.get(&pos).copied()
    }
    /// Fell the tree: its logs, or `None` when it is already gone.
    // Spec: specs/F3-test-harness.md §4.3
    pub fn take(&mut self, pos: Pos) -> Option<u32> {
        self.trees.remove(&pos)
    }
    pub fn trees_left(&self) -> usize {
        self.trees.len()
    }
}
