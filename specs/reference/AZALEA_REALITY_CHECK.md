# Azalea reality check

> **Status (2026-10-14): current reference.** Its corrections are applied in the specs and ARCHITECTURE v0.8 §9. The build recipe (§2) was reproduced by the AD-F5.2 spike (`SPIKE_F5_BOUNDARY.md`), which also ran §9 step 1's join test in Azalea's simulator. A later run joined the fork to a live vanilla 1.21.11 server (spike §6): login, chunk load, walk, chest and block placement worked; mining was not tried. §8's three options were superseded by the debate's hybrid (`history/AD-F5.2-bevy-boundary.md`).

**2026-10-13.** Compares what Azalea ships today with what specs F1, F2, F5 and B1–B7 assume or plan to build, and tests whether a "latest features, 1.21.11 protocol" custom build is realistic. Nothing under `specs/` or `design/` was edited.

## 1. Bottom line

1. **The custom build is plausible, and I built it.** Azalea keeps one Minecraft version per branch, and its `1.21.11` branch is simply `main` as it stood the day before the 26.1 update. Cherry-picking 39 of the 47 later upstream commits onto it (the 8 left out are listed in §2.4) gave a tree that **type-checks on today's nightly Rust, keeps the 1.21.11 protocol, and passes the 59 upstream library and simulation tests I ran** (36 library tests, 23 client simulation tests). The 8 others need hand-merging or a codegen run, and one is 26.x-only. Details in §2.
2. **Two things would have bitten us.** (a) The pristine `1.21.11` branch does not compile on today's nightly (the compiler crashes), so it needs a pinned nightly or three small upstream fixes. (b) Azalea's code generator can no longer regenerate 1.21.11 data: the 26.1 update deleted its support for obfuscated-jar mappings. Regenerated data for the fork must use the old generator.
3. **Azalea covers more of the plumbing than the specs credit, and less of the game logic.** It has swarm mode, shared world memory, a Baritone-derived pathfinder, vanilla-faithful physics for walking and jumping, mining with correct break times, all menu types and click modes, per-state collision and outline shapes, and a scripted-packet test simulator. It has almost none of B2, B3's crafting/trading/enchanting, B5's weapon logic, B7's vehicles, or F4's telemetry.
4. **AD-B1.1 (pathfinder): reuse it first, as the spec leans.** Cost layers are possible through custom move functions. Sharing computation across bots is not possible without changing Azalea. See §5.1.
5. **AD-F2.2 (world storage): keep a separate belief store, as the spec leans.** Azalea's chunk storage already shares chunks across bots, now has a pluggable storage trait, and holds only blocks, biomes and heightmaps, with no timestamps, light, or block entities. See §5.2.
6. **Corrections the specs need** are collected in §7. The biggest: Azalea does not record server game time, which F2's `at_gt` depends on, and the swarm runs as one ECS behind one lock, which matters for the "about 1,000 bots per laptop" estimate.

## 2. Version plan: a 1.21.11 build with the latest features

### 2.1 How Azalea handles versions

