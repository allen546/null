# Online builder: design v4.1

> **Being replaced (Allen, 2026-10-14).** Allen treats v4.1 as a specialized attempt that won't generalize. Its role passes to [proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md) once that is adopted; this document stays as the record of what was learned.
>
> **Status (2026-10-14).** Design v4.1, owned by Allen's builder agent. The JavaScript prototype that implements it is archived, read-only, in `null-builder-archive-2026-10-14`. Its as-built notes are in [`reference/builder/`](reference/builder/) and its run log is [`logs/builder/online/PROGRESS.md`](logs/builder/online/PROGRESS.md).
>
> Several decisions and rule changes made while prototyping amend this design but are **not yet folded into the text below**:
> - **K1:** a temp pillar stays only while the bot uses it. It may stay as a ladder only in a column with no target cell.
> - **K2:** a change that comes and goes inside a step is allowed when every cell the step touches ends in its before or target state.
> - **K3:** waterlog and drain clicks stand, not sneak, on 1.21.11.
> - **M1:** witnesses aren't judged against the finished build, and the bot may stand on the build.
> - Temps are left out of the witness veto.
> - Support temps are used for steps with no reference block.
> - Tile NBT is upgraded once at target export.
>
> Each is recorded with its reason in `reference/builder/LOOP.md` or the progress log. Where one disagrees with the text below, the later decision applies.

Clean-slate design. It assumes nothing from the existing planner code, graph files or learned edges. What gets reused is decided in the implementation plan. The version history and review outcomes are in §10.

## 0. Problem and assumptions

Build a schematic (up to ~20k blocks) on a live technical server with ticks running. The bot builds layer by layer, like a human, and no action may cause lasting unintended change: no fluid escaping, no blocks popping, no machine misfiring, no wrong final redstone state.

The product is a **guarantee** (§1b). The builder delivers either a certified plan or a stuck report. The live server never sees search, repairs or retries.

**Regions:**

- **Build region:** the schematic's bounding box.
- **Work envelope:** the build region grown by a few cells. Its non-build cells must be air on live. The bot may stand there and put temps there.
- **Margin:** a further shell around the work envelope. It is read-only, and any change in it is damage. It also contains the **entry area**: a fixed set of ground stands outside the build, connected by walking to where the bot arrives.

**What we assume we have:**

- **S1 Simulator.** It runs the **same server software, version, configuration, mods and gamerules as live**, plus an observation plugin that doesn't change game logic.
  - It can snapshot and restore a region: blocks, block entities, block/fluid tick queues in order, block events, entities, and the bot's position.
  - It can advance ticks one at a time.
  - It can read the state and report changes.
  - It reports update provenance on a best-effort basis: which cells' updates reached a given cell.
- **S2 Simulated player.** A fake player that **exists in the world**, so plates, tripwire, sculk, pistons and collisions affect it. It receives exactly the packets the live bot will send, one server tick at a time: position, rotation, on-ground, sneak/input, use-item-on (with cursor point), use-item and block-action.
- **S3 Live bot.** It replays the certified **packet stream** open-loop, one packet group per client tick.
  - It runs no physics and no pathfinding of its own.
  - Before a movement that depends on a block it just placed, it waits for the server's acknowledgement of that click (the block-change sequence number).
  - It halts on any server position correction.
  - While replaying, it listens to every block-change packet in the work envelope and margin, and halts on the first change that isn't in the certified per-tick change sequence for that step.
  - After each step it waits at least the certified settle time plus a margin, counted in game ticks, not wall-clock time.
  - Before each step it checks that no other player or mob is within the largest trigger range in the work envelope. If one is, it pauses; every step boundary is a safe place to wait (§1b).
  - After each step it also reads back what a client can see: all blocks in the work envelope and margin, its own position, the entity list, and container contents where it has permission (`/data get block`, if op). On any difference it halts.
