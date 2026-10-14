# Scout report: real-world construction, robotic assembly, commissioning

Scope: how engineers and robots sequence work where order matters and partial states are dangerous. About 25 searches and page reads. Items marked **unverified** are from a search snippet or a paper summary I did not check against the full text.

Sources were read through summaries. Where a claim is central, I note what the summary did not confirm.

---

## 1. Conflict-derived precedence graph as the deadlock certificate (APEX-MR)

**Idea.** APEX-MR (Multi-Robot Asynchronous Planning and Execution for Cooperative Assembly, https://arxiv.org/pdf/2503.15836) plans the task sequence first and then turns it into a temporal plan graph (TPG). Same-robot order edges are added, plus inter-robot edges wherever a collision check between two nodes says they interfere. Each interference edge points from the node that comes earlier in the sequential plan to the later one. This keeps the graph acyclic. The authors claim execution is deadlock-free if and only if the TPG has no cycles. At run time, a node is dispatched only when all its predecessors have finished. Timestamps are used only to build the graph, so arbitrary execution delays are tolerated.

**Mapping to the builder.** This replaces the current certification step. Instead of seven clean replays under timing perturbation, the planner would derive, for every pair of bot actions, whether they interfere: one bot's access path or reach crosses another's footprint, or one action changes a block that another reads. Each interfering pair gets an order edge taken from the sequential plan. Deadlock becomes a cycle check on a graph, and any dispatch order consistent with the graph is safe by construction. This directly targets the "many bots" hard case and the timing-sensitivity of redstone. The reported gains (in simulation on nine LEGO tasks) were 36 to 48 percent less makespan and 77 to 85 percent less waiting time, which are their numbers, not mine.

**Caveat.** The collision relation in APEX-MR is geometric motion collision. In Minecraft, interference is at the level of block updates and neighbour reads, so the conflict relation must come from a dependency model (which cells each action reads and writes, including neighbours within the redstone propagation radius). That model is not free to build, and an under-approximated one makes the certificate wrong. The paper also does not cover failures or recovery.

**Estimate.** Large for the multi-bot and certification parts (I estimate it could remove most of the seven certification replays and most waiting between bots). Small for single-bot redstone ordering, which it does not address.

---

## 2. Cheap structural action masks and a geometric pre-filter before any simulation

**Idea.** Two related sources.

- Physics-aware combinatorial assembly sequence planning with data-free action masking (LEGO, https://arxiv.org/pdf/2408.10162). An action is only allowed if it passes five masks: task (inside target), collision-free, inventory, operability (the brick needs a free top or bottom face so it can be inserted vertically), and stability. The stability mask uses an optimisation-based force-balance check, not a physics engine. Results: a policy with full masks reached 100 percent on four test sets, against 30 to 92.7 percent for a heuristic mask. Inference was about 0.2 s per step, against about 15 s per step for MCTS with the same masks. The authors note that mask computation takes 60 to 70 percent of training time.
- Robust Assembly Planning (2D, https://arxiv.org/pdf/2009.09408). A non-directional blocking graph over translation directions finds feasible partitions geometrically. The physics simulator is queried only when the geometry leaves a real choice. Average time was 21.58 s for the simulator-only baseline against 2.38 s for the method, and about 1.4 s on tasks where the simulator was never queried. The method is planar and gravity-free, so it transfers in shape but not in detail.

**Mapping to the builder.** The current design calls the simulator on every candidate action. Most candidates are obviously illegal for structural reasons that can be checked in microseconds: no exposed approach face for the click; the cell is not within reach of any station; the placement would enclose an unplaced cell; the bot's standing cell would be walled in. A mask layer of this kind would cut the simulator calls, which are the dominant cost at 0.4 s each. The operability mask is the direct analogue of "no clickable face," and the blocking-graph idea gives a fast test for "can this cell still be reached from outside after I place this block." The learned-classifier half of RAP is less useful here because the Minecraft rules are exact and cheap to evaluate.

**Estimate.** I expect a 3 to 10 times reduction in simulator calls from static masks alone, with the simulator still certifying what remains. This is an inference from the ratios above, not a measured result. The main risk is a mask that is too strict and blocks a legal but unusual route.

---

## 3. Frontier (growth) invariant that keeps every placement reachable and supported

**Idea.** The hierarchical lattice paper (https://arxiv.org/pdf/2510.13686) grows the structure from a feed point. Each new voxel rests on a voxel in the layer below, so a reachable surface always exists for the next robot step. The robot path planner runs A* that avoids the partly built structure. The paper's safeguards are the growth order, the connectivity check, and a realignment routine. It does not discuss deadlock explicitly. A brick-wall case study, "Mobile Robotic Fabrication at 1:1 scale: the In situ Fabricator" (https://arxiv.org/pdf/1701.03573), does the same at a coarser level: it splits the wall into patches, each built from a fixed robot base position, so reachability is solved once per station.

**Mapping to the builder.** For ordinary (non-redstone) parts of a schematic, the builder could impose a monotone invariant: place a cell only when it is adjacent to a built cell in a designated direction (for example upward or away from a source), and only when the cell's approach face is outside the built region. Reachability and support then hold by construction and need no search. For redstone the invariant is not enough, because the danger there is update-ordering, not access. The station idea is still useful: assign each bot a fixed station, with its reach set precomputed, and let each bot own the cells its station can reach. This removes most access conflicts between bots. The Gramazio/Kohler paper also reports that sequencing was driven by a plan generated before building, with the base repositioned about 14 times for 1,600 bricks. That is a reasonable model for planning station moves.

**Estimate.** Large for the access problem on static parts (I would expect the search to shrink to the redstone portion). Small for the hazard problem.

---

## 4. Counterexample-guided ordering repair (CEGAR-style)

**Idea.** The sequential 3D printing packing and scheduling paper (https://arxiv.org/pdf/2503.05071) solves a relaxed problem first, checks the result for violated pairwise non-collision constraints, adds only the specific violated constraints (conditioned on print order), and re-solves. The authors report that this lazy refinement beat encoding every constraint up front, because violations are rare. Their optimality claim holds only for the approximated (2D) model.

**Mapping to the builder.** Right now the repair step is local search (reorder, wrap, backtrack) on the live plan. A counterexample-guided version would work at the level of the plan's ordering. When the simulator finds that action A breaks action B, the planner records the pair as an ordering constraint (A before B, or B before A, whichever the sequential plan needs), and then re-derives the ordering with those constraints added. The constraint set grows only with observed conflicts, so it stays small. The ordering problem itself can be solved by topological sort or a small SAT or ILP, which is fast. This also turns the LLM's job from "debug a stuck report" into "inspect a conflict pair," which is narrower and checkable. It fits with idea 1: the TPG's edges are exactly the constraints this loop learns.

**Estimate.** Moderate. It cuts repair search and makes repairs explainable. It does not reduce the number of simulator calls needed to discover each conflict, so its gain is in the repair loop, not in the first pass.

---

## 5. Supervisory control: forbidding dangerous partial states with a safety supervisor

**Idea.** Supervisory control theory (Ramadge-Wonham and its Petri-net forms) builds a supervisor that forbids states from which uncontrollable events can lead into a forbidden state. The Petri-net synthesis paper by Alla (https://arxiv.org/pdf/0904.0586) finds the "border" forbidden states (reached from safe states by a controllable step), reduces them to linear constraints on place markings, and merges them to a small covering set. It is structural rather than search-based. Its limits, which the summary confirmed: it needs safe, conservative nets; the state space can be huge (one example quotes about 31 million states for four workstations and four zones); the minimum constraint set is a covering problem with no complexity bound; and the paper does not claim nonblocking or deadlock freedom.

**Mapping to the builder.** The builder's hard cases map onto the theory cleanly. Bot actions are controllable. Redstone propagation, piston timing, gravity and fluids are uncontrollable. Latched memory states are exactly "a sequence of uncontrollable events that reaches a bad state." The useful output is a set of forbidden partial-state patterns (for example, "a repeater locked by an unfinished neighbour" or "a piston face exposed with its power source absent"), each checked as a constraint on the current world. The planner would then reject any action whose result matches a forbidden pattern before calling the simulator. The value depends on whether one can derive the patterns from a discrete abstraction of redstone (one per circuit type). Each circuit type would need its own abstraction. This is a long-term formal route rather than a fast win.

**Estimate.** Uncertain. If a per-circuit abstraction can be built, it could prune much of the simulator search in a way that is checkable. As it stands, I rate the transfer as speculative (**unverified** that the abstraction is feasible).

---

## Honourable mentions (weaker, or with caveats)

**Stigmergic local-rule builders (TERMES).** Werfel, Petersen and Nagpal (Science 2014, https://del.seas.harvard.edu/publication/designing-collective-behavior-termite-inspired-robot-construction-team) describe robots with no central controller and no direct communication. Each robot acts on the brick configuration it senses, which is a stigmergic coordination. The Harvard summary says the rules are generated automatically from a desired structure and "guarantee production of that structure." The claim in the common brief that the system provably builds any structure without deadlock is **not supported** by the pages I could read. The Harvard news page explicitly does not claim it, and the Wikipedia summary says only that structures "emerged." Treat the guarantee as **unverified**. The useful transfer is the pattern: each bot picks its next action from a predicate over the currently observed world, and a well-founded potential function would give a termination argument. Deadlock-freedom would still have to be shown per structure. Estimate: useful for robustness to bot failure, but rule synthesis is the hard part and I could not verify that it is easy.

**Dead-end-aware value for search (S2A).** The subassembly-to-assembly paper (https://arxiv.org/pdf/2409.13620) trains a learned Q-function with a delayed penalty for steps that lead to a state with no feasible continuation. Success on 7-part problems rose from 16.7 percent (heuristic) to 84.7 percent with beam width 5. This is a way to predict "stuck" states before they happen, so the planner could avoid them. It needs a training set per domain. Its reported real-world demonstration was a Jenga tower, which is a small example.

**Hardest-first ordering with pre-fixed waypoints (timber assembly).** Huang et al. (https://web.mit.edu/yijiangh/www/papers/Timber_Assembly_SCF2021.pdf) found that ordering reachability-hard movements first, and fixing a few tool-station configurations, raised success on one difficult element from 0 percent (linear solver) to 32.4 percent (nonlinear solver with ordering), and then higher with fixed configurations. They also report that unmodelled temporary clamps caused repeated collisions. The lesson for the builder is concrete: model temporary scaffolding explicitly, and place the hardest-to-reach cells first while access is still open.

**Make-ready and conditions of satisfaction (Lean construction).** The Lean Construction Journal article (https://leanconstruction.org/lcj_20_007) describes what construction planners do. Each trade or zone defines what must be complete before entry and what standard its work must meet on exit. The next crew confirms the zone is ready before entering. Commissioning is planned by system, not by location. Crews that share a location have one owner, and the other works on backlog. This is a sound pattern for zones of the schematic (one redstone circuit per zone, with entry and exit conditions), and "commissioning by system" matches the suggestion to verify each circuit as a unit. I did not find detailed sequencing algorithms in the summary.

**Energize-last and lockout/tagout.** I searched for energisation sequencing and commissioning practice, but the results were generic safety SOPs. I did not find a method or algorithm to cite. The principle "build inert, verify, then energise, and lock out regions while work is in progress" is standard and supports the "build cold, arm last" idea already on the table. The specifics are **unverified**. A lock per region with an explicit owner is a simple mutex and is worth copying into the multi-bot design.

**Not found or not read.** Hadrian bricklaying sequencing: I found product descriptions only, with no sequencing method. Aerial or drone construction: no usable source found. Modular volumetric construction: a crane-lift path-planning paper exists (ISARC 2021) but I did not read it. Support-free and toolpath printing (https://arxiv.org/html/2408.09198v1): a learned ordering of nodes that works from a starting node, with restarts when stuck. Its overhang and collision handling is limited, so I rate it low for this problem.

---

## Consolidated top 5

Ranked by how much each changes the shape of the solution, with an honest estimate of the gain.

1. **Conflict-derived precedence graph as the deadlock certificate (idea 1).** It replaces sampled timing replays with a graph property that can be checked directly, and it matches the multi-bot problem. Gain: large for certification and multi-bot waiting. Main risk: the conflict relation must be exact, so it must be built from a read/write dependency model that covers redstone neighbours.

2. **Structural action masks and a geometric pre-filter before simulation (idea 2).** This removes most candidate actions before the 0.4 s simulator call. The mask set (approach face, station reach, enclosure, support) comes straight from the real-world analogues. Gain: estimated 3 to 10 times fewer simulator calls (inference from published ratios, not measured here).

3. **Monotone frontier invariant and fixed bot stations for static parts (idea 3).** Reachability and support hold by construction for ordinary blocks, and bots get precomputed reach sets. Gain: large for access on non-redstone parts, with the search left mostly for the redstone portion.

4. **Counterexample-guided ordering repair (idea 4).** Each simulator-found conflict becomes a pairwise order constraint, and the ordering is recomputed with a small solver. This replaces ad hoc reorder and wrap steps and makes stuck reports checkable. Gain: moderate, in the repair loop rather than the first pass.

5. **Supervisory control with forbidden partial-state patterns (idea 5).** This is the most formal route to guaranteed pruning of dangerous states, especially latched memory states. The transfer needs a per-circuit abstraction of redstone that I could not verify is feasible. Gain: uncertain and long-term.

Honourable mentions: stigmergic local rules (TERMES; the "any structure, no deadlock" guarantee is unverified), a learned dead-end predictor (S2A), and hardest-first ordering with fixed waypoints (timber assembly).

---

## Source list

- APEX-MR: https://arxiv.org/pdf/2503.15836
- Physics-aware assembly sequence planning with action masking: https://arxiv.org/pdf/2408.10162
- Robust 2D assembly sequencing (RAP): https://arxiv.org/pdf/2009.09408
- Subassembly-to-assembly (S2A) with dead-end penalty: https://arxiv.org/pdf/2409.13620
- Optimal robotic assembly sequence planning (MDP, assembly-by-disassembly): https://arxiv.org/pdf/2310.17115
- Hierarchical discrete lattice assembly: https://arxiv.org/pdf/2510.13686
- In situ Fabricator: https://arxiv.org/pdf/1701.03573
- Timber assembly (SCF 2021): https://web.mit.edu/yijiangh/www/papers/Timber_Assembly_SCF2021.pdf
- Sequential 3D printing packing with CEGAR: https://arxiv.org/pdf/2503.05071
- Supervisory control synthesis (Alla): https://arxiv.org/pdf/0904.0586
- TERMES and Werfel et al. 2014: https://del.seas.harvard.edu/publication/designing-collective-behavior-termite-inspired-robot-construction-team ; https://seas.harvard.edu/news/robotic-construction-crew-needs-no-foreman ; https://en.wikipedia.org/wiki/Termite-inspired_robots
- Lean construction (LOB, takt, make-ready): https://leanconstruction.org/lcj_20_007
- 4D BIM spatial reasoning (Weldu and Knapp 2012, abstract only): https://repository.lsu.edu/mechanical_engineering_pubs/1482
- Toolpath planner for 3D printing: https://arxiv.org/html/2408.09198v1
