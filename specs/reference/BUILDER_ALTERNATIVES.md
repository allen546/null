# Builder alternatives: faster, less LLM-dependent ways to build schematics

Research proposal, 2026-10-14. Written for Allen and his builder agent, who own the implementation. It builds on [`ONLINE_BUILDER.md`](../ONLINE_BUILDER.md) (design v4.1), [`builder-hard-cases.md`](builder-hard-cases.md) and the interface in [`C1-builder.md`](../C1-builder.md), and edits none of them. Inputs were six scout reports, copied into [`builder-alternatives-scouts/`](builder-alternatives-scouts/). Nothing here is measured unless it says so. Speed figures are estimates with their reasoning shown.

## Summary

1. Keep v4.1's certificate (§1b) exactly as it is, and change what feeds it. v4.1 already says search trials are not the proof, so the search can be optimistic without weakening the guarantee.
2. Most of v4.1's cost is not game physics. It is per-trial overhead, a 1,200 gt wait at each of ~20k step boundaries (~24M ticks for a 20k-block build), and 200-trial local repairs that never learn why a trial failed. That last point is also why it needs the LLM.
3. Build a layered hybrid on top of it:
   - a fast in-process simulator harness with exact provenance (A);
   - station-sized steps, and static acceptance where nothing can react (B, C);
   - an exact "build cold, arm last" (D);
   - reactive clusters solved in parallel, with learned ordering constraints (E, F);
   - access planned by peeling the finished build (G);
   - a dependency graph so many bots can share one plan (I).
4. Expected gains, all estimates: 10–100× less planning time, about 3× less live time per bot, and more with parallel bots. Dense redstone gains least.
5. The LLM stops debugging state dumps. It writes reusable macros and generators from small conflict cores instead (K).
6. Decisions for Allen: air placement (H), coarser pause points in inert regions (B), and an opt-in weaker certificate for random outcomes (J).
7. First experiments: profile one trial (1 day), a static census of the test corpus (2–3 days), and an air-place test on the 1.21.11 jar (1 hour).

## 1. Where v4.1's cost and fragility come from

### 1.1 The guarantee lives in certification, not in the search

v4.1 §1b says it directly: "Search trials are not the proof, because they run from restored states." The proof is the set of clean replays from a fresh simulator, in which every step is re-judged with the full classification and checked against a recorded hash.

The search principles P2 ("only the simulator admits an action") and P3 ("the committed prefix is always a valid partial build") make the search tidy. They don't make the result safe; certification does that.

So a planner can propose actions it has never simulated and let the first clean replay find its mistakes. It still delivers exactly the §1b statement. Two things change:

- A mistake now costs a re-plan from a checkpoint, where before it cost a rejected trial.
- A stuck report's valid prefix is now "the prefix the last replay verified".

Every candidate below that claims "guarantee unchanged" rests on this. If Allen wants P2 kept literally, candidates B and C still work. They just lose most of their search-side benefit.

### 1.2 A cost model

The prototype's numbers aren't available here. The model below uses v4.1's own figures (0.4 s per trial, 200 trials per failure, about 7 certification replays, a 1,200 gt wait at every step boundary) plus stated assumptions. It is meant to show which terms dominate, not to predict wall time.

Reference build: 20,000 target blocks, 5% of them reactive or next to something reactive, one bot. v4.1 makes one step per action, about 20,000 steps. I assume about 40 simulated ticks per step for walking, sneaking, clicking, the acknowledgement and settling, 1.3 trials per step, and 200 failures that need repair at about 50 trials each.

| term | quantity | at 20 TPS (real time) | at ~1 ms/tick (sprint) |
|---|---|---|---|
| search trials | ~36,000 × 0.4 s | ~4 h (measured per-trial figure) | unknown until profiled |
| 6 plain certification replays | 6 × 20,000 × 40 ≈ 4.8M ticks | ~2.8 days | ~1.3 h |
| fixed-point wait replay | 20,000 × 1,200 ≈ 24M ticks | ~14 days | ~6.7 h |
| live build, one bot | 20,000 steps × ~1.5 s | ~8 h | n/a |

Four conclusions follow.

- **The fixed-point wait dominates certification.** It costs about five times all the other replays together. It scales with the number of step boundaries, not with the amount of work, so fewer, larger steps where nothing can react cut it directly (candidate B).
- **The 0.4 s per trial is probably mostly overhead.** A vanilla tick for a small loaded region should cost around a millisecond (not measured here). A trial without movement needs perhaps 5–20 ticks. Note that 8 ticks at 20 TPS is exactly 0.4 s. That fits a simulator paced in real time, or one dominated by IPC, snapshots and region diffs. Either way it is an engineering cost, not an algorithmic one (candidate A).
- **On the happy path, search is about one replay.** The rest of search time goes to failures: K = 8 alternatives per step, and 200-trial repairs. The budgets were sized around the slow trial.
- **Live time is one walk, one acknowledgement and one settle per block.** One action per step makes the walking and waiting per block the dominant live cost (B, I).

### 1.3 Why it leans on the LLM

| kind of stuck report | example | why v4.1's search can't fix it | what removes it here |
|---|---|---|---|
| inexpressible state | BUD-extended piston, minecart locked by pistons, odd copper-bulb parity with no input pulse | no ordering of the vocabulary produces it | macros and generators (K) |
| non-local ordering | the fix is "place the observer before a wall built 3,000 steps ago" | backtrack is limited to provenance suspects, with a fixed budget and iterative deepening | learned constraints and global re-solve (F), clusters (E) |
| access dead end | a cell enclosed before it was placed, or a needed stand removed | witnesses and vetoes are a lookahead heuristic; pull-forward can fail | peel order (G), air placement (H) |
| arming flagged as damage | arming extends pistons into their target positions | §2 allows memory changes only on the action's own cell | one classifier rule (D) |
| budget exhaustion | the right plan exists, but is 300 trials away | 200 trials per failure, at 0.4 s each | speed (A, C) |
| random outcome | painting variant, drops when breaking temps in survival, mob movement | "randomness doesn't decide anything" (S5) | contingent steps (J), opt-in |
| never-quiet or time-critical | clocks, capping lava within ticks | not certifiable by design (§9) | stays a stuck report; D gives clocks a clean arming step behind approval |

There are two structural causes underneath the table.

1. **Failures teach the search nothing that transfers.** The memo is keyed on (action, exact state hash). Nothing learned from one failure carries to the next state or to the next copy of the same module.
2. **Repair is local and has a fixed budget.** Reorder, wrap and backtrack all work near the failure. A fix that needs a decision far back, or a different mechanism, is out of reach.

The LLM is currently the only component that generalises "why". Speed and autonomy are also coupled: with trials 100× cheaper, the same wall time buys a 100× larger repair budget.

## 2. Candidates

The table gives an overview. Each candidate is then written up with the same headings: idea and source, how it works, hard cases it addresses, guarantee, speed-up, the LLM's role, the biggest risk, and the smallest experiment.

