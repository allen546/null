# Build planner prototype: implementation plan

Goal: the "First milestone" from `BUILD_PLANNER_SPEC.md` on the cobble schematic, simulator only.
Orchestrator: Opus (main session). Workers: Sonnet 5.5 subagents, each with a cold-start brief,
owned files and an acceptance test. Status: approved 2026-10-10; in progress.

## What the inputs actually are

**Schematic** (`2025.7.1 515k cobblestone farm.litematic`, copied to `planner/cobble.litematic`).
It parses with the loader in `planner/litematic.py`. 15×15×15, 1882 blocks, 33 block entities
(28 empty droppers, 4 comparators, 1 sign), no entities, no pending ticks saved.

| Block | Count | Notes |
|---|---|---|
| observer | 234 | all `powered=false` |
| piston / sticky_piston | 158 / 23 | 1 piston and 6 sticky pistons extended, 7 `piston_head` |
| redstone_wire | 133 | 16 powered |
| lava | 122 | 108 sources, 12 level 2, 2 level 8 |
| repeater | 75 | 30 powered, none locked |
| iron_trapdoor / activator_rail / note_block / target / dropper | 59 / 58 / 34 / 20 / 28 | |
| waterlogged blocks | 60 | mostly leaves |
| tnt, lever, button, redstone_block ×2, water ×2 | | lever at (7,13,14) |

Two things to note early:
- **The active set is most of the build.** Actuators, observers, wiring and liquids make up well over
  half the blocks. Once the cone of influence is added, "bulk" is probably just glass and snow. So
  knockouts will cover about 1000 blocks, not a few dozen.
- **The lever is `powered=false` in the file.** The brief says the schematic holds a lever that is on.
  Either the brief is wrong, or the holding is done by something else, such as the redstone block at (3,13,7).
  This is the first assumption to check.

**Server** (`mc_server/`): this is **Leaves** 1.21.11, a Paper fork, not vanilla. The jar is
Mojang-mapped, so a plugin can call NMS (`LevelTicks`, `ServerLevel.blockEvents`) directly, and that
gives us the "small server-side mod" the spec asks for. The vanilla jar is also in
`cache/mojang_1.21.11.jar`, but it can't load a plugin. Paper has `allow-piston-duplication: false`
(the TNT duper needs it), while Leaves has `mc-technical-survival-mode: true`, which should turn it
back on. That has to be verified, not assumed. RCON is on 25575.

**Reference repo** (`lively-mendel/mineflayer-cannon-builder`): this is a mineflayer layer builder
for 1.21.4. The useful parts are its target-matching ignore lists (`lib/strictSchematic.js`
`dynamicProperties`, a good starting point for "derived" and "connection shape") and its warning
that tick freeze doesn't block synchronous neighbour updates. Its observer-gate Java agent modifies
the game, so it breaks Principle 1 and we won't use it.

## Decisions (defaults I will take unless you say otherwise)

1. **Simulator = Leaves + our plugin.** (User, 2026-10-10:) this Leaves build is patched by the user
   and is to be treated as 100% vanilla-compatible. No vanilla-jar cross-check. Leave its gameplay
   config as-is; only world settings (gamerules, world type) change.
2. **Language: Python for the planner**, managed by **uv** (`planner/pyproject.toml`; run everything
   with `uv run`, add deps with `uv add`). Expected deps: `networkx` (SCC/condensation), `pytest`.
   The hand-written NBT loader stays because it already works. Java for the plugin. Azalea/Rust isn't
   involved, because bot control is out of scope.
3. **World reset:** `flat_world*`, `logs/` and `usercache.json` get moved to the macOS Trash, not
   `rm`-ed. The new world is superflat with mob spawning, daylight and weather off and
   `randomTickSpeed 0`. The build is pasted at a chunk-aligned origin at y=100, clear of the ground,
   and kept loaded with `forceload`.
4. **Simulated placement = `setBlock(target, UPDATE_ALL)`**, as the milestone says. It's a known
   fidelity gap: a real player placement computes its state through `getStateForPlacement`, which
   can differ for dust, rails, stairs and the like. That gap is logged as a phase-3 item, not fixed now.

## Architecture

```
planner/                    Python, uv project (pyproject.toml, uv.lock)
  litematic.py              NBT + litematic loader (exists)
  blocks.py                 property-class table: placement / derived / history / shape
  classify.py               bulk vs active set + cone of influence
  seed.py                   static seed edges: support, attachment
  sim.py                    client for the plugin protocol (+ FakeSim for unit tests)
  graph.py                  dependency graph, SCC, condensation, section order
  checkpoint.py             checkpoint comparator (modulo shape and temporaries, irreversible log)
  knockout.py               knockout / put-back pass -> edges.json
  execute.py                run a plan quasi-statically, report the first failing checkpoint
  milestone.py              steps 1-5 end to end -> out/report.json + out/report.md
plansim-plugin/             Java Leaves plugin, built with javac against the server jar
mc_server/                  the server (reset)
```

### Simulator protocol (the contract every workstream codes against)

The plugin listens for JSON lines on TCP `127.0.0.1:25580`, and every op runs on the main thread.
Positions are absolute world coordinates, and block states are vanilla strings
(`minecraft:repeater[delay=1,facing=north,...]`).

