# F2 World model

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.1, §3.7, §3.10, §3.12, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** F1 (perception events), F5 (block and entity types). **Used by:** every skill, K5 (shards), K1 (coarse map), C3, C5, C8, C9, C10.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Observation events | Deterministic | A fixed schema stamped with who, when and how |
| 4.2 | Merging beliefs | Deterministic | Newest observation of a cell wins, with explicit rules for conflicts |
| 4.3 | Age and staleness queries | Deterministic | Every answer carries its age; limits are per query type |
| 4.4 | Containers | Deterministic | Contents are known only when opened; that rule is complete |
| 4.5 | Entity tracking | Deterministic | UUID-keyed with last-seen; cargo flag from C9 |
| 4.6 | Points of interest and structures | Deterministic | Structures are recognised from known block palettes and generation rules |
| 4.7 | Hazards | Deterministic | Lava, falls, light levels and mob spawnability follow game data |
| 4.8 | Player sightings | Deterministic | Stored facts only; what a sighting *means* is C8's job |
| 4.9 | Change detection | Deterministic | Diffs between observations; attributing intent is C8 |
| 4.10 | Shard storage, event log, coarse map | Deterministic | SQLite shards plus append-only logs; Postgres coarse map |
| 4.11 | Retention and compaction | Measured | The rules are fixed; how much to keep is tuned by measurement |

## Architecture decisions

### AD-F2.1 Shard belief by operation, or by fixed map tiles? (decided)

**Decided by Allen, 2026-10-15: fixed tiles (b).** Tiles (for example 512 × 512 blocks per dimension) are the unit of storage. An operation is the current writer for the tiles it covers, and a background store keeps tiles that no operation covers. The linked question is settled with it, as the lean said: each operation's service runs inside the node process, with operations sized to fit one node, so belief sits next to the chunk data and only diffs and summaries leave the node. No ⚑. The options were (a) one shard per operation (v0.7), (b) fixed tiles, and (c) one global store per dimension with regional caches. Both sides are kept in [DECISIONS.md](DECISIONS.md#ad-f21-shard-belief-by-operation-or-by-fixed-map-tiles).

### AD-F2.2 Extend Azalea's world storage, or keep a separate belief store? (decided)