- **S4 Vanilla shapes and shape updates.** Per block state: outline and collision shapes, and the vanilla neighbour-shape update (`updateFromNeighbourShapes`). These allow geometric and "derived" checks without trials.
- **S5 Environment.** These are the assumptions the guarantee rests on:
  - **The same game logic as S1.** Same server jar, version, mods and settings, and the same redstone implementation. The gamerules (including `randomTickSpeed`) are read from live and copied into S1.
  - **The site stays in fully ticking chunks for the whole build.** The site plus margin is force-loaded (`/forceload`) for the whole build.
  - **No foreign entity is in the work envelope, the margin or any sculk range during a step.** S3's presence check enforces this.
  - **The live start state is known server-side.** Either the margin is exported server-side (structure or world copy), or it is shown inert: no block entities, no power, no pending ticks.
  - **Randomness doesn't decide anything.** Any trial whose outcome changes with the RNG seed is not certifiable. That includes multi-slot dispensers and droppers firing, fire, and random drop velocities.
  - **The bot and S2 are both in creative**: unlimited items, instant breaking, no drops. Flight is never used (§2b).
  - **Nobody else edits the work envelope or the margin.**
  - **No weather or mobs reach the site.** Daylight-dependent blocks are handled in §9.
  - **The margin in S1 is copied from live** (see the start-state item). Certification shows nothing passes from the margin into the work envelope by power, fluid, entity or vibration (§1b).
  - **Random ticks are not assumed away.** If live `randomTickSpeed` > 0, random-tick effects are static hazards (§2), and certification checks the end state with random ticks on (§1b).

## 1. Principles

- **P1. Ask every question in the state where it matters.** "Is it safe to do a now?" is answered in the actual partial build at that moment. Nothing is inferred from counterfactuals about the finished build. No knockouts, and no global "x before y" facts except physics (support, and access geometry computed from the target).
- **P2. Only the simulator admits an action.** Geometry, the hazard model, caches and priors only order candidates and prune generators. Ordering does affect *which* valid prefix gets committed, so it affects success and speed, but never safety.
- **P3. The committed prefix is always a valid partial build.** Every committed action was trialled from the exact committed state and judged clean.
- **P4. Bounded.** Every failure gets a fixed trial budget. When it runs out, the builder stops with a stuck report. The budget guarantees the builder stops and stays safe. **It does not guarantee success.**

## 1b. The guarantee

The builder delivers a plan carrying this guarantee, or nothing.

**Statement.** Suppose the live world starts equal to the certified start state, and S5 holds. Then replaying the certified packet stream:

1. **Every action is proven.** Each packet is sent in exactly the state where it was proven.
2. **Every step ends inert and clean.**
   - **Inert** (§2) means the region is quiet.
   - **Clean** means the region differs from the previous step's state only by that step's intended change plus derived and power-derived updates (§2), and the bot is at its planned position.
   - So **pausing at any step boundary is safe while S5 holds.** That covers S3's pauses for nearby players or mobs, and stopping the build on purpose.
   - **A halt caused by divergence is not covered.** The halt happens *because* the world has left the proof. Continuous change monitoring (S3) makes this happen at the first unexpected block change rather than at the end of a step, which keeps the damage small. It cannot make the damage zero.
3. **The end state is the target.** No temps are left, and the state stays the target after a forced neighbour update of every cell.

**Certification.** Search trials are not the proof, because they run from restored states. A plan is certified only by a separate pass of **clean replays**: the whole plan from a fresh S1 loaded with the start state, with no restores. In every clean replay:

- **Every step is re-judged** with the full §2 classification, and its inert state is recorded.
- **Its hash must equal** the state search committed for that step.

Clean replays run in several variants, and every variant must give the same recorded states:

- **Determinism.** Two replays on different simulators, with different world RNG seeds and a different chunk-load order, so that block entities tick in a different order.
- **Perturbation.** Replays that vary what can differ on live:
  - ±1–2 gt jitter between the inputs inside each step;
  - random batching, so that several of a step's packets land in the same server tick, as on a lagging server or behind an acknowledgement wait;
  - shifted absolute game time;
  - a 1200 gt wait at every step boundary, to check that "inert" really is a fixed point.
- **Random ticks** (only if live `randomTickSpeed` > 0): the end state is run for 24000 gt with random ticks on. It must not change.
- **Isolation** (only for a self-contained build, site `selfContained: true`). One replay where the margin is replaced by stone, except the cells the bot's body passes through and the entry stands, and one with the live margin copy. The recorded states inside the work envelope must match. Stone at least 3 cells thick stops every redstone signal, so a match shows that nothing passes from the margin into the build. A partial build (one that connects to other builds) depends on its surroundings on purpose: it has no isolation replay, its certificate says so, and it rests on the live margin copy.
- **Change sequences.** Each step's per-tick block-change sequence is recorded for S3's continuous monitoring.
- **End check** (§5).

