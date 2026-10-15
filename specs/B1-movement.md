# B1 Movement

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.4, §3.7, §3.9, §3.12, §5, §8, §10; ONLINE_BUILDER.md §2b. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** F1, F2, F5. **Used by:** every skill and capability.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Local pathfinding | Deterministic | A* over a block graph with movement costs; well understood (Baritone) |
| 4.2 | Vanilla-accurate execution | Measured | Azalea ports vanilla physics; residual error is measured against the server |
| 4.3 | Path costs: hazards, claims, keep-out, cargo | Deterministic | Weighted costs from known layers and policy |
| 4.4 | Long-range travel | Deterministic | Graph search over roads, portals and the coarse map, per travel mode |
| 4.5 | Getting unstuck | Deterministic | A fixed escalation ladder ending in `blocked` |
| 4.6 | Falls, swimming, climbing | Deterministic | Physics plus known recipes (water bucket, ladders) |
| 4.7 | Multi-bot traffic | Measured | Cell reservations and soft costs; tuned under load |
| 4.8 | Replay mode (physics off) | Deterministic | A switch that hands the body to the builder's packet stream |
| 4.9 | Movement reflexes | Deterministic | Registered per the reflex contract, each switchable |

## Architecture decisions

### AD-B1.1 Reuse Azalea's pathfinder, or write our own? (decided)

**Decided by Allen, 2026-10-15: (a), reuse Azalea's pathfinder behind B1's interface, from the Azalea benchmark ([reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md)).** Keep the default search pool (25% of cores, at most 4 threads; 3 on the benchmark machine). Don't widen it: completed searches level off at about 22–28 per second whatever the pool size, and more threads slow the ECS tick, cut packet reads and drop bots at 100 bots. The first fork change is a process-wide section cache, invalidated on block updates: 99% of section building was repeated work, about a third of A* CPU. Watch A* that runs on the main thread (p99 13–22 ms against a 50 ms tick). B1's interface stays ours, so a later swap stays local. No ⚑. The options were (a) Azalea's pathfinder, (b) our own from the start, and (c) start with (a) and replace it when its limits show. Both sides are kept in [DECISIONS.md](DECISIONS.md#ad-b11-reuse-azaleas-pathfinder-or-write-our-own).

---

## 1. Purpose

B1 moves one bot's body from where it is to where a job needs it, safely and in a way the server accepts as vanilla movement: local paths, long trips across dimensions, recovering from being stuck, and handing the body over entirely when the builder replays a certified packet stream.

## 2. Scope

**Owns:** local pathfinding; movement execution; path costs; long-range travel (roads, nether travel, boats, elytra later); getting unstuck; safe falls; swimming; climbing; local traffic among bots; replay mode; movement reflexes.

**Does not own:** convoy formation (C9, K6); moving claims (K5/C9); route choice for convoys at operation level (C9); placement during pillaring or bridging outside replay (B6 provides the click, B1 the posture).

## 3. Interfaces

**Skill calls (jobs or other skills):**
- `goto(target, tolerance, posture?, constraints)` → arrives, or `failed(no_path | stuck | hazard)`, `blocked(claim | keep_out)`.
- `travel(destination, dimension, modes_allowed)` for long trips.
- `follow(entity_or_bot, distance_band)` for escorts and keepers.
- `hold(position, band)` for presence.
- `enter_replay()` / `exit_replay()` for the builder.

**Constraints passed in:** avoid regions (keep-out, other claims), stay within a band of a point, stay out of line of sight of cargo, max fall height, allow block breaking or placing (and which blocks).

## 4. Requirements

### 4.1 Local pathfinding

- A* (or a variant) over standable cells with moves: walk, sprint, jump up 1, step up ≤ 0.6, drop ≤ safe height, swim, climb ladders and vines, open doors and gates, and optionally break or place blocks when the job allows.
- Paths are re-planned incrementally when the world changes in front of the bot.
- Chunks not loaded on the client are treated as unknown, with a cost penalty, never as passable.
- Azalea's pathfinder has walk, ascend, descend, diagonal, dig-down, parkour and surface swimming, but no ladders or vines, doors or gates, bridging or pillaring, portals or boats; those are added as move functions through `successors_fn`. It treats a closed door as solid and, with mining allowed (its default), digs through doors and player builds, so B1 sets `allow_mining(false)` except in jobs that call for digging (Azalea reality check, 2026-10-13).

### 4.2 Vanilla-accurate execution

- Movement goes through Azalea's physics, which ports vanilla. Inputs per tick are ones a vanilla client can produce: no speed beyond sprint-jump, no position jumps, rotations changed at rates a client produces.
- Server corrections are counted per skill and movement type (F4); each one is a bug to fix.
- Constants (step heights, jump timing, ice and soul sand speeds) come from F5.
- Known gaps in Azalea's physics: entity pushing and sprint swimming aren't implemented, using-item slowdown and passenger state are hard-coded off, and there's no world border or slow-falling gravity. Each shows up as server corrections; using-item slowdown is the first fork patch, because eating while moving otherwise looks wrong to a server (Azalea reality check, 2026-10-13).

### 4.3 Path costs: hazards, claims, keep-out, cargo