| Op | Args | Returns |
|---|---|---|
| `region` | `min, max` | `id` (defines the watched box, also used for event capture) |
| `paste` | `region, blocks[[x,y,z,state]], tiles[[x,y,z,snbt]]` | ok. No updates. Clears region first. |
| `read` | `region` | `palette[], indices[]` (x fastest, then z, then y) + block-entity SNBT |
| `set` | `pos, state, updates=true` | ok. `state="minecraft:air"` removes. |
| `settle` | `region, quiet=20, cap=600` | `{settled, ticks, reason, events[]}` |
| `snapshot` / `restore` | `region` / `id` | blocks + block entities + scheduled block/fluid ticks in the region; restore also kills non-player entities in it |
| `status` | `region` | pending block ticks, fluid ticks, block events, moving pistons, entities |

How `settle` works: the server is kept under `/tick freeze`. Settle sprints and checks the region at
each tick end (`ServerTickEndEvent`). It succeeds once the region has had no pending ticks, block
events, `moving_piston` blocks or non-player entities, and no block-state change, for `quiet`
consecutive ticks. After `cap` ticks it returns `reason=cap`. During every op the plugin records
**irreversible events** in the region: a piston extend or retract that moves at least one block,
an entity spawn (item, TNT, falling block), liquid flow (`BlockFromToEvent`), a dispense, and an explosion.

## Workstreams

**Phase 0 (Opus, serial, short).** Freeze the protocol above into `planner/PROTOCOL.md`, write the
`sim.py` client signature and the plan/edge JSON formats, run `uv init` in `planner/` with
`networkx` and `pytest`, and run `git init` on the project so
every agent's diff can be reviewed.

**Phase 1 (three Sonnet 5.5 agents in parallel, each touching only its own files).**

| Agent | Owns | Delivers | Acceptance |
|---|---|---|---|
| **A: simulator** | `plansim-plugin/`, `mc_server/` | world reset, vanilla-ish config, plugin implementing the protocol, `build.sh`, `start.sh` | scripted smoke tests on a live server: a repeater clock → `settled=false, reason=cap`; an observer-piston pair pushing a block → irreversible event logged; snapshot during a repeater delay, restore, and the repeater still fires (scheduled ticks survive); paste/read round-trips the cobble schematic exactly; TNT duper check (piston duplication enabled) |
| **B: schematic & classes** | `blocks.py`, `classify.py`, `seed.py` + tests | property-class table for every block id in the schematic, active set + cone, seed edges | unit tests; prints bulk/active counts for cobble; every palette entry is classified (unknown = error) |
| **C: planner core** | `graph.py`, `checkpoint.py`, `execute.py`, `sim.py` FakeSim + tests | SCC/condensation/topo order with upstream-first tie-break, checkpoint comparator, quasi-static executor against the `Sim` interface | unit tests against FakeSim (a hand-built cycle gives one multi-block section; a checkpoint diff catches regression, shape-only mismatch, and an irreversible event) |

Only agent A starts the server. B and C never touch it.

**Phase 2 (after A passes).**
- **Agent D (Sonnet): knockouts.** `knockout.py`: paste the target, settle, snapshot. Then for each
  active block: remove it, settle, record the changed blocks and the result (`settled`, `cap` or
  `irreversible`), put it back, settle, check whether the region returns to the target, restore.
  Output is `out/edges.json`, `out/history_dependent.json` and `out/triggers.json`. It starts with
  one knockout per run (about 1000 runs, an estimated 10–20 min). If that's too slow it moves to
  group testing, or shards across 2–3 server copies on separate ports.
- **Opus: integration.** `milestone.py` wires steps 1–5 together. I review each agent's diff
  against the contract before merging.

**Phase 3 (depends on what the milestone shows).** If a checkpoint fails, which I'd expect given
the oscillator, the next steps are the loop-section search (state-merging BFS over orders, k ≤ 12),
then temporaries and arming actions, then placement fidelity via `getStateForPlacement`. Each of these is a separate agent brief, written once the failure data exists.

## Subagent rules (go into every brief)

- Read `BUILD_PLANNER_SPEC.md`, this file and `planner/PROTOCOL.md` first. Don't change the protocol.
  If it needs to change, stop and report.
- Only edit the files you own. Python goes through uv (`uv run`, `uv add`), never bare pip or
  system python. Only agent C adds deps to `pyproject.toml`; others ask. Java 21 target.
- Never model redstone in code. Facts about behaviour come from the server.
- Finish with: what was built, the exact test commands and their output, and every assumption from
  the spec that broke, each with evidence.

## Milestone output

`out/report.md` covers the bulk/active counts, edges, history-dependent blocks, triggers, the
sections and their order, and the first failing checkpoint with the block diff and the irreversible
events. Otherwise it records a full linear pass. Each spec assumption gets marked as held or broke.

## Open questions for you

Resolved 2026-10-10:
1. The schematic is in the charged state: the TNT duper is on. The lever is inverted, so
   `powered=false` is the "on" state of the machine.
2. Leaves (this patched build) counts as vanilla. No cross-check needed.