A step that only passes with exact timing is a time-critical pair (§9). It isn't certifiable, and ends in a stuck report.

Any mismatch means no certificate. It is treated as a bug in S1/S2 or a missing state component, never as something to retry around.

**What the live readback can and can't see.** A client sees blocks, its own position and, if op, container data. It can't see tick queues, hopper cooldowns or vibrations in flight. Those rest on S5 and on every step being inert. The readback is an alarm that the world has left the proof, not part of the proof.

## 2. Model

**State.** The work envelope and margin: blocks, block entities, tick queues, block events, entities. Plus the temps in place and the bot's position.

**Goal.** Every build cell matches the target, the work envelope's non-build cells are air, and no temps remain. Final acceptance is in §5.

**Core actions.** These are what the search itself moves through. The list is meant to be complete for block-and-fluid problems:

| action | what it is |
|---|---|
| `move(path)` | walk to a stand along a path of §2b moves; the trace is part of the trial |
| `pillar_up(n)` / `pillar_down(n)` / `bridge(c)` | survival movement that puts or removes temps (§2b) |
| `place(c)` | put the target block at c (the click chosen per §3.3) together with its `tune`/fill. **Compound:** judged against the full target state of c |
| `tune(c)` | stick clicks (repeater delay, comparator mode, note), opening a door/trapdoor/gate, container contents, sign text. **Unsneaked** (§2b). Contents and text go in as part of the block's own `place`, during the build, never as a pass after it: many machines depend on exact item counts from the moment they are built |
| `pour(c)` / `waterlog(c)` | water or lava source; waterlog a block |
| `drain(c)` | pick up a source or un-waterlog a block with an empty bucket |
| `temp(c)` / `untemp(c)` | put or remove a temporary block (dirt) inside the work envelope; for click references, standing or fencing. Allowed in cells whose final target is air, and in target cells not yet placed (the `untemp` then comes before that cell's `place`) |
| `break(c)` | remove a committed target block; only inside a repair plan, which must re-place it |

Multi-block targets are one `place`: door halves, beds, double chests, and piston heads as a result of extension/memory.

Scaffolding, keeping cells open, breaking and replacing blocks, and doing fluids late are **not macros**. They are ordinary moves in this vocabulary, and the search must find them itself.

**Inert.** The region is inert when all of these hold:

- no block changes;
- empty block and fluid tick queues and no block events;
- no block entity with pending work (hopper cooldown or transfer, a vibration in flight, sculk sensor or calibrated sensor phase, a brewing or furnace process, crafter cooldown);
- no entities except the bot;
- the bot at rest (zero velocity), on a valid stand (§2b).

**Trial.** Run one action from the current state. Wait the action gap, then advance ticks until the region is inert (cap 200 gt by default, more for lava). Compare the result with the *intended* state: the old state plus the action's own change.

Each other changed cell, and each other difference, is classified:

- **Derived:** the new state equals vanilla's neighbour-shape update of the old state in the new surroundings (glass panes, fences, walls, stairs shape, chest type, wire connections, `snowy`, …). Allowed.
- **Power-derived:** only the power-level properties of non-memory blocks changed: wire `power`, comparator output, `powered` on blocks where it has no lasting effect. Allowed, and recorded for the repair search.
- **Memory:** any change to a memory cell's state, at any tick of the trial, not only at its end (a piston that extends and retracts within one step has changed). A memory change is **damage**, unless it is on the action's own cell and is part of that cell's target. Memory cells are:
  - pistons and sticky pistons (`extended`);
  - copper bulbs (`lit`);
  - repeaters (`locked`);
  - doors, trapdoors, fence gates and barrels (`open`);
  - droppers, dispensers and crafters (`triggered`, contents, crafter slots);
  - any block with contents;
  - lecterns (book, page);
  - rails (`shape`, `powered`, for powered rails);
  - note blocks (`note`);
  - sculk and calibrated sculk sensors (phase);
  - redstone ore (`lit`).
- **Damage:** anything else. That covers:
  - other cell changes, including any change in the margin;
  - any entity appearing (in creative there are no drops, so any item entity is damage);
  - the bot's position changing during the action or settle, other than by its own planned movement (pushed, support removed, opened under it);
  - the region not becoming inert within the cap.

**Static hazards.** These are rules for effects S1 might not show inside the cap. Any trial whose result violates one is treated as damage:

- lava within 2 cells of a flammable block;
- an open path from a fluid source into a cell outside its target;
- if live `randomTickSpeed` > 0, any temp that is random-tickable or can be changed by a random tick (e.g. dirt that grass or mycelium can spread onto), and any build cell whose random tick would change it before the build ends.

## 2b. Movement model (survival movement, creative building)

The plan only uses movement a survival player could do. Building itself happens in creative, so items are unlimited and breaking is instant. Flight is never toggled; the bot never double-jumps.

**Body, posture and reach:**

- **Standing:** 0.6 × 1.8, eye height 1.62. **Sneaking:** 0.6 × 1.5, eye height 1.27.
- **Placing clicks and bucket uses are sneaked.** That way they never use the clicked block.
- **`tune` clicks are unsneaked.** Vanilla skips block use when the player is sneaking and holding an item. So a `tune` needs a stand that is safe unsneaked: not at an edge where the bot could slip, and with room for the 1.8 body.
- **Each click has one posture**, which fixes its eye height. A 1.5-high gap is a valid stand only for sneaked clicks.
- **Reach** is the survival block interaction range, 4.5, measured from the eye to the click point. This is conservative compared with the server's own check.

**Stand.** A stand is a feet position where:

- the collision shapes below (block tops, slabs, stairs, etc.) support the body;
- the body box for the posture is clear of collision shapes;
- the body is not in a fluid and not in a block that reacts to entities (pressure plate, tripwire, powdered snow, cobweb, sweet berry bush, …).

**Moves.** A path is a sequence of:

- walking to an adjacent stand (diagonals only when both corners are clear);
- stepping up ≤ 0.6 (slabs, stairs) or jumping up 1 block;
- dropping down ≤ 3 blocks;
- sneaking along edges.

There is no parkour, swimming, ladders or elytra.

These compound moves are core actions, not macros:

- **`pillar_up(n)`:** jump and place a block under yourself, n times. The block is a temp, or the target block when the cell's target is a full solid block and placing it there is clean.
- **`pillar_down(n)`:** break the temp under your feet and drop, n times.
- **`bridge(c)`:** sneaking at an edge, place a block at the next cell against the block you stand on. As with `pillar_up`, it is a temp or that cell's solid target block.

**Movement as packets.** A move's trace is produced once, at planning time, by a physics engine against the S1 world. That trace is the exact packet stream:

- **S2 sends it** to the in-world fake player.
- **Live sends it verbatim** (S3).

The live bot never re-simulates. That avoids the latency problem when pillaring: the client only sees a block it placed after a round trip. Packet order guarantees the server already has the block.

**What a move trial checks:**

- the server accepted every position, with no correction;
- the bot ended at the planned position;
- nothing the bot touched or came near changed: pressure plates, tripwire, sculk vibrations (no step vibrations while sneaking, but vibrations from placing still count), turtle eggs, farmland, dripleaf, redstone ore;
- nothing pushed the bot.

**Standing needs blocks:**

- **Elevated stands are temp structures** (pillars, staircases, bridges), unless the partial build itself provides the floor.
- **Temps follow one set of rules.** They are trialled when placed and when removed. They are covered by witnesses (§3.3). They must be removable, which itself needs a stand: usually `pillar_down` from the top.
- **Temp-safe cells.** A temp may only be proposed in a cell where placing or removing it can't update a sensitive block, per the hazard model:
  - not in front of an observer's face;
  - not adjacent or quasi-connected to a piston, dropper or dispenser the update could fire or BUD;
  - not in a sculk sensor's range (placing and breaking are vibrations);
  - not on a tripwire or pressure plate;
  - not as a neighbour that changes the shape or state of a placed block (rails, wire, …);
  - not random-tick-affected (§2).

  This only prunes what gets proposed. Every temp placement and removal is still trialled.
- **Temp lifecycle.** Every `temp` creates a pending `untemp` step with its own witnesses. Temps are removed lazily, but always before anything would veto their removal.
  - A temp *under* a block that needs support can never be removed. So a temp is never proposed where a pending target would rest on it.
- **The bot's body is an obstacle.** Vanilla refuses to place a block whose collision box would intersect the bot. Non-colliding blocks (wire, rails, torches) can go in the bot's cell.
- **Edges and sneaking.** The bot sneaks along edges so it can't fall off. Drops and `pillar_down` leave the edge deliberately, unsneaked.
- **The bot can't trap itself.** After every step there must be a way out to the entry area: the **exit witness** (§3.3).

## 3. The step loop

```
loop:
  R = candidates (§3.1), ranked (§3.2)
  for a in R, up to K = 8 trials:
      skip a if the access reservations (§3.3) veto it
      trial(the moves and temps a needs, then a) from the current state   # one step, §4 counts it once
      clean  -> commit, record undo-log entry (cells, tick queues, bot position), continue loop
      damage -> memo (a, hash of the region state) as failed; try the next
  nothing clean -> failure: pick s = the top-ranked unvetoed candidate, run repair search (§4)
```

### 3.1 Candidates

- **Support.** A cell is a candidate when its support exists, or can be given by a temp in a temp-safe cell. Temps go below or beside the cell and must be removable afterwards: checked geometrically when proposed, trialled when removed.
- **A stand.** A cell is also a candidate only if it has a reachable stand for its click's posture (§2b), possibly using a bounded temp pillar (§3.3).
- **Fluids and `waterlog` steps** are candidates once all their flow cells are closed or targeted. This is ordering only; the trial decides.

### 3.2 Ranking (ordering only; P2)

- **Layer:** a strong soft term. Lower layers first. A higher-layer cell may jump ahead when it is the support, fence or access witness (§3.3) of a pending lower cell, or when the reservations pull it forward. Layers are never a hard barrier, because vertical machinery spans layers.
- Supports and fences before what they hold.
- Inert blocks before redstone components.
- Components after the cells they watch, push or are powered by, per the hazard model.
- Fluids and waterlogging late.
- Shortest walk from the bot first.
- Within-build module replay and priors (§7).

### 3.3 Click choice and access reservations (geometric, no simulator)

**Click choice.** For `place(c)` or `tune(c)`, enumerate the stand, posture, reference face and look. Reject any where:

- the float look does not robustly hit the face;
- the stand is not valid for the posture (§2b);
- the eye of the click's own posture is more than 4.5 from the click point;
- there is no path (§2b) from the bot's current position, using existing blocks plus the temps this step proposes. The step's moves, temps and click are one compound trial.

Prefer the shortest path that avoids hazards (pressure plates, tripwire, sculk ranges).

**Access witnesses.** For every unplaced step y in any layer, keep its witnesses: (stand, posture, reference, ray), computed **in target terms**. Witnesses don't depend on the bot's position.

- **A reference counts** if it is placed already, or is a target cell that is not itself waiting on y.
- **A stand counts** if it is **reachable**, by one shared, incremental flood fill over stands. The flood fill:
  - starts from the entry area;
  - uses the §2b moves;
  - works in target terms: placed blocks plus target cells not waiting on y count as floor;
  - allows hypothetical temps only as **pillars of height ≤ 4 under a stand, in temp-safe air cells**. Nothing more general, so witnesses can't be satisfied by imagined scaffolding everywhere.

**Veto.** Before an action adds a block at x, check whether x would do any of these to some pending y:

- occupy the ray or body of every remaining witness of y;
- disconnect every stand of y in the flood fill;
- occupy y's last pillar cells.

If so, veto x. The veto is released when y is committed. This costs one incremental flood-fill update per candidate.

When every candidate is vetoed, the builder does not commit a killer. It pulls a vetoing y forward instead, across layers if needed, along with y's own support, reference and fluid prerequisites. Only if that is impossible does it treat the step as a failure (§4).

Waterlog, drain and tune steps have witnesses too.

**Exit witness.** After every step, the bot must be able to get from its actual position to the entry area. It may use the full move vocabulary: walking, `pillar_up`/`bridge` in temp-safe cells, `pillar_down`, and breaking its own temps.

When the exit first comes to depend on temps (the bot has walled itself in), the escape is **trialled once** from that state, temp placements and removals included, and then restored. If it isn't clean, the step that walled the bot in is not committed.

The escape then becomes a **maintained obligation**:

- Its temp cells, and their update neighbourhood, are reserved by vetoes.
- Any later step that a hazard-model change reaches inside that neighbourhood re-trials the escape before committing.

So the bot never commits into a hole whose only exit would flip an observer or a piston, either then or later.

**Observer case.** The observer's witness through the corner stand needs the two obsidian cells open, and its reference is a target cell. So the last obsidian is vetoed. The trapdoor's `waterlog` stays legal, but the hazard model ranks it after the obsidian it would flood, so it waits too.

## 4. Repair search (one mechanism for detours and backtracks)

A failure is any of:

- a blocked step s;
- a damaging memory change;
- a failed end check.

Repair is a **bounded local plan search over core actions**. Each candidate plan is trialled step by step from a real state. The first plan clean all the way through (including s) is committed.

A plan's length counts only core block-changing actions. The moves, pillars and temps that serve a step are part of that step. The memo key and the undo log include the bot's position.

**Plan generators, cheapest first.** All are ordered by the hazard model (fluid flood-fill, power reach, vibration reach):

1. **Reorder:** do pending steps first. These are the witnesses' references, fences, and drains of nearby fluid.
2. **Wrap:** undo a set O of committed cells, do s, redo O.
   - Nesting wraps outward: `undo(O₂) undo(O₁) s redo(O₁) redo(O₂)`, where O₂ is what made undoing O₁ dirty.
   - O comes from witnesses (geometric: cells blocking all of s's witnesses), and from the hazard model and provenance (mechanical).
   - A memory cell may be in O only if its redo provably restores its full state (`place` is compound).
3. **Backtrack:** restore to before committed step j (via the undo log), then replay the committed suffix with a *block* of steps moved after s. The block is j plus the steps that used j as reference, support or stand.
   - Suspects j are chosen from update provenance reaching the failed cells, newest first. They are not "anything nearby".

**Search order.** Iterative deepening on plan length. The memo prunes repeated (action, state-hash) failures.

**Budget.** One trial cap per failure: 200 trials by default, to be re-measured once movement trials are included (§8). Escalation widens the breadth (the generator candidates), never the depth.

**Worked example (observer, from the dead end with no reservations):**

```
reorder: nothing pending helps
wrap, O = {obs1, obs2} (geometric)
    break(obs1) is dirty: water spill, provenance points to the trapdoor's water
    O₂ = {trapdoor water}
plan: drain(T) break(obs1) break(obs2) place(observer) place(obs2) place(obs1) waterlog(T)
```

That is 7 steps, each with its own moves, well under budget.

**Why backtracking is not a knockout.** Nothing is concluded about j in general. A full alternative prefix is trialled from real states and adopted only if clean. The "j after s" it produces is scoped to this run.

## 5. Delayed failures and end checks

- **During the build:** each memory cell is checked whenever any of its neighbours or power sources changes. Its state must equal the target once all of its inputs are committed, and must never take a state change the target doesn't have. Otherwise it is a failure (§4), and provenance gives the suspects.
- **Derived and power-derived cells** are not checked during the build. Their final values are fixed by their surroundings.
- **End (each module and the whole build):** in S1, force a neighbour update of every cell, then wait until inert. Nothing may change, and the state must equal the target. A mismatch is a failure, and goes to the repair search like any other.

## 6. Macros

A macro is for a target the core vocabulary **cannot express at all**: no ordering of `place`, `tune`, fluids, temps or wraps produces it. Examples:

- an entity locked in place (a boat pushed in by 4 pistons, a minecart in a slot);
- budded or extended pistons with no visible power;
- blocks only pistons can put in place;
- states only reachable through update tricks.

A macro has:

- a **precondition pattern**: target cells plus local context, matched by rotation and mirror;
- an action script that may use temporary mechanisms, with its own cleanup;
- a **postcondition**: the cells it produces and the cells it needs open.

It is trialled and certified like one compound step. A person writes each macro, usually from a stuck report they solved in a sandbox. Macros are tried only on cells the builder can classify as inexpressible: a non-air target in an entity slot, a memory state with no input that could produce it, and so on.

## 7. Learning across builds

**Exact local-window caches are dropped.**

- At radius ≥ 2, windows almost never repeat across different builds.
- At radius 1, they say nothing about redstone, which reaches 15+ cells.

**What transfers:**

1. **Module library.** Hash structures canonically (rotation, mirror, block palette). Redstone builds reuse standard modules: clocks, sorters, flying machines, pulse extenders.
   - For each module solved before, store its whole committed sequence, repairs included.
   - In a new build, a matching module tries that sequence first. It is still trialled step by step, because the surroundings differ.
   - Within one build, repeated modules use the same mechanism. That is where most of the gain is expected.
2. **Macros** (§6).
3. **Hazard-model corrections.** Every time a trial is dirty in a way the model didn't predict, log it. A person turns recurring misses into model rules. This is learning, but deliberately human-reviewed.
4. **Repair priors and budget calibration.** Which generator fixes which failure class, and how many trials each class needs. These are small speedups.

**What doesn't transfer:** coordinates, global orders, one build's topology.

**Does it improve the next build?** Only through modules, macros and model fixes. Whether a new build finishes is decided by the core search plus budgets. When it doesn't finish, the outcome is a stuck report, never an unsafe step.

## 8. Budgets, live replay, cost

**Budgets:**

- K = 8 trials per step before failure;
- 200 trials per failure;
- escalation: 4× breadth once, then macros, then a stuck report;
- per-layer wall time 5× its clean cost.

All are re-measured after the first movement-inclusive runs.

The stuck report contains the state, s, every plan tried with its first dirty diff, and the vetoes in force.

**Undo log instead of snapshots.** Restoring = reverting the changed cells, block entities, tick queues, block events, entities, and the bot's position. It costs about the same at any region size.

**Live replay.** Planning and certification finish completely before the live bot sends anything. Replay is paced by server game time.

**Cost (to measure, not assume).**

- **Simulator:** ~0.4 s per trial measured on a small region without movement. Walks add ticks per step, and lava settling is 30 gt per cell of flow. Certification costs about 7 clean replays.
- **Live:** walking, temp pillars and bridges, and settle waits. There is no break time or restock, because the bot is in creative.
- Two simulators help with speculative candidate trials, independent modules and the certification replay variants.

## 9. Open questions

*Superseded (2026-10-15): v4.1 is replaced by [proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md) (Allen, 2026-10-14). These questions are kept as a record; any that still matter must be raised again in that design.*

1. **Time-critical pairs.** Examples: capping lava within ticks, blocking a piston before its next pulse. These aren't certifiable in v4; they end in stuck reports. A future timed compound action would need its own proof against live timing.
2. **Never-quiet builds.** Clocks have to be built inert, with the loop left open and closed last. Otherwise they aren't certifiable.
3. **Time-of-day blocks.** Daylight detectors change with the time of day, not with the build. v4 needs them isolated from everything they could power until the build ends, or the build isn't certifiable.
4. **Hazard model scope.** v4 covers fluid flood-fill, power reach (wire 15, observer chains, quasi-connectivity), vibration reach and random-tick effects. Misses are found by trials anyway, at a cost in speed.
5. **Provenance.** If S1 can't provide update provenance reliably, suspects fall back to cells within the power or fluid reach of the failure. That is slower, but still trial-checked.
6. **Movement scope.** Ladders, scaffolding blocks, water elevators and parkour are left out. Some tall, thin builds may need them. Without them, those builds end in stuck reports, not unsafe moves.
7. **Server-side movement checks.** The fake player in S2 must go through the same movement validation as a real connection (the "moved wrongly" and "moved too quickly" checks). Otherwise S2 could accept a trace that live rejects. The live readback of the bot's position catches a rejection, but only as a halt.

## 10. History and review outcomes

**v0 → v1, first adversarial review:**

| review point | change |
|---|---|
| Detour recursion did not produce the worked example | Repair is one local plan search; nesting wraps outward (§4) |
| The lookahead only fired for blocks that already had click options, so it never fired for the observer | Access witnesses in target terms, all layers, pull-forward instead of committing a killer (§3.3) |
| Volatile list was wrong both ways | Derived = vanilla neighbour-shape update; explicit memory-cell list; static hazards (§2) |
| Movement untested | Moves are trialled (§2b) |
| "Sealed" unimplementable | Memory-cell checks plus a forced-update end check (§5) |
| Backtrack and detour cost blowup | One trial budget per failure; provenance-picked suspects; moves blocks of steps (§4) |
| Undo/redo for components | `place` is compound; memory cells are wrappable only if fully restorable (§2, §4) |
| §7 overclaimed | Window cache dropped; module library, macros, model fixes; budgets guarantee stopping, not success (§7) |
| Hard layer barrier | Layer is a soft ranking term (§3.2) |

**v2, user decisions:**

- The guarantee comes before finishing.
- The live server gets only a certified full plan, so there is no pipelining.
- Drops depend on the block, tool and mode (glass, powdered snow).

**v3, user decisions:**

- Survival movement without flight, building in creative.
- The exit may use temps, but temps must be in temp-safe cells.

**v3 → v4, second adversarial review (verdict: pass with changes):**

| review point | change |
|---|---|
| The guarantee assumed, without saying so, that S1 and live run the same game logic; random ticks were assumed away | S5 requires the same server stack and gamerules; random-tick hazards are modelled (§0, §2) |
| Certification never re-judged steps | Every clean replay re-classifies each step and hash-checks it against search (§1b) |
| Live physics would diverge from the trace when pillaring | Live replays the certified packet stream verbatim (§0, §2b) |
| Sneaking blocks `tune` clicks | Tune is unsneaked; each click has a posture (§2b) |
| "Quiet" was incomplete, and one wait sample was too weak | "Inert" includes block-entity work and entities; several wait, batching, seed and order variants (§1b, §2) |
| Readback blind spots | What the readback can and can't see is stated; pause for nearby players or mobs; container data if op (§0, §1b) |
| Chunk loading | S5: the site stays in fully ticking chunks (§0) |
| Margin activity and edits | Any margin change is damage; isolation replay (§1b, §2) |
| Exit only on foot, so pillars over 3 were impossible | The exit uses the full move vocabulary; the entry area is defined (§0, §3.3) |
| Witness explosion, or witnesses made vacuous by imagined temps | Witnesses = (stand, posture, ref, ray); one shared flood fill; only bounded temp pillars (§3.3) |
| Were air targets "targets"? | Temps go in cells whose final target is air, inside the work envelope (§0, §2) |
| Wall-clock waits | Waits counted in game ticks (§0) |
| Bot moved during a settle | Counts as damage; the bot's position is in the undo log (§2, §8) |
| Repair cost with movement | Moves and temps count as part of their step; budgets re-measured (§4, §8) |
| Smaller gaps | Memory list extended (sculk phase, lectern, crafter, barrel, redstone ore); creative means any item entity is damage; the pillar and bridge moves can place target blocks |

**v4 → v4.1, the same reviewer on the temp-exit amendment (verdict: pass with changes):**

| review point | change |
|---|---|
| Live latency while pillaring | Open-loop replay, acknowledgement gating, halt on position correction (§0) |
| S5 incomplete | Force-loading, no foreign entities, start state known server-side, randomness not certifiable, random ticks not assumed away (§0) |
| "The halt is safe" was false | Only pauses are covered. A divergence halt is not; continuous change monitoring keeps it small (§0, §1b) |
| Quiet missed moving entities | The bot must be at rest on a valid stand (§2) |
| The wait test gave false confidence | Perturbation certification: jitter inside steps, batching, shifted game time, a fixed-point check (§1b) |
| The escape was trialled once but used later | The escape is a maintained obligation with reserved cells (§3.3) |
| Temps in target cells undefined | Allowed in unplaced target cells, with the `untemp` before the `place` (§2) |
| Scaffold lifecycle | Every temp creates a pending `untemp`; temps are removed lazily but before a veto (§2b) |
| Edge sneaking vs drops; bot body; click reach per posture | Clarified (§2b, §3.3) |
