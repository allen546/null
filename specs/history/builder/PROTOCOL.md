# Contracts

Every workstream codes against this file. Don't change it on your own. If something here is wrong or
can't be implemented, stop and report it, and the orchestrator will amend it.

## 0. Coordinates

- **Planner coordinates** are schematic-relative: `(0,0,0)` is the bounding-box minimum that
  `litematic.load()` returns. Every planner file (`edges.json`, `plan.json`, reports) uses them.
- **World coordinates** = planner coordinates + `ORIGIN`. For the cobble milestone,
  `ORIGIN = (0, 100, 0)`, which is chunk-aligned and well above the superflat ground.
- Only `sim.py` translates between them. The plugin only ever sees world coordinates.
- Positions are `[x, y, z]` lists in JSON, and `(x, y, z)` tuples in Python.
- Block states are full vanilla strings with sorted properties, e.g.
  `minecraft:repeater[delay=1,facing=north,locked=false,powered=false]`.
  `litematic.state_string()` produces this form. The plugin must return the same form, using
  `BlockState` → `BlockStateParser.serialize`, with properties sorted by name.

## 1. Simulator plugin protocol (`plansim`)

The transport is TCP `127.0.0.1:25580`, newline-delimited JSON, one request at a time per connection.

```
request : {"id": 7, "op": "settle", ...args}
response: {"id": 7, "ok": true, ...result}      or   {"id": 7, "ok": false, "error": "text"}
```

The plugin runs every op on the server main thread. Operations against the world use NMS
(`ServerLevel`), not the Bukkit API, wherever that matters for exactness, such as flags and tick queues.
Only the overworld is used.

The plugin **keeps the server under `/tick freeze`** (set on enable, and restored after every op). Game
time advances only inside `settle` and `step`.

### Ops

| op | args | result |
|---|---|---|
| `ping` | – | `{"version": "plansim-1"}` |
| `region` | `min`, `max` (inclusive world coords) | `{"region": "<id>"}`. Adds plugin chunk tickets so every chunk touching the box stays loaded and entity-ticking. |
| `paste` | `region`, `blocks: [[x,y,z,state],...]`, `tiles: [[x,y,z,snbt],...]` | `{"placed": n}`. In this order: kill non-player entities in the region, clear scheduled block and fluid ticks in the region, set **every** cell of the region to air, set the given blocks, load the block-entity NBT, clear the region's event log. All block sets use flags `UPDATE_CLIENTS \| UPDATE_KNOWN_SHAPE \| UPDATE_SUPPRESS_DROPS` (no neighbour or shape updates, no `onPlace` side effects). Unlisted cells stay air. |
| `read` | `region` | `{"size":[sx,sy,sz], "palette":[state,...], "data":[int,...], "tiles": {"x,y,z": snbt}}`. `data` index = `((y-miny)*sz + (z-minz))*sx + (x-minx)`. `tiles` holds every block entity in the region, saved without metadata (no `x/y/z/id`), as SNBT. |
| `set` | `pos`, `state`, `updates` (bool, default true), optional `tile` (snbt) | `{"previous": state, "events": [...]}`. `updates=true` → `level.setBlock(pos, state, UPDATE_ALL)` (neighbour updates, neighbour shape updates and `onPlace`, like a placement whose state is already decided). `updates=false` → the `paste` flags. `state="minecraft:air"` is a removal. `events` are the events logged during this call. |
| `settle` | `region`, `quiet` (default 20), `cap` (default 600) | `{"settled": bool, "reason": "quiet"\|"cap", "ticks": n, "events": [...]}`. See below. |
| `step` | `n` | `{"ticks": n}`. Runs exactly n game ticks, then freezes again. |
| `status` | `region` | `{"blockTicks": n, "fluidTicks": n, "blockEvents": n, "movingPistons": n, "entities": [{"type","pos":[x,y,z],"vel":[x,y,z]}]}`. Positions are doubles for entities. |
| `snapshot` | `region` | `{"snapshot": "<id>"}`, held in memory. It captures block states, block-entity NBT, and the scheduled block and fluid ticks in the region with their **remaining delay**, priority and sub-tick order. |
| `restore` | `snapshot` | `{}`. Kills non-player entities in the region, clears the region's scheduled ticks, sets the blocks with the `paste` flags, loads the NBT, reschedules the saved ticks with the same remaining delays and relative order, and clears the region's event log. After this, the region must behave identically to when the snapshot was taken. |
| `drop` | `snapshot` | `{}` |
| `events` | `region`, `clear` (bool, default true) | `{"events": [...]}`, every event logged since the last clear |
| `kill` | `region` | `{"killed": n}`, removes non-player entities |

