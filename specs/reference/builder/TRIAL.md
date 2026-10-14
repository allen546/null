# trial as built

Documents `online/lib/trial.js`.

Trial runner. Design references: planner/ONLINE_BUILDER.md §2 "Inert", "Trial", "Static hazards"; IMPLEMENTATION.md F1.

Entry point: `trial(sim, state, step) -> {clean, rejected, diff, hash, settle, changes, snapshot}`.

## State

- region: the sim region id (sim.region) of the whole site margin, including the grass layer below plan y = 0. Everything in it is read, classified and hashed. The static hazards run on the same cells.
- randomTickSpeed: the live site's gamerule (site file `gamerules.randomTickSpeed`), passed to staticHazards.
- The plan origin is the Sim's own (sim.origin). The actor's start position is read from the sim, because the planner's belief and the world must agree; the actor check compares it with the step's planned end position.

## Step shape

Step (IMPLEMENTATION.md §1.4, plan coordinates): `{kind, cell: [x,y,z], state, ticks: [[packet]]}`. `cell`/`state` are the intended cell and its target state (state 'minecraft:air' for a removal). The planned end position of the actor is the position of the last `move` packet in `ticks`, or the start position if the step has none.

## Per-tick rule (K2)

Per-tick rule (R4, relaxed by K2): the step's `changes` are looked at as a whole. When EVERY cell in `changes` ends in its before-state (a non-intended cell, a transient temp) or in its target state (an intended cell), the intermediate states are not judged: a piston that extends and retracts, a rail pulse that returns, an observer chain that settles all leave the build as it was. The final-state rules (classifier) still apply to every cell.

When some cell in `changes` ends elsewhere (a block was pushed out, a persistent change, an intended cell that missed its target) the strict R4 rule judges the step: a non-intended, non-transient cell in `changes` whose recorded state is neither its before-state, its final state, nor a power-only change of its before-state is a memory or damage entry (see judgeTransients). A cell that changes and changes back within ONE tick is not in `changes`, and so is not judged; that is accepted.

## Known gaps of K2

Known gaps of K2 (it is sound only inside these limits; see PROGRESS.md "Decisions (Allen, 2026-10-13)"):

- Entities. K2 assumes no entities in the build. Any entity spawn or move in a step is still damage. A minecart on a rail that is powered during a step is not modelled; an intended cart start (a cart put on a powered rail to begin a cycle) is not supported at all. Both belong to the entity stage (J1 or later) and need their own rule.
- Tile contents of non-intended cells. The end comparison covers block states, and tile NBT only for intended cells. A transient pulse that moves items (hopper, dropper, dispenser, crafter) in a non-intended cell passes unnoticed unless the block state also changes. Until the trial compares the tile NBT of every touched cell, K2 is only sound for builds whose containers are empty while the pulse happens.
- Effects outside the trial region (sounds reaching sculk sensors outside it, items or entities leaving it) are only caught as far as the region and the spawn rule see them.
- Timing: a transient that depends on tick timing is judged once per trial; the certification variants (jitter, batching, time shift) are what test that the end state is the same under other timings.
- A cell that ends in a persistent power-only change counts as "elsewhere" (the strict rule then judges the step), because K2 allows only before/target end states. That keeps K2 a pure relaxation of R4.

## Transient cells

Transient cells: a step may carry `transient: [[x,y,z], ...]`, cells the step's own packets place and remove within the step (an escape's bridge temp). Their intermediate states are not judged; their final state is judged as for any non-intended cell (it must equal its before-state).

## Tile NBT

