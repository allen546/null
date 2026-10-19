//! Null's world belief, minimal form (F2, S5). A store of blocks as last seen, keyed by fixed
//! 512x512 tiles (AD-F2.1). It depends only on the bot API: no Azalea, no Bevy.
//! Not here yet: persistence, event log, node merging, entities, containers, hazards.

use std::collections::HashMap;

use null_bot_api::{BlockPos, BotView};

/// Tile edge in blocks (AD-F2.1). Tiles span all heights.
pub const TILE_SIZE: i32 = 512;

/// Tile coordinates of a position. Floor division, so negative coordinates tile correctly.
// Spec: specs/F2-world-model.md AD-F2.1 (fixed 512x512 tiles)
pub fn tile_of(pos: BlockPos) -> (i32, i32) {
    (pos.x.div_euclid(TILE_SIZE), pos.z.div_euclid(TILE_SIZE))
}

/// Strip the "minecraft:" prefix so "oak_log" and "minecraft:oak_log" compare equal.
fn short(name: &str) -> &str {
    name.strip_prefix("minecraft:").unwrap_or(name)
}

/// Air in any of its three vanilla forms.
pub fn is_air(name: &str) -> bool {
    matches!(short(name), "air" | "cave_air" | "void_air")
}

/// The previous block of a cell and the tick the cell was seen to differ.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub from: String,
    pub at_tick: u64,
}

/// What the store believes about one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Block name as `BotView::block_at` gives it ("minecraft:oak_log").
    pub block: String,
    /// Who saw it (the bot's account name).
    pub observer: String,
    /// Game tick of the observation.
    pub tick: u64,
    /// Set when a newer observation showed a different block than the one before it.
    pub changed: Option<Change>,
}

/// An entry with its age at a given tick.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Seen {
    pub pos: BlockPos,
    pub entry: Entry,
    /// `now - entry.tick`, saturating.
    pub age: u64,
}

/// Which block names a query accepts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockMatch {
    /// One name, with or without the "minecraft:" prefix.
    Exact(String),
    /// Names ending with the suffix, e.g. "_log".
    Suffix(String),
    /// Names starting with the prefix, e.g. "stripped_".
    Prefix(String),
}

impl BlockMatch {
    pub fn matches(&self, name: &str) -> bool {
        let n = short(name);
        match self {
            BlockMatch::Exact(e) => short(e) == n,
            BlockMatch::Suffix(s) => n.ends_with(s.as_str()),
            BlockMatch::Prefix(p) => n.starts_with(p.as_str()),
        }
    }
}

/// "oak_log" is exact; "*_log" is a suffix; "stripped_*" is a prefix.
impl From<&str> for BlockMatch {
    fn from(s: &str) -> Self {
        if let Some(rest) = s.strip_prefix('*') {
            BlockMatch::Suffix(rest.to_string())
        } else if let Some(rest) = s.strip_suffix('*') {
            BlockMatch::Prefix(short(rest).to_string())
        } else {
            BlockMatch::Exact(s.to_string())
        }
    }
}

/// The belief store.
#[derive(Default, Debug)]
pub struct Belief {
    tiles: HashMap<(i32, i32), HashMap<BlockPos, Entry>>,
}

impl Belief {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `observer` saw `block` at `pos` at game tick `tick`.
    /// Returns true when the store took it (false: an older observation than the one held).
    // Spec: specs/F2-world-model.md §4.1, §4.2 (newest wins), §4.9 (change recorded)
    pub fn observe(&mut self, observer: &str, pos: BlockPos, block: &str, tick: u64) -> bool {
        let tile = self.tiles.entry(tile_of(pos)).or_default();
        match tile.get_mut(&pos) {
            None => {
                tile.insert(pos, Entry { block: block.to_string(), observer: observer.to_string(), tick, changed: None });
                true
            }
            // Equal ticks: the later call wins (receive order, F2 §4.2).
            Some(old) if tick < old.tick => false,
            Some(old) => {
                if old.block != block {
                    old.changed = Some(Change { from: std::mem::replace(&mut old.block, block.to_string()), at_tick: tick });
                }
                old.observer = observer.to_string();
                old.tick = tick;
                true
            }
        }
    }