### Settle

Settle runs ticks one at a time and evaluates the region after each tick. Sprinting (or a raised tick
rate) is fine, as long as the evaluation happens after **every** tick and the run stops within one
tick of the decision. The **active** conditions are:

- any scheduled block tick or fluid tick with a position inside the region
- any queued block event inside the region
- any `minecraft:moving_piston` block in the region
- any non-player entity in the region that is primed TNT, a falling block, or has |velocity| > 1e-4
- any block state in the region that differs from the previous tick (a cheap per-tick hash is fine)

`quietCount` resets to 0 on any tick where one of those holds, and otherwise goes up by 1. The result
is `settled=true, reason="quiet"` when `quietCount == quiet`, or `settled=false, reason="cap"` when
`ticks == cap`. `ticks` = the number of ticks actually run.

### Events

The plugin logs events for the region with Bukkit listeners at `MONITOR` priority, never cancelling
anything. Each event is `{"type": ..., "tick": <game time>, ...}`:

| type | source | extra fields |
|---|---|---|
| `piston` | `BlockPistonExtendEvent` / `BlockPistonRetractEvent` | `pos`, `dir`, `extend` (bool), `moved: [[x,y,z],...]` (may be empty) |
| `spawn` | `EntitySpawnEvent` (covers items, TNT, falling blocks, mobs) | `entity`, `pos` |
| `flow` | `BlockFromToEvent` | `from`, `to` |
| `form` | `BlockFormEvent` (cobblestone, obsidian, …) | `pos`, `state` |
| `dispense` | `BlockDispenseEvent` | `pos`, `item` |
| `explode` | `EntityExplodeEvent`, `BlockExplodeEvent` | `pos`, `blocks: [[x,y,z],...]` |
| `entity_block` | `EntityChangeBlockEvent` | `pos`, `entity`, `state` |

The plugin logs these and doesn't judge them. Deciding which events count as irreversible is the
planner's job (`checkpoint.py`). Starting rule: a `piston` with a non-empty `moved`, and any `spawn`,
`flow`, `form`, `dispense`, `explode` or `entity_block`.

## 2. Python simulator client: `sim.py`

`sim.Sim(origin=(0,100,0), host="127.0.0.1", port=25580)` exposes the ops above as methods, in
**planner coordinates**. `sim.py` adds or subtracts `origin`, including for positions inside events
and inside `read`. Agent C's `FakeSim` (in `fakesim.py`) implements the same methods with a
scripted world. See `sim.py` for the exact signatures.

## 3. Block knowledge: `blocks.py` (agent B)

These are pure functions of a state string, with no I/O:

```python
parse_state(s) -> (block_id: str, props: dict[str,str])
format_state(block_id, props) -> str                       # sorted, inverse of parse_state
prop_class(block_id, prop) -> 'placement' | 'derived' | 'history' | 'shape'
checkpoint_props(state) -> set[str]   # props compared at section checkpoints (placement + derived + history)
shape_props(state) -> set[str]        # connection-shape props, excluded from checkpoint equality, checked at the end
role(state) -> 'bulk' | 'source' | 'wiring' | 'actuator' | 'reactive' | 'liquid' | 'payload'
```

`prop_class` must raise `KeyError` for any (block, prop) pair it doesn't know. It must never guess silently.

## 4. Files

