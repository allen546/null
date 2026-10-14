# Online builder: implementation plan

This plan implements the design in `planner/ONLINE_BUILDER.md` (v4.1). The design is the authority. This plan says how to build it, in which order, and how to tell each piece is done.

It is written for an implementer with no spatial sense. Every spatial fact you need is either given in Appendix B, produced by the simulator, or checked by a property test that compares two computations. You are never asked to work out a direction, face, coordinate or look angle in your head.

## 0. Read this first

### 0.1 Hard rules (never break these)

1. **Live server: never touch it.** Never connect to, start, stop or edit `mc_server` (the live server, port 25565). Running anything live is only ever done by Allen.
2. **Allen's sandbox: never touch it.** Never connect to, start, stop, deploy to or edit `mc_server3` (ports 25567, 25577, 25582).
3. **Use only these simulators:**
   - `mc_server2`: plugin 25581, game 25566, RCON 25576, password `antigravity`;
   - `mc_server4`: plugin 25583, game 25568, RCON 25578, password `antigravity`.
4. **Never weaken a test to make it pass.** Never delete a failing test, mark it skipped, loosen its tolerance or change its expected value.
5. **Never hand-derive a spatial fact** in code comments, tests or expected values: which face, which direction, which cell is "in front", stand coordinates, look angles. Spatial facts come only from:
   - Appendix B;
   - the existing geometry code named in this plan;
   - the simulator.

   If a task seems to need one from anywhere else, escalate (0.2, E2).
6. **Never hide a failure.** No silent fallbacks, no `try/catch` that swallows an error, no "retry until it works". A failure must surface as an error, a failed test or a stuck report.
7. **Don't change the design or this plan's interfaces.** Don't edit `planner/ONLINE_BUILDER.md`. Don't change any JSON format, function signature or op name defined here without escalating.
8. **Don't modify the old code.** Leave `planner/` and `builder/` alone, except where a task explicitly says otherwise. Copy files into `online/` instead.
9. **Python:** run it with `uv run`. Never use `pip`.
10. **Markdown:** don't hard-wrap lines. One line per paragraph or list item.
11. **Dependencies:** add none beyond those named in a task, without escalating.

### 0.2 Escalation

When a trigger below fires:

1. Stop the task.
2. Append an entry to `online/ESCALATIONS.md`:
   - task id;
   - what you tried (at most 3 bullets);
   - the exact command and its output (trimmed to the relevant 40 lines);
   - one plain question for Allen.
3. Tell Allen in chat, in plain words, then wait.

Don't start tasks that depend on the blocked one. Independent tasks are listed in §2 and are fine to continue with.

| id | trigger |
|---|---|
| E1 | The same test still fails after 2 honest fix attempts. |
| E2 | You need a spatial fact that isn't in Appendix B, the named code or the simulator. |
| E3 | The simulator behaves differently from what the plan says: an op returns something unexpected, or vanilla does something the plan didn't predict. |
| E4 | You need to change an interface, JSON format or op defined in this plan. |
| E5 | Java compile errors about `net.minecraft` classes, methods or fields that don't exist, after one attempt to find the right name. |
| E6 | A test takes more than 10 minutes, or a number in the plan (budget, cap, tolerance) looks wrong in practice. |
| E7 | Anything that touches the guarantee: a §2 classification case the lists don't cover, or a damage case you're unsure how to classify. |
| E8 | You feel the urge to add a fallback, special case or retry to get a test green. |
| E9 | A task's "done when" can't be met as written. |

### 0.3 Working routine

1. Work on branch `online-builder`. Commit after each task that meets its "done when", with message `online: <task id> <title>`.
2. Keep `online/PROGRESS.md`: one line per task with its status (todo / doing / done / escalated) and the commit hash.
3. Run the whole test suite (`cd online && npm test`) before every commit.
4. Tests that need a simulator are named `*.sim.test.js` and run with `npm run test:sim`. They use `mc_server2` unless a task says otherwise.
5. **Restarting a simulator after a plugin change:** `./plansim-plugin/build.sh`, then in each of `mc_server2` and `mc_server4` run `./stop.sh && ./start.sh`.

## 1. Layout and formats

### 1.1 Layout

```
online/
  IMPLEMENTATION.md   this plan
  PROGRESS.md         task status
  ESCALATIONS.md      escalation log
  package.json        node >= 20, "test": "node --test test/*.test.js", "test:sim": "node --test test/*.sim.test.js"
  lib/                library modules (one per task below)
  tools/              command-line entry points
  test/               tests
  fixtures/           target JSON files and site files
```

### 1.2 Target file (`*.target.json`)

Coordinates are plan coordinates. Air cells are omitted.

```json
{"name": "f1_floor", "size": [5, 1, 5],
 "blocks": [[0, 0, 0, "minecraft:stone"], ...],
 "tiles": [[x, y, z, "<snbt>"], ...]}
```

### 1.3 Site file (`*.site.json`)

```json
{"origin": [5000, -60, 0], "envelope": 3, "margin": 4,
 "entry": [[-5, 0, 0], [-5, 0, 1], [-5, 0, 2]],
 "sims": [{"host": "127.0.0.1", "port": 25581}, {"host": "127.0.0.1", "port": 25583}],
 "gamerules": {"randomTickSpeed": 0}, "selfContained": true}
```