**Decided by Allen, 2026-10-15: (b), a separate belief store fed by a packet observer, from the Azalea benchmark ([reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md) §5).** The observer takes the `chan` form: the ECS system only clones the packet's `Arc` and sends it to a worker thread, which deduplicates and stores. Its cost on the ECS thread was 1.2 ms per second at 100 bots, with no change to the tick. Deduplication is required: 96% of chunk packets repeat a chunk another bot already sent. Kept from the recommendation: a small `ChunkStorageTrait` plug-in so chunk block data outlives every bot's view, and hot-path reads through the adapter to Azalea's loaded chunks. No ⚑. The options were (a) timestamps and provenance inside Azalea's world storage, and (b) a separate store. Both sides are kept in [DECISIONS.md](DECISIONS.md#ad-f22-extend-azaleas-world-storage-or-keep-a-separate-store).

---

## 1. Purpose

No bot sees the world; each sees a few chunks through a laggy connection, and what it saw goes stale as soon as it leaves (ARCHITECTURE §1.1). F2 turns those partial, timestamped observations into a queryable **belief** that always answers "what we last knew, and how old it is", never "what is".

## 2. Scope

**Owns:** the observation schema; belief merging; age-aware queries; container contents as last opened; entity tracking keyed by UUID; points of interest and recognised structures; hazard layers; player sightings as raw facts; change detection; storage of region shards, their event logs, and HQ's coarse map; retention.

**Does not own:** Azalea's own chunk cache (F1/F5 read it); claims (K5 registry); the item and entity ledgers (C3, C9), though it supplies their observations; threat assessment (C8); site scoring (C10).

## 3. Interfaces

**In:** `Observation` events from bots (via F1 perception, merged by K6 before going up).

**Out (queries, all answered with ages):**
- `block(pos)`, `region_blocks(box)`, `find_blocks(type, near, radius)`
- `container(pos)` → contents as last seen, by whom, when
- `entity(uuid)`, `entities(type, region)`
- `poi(kind, near)`, `structures(kind, region)`
- `hazard(pos)`, `hazards(region, kinds)`
- `sightings(player_handle | region, since)`
- `changes(region, since)` → cells that differ from the previous observation

**Out (summaries to HQ's coarse map):** per-region stock hints, danger, structures, territory markers, last-visited time.

## 4. Requirements

### 4.1 Observation events

```
Observation {
  observer: bot_id, node, at_gt (server game time), at_wall,
  how: seen_chunk_data | block_update | opened_container | entity_spawn | entity_move | interaction | inferred,
  dimension, payload: block states | container slots | entity snapshot | sighting
}
```

- Observations stay on the node and in the owning operation's shard. Nothing per-block crosses to HQ (§3.7).
- Bots in one node share Azalea's world storage; F2 adds the timestamps and provenance Azalea doesn't keep.
- Azalea discards server game time (`ClientboundSetTime`), light data, block entities and container progress data. F2 records them itself from packet observers (`ReceiveGamePacketEvent`), including the game-time clock behind `at_gt` (Azalea reality check, 2026-10-13).

### 4.2 Merging beliefs

- Per cell, the observation with the newest `at_gt` wins. Server game time orders observations across bots in the same dimension; wall time breaks ties only when game time is equal.
- A client learns game time from a time packet about once a second and estimates in between, so `at_gt` isn't tick-accurate across bots. Within a node, observations are ordered by receive sequence. Across nodes, two observations of the same cell less than about a second apart that disagree are a conflict: the cell is marked for re-observation instead of picking a winner (spec audit #9, 2026-10-13).
- `inferred` observations (for example "this cell is air because the bot mined it") are replaced by any direct observation.
- Observations from two nodes about the same cell merge by the same rule in the shard; nodes don't merge with each other directly.

### 4.3 Age and staleness queries

- Every query result carries `observed_at` and `observer` per value, or the oldest among aggregated values.
- Callers pass a `max_age`; results older than it are returned marked `stale`, never dropped silently.
- Default staleness limits per data kind are configuration (containers short, terrain long, entities very short).

### 4.4 Containers

- Contents are known only when a bot opens the container. The world model stores contents per open, with the opener and time.
- On an SMP, contents are always assumed possibly changed since last opened. C3's reservations against a container are soft until the container is reopened.
- Opening a container is an action with side effects (barrel `open` state, trapped chest power), which B3 and C1 account for.

### 4.5 Entity tracking

- Keyed by entity UUID, which the server assigns and every node sees identically (§3.10).
- Per entity: type, last position, last seen time and observer, vehicle, name (quarantined if player-set), persistence hint, and the `cargo` tag owned by C9.
- Entities not seen for longer than their kind's limit are marked `unknown`, not deleted.

### 4.6 Points of interest and structures

- POIs: beds, workstations, portals, spawners, trial spawners, vaults, chests, end portal frames, nether fortress and bastion blocks, villages.
- Structures are recognised from signature blocks and generation rules in the per-version data (F5), for example trial spawners plus tuff and copper for a trial chamber.
- Every POI carries who found it, when, and whether a bot has verified it in person.

### 4.7 Hazards

Layers per region: lava and fire, fall heights, water depth, light level (for mob spawning), powder snow, magma, cactus and berry bushes, known traps, operator `keep_out` annotations (from K1), and interrupt quarantines: marks over the cells an interrupt involved plus a margin, each with its evidence id and an expiry, renewed if it triggers again; a mark caused by a player follows that player's last-seen positions until it expires (INTERRUPTS §2.2). Spawnability uses the light and block rules for the version.

### 4.8 Player sightings

- A sighting is a fact: player handle, position, dimension, time, observer, visible gear, and whether the player was moving towards something.
- Player names are shown as stable handles (`P-17 "Steve"`), per the quarantine rule (CONVENTIONS §3.7).
- Relation (ally, neutral, enemy) and intent are C8's data, joined at query time.

### 4.9 Change detection

- When a bot re-observes a region, F2 computes changed cells against the previous belief.
- Changes are found by comparing decoded block states, never chunk bytes. In a world with no changes, about 3% of repeated chunk packets arrived with different bytes ([reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md) §5).
- Changes inside a region where our own jobs ran are matched against those jobs' expected changes. Unmatched changes are emitted as `unexplained_change(region, cells, window)`.
- F2 does not decide whether an unexplained change was griefing, a creeper or another player's legitimate work. It reports.

### 4.10 Shard storage, event log, coarse map

- Each operation's shard: SQLite, local to the operation service, with an append-only event log shipped to a VPS (§9). Raw chunk data can't cross from node to a separate operation-service process within the bus budget, so the operation service runs inside the node process (AD-F2.1, decided 2026-10-15), and its shard holds the fixed tiles it currently writes. Either way, only diffs and summaries leave the node (spec audit #3, 2026-10-13).
- A restarted operation rebuilds its shard from the log (§3.7).
- HQ's coarse map: Postgres on a VPS, holding region summaries, owners, territory, threats, structures, annotations.
- Cross-shard queries go through HQ's coarse map for summaries, or to the owning shard for detail.

### 4.11 Retention and compaction

- Terrain: keep the latest state per cell plus a change history bounded by count and age.
- Containers: keep the last N opens.
- Entities and sightings: keep a time series downsampled with age.
- Bounds are tuned by measuring storage and query latency at 2,000 bots.

## 5. Failure handling

| failure | response |
|---|---|
| Shard process crash | Rebuild from the event log on restart |
| Log shipping lag | The operation keeps running; recovery may lose the unshipped tail, which is re-observed |
| Two bots disagree within one tick | Newest game time wins; equal times keep both and mark the cell `conflict` until re-observed |
| Server restart | Every belief stays, with its age; nothing is assumed reset except what the restart necessarily changes (entities not persistent are marked `unknown`) |

## 6. Scale and performance

- Observation write rate at 2,000 bots is dominated by block updates during mining and building. Writes stay on the node and shard; only summaries travel.
- Query latency for skills (local cell lookups) must be well under a tick; shard queries for planners can take milliseconds.

## 7. Version-sensitive parts

Block states, shapes, structure signatures, light and spawn rules, entity types: all from F5's per-version data.

## 8. Tests

- Two bots observe a chest at different times; query returns the newer contents with both ages available.
- A harness player edits a region between visits; `unexplained_change` is emitted with the right cells.
- Operation kill and restart: shard rebuilt from the log matches the pre-crash shard.
- Trial chamber recognition on a fixed-seed world finds every chamber the harness knows of within the explored area.

## 9. Open questions

- Do we need per-cell observation history for anything beyond change detection? If not, keep only latest plus diffs.
