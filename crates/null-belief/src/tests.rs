use super::*;
use null_bot_api::fake::FakeView;

fn p(x: i32, y: i32, z: i32) -> BlockPos {
    BlockPos::new(x, y, z)
}

#[test]
fn newer_replaces_older() {
    let mut b = Belief::new();
    assert!(b.observe("a", p(1, 64, 1), "minecraft:oak_log", 10));
    assert!(b.observe("b", p(1, 64, 1), "minecraft:oak_log", 20));
    let s = b.block(p(1, 64, 1), 25).unwrap();
    assert_eq!((s.entry.observer.as_str(), s.entry.tick, s.age), ("b", 20, 5));
}

#[test]
fn older_never_overwrites_newer() {
    let mut b = Belief::new();
    b.observe("a", p(0, 0, 0), "minecraft:air", 50);
    assert!(!b.observe("b", p(0, 0, 0), "minecraft:stone", 40));
    let e = b.block(p(0, 0, 0), 50).unwrap().entry;
    assert_eq!((e.block.as_str(), e.observer.as_str(), e.tick), ("minecraft:air", "a", 50));
    assert!(e.changed.is_none());
}

#[test]
fn equal_tick_later_call_wins() {
    let mut b = Belief::new();
    b.observe("a", p(0, 0, 0), "minecraft:stone", 5);
    assert!(b.observe("b", p(0, 0, 0), "minecraft:dirt", 5));
    assert_eq!(b.block(p(0, 0, 0), 5).unwrap().entry.block, "minecraft:dirt");
}

#[test]
fn log_to_air_is_a_change() {
    let mut b = Belief::new();
    b.observe("bot", p(3, 70, 3), "minecraft:oak_log", 10);
    assert!(b.block(p(3, 70, 3), 10).unwrap().entry.changed.is_none());
    b.observe("bot", p(3, 70, 3), "minecraft:air", 30);
    let e = b.block(p(3, 70, 3), 30).unwrap().entry;
    assert!(is_air(&e.block));
    assert_eq!(e.changed, Some(Change { from: "minecraft:oak_log".into(), at_tick: 30 }));
    assert_eq!(b.changes_since(30).len(), 1);
    assert!(b.changes_since(31).is_empty());
}

#[test]
fn same_block_again_is_not_a_change() {
    let mut b = Belief::new();
    b.observe("a", p(0, 0, 0), "minecraft:stone", 1);
    b.observe("a", p(0, 0, 0), "minecraft:stone", 9);
    let e = b.block(p(0, 0, 0), 9).unwrap().entry;
    assert_eq!(e.tick, 9);
    assert!(e.changed.is_none());
    assert!(b.changes_since(0).is_empty());
}

#[test]
fn unknown_is_not_air() {
    let b = Belief::new();
    assert!(b.block(p(5, 5, 5), 0).is_none());
    assert!(b.find_blocks("air", p(5, 5, 5), 10, 0).is_empty());
}

#[test]
fn scan_skips_unloaded_cells() {
    let mut b = Belief::new();
    let n = b.observe_scan("a", 3, p(0, 0, 0), 1, |q| (q.x >= 0).then(|| "minecraft:stone".to_string()));
    assert_eq!(n, 9 + 9);
    assert!(b.block(p(-1, 0, 0), 3).is_none());
    assert!(b.block(p(0, 0, 0), 3).is_some());
}

#[test]
fn scan_from_a_view() {
    let mut v = FakeView::default();
    v.blocks.push((p(2, 64, 2), "minecraft:birch_log".to_string()));
    let mut b = Belief::new();
    let n = b.observe_view("bot", &v, p(2, 64, 2), 2);
    assert!(n >= 1);
    assert_eq!(b.find_blocks("*_log", p(2, 64, 2), 2, 0).len(), 1);
}

#[test]
fn tile_boundaries_including_negative() {
    assert_eq!(tile_of(p(0, 0, 0)), (0, 0));
    assert_eq!(tile_of(p(511, 0, 511)), (0, 0));
    assert_eq!(tile_of(p(512, 0, 512)), (1, 1));
    assert_eq!(tile_of(p(-1, 0, -1)), (-1, -1));
    assert_eq!(tile_of(p(-512, 0, -512)), (-1, -1));
    assert_eq!(tile_of(p(-513, 0, -513)), (-2, -2));
    assert_eq!(tile_of(p(5, 300, 5)), (0, 0));
}

#[test]
fn find_across_tile_and_origin_boundaries() {
    let mut b = Belief::new();
    for x in [-2, -1, 0, 1] {
        b.observe("a", p(x, 64, 0), "minecraft:oak_log", 1);
    }
    for x in [510, 511, 512, 513] {
        b.observe("a", p(x, 64, 0), "minecraft:oak_log", 1);
    }
    assert_eq!(b.find_blocks("oak_log", p(0, 64, 0), 3, 1).len(), 4);
    assert_eq!(b.find_blocks("oak_log", p(512, 64, 0), 3, 1).len(), 4);
    assert_eq!(b.len(), 8);
}

#[test]
fn find_orders_nearest_first_and_respects_radius() {
    let mut b = Belief::new();
    b.observe("a", p(5, 0, 0), "minecraft:oak_log", 10);
    b.observe("a", p(1, 0, 0), "minecraft:oak_log", 10);
    b.observe("a", p(3, 0, 0), "minecraft:spruce_log", 4);
    b.observe("a", p(0, 0, 9), "minecraft:oak_log", 10); // outside radius 8
    b.observe("a", p(2, 0, 0), "minecraft:stone", 10);
    let r = b.find_blocks("*_log", p(0, 0, 0), 8, 14);
    let xs: Vec<_> = r.iter().map(|s| s.pos.x).collect();
    assert_eq!(xs, vec![1, 3, 5]);
    assert_eq!(r[1].age, 10);
    // Euclidean: (6,0,6) is 8.49 away, outside 8 although inside the cube.
    b.observe("a", p(6, 0, 6), "minecraft:oak_log", 10);
    assert_eq!(b.find_blocks("*_log", p(0, 0, 0), 8, 14).len(), 3);
}

#[test]
fn matching_forms() {
    assert!(BlockMatch::from("oak_log").matches("minecraft:oak_log"));
    assert!(BlockMatch::from("minecraft:oak_log").matches("oak_log"));
    assert!(!BlockMatch::from("oak_log").matches("minecraft:stripped_oak_log"));
    assert!(BlockMatch::from("*_log").matches("minecraft:stripped_oak_log"));
    assert!(BlockMatch::from("stripped_*").matches("minecraft:stripped_oak_log"));
    assert!(is_air("minecraft:cave_air") && !is_air("minecraft:stone"));
}

#[test]
fn batch_counts_taken() {
    let mut b = Belief::new();
    b.observe("a", p(0, 0, 0), "minecraft:stone", 10);
    let n = b.observe_batch("b", 5, [(p(0, 0, 0), "minecraft:dirt"), (p(1, 0, 0), "minecraft:dirt")]);
    assert_eq!(n, 1);
}