- **World = plan + origin.** The simulators are superflat: grass at world y = -61. So origin y = -60 puts plan y = 0 on the first air layer above the grass.
- **Envelope and margin.** The work envelope is the target's bounding box grown by `envelope` cells on every side, except downward (it never goes below plan y = 0). The margin is the envelope grown by `margin` more cells, and includes the grass layer below.
- **Entry** cells are feet cells for the bot, standing on grass, inside the margin.
- **`selfContained`** (required, boolean): true when the build works alone, false when it connects to other builds (a partial schematic). Only a self-contained build gets the isolation variant (H1 variant 7). All fixtures are self-contained.
- **Fixture origins.** Each fixture gets its own origin x (5000, 5100, 5200, …) so fixtures never overlap.

### 1.4 Step and packet stream (`plan.json`)

A plan is a list of steps. A step is a list of ticks. A tick is a list of packets, sent together in that client tick.

```json
{"target": "f1_floor.target.json", "site": "f1_floor.site.json",
 "start": {"hash": "<sha256>", "bot": {"pos": [x, y, z], "yaw": 0, "pitch": 0}},
 "steps": [
   {"id": 17, "kind": "place", "cell": [2, 0, 3], "state": "minecraft:stone",
    "ticks": [[{"t": "input", "sneak": true}], [{"t": "move", "pos": [..], "yaw": 0, "pitch": 30, "onGround": true}], ...],
    "settle": 23, "hash": "<sha256 after the step is inert>", "changes": [[tick, [x, y, z], "state"], ...]}
 ]}
```

Packet types (world coordinates inside `ticks`):

| `t` | fields | vanilla packet |
|---|---|---|
| `move` | `pos`, `yaw`, `pitch`, `onGround` | `ServerboundMovePlayerPacket.PosRot` |
| `input` | `sneak`, `jump`, `forward` (booleans) | `ServerboundPlayerInputPacket` |
| `slot` | `item` (for example `minecraft:dirt`) | creative set-slot into hotbar slot 0, plus select slot 0 |
| `use_on` | `hit: {pos, face, point}` | `ServerboundUseItemOnPacket`, main hand |
| `use` | `yaw`, `pitch` | `ServerboundUseItemPacket`, main hand |
| `break` | `pos`, `face` | `ServerboundPlayerActionPacket` `START_DESTROY_BLOCK` (creative breaks instantly) |
| `await_ack` | (none) | not a packet: the live bot waits for the acknowledgement of the last `use_on`/`break`. In the simulator it is a no-op |

### 1.5 Certificate (`certificate.json`)

```json
{"plan": "<sha256 of plan.json>", "variants": [{"name": "clean", "ok": true}, ...], "end": {"ok": true}, "ok": true}
```

## 2. Task order and dependencies

| id | title | depends on | risk |
|---|---|---|---|
| A1 | Package, copied libraries, build script | – | low |
| A2 | Simulator client | A1 | low |
| A3 | Target export and fixtures | A1 | low |
| B1 | In-world actor player | A2 | high |
| B2 | `play` op | B1 | high |
| B3 | Complete snapshot/restore | B1 | medium |
| B4 | `inert` op | B3 | medium |
| B5 | `hash` op | B3 | low |
| B6 | `shape_update` op | A2 | low |
| B7 | `poke` op | A2 | low |
| B8 | Perturbation options | B3 | medium |
| C1 | Classifier | B6 | medium |
| C2 | Hazard model | A3 | medium |
| D1 | Stands | A3 | high (spatial) |
| D2 | Stand graph and reachability | D1 | high (spatial) |
| D3 | Walking traces | D2, B2 | high (spatial) |
| D4 | Pillar and bridge traces | D3 | high (spatial) |
| E1 | Click choice with posture | D2 | high (spatial) |
| E2 | Witnesses and vetoes | E1 | high (spatial) |
| F1 | Trial runner | B2, B4, B5, C1 | medium |
| F2 | Step loop (no repair) | F1, C2, D4, E2 | high |
| F3 | Exit witness and escape obligation | F2 | high |
| G1 | Repair search | F2 | high |
| G2 | Update provenance (best effort) | B2 | medium |
| H1 | Certification | F2, B8 | medium |
| I1 | Replay bot and rehearsal | H1 | medium |
| J1 | Block-entity contents | G1 | high |

**Usage advice for Allen.** If Antigravity usage runs short, A1–C2 are the most mechanical and the best value. The tasks marked "high (spatial)" are where escalations are most likely.

## 3. Tasks

### A1. Package, copied libraries, build script

**Goal.** Set up the `online/` package.

**Steps:**

1. Create `online/package.json` with the dependency versions from `builder/package.json`: mineflayer, minecraft-data, prismarine-block, prismarine-item, vec3. Add `prismarine-physics` at the version found in `builder/node_modules/prismarine-physics/package.json`.
2. Copy `builder/lib/solver.js`, `builder/lib/blockstate.js` and `builder/lib/actions.js` to `online/lib/`, and `builder/test/actions.test.js` to `online/test/`. Fix the require paths. Make no other changes.
3. In `plansim-plugin/build.sh`, change the deploy loop to `../mc_server2 ../mc_server4` only, and the echo text to match. This is the one allowed edit outside `online/`.
4. Create `PROGRESS.md` and `ESCALATIONS.md` (empty headings).

**Done when:**
- `npm test` passes;
- `build.sh` builds and copies only to `mc_server2` and `mc_server4`.

### A2. Simulator client (`lib/sim.js`)

**Goal.** A JavaScript port of `planner/sim.py`: the same protocol, with JSON lines over TCP.

