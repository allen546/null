# Scout report: compilers, program synthesis and learning for ordering and construction

Scope: alternatives to slow per-action simulator search guided by an LLM debugger, for the Minecraft schematic builder described in problem.md. Method: about 17 web searches and about 17 page fetches. Several fetches returned only arXiv abstracts, so details marked "unverified" come from memory or were not checked. Hard-case labels used below: **R** = redstone ordering, **M** = memory states, **A** = access and scaffolding, **E** = entities, **B** = multi-bot interference, **S** = speed.

Speed estimates are guesses unless a source is cited. The current baseline is about 0.4 s per simulator trial, thousands of trials, and about 7 certification replays per build.

---

## 1. Ordering as SAT/ILP with CEGIS-learned lemmas

**Idea.** Encode "which cell goes in at which time" as a constraint problem. Feed the solver the static constraints: support before dependent, access and line-of-sight, and "do not place X while Y is empty". Solve for an order. Run the real simulator on that order. If something diverges, extract the smallest set of placements that caused it, add a clause forbidding that pattern, and re-solve. This is the counterexample-guided inductive synthesis (CEGIS) loop. The solver does the global search; the simulator only checks and produces counterexamples.

**Source.** CEGIS: Solar-Lezama et al., "Combinatorial sketching for finite programs" (ASPLOS 2006), and Solar-Lezama's 2008 thesis on sketching (well-known, not fetched). Oracle-guided and counterexample-guided synthesis modulo theories: Jha, Gulwani, Seshia and Tiwari (CAV 2010, cited in search results, not fetched).

**Mapping.**
- **R, M, A.** Each counterexample is a local event, such as "piston B fired because cell C was placed while cell D was empty". That gives a clause about a few cells and their ordering, so the solver stops revisiting it.
- **Generalisation (explanation-based learning).** If the clause is written over canonical block types and relative geometry rather than absolute coordinates, it applies to every place where the same pattern recurs in the structure, and to later builds. This is the same idea as explanation-based learning in PRODIGY and SOAR chunking, which turn a solved case into a rule (sources in search results, not fetched).
- **Soundness condition.** A learned clause is only sound if the pattern's radius of influence is captured. Redstone signals travel up to 15 blocks, so the pattern must include that context or be rechecked. Otherwise the solver will confidently prune valid orders, or accept invalid ones.
- **Encoding size.** Pairwise ordering variables for 20,000 cells are too many. Constraints must be restricted to pairs within an interaction radius, plus support edges.

**Estimate.** A SAT call on a sparse, local encoding should take milliseconds to seconds, versus 0.4 s per trial. If learned lemmas transfer well, the number of full simulator runs could drop from thousands to tens. This is unverified. Note that UNSAT under learned lemmas is only a proof relative to those lemmas, not relative to the true physics.

**Verdict.** Changes the shape of the solution (global search moves into a solver with a growing lemma store). Most promising of the set.

---

## 2. Separator-based divide and conquer ("arm last", applied recursively)

**Idea.** Build an interaction graph over cells or components. Put an edge between any two cells that can influence each other: adjacency, support, redstone signal range, or bot access. Find a balanced separator S. Solve each side independently and in parallel. Place S last. Recurse inside each side. This is nested dissection, the standard fill-reducing ordering for sparse factorisation, which uses separators to split a graph into independent parts.

**Source.** Nested dissection (George, 1973) and planar separator theorems (Lipton, Rose and Tarjan, 1979). Wikipedia "Nested dissection" was found in search but not fetched. Parallel sparse ordering literature (e.g. ParMETIS, SCOTCH) was also found in search and not fetched.

**Mapping.**
- **B.** Independent subgraphs can be assigned to different bots with no interference by construction. This is the most direct answer to multi-bot interference.
- **A.** S is exactly the "arming set" in the "build cold, arm last" idea, generalised to a hierarchy. The hierarchy also tells you the order in which scaffolds can be removed.
- **R.** Redstone circuits with long signal paths make the graph dense, which hurts separators. The fix is to coarsen each self-contained circuit into a single super-node, solve it as a module (idea 3), and only build separators over the super-nodes.

**Estimate.** Parallel speedup is bounded by how balanced the separators are. On 3D grids the separator size grows roughly as n^(2/3), so the top-level separator for a solid 20k-block build could be large. The method should be strong for modular builds (houses, machine blocks separated by walls) and weak for dense solid builds. Real schematic graph structure is unverified and would need measuring.

