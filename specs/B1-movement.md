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
- Block movement effects (fork patch 26): the bot's own physics applies what a vanilla client applies for the blocks it is in or on: the stuck multiplier of a cobweb, sweet berry bush and powdered snow (set at the end of a tick, taken by the next move), the speed factor of soul sand and honey, the server's powdered-snow frost (an attribute update), and the climb out of powdered snow only with leather boots. Damage and freezing are the server's. Without them the server moves the bot slower than it predicts and Grim flags `Simulation`.
- Server-set attributes (fork patch 27): the bot takes every attribute the server sends for it (`handleUpdateAttributes`: the base value and all modifiers are the server's) and its physics reads them: movement speed, sneaking speed, jump strength, gravity, step height, movement efficiency, water movement efficiency, block and entity interaction range, block break speed, mining efficiency, submerged mining speed. Values are clamped to the attribute's range. Inputs the client computes with but Null does not model (attribute `scale`; effects levitation, slow falling, dolphin's grace) are listed by `BotView::unmodelled_inputs` with a kind (`Movement` or `Action`); while one is in force the host ends the running skill and sends `Stop`. Blindness stops and forbids sprinting. A respawn keeps what vanilla keeps (built, patch 30): the base values always, and the modifiers too when the respawn packet's keep bit 1 is set (`ClientPacketListener#handleRespawn`); effects start empty and the server resends them.
- Tier 1 of the safe envelope ([AD-F5.5](DECISIONS.md#ad-f55-how-much-of-the-vanilla-client-does-the-fork-model); fork patch 30 built: Levitation, Slow Falling, the entry kinds and the disconnect; fork patch 31 built: interpolation of other entities and the bot-to-bot view (F5 §4.6, A18); fork patch 32 built: pushes, teams and the entity tick order (A19); fork patch 34 built: refusals (A20); fork patch 36 built: pathfinder walls (A21); rules and Mojang names in `specs/plans/PATCH_30_TIER1.md` §2):
  - Levitation (`LivingEntity#travelInAir`): in air travel the vertical speed moves 20% of the way toward 0.05 × (amplifier + 1) each tick, then the air drag applies; no effect in water or lava. The client keeps an effect until the server removes it. Built in patch 30 (A17 `levitation_i_rises`, `levitation_ii_rises`).
  - Slow Falling (`LivingEntity#getEffectiveGravity`): while the vertical speed is not upward, gravity is at most 0.01, in air, water and lava travel. Built in patch 30 (A17 `slow_falling_fall_20_blocks`, `slow_falling_arrow`). In air travel the speed is read after the move step, as `travelInAir` does; in fluid travel at the start of the tick.
  - While Levitation or Slow Falling is in force, the physics is right but skills plan with normal gravity, so each is an `Action` entry: the skill stops and the bot stays connected (Allen, 2026-10-20; built in patch 30, A16).
  - Pushes from entities (`LivingEntity#pushEntities`, `Entity#push`; built in fork patch 32, `azalea-client/src/plugins/pushes.rs`, `azalea-entity/src/push.rs`): the client pushes the bot from the other entities it ticks, never from the bot's own call. Every living entity and remote player pushes with its own box (strict intersection); a dying mob and a ridden horse still push. Exceptions: an armor stand, a bat, a parrot (players), and a pusher with `noPhysics` (a spectator remote player, a Vex, the Ender Dragon). A boat pushes with its box inflated 0.2 sideways and -0.01 vertically, by distance, and only when the bot's box bottom is at or below the boat's (`AbstractBoat#push`). Minecarts, items and projectiles never push on the client. The bot is not pushed while a spectator, dead, on a climbable, a rider, or (by a living pusher, not a boat) asleep. The amount is `Entity#push(Entity)` (offset under 0.01 does nothing; else scaled by min(1, 1/sqrt(f)) and 0.05), added to the velocity away from the pusher. It uses each pusher's interpolated position in that tick (F5 §4.6, patch 31; another bot is read from the bot's own `PeerView`), the client's entity tick order (kept per bot as sequence numbers in `EntityIdIndex`: the bot is first at login, last of the numbers so far at a respawn in the same dimension, first again after a dimension change; an entity removed and added again goes to the end), and the team collision rules (each bot keeps its own `Scoreboard` from `ClientboundSetPlayerTeam`; players are keyed by name, other entities by UUID text). Two passes of one function: before the physics chain for pushers that tick before the bot (this tick's travel), after the position packet and the interpolation for those that tick after it (the next tick's travel). `PushRecord` counts the pushes for the ability check. Skipped on purpose: the pusher's own mirror velocity, cramming (server only), the frozen-entity state (mobs and boats keep pushing while the server freezes ticks), the chunk-section ticking state, and passengers on the bot. Grim does not compute the push (`MovementTicker#handleEntityCollisions` only widens the tolerance of `Simulation`), so the exact check is the model in A19.
  - A `Movement` entry disconnects the bot at once, before the next tick computes a position (F5 §4.6; built in patch 30, A17 `disconnect_on_scale`, `disconnect_on_ride`, `disconnect_in_bubble_column`). The entries: scale away from 1, Dolphin's Grace, a vehicle, the server setting the glide flag on the bot (`Gliding`, patch 34), and the bot's box touching a bubble column, slime block, bed, scaffolding, moving piston, honey from the side, or the world border.
- Fall in an unloaded chunk (fork patch 38, A23): when the client has no chunk at the block below the feet, `LivingEntity#travelInAir` sets the vertical speed to a fixed -0.1 above the world bottom and 0 at or below it, instead of gravity (`Level#hasChunkAt`). The bot does the same (`azalea-physics` `unloaded_chunk_vertical_speed`). Grim does not predict movement while the player stands in an unloaded chunk (`SetbackTeleportUtil#insideUnloadedChunk`), so a bot that fell at normal gravity drew `Simulation` alerts once the chunk came in after an RCON teleport.
- Inputs only the bot can start are refused on the client (fork patch 34, A20, F5 §4.6): an air jump with a glider worn, a Riptide-class trident, a use with other `use_effects`, and any right-click that would mount.
- Known gaps in Azalea's physics (entity pushing was built in patch 32; Slow Falling and the world border as a disconnect trigger are built in patch 30): sprint swimming isn't implemented, using-item slowdown is hard-coded off, a vehicle and the world border disconnect the bot (`Movement` entries) instead of being modelled. Each shows up as server corrections; using-item slowdown is the first fork patch, because eating while moving otherwise looks wrong to a server (Azalea reality check, 2026-10-13).

### 4.3 Path costs: hazards, claims, keep-out, cargo

Costs add: hazards from F2 (lava, fire, falls, powder snow, magma), unknown chunks, light level (dark areas cost more at night), cells claimed by other jobs (prohibitive unless the job holds the claim), `keep_out` annotations and interrupt quarantines (forbidden, INTERRUPTS §2.2), line of sight to cargo when the job carries the `avoid_cargo_los` constraint, and enemy sightings (a policy weight). Weights are policy values, set per operation.

A cobweb is not passable for the planner and is never mined (its mining cost is infinite), and a diagonal move does not cut a corner next to a cobweb, sweet berry bush or powdered snow: they do not stop the bot, so its box would end up inside them. Soul sand and honey are not standable for the planner (their top is lower than a full block), so no path crosses them; if the bot is on one anyway the executor needs no change (it checks the cell, not the time).

Blocks the fork does not model are walls to the planner until a later tier models them (AD-F5.5; built in fork patch 36, scenario A21, `azalea/src/pathfinder/world.rs`): one list with the patch 30 gate (`is_unmodelled_block`), so the two cannot drift. Per-state walls: bubble columns and moving pistons are not passable; slime blocks, beds and scaffolding are not passable, solid or standable. Neighbour walls, read at query time through a bitset of source blocks: the four side cells of a honey block or of scaffolding at body height, and the cell in front of any piston base's face (sticky or not, extended or not); a walled cell is never mined. The world border: `CachedWorld` takes a `BorderWall` snapshot of the bot's `WorldBorderState` when the path is planned (the size it moves to, if it shrinks), and cells within 3 blocks of the border (vanilla's `WorldBorder#isInsideCloseToBorder` distance of 2 plus 1) are walls; with no border state there is no border wall. Moves: a diagonal never passes over the corner of a cell whose floor is slime, a bed or scaffolding, and parkour never jumps over one, because the bot's box is within a millimetre of that block's top face at take-off and landing and the gate counts that as touching it (`SlimeBlock#stepOn`). Scaffolding gets side walls because the fork collides with a shape that vanilla's side has not (`ScaffoldingBlock#getCollisionShape`), and a bot hugging its corner drew Grim `Simulation`. The walls lower the chance that the bot ends up in one; touching one anyway is a `Movement` entry and disconnects it (F5 §4.6).

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