Tile NBT: a step may carry `tiles: [[[x,y,z], nbt], ...]`, the expected NBT (SNBT string, as in the target file's `tiles`) of intended cells.

The simulator returns tiles in its own normal form (no `id`, `components:{}` added, sign messages unquoted), so the expected NBT is first normalised by the simulator itself: it is pasted into a scratch cell high above the site (stone all around) and read back.

After the step an intended cell with a block entity must have exactly that NBT (canonical JSON compare) and one without a listing must have no tile.

A placed dropper always gets `{Items:[],components:{}}`, so the step must list it (evidence: test/trial.sim.test.js).

A mismatch is damage, except that an intended comparator whose only difference is OutputSignal is power-derived.

## Compound steps (F2 extension)

F2 extension (compound steps, one trial for a step's moves, temps and action): a step may carry `intended: [[cell, state], ...]` instead of (or besides) `cell`/`state`: every cell the step changes on purpose, the action's own cell plus any temp it places or removes.

When `intended` is present it is used and `cell`/`state` are not needed (a pure walk has `intended: []`). With only `cell`/`state` it is `[[cell, state]]`.

A step is `rejected` when it has intended cells and none of them changed.

## What trial does

What it does: snapshot, read the before-state, play the ticks with the inert phase (quiet 20, cap 200), read the after-state, classify, run the static hazards, hash.

The world is LEFT IN THE AFTER-STATE; the caller restores with `sim.restore(result.snapshot)` (the snapshot id is returned, and is the caller's to drop).

## Result

- clean: no damage and no memory change, and not rejected.
- rejected: every intended cell still equals its before-state after the step: the action had no effect (the server refused it). rejected is never clean. The intended cells are then not judged against their targets; every other cell is classified as usual and anything that changed is still in `diff`. A step with no intended cell (a walk) is never rejected.
- diff: the classifier's {derived, power, memory, damage} (classify.js entry shape) plus these damage entries added here, with kind 'inert' / 'hazard' / 'actor':
  - not inert: {at: 'region', kind: 'inert', why: 'not inert: <reason>', before: null, after: <inert result>}
  - hazard: {at: 'cell x,y,z', kind: 'hazard', why: 'static hazard <type>: <message>', before: null, after: <violation>}
  - spawn: {at: 'entity <type> at <pos>', kind: 'entity', why: 'entity spawned during the trial ...', before: null, after: {type, pos}}: one per `spawn` event, so an item that floated out of the region (or vanished) is still damage. The classifier's own entity entries may name the same entity.
  - correction: {at: 'actor', kind: 'actor', why: 'the server corrected the actor's movement', ...}: a correction means a packet was refused or altered, so a live bot would diverge from the plan.
- hash: sim.hash(region) of the after-state.
- provenance: play's update provenance [{tick, pos, source, how}] (plan coordinates), for G1's suspects.
- settle: the ticks of the inert phase, as play reports them (the quiet window included).
- changes: [[tick, [x,y,z], state]]: play's `blockChanges`, the plugin's per-tick diff of every cell of the region (end-of-tick states, which is what a client receives; a cell that changes and changes back within one tick does not appear). tick counts from 0 for the first played tick. It includes the intended cell's own change and every ordinary change (wire power, piston heads, observer pulses). No event-derived data is in it; events are used only for the `spawn` damage rule.

## Transient changes (judgeTransients)

Transient changes (design §2: memory holds at every tick, relaxed by K2). `changes` is [[tick, [x,y,z], state]], the end-of-tick states of each cell that changed. A cell that changes and changes back within ONE tick is not in it; that is accepted.

K2: if every cell in `changes` ends in its before-state (not intended) or its target state (intended, per the `intended` map key -> state), nothing is judged here: the step left every cell it touched as it should be, and the intermediate states do not matter.

Otherwise, for each cell that is neither intended nor transient, every recorded state must equal the before-state, or the final state (judged by the classifier), or be the before-state's block with only its POWER_PROPS changed. Otherwise it is an entry: memory when a MEMORY prop of either block differs from before, else damage. Pure.

## Fluid flow (Allen, 2026-10-14)

A cell that is not intended may go from air (or from its target's fluid) to the fluid its target has, at any level: the classifier puts it in `fluid`, not `damage`, and `judgeTransients` accepts the levels on the way. Only the target's own fluid counts; flow into any other cell is still damage. The rule needs `state.target` (the loop and certification pass it). Without it nothing is allowed.

The loop marks a pending place step done when flow leaves its cell at exactly the target state (a flowing cell such as `lava[level=2]`). A source target that flow filled at another level stays pending and eligible: a bucket into the flowing fluid makes it a source.

Waterlogged blocks are not flammable: the `lava_flammable` hazard skips them.