**Verdict.** Strong for B and for scheduling parallel bots. Formalises a known idea rather than replacing it.

---

## 3. Module memoisation: canonical hashing, pattern databases and cached simulator results

**Idea.** Solve small redstone modules (repeater chains, piston doors, observer clocks, rail loops, sculk setups) offline and exhaustively against the real vanilla server. Store each result as a verified macro, keyed by a canonical form of the module: block types, relative orientation, and the class of its neighbours. Look up keys online. On a hit, reuse the order after a cheap local check. On a miss, search. More generally, cache exact simulator outputs keyed by a canonical local context. This turns "the simulator as a judge" into "the simulator as a memo".

**Source.** Korf, "Finding Optimal Solutions to Rubik's Cube Using Pattern Databases" (AAAI 1997; listed in search results, not fetched). Felner, Korf and Hanan, "Additive Pattern Database Heuristics" (JAIR 2008, arXiv 1107.0050; fetched). Their abstract describes disjoint partitions whose costs sum admissibly, using static partitions for all states or dynamic partitions per state, with the best choice depending on the domain. The fetched abstract gave no speedup numbers.

**Mapping.**
- **M and E.** Memory states and entity setups recur across a structure (rows of identical piston doors, repeated minecart rails). Caching lets you solve each distinct module once.
- **S.** Hit rate decides the gain. Repetitive builds could see large reductions in trials. Unique builds would see almost none.
- **Additive PDBs for pruning.** If the global cost (number of wraps or failed trials) decomposes over disjoint modules, the sum of per-module costs is an admissible lower bound. That allows branch-and-bound pruning in the global search. Felner et al.'s static versus dynamic partition distinction maps onto "fixed module boundaries" versus "boundaries recomputed per state".

**Caveat.** Minecraft context dependence is strong: a piston's memory state depends on neighbours that may be far away. The canonical key must include the full influence radius, and every hit still needs a cheap verification.

**Estimate.** An order of magnitude fewer trials on repetitive builds; roughly none on unique ones. Unverified.

**Verdict.** Cheap to build, and it composes with ideas 1 and 2.

---

## 4. Periodic structures: iterative modulo scheduling

**Idea.** For a repeated unit, schedule one period with an initiation interval (II), meaning the offset between successive units, and then replicate it. The loop-carried dependencies and the bot resource limits are the constraints. Iterative modulo scheduling (IMS) is a backtracking heuristic that searches for the smallest feasible II.

**Source.** Rau, "Iterative Modulo Scheduling: An Algorithm for Software Pipelining Loops" (MICRO-27, 1994). The fetched abstract confirmed only the framing: a practical algorithm for realistic machine models, judged on schedule quality and compute cost. Details of the II search and the backtracking budget are unverified.

**Mapping.**
- **S and B.** Solve once per unit type, not once per instance. Resource constraints map onto bot count and reach.
- **R.** Periodic redstone (clocks, repeater trains, staircase piston sets) is precisely the case where the order of one period can be reasoned about in isolation.
- **Limit.** Only helps where the build is periodic. Unique structures get no benefit.

**Estimate.** Large gain on periodic builds, no gain elsewhere. Unverified.

**Verdict.** A narrow but clean tool, and a good fit for the module solver in idea 3.

---

## 5. LLM as one-shot program writer, verified on samples (amortising LLM cost)

**Idea.** Instead of the LLM debugging each stuck build, have it write, once per module family, a generator program that emits a valid order for any instance. Check the generator against a few instances in the simulator, feed failures back to the LLM, and then run the generator on every new instance. Store verified generators as a library of macros, as Voyager does with executable skills. The simulator still certifies each instance. LLM debugging then happens at the generator level, not at each build.