### `out/classes.json` (agent B)
```json
{"version": 1, "counts": {"bulk": 0, "active": 0},
 "blocks": [{"pos": [x,y,z], "state": "...", "role": "...", "active": true, "why": "observer",
             "placeable": true, "how": "place|bucket|derived|unplaceable"}]}
```
`placeable=false` marks a block that a survival player can't put directly into its target state:
`how="derived"` for things another block produces (a `piston_head` from its extended piston,
flowing liquid from its source), and `how="unplaceable"` for states that need a designer procedure.
`how="bucket"` covers liquid sources and waterlogging. For the milestone the executor still uses
`set` for every block, but it reports the derived blocks separately.

### `out/edges.json` (seed: agent B; knockouts: agent D; corrections: later)
```json
{"version": 1,
 "edges": [{"from": [x,y,z], "to": [x,y,z], "kind": "support|attach|knockout|correction",
            "why": "free text"}]}
```
`from → to` means **`to` needs `from`**: place `from` first. The knockout convention: removing `A`
changed `B`, which gives the edge `A → B`.

### `out/knockouts.json` (agent D)
```json
{"version": 1, "settle": {"quiet": 20, "cap": 600},
 "baseline": {"settled": true, "ticks": 20},
 "results": [{"pos": [x,y,z], "state": "...",
   "removal": {"settled": true, "reason": "quiet", "ticks": 31,
               "changed": [{"pos": [x,y,z], "before": "...", "after": "..."}], "events": [...]},
   "putback": {"settled": true, "reason": "quiet", "ticks": 22, "returned": false,
               "diff": [{"pos": [x,y,z], "target": "...", "actual": "..."}], "events": [...]},
   "verdict": "late_safe|history_dependent|trigger|irreversible"}]}
```

### `out/plan.json` (agent C produces the format; milestone fills it)
```json
{"version": 1, "origin": [0,100,0],
 "sections": [{"id": 0, "kind": "single|loop|bulk", "after": [section ids],
   "steps": [{"op": "place", "pos": [x,y,z], "state": "..."},
             {"op": "place", "pos": [x,y,z], "state": "...", "temporary": true},
             {"op": "remove", "pos": [x,y,z], "temporary": true},
             {"op": "use", "pos": [x,y,z], "note": "arming: flip lever"}]}]}
```
Steps run in order and sections run in list order (which is a valid topological order). A
checkpoint runs after every section.

### `out/run.json` (executor result)
```json
{"version": 1, "passed": false, "sections_done": 41,
 "failure": {"section": 42, "step": 3, "kind": "mismatch|regression|irreversible|no_settle",
             "diff": [{"pos": [x,y,z], "target": "...", "actual": "..."}], "events": [...]}}
```

## 5. Amendments (2026-10-10, after Phase 1 reviews)

- `classes.json` blocks may carry `"needs_use": true`: the bot must click the block after placing it
  (a repeater delay above 1, a note block note other than 0, a comparator in subtract mode). This is additive.
- `plan.json` steps may carry `how` (copied from classes), `tile` (snbt), and for `op: "use"` a
  `state` to set (the executor simulates a use as a `set` with updates). Bulk sections settle once at the end.
- `run.json` adds `snapshot` (the initial snapshot id), `derived` (steps placed by `set` although
  `how != "place"`) and `final` (the `final_check` diff). Checkpoint failure kinds include `intruder`
  (a never-placed cell that is no longer air and not its own target state). `run.json` reports it under
  top-level kind `mismatch`, and each diff entry keeps `kind: "intruder"`.
- The irreversible rule allows, as checkpoint.py documents: a `flow` into a cell whose target is a
  liquid, and a `form`/`entity_block` that produces the target block id at that position.

## 6. Amendments (2026-10-10, after the agent A review)

Accepted deviations from agent A: `paste` and `set updates=false` use flags 562
(including `UPDATE_SKIP_ON_PLACE`). A grounded entity counts as moving only when its horizontal speed
exceeds 1e-4. `settle.events` = the events logged in the region during that settle, and `set.events` =
the events during that call, so the region log double-counts if you also call `events()`. Extra
`gameTimeDelta` on `settle` and `step`.