    /// Record many cells seen by one observer at one tick. Returns how many the store took.
    // Spec: specs/F2-world-model.md §4.1
    pub fn observe_batch<'a>(
        &mut self,
        observer: &str,
        tick: u64,
        cells: impl IntoIterator<Item = (BlockPos, &'a str)>,
    ) -> usize {
        cells.into_iter().filter(|(p, b)| self.observe(observer, *p, b, tick)).count()
    }

    /// Record every loaded cell in the cube of half-size `radius` around `center`, as `read` reports it.
    /// Unloaded cells (`None`) are skipped, so they stay unknown. Returns how many cells were taken.
    // Spec: specs/F2-world-model.md §4.1 (observations from a bot's own view)
    pub fn observe_scan(
        &mut self,
        observer: &str,
        tick: u64,
        center: BlockPos,
        radius: i32,
        mut read: impl FnMut(BlockPos) -> Option<String>,
    ) -> usize {
        let mut taken = 0;
        for x in center.x - radius..=center.x + radius {
            for y in center.y - radius..=center.y + radius {
                for z in center.z - radius..=center.z + radius {
                    let pos = BlockPos::new(x, y, z);
                    if let Some(b) = read(pos) {
                        taken += self.observe(observer, pos, &b, tick) as usize;
                    }
                }
            }
        }
        taken
    }

    /// `observe_scan` over a bot's view, at the view's tick.
    // Spec: specs/F2-world-model.md §4.1
    pub fn observe_view(&mut self, observer: &str, view: &dyn BotView, center: BlockPos, radius: i32) -> usize {
        self.observe_scan(observer, view.tick(), center, radius, |p| view.block_at(p))
    }

    /// The entry for `pos`, with its age at `now`. `None` means unknown, never air.
    // Spec: specs/F2-world-model.md §3, §4.3 (every answer carries its age)
    pub fn block(&self, pos: BlockPos, now: u64) -> Option<Seen> {
        let entry = self.tiles.get(&tile_of(pos))?.get(&pos)?;
        Some(Seen { pos, entry: entry.clone(), age: now.saturating_sub(entry.tick) })
    }

    /// Cells believed to match `kind` within `radius` blocks (Euclidean) of `near`, nearest first.
    /// Ties break by x, y, z so the order is deterministic.
    // Spec: specs/F2-world-model.md §3, §4.3
    pub fn find_blocks(&self, kind: impl Into<BlockMatch>, near: BlockPos, radius: i32, now: u64) -> Vec<Seen> {
        let kind = kind.into();
        let (tx0, tz0) = tile_of(BlockPos::new(near.x - radius, 0, near.z - radius));
        let (tx1, tz1) = tile_of(BlockPos::new(near.x + radius, 0, near.z + radius));
        let r2 = i64::from(radius) * i64::from(radius);
        let mut out: Vec<(i64, Seen)> = Vec::new();
        for tx in tx0..=tx1 {
            for tz in tz0..=tz1 {
                let Some(tile) = self.tiles.get(&(tx, tz)) else { continue };
                for (pos, e) in tile {
                    let d2 = dist2(*pos, near);
                    if d2 <= r2 && kind.matches(&e.block) {
                        out.push((d2, Seen { pos: *pos, entry: e.clone(), age: now.saturating_sub(e.tick) }));
                    }
                }
            }
        }
        out.sort_by_key(|(d2, s)| (*d2, s.pos.x, s.pos.y, s.pos.z));
        out.into_iter().map(|(_, s)| s).collect()
    }

    /// Cells whose block was seen to change at `since` or later, oldest change first.
    // Spec: specs/F2-world-model.md §4.9, §3 (`changes`), minimal form without regions
    pub fn changes_since(&self, since: u64) -> Vec<Seen> {
        let mut out: Vec<Seen> = self
            .tiles
            .values()
            .flatten()
            .filter(|(_, e)| e.changed.as_ref().is_some_and(|c| c.at_tick >= since))
            .map(|(p, e)| Seen { pos: *p, entry: e.clone(), age: 0 })
            .collect();
        out.sort_by_key(|s| (s.entry.changed.as_ref().map(|c| c.at_tick), s.pos.x, s.pos.y, s.pos.z));
        out
    }

    /// Number of cells held.
    pub fn len(&self) -> usize {
        self.tiles.values().map(HashMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.tiles.values().all(HashMap::is_empty)
    }
}

fn dist2(a: BlockPos, b: BlockPos) -> i64 {
    let (dx, dy, dz) = (i64::from(a.x - b.x), i64::from(a.y - b.y), i64::from(a.z - b.z));
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests;