**Source.**
- Silver et al., "Generalized Planning in PDDL Domains with Pretrained Large Language Models" (arXiv 2305.11014, fetched). GPT-4 writes a Python program that solves a whole PDDL domain. Automated debugging against training tasks was reported as very important. Often two training tasks sufficed for strong generalisation. The paper covers seven domains, and the specific success rates were not in the fetched text.
- Kambhampati et al., "Position: LLMs Can't Plan, But Can Help Planning in LLM-Modulo Frameworks" (ICML 2024, arXiv 2402.01817, fetched abstract). Auto-regressive LLMs cannot by themselves plan or self-verify. Use them as generators inside a loop with external model-based verifiers.
- Voyager (Wang et al., arXiv 2305.16291, fetched). An ever-growing library of executable skills, built by iterative prompting with environment feedback and self-verification. Reported 3.3x more unique items, 2.3x longer distances and up to 15.3x faster tech-tree progress versus the prior state of the art (Minecraft survival, not building).
- PlanBench (Valmeekam et al., arXiv 2409.13373, fetched). The authors say o1 is a "quantum improvement" on the benchmark but "still far from saturating it". This is evidence against direct LLM planning.
- MineAnyBuild (arXiv 2505.20148, fetched). Best zero-shot MLLM overall score was 41.02 out of 100 (GPT-4o). The authors name an "implementation gap": text plans often fail to convert into valid blueprint matrices, with errors in indexing and orientation. This is evidence for why LLM-written blueprints need verification.
- APT (arXiv 2411.17255, fetched). An LLM agent with memory and reflection built complex Minecraft structures, including redstone systems. A memory ablation helped significantly. The abstract gives no quantitative metrics.

**Mapping.**
- **All hard cases.** The LLM contributes rules and generators rather than per-build reasoning, which directly targets the owner's complaint about LLM debugging.
- **M and R.** Generators can encode hazard-avoiding patterns (e.g. arm this piston last) once per family.

**Estimate.** Amortises LLM cost over every build of a family. Generator correctness on new instances is unverified and will need simulator checks. The main risk is that generators written from a few samples overfit.

**Verdict.** Strong, and it fits the owner's stated budget constraint. Not a solver by itself.

---

## 6. Learned priors and surrogates used only to rank candidate actions

**Idea.** Learn a ranking over candidate actions (a policy prior or a safety classifier) to order the simulator trials. The simulator still makes every accept or reject decision. A learned surrogate of the simulator is not used as the judge.

**Source.**
- Sanchez-Gonzalez et al., "Learning to Simulate Complex Physics with Graph Networks" (arXiv 2002.09405, fetched). Learned graph-network simulators generalised to rollouts of thousands of steps and to at least 10x more particles. Those are continuous-physics results.
- Graph Transformer for LEGO assembly sequence planning (arXiv 2210.05236, fetched). Trained to imitate brute-force search sequences, it reached a Kendall tau of 0.44 against ground truth. The authors call that medium correlation.
- AssemblyComplete (arXiv 2410.15469, fetched). RL with an action mask removing collisions and stability violations. No numbers in the abstract.
- Brick-by-Brick (arXiv 2110.15481, fetched). A learned validity predictor screens actions before an actor-critic chooses one.
- Learning from demonstrations in Minecraft (arXiv 2003.06066, found in search, not fetched). Sample-efficiency details unverified.

**Mapping.**
- **Against a learned surrogate as judge.** The physics here is discrete and exact, and the failures that matter are single-tick events. A surrogate that is right 99% of the time will still break builds. So it can screen, but not decide.
- **Against RL from scratch.** At 0.4 s per trial, RL is sample-starved: a million episodes is about 4.6 days on one simulator instance. RL only becomes practical on a cached or compiled simulator (idea 3), or offline on logs.
- **For a ranking prior.** A Kendall tau of 0.44 suggests a ranker could still cut trials meaningfully by putting likely-good actions first. The gain is unverified, perhaps 2-3x in trials.
- **Free training data.** v4.1 already produces labelled accept and reject examples for every candidate it tries. A classifier trained on those logs is cheap and does not need new simulation.

**Verdict.** Useful as a search accelerator, not as an alternative to certification.

---

## Evidence on the known ideas