**Schematic migration:** `litematic.load` renames block ids for the server version
(`minecraft:chain` → `minecraft:iron_chain` for 1.21.4 → 1.21.11). Block-entity SNBT must be
compared parsed, never as text.

### New op `place` (player-placement emulation, used by the executor for every placeable block)

| op | args | result |
|---|---|---|
| `place` | `pos`, `state` (the target), `keep: [prop names]`, optional `tile` | `{"state": <actual state after the call>, "events": [...]}` |

The plugin starts from `block.defaultBlockState()` and copies only the `keep` properties from
`state` (the planner sends the placement-class props, plus history props until `use` exists). It then
applies `Block.updateFromNeighbourShapes(s, level, pos)` for connection shape, runs
`level.setBlock(pos, s, UPDATE_ALL_IMMEDIATE /*11*/)` as `BlockItem.place` does, then
`block.setPlacedBy(level, pos, placedState, null, new ItemStack(block.asItem()))` (this is how diodes
schedule their first tick and doors place their top half), then loads `tile` if given. Derived
properties (powered, extended, power, lit, …) are never copied, so the game computes them.

### New plan step `expect`
`{"op": "expect", "pos": [x,y,z], "state": "..."}` is used for `how="derived"` blocks (piston heads,
flowing liquid, door top halves). The executor does nothing for it. From that section's checkpoint on,
the position is treated as placed, so it must match its target, having been produced by something
else.

Implementation notes for `place` (agent A, accepted): `updateFromNeighbourShapes` is applied only if
it keeps the same block (a lone lower door would otherwise turn into air). Double-high items clear the
cell above first, as `DoubleHighBlockItem` does. `setPlacedBy` receives the state read back after
`setBlock`. The result adds `placedBy: bool`.

## 6b. Amendment (2026-10-11): player actions (`act`)

The emulated `place` writes the target's keep-props and ignores where a player could stand, so a proof with it
cannot see accessibility (stand spot, reach, line of sight, look direction) or what the placing act itself does
(a repeater placed at delay 1 and clicked up, leaves waterlogged after placement, scaffolds put up and broken).

### New op `act`
`{"op":"act","kind":"use_on"|"use"|"destroy","feet":[x,y,z],"yaw":f,"pitch":f,"item":"minecraft:repeater",
"sneak":bool,"expect":{"pos":[x,y,z],"face":"up","point":[x,y,z]},"pos":[x,y,z],"tile":snbt?,"range":4.5?}`
→ `{"state": state at pos, "hit": {pos, face, point}, "events": [...]}`.

A server-side player (never added to the world; creative, like the live bot) is put at `feet` with that yaw/pitch
(Minecraft degrees), holding a fresh `item`, sneaking or not. The crosshair is vanilla's own clip from the eye
(OUTLINE shapes, no fluids, `range`); it must land on `expect.pos`/`expect.face` and within 0.01 of
`expect.point`, else the op fails. Then the act goes through the packet handler a client's packet reaches:
`use_on` → `handleUseItemOn` (with `expect.point` as the cursor), `use` → `handleUseItem`, `destroy` →
`handlePlayerAction(START_DESTROY_BLOCK)`. The game decides the resulting state. Ticks do not advance.

### Executor and corrector
`execute.run(..., actor=actor.Actor(), gap=4)`: every place/remove step is done with `act`. builder/actor.js
picks each action against a mirror of the simulator's actual region (synced after every action): the placing
click, then tuning/waterlogging clicks for the state actually there, with a scaffold chain (dirt, temporary,
removed after) when no click exists, or a command-placed anchor when there is nothing at all to click.
`gap` ticks run after each action. New failure kind `unreachable` with `blockers`: placed cells whose absence
would allow the step. `correct.py --actor` turns them into edges of kind `access` (b → x: place b while x is
still empty) and writes the proven plan with every step's `moves` ([{label, pos, action, after}]) to
`plan_actions.json`. `live_build.py` replays exactly those moves with the mineflayer bot.