| id | candidate | layer | attacks | guarantee vs v4.1 | planning speed (estimate) | live speed | confidence |
|---|---|---|---|---|---|---|---|
| A | In-process, tick-sprinting simulator harness | oracle | speed, provenance | unchanged | 10–100× per trial; ×N processes | none | high that it helps; size unknown |
| B | Station-sized steps where nothing can react | step granularity | certification cost, live time | same statement, coarser pause points | ~20× on the wait replay | ~3× per bot | high |
| C | Risk tiers: the cheapest sufficient check per action | search | trial count | unchanged (P2 reworded) | 5–20× fewer trials | none | medium–high |
| D | Cold build, arm last, made exact | problem shaping | redstone firing, memory | unchanged, plus one classifier rule | indirect, large | none | medium |
| E | Reactive clusters solved separately, in parallel | problem shaping | speed, locality | unchanged | ×cores on multi-cluster builds | enables I | medium |
| F | Learned ordering constraints instead of local repair | search | non-local ordering, LLM load | unchanged | ~10× fewer repair trials | none | medium (needs a pilot) |
| G | Access by peeling the finished build | problem shaping | access, scaffolding | unchanged | fewer dead ends | fewer temps | medium–high |
| H | Air placement | vocabulary | access, orientations | unchanged (policy decision) | fewer steps | fewer temps | claim unverified |
| I | Certified plan → dependency graph for many bots | execution | many bots | extended; rests on an independence model | none | ×DAG width on inert work | medium |
| J | Contingent steps for random outcomes | vocabulary | entities, drops, paintings | weaker, labelled, opt-in | none | none | medium |
| K | A library that ratchets, with the LLM writing into it | library | unreachable states, LLM load | unchanged | amortised | none | high that it helps |

### A. A fast oracle: an in-process, tick-sprinting simulator harness

**Idea and source.** Use the offline server as a library rather than a server. MCHPRS teaches the inverse lesson. It is fast because it doesn't run vanilla, which this project can't give up, so the speed has to come from the harness. Time Warp (Jefferson, 1985: optimistic parallel discrete-event simulation with rollback) is the model for speculating ahead on spare simulators.

**How it works.** A mod is loaded only into the offline jar. It is the same kind of component as v4.1's observation plugin and doesn't change game logic.

- **Ticks:** the world stays frozen, and the harness steps it. Certification replays run in sprint. `/tick freeze`, `/tick step` and `/tick sprint` exist since 1.20.3; sprint runs ticks "as fast as it can" (verified on minecraft.wiki).
- **Diffs and undo:** hooks on block-state, block-entity, scheduled-tick and entity changes log every change as it happens. A diff or an undo then costs time proportional to the number of changes, not a region scan or a snapshot.
- **Exact provenance:** neighbour-changed, shape-update and scheduled-tick calls are wrapped with a cause stack. Every change then carries the chain of cells that caused it. This answers v4.1 §9.5, and it is the input candidate F needs.
- **The fake player:** packet objects go straight to the connection handler, with no socket.
- **Parallelism:** N server processes, each holding a copy of the region. They serve speculative candidates, independent clusters (E) and certification variants. Speculation works like Time Warp: while step k's trial runs, another process trials step k+1's top candidate from the predicted state, and the result is discarded if the prediction fails.
- **Movement traces:** Azalea's physics generates them. The reality check calls it vanilla-faithful for walking, jumping and collision. The simulator checks each trace only once it is part of a trial.

**Hard cases.** None directly. It multiplies the effect of every other candidate and of the repair budget.

**Guarantee.** Unchanged. Game logic is untouched and the hooks only observe. The one risk is a hook that perturbs behaviour, for example by changing iteration order. v4.1's determinism variant already runs a second simulator, and running that one without hooks catches it.

**Speed-up.** 10–100× per trial if the 0.4 s is pacing or overhead, which is likely (§1.2), and ×N with N processes. In sprint, a plain replay of 20,000 steps × 40 ticks takes about 15 minutes at 1 ms/tick, against about 11 hours at 20 TPS.

**LLM.** No direct role. Larger budgets mean fewer stuck reports caused by budget alone.

**Biggest risk.** The vanilla tick itself may cost more than assumed: chunk ticking, lighting, entity ticks. Lighting runs asynchronously and must be drained before each diff to keep results deterministic.

**Smallest experiment.** Break one current trial down into pacing waits, IPC, snapshot and restore, ticks, and diff. Then time `/tick sprint 10000` on a loaded build region to get ms per tick. One day.

### B. Station-sized steps where nothing can react

**Idea and source.**

- *Real world:* the In situ Fabricator built a 1,600-brick wall from about 14 base positions, solving reachability once per position ([realworld scout](builder-alternatives-scouts/scout_realworld.md) §3). Human builders work the same way.
- *Pooled testing* (Dorfman, 1943, screening WWII recruits): test many items with one test, and split a pool only when it tests positive.

**How it works.** There are two kinds of step.

- **Station step.** Walk to a stand, then place every planned cell reachable from it, including its pillar and bridge temps. A station step is allowed only if every action in it is tier 1 (candidate C), meaning no reactive, sensor, fluid, gravity or entity cell in its footprint. Size is bounded by inventory and by a maximum live duration (say 20–30 s), so typically 20–200 placements.
- **Single step.** Anything near a reactive cell keeps one action per step, as in v4.1.

**In search**, the harness classifies after each action within the station's forward run. A dirty action is found in the same run and the station is split there; nothing needs bisecting. Pooled testing with prefix bisection (log₂ of the step size in extra trials) is the fallback if per-action classification inside a batch turns out unreliable.

**In certification**, nothing changes in kind. Every step is re-judged, per-tick change sequences are recorded, and the jitter and batching variants apply inside station steps too. But the fixed-point wait now runs at roughly (stations + reactive actions) boundaries, not ~20,000.

**Live**, there is one walk, one acknowledgement wait and one settle per station. The placement rate is a policy for looking human: vanilla doesn't throttle by default, since `rate-limit` in server.properties defaults to 0 ([mechanics scout](builder-alternatives-scouts/scout_mechanics.md) §9).

**In survival**, a station's items must fit the inventory, which makes station boundaries the natural restock points. This matches C1's AD-C1.2 options (a) and (c).

**Hard cases.** Speed, live and certification. It also makes reach explicit for access.

**Guarantee.** Same statement, coarser granularity.

- Pausing is safe only at step boundaries. A pause request (a player approaching, S3) may wait up to one station's duration.
- Station steps only happen where nothing in the build can react. A foreign entity arriving mid-station therefore can't trigger anything, but S5 is technically broken for that window.

Whether that's acceptable is Allen's call. Capping station duration limits the exposure.

**Speed-up.** In the reference build, about 150 stations plus 1,000 reactive actions give about 1,150 boundaries. The wait replay falls from about 24M to about 1.4M ticks, roughly 17×. Live time per bot falls about 3×: about 1.4 h of placing at 4 per second, plus about 0.6 h of station moves, plus about 0.4 h of reactive steps, against about 8 h. Search benefits little.

**LLM.** None.

**Biggest risk.** Tier-1 misclassification makes a station dirty. The cost is a split, not safety. Survival drops and inventory limits complicate what a station can hold.

