# Build planner spec: survival litematic paster

Handoff brief from a design discussion. Nothing here has been validated against a real schematic or server yet. Treat every heuristic and number as a hypothesis to test, and report what breaks.

## Goal

A bot (later a swarm of about 20) that builds a litematic on a survival server without creative mode, including working redstone. Target is the largest and hardest builds. Small schematics don't need this.

Scope of this document: the build planner only, meaning the thing that turns a schematic into a verified, ordered placement plan. Bot control, pathfinding, material logistics and swarm command are out of scope.

## Project context

- Target version: 1.21.11. Vanilla behaviour is assumed everywhere. No effort goes into detecting non-vanilla servers.
- Low-level bot library: Azalea (Rust). An earlier prototype used mineflayer.
- First test build: `cobble.litematic`, a cobblestone generator with a TNT duper. The schematic is saved in its charged state: a lever (on) holds a powered piston, and turning the lever off starts a high-frequency oscillator.
- Hard test case, later: a giant raid-tower item sorter with many pistons pushing observers. Known trap: the pistons must not end up powered by the observers, and placing pistons before observers is reported to be the mistake.
- Failures already seen with earlier agents: pistons retracted where the schematic has them extended, a deactivated repeater, and an agent that substituted cobwebs for redstone dust mid-build.
- Measured simulator speed on the dev laptop: about 400 TPS at best on a real server, far lower under heavy load.

## Principles

1. **The real game is the only oracle.** Never model redstone. No line can be drawn around "redstone" (sand matters in sand dupers), so any model is either wrong or a rewrite of Minecraft. All verification runs on a real vanilla server instance.
2. **No rule is trusted.** Static rules, heuristics and learned patterns only affect how fast a plan is found. Correctness comes from one full verified simulator run of the final plan for that specific build.
3. **Static often means armed.** A resting machine is frequently one block update away from firing, and armed vs discharged are both stable. "At rest" is not "safe", and arming is a main event of the build, not cleanup.
4. **State is acquired monotonically.** State can't be preserved before it is reachable, but once a placed block reaches its target state it must not leave it unless the plan says so.
5. **Never learn by failing on the real server.** Failures there are irreversible. All search and learning happens in the simulator.

## Block property classes

| Class | Examples | Handling |
|---|---|---|
| Placement-determined | facing, half, waterlogged | Planner must get right at placement |
| Derived | powered, dust level, piston extended | Function of neighbours; checked at checkpoints |
| History-dependent | latch contents, locked repeaters, container contents | Needs arming or commissioning actions |
| Connection shape | dust, fence, stair shape | Can't match until neighbours exist; excluded from checkpoint equality, checked at the end |

Note: connection shape still has functional consequences (dust shape decides what gets powered). It is excluded from the equality check only, not from the "nothing irreversible happened" check.

## Core invariant (checkpoint)

The build is cut into sections. At the end of every section:

- Every placed block matches the target, modulo connection shape and declared temporaries.
- No block belonging to an earlier section has changed.
- Nothing irreversible has happened: no block moved by a piston, no item dropped, no entity spawned, no liquid spread, no dispenser fired.

Inside a section, state may be temporarily wrong.

