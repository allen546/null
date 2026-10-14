# Scout report: changing a live system safely

Domain: hardware, circuits and software where partial states must not glitch. Method: about 20 web searches and 14 fetches (sources below). Anything I could not confirm in a fetched source is marked **unverified**.

## What the sources actually say (evidence that constrains the ideas)

- **MCHPRS redpiler is not a live incremental simulator.** The README (github.com/MCHPR/MCHPRS) says placing or breaking a block resets Redpiler and disables it, and that `--optimize` "can, and will, break the state of your build." The docs file (docs/Redpiler.md) was not fetched, so this is README-level evidence only. It weakens the on-table idea of using MCHPRS as a per-action pre-filter: unless recompilation per action is cheap, it fits only for checking whole arming configurations.
- **SAT-synthesised redstone ignores ticks.** The Petersen redstone-from-SAT write-up (alloc.dev, Jan 2026) encodes static connectivity and power, and explicitly does not model ticks or update order. Its only ordering device is an acyclicity constraint (dust IDs lower than adjacent torches) to rule out unwanted memory. So static acyclicity is the cheap proxy the field already uses for ordering.
- **Railway interlocking verification covers adversarial timing but not reconfiguration.** The Swansea/Westrace SAT paper checks safety over reachable transitions with arbitrary inputs, so the controller cannot refuse any input. It explicitly does not model layout or program changes, listing that as future work. The conclusion for us: the field has no standard method for safe changes; the closest good method is in consensus protocols (idea 5).
- **Partial-order reduction works, but its numbers are preliminary.** Dynamic POR (Flanagan and Godefroid, POPL 2005) explores a persistent set of transitions and, on small benchmarks, collapses to a single path. The authors call the results preliminary.
- **Order-free does not mean easy.** Abelian sandpiles have a final configuration and toppling counts that are independent of toppling order (Dhar). The same Wikipedia page notes that halting problems for three-dimensional abelian sandpiles are undecidable. Order-independence is a property to check, not a shortcut.
- **Monotone steady states are order-free.** Chaotic-iteration results (Cousot and Cousot, 1977; **unverified** from memory) say that a monotone system on a finite lattice reaches the same fixed point under any fair update order. That is why wire levels settle independently of order, while non-monotone parts do not.
- **Robust timing-independent design is expensive.** Delay-insensitive minterm synthesis (Wikipedia) is robust but "large and slow."
- **Hashlife speedups are not automatic.** A GPU study (fpgacpu.ca) found that a direct port of Hashlife gained only 1.18x over a plain GPU Game of Life, and that it was memory-bound.
- **Expand/contract keeps every stage compatible with the previous one** (Pete Hodgson's write-up): expand, backfill, migrate reads, contract. Each stage must remain valid on its own.
- **FPGA partial reconfiguration isolates the region with decouplers** "in an orderly manner" (arXiv 2408.08626). The paper gives no formal sequence and no safety argument; it is empirical.

## Ideas

### 1. Commutation graph and partial-order reduction (rank 1)

**Idea.** Two candidate actions are independent if, from the same state, doing them in either order gives the same result and neither enables or disables the other. Compute that relation from local pair simulations rather than from the whole world. Then a search only needs one order per equivalence class of the dependence relation (persistent or sleep sets, as in DPOR). If the local rewriting also terminates and is locally confluent (every pair of single steps can be rejoined), Newman's lemma gives a unique final state, so order does not matter at all.

**Source.** Newman's lemma (en.wikipedia.org/wiki/Newman%27s_lemma); Flanagan and Godefroid, "Dynamic partial order reduction for model checking software," POPL 2005 (cs.columbia.edu/~junfeng/17sp-e6121/papers/dpor.pdf).

**Mapping.**
- *Redstone ordering:* a non-commuting pair gives a precedence edge. The planner becomes a graph problem: build the dependence DAG, schedule in topological order, and cut cycles with an arming set (a feedback vertex set on the cells).
- *Multi-bot:* actions in the same antichain of the DAG can run concurrently. This is the most direct multi-bot scheduling output of any idea here.
- *Speed:* pairwise tests use cached local simulations within the interaction radius. The 7 certification replays become one check per trace class.
- *Caveats:* scheduled ticks (repeaters, observers, gravity blocks) break the instant-update picture, so any pair inside a delay window must be treated as dependent. Pairwise independence misses three-body effects, so a final global replay stays.

**Estimate (unverified).** If most action pairs in a typical build are independent (plausible for wiring and solid blocks), simulator calls could drop from thousands of full trials to hundreds of cached local calls plus one certification. In dense redstone clusters, where most pairs interact, the gain is small.

### 2. Monotone / non-monotone stratification (rank 2)

**Idea.** CALM (Hellerstein and Alvaro, "Keeping CALM: When Distributed Consistency is Easy," arXiv:1901.01930): a computation can run without coordination exactly when it is monotone, meaning that adding inputs never retracts outputs. Classify each component port as monotone (it only adds support or power, e.g. a solid block, or wire levels computed as max-minus-one over neighbours) or non-monotone (it inverts, delays or latches: torches, comparators, pistons, observers, repeaters).

**Source.** arXiv:1901.01930; chaotic-iteration result as above (**unverified** citation details).

**Mapping.**
- *Redstone ordering:* ordering constraints apply only to the non-monotone cells. Monotone cells can go in any order.
- *Multi-bot:* bots place monotone parts with no coordination. Coordination (turn-taking, locks) is needed only at non-monotone strata, the same way stratified negation works in Datalog.
- *Memory:* the non-monotone set is exactly where the memory hazards live (stuck pistons, locked repeaters, toggled bulbs). That gives a useful sanity check on the classification.
- *Caveat:* monotonicity is a property of each port, not of the whole component. A torch takes a monotone dust input but its output is antitone; classify per port.

**Estimate (unverified).** This shrinks the ordering problem to the non-monotone subset. If wiring and solid blocks make up most of a 20,000-block schematic, the search space shrinks a lot. It does not remove timing certification for the non-monotone cells. It overlaps with the on-table "static pre-pass," but gives that pass a theorem-backed criterion rather than a heuristic one. It composes with idea 1: stratify first, then run commutation analysis inside the non-monotone stratum.

### 3. Three-valued (X) abstract simulation for one-pass hazard screening (rank 3)

**Idea.** Eichelberger's ternary simulation (1965; **unverified** from memory; topic confirmed by the title "Hazard detection in logic simulation," lume.ufrgs.br/handle/10183/126715): inputs whose timing is unknown are set to X. If an output that must be determined stays X, a hazard is possible. Soundness: a value that is determined in three-valued simulation is determined under every ordering of the unknown updates. So one abstract pass over-approximates all orders at once.

**Mapping.**
- When a candidate action is screened, mark every cell whose update time is not fixed relative to the action as X, run one abstract tick, and reject the action if a memory-holding cell (piston, repeater, bulb, rail) output stays X. Accepted actions still go to the real simulator.
- *Redstone and memory:* an X on a stateful cell is precisely a potential latch, so the check targets the silent, permanent error type.
- *Speed:* one pass instead of thousands of 0.4 s trials per screening step. The simulator is still the authority.
- *Caveats:* X-pessimism rejects some safe actions, so the screen is conservative. Each block type needs a hand-written three-valued model, which is error-prone and must be validated against the real simulator.

**Estimate (unverified).** If the abstraction is precise enough, the screening stage could be one to two orders of magnitude cheaper. If it is coarse, most candidates will be rejected and the gain disappears.

### 4. Memoised local simulation with early cutoff (rank 4)

**Idea.** Store the result of simulating a local region as a function of its input neighbourhood, and reuse it whenever the same neighbourhood recurs. If a re-simulated region produces the same boundary output, stop propagating (early cutoff). Mokhov, Mitchell and Peyton Jones classify this as constructive traces in "Build Systems à la Carte" (microsoft.com/en-us/research/uploads/prod/2018/03/build-systems-a-la-carte.pdf). Salsa (github.com/salsa-rs/salsa) does demand-driven incremental recomputation of the same kind. Hashlife memoises a quadtree of cellular-automaton space-time.

**Mapping.**
- *Speed:* schematics repeat motifs (clock cells, adders, staircases), and every placement invalidates only its own neighbourhood. The planner's trial-and-undo becomes cache lookups.
- *Caveats:* the key must cover the full interaction radius (redstone signals travel 15+ blocks) and every input that affects the result: tick phase, scheduled ticks, entity state, and possibly bot position.
- *Evidence against naive versions:* the Hashlife GPU result above, and the redpiler README stating that any placement resets the compiled circuit. Caching at the neighbourhood level sidesteps the second problem, because nothing is recompiled globally.

**Estimate (unverified).** Large for schematics with much repetition, little for unique builds.

### 5. Invariant-preserving intermediate states (rank 5)

**Idea.** Raft joint consensus (cockroachlabs.com/blog/joint-consensus-raft/): to change membership from C_old to C_new, the cluster goes through a joint configuration in which decisions need a majority of both. A direct swap lets two groups act independently and split-brain. The one-server-at-a-time alternative is safe at each step, but its intermediate state can fail badly if one region goes down. Apply the same rule to the builder: any transition between two designs (removing scaffolding, replacing a component, the undo-then-redo "wrap" in repair) must satisfy the safety invariant of both the old and the new configuration at every intermediate state, and must stay safe if one bot dies at that step.

**Mapping.**
- *Access and scaffolding:* the scaffold removal order must keep the build safe with a bot missing.
- *Multi-bot:* a dead or delayed bot is simply another input the intermediate invariant must survive.
- *Stuck repair:* the wrap move gets a checkable criterion, so the LLM no longer has to judge repair moves heuristically.

**Estimate (unverified).** No speedup. This is mainly a robustness and repair-quality gain.

## Honourable mentions

- **Completion detection (delay-insensitive design).** A bot treats a step as done only when its local world reads as settled. Dual-rail DIMS is robust but "large and slow," so use this only at non-monotone cells.
- **Region decoupling (FPGA partial reconfiguration).** Build a region behind a gate the live circuit cannot see (a solid block or a deliberately unpowered cell) and connect it last. The decoupler paper gives no formal order, so treat this as a practice, not a guarantee.
- **Expand/contract for scaffolding.** Scaffold placement is "expand," removal is "contract," and each removal must leave a valid intermediate. This names standard practice rather than adding an algorithm.
- **Metal-only ECO.** Pre-placed spare cells whose function is set by a late change are a close analogue of "build cold, arm last." Only the title is confirmed ("Functional ECO Using Metal-Configurable Gate-Array Spare Cells," ir.lib.nycu.edu.tw/handle/11536/134709); the mechanism is **unverified**.
- **Supervisory control.** Ramadge-Wonham synthesis yields the most permissive controller that keeps bots out of bad states under uncontrollable events such as ticks and physics. It is heavy, and I could only confirm its setup from the Wikipedia page.
- **Reset-first discipline.** Give every stateful cell a reset path placed before arming, so every memory element is known before the build is armed (power-on reset). This is a design rule, not an algorithm.

## Ranked top 5

1. **Commutation graph and partial-order reduction (idea 1).** It changes the shape of the search from enumerating orders to a graph over pairs. It gives a certificate type (trace equivalence) and a direct multi-bot schedule (antichains). Its cost is local pair simulation, which is cacheable. The main risk is that ticks may make most pairs dependent.
2. **Monotone / non-monotone stratification (idea 2).** It decides which cells need ordering at all, and it puts the coordination cost only where memory hazards live. It is the strongest candidate for shrinking the problem, and it composes with idea 1.
3. **Three-valued screening (idea 3).** A sound one-pass abstraction turns sampled trials into a single conservative check. It is the best speed lever for the candidate screen, but it needs block-level models and is only as good as their precision.
4. **Memoised local simulation (idea 4).** It is the most practical speed gain for repetitive schematics, and it avoids the redpiler's reset problem by working at neighbourhood scale. Its value depends almost entirely on how repetitive the schematics are.
5. **Invariant-preserving intermediate states (idea 5).** It gives the stuck-repair loop and scaffold removal a checkable criterion and a multi-bot robustness requirement, but it does not speed anything up.

## Sources

- Newman's lemma: https://en.wikipedia.org/wiki/Newman%27s_lemma
- Abelian sandpile model: https://en.wikipedia.org/wiki/Abelian_sandpile_model
- Flanagan and Godefroid, DPOR: https://www.cs.columbia.edu/~junfeng/17sp-e6121/papers/dpor.pdf
- CALM, "Keeping CALM": https://arxiv.org/abs/1901.01930
- Build Systems a la Carte: https://www.microsoft.com/en-us/research/uploads/prod/2018/03/build-systems-a-la-carte.pdf
- Salsa: https://github.com/salsa-rs/salsa
- Hashlife GPU study: https://fpgacpu.ca/writings/HashLife-GPU.pdf
- MCHPRS README: https://github.com/MCHPR/MCHPRS
- Redstone from SAT: https://alloc.dev/2026/01/09/redstone_from_sat
- Railway interlocking SAT verification: https://csrail.swansea.ac.uk/Pubs/pdf/avocs10.pdf
- Raft joint consensus (CockroachDB): https://www.cockroachlabs.com/blog/joint-consensus-raft/
- Delay-insensitive minterm synthesis: https://en.wikipedia.org/wiki/Delay-insensitive_minterm_synthesis
- Supervisory control theory: https://en.wikipedia.org/wiki/Supervisory_control_theory
- FPGA partial reconfiguration: https://arxiv.org/pdf/2408.08626
- Expand/contract (Pete Hodgson): https://blog.thepete.net/blog/2023/12/05/expand/contract-making-a-breaking-change-without-a-big-bang/
- Metal-configurable spare-cell ECO: https://ir.lib.nycu.edu.tw/handle/11536/134709
- Hazard detection in logic simulation (topic only): https://www.lume.ufrgs.br/handle/10183/126715