**Interface:**
- `class Sim { constructor({host, port, origin}); async call(op, args); close() }`
- Plus the wrappers `region`, `paste`, `read`, `set`, `act`, `settle`, `step`, `status`, `snapshot`, `restore`, `drop`, `events`, `kill`, `shapes`.
- Each wrapper converts plan and world coordinates exactly as `sim.py` does, using the same keys (`pos`, `from`, `to`, `moved`, `blocks`).

**Tests (`sim.sim.test.js`, on mc_server2):**
1. `ping` returns a version.
2. Paste three blocks into a region, `read` them back, and the states are equal.
3. Snapshot, `set` one cell to air, restore, read back: the original state is there.

**Done when:** these tests pass.

### A3. Target export and fixtures

**Goal.** Turn schematics into target files, and provide the fixtures.

**Steps:**

1. **`planner/export_target.py`** (a new file; allowed). Use `litematic.load` and `snbt.snbt` from `planner/` to write the §1.2 format. Usage: `uv run planner/export_target.py <in.litematic> <out.target.json>`.
2. **`online/tools/fixture.js`.** It builds target files from the box lists in Appendix A. `box(x0..x1, y0..y1, z0..z1, state)` fills every cell with x0 ≤ x ≤ x1, y0 ≤ y ≤ y1 and z0 ≤ z ≤ z1. Later boxes overwrite earlier ones, and `air` removes a cell.
3. **Site files.** Generate the fixtures F1–F6 and their site files exactly as Appendix A lists them. Also export F7 from `planner/cobble.litematic`.

**Tests:**
- Block counts per fixture equal the counts listed in Appendix A.
- The F7 count equals the number of non-air blocks `litematic.load` reports.

**Done when:** the tests pass and all files are in `online/fixtures/`.

### B1. In-world actor player (plugin) — risk: high

**Goal.** The plugin's actor player (`actor()` in `PlanSim.java`) currently never joins the world, so pressure plates, sculk sensors and pistons can't sense or push it. Make it a real entity in the level.

**Spec:**

1. **Join the level.** In `actor()`, after creating the player, add it to the level. Use `level.addNewPlayer(p)` or the closest equivalent that the server provides.
   - Keep `p.connection` as the `ActorListener`.
   - Keep `valid = true` and creative mode.
2. **Record teleports.** Override the listener's teleport method (the one that sends a `ClientboundPlayerPositionPacket` and sets the awaited teleport id):
   - call `super`;
   - append `{tick, pos}` to a `corrections` list on the plugin;
   - immediately call `handleAcceptTeleportPacket` with the awaited id, as a real client would.
3. **New op `actor`** `{op: "actor", pos?: [x,y,z], yaw?, pitch?}`.
   - It teleports the actor without recording a correction.
   - It returns `{pos, yaw, pitch, onGround, velocity: [vx,vy,vz], sneaking}`.
4. **Existing behaviour.** The existing `act` op must still work. Its tests in `builder/test` and `planner/` are not to be run; instead re-run `A2` and the B-task tests.

**Tests (`actor.sim.test.js`):**
1. The `actor` op puts the player at a given position on a stone floor. After `step 5` it is still there and `onGround` is true.
2. With the actor standing on the floor, `set` a `minecraft:stone_pressure_plate` at the actor's feet cell, then `step 2`. The plate's state reads `powered=true`. This shows the actor is sensed.

**Escalate** with E5 after one failed attempt at finding the method names, and with E3 if the plate is not powered.

### B2. `play` op (plugin) — risk: high

**Goal.** Feed a step's packet stream to the actor exactly as a client would, one server tick per stream tick.

**Spec.** `{op: "play", region, ticks: [[packet, ...], ...], inert?: {quiet: 20, cap: 200}}`, with packets as in §1.4 (world coordinates).

1. **Each tick:**
   - Send each packet to the actor's listener through the matching `handle...` method. Reset the spam limiter field before each use packet, as `opAct` does.
   - Then advance exactly one server tick, using the same mechanism as `step`.
2. **`slot`:** put the item in hotbar slot 0 and select slot 0, through `handleSetCreativeModeSlot` and `handleSetCarriedItem`.
3. **`await_ack`:** does nothing.
4. **After the last tick,** if `inert` is given, run the B4 inert check.
5. **Return:**

   ```json
   {"ticks": [{"pos": [..], "onGround": bool, "corrections": n}, ...],
    "corrections": [...], "events": [...], "inert": {...}}
   ```

**Tests (`play.sim.test.js`, on a 7×1×7 stone floor):**
1. Walk 3 blocks in a straight line using positions from a prismarine-physics run (D3 will do this properly; here use 0.2-block steps along one axis on flat ground, with `onGround` true). Expect zero corrections, and a final actor position equal to the last sent position within 1e-6.
2. Send one move 10 blocks away in one tick: expect at least one correction.
3. Sneaked `use_on` with `minecraft:stone` against the top face of a floor block places stone above it. Take the hit from `solver.js` via `actions.placeClick`, as in `builder/lib/actions.js`.
4. Place `minecraft:repeater` the same way, then do one unsneaked `use_on` on it with `minecraft:stick`: the delay goes from 1 to 2. Do the same click sneaked: the delay doesn't change.

### B3. Complete snapshot/restore (plugin) — risk: medium

**Goal.** Make snapshot/restore cover entities and the actor.

**Spec:**
- **Snapshot also stores:**
  - every non-player entity in the region, as full NBT, in a stable order;
  - the actor's position, yaw, pitch, velocity, `onGround` and sneaking.