- **"Build cold, arm last".** Supported by the separator logic in idea 2 and by the CEGIS view in idea 1, where arming cells are the ones whose ordering constraints are most often violated. The known idea is a one-level version of the nested dissection hierarchy.
- **Static pre-pass hazard classification.** Aligns with the canonical neighbourhood keys in idea 3 and the lemma store in idea 1. The pre-pass should output the canonical classes, so it doubles as a cache key.
- **Disassembly planning (teardown then reverse).** Weak. Mechanical assembly sequences are reversible, but Minecraft block updates are not symmetric: removing a block fires neighbour updates of its own, and a piston's memory state cannot be undone by reversing its placement. This is reasoning, not a sourced result. The LEGO assembly sequence work (arXiv 2210.05236) uses forward sequences and shows only medium agreement with search.
- **MCHPRS-style compiled redstone simulation.** The fetched README shows MCHPRS targets Minecraft 1.20.4 in creative mode, not 1.21.11 survival. It also warns that placing or breaking blocks while Redpiler runs resets and disables the compiled state, so it can only be used on completed sub-circuits (offline, as in idea 3), not per placement. It makes no accuracy claim against vanilla.
- **Tree of Thoughts.** The abstract reports 74% on Game of 24 with ToT versus 4% with GPT-4 chain-of-thought (arXiv 2305.10601, fetched). But the value comes from LLM-generated and LLM-evaluated thoughts. Here the exact simulator is already the evaluator, and the costly part is the simulator itself. Low value for this problem.
- **Litematica printers, Baritone, WorldEdit type-staged ordering.** Not researched in this pass. No evidence gathered.

## Honourable mentions (not researched in depth)

- Sethi-Ullman and list scheduling (critical-path priority) for scaffold budgeting. Sethi-Ullman is optimal for expression trees, so it fits tree-shaped supports such as roofs, but register need is NP-hard for general DAGs. A cheap pre-pass lower bound on open scaffolds is the realistic use. Standard results, not fetched.
- Stochastic superoptimisation (STOKE, ASPLOS 2013): search over instruction sequences with a cheap cost function, then formal certification. Structurally the same as v4.1 (search, then certify), so it offers little new, but it is a reference for keeping the cheap cost function separate from the final check. Not fetched.
- Search did not turn up any published work on learned ordering for Minecraft redstone specifically. The queries on redstone contraption generation returned only spam. Treat this area as unexplored.

---

## Consolidated top 5

1. **SAT/ILP ordering with CEGIS-learned pattern lemmas (idea 1).** Moves the global search into a solver and makes each simulator counterexample reusable, which attacks R, M and A at once and cuts simulator calls. Main risk: encoding size and unsound generalisation, both checkable.
2. **Separator-based decomposition, nested-dissection style (idea 2).** Formalises "arm last" recursively and gives independent subproblems for parallel bots (B). Strong for modular builds; weak for dense solid ones.
3. **Module memoisation with canonical keys and cached exact simulation (idea 3), plus modulo scheduling for periodic units (idea 4).** Solves each repeated module once and reuses it. Large gains on repetitive builds, none on unique ones.
4. **LLM as one-shot generator, verified on samples, stored as a macro library (idea 5).** Moves LLM cost from per-build debugging to per-family generation, which answers the owner's main complaint. Generator quality on new instances needs simulator checks.
5. **Learned ranking prior trained on v4.1's own accept and reject logs (idea 6).** Cuts simulator trials without giving up certification. Gain unverified, likely modest. Learned surrogates are rejected as judges because single-tick discrete failures matter.

Sources: [LLM-Modulo, arXiv 2402.01817](https://arxiv.org/abs/2402.01817); [PlanBench o1, arXiv 2409.13373](https://arxiv.org/abs/2409.13373); [Silver et al. generalized planning, arXiv 2305.11014](https://arxiv.org/abs/2305.11014); [Tree of Thoughts, arXiv 2305.10601](https://arxiv.org/abs/2305.10601); [Voyager, arXiv 2305.16291](https://arxiv.org/abs/2305.16291); [APT, arXiv 2411.17255](https://arxiv.org/abs/2411.17255); [MineAnyBuild, arXiv 2505.20148](https://arxiv.org/html/2505.20148v3); [AssemblyComplete, arXiv 2410.15469](https://arxiv.org/abs/2410.15469); [Brick-by-Brick, arXiv 2110.15481](https://arxiv.org/abs/2110.15481); [LEGO assembly graph transformer, arXiv 2210.05236](https://arxiv.org/abs/2210.05236); [Additive PDB heuristics, arXiv 1107.0050](https://arxiv.org/abs/1107.0050); [Iterative Modulo Scheduling, MICRO-27](https://american.cs.ucdavis.edu/Micro27/html/abst7.html); [Graph network simulators, arXiv 2002.09405](https://arxiv.org/abs/2002.09405); [MCHPRS](https://github.com/MCHPR/MCHPRS); [Nested dissection](https://en.wikipedia.org/wiki/Nested_dissection); [CEGIS modulo theories (Jha et al.)](https://www.springerprofessional.de/counterexample-guided-inductive-synthesis-modulo-theories/15968656).