- Azalea targets exactly one Minecraft version per release and lists "supporting multiple versions at the same time" as a non-goal, pointing to a separate ViaVersion plugin instead ([README](https://github.com/azalea-rs/azalea/blob/0bd22cbc/README.md)). That matches F5's "one binary per version".
- Each Minecraft version lives on its own branch of the repo (`1.21.11`, `26.1`, `26.3`, …) and `main` follows the newest release. `main` is currently `0.16.0+mc26.2`; a `26.3` branch is in progress.
- Version bumps are single large commits produced by `codegen/migrate.py`: generated blocks, shapes, registries, tags, entity metadata, language file, plus hand-fixed packets ([codegen README](https://github.com/azalea-rs/azalea/blob/0bd22cbc/codegen/README.md)).

### 2.2 Where 1.21.11 sits

| Point | What it is |
|---|---|
| `0.15.0+mc1.21.11` (2025-12-18) and `0.15.1` (2026-02-03) | The crates.io releases for 1.21.11. |
| branch [`1.21.11`](https://github.com/azalea-rs/azalea/tree/1.21.11), tip `41a9ae6a` (2026-03-22) | `main` on the day before the 26.1 bump. It is **133 commits ahead of the 0.15.1 release branch** and is itself an ancestor of `main`. |
| [`eeaf1435`](https://github.com/azalea-rs/azalea/commit/eeaf1435) 26.1, [`35013ff7`](https://github.com/azalea-rs/azalea/commit/35013ff7) 26.1.2, [`0d846995`](https://github.com/azalea-rs/azalea/commit/0d846995) 26.2 (+ two follow-up fixes) | The only version bumps after the branch point. |
| `main` tip `0bd22cbc` (2026-10-06) | 54 commits after the branch point, 7 of them version bumps or release bookkeeping, **47 version-neutral**. |

So "latest features plus 1.21.11" means: start from the `1.21.11` branch tip and take the 47 version-neutral commits.

### 2.3 What pinning an older point would lose

**Pinning the 0.15.1 release** loses 133 commits that are already on the branch, including: pathfinder swimming, the simulation-based path executor ([`268c6258`](https://github.com/azalea-rs/azalea/commit/268c6258)), a radix-heap and cache-locality speedup, `EntityRef`, extensible `Account` and extensible `ChunkStorage` ([`88606d9c`](https://github.com/azalea-rs/azalea/commit/88606d9c)), the speed effect, threaded ECS, swarm-pathfinder optimisations plus the [performance guide](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea/src/_docs/performance.md), the `Instance`→`World` rename, protocol memory-leak fixes and a protocol fuzzer. I recommend against it.

**Pinning the branch tip unmodified** loses these upstream fixes and features, all version-neutral:

| Upstream commit | Why we want it |
|---|---|
| [`e347748e`](https://github.com/azalea-rs/azalea/commit/e347748e) | Fixes left and right movement being swapped: per the diff, a bot told to strafe left moved right while its input flags said left. Matters for anti-cheat (B1 §4.2). |
| [`a8959f62`](https://github.com/azalea-rs/azalea/commit/a8959f62), [`a630da47`](https://github.com/azalea-rs/azalea/commit/a630da47) | Physics: crouching in water; seagrass and kelp count as water. |
| [`c963b0a5`](https://github.com/azalea-rs/azalea/commit/c963b0a5), [`b71a3f77`](https://github.com/azalea-rs/azalea/commit/b71a3f77), [`2d3e4194`](https://github.com/azalea-rs/azalea/commit/2d3e4194), [`b9995f10`](https://github.com/azalea-rs/azalea/commit/b9995f10) | Pathfinder: long-path stability, skip tolerance, instant updates for simple paths, faster cached sections. |
| [`86dc16c5`](https://github.com/azalea-rs/azalea/commit/86dc16c5) | `SwarmEvent::Tick`. |
| [`2e601b43`](https://github.com/azalea-rs/azalea/commit/2e601b43), [`8a32e091`](https://github.com/azalea-rs/azalea/commit/8a32e091) | Batch packet writing. |
| [`0d392ddc`](https://github.com/azalea-rs/azalea/commit/0d392ddc) | `TokioRuntimeHandle` resource. |
| [`9ffd0e80`](https://github.com/azalea-rs/azalea/commit/9ffd0e80) | `Client` functions return `AzaleaResult` instead of panicking when an entity's data is missing. Important for a 2,000-bot process where one panic would take down every bot. |
| [`153c90aa`](https://github.com/azalea-rs/azalea/commit/153c90aa), [`b65fa8cf`](https://github.com/azalea-rs/azalea/commit/b65fa8cf), [`725d3f7f`](https://github.com/azalea-rs/azalea/commit/725d3f7f) | Keep it compiling on current nightly Rust (see §2.5). |
| [`40455fc1`](https://github.com/azalea-rs/azalea/commit/40455fc1), [`0d0e253f`](https://github.com/azalea-rs/azalea/commit/0d0e253f) | Faster packet reads. |

### 2.4 Dry-run of the backport

Method: `git cherry-pick -x` of each of the 47 version-neutral commits, in order, onto the `1.21.11` tip. Trivial conflicts in `CHANGELOG.md`, `Cargo.lock`, example programs and codegen scripts were resolved mechanically.

- **39 applied** (several only after mechanically resolving the trivial files above). The set of 38 I built and tested is listed in the appendix; the 39th (`cabc8b60`, brigadier `Result`) applied afterwards and was not compiled.
- **8 did not apply**, in four groups:

| Group | Commits | What it takes |
|---|---|---|
| 26.x-only | `ddef3711` (attack packet serialisation) | Not needed: 1.21.11 has no `ServerboundAttack`. |
| Generated data | `eeb729eb` (component impls), `ee7358eb` (shapes for blocks with random offsets, so bots stop snagging on bamboo and dripstone) | Re-run the 1.21.11 codegen or hand-port. Needs the old generator (§2.5). |
| Dependency bumps | `5e0484c9`, `9503c287` | `9503c287` is the Bevy 0.18→0.19 upgrade. Skip until we choose to take it. |
| Attack and movement cluster | `8d09b4ee` (`GameMode` split), `e73bb165` + `d3422ec8` (elytra) | Hand-merge in `attack.rs` and `movement.rs`. The elytra commits depend on the `GameMode` split, which collides with the 26.1 attack rewrite. Elytra is "later" in B1 anyway. |

- **Reverse direction also works.** Reverting the five bump commits on `main`: four revert cleanly; reverting 26.1 itself conflicts in nine files (generated data, `Cargo.lock`, attack code and the `s_attack.rs` packet). I did not pursue this route.

### 2.5 Build results

| Tree | Toolchain | Result |
|---|---|---|
| `1.21.11` branch tip, unmodified | nightly 2026-10-12 (rustc 1.101) | **Compiler crash (ICE) in `azalea-entity`.** |
| same | nightly 2026-03-22 | Type-checks. |
| `main` | nightly 2026-10-12 | Type-checks. |
| branch + 35 cherry-picks | nightly 2026-10-12 | Still crashes (the nightly fixes had not applied yet because of a cascade from `9ffd0e80`). |
| branch + 38 cherry-picks (adds `9ffd0e80`, `b65fa8cf`, `725d3f7f`) | nightly 2026-10-12 | **Type-checks.** Library tests: 36 passed. Client simulation tests: 23 passed. |

Notes:
- Azalea needs **nightly Rust** (`rust-toolchain.toml` says `nightly`, with feature gates such as `generic_const_exprs`). Upstream needed three fix commits for newer nightlies between August and September 2026 (`153c90aa`, `b65fa8cf`, `725d3f7f`). The fork must either pin a nightly date or keep taking such fixes.
- **Codegen cannot rebuild 1.21.11 data today.** The 26.1 commit deleted `codegen/lib/mappings.py` (Mojang name mappings for obfuscated jars; 26.1 jars are unobfuscated) and switched to a different Burger fork. The generated 1.21.11 data is already committed on the branch, so the fork doesn't need to regenerate it unless it takes the two generated-data commits above, and then it needs the branch's own `codegen/` directory.
- I ran type-checks and tests only. I did not join a server, and I did not check release-mode performance.

### 2.6 What the fork costs to keep

- Upstream moves roughly every three months (releases 0.13 → 0.16: June 2025, Sept 2025, Dec 2025, Mar 2026) and Bevy has moved four majors in fourteen months: 0.16 (April 2025), 0.17 (2025-09-28), 0.18 (2026-01-14, the version on the `1.21.11` branch), 0.19 (2026-07-02, on `main`). A frozen 1.21.11 fork chooses when to take a Bevy bump; that decouples us from Bevy churn at the price of falling behind upstream.
- **Bus factor is one.** 245 of the 262 commits since October 2025 are by one maintainer, and the README calls Azalea a hobby project maintained primarily by one person. Our fork should assume it may have to carry its own fixes.
- **Merge rule that falls out of the data:** merge `main` continuously **except** the version-bump commits, and keep the generated-data fixes as a separate, regenerate-on-merge step. For a later port (F5 §4.7), the equivalent is merging upstream's own bump commit for the target version; F5 step 1 "pin the Azalea release" is the wrong shape for a fork.
- **Precedent for forks:** ShaysBot patches Azalea with a fork, stasis-bot carries its own 1.21.4 inventory backport branch, potato-bot-2 pinned a `1.19.2` branch, ErrorNoWatcher tracks git `main`. Forking Azalea is the normal pattern.

**Verdict: realistic.** The fork is a bounded merge-and-regenerate job, not a rewrite. The remaining unknown is behaviour against a live 1.21.11 server, which only a join test can settle.

## 3. Coverage by area

Verdicts: **Covers** (use as is), **Partly** (usable base, gaps named), **Missing** (we build it). "Spec plan" says what the specs currently intend.

| Area | Azalea | Spec plan | Notes |
|---|---|---|---|
| Swarm mode | **Covers** the process shape. `SwarmBuilder`, per-bot state, shared swarm state, `join_delay`, `reconnect_after`, per-bot proxy options ([swarm/mod.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea/src/swarm/mod.rs)). | F1 §4.1 uses it. | One swarm is **one Bevy ECS behind one `RwLock`**; the schedule loop takes the write lock each pass (Update ≤60 Hz, GameTick 20 Hz) ([client.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea-client/src/client.rs)). See §7. |
| ECS and plugins | **Covers.** Bevy ECS; a `DefaultPlugins` group of ~25 plugins; actions are ECS messages (`StartWalkEvent`, `LookAtEvent`, `StartMiningBlockEvent`, `SendGamePacketEvent`…); `ReceiveGamePacketEvent` for watching packets. | F1: skills and reflexes as Bevy plugins. | Azalea's own guide recommends writing bot logic as a plugin at scale, since the `Client` handle path takes locks. |
| World storage | **Partly.** `WeakChunkStorage`: per-chunk `Arc` shared across bots, dropped when no bot holds the chunk; pluggable via `ChunkStorageTrait` on the 1.21.11 branch. | F2 §4.1, AD-F2.2. | Chunks hold blocks, biomes, heightmaps only. See §5.2. |
| Entity tracking | **Covers** the basics. Entities are ECS entities shared across a swarm; lookup by UUID (`entity_by_uuid`), nearest-entity queries, generated metadata for all entity kinds. | F2 §4.5. | Timestamps and cargo tags are ours. |
| Chunk and bandwidth control | **Partly.** View distance via `ClientInformation`; chunk batch acknowledgements pace the server by measured processing speed. | F1 §4.11. | **No byte counters anywhere.** Per-bot and per-host traffic must be added (socket wrapper in the fork, or OS-level socket stats). |
| Physics | **Partly.** Vanilla-faithful walking, jumping, fluids, ladders, collision. | B1 §4.2 | Gaps: using-item slowdown, riding vehicles, sprint swimming, entity pushing, world border, slow falling, powder snow handling. See §6 (B1, B2). |
| Pathfinding | **Partly.** See §5.1. | B1 §4.1, AD-B1.1 | |
| Mining and breaking | **Covers.** Break progress from hardness, tool, efficiency, haste, fatigue, on-ground; `auto_tool`. | B6 §4.4 | Aqua-affinity underwater penalty is a TODO in the code. |
| Block placing and use | **Partly.** Right-click from the raycast hit; sneaking; sequence-numbered ack and prediction rollback tracking; raw packets via `write_packet`. | B6 §4.2, §4.6 | No client-side placement prediction; look direction snaps instantly (no smoothing); interaction shapes (cauldron, hopper…) are an empty TODO. |
| Inventory and containers | **Covers** the mechanics. All vanilla click modes (pickup, quick-move, swap, clone, throw, quick-craft, pickup-all); menu types for every container incl. crafter, anvil, enchanting, brewing, merchant; `open_container_at`. | B3 §4.7 | See B3 in §6 for what is missing. |
| Crafting, smelting, trading, enchanting, brewing | **Missing.** Server recipe packets, recipe-book packets, merchant offers and container progress data are received and discarded. The serverbound packets exist in `azalea-protocol`, with no helpers. | B3 | |
| Item use (eating, bows, shields) | **Missing** beyond a single right-click; no hold or release state. | B2, B5 | |
| Combat | **Partly.** `attack`, cooldown tracking, a killaura example. | B5 | No shield, bow, crossbow, trident, mace, totem or ballistic logic. |
| Vehicles | **Missing.** Passenger handling is a TODO and `move_vehicle` is a stub. | B7 §4.4 | |
| Elytra | **Main only.** Not on the 1.21.11 branch. | B1 (later) | Part of the 8 unapplied commits. |
| Death and respawn | **Covers.** `auto_respawn`, death event with the kill packet. | B2 §4.7 | |
| Auth and accounts | **Covers.** Microsoft auth with token cache, offline accounts, `AccountTrait` for custom accounts, SOCKS5 proxy for the game connection and for the session server. | F1 §4.2 | Credential vault, lease, rotation are ours. |
| Reconnect | **Partly.** Fixed delay (default 5 s) and a `SwarmEvent::Disconnect` hook. | F1 §4.3 | Backoff with jitter and failure classification are ours. |
| Game data | **Partly.** Block states, properties, behaviour (hardness, friction, jump factor, explosion resistance), collision **and** outline shapes for every state, all registries, tags, default item components (tool, food…), entity dimensions and metadata. | F5 §4.2 | Not shipped: recipes, loot tables, placement/support rules, spawn rules, mob stats beyond attributes. |
| Test harness | **Partly.** `Simulation` feeds scripted packets to a headless client; 23 integration tests use it; criterion benches for pathfinder and physics. | F3 | No server; F3's real-server scenarios are still ours. |
| Telemetry and ops | **Missing.** `tracing` logs, one MSPT example. | F4 | |

## 4. Where specs plan to rebuild what Azalea has

| Spec | Plan | What Azalea already has | Suggested change |
|---|---|---|---|
| F5 §4.2 | "Extracted collision and outline shapes, stored per version" | Both, for every block state (32,366 on `main`), plus block behaviour constants, generated by `codegen/` | Take shapes and block behaviour from Azalea; run the data generator only for what Azalea lacks (recipes, loot, support rules, mob data). |
| F5 §4.7 | Porting procedure: pin release, generate data, fix adapter | Upstream's `migrate.py` plus bump commits do this | Replace step 1–2 with "merge upstream's bump commit, run migrate.py". |
| F5 §4.1 | One binary per version | Same constraint, by design | None. |
| F2 §4.1 | Bots share Azalea's world storage | Yes, via weak shared chunks | None. |
| F1 §4.3 | Reconnect and backoff | A fixed-delay reconnect only | Keep ours; use the `SwarmEvent::Disconnect` hook. |
| F1 §4.5, B1 §4.9, B2 §4.9 … | Reflex host and arbitration | Nothing | Ours; the ECS gives us ordering tools (system sets, run conditions). |
| B1 §4.1 | A* over a block graph, Baritone-style | A Baritone-derived A* with 8 move families | Reuse; extend moves (§5.1). |
| B6 §4.4 | Break time from hardness, tool, effects | `get_mine_progress` | Reuse. |
| B3 §4.7 | Container open, click, verify | `open_container_at`, all click modes, menu types | Reuse for mechanics; verification logic is ours. |
| B6 §4.6 | Ack and readback | Sequence-numbered ack handling and rollback bookkeeping | Reuse the ack tracking; add the state comparison. |
| B6 §4.7 | Packet replay | Raw packet send exists | Reuse sending; see the replay caveat in §6 (B6). |

I found no case where a spec plans a large component that Azalea already fully provides, apart from the data-generation and porting items above. The risk runs the other way: the specs lean on Azalea for things it doesn't do.

## 5. The two open decisions

### 5.1 AD-B1.1: reuse Azalea's pathfinder or write our own

The spec's question: can it take pluggable costs, and can it share computation across bots in one process?

**Pluggable costs: partly yes, through move functions.**
- `PathfinderOpts::successors_fn` takes a function pointer `fn(&mut MovesCtx, RelBlockPos)` that pushes edges, each with a cost. A wrapper can call `default_move`, then drop or re-price edges. Cells that are claimed, keep-out, hazardous or in cargo line-of-sight can be handled this way ([moves/mod.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea/src/pathfinder/moves/mod.rs)).
- `CustomPathfinderState` hands per-bot arbitrary data into `MovesCtx`. A shared claims map can be put behind an `Arc` in each bot's state. It's a function pointer, so closures are not allowed. A read lock on the state is held for the whole search, which can be seconds.
- `Goal` is a two-method trait (`heuristic`, `success`) with composable goals, so new goal shapes are easy.
- `astar::a_star` is a public, generic function over any node type. B1's long-range travel graph (roads, portals) can reuse it.
- Timeouts are settable per call, either by time or by node count (the node count is deterministic, which suits tests).
- Not pluggable: the base cost constants, and which block counts as passable (`is_block_state_passable` is a plain function).

**Sharing across bots: no.**
- Each search builds its own `CachedWorld`, a per-search cache of section bitsets, from the shared chunks ([pathfinder/world.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea/src/pathfinder/world.rs)). Nothing is shared between bots, no flow field, no path cache.
- Searches run on Bevy's `AsyncComputeTaskPool`, which by default gets **25% of cores, at most 4 threads** ([task_pool.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea-client/src/plugins/task_pool.rs)). 300 bots pathing at once on a laptop will queue behind 2–4 threads unless `TaskPoolOptions` is changed. That is configurable in the fork.
- The cached section bitsets are derived from immutable chunk data, so a process-wide section cache is a natural fork-level addition. A route-level cache across bots would be ours on top.

**What it can't do today (matters for B1 §4.1):**
- Moves: forward, ascend, descend, diagonal, dig-down, parkour (1–3 gaps), a few uncommon descents, and swimming/surface travel. No ladder or vine climbing, no door or gate opening, no bridging or pillaring, no portals, no boats.
- A closed door is solid to it, and with mining allowed (the default) it will **dig through** doors and player builds. Set `allow_mining(false)` outside jobs that call for digging. This is the pathfinder-side half of B4 §4.8's ownership check.
- Execution has two engines: the default, and `SimulationPathfinderExecutionPlugin` (steps its own physics forward for smoother movement).

**Recommendation: keep option (c).** Start with Azalea's, behind B1's interface. The spike question now has a partial answer: costs can be layered with the extension points above, and shared computation can't. The first fork-level change should be the compute-pool size and, once CPU is measured, a shared section cache. Missing moves (ladders, doors, bridging) are additions through `successors_fn`, since the move table is open. I did not benchmark the pathfinder; the repo has one criterion bench (a synthetic mining world).

### 5.2 AD-F2.2: extend Azalea's world storage or keep a separate store

**What Azalea stores.** `World` holds a `ChunkStorage`, a weak-pointer map from chunk position to `Arc<RwLock<Chunk>>`. Each bot's `PartialWorld` holds strong references to the chunks in its view; a chunk is dropped when the last bot lets go. A `Chunk` is sections (block states and biomes in paletted containers) and heightmaps ([chunk/mod.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea-world/src/chunk/mod.rs), [chunk/storage.rs](https://github.com/azalea-rs/azalea/blob/0bd22cbc/azalea-world/src/chunk/storage.rs)).

**What it drops.**
- Light data: the light packet is ignored and chunk light data is not read. F2 §4.7's light-level hazards cannot come from Azalea.
- Block entities: `ClientboundBlockEntityData` is ignored. Sign text, spawner and vault data are not stored.
- Server game time: `ClientboundSetTime` is ignored. Azalea has `ticks_connected` per client, not game time. F2 §4.1 stamps observations with `at_gt` (server game time), so F2 must record it itself.
- Per-cell or per-chunk timestamps, provenance, anything about who saw what.
- Container contents beyond the currently open menu; merchant offers; furnace and brewing progress (`ClientboundContainerSetData` is a TODO).

**What changed since the spec was written.** `ChunkStorageTrait` ([`88606d9c`](https://github.com/azalea-rs/azalea/commit/88606d9c), 2026-03-21) makes the storage pluggable: a custom type implementing `get`, `upsert`, `chunks`, `min_y`, `height` can replace the weak map without patching Azalea's world code. It is on the branch tip but not in the 0.15.1 release.

**Both sides, updated:**
- *For extending:* the trait makes a persistent or tiered chunk store a plug-in, not a patch; hot reads stay in one structure.
- *For separate:* none of the things F2 needs to attach (timestamps, provenance, containers, POIs, claims, light, block entities) has a home in `Chunk`, and adding per-cell metadata would spread through paletted-container code that upstream still optimises (several storage and palette optimisations landed in Feb–Mar 2026).

**Recommendation: keep the separate store (option b), with one addition.** Use `ChunkStorageTrait` for the one thing the spec's lean says needs persistence: chunk block data that outlives every bot's view. That is a small plug-in on the storage seam, not a change to Azalea's world code. Belief metadata stays outside. Feed F2 from `ReceiveGamePacketEvent` / `SendGamePacketEvent` observers (block entities, light, game time, container data) so no fork patch is needed for those. Cloned packets cost memory on the hot path (the chunk packet is cloned before it is read), which the spike should measure.

## 6. Per-spec findings

### F1 Swarm runtime
- **§4.1 Node model.** Matches Azalea. Two cautions. (1) A swarm is one ECS under one lock. Everything in a pass, including every bot's physics, packet handling and path execution, must finish in the 50 ms game-tick budget or Azalea logs "GameTick is more than 10 ticks behind" and skips ticks (client.rs). That limits bots per ECS by CPU, not memory. One way to scale is several swarm instances (each its own ECS) per host, at the price of not sharing world memory between them. (2) Azalea's own guide says ~20–40 MB for a lone bot, "usually up to 1 MB extra per bot" in a swarm "though it may be more if they're spread apart", and "swarms should be able to handle up to a few hundred bots" by default. Allen's ~1,000 per laptop is plausible with low view distance and clustered bots, but I found no published figure near it. The benchmark idea from the coordinator is the right check.
- **§4.2–4.3 Accounts, login.** Azalea gives auth and token cache, offline accounts, per-bot proxies, a fixed reconnect delay and sequential `join_delay`. Failure classification, backoff with jitter and the lease registry are ours. `Event::ConnectionFailed` exists on the branch.
- **§4.4 Lifecycle.** Events available: init, login, spawn, death, disconnect (with reason text), keep-alive, tick, chunk received. `auto_respawn` exists.
- **§4.5 Reflex host.** Ours. Azalea has no arbitration layer.
- **§4.10 Kicks.** The disconnect reason arrives as formatted text; classification is ours.
- **§4.11 Bandwidth.** No traffic counters. View distance can be set per bot at join and later. Chunk batch acknowledgements already slow the server's chunk sending when a node is slow to process them, which is a natural brake.

### F2 World model
Covered in §5.2. Add: entity identity is by UUID via `entity_by_uuid`, which fits §4.5. Entities are shared across the swarm; per-bot `updates_received` counters stop duplicate updates being applied twice.

### F5 Version layer
- **§4.1 Boundary.** See §8 for the ECS question.
- **§4.2 Generated data.** See §4. Also: Azalea's block states and registry enums are generated per version, so the adapter round-trip test (F5 §8) mostly checks Azalea's own representation against itself.
- **§4.7 Porting.** See §2.6.
- **AD-F5.1 (decided).** Confirmed feasible, §2.

### B1 Movement
- **§4.2 Vanilla-accurate execution.** Azalea's physics is a faithful port for walking, jumping, fluids and collision, but its own README and TODOs list: **entity pushing and sprint swimming not implemented**, **using-item slowdown hard-coded off** (`let using_item = false`), **passenger state hard-coded off**, no world border, no slow-falling gravity change, and the fall-damage and minor-collision TODOs. Also, `look_at` snaps the look direction instantly, with no smoothing. B1's "server corrections are bugs to fix" will find these.
- **§4.1, §4.3** See §5.1.
- **§4.4 Long range.** Ours, on top of the public `a_star`.
- **§4.6 Falls and swimming.** Pathfinder swims on the water surface (since January); no water-bucket landing, which is a reflex we write.
- **§4.8 Replay mode.** Raw sending works (`write_packet`, `SendGamePacketEvent`). But there is **no per-bot switch to stop Azalea's own movement output**: `send_position` and the physics set run for every local entity. Replay needs a small fork patch (a marker component and `Without<…>` filters on the movement, physics and pathfinder systems).

### B2 Survival upkeep
Azalea tracks `Health` and `Hunger` (food and saturation) and respawns. Everything else is ours: eating (a single right-click works server-side, but with the slowdown unimplemented the bot would move at full speed while "eating", which anti-cheats flag), sleeping, armour and tool swapping (`auto_tool` handles tool choice for breaking), repair, hazard escape, death recovery.

### B3 Inventory and crafting
Reuse the click modes and menus. Missing: recipe data (`ClientboundUpdateRecipes`, recipe-book packets are ignored, so crafting means manual slot clicks driven by F5 recipe data), merchant offers (ignored, so trading needs a packet watcher), enchantment offers and anvil/brewing/furnace progress (`ContainerSetData` TODO). The serverbound packets exist in `azalea-protocol` (place recipe, select trade, rename item, container button click, set beacon) and can be sent with `write_packet`, but the client layer has no helpers or state for them, and `fill_exact` verification.

### B4 Gathering
Mining is covered at the break level. Vein following, tree felling, farming, fishing (needs hook entity tracking; sound packets are discarded), breeding and the ownership check are ours.

### B5 Combat
Attack and cooldown exist (`attack.rs`; the cooldown calculation for tools was fixed in 0.15.1). Weapon behaviour, shields, totems, bows and crossbows, ballistics, mace and spear logic, reflexes and doctrine are ours. The attack packet differs between 1.21.11 and 26.x, one more reason to keep combat behind the adapter.

### B6 Placement and breaking
- Right-click uses the ray from the bot's eye along its look direction. `StartUseItemQueued::force_block` makes up a hit point (block centre, face `Up`), and the source notes that this "should be avoided to stay compatible with anticheats". For B6's recipes (specific face, hit point, yaw, pitch) the correct route is to set the look direction and let the raycast produce the hit, or to send `ServerboundUseItemOn` directly with the recipe's values.
- Reach checks (4.5), collision-box check, sneak and rotation interpolation are ours.
- Replay executor: ours, with the movement-output caveat under B1.

### B7 Mob handling
Entity interaction exists (`entity_interact`, usable for leads, name tags, breeding items). **Vehicles are missing**: passenger state is a TODO and `move_vehicle` is a stub, so rowing a boat with mobs (B7 §4.4) needs real work in physics and packets.

## 7. Corrections the specs need

1. **F1 §4.1, §6.** Replace "~1 MB per bot, ~1,000 per laptop" with Azalea's stated numbers and the single-ECS, single-lock limit; keep Allen's estimate as a target to confirm. Add the tick-skip behaviour as the overload signal for `NodeHealth.tick_lag`.
2. **F1 §4.11.** Azalea has no traffic counters; they are new work.
3. **F2 §4.1.** `at_gt` needs a game-time source (Azalea discards `ClientboundSetTime`). Light levels, block entities and container progress data are discarded too.
4. **F5 §4.7 and §3.** "Pin the Azalea release" becomes "carry a fork and merge upstream, minus its bump commits".
5. **F5 §4.2.** Shapes and block behaviour already come with Azalea.
6. **B1 §4.1.** No ladders, doors, gates, bridging, pillaring, portals or boats in the pathfinder. It digs through closed doors by default.
7. **B1 §4.2, B2.** Using-item slowdown and passenger physics are unimplemented; eating while moving will look wrong to a server.
8. **B1 §4.8, B6 §4.7.** Replay needs a small fork patch to switch off Azalea's own movement output per bot.
9. **B6 §4.8.** Azalea snaps rotations instantly; interpolation is ours.
10. **B3.** Recipes, trades and enchant/brew/furnace data are not exposed by Azalea.
11. **B7 §4.4.** Vehicle riding is not implemented.

## 8. For the F5 boundary question: how stable is the ECS surface?

The audit found that "only `mc-adapter` imports Azalea" conflicts with every skill being a Bevy plugin. What the evidence says:

**Churn, from the changelog and git history.**
- Four Bevy major versions in fourteen months (0.16 in April 2025, then 0.17, 0.18, 0.19). They renamed things that every plugin touches: the `Event` type became `Message`, and Azalea renamed its system sets to match Bevy's `…Systems` convention ([`43f20b82`](https://github.com/azalea-rs/azalea/commit/43f20b82)).
- Azalea's own breaking changes per release (the "Changed" sections, 0.13 → 0.16) run 9–16 entries each, and by a rough keyword count 3–9 per release are ECS or plugin level: `PhysicsState` → `ClientMovementState`, `LocalGameMode` split into `GameMode` and `PreviousGameMode`, `SendPacketEvent` → `SendGamePacketEvent`, `Inventory` moved crates, `Client` moved crates, plugin and event types turned into triggers, `Instance` → `World`, `ResourceLocation` → `Identifier`, `Block`/`Item` → `BlockKind`/`ItemKind`.
- Upstream states "almost every Minecraft version is breaking anyway" and does not follow semver ([CHANGELOG](https://github.com/azalea-rs/azalea/blob/0bd22cbc/CHANGELOG.md)).

**How other projects cope** (a sample of four, plus two official plugins; not a survey):
- I read only the manifests of four bots (ShaysBot, stasis-bot, potato-bot-2, ErrorNoWatcher), not their source layout, so I can't say whether any of them wraps Azalea. What the manifests show is a direct dependency that **pins a git revision or their own fork** (ShaysBot with a `[patch]` to a fork, stasis-bot on its own 1.21.4 backport branch, potato-bot-2 on a `1.19.2` branch).
- `azalea-hax` (anti-knockback, a tiny plugin) has 8 commits in total; after its first week it was touched three times in about two years three months, the latest (January 2026) to change how it intercepts knockback. `azalea-viaversion` has about 70 commits and has a Minecraft-version commit for each of 1.21.8, 1.21.9, 1.21.10 and 1.21.11, plus several "update Azalea" commits.

**What this means for the decision.**
- Whatever the rule says, skills that run as Bevy systems depend on **Bevy's** API, which no adapter crate hides. Bevy types (`Entity`, `Query`, `Commands`, messages) would appear in skill code in every variant except one where skills never run in the ECS.
- Azalea's own guide says logic in the `Client` handle (the async API) takes locks and negates most of the performance benefit; writing the bot as a plugin is the recommended path at scale. So "skills outside the ECS, talking to the adapter over `Client`" costs CPU at the scale the project targets.
- A frozen fork changes the picture: we choose when to take a Bevy or Azalea API change, so churn arrives on our schedule. The boundary's job is to limit the rewrite when we do take one.

**Options this supports** (Allen's decision):
1. **Keep the rule literally; skills use Null-owned components and events.** The adapter is a Bevy plugin that translates between Azalea's components and messages and Null's. Skills never name an Azalea type. Costs: a mirrored component set or wrapper `SystemParam`s (the latter are zero-copy) that the adapter has to maintain, and Bevy API exposure stays.
2. **Loosen the rule to "one adapter module re-exports a curated Azalea ECS surface".** Skills import aliases (`BotPosition`, `WalkIntent`, …) from the adapter. A rename is absorbed in one file. Semantic changes (such as the strafing fix) still leak into skills.
3. **Allow direct imports in skill crates, with a build check that lists which Azalea types each skill uses.** Cheapest now, most expensive at the next Bevy or version bump, and it matches what the other projects do.

My read of the evidence: option 2 gets most of the benefit of option 1 for much less upkeep, and it matches how the churn actually shows up (renames and moves). Option 1 is worth its cost only for the few interfaces where behaviour, not names, may change between versions: movement, placement and item use.

## 9. Suggested next steps (small, evidence-producing)

1. **Join test of the fork build** against a vanilla 1.21.11 server: login, chunk load, walk, mine, open a chest, place a block, with the left/right fix in place. This is the one thing I could not do here. *Done 2026-10-14 in the AD-F5.2 spike's live run (`SPIKE_F5_BOUNDARY.md` §6), except mining.*
2. **Bandwidth and CPU benchmark** (the coordinator's option 4), now with specifics to measure: GameTick duration vs bot count in one ECS; the compute-pool cap; bytes per bot at view distance 2 vs 8; the "few hundred bots" claim vs ~1,000.
3. **Fork patch list**, in this order: nightly pin and toolchain file; per-bot replay switch; byte counters; compute-pool options; game-time and light/block-entity capture plugin; using-item state.
4. **Data gap check for F5**: recipes, loot, support rules and mob stats still need a source; confirm what the vanilla data generator provides.

## Appendix A. Cherry-pick set that built and passed

On top of the `1.21.11` tip `41a9ae6a`, in upstream order (hashes are upstream's): `2d3e4194` `f09d07ce` `0d0e253f` `b9995f10` `ed12c2dd` `7b7b1222` `29beda74` `615df92e` `4af97628` `4d1f4304` `a6fbdea9` `9ffd0e80` `64b3c514` `c963b0a5` `86dc16c5` `5cefefee` `2e601b43` `b71a3f77` `0d392ddc` `e347748e` `0366d180` `0b8f6b5c` `8a32e091` `a630da47` `8b36ee17` `d1de2f4f` `7bf742db` `5064b92b` `a8959f62` `fe6ad400` `6249c295` `153c90aa` `b65fa8cf` `725d3f7f` `6b60d81d` `e88ecfa2` `40455fc1` `0bd22cbc`.

Applied afterwards, not compiled: `cabc8b60`. Not applied: `ddef3711` `eeb729eb` `ee7358eb` `5e0484c9` `9503c287` `8d09b4ee` `e73bb165` `d3422ec8`.

Mechanical resolutions I made: `CHANGELOG.md`, `Cargo.lock`, `README.md`, example programs and `codegen/*.py` taken from the cherry-picked side. The example `steal.rs` no longer compiles as a result (the other examples were not built); library code and all tests are unaffected. Tests run: library tests of `azalea-client`, `azalea-physics`, `azalea-world` and `azalea` (36), and the `azalea-client` simulation suite (23).

## Appendix B. Method and limits

- Source: a clone of [azalea-rs/azalea](https://github.com/azalea-rs/azalea) at `main` `0bd22cbc` and the `1.21.11` branch `41a9ae6a`. All claims about behaviour come from reading that source, the changelog and git history. `docs.rs` and `azalea.rs` were not reachable from this environment, so I did not read the rendered docs.
- Build and test results are from a 4-core container with debug-profile type-checks and tests. They say nothing about runtime performance, and nothing was run against a Minecraft server.
- Third-party project comparisons use only their public `Cargo.toml` and commit logs.
- Counts such as "245 of 262 commits" are `git shortlog` over `main` since 2025-10-01.