**Smallest experiment.** Certify the corpus's decorative baseline both ways, then compare total certification ticks and simulated live time. Two days.

### C. Risk tiers: check each action with the cheapest sufficient check

**Idea and source.**

- *Robotic assembly:* the LEGO action-mask planner and RAP query the physics engine only when cheap geometric checks leave a real choice. RAP went from 21.6 s to 2.4 s on average ([realworld scout](builder-alternatives-scouts/scout_realworld.md) §2).
- *Planning with reduced models* (Chrpa et al., [nphard scout](builder-alternatives-scouts/scout_nphard.md) §6): trust a cheap model except where a risky-state predicate says the full one is needed.
- *Allen's own split:* placement can be emulated offline; redstone cannot.

**How it works.** For each action a, compute a static footprint F(a): the action cell plus the closure of derived-shape relations. Those relations are stairs, connectables, chests, doors, wire connections, rails, leaves distance and scaffolding distance, taken from per-version data. Each action then gets a tier.

- **Tier 1 (static).** All of these hold:
  - no cell of the signal graph (wire, diodes, components, actuators, memory cells) within 2 cells of F(a), which covers quasi-connectivity and comparators reading through a block;
  - no fluid or waterlogged cell, and no gravity block it would leave unsupported;
  - no entity and no entity support;
  - no sculk sensor within vibration range (8 cells, 16 for calibrated);
  - a bot path that touches no sensor.

  The action is accepted statically, with its recipe and derived states from the placement emulator. The search never simulates it. It is first simulated in certification replay 1.
- **Tier 2 (local).** F(a) reaches reactive cells, but its effects are bounded, for example by the cold cut (D). The action is trialled in its cluster's own small simulator (E).
- **Tier 3 (global).** Arming steps, and anything whose effects can reach an unbounded distance: an uncut power path, fluids, observer chains into the margin. It gets a full trial from the exact state, as in v4.1.

When certification finds a tier-1 step dirty:

- the action is re-tiered;
- the miss is logged as a proposed hazard-label correction (v4.1 §7.3, reviewed by a person);
- planning resumes from the last checkpoint.

**Hard cases.** Speed. Also the correctness of placement states, through the emulator.

**Guarantee.** Unchanged (§1.1). Certification replay 1 judges every tier-1 step from the exact state. P2 becomes "only certification admits a plan".

**Speed-up.** Search trials scale with the reactive fraction rather than with the whole build. At 5–15% tier 2 and tier 3 (to be measured), that is 7–20× fewer search trials. Inert actions cost only their share of the replay, which is paid anyway.

**LLM.** None. Label corrections stay with a person.

**Biggest risk.** The tier-1 rule may miss interactions (comparators reading through blocks, item frames, bells, note-block instruments, and so on). Each miss is safe but costs a re-plan, and a systematically wrong rule costs many. The emulator must also reproduce derived states exactly, or tier-1 steps fail certification on derived mismatches.

**Smallest experiment.** A static census of the fraction of tier-1 actions per corpus schematic. Then build the decorative baseline from the emulator and from the simulator and count mismatched cells. Two to three days.

### D. Build cold, arm last, made exact

**Idea and source.**