- **Restore** kills the entities in the region, re-creates the saved ones from NBT, and puts the actor back exactly. That teleport must not count as a correction.

**Test:**
1. Spawn an item entity in the region and move the actor.
2. Snapshot.
3. Kill the item and move the actor again.
4. Restore.
5. The item is back at the same position, and the actor is back with the same position and velocity.

### B4. `inert` op (plugin) — risk: medium

**Goal.** Implement design §2 "Inert".

**Spec.** `{op: "inert", region, quiet: 20, cap: 200}` advances ticks until, for `quiet` consecutive ticks, all of the following hold:

- the block states are unchanged;
- every block entity's saved NBT is unchanged;
- the block tick queue, fluid tick queue and block events in the region are empty;
- there are no non-player entities in the region;
- the actor's velocity is zero (|v| < 1e-6).

**Returns** `{inert: bool, ticks, reason}`, where `reason` names the first failing condition when `inert` is false.

**Tests:**
1. A water source placed on the floor becomes inert within the cap.
2. A hopper holding 5 items above a chest is not inert while items transfer, and becomes inert once they're done.
3. Two observers facing each other never become inert. Use `minecraft:observer[facing=east]` at plan (0,1,0) and `minecraft:observer[facing=west]` at (1,1,0). Expect `inert: false` at the cap.

### B5. `hash` op (plugin) — risk: low

**Spec.** `{op: "hash", region}` returns `{hash}`: SHA-256 over, in order:

- block states in y, z, x order;
- block-entity NBT, sorted by position, as SNBT;
- the tick queues: (relative trigger tick, priority, position, type), sorted;
- block events;
- entities, sorted by type then position;
- the actor's position, rounded to 1e-6.

**Test:** the hash is equal before a snapshot and after its restore, and differs after one block change.

### B6. `shape_update` op (plugin) — risk: low

**Spec.** `{op: "shape_update", pos, state}` returns the state vanilla's `Block.updateFromNeighbourShapes(state, level, pos)` gives. It does not change the world.

**Test:** with stone at plan (0,1,0) and (2,1,0), `shape_update` at (1,1,0) for `minecraft:glass_pane` returns a state with `east=true` and `west=true`.

### B7. `poke` op (plugin) — risk: low

**Spec.** `{op: "poke", region}` does two passes over every cell in the region:

1. If `updateFromNeighbourShapes` gives a different state, set it with flag 3 and record the change.
2. Call the level's neighbour-changed handling for that cell (`level.neighborChanged(pos, block, null)` or the equivalent).

It returns `{changed: [[pos, old, new], ...]}`.

**Test:** on a fixture built and settled through `paste` + `inert`, `poke` then `inert` changes nothing.

### B8. Perturbation options (plugin) — risk: medium

**Spec:**
- **`restore` takes `shuffle: <int seed>`:** it restores cells with block entities in a seeded random order, so their ticking order differs.
- **New op `seed`** `{seed}`: sets the level's random seed.
- **New op `time_shift`** `{ticks}`: adds to the game time.

**Test:** the F1 fixture is pasted and inert, then restored with two different shuffle seeds. The hashes are equal.

### C1. Classifier (`lib/classify.js`) — risk: medium

**Goal.** Implement design §2 "Trial" classification.

**Interface.**

```js
classify({before, after, intended, actorBefore, actorAfter, actorPlanned, shapeUpdate})
  -> {clean, derived, power, memory, damage}
```

- `before` and `after`: `{blocks: Map<"x,y,z", state>, tiles: Map, entities: [...]}`.
- `intended`: a Map from cell to the target state of the action's own change.
- `shapeUpdate(pos, state)`: an async function backed by B6.

**Rules,** in this order, for each cell whose state differs and is not intended:

1. **Derived:** `shapeUpdate(pos, before)` equals `after`.
2. **Power-derived:** only properties in `POWER_PROPS[block]` differ, and the block isn't in `MEMORY`.
3. **Memory:** the block is in `MEMORY` and a property in `MEMORY[block]` differs. This is damage.
4. **Damage:** everything else.

**Also damage:**
- any entity in `after` not in `before`;
- any tile NBT change on a cell that isn't intended;
- `actorAfter` not equal to `actorPlanned` (tolerance 1e-6).

**Tables:** use exactly Appendix C. If a block or property needed by a test isn't in Appendix C, that's E7.

**Tests:** table-driven, with hand-written before/after maps. These are not spatial; they are lists of states.

### C2. Hazard model (`lib/hazard.js`) — risk: medium

**Goal.** The pruning and ordering rules from design §2 and §2b.

Everything here is distance-based. Use only the formulas given. Don't reason about which way things face.

**Interface.** `world` is a function `(x, y, z) -> state | null`; `null` means air.

- **`sensitiveCells(world, box)`** returns the set of cells within Manhattan distance 2 of any block in `SENSITIVE` (Appendix C).
- **`sculkCells(world, box)`** returns the cells within Euclidean distance 8 of a `sculk_sensor`, or 16 of a `calibrated_sculk_sensor`.
- **`tempSafe(world, cell, rts)`** is true when all of these hold:
  - the cell is not in `sensitiveCells`;
  - the cell is not in `sculkCells`;
  - none of the 6 neighbours is a tripwire or pressure plate;
  - none of the 6 neighbours is in `SHAPE_NEIGHBOUR` (Appendix C);
  - if `rts > 0`, no block in `SPREADS_TO_DIRT` is within Chebyshev distance 1 of the cell, horizontally or one cell above or below.
