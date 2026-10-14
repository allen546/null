# The step loop (`lib/loop.js`) as built

Design: planner/ONLINE_BUILDER.md §2b, §3, §3.1–3.3. Task: IMPLEMENTATION.md F2, F3, G1. This file records the decisions the code implements, so the code itself only names them.

## Contract

`runLoop({target, site, sim?, log?, tag?, vetoes?, repair?, forceFirst?, repairBudget?, repairGenerators?})` returns `{complete, plan, stuck, stats}`.

The loop pastes the start state (the margin, terrain only: a grass layer at plan y = -1) at the site origin, puts the bot on the first entry stand, builds until nothing is pending, then walks back to an entry stand. Every committed step was one `trial` (lib/trial.js) from the exact committed state, judged clean. If nothing clean exists and pull-forward and repair cannot help, the result is a stuck report, never an unclean step.

Test options (the defaults are the product):

- `vetoes: false`: ignore the witness vetoes when choosing steps. The escape obligation stays (it is a safety rule).
- `repair: false`: no repair search; a blocked loop is a stuck report at once.
- `repairBudget`: trials per repair call (default 200).
- `repairGenerators`: a subset of `reorder`, `wrap`, `backtrack`.
- `forceFirst: [[x,y,z], ...]`: these cells' place steps jump the ranking when eligible, so a test can force a bad order and see repair fix it.

## Steps

- `place`: put the target state at a target cell. Moves, temps, the click and any tuning clicks (repeater delay, note) are one trial, judged against the full target state.
- `waterlog`: fill a waterlogged target with a water bucket. The `place` before it uses `waterlogged=false`.
- `break`: repair only. Remove a committed target block; its place (and waterlog) steps become pending again.
- `untemp`: remove one temp. The bot walks to the top of its ladder and does `pillar_down(1)`. A temp is eligible only when nothing is above it.
- `walk`: the final walk back to an entry stand. It is not pending; it is added when nothing else is.

## Tiles (J1 interim rule)

A target tile is accepted when plain placement gives exactly its NBT (an empty container, a blank sign, a comparator that differs only in OutputSignal). This is decided once per distinct (state, NBT) at the start, on the simulator (trial.js `normalizeTile`, `defaultTile`). The place step of such a cell carries `tiles`, so the trial judges its NBT exactly. Any other tile gives the stuck report "needs J1". The end check compares every tile.

## Waiting on y

`refs(c)`: the neighbour cells a click for c's target state could use, for each direction in `solver.placementOptions`: a target cell or terrain that the solver accepts as a reference: not in vanilla's `replaceable` tag (so redstone wire counts), and for a bucket also with a collision box.

`waits(c, y)` is the least fixed point of: y waits on y; c waits on y when c has at least one ref and every ref of c waits on y. Terrain never waits. A cell with no ref waits on nothing and needs a temp. Cells that wait on y cannot exist before y.

## Witnesses (M1, Allen 2026-10-13)

A step y's witnesses are computed in y's target-terms world: placed blocks, plus target cells in y's layer or below that do not wait on y. Higher layers are left out. Judged against the finished build, the inside of a build has no witness from the start, so the veto never protected it (F7 run 6: 1413 of 1881 steps). The bot builds layer by layer and stands on what is below.

The witness cache keeps a step's entry when every changed cell keeps its stand view (name, collision, waterlogged where it matters) and, within `RAY_RANGE` of the step, its outline. A waterlogged full cube has the same view as a dry one. An outline that only grows (every old box inside a new one), at a cell not next to the step, cannot add a witness, so it makes the entry stale only if it lies on one of the entry's witness rays (F7: one wire or rail placed reshapes its neighbours, and without this every pending step within `RAY_RANGE`, nearly all of F7, was recomputed, ~100 s).

Temps are left out of the witness veto (Allen, 2026-10-14): the veto guards against losing access for good, and a temp comes down again (F7 run 9: a 2-high pillar for the observer at 12,3,14 killed the last witness of a layer-7 cobblestone, and the observer stayed vetoed). The escape obligation still sees temps.

## Temps and the ladder (K1)

Temps that stay standing are pillars only (one-step supports: below): h ≤ 8 blocks in a reached stand's own column, every cell in the envelope, empty, `tempSafe`, and not under a target cell. The block is dirt, except a pending target cell whose target is a plain full block: it gets its target, and that step is done by the pillar.

K1, Allen 2026-10-13, ladder version that night ("why not stand on the build? That's going to be literally impossible once builds get larger"; the bot stays on top as long as possible):

- A pillar is either a click pillar (the click is made from its top) or a climb (its top steps off onto the build, and the click is made from any stand reached from there).
- Only a pillar in a column with no target cell at any height may stay as a ladder (F7 run 7: a click pillar inside the build's footprint was left standing, the layers sealed it into a pit, and it could not come down). A pillar in a target column follows the old rule: the bot clicks only from its top and takes it down before moving on. Climbs are planned only outside target columns.
- At most one temp column stands at a time. No new pillar while it stands.
- While it stands, every plan must end where the bot can walk (D2) back to the ladder top (`guardPillar`, `onLadder`); `commit` asserts it.
- Untemps come after every place step and after pull-forward, so the bot stays up while anything can be done from there. Once an untemp has run, only untemps and repair breaks run until the ladder is gone (`dismantling`).
- The exit check (`exitOk`, `findEscape`) may walk to the ladder top and take the ladder down.

Limits: the plan uses the first click `clicks.choose` finds; if that ends out of walking reach of the ladder, the plan is refused even when another stand would do. Coming down by a water bucket is not modelled.

## Support temps

A place step with no ref at all (F7's eight floating vertical chains: a chain with `axis=y` is placed only against a top or bottom face, and the target has air above and below) is eligible when a support cell exists: an empty non-target cell in the envelope where a ref would be, next to a placed block that is not `replaceable`. One step places dirt there, clicks the target block against it, and breaks the dirt (`planSupport`). Nothing stays, so the ladder rule and the witnesses do not see it. There is no `tempSafe` check: the trial judges everything the support sets off.

## Exit and escape obligation (F3)

A plan is offered only if, after it, the bot can reach an entry stand: walking, or via its ladder. When the exit first needs temps, the escape is trialled once and becomes an obligation: its cells and their update neighbourhood are reserved, and later steps that reach that neighbourhood re-trial it.

## One iteration

1. Rank the eligible pending steps by: layer; 0 if the cell is a reference of another pending step or bounds a pending fluid; 0 if not a fluid or waterlog; 0 if not redstone (fluids after redstone: F7 run 9, the lava was tried before the observer that bounds it); path length from the bot (the BFS depth of the nearest reached stand within click distance).
2. For each, at most K = 8 trials: plan it (direct click, else a pillar or climb), skip it if a witness veto or the memo (same action, state hash and bot position) says so, else trial it. Clean: commit. Dirty: restore, memo, next.
3. If nothing was committed and some steps were vetoed: pull-forward. Try the vetoed steps' prerequisites (ready ones first), then the step, depth ≤ 3, inside the same K.
4. Then the untemps.
5. Nothing committed: repair (lib/repair.js) on the blocked step; if it finds nothing, a stuck report. A repair is adopted only if every action of it was trialled clean through `attempt`.

Commit applies the trial's changes to the planner's mirror, moves the bot, removes the done steps, adds one untemp per new temp, and keeps the trial's snapshot as the step's undo entry. After every commit the mirror is compared with a full read of the region; a difference throws.