- *Industrial practice:* commissioning by energising last, and lockout-tagout ([realworld scout](builder-alternatives-scouts/scout_realworld.md), honourable mentions).
- *Hardware:* FPGA partial reconfiguration builds behind a decoupler, and metal-only ECO uses spare cells ([livesystems scout](builder-alternatives-scouts/scout_livesystems.md)).
- *Graph theory:* a minimum vertex cut between a source set and a sink set is polynomial (Menger's theorem; max-flow with node splitting).

Hard-cases §3.2 introduced "arm last" as a ranking input. Here it becomes a phase, with its own target and a classifier rule.

**How it works.**

1. **Signal graph.** Build it from the target T. Nodes are cells. Edges are any way one cell can deliver power or a pulse to another:
   - wire adjacency and diode facing;
   - strong power through solid blocks;
   - quasi-connectivity, by the wiki's rule;
   - observer outputs, and comparator side and back inputs, including reads of containers through a block;
   - sculk vibration ranges, as wireless edges.
2. **Sources and sinks.**
   - *Sources* are anything that emits without the bot acting on it: torches, redstone blocks, levers and daylight detectors in the on state, observers, sculk sensors, plates, tripwire hooks, target blocks, comparators reading non-empty containers, lecterns and trapped chests.
   - *Sinks* are actuators and memory cells: pistons, droppers, dispensers, crafters, copper bulbs, powered and activator rails, doors, trapdoors and gates, note blocks and TNT.
   - *Feedback loops are memory or clocks.* Each strongly connected component of the signal graph (torch latches, observer loops, clocks) is treated as a sink.
3. **The arming set A.** It is a weighted minimum vertex cut between sources and sinks, plus a feedback vertex set inside each loop that contains its own sources.
   - Weights are cheap for dust and repeaters with no dependants, and expensive for cells that support others, for cells deep inside the build (access cost from G), and for memory cells.
   - The cut is max-flow and takes milliseconds on 20k nodes. The feedback vertex sets are NP-hard in general but small in practice.
   - The result is the precise version of v4.1 §9.2's "leave the loop open".
4. **The cold target C.** C is T minus A, with every memory cell at its placement default (pistons retracted, bulbs unlit, repeaters unlocked). Power-derived properties are whatever the source side produces.
   - Compute it in the simulator: place T \ A with `setblock … strict` (1.21.5+, offline with op; verified on the wiki), set memory cells to their defaults, force an update pass and settle.
   - If anything on the sink side still moves, the cut leaks. Enlarge it.
5. **The cold build reaches C.** No source can reach a sink, so the remaining hazards are the update-shaped ones:
   - observers: the watched cell must be final before the observer goes in;
   - rails: lock-in, so place the curve last;
   - sculk: shield it with temporary wool, or place it last;
   - containers before the comparators that read them.

   A BUD or quasi-connectivity update does nothing to a piston when no power can reach it. Most of the cold build is tier 1 or tier 2.
6. **Arming.** Place A's cells in an order found by full trials. **This needs a classifier rule that v4.1 lacks.** An arming step may change memory cells other than its own, but only into their target states, and at the end of the step the state must equal T on every cell its provenance reached. v4.1 §2 allows memory changes only on the action's own cell, so as written it classifies every arming effect as damage, such as a piston extending into its target position.
7. **Shields and holds** are temps, trialled like any other: wool around sculk sensors, and a side-locked repeater as a macro where a cut cell can't stay open until the end. A repeater locked from the side "ignores all block updates and scheduled ticks", per the wiki via the [mechanics scout](builder-alternatives-scouts/scout_mechanics.md) §4; verify in the simulator before using it.

**Hard cases.** Redstone firing mid-build and memory latching: everything downstream of the cut is unpowered while it is built. Never-quiet builds: their loop is closed by an arming step that needs approval (C1 §4.11).

**Guarantee.** Unchanged. Arming steps are certified like any other step. The new rule changes §2's definitions and should be reviewed like one. It widens "intended" to "toward the target", which is safe because it is checked against T at the end of the step on every reached cell.

**Speed-up.** Indirect but probably the largest after A. Most redstone becomes plain placement, so tiers 2 and 3 shrink to:

- clusters around observers, rails and sculk;
- the arming phase.

Most failures that would have needed repair never happen.

**LLM.** Needed when C can't be armed into T: BUD-extended pistons, bulb parity that no input pulse sets, latches whose rest state needs history. That is exactly the unreachable-state class, which goes to macros (K). Choosing what stays unarmed is a policy question for approval.

**Biggest risk.** In logic-heavy builds (torch inverters, latches everywhere), A becomes large and arming becomes an ordering problem of its own. Access is a second risk: cut cells must still be reachable at the end, so their witnesses or peel positions must be reserved (G).

**Smallest experiment.** Compute A for each corpus machine and report |A|, how many sinks need cuts of their own, and whether C settles. Then build C and arm three machines in the simulator: the 3×3 piston door, the item sorter and the copper-bulb T flip-flop. Three to five days.

### E. Reactive clusters, solved separately and in parallel

**Idea and source.**

- *Sparse linear algebra:* nested dissection solves independent parts separately and the separator last ([learning scout](builder-alternatives-scouts/scout_learning.md) §2).
- *Sokoban solvers:* Botea et al. solve rooms locally and plan globally over rooms ([games scout](builder-alternatives-scouts/scout_games.md)).
- *Lean construction:* "commissioning by system", with entry and exit conditions per zone.

**How it works.** Once D has removed the power edges, build an interaction graph over the tier-2 and tier-3 actions. There is an edge where footprints overlap or where one action's effect reaches the other's cell. Its connected components are clusters, for example an observer line with its watched cells, a piston door, a rail loop or a sorter. The tier-1 work is the shell.

- **Global schedule.** Interleave shell stations and clusters. Each cluster gets:
  - a slot, for example "after the floor at y = 64, before the walls at y = 66";
  - a boundary context: the shell cells within radius r that exist at that slot.
- **Per cluster.** Each cluster is solved in its own small world (the cluster, its context and a stone border), in its own simulator process. The solver is v4.1's step loop plus F's learned constraints. Clusters run in parallel.
- **Repeats.** Identical clusters are solved once and replayed. Identity is a canonical hash over rotation, mirror and palette, including the context. This is v4.1 §7's module library, applied within one build where most of the gain is.
- **Composition.** Splice the cluster plans into the global order. Certification proves the composed plan. A failure at a cluster's boundary means its context assumption was wrong: fix the context and re-solve that cluster.

**Hard cases.** Redstone ordering, memory, and speed. Independent clusters can also go to different bots (I).

**Guarantee.** Unchanged. The cluster simulators are search; certification of the composed plan is the proof.

**Speed-up.** The number of clusters running at once (up to the process count) times the cheaper trials of small worlds, plus repeated modules solved once. About 10× is plausible on a 16-core machine for a build with many clusters, and about 1× for a single dense machine.

**LLM.** Same kind of work, but a stuck cluster arrives as a small self-contained world. That is a much better input than a whole-build state dump.

**Biggest risk.** A redstone computer or a bank of flying machines forms one giant cluster. Context assumptions also break when the shell's access plan changes.

**Smallest experiment.** Count clusters and their size distribution from the corpus census. One day once the census exists.

### F. Learn ordering constraints from failures instead of retrying locally

**Idea and source.** Three scouts converged on this independently:

- CEGAR (counterexample-guided abstraction refinement; Surynek's CEGAR-SAT for multi-agent path finding);
- conflict-based search, which branches on "A before B" against "B before A";
- lazy clause generation (Schutt et al.), where a failed propagator explains itself as a clause;
- the Factorio blueprint pipeline's stage-and-nogood loop.

Explanation-based learning (PRODIGY, SOAR chunking) supplies the lifting step.

**How it works.**

- **Constraint store.** It holds constraints over order literals x ≺ y, of four kinds:
  - hard ones (support);
  - access ones from G, which are disjunctive over witnesses;
  - static hazard rules: observer after its watched cell, piston after its quasi-connectivity cell, a curved rail after its neighbours, containers before the comparators that read them;
  - learned clauses.
- **Learning.** When the trial of x is dirty, exact provenance (A) gives the chain of present cells c₁…c_k that carried the damage. For example: x updated observer c₁, which pulsed through dust c₂ and c₃ into piston c₄. Learn the clause x ≺ c₁ ∨ … ∨ x ≺ c_k: x goes in before at least one link of the chain exists.
- **Lifting.** Rewrite the clause in coordinates relative to x and in block types. Add it, marked as a hypothesis, at every congruent place in the build.
- **Solving.** While the constraints are acyclic, a topological sort with priorities is enough. When learned disjunctions conflict, solve a small SAT or CP-SAT model restricted to the cluster's pairs: hundreds of literals, not 20k².
- **Resuming.** Replay from the checkpoint before the earliest step whose position changed.
- **Wraps.** Break-and-replace stays in the vocabulary. The solver may propose it as an optional action when no permutation works.
- **When the model is unsatisfiable.** First re-check the lifted clauses in its core by direct probes: build the state in the simulator with strict placement, place x, observe. Drop any clause that isn't confirmed and solve again. If it is still unsatisfiable, the stuck report is the unsatisfiable core: usually a handful of cells and clauses, each with its trial evidence.

**Hard cases.**

- Redstone ordering, including fixes far back in the plan that backtracking can't reach.
- Memory latching.
- Repeated failures across copies of the same module.

**Guarantee.** Unchanged, because clauses only order candidates.

- Over-general lifted clauses could make a solvable cluster look unsolvable. The re-check step mitigates this.
- The worst case is a false stuck report, never an unsafe plan.
- This is the same "correct or stuck" contract as v4.1.

**Speed-up.** Each distinct failure pattern costs about one trial plus solver time, where a 200-trial local search used to run. Congruent copies never fail at all. v4.1's memo, keyed on exact state hashes, transfers nothing. An order of magnitude fewer repair trials is plausible but unmeasured; the scouts' estimates are not evidence.

**LLM.** The stuck report shrinks from "the state, every plan tried with its first dirty diff, and the vetoes in force" to an unsatisfiable core. The LLM's job becomes inventing a mechanism for that core (K), not debugging a state.

**Biggest risk.** Provenance only names cells that were present. It can't name a cell whose presence would have prevented the damage, such as a shield, so learned clauses only ever push x earlier. Fixes of the form "x later, after a blocker" still have to come from static rules or from the solver's freedom.

**Smallest experiment.** Start the observer chain and the 3×3 piston door from a deliberately bad order. Count replay iterations until the order is clean, and compare with v4.1's repair trials from the same start. One to two weeks, and it needs A's provenance hooks.

### G. Access by peeling the finished build

**Idea and source.**

- *Robotic assembly:* planners often plan the disassembly of the finished product and reverse it. "Can this part come out now" is checkable in the current state; "will this placement trap something later" needs lookahead (arXiv 2310.17115 via the [realworld scout](builder-alternatives-scouts/scout_realworld.md); hard-cases §1).
- *Dwarf Fortress* shows the failure this prevents: a dwarf with no approach cancels the wall ([games scout](builder-alternatives-scouts/scout_games.md)).
- *Choosing stations* is set cover, close to the art gallery problem (place the fewest guards so every point is seen). Both are NP-hard, and greedy is within a logarithmic factor.

**How it works.** Geometry only; the peel never simulates physics.

1. Start from the full target, P = T.
2. Repeatedly pick a cell x that can be removed now:
   - some stand reaches a face of x along a clear ray, with reach alone if H is adopted;
   - that stand is reachable from the entry area within P minus x, using §2b's moves and bounded hypothetical temp pillars;
   - no cell in P rests on x.

   Remove x.
3. Among the removable cells, prefer:
   - arming cells (D) and reactive cells, so they come late in forward order;
   - cells with few remaining witnesses, which makes forward order hardest-first.
4. Reverse the removal sequence to get the forward order. When x is placed forward, the cells present are exactly P minus x at its removal time, so x's stand, ray, support and walking floor all exist. No witness bookkeeping, veto or pull-forward is needed.
5. Group consecutive forward cells by stand into stations, by greedy set cover.
6. The peel's choices become access constraints for F: "x before every cell blocking x's chosen witness". If those conflict with hazard clauses, the solver can pick another witness, since the constraint is a disjunction.

**Hard cases.**

- Access and scaffolding, including v4.1 §3.3's observer-in-obsidian case: the observer is peeled after the obsidian, so it is placed before it.
- The arming set staying reachable to the end.

**Guarantee.** Unchanged. The peel only proposes an order, and forward trials plus certification judge it. Don't run the peel as a backward *physics* search. Removal and placement are not inverses under block updates (hard-cases §1, and the learning and nphard scouts agree), so a teardown in the simulator proves nothing about the build.

**Speed-up.** One peel up front, taking seconds, replaces the incremental flood fill and veto checks done per candidate. Fewer access dead ends mean fewer repairs.

**LLM.** Most access-related stuck reports should disappear.

**Biggest risk.** Hypothetical temps in the peel must become real temp lifecycles in forward time, and their placement and removal are trialled like any other step. A greedy peel can also choose orders that hazard constraints dislike, and F then has to reconcile them.

**Smallest experiment.** Peel the corpus and three large decorative schematics. Count cells that need temps, peel dead ends, and stations. If prototype logs exist, compare with v4.1's vetoes and pull-forwards. Two to three days, no simulator.

### H. Air placement (a decision for Allen)

**Idea and source.** This is my reading of the vanilla 1.20.5+ use-item-on handler; I have not verified it against 1.21.11. The handler checks:

- that the clicked block is within interaction range: the block-interaction-range attribute plus 1.0, measured from the eye to the block's box;
- that the hit point lies within about 1.0 of the clicked block's centre on each axis;
- that no teleport is pending, and spawn protection.

It does not check that the clicked block is solid, or that there is line of sight. Air is replaceable, so clicking an air cell places the block in that cell. This fits with what exists: a client-side AirPlace Fabric mod listed for 1.20 to 1.21.11, and anti-cheat plugins that ship air-placement checks.

**How it works.** Every target cell can be clicked "on itself", with whatever face and hit point the recipe wants, from any stand in reach. There are two levels.

- **Air placement**, with no reference block.
  - *Equivalent to a temp:* placing against a temp reference and then removing it gives the same state, so every state air placement produces, legitimate play produces with one more temp.
  - *What it saves:* reference temps.
  - *Orientations:* the clicked face and hit point become free, which covers logs, hoppers, slabs, stair halves and trapdoors.
  - *What it doesn't change:* attached blocks such as torches and buttons still need their support.
- **No line of sight**, clicking through blocks. This is not outcome-equivalent, because legitimate play would first have to open a path. It is the larger shortcut and the more clearly non-human one.

Even if Allen rejects both, the planner can plan with *virtual* references and then realise them as temps through v4.1's temp machinery. That is a cleaner way to find out where temps are actually needed.

**Hard cases.** Access, and orientations that are hard to reach.

**Guarantee.** Unchanged. The simulator runs the same jar, so certification tests exactly what the server accepts.

**Speed-up.** Fewer temps mean fewer steps. With H, peeling (G) seldom needs temps, and every "no reference face" failure disappears.

**LLM.** None.

**Biggest risk.** Policy: whether it counts as "legit play", and whether admins notice. The behaviour itself is unverified on 1.21.11.

**Smallest experiment.** One hour on the offline jar. A fake player sends use-item-on at an isolated air cell with several faces and hit points, then at a cell behind a wall. Record what gets placed and in which state.

### I. Many bots: turn the certified plan into a dependency graph

**Idea and source.**

- *Multi-robot assembly:* APEX-MR orders two steps of a sequential plan only if they interfere, and dispatches a step once its predecessors are done; delays then don't matter ([realworld scout](builder-alternatives-scouts/scout_realworld.md) §1).
- *Concurrency theory:* Mazurkiewicz traces and partial-order reduction. Linearisations that only swap independent actions reach the same state ([livesystems scout](builder-alternatives-scouts/scout_livesystems.md) §1).
- *Games and construction:* Factorio's construction robots consume a plan, and lean construction gives each zone one owner.

**How it works.**

- **Per-step sets.** From the certified single-bot plan, record for each step s:
  - W(s): the cells it changed at any tick, taken from the replay;
  - R(s): the cells it reads, taken conservatively. That means the neighbours of W(s), its power and quasi-connectivity reach from the hazard labels, its stand, path and ray cells, and its vibration range if it makes vibrations.
- **Dependencies.** Steps s before t are independent if W(s) is disjoint from R(t) ∪ W(t), and W(t) is disjoint from R(s), with a margin of 2 cells (16 for vibrations). Otherwise add an edge s → t. The transitive reduction is the DAG.
- **Tasks.** A task is a station step or a whole cluster. One bot works a cluster at a time, as in AD-C1.1(a). Bots take tasks that are ready, and claims (ARCHITECTURE's claim primitive) lock R ∪ W for the task's duration.
- **Movement between tasks.** In inert corridors, with no reactive, sensor or entity-sensitive cell within body and vibration range, bots use Azalea's pathfinder closed-loop with mining off, not certified packet streams. Near reactive cells, they use certified streams as in v4.1. That uses Azalea's built-in pathfinder, as Allen prefers, and needs the replay switch ([AZALEA_REALITY_CHECK](AZALEA_REALITY_CHECK.md) §6, B1) only for certified segments.
- **Certification adds** K random linearisations and K concurrent schedules with jitter. Each must reproduce the recorded per-step states on every step's W.

**Hard cases.** Many bots.

**Guarantee.** The statement extends to "any dispatch consistent with the DAG". It is proven for the sampled schedules. For the rest it rests on the independence model, meaning the soundness of R. Precisely, it is weaker than single-bot certification by exactly that assumption. Limiting concurrency to inert stations, and to clusters further apart than their interaction radius, makes the assumption very robust, because a tier-1 step's effects are local by definition.

**Speed-up.** Planning: none. Live time divides by the DAG's effective width over the inert work. C1 §6 already expects logistics, not placement, to set the build rate at scale.

**LLM.** None.

**Biggest risk.**

- R(s) may miss something, such as a comparator reading through a block or a long observer chain. That gives a missing edge the sampled schedules may not catch.
- In survival, drops from breaking temps can wander into another bot's region.

**Smallest experiment.** Compute the DAG for a certified corpus plan. Report its width over time and its critical-path length, then run three random concurrent schedules in the simulator. Two days.

### J. Contingent steps for random outcomes (opt-in)

**Idea and source.** Planning for fully observable but non-deterministic actions (FOND). A *strong cyclic* plan covers every outcome of an action and reaches the goal under fair retries (Cimatti, Pistore, Roveri and Traverso, 2003; general knowledge, not fetched). v4.1 refuses any trial whose outcome depends on the RNG. Survival makes that common:

- drops from the bot's own breaks scatter randomly;
- a painting's variant is random;
- mobs move.

**How it works.**

- Run the step's trial with N seeds, say 32.
- Group the outcomes by what live can observe: blocks, entities by cell, and the painting variant.
- Accept the step only if:
  - every observed class is clean and inert, with intended entity changes inside C1 §4.10's tolerance;
  - live readback can tell the classes apart;
  - every class other than success has a certified recovery: pick up the drop, break the painting and retry, or reopen the cell and re-insert.
- Live, execute the step, read back the outcome and follow that branch. An outcome outside the certified classes is a divergence halt, as in v4.1.
- A static rule applies: no contingent step whose drop-scatter radius reaches wooden pressure plates, hoppers, tripwire or sculk sensors.

**Hard cases.** Entities (paintings, mobs, carts on contested rails), survival drops, and multi-slot dispensers.

**Guarantee.** Weaker, and labelled per step in the certificate. Each step is proven for every sampled outcome class, and an unsampled outcome halts. It is opt-in. Today these steps all end in stuck reports.

**Speed-up.** None; it adds coverage, not speed.

**LLM.** Less work: random steps no longer all need hand-written macros.

**Biggest risk.** Rare outcomes may never be sampled. Mob behaviour also depends on state far away (POIs, other villagers) that the simulator's copy may lack.

**Smallest experiment.** Place a painting of one chosen variant inside a frame of temps as a strong cyclic step. Count tries and check that every branch is clean. Two days.

### K. A library that ratchets, with the LLM writing into it

**Idea and source.**

- *Games and their solvers:* Sokoban solvers' macro moves, and StarCraft build-order search with macro actions.
- *LLM agents:* Voyager's library of verified skills; Silver et al., where an LLM writes one program per PDDL domain and debugs it on a few tasks; and LLM-Modulo, where an LLM generates and an external verifier checks ([learning scout](builder-alternatives-scouts/scout_learning.md) §5).

**How it works.**

- **Macros (v4.1 §6).** Seed them from community knowledge: BUD pistons, piston-placed blocks, boats and carts locked in, starting an observer clock, and entity insertion procedures (C1 §4.6).
- **Click-count states as data, not search.** These are arithmetic on clicks or pulses from a known base:
  - note pitch (25 states) and repeater delay (4);
  - comparator mode and item-frame rotation (8, or 4 for maps);
  - copper-bulb parity (a pulse count) and daylight-detector inversion;
  - crafter slots and lectern page.
- **Generators.** When a cluster's unsatisfiable core (F), or an unreachable state from a pre-build report, recurs as a family, the LLM writes a generator program for that family.
  - Input is the cluster pattern; output is an action sequence with its temps.
  - It is verified in the simulator on every instance in the build and on synthetic variants.
  - It is stored by canonical pattern and by version.
- **Hazard rules ratchet too.** Every certification miss of the tier-1 rules becomes a proposed rule, flagged automatically and approved by a person (v4.1 §7.3, C1 §4.9).

**Hard cases.** Unreachable states, entities, and whatever LLM dependence remains.

**Guarantee.** Unchanged. Every macro and every generator output is trialled and certified like any other step.

**Speed-up.** LLM time is amortised over every later build. Modules are reused.

**LLM.** This is where it belongs: once per mechanism family, not once per build.

**Biggest risk.** Generators overfit to the instances they were checked on, and the library has to be re-verified on the corpus for every version.

**Smallest experiment.** Have the LLM write a generator for one known unreachable class, the BUD-extended piston, and verify it on ten rotated variants in different contexts. Two to three days.

**A minor one: a learned ranking prior.** Train a ranker on v4.1's own logs, where every trial is labelled clean or dirty. It might cut rejected candidates 2–3× (an unverified figure from the learning scout). Defer it until logs exist, and never let it be a judge.

## 3. Three architectures compared

| | v4.1 | Compile and verify (radical) | Recommended hybrid |
|---|---|---|---|
| unit of search | each action, trialled from the exact state | none: a static plan; replays find counterexamples | tier 1: static; clusters: step loop; arming: full trials |
| failures | local repair, 200 trials per failure | learned clauses, global re-solve | learned clauses in clusters; wrap kept as an action |
| access | witnesses, vetoes, pull-forward | peel | peel; witnesses kept as a safety net during repair |
| redstone | ranking plus trials | cold cut plus clauses | cold cut plus cluster search |
| proof | about 7 replays, one action per step | same | same statement; fewer boundaries; variants in parallel |
| what the LLM sees | whole-state stuck reports | unsatisfiable cores | unsatisfiable cores, turned into library entries |
| fails where | slow; budget-limited | dense redstone: many replay iterations, and "earlier-only" clauses miss fixes | more components to build |

**Compile and verify** is the radical alternative. There is no per-action search at all:

1. A static compiler (C, D, G) emits a complete order.
2. Certification replay 1 checks it.
3. Each counterexample adds a clause (F), and the order is solved again.
4. Repeat until clean, then run the remaining variants.

It is as fast as it gets when the static model is good. On dense redstone, each counterexample costs a replay from a checkpoint, and fixes that need "later" rather than "earlier" are not found. The hybrid keeps v4.1's step loop for exactly those parts.

## 4. Recommendation

### 4.1 The architecture

```
schematic + site
   |
   v
[1 Compiler, no simulator]  per-version labels, placement emulator,
   |                         cold cut (D), peel order and stations (G, B),
   |                         risk tiers (C), clusters (E), unreachable report
   v
[2 Planner]  tier 1: accepted statically
   |         clusters: v4.1 step loop + learned constraints (F),
   |                   one small simulator per cluster, in parallel (A, E)
   |         arming: full trials with the toward-target rule (D)
   v
[3 Certifier: v4.1 §1b, statement unchanged]
   |         replay 1 also catches tier-1 mistakes -> back to 2
   |         variants in parallel, sprint ticks, fewer boundaries (B)
   v
[4 Executor]  dependency DAG over stations and clusters (I);
              Azalea movement in inert corridors, certified streams near redstone;
              contingent steps where opted in (J)

[5 Library (K)]  macros, click-count tables, generators, hazard rules
                 <- written by the LLM and by people, verified in the simulator
```

**What stays from v4.1:**

- the §1b certificate and its variants;
- the §2 classification, plus one rule;
- the §2b movement model, for certified segments;
- the §3 step loop, inside clusters;
- wrap as an action, macros, and assumptions S1–S5.

**What changes:**

- P2 and P3 are reworded as in §1.1;
- the ranking becomes the compiler;
- witnesses give way to the peel, but stay as a safety net during repair;
- the reorder and backtrack generators give way to learned constraints;
- "one action per step" gives way to station steps in tier-1 regions.

**Expected effect on the reference build (estimates).**

- **Search:** about 1,000 reactive actions at roughly 1.3 trials each, plus 200 failures at about 5 trials each, at about 20 ms per trial. That is under a minute of work, spread across processes.
- **Certification:** about 2.5M ticks in total, against about 29M, so roughly 40 minutes of simulator time at 1 ms/tick. With variants in parallel, the wait replay (about 25 minutes) is the longest single variant, unless it is split across processes from checkpoints of the plain replay.
- **Live:** about 2–2.5 hours for one bot, against about 8 hours, before adding bots on the shell.

Overall that is 10–100× in planning wall time. It is more if the current harness runs at 20 TPS, and less for a single dense machine.

### 4.2 Migration path from v4.1

**Phase 0: measure (about a week).** Experiments 1–3 in §5. Nothing in the design changes.

**Phase 1: no-regret changes inside v4.1 (2–4 weeks).**

- A: the in-process harness with provenance hooks.
- B: station steps for tier-1 work.
- C: static acceptance of tier 1, with replay 1 as the check.
- D: the toward-target classifier rule, and the cold cut as a ranking input.

v4.1's planner otherwise stays as it is. Gate: the corpus certifies with no more stuck reports than before, and planning and certification time are measured.

**Phase 2: a new core for the reactive parts (1–2 months).**

- E: clusters in parallel simulators.
- F: learned constraints replacing reorder and backtrack.
- G: the peel order seeding access.
- K: click-count tables.

Gates: stuck reports per corpus build, trials per block, and how many stuck reports the LLM still has to see.

**Phase 3: scale and coverage.**

- I: multi-bot DAGs and corridor movement.
- J: contingent steps, if Allen opts in.
- K: generators written by the LLM.
- H: air placement, if approved.

### 4.3 Decisions that need Allen

1. **Air placement and no-line-of-sight clicks (H).** Packet-legal, if the claim holds. Air placement is outcome-equivalent to using a temp; clicking without line of sight is not.
2. **Coarser pause points in inert regions (B).** A pause request may wait up to one station's duration.
3. **A labelled, weaker certificate for random outcomes (J).** Today those steps all end in stuck reports.
4. **Concurrency resting on an independence model (I).** Proven for the sampled schedules only.
5. **Rewording P2 and P3 (§1.1).** The guarantee then rests on certification alone, which §1b already says.

## 5. Experiments, ranked

| rank | experiment | decides | effort |
|---|---|---|---|
| 1 | Profile one v4.1 trial (pacing, IPC, snapshot, ticks, diff); time `/tick sprint` on a build region | A's size, and every cost estimate here | 1 day |
| 2 | Static corpus census: tier-1 fraction, clusters and their sizes, cold-cut size, peel dead ends and temps | C, D, E, G, B together, with no simulator | 2–3 days |
| 3 | Air placement and a no-line-of-sight click on the 1.21.11 jar | H | 1 hour |
| 4 | Certify the decorative baseline with station steps vs one action per step: certification ticks, simulated live time | B | 2 days |
| 5 | Build cold and arm the 3×3 piston door, item sorter and copper-bulb T flip-flop; count classifier events | D, and the toward-target rule | 3–5 days |
| 6 | Width and critical path of the DAG for a certified plan; 3 concurrent schedules in the simulator | I | 2 days |
| 7 | Learned-constraint pilot on the observer chain and 3×3 door, from a bad order: iterations vs v4.1 repair trials | F | 1–2 weeks |
| 8 | Painting variant as a strong cyclic step | J | 2 days |
| 9 | LLM-written generator for the BUD-piston family, verified on 10 variants | K | 2–3 days |

Experiments 1 and 2 decide most of the rest. If the census shows tier 1 well under 80% for typical technical builds, B and C shrink in value and E and F matter more.

## 6. Considered and rejected

| idea | source | why rejected |
|---|---|---|
| MCHPRS / redpiler as a per-action pre-filter | on the table; livesystems and learning scouts | Lists only 1.20.4; its README says placing or breaking a block resets the compiled circuit; no fidelity claim against vanilla. The bottleneck is the harness (§1.2), not redstone compute. |
| Three-valued (X) simulation for hazard screening | livesystems scout | Needs a hand-written three-valued model of every block: a redstone reimplementation, which Allen's split rules out. |
| A learned surrogate as judge; RL from scratch | learning scout | Single-tick discrete failures matter, so 99% accuracy still breaks builds. RL is starved of samples at simulator speed. |
| Placeholder materialisation (inert stand-ins, then swap) | games scout | No vanilla block is update-silent. A swap is two update events next to the sensitive cell. The useful part, a temp in an unplaced target cell, is already in v4.1 §2. |
| Chunk-level "cold" regions | mechanics scout | The bot's own presence keeps the chunk it builds in ticking, and queued ticks fire later. The scout reached the same conclusion. |
| Update suppression and crash or dupe tricks | mechanics scout (also rejected there) | Exploits, not legitimate building, and they can crash servers. |
| Redstone Experiments data pack | mechanics scout | Can't be enabled on a vanilla server without op. Its status in 1.21.11 is unverified. |
| Backward (disassembly) search in the simulator | on the table; hard-cases | Removal and placement aren't inverses under updates, so a teardown proves nothing. The geometric half is kept (G). |
| Hashlife-style or global memoisation of simulation | livesystems scout | The key must cover the whole interaction radius, so hits are rare outside repeated modules, which E already reuses. The GPU Hashlife result (1.18×) is not encouraging. |
| Sokoban-style retrograde dead-end tables | games and nphard scouts | Minecraft partial builds are nearly always recoverable by breaking, so dead ends are rare. The expense is detecting conflicts. Access dead ends are handled by G. |
| Anytime branch and bound for makespan | games scout (StarCraft) | The problem is feasibility, not optimality. Fast-forwarding through waits is already implicit. |
| Supervisory control synthesis | realworld and livesystems scouts | Needs a discrete model of each circuit type. Its useful output, forbidden partial-state patterns, is what static rules and learned clauses (F) provide. |
| Stigmergic local rules (TERMES) | realworld scout | No verified guarantee, and rule synthesis is the hard part. DAG dispatch (I) keeps the useful half: act on an observed predicate. |
| LLM planning or Tree of Thoughts | learning and games scouts | The evaluator here is already exact; the LLM is weakest at grid geometry (FLE, the Builder benchmark). Its role is the library (K). |
| Monotone stratification (CALM) as its own method | livesystems scout | It gives the same partition as the tiers and cold cut (C, D). Kept as the rationale, not as a component. |
| Modulo scheduling for periodic parts | learning scout | Narrow. Within-build module reuse (E) covers periodic units. |
| Reversible pebbling for scaffolding | nphard scout | Gives lower bounds only, and reach and line of sight aren't pebble rules. |
| Invariant-preserving intermediate states (Raft joint consensus) | livesystems scout | Already v4.1's "every step inert and clean". No new mechanism. |
| Packet batches per tick to shrink the timing space | mechanics scout | Live can't pin packets to a server tick, so v4.1's batching variant is still needed (§7). |
| Pipelining planning with live building | not raised by scouts | It would hide planning time, but Allen decided against it in v2 ("only a certified full plan"). Deferred, not rejected: the trade-off is time to first block, not throughput. |
| Learned ranking prior | learning scout | Not rejected, deferred: worthwhile once v4.1 logs exist (§2, minor). |

## 7. Scout claims: corrections and open items

**Wrong or overstated.**

1. **"A minimum arming set is NP-hard"** ([nphard scout](builder-alternatives-scouts/scout_nphard.md), idea 2 and "evidence on the ideas"). Half wrong.
   - Cutting all source-to-sink paths is a minimum s–t vertex cut, which is polynomial (Menger's theorem; max-flow with node splitting).
   - Only opening feedback loops (a feedback vertex set inside strongly connected components) is NP-hard, and those components are small. D uses both.
2. **Placeholders "that send and receive no NC updates"** ([games scout](builder-alternatives-scouts/scout_games.md), top 5 item 2). No such vanilla block exists: placement and removal both notify neighbours.
   - The follow-on claim is wrong too: "deterministic update order means a swap is checked once per neighbourhood". Determinism doesn't give locality, because power arrives from far away. Locality has to come from the cold cut (D).
3. **APEX-MR's graph or partial-order reduction "replaces the seven certification replays"** ([realworld](builder-alternatives-scouts/scout_realworld.md) §1, [livesystems](builder-alternatives-scouts/scout_livesystems.md) §1). Overreach.
   - Those replays test the gap between the simulator and live: seeds, chunk order, jitter, batching, the fixed point and isolation. An ordering graph doesn't touch any of that.
   - A DAG adds orderings to certify (I); it doesn't remove certification.
4. **Per-tick packet batches "shrink the timing-perturbation space"** ([mechanics scout](builder-alternatives-scouts/scout_mechanics.md) §9 and top 5 item 5). This doesn't follow.
   - Packets handled between two ticks do run, together with their immediate update chains, before the next tick's scheduled ticks; I believe that is right.
   - But live can't control which inter-tick window a packet lands in. That is why v4.1 certifies random batching.
5. **"The permanent memory is usually a stuck-on source, not the repeater"** ([games scout](builder-alternatives-scouts/scout_games.md), Part 2). Half right.
   - `locked` follows the side input and recovers.
   - What a lock holds is the repeater's output while it is locked.
   - v4.1 lists `locked` as a memory property. That is conservative: it costs search, not safety.

**Unverified claims this document relies on:**

- **Air placement and no-line-of-sight clicks on a vanilla 1.21.11 server (H).** This is my reading of the handler, plus indirect evidence (a client-side mod, anti-cheat checks). Experiment 3 settles it.
- **The breakdown of the 0.4 s per trial, and ms per tick in sprint (A, §1.2).** Experiment 1 measures both.
- **Azalea's physics matching the server closely enough to generate traces (A, I).** The reality check calls it vanilla-faithful for walking, jumping and collision. Certification catches mismatches either way.
- **The side-locked repeater as a hold (D, step 7).** From the wiki via the mechanics scout. Use it only as a macro after testing it in the simulator.

**Verified for this document:**

- `/tick freeze`, `/tick step` and `/tick sprint` exist since 1.20.3, and sprint runs ticks as fast as possible ([minecraft.wiki](https://minecraft.wiki/w/Commands/tick)).
- `setblock … strict` places "as-is without triggering block updates and shape updates", since 1.21.5 ([minecraft.wiki](https://minecraft.wiki/w/Commands/setblock)).

**Unverified scout claims, not relied on.** All from the [mechanics scout](builder-alternatives-scouts/scout_mechanics.md):

- dust next to an observer not waking it;
- the placement trigger for copper bulbs;
- villagers and leads;
- the default simulation distance;
- Redstone Experiments in 1.21.11.

## 8. Sources

**Scout reports** (Haiku models, web research; read as leads, not facts):

- [Real world, robotics, commissioning](builder-alternatives-scouts/scout_realworld.md)
- [Games and their solvers](builder-alternatives-scouts/scout_games.md)
- [NP-hard and planning](builder-alternatives-scouts/scout_nphard.md)
- [Changing live systems safely](builder-alternatives-scouts/scout_livesystems.md)
- [Vanilla mechanics](builder-alternatives-scouts/scout_mechanics.md)
- [Compilers, synthesis and learning](builder-alternatives-scouts/scout_learning.md)

**Project documents:**

- [`ONLINE_BUILDER.md`](../ONLINE_BUILDER.md) (v4.1)
- [`builder-hard-cases.md`](builder-hard-cases.md)
- [`C1-builder.md`](../C1-builder.md)
- [`AZALEA_REALITY_CHECK.md`](AZALEA_REALITY_CHECK.md)

**Checked for this document:**

- `/tick`: https://minecraft.wiki/w/Commands/tick
- `/setblock` and `strict`: https://minecraft.wiki/w/Commands/setblock
- The AirPlace mod page (client-side; listed for 1.20 to 1.21.11): https://modrinth.com/mod/airplace

**Primary sources cited through the scouts:**

- APEX-MR: https://arxiv.org/pdf/2503.15836
- LEGO action masking: https://arxiv.org/pdf/2408.10162
- RAP: https://arxiv.org/pdf/2009.09408
- Assembly by disassembly: https://arxiv.org/pdf/2310.17115
- In situ Fabricator: https://arxiv.org/pdf/1701.03573
- CEGAR-SAT for multi-agent path finding: https://arxiv.org/abs/2301.08687
- Lazy clause generation: https://arxiv.org/abs/1009.0347
- Reduced models (Chrpa et al.): https://arxiv.org/pdf/1905.09355
- Factorio blueprints: https://arxiv.org/html/2310.01505v1
- Botea et al., Sokoban: https://webdocs.cs.ualberta.ca/%7emmueller/ps/botea-sokoban.pdf
- Generalised planning with LLMs (Silver et al.): https://arxiv.org/abs/2305.11014
- LLM-Modulo: https://arxiv.org/abs/2402.01817
- Voyager: https://arxiv.org/abs/2305.16291
- MCHPRS: https://github.com/MCHPR/MCHPRS
- minecraft.wiki: Block update, Repeater, Tutorial:Quasi-connectivity, Reach, Server.properties

**General knowledge, not fetched this session:**

- Dorfman (1943), pooled testing.
- Menger's theorem and max-flow vertex cuts.
- Jefferson (1985), Time Warp.
- Cimatti, Pistore, Roveri and Traverso (2003), strong cyclic planning.
- Mazurkiewicz traces.
- The art gallery problem and greedy set cover.
- The vanilla use-item-on handler's checks (H).