- **`fluidReach(world, cell, kind)`** is a BFS over cells that are air or non-colliding. It moves to the cell directly below whenever that is open. Otherwise it moves to the 4 horizontal neighbours, at most 7 steps from the last downward move (lava: 3). It returns the reached set.
- **`staticHazards(world, rts)`** returns these violations:
  - lava within Manhattan distance 2 of a block in `FLAMMABLE`;
  - if `rts > 0`, any temp next to a block in `SPREADS_TO_DIRT`.

**Tests:** small worlds built from box lists. The expected sets are computed by the formulas, never by eye. Each test states which formula produced its expectation.

### D1. Stands (`lib/stands.js`) — risk: high (spatial)

**Goal.** Design §2b "Stand", as exact geometry.

**Interface.** `isStand(world, cell, posture) -> {ok, feetY, why}`, where `cell` is the feet cell `[x, y, z]` and `posture` is `"stand"` (height 1.8) or `"sneak"` (height 1.5).

**Algorithm** (boxes come from `blockstate.collisionBoxes`, offset to world coordinates):

1. **Floor.** `floorTop` = the highest `maxY` among the collision boxes of the block at (x, y-1, z) and of the block at (x, y, z) whose `maxY` ≤ y + 0.6. Use the box only if it overlaps the column [x+0.2, x+0.8] × [z+0.2, z+0.8] in x and z. If there is no such box, or `floorTop` < y - 0.0001, the result is not ok (`why: "no floor"`).
2. **Body.** `body` = [x+0.2, x+0.8] × [floorTop, floorTop+height] × [z+0.2, z+0.8]. The result is not ok if `body` intersects any collision box of any block in the cells it overlaps.
3. **Cells.** The result is not ok if any cell `body` overlaps holds a fluid or a block in `ENTITY_REACTIVE` (Appendix C).
4. **Feet.** The feet position is (x+0.5, floorTop, z+0.5).

**Oracle test (`stands.sim.test.js`):**
1. Generate 300 random 5×4×5 worlds from a seeded generator. Each cell is air with probability 0.6; otherwise a random block from `[stone, oak_slab[type=bottom], oak_slab[type=top], oak_stairs[facing=north,half=bottom], glass, oak_fence, water, stone_pressure_plate]`.
2. Paste each world into the simulator. For every cell where `isStand` says ok, put the actor at the feet position with the `actor` op and `step 10`.
3. Pass if the actor's position is unchanged, `onGround` is true and the velocity is zero.
4. For 200 sampled cells where `isStand` says not ok because of "no floor", the actor must move.
5. **Accept ≥ 99%** in both directions. List the failures. If below 99%, that's E1, not E2: describe the failing cells by their block lists, without guessing why.

### D2. Stand graph and reachability (`lib/reach.js`) — risk: high (spatial)

**Goal.** Design §2b "Moves" as a graph, and a flood fill over it.

**Edges** from stand a to stand b, where both are `isStand`-ok with `posture = "stand"` unless noted:

| edge | condition |
|---|---|
| walk | b is a horizontal neighbour of a, and \|feetY(b) - feetY(a)\| ≤ 0.6 |
| diagonal walk | b is a diagonal horizontal neighbour, and both orthogonal cells between them are ok stands at the same feetY |
| jump | b is a horizontal neighbour, 0.6 < feetY(b) - feetY(a) ≤ 1.25, and the cell above a's head is free of collision boxes |
| drop | b is a horizontal neighbour's column, at the first stand going down, with 0.6 < feetY(a) - feetY(b) ≤ 3.0 |

**Interface:**
- `neighbours(world, stand)`.
- `reach(world, sources)` returns `Map<stand, parent>`.
- `path(world, from, to)` returns a list of `{stand, edge}`, by BFS.

**Hypothetical worlds.** Every function takes `world` as a function, so callers can pass the current world or a target-terms world. A target-terms world shows placed blocks plus the target blocks that count, per design §3.3.

**Tests:**
- **Property test:** in random worlds, `path` returns only edges that `neighbours` lists, and every stand in `reach(sources)` has a path back to a source.
- **Oracle:** covered by D3.

### D3. Walking traces (`lib/trace.js`) — risk: high (spatial)

**Goal.** Turn a D2 path into a §1.4 packet stream.

**Method:**

1. **Run the physics.** Run `prismarine-physics` against a mirror of the world, built from the target world plus the current world, as in `builder/actor.js`.
2. **Control.** The controller, each tick:
   - turns toward the next stand's feet position (yaw only);
   - presses forward;
   - presses jump on `jump` edges until it has risen;
   - sneaks on `walk` and `diagonal` edges where the next stand's floor is at the same height and the far side is a drop greater than 0.6;
   - stops sneaking on `drop` edges.
3. **Stop** when the horizontal distance to the final feet position is < 0.05 and the velocity is < 1e-3. Release all keys and run ticks until the velocity is 0.
4. **Record** each tick's resulting position, look and `onGround` as `move` packets. Record input changes as `input` packets.
5. **Give up** after 40 ticks per edge. That makes the edge unavailable; it is not an error.