Costs add: hazards from F2 (lava, fire, falls, powder snow, magma), unknown chunks, light level (dark areas cost more at night), cells claimed by other jobs (prohibitive unless the job holds the claim), `keep_out` annotations and interrupt quarantines (forbidden, INTERRUPTS §2.2), line of sight to cargo when the job carries the `avoid_cargo_los` constraint, and enemy sightings (a policy weight). Weights are policy values, set per operation.

After an interrupt, `goto(last_safe)` takes the bot back along the path it came by, skipping cells that changed, to its last safe position (INTERRUPTS §2.2).

### 4.4 Long-range travel

- Modes: walking, roads, boats (and ice roads), nether travel through known portals (overworld distance / 8), minecart lines where they exist, elytra later.
- Route search over a travel graph built from the coarse map and known portals, roads and rails. Unknown terrain is planned in segments.
- Portal use: check the link, cross, confirm the exit position matches the expected linked portal. A mismatch returns `failed(portal_link_mismatch)` to the caller.

### 4.5 Getting unstuck

Ladder, each rung with a game-tick budget:

1. Re-path from the current position.
2. Back up and re-path with the blocking cells marked.
3. If the job allows: break the obstruction (B6) or pillar out with a temp block.
4. Ask the squad leader (`stuck` event with position and surroundings).
5. Return `blocked(stuck, at)`.

### 4.6 Falls, swimming, climbing

- Planned drops never exceed the safe fall height for the bot's current health and armour (feather falling included).
- Unplanned falls are handled by the `move.mlg_bucket` reflex when it's on. It sends `UseItem` (not a block use), and tests reach from the eyes (1.62 above the feet) against the 4.5 interaction range; a click from 3.2 blocks feet-to-floor was ignored live (spike §6.3).
- Long falls can skip the reach window: near terminal speed (about 3.9 blocks per tick) the window is about 2.9 blocks, so roughly a quarter of high falls have no tick inside reach (arithmetic, not measured). The edge guard and planned drops are the main defence; the bucket is a backstop.
- Swimming: surface for air before the air bar runs low (coordinated with B2's `upkeep.surface_air`).

### 4.7 Multi-bot traffic

- Bots in the same area reserve their next few cells softly through the shared world in the node; others path around them.
- Choke points (doorways, one-wide tunnels, portals) get a short queue managed by the squad leader.
- Bots never push each other off ledges: entity collision is part of the hazard model.

### 4.8 Replay mode (physics off)

- `enter_replay()` stops B1's own physics prediction, pathing and all movement reflexes, and hands the bot's movement packets to the builder's open-loop replay (ONLINE_BUILDER §0 S3, ARCHITECTURE §3.9).
- During replay B1 only observes: it reports server position corrections immediately so the replay halts.
- `exit_replay()` resyncs B1's state from the server's last position.
- The job that enters replay mode also declares `reflexes_off: all` (CONVENTIONS §4.4).
- Azalea has no per-bot switch for its own movement output: `send_position` and the physics systems run for every local bot. Replay needs a small fork patch, a marker component with `Without<…>` filters on Azalea's movement, physics and pathfinder systems (Azalea reality check, 2026-10-13).

### 4.9 Movement reflexes

| id | trigger | action | default | turn it off when | if off |
|---|---|---|---|---|---|
| `move.repath` | path blocked by a new obstacle | re-plan locally | on | a squad formation step needs bots to wait in place | bot stops and emits `ReflexSuppressed`; squad decides |
| `move.mlg_bucket` | falling beyond safe height with a water bucket | place water on landing | on | near redstone or cargo cells where water would do damage; builder sites | bot takes fall damage; event emitted |
| `move.edge_guard` | about to step off an unplanned drop beyond safe height | stop or sneak | on | parkour or deliberate drops the skill plans itself | bot may fall; the skill must plan drops |
| `move.unstuck_jump` | no progress for N gt while walking | jump and re-path | on | inside the builder's work envelope, or near pressure plates | bot stays put; `stuck` event |

All four are switchable at every scope (CONVENTIONS §4.2), and all four are off during replay.

## 5. Failure handling

| failure | response |
|---|---|
| No path | `failed(no_path)` with the frontier reached |
| Stuck | Ladder in 4.5 |
| Portal goes somewhere unexpected | `failed(portal_link_mismatch)`; the caller decides (C9 treats it as a possible hijack) |
| Repeated server corrections | Stop the move, report to F4, return `failed(server_rejected_movement)` |

## 6. Scale and performance

Pathfinding runs per bot on the node; budgets per tick across the node so every bot on it pathing at once doesn't stall a tick, sized for the planned node size (F1 §4.1), not a fixed 300 (spec audit #10, 2026-10-13). Long-range route search is a capability service shared by the node, cached per route.

## 7. Version-sensitive parts

Physics constants, block collision shapes, new movement items (spears' lunge, wind charge jumps, happy ghasts) via F5 feature queries.

## 8. Tests

- Cave with lava and a creeper: arrives without damage or reports `hazard`.
- 100 bots crossing one doorway: no deadlock, throughput measured.
- Replay mode: a certified packet stream replayed with zero B1 interference; a forced server correction halts the replay within one tick.
- Reflex switches: `move.mlg_bucket` off at squad scope; a fall from 20 blocks produces `ReflexSuppressed` and no water placement.
- Anti-cheat profile: an hour of mixed movement on a harness server with the target's anti-cheat plugin; setbacks per hour measured.

## 9. Open questions

- Elytra travel is "later" in the architecture. It needs firework logistics and is more likely to trip movement checks; scope it when the End is reached.