Temporaries (a blocker in a piston's path, a holding lever or redstone block, a block stopping a liquid) are declared wrong blocks with an explicit place step and remove step. Observers see temporaries being placed and removed.

## Algorithm

### 1. Classify blocks

Split the schematic into trivial bulk and the active set. The active set is every actuator or reactive block (pistons, dispensers, droppers, TNT, observers, liquids, gravity blocks, and so on) plus its cone of influence: wiring and sources that can reach it, and blocks it can physically affect. Bulk only needs the support rule (a block is placed against an existing block) and bot reach.

### 2. Build the dependency graph

Edge A→B means: B's target state needs A, or placing A can disturb B. Edges come from three sources, in decreasing trust:

**a. Hand-written seed (offline, tiny).** Only non-redstone facts: placement support, attachment support, bot reach.

**b. Knockout experiments (per build, in the simulator).** Paste the finished target. For each active block:

- Remove it, wait for the region to settle, record every other block whose state changed. Each one depends on the removed block.
- Put it back. If the region returns exactly to the target, the block is safe to place late. If not, it is history-dependent and belongs in a section that needs arming.
- If the region never settles after removal, flag the block as a trigger and exclude it from further knockouts.

Scaling: batch many knockouts per run when the blocks are far enough apart that effects can't overlap (group testing), attribute changes by proximity, and re-test ambiguous cases alone. Crop each run to the connected component plus a margin. For tiled machines, test one cell plus its neighbours.

**c. Corrections from failed checkpoints (evolving, in the simulator).** Knockouts only see steady-state dependence. Transient disturbance and latch-masked dependencies show up as checkpoint failures during planning. Shrink each failure to the few nearby blocks that mattered and add or flip an edge.

Optionally keep a cross-build library of patterns keyed by block types and relative geometry. These are hints for guessing edge directions and skipping experiments, nothing more.

### 3. Sections

Strongly connected components of the graph are sections. The condensed graph is a DAG, which gives the section order. Default direction is upstream first: sources, then wiring, then actuators, then payload, so each block lands directly in its target state.

- Single-block sections: trivial, correct at placement.
- Multi-block sections: feedback (clocks, latches, a piston moving the observer that triggers it). These need an internal search.

Ordering heuristics inside and across sections:

- Restraint before restrained. Anything that holds something back (an extended piston, a locked repeater, the generator's lever) is placed, powered and in its target state before the thing it restrains exists (TNT, water, sand, a piston's payload).
- Trigger last. Everything inside an armed element's update range is placed before it is armed. Nothing is placed there afterwards.
- Entities and container contents get their own late steps, with hoppers held locked.

### 4. Solve loop sections

Starting from the predecessors' checkpoint state, search the internal order in the simulator. Every step is quasi-static: place one block, wait for the region to settle, reject if it never settles, if something irreversible happened, or if a block regressed from its target state.

Tools, in order:

1. Heuristic order first. Most sections should pass on the first try.
2. Flat search with state merging for small sections. Hash the region after each step and merge identical states, so the space is about k × 2^k step simulations for k blocks, not k!. Rough budget: k=10 is about 10,000 steps, k=15 about 490,000. Don't go past roughly 10 to 15.
3. Constraint learning for larger sections: run a proposed order, shrink the failure to a local precedence constraint, have a solver propose a new order satisfying all constraints so far, repeat. The solver only handles ordering constraints. It never encodes redstone.
4. Temporaries, as extra place-then-remove steps drawn from a small palette.
5. Arming actions (lever flip, temporary pulse) when the section rests in the wrong stable state.
6. Tiling: find the repeating cell, solve it with its neighbours, replicate, verify with one linear run.
7. If a section is too large for all of the above, find a temporary that breaks the loop and split it.

### 5. Verify

Run the complete plan once, linearly, in the simulator at the true coordinates and orientation (some update order depends on position). The output is the ordered sections plus their internal orders, temporaries and arming actions.

### 6. Execution on the real server

- The bot runs the same checkpoint diff after each section. A failed diff means stop, copy the region into the simulator, and replan from that state.
- Sections with no path between them in the DAG can be built in parallel. Each active section belongs to exactly one bot, because concurrent placements make update order nondeterministic. Bulk is spread across workers.
- Repair later uses the same machinery: shut down to a known state, diff the structure, fix, re-arm.

## Simulator requirements

- Vanilla server, same version, region pasted at the real coordinates.
- Random tick speed 0 during planning, for determinism.
- Driven with `/tick freeze`, `/tick step`, `/tick sprint`.
- Settle detection, checked directly and not by a fixed wait:
  - no pending scheduled ticks or block events in the region
  - no moving-piston blocks
  - no moving entities (falling blocks, primed TNT, items in flight)
  - then a quiet window of about 20 game ticks
  - hard cap of about 600 ticks, after which the run counts as "doesn't settle"
  - reading the pending-tick queue needs a small server-side mod
  - both numbers are starting guesses
- Items stay out of hoppers until the contents phase, or hopper lines never go quiet.
- Snapshot and restore of region state, including scheduled ticks. This is expected to be the main engineering cost. Options: re-paste under tick freeze, or lay out many copies of the region in one world and explore one branch per copy.

## Outside the planner

These have no valid placement order and need a designer's procedure, written by a human or agent and checked by the same simulator:

- Unplaceable states: anything that only exists through update suppression, headless pistons, or blocks that were pushed into position.
- Self-assembling machines: the plan needs a "build this much, run it, continue" phase.
- Mid-motion snapshots (flying machines, bores): the schematic may not contain the parked state.

These aren't block problems at all and are run by the swarm as designed jobs:

- Entities with identity or behaviour: mobs to be transported and held, villagers, loaded minecarts, boats or minecarts at exact positions.
- World requirements: chunk alignment, perimeters, light, biome, specific spawners.

A complete build is therefore a package: schematic, section plan, temporaries, arming and commissioning steps, health checks.

## Open questions

1. Are hazards really local? Constraint learning assumes failures shrink to a few nearby blocks. The raid sorter is the test.
2. Can "restraint" be identified statically, or only learned from failures?
3. Which history-dependent mismatches matter, and which are don't-cares because the machine initializes itself? Probably needs a behavioural test (for example cobble output rate) or an annotation.
4. How do knockouts behave on machines with no resting state once a block is missing?
5. How expensive is exact snapshot and restore in practice?
6. Does the default "upstream first, payload last" guess hold for the piston-and-observer sorter, where the reported safe order is observers before pistons?

## First milestone

On `cobble.litematic`, in the simulator only:

1. Load the schematic, paste it at fixed coordinates, implement settle detection.
2. Classify bulk vs active set and print the counts.
3. Run the knockout and put-back pass on the active set. Report the dependency edges, the history-dependent blocks and the triggers.
4. Compute sections and the section order.
5. Execute the plan with simulated placements (setblock, one block per step, quasi-static) and report the first checkpoint that fails, if any.

Success for the milestone is a plan that passes the full linear verification, or a precise report of which assumption in this document failed.