**Oracle test:**
1. In 100 random worlds (D1's generator), pick random pairs of stands joined by a D2 path of length ≤ 6.
2. Make the trace, then `play` it in the simulator.
3. Pass if there are zero corrections and the final actor position equals the trace's last position (1e-6).
4. **Accept ≥ 99%.** If below, that's E1. Report the failing worlds as box lists.

### D4. Pillar and bridge traces — risk: high (spatial)

**Goal.** Design §2b compound moves as traces.

- **`pillar_up(n)`:** look down (pitch 90). Each time:
  1. hold `slot` with the block;
  2. press jump;
  3. when the bot's feet are ≥ the cell's y + 1.0, send `use_on` on the top face of the block below the feet cell, with the point at the centre of that face;
  4. send `await_ack`;
  5. land.
- **`pillar_down(n)`:** look down. Each time:
  1. send `break` on the block below the feet;
  2. send `await_ack`;
  3. fall until on the ground.
- **`bridge(c)`:** the bot stands sneaking on the stand next to c, at the edge facing c.
  1. Pick the click from `solver.js` placement for the target cell c, restricted to that stand and the sneaking eye height.
  2. Send `slot`, the `use_on`, then `await_ack`.

**Oracle test:** in the simulator, on a 5×1×5 floor:
- `pillar_up(4)` gives 4 stacked blocks with the actor on top and zero corrections;
- `pillar_down(4)` brings it back to the floor;
- `bridge` places the block.

### E1. Click choice with posture (`lib/clicks.js`) — risk: high (spatial)

**Goal.** Design §3.3 "Click choice", reusing `solver.js`.

**Refactor.** In `online/lib/solver.js`, turn `HEIGHT` and `EYE` into parameters, and add `solveFromStands(world, target, {stands, eye, sneak, mode, name, props})`.
- It tries only the given feet positions.
- It uses the existing face-point, steady-hit, look-margin and `placementOptions` logic unchanged.
- It rejects options whose eye-to-click-point distance is > 4.5.
- Existing behaviour must stay identical when called the old way. Keep `actions.test.js` passing.

**`clicks.choose(world, step, botStand)`** returns `{click, stand, posture, path}`.
- Placing clicks and `pour`/`waterlog`/`drain` use the `"sneak"` posture (eye 1.27). `tune` uses `"stand"` (eye 1.62).
- Candidate stands are the D2 stands within horizontal distance 5 of the cell.
- It orders them by path length from `botStand`, and returns the first with a solution.

**Oracle test:** for every block of fixtures F1, F5 and F6 in a valid build order (F2's order is given by D2/F2 later, so skip F2 here):
1. Paste everything before that block.
2. Choose a click and `play` it.
3. The resulting state at the cell must equal the target state, including tuning for F5.

### E2. Witnesses and vetoes (`lib/witness.js`) — risk: high (spatial)

**Goal.** Design §3.3 "Access witnesses" and "Veto".

**Interface:**
- **`witnesses(targetWorld, step)`** returns a list of `{stand, posture, ref, face, rayCells, bodyCells}`.
  - It uses E1's search over stands within distance 5 in the target-terms world.
  - `rayCells` are the cells the ray passes through, taken from `solver.raycast`'s cell traversal. Add a `cells` output to `raycast` if needed.
- **`reachable(targetWorld, entry)`** wraps D2's `reach`. It also allows hypothetical temp pillars of height ≤ 4 in cells where `tempSafe` is true.
- **`veto(state, x)`** returns the pending y's whose witnesses all die, or whose stands all disconnect, when x is added. It returns `[]` if none.

**Property tests** (random worlds; no expected coordinates):
- **Consistency.** For random x, `veto(state, x)` contains y exactly when y has ≥1 witness before x is added and 0 after, by full recomputation. The exceptions are y's own references and support, which the design exempts.
- **Monotonicity.** Adding blocks never increases a pending step's witness count, except by creating a new reference.

### F1. Trial runner (`lib/trial.js`) — risk: medium

**Interface.** `trial(sim, state, step)` returns `{clean, diff, hash, settle, changes}`.

**What it does:**
1. Snapshot.
2. `play` the step's ticks with the inert check.
3. Read the after-state.
4. Classify (C1).
5. Run `staticHazards` (C2).
6. Read the hash.
7. Leave the world in the after-state; the caller decides whether to commit or restore.

`changes` is the per-tick block-change list, from the plugin's events, for §1.4.

**Test:** on F1, a valid stone placement is clean. Placing a stone where the actor stands fails with "rejected", not as damage. Lava next to an oak log is damage, through the static hazard.

### F2. Step loop, no repair (`lib/loop.js`, `tools/plan.js`) — risk: high

**Goal.** Design §3 without §4: candidates, ranking, vetoes and pull-forward. On failure it writes a stuck report instead of repairing.

**Spec:**
- **Ranking key**, as a tuple, compared in order:
  1. layer y;
  2. 0 if the cell is a support, fence or reference of another pending step, else 1;
  3. 0 for blocks not in `REDSTONE`, else 1;
  4. 0 for non-fluids, else 1, with waterlog steps counted as fluids;
  5. the path length from the bot.
- **Memo key:** (step id, state hash).
- **Undo:** use a snapshot per committed step for now.
- **Stuck report** (`stuck.json` + `stuck.md`):
  - the step;
  - every candidate tried, with the first damage entry;
  - the vetoes in force;
  - the snapshot id.
- **`tools/plan.js <target> <site> <out.plan.json>`.**

**Done when:** F1, F2, F3, F5 and F6 produce complete plans in mc_server2, every step clean, with the bot ending on an entry stand.

### F3. Exit witness and escape obligation — risk: high

**Goal.** Design §3.3 "Exit witness" and the maintained obligation.

**Spec:**
- **After each commit**, `reachable` from the bot's stand to the entry must be true, allowing temps per design.
- **When it first needs temps,** trial the escape (D4 moves plus the walk), then restore. If the escape is not clean, roll back the step.
- **The escape's temp cells and their Manhattan-2 neighbourhood become vetoed cells** until the bot is out.

**Done when:** F4 produces a complete plan, and the bot never ends a step without an exit.

### G1. Repair search (`lib/repair.js`) — risk: high

**Goal.** Design §4: the reorder, wrap and backtrack generators, iterative deepening, a 200-trial budget, and the memo.

**Suspects** without provenance (G2): committed cells within Manhattan distance 3 of the failed cells, plus redstone components connected to those through 6-neighbour chains of `REDSTONE` blocks.

**Done when:**
1. With vetoes disabled (`--no-veto`), F6 still completes. Repair must fix any observer-first ordering.
2. F7 (cobble) either completes or ends in a stuck report whose step is not the observer case.

If F7 is stuck, that is not a failure of this task: escalate the stuck report to Allen as E9 with the report attached.

### G2. Update provenance (plugin, best effort) — risk: medium

**Spec.** While `play` and `inert` run, record (source cell → changed cell, tick) for neighbour updates inside the region. Use Paper's `BlockPhysicsEvent` source block where available. Return it in `play`'s result as `provenance`.

**Use:** G1 suspects come first from provenance, then fall back to the distance rule.

**Done when:** on F6 with the stone placed after the observer, the provenance of the piston change includes the observer's cell.

### H1. Certification (`lib/certify.js`, `tools/certify.js`) — risk: medium

**Goal.** Design §1b certification.

**Variants,** each a clean replay of the whole plan from a fresh paste of the start state:

1. clean, on mc_server2;
2. clean, on mc_server4, with a different `seed` and a `shuffle` restore of the start;
3. jitter: insert 0–2 empty ticks at random between a step's ticks;
4. batching: merge random adjacent ticks inside a step;
5. game time shifted by 100000;
6. a 1200-tick wait at every step boundary;
7. isolation, only when the site is `selfContained`: every margin cell is stone, except the cells the bot's body box touches at any tick of the plan and the entry stands' feet and head cells. When the site is not self-contained the variant does not run and the certificate records `{"name": "isolation", "ok": null, "skipped": "partial build (selfContained: false)"}`; it is never skipped silently.

**In every variant,** every step is re-classified, and its inert-state hash must equal the plan's `hash` for that step. Variant 7 compares only blocks inside the envelope.

**End check:** `poke` + `inert`, then compare with the target.

**Output:** writes `certificate.json` (§1.5) or a failure report naming the variant and step.

**Done when:** F1–F6 certify.

### I1. Replay bot and rehearsal (`tools/replay.js`) — risk: medium

**Goal.** Design §0 S3. Only Allen ever runs it against the live server.

**Spec:**
- **Packets.** A mineflayer bot with physics off (`bot.physicsEnabled = false`). It sends each tick's packets with `bot._client.write`, one tick group per 50 ms.
  - At `await_ack`, it waits for the server's block-change acknowledgement for that sequence number. On timeout (5 s) it halts.
- **Monitoring:**
  - It listens to block updates in the envelope and margin, and halts on any change not in that step's `changes`.
  - It halts on any `position` packet from the server, which is a correction.
- **After each step:**
  - Wait until world age ≥ the step's end + `settle` + 20 ticks. Read world age from update-time packets.
  - Compare every block in the envelope and margin with the certified state, plus the bot's position, and halt on difference.
- **Before each step:** if any other player or mob is within 16 blocks of the envelope, pause until it isn't.
- **Halts** print the step, the tick and the difference, then disconnect.

**Rehearsal (the done-when):**
1. On mc_server4, stop the freeze (`/tick unfreeze` over RCON).
2. Paste F1's start state.
3. Connect the bot on game port 25568 and replay F1's certified plan.
4. The final state equals the target, and there were no halts.
5. Re-freeze afterwards (`/tick freeze`).

### J1. Block-entity contents — risk: high

**Goal.** Design §2 `place(c)` is compound with its `tune`/fill: container contents and sign text go in as part of the block's own `place` step, during the build (like `waterlog`, but in the same step). Never as a pass after the build: machines depend on exact item counts.

**Until J1 is done (rule for F2 onward):** a target tile is accepted only when plain placement produces exactly its NBT (empty container, blank sign, comparator `OutputSignal` which is power-derived and compared by the end check). The trial compares the placed cell's NBT exactly with the target's. Any other target tile ends in a stuck report whose reason says "needs J1 (block-entity contents)".

**Spec (to be detailed before work starts):** new packet types for opening a container, clicking items into exact slots (creative), closing it, and editing a sign; the plansim actor and the replay bot (I1) must both handle them; the trial judges the cell's NBT exactly against the target.

## Appendix A. Fixtures

All coordinates are plan coordinates. Each fixture starts from an empty target. `entry` is the same for all fixtures: `[[-5, 0, z] for z in 0..4]`.

| id | origin | boxes, in order | blocks |
|---|---|---|---|
| F1 floor | [5000, -60, 0] | `box(0..4, 0..0, 0..4, minecraft:stone)` | 25 |
| F2 tower | [5100, -60, 0] | F1's box; `box(2..2, 1..9, 2..2, minecraft:stone)` | 34 |
| F3 lava pool | [5200, -60, 0] | F1's box; `box(1..3, 1..1, 1..3, minecraft:obsidian)`; `box(2..2, 1..1, 2..2, minecraft:lava[level=0])` | 34 |
| F4 closed box | [5300, -60, 0] | F1's box; `box(0..4, 1..3, 0..4, minecraft:stone)`; `box(1..3, 1..3, 1..3, air)` | 73 |
| F5 tuning | [5400, -60, 0] | F1's box; `box(1..1, 1..1, 2..2, minecraft:repeater[delay=3,facing=east,locked=false,powered=false])`; `box(3..3, 1..1, 2..2, minecraft:note_block[instrument=basedrum,note=5,powered=false])` | 27 |
| F6 observer and piston | [5500, -60, 0] | F1's box; `box(2..2, 1..1, 2..2, minecraft:piston[extended=false,facing=up])`; `box(2..2, 1..1, 3..3, minecraft:observer[facing=south,powered=false])`; `box(2..2, 1..1, 4..4, minecraft:stone)` | 28 |
| F7 cobble | [5600, -60, 0] | exported from `planner/cobble.litematic` | from the file |

## Appendix B. Spatial facts (the only ones you may use)

- **Axes.** +x is east, −x is west, +z is south, −z is north, +y is up.
- **Directions** (`DIR` in `solver.js`): north (0,0,-1), south (0,0,1), west (-1,0,0), east (1,0,0), up (0,1,0), down (0,-1,0).
- **Look.** Yaw 0 looks south (+z) and yaw 90 looks west (−x). Pitch 90 looks straight down and −90 straight up. Convert with `solver.yawPitch` and `solver.viewVector` only.
- **Cell and point.** A cell (x,y,z) spans [x, x+1) × [y, y+1) × [z, z+1). A world point p is in cell (floor(px), floor(py), floor(pz)).
- **Body.** Width 0.6. Height 1.8 standing, 1.5 sneaking. Eye 1.62 standing, 1.27 sneaking.
- **Reach.** 4.5, from the eye to the click point.
- **Neighbour sets.**
  - The 6 neighbours of c are c + DIR[d].
  - Manhattan distance is |dx|+|dy|+|dz|.
  - Chebyshev distance is max(|dx|,|dy|,|dz|).
  - Euclidean distance is between cell centres.
- **Observer:** detects changes at `cell + DIR[facing]`. (Only F6's description relies on this; the code never needs it, because tests use the simulator.)

## Appendix C. Tables

Block names are without `minecraft:`.

- **`POWER_PROPS`:** `redstone_wire: [power]`; `comparator: [powered]`; `repeater: [powered]`; `redstone_torch, redstone_wall_torch: [lit]`; `redstone_lamp: [lit]`; `powered_rail, activator_rail: []` (memory instead); `observer: [powered]`; `target: [power]`; `daylight_detector: [power]`; `lever: []`; `note_block: [powered]`; `door, trapdoor, fence_gate: [powered]` (`open` is memory).
- **`MEMORY`:**
  - `piston, sticky_piston: [extended]`;
  - `copper_bulb` and its variants: `[lit]`;
  - `repeater: [locked]`;
  - every `*_door`, `*_trapdoor` and `*_fence_gate`: `[open]`;
  - `barrel: [open]`;
  - `dropper, dispenser: [triggered]`;
  - `crafter: [triggered, crafting]`;
  - `lectern: [has_book]`;
  - `rail: [shape]`;
  - `powered_rail, activator_rail, detector_rail: [powered, shape]`;
  - `note_block: [note]`;
  - `sculk_sensor, calibrated_sculk_sensor: [sculk_sensor_phase]`;
  - `redstone_ore, deepslate_redstone_ore: [lit]`.
  - Plus: any block with a block entity whose NBT changed (contents).
- **`SENSITIVE`:** `observer, piston, sticky_piston, dropper, dispenser, crafter, redstone_wire, repeater, comparator, redstone_torch, redstone_wall_torch, rail, powered_rail, activator_rail, detector_rail, note_block, hopper, tnt, copper_bulb` (and variants), `target, bell, trapdoors, doors, fence_gates, lectern`.
- **`SHAPE_NEIGHBOUR`:** `redstone_wire, rail, powered_rail, activator_rail, detector_rail, glass_pane` (and coloured panes), `iron_bars, fence` (all woods), `nether_brick_fence, wall` (all), `chest, trapped_chest`, `*_stairs`.
- **`ENTITY_REACTIVE`:** `stone_pressure_plate`, all wooden pressure plates, `light_weighted_pressure_plate, heavy_weighted_pressure_plate, tripwire, tripwire_hook, powdered_snow, cobweb, sweet_berry_bush, turtle_egg, farmland, big_dripleaf, magma_block, soul_sand, honey_block, slime_block, scaffolding, sculk_sensor, calibrated_sculk_sensor, sculk_shrieker, redstone_ore, deepslate_redstone_ore, detector_rail`.
- **`REDSTONE`:** `SENSITIVE` plus `redstone_block, lever, stone_button` and all buttons, `daylight_detector, lamp`.
- **`FLAMMABLE`:** every block whose name contains `planks`, `log`, `wood`, `leaves`, `wool`, `carpet`, `bookshelf`, `hay_block`, `scaffolding` or `tnt`.
- **`SPREADS_TO_DIRT`:** `grass_block, mycelium`.

If a test or a fixture needs something these tables don't cover, that's E7. Don't extend the tables yourself.
