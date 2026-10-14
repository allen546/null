# Scout report: planning, construction and automation games, and their solvers

Scope: Factorio, Dwarf Fortress, Oxygen Not Included, Zachtronics, Sokoban and block-pushing, StarCraft build orders, Minecraft building tools and agents, and a few sandbox builders. Each idea below is tied to the Minecraft builder problem in problem.md. Items marked "unverified" were seen only in a search snippet or README, not in the full source.

## Part 1: What the other games do

### Factorio: ghosts and construction robots

- A player's plan is stored as entity ghosts. A ghost is a placed plan object that holds settings (for example, an assembler's recipe) which take effect only when the real entity is placed. Ghosts can be placed at any distance. Robots build a ghost only when materials are available in the network and enough robots are free. (Source: https://wiki.factorio.com/Entity_ghost)
- The game never exposes a half-built machine to the simulation. A ghost is inert plan data, and a real entity appears only after the build step completes. Robots do not "partially" place a working machine. This is inferred from the ghost model, not stated on the wiki page.
- Robots and ghosts decouple planning from execution. The plan can be arbitrarily large, and execution can stall without corrupting anything. A stuck build shows up as "missing materials," not as wrong behavior.
- Ghosts have a time limit (one week of game time for ghosts left by destroyed entities). They also clear up on their own when the real build finishes.
- Community tips on build order ("build the input side, then test, then the output") are heuristics from players. I did not find a documented, enforced safe build order for circuits. Factorio has no update-order physics, so the problem of circuit safety does not exist in the same form.

Mapping to Minecraft: Factorio's split between an inert plan and atomic materialization is the strongest analogue for the owner's "build cold, arm last" idea. It supports a stronger version: build every hazardous cell as a non-reactive placeholder, then swap each to its final form in a safe order (see Idea 2 below).

### Factorio research: planning and layout search

- **Factorio Learning Environment (FLE)**, arXiv 2503.09617 (NeurIPS 2025 Datasets and Benchmarks). LLM agents write Python against a 23-method API for a real Factorio server. On lab-play throughput tasks, Claude averaged 21.9% (+-1.3) success and completed 7 of 24 tasks. GPT-4o averaged 16.6%. About 56% of steps in successful lab-play runs produced execution errors. Main failure modes: placing entities too close together or on top of each other, leaving no room for connections, and debug loops (GPT-4o repeated one wrong API call 78 times in a row).
  - Mapping: this is the same failure the owner sees with LLM debugging stuck reports. Spatial reasoning fails before logic does. Keep the LLM off the geometry path.
- **Towards Automatic Design of Factorio Blueprints**, arXiv 2310.01505. Frames blueprint generation as a constraint problem (Essence Prime, solved via Savile Row). Because a single model blew up on continuous item rates, the authors use a three-stage, Benders-inspired decomposition: (1) choose recipes and counts, (2) bin-pack assemblers and inserters, (3) route belts. If a stage is unsatisfiable, the previous stage generates a new candidate, and past candidates are stored so they are not regenerated. The paper gives no quantitative results in the parts I could read.
  - Mapping: staged decomposition with nogood-style feedback (Idea 3).
- **The Factory Must Grow**, arXiv 2102.04871. Defines a "logistic transport belt" problem and compares parallel simulated annealing, quick genetic programming, and evolutionary RL on 3x3, 6x6 and 12x12 grids. Simulated annealing found solutions only on 3x3 grids. The best method (qGP) failed on 6x6 with obstacles. The authors estimate the 12x12 search space at 9!^(144). Lesson: generic metaheuristics over raw cells collapse with size, which is the owner's situation at 20,000+ cells.

### Oxygen Not Included and Dwarf Fortress

- **Oxygen Not Included.** The priority system orders errands by priority level (very high down to disabled), then sub-priority (9 down to 1), then a hidden tie-break (or proximity if enabled). The wiki page does not cover construction blocking or build-trap ordering. Low value for this problem.
- **Dwarf Fortress.** The wiki (v0.34 "Micromanaging tricks") says construction orders run last-in-first-out, so the newest order is built first. Your fix is to cancel and re-issue an order to raise its priority. The more relevant entry is "Dwarf cancels construction of Wall: Creature occupying site." A builder that has no legal approach path can stand on the site it is building, and the wall is cancelled. The wiki calls this mainly a blocking and access problem, not an ordering problem. Fixes are to submit awkward tiles first, steer approach paths, or open an alternate route.
  - Mapping: DF confirms that in a building game, access (where the builder can stand) is the failure, not just order. This matches the owner's "access" hard case. Unverified: I did not confirm the exact bug number or patch behavior.

### Zachtronics and block-assembly puzzles

- **Opus Magnum, SpaceChem, Infinifactory, EXAPUNKS.** I could not find a published solver or solver paper for any of these in the search results. Treat any solver claim as unverified. The design lessons (cost metrics, hard-constrained layouts) are not separately useful here.

### Sokoban, block pushing and Baba Is You

- **Botea et al., "Power Plan" and tunnel macros** (https://webdocs.cs.ualberta.ca/%7emmueller/ps/botea-sokoban.pdf). Key tools:
  - Macro moves: sequences that the search treats as one move, which collapse a subtree into one step.
  - Room and tunnel abstraction: each room is solved locally, and the global search plans over which stone moves between rooms.
  - Deadlock detection by retrograde analysis per room: positions with no path to a goal are marked dead. The paper reports speedups of roughly 80x to 10,000x over tunnel Sokoban on a few problems (Table 2). Rolling Stone solves about two-thirds of the 90-problem suite. The authors call the work preliminary. Preprocessing of large rooms is the bottleneck.
- **Baba Is You.** The search results gave only general material, including a 2024 ICML workshop paper on LLM agents for Baba Is You. I did not find a solver with numbers. Unverified.
- **Lemmings.** Only general planning papers appeared. Nothing concrete.

Mapping: Sokoban is the closest formal match to "stones that cannot be unplaced." Deadlock detection by backward reasoning from the goal is what the owner needs for access and redstone deadlocks (Idea 4).

### StarCraft build orders

- **Churchill and Buro, "Build Order Optimization in StarCraft," AIIDE 2011.** Depth-first branch and bound with admissible lower bounds (a landmark bound from tech-tree prerequisites, plus a resource bound). It is anytime, so it can stop and return the best plan so far. A random legal plan gives the initial upper bound. The state is resource-based rather than a raw build-order list. It fast-forwards time to the next executable action, so null waits never appear as search nodes. Macro actions, such as repeated production, shorten the search depth. Reported results: with macro actions, the 90th-percentile makespan was about 1.00 to 1.08 times the professional plans. CPU time was about 0.02% to 10% of makespan (source: https://ojs.aaai.org/index.php/AIIDE/article/view/12435, full text at the PDF link).
- **StarCraft II build order with deep RL and MCTS**, arXiv 2006.10525. MCTS with UCB1 on a 5-action abstract state. Scores plateau near 90 to 94 (novice-level) after 2 to 10 iterations. Random rollouts are weak evidence for planning in this setting. The authors propose a learned value network, which is untested.

Mapping: Churchill's landmark bounds and fast-forwarding transfer directly to a builder that orders cells and waits for timers. The landmark bound is a dependency lower bound on the number of steps, and the owner's dependency graph (supports before torches, access before walls) can supply it (Idea 5).

### Sandboxes: Space Engineers, Stormworks, Besiege, KSP

- Space Engineers has a projector that shows a hologram of a blueprint. It is an inert plan that is materialized by welding. Unverified beyond the wiki search snippet.
- I did not find a source on Stormworks, Besiege or KSP that addresses build-then-arm ordering. The general pattern is "staging and action groups," which is a trigger mechanism, not a construction ordering. Low value.

## Part 2: Minecraft-specific evidence

### Redstone and update order (verified against the Minecraft Wiki)

- **Block updates are deterministic and fixed.** When a block is placed, destroyed or changes state, it sends post-placement (PP) updates to its six neighbors in the order west, east, north, south, down, up. Most blocks also send neighbor-changed (NC) updates in the order west, east, down, up, north, south. The wiki says updates follow depth-first order. (Source: https://minecraft.wiki/w/Block_update.)
  - Implication: given the same placement sequence, the outcome is reproducible. That supports a static analysis of update effects and a deterministic simulator. It also weakens any "random" framing of hazards.
  - The page notes exceptions (pistons, redstone wire, beds, shulker boxes and others) and a self-update bug, which I did not fully read.
- **Repeater locking is reversible.** A repeater is locked when a powered repeater or powered comparator faces its side. A locked repeater holds its output and ignores block updates and scheduled ticks. It unlocks when the side source turns off. Lock and input changing on the same tick can prevent a lock from taking effect. (Source: https://minecraft.wiki/w/Repeater.)
  - Correction to problem.md: the "permanent" memory is usually a stuck-on source, not the repeater. Find the source and the state is recoverable. This matters for the reset and repair plan.
- **Quasi-connectivity** (pistons, droppers, dispensers) was not covered in the page I read. Unverified.

### Printers and builders

- **Litematica printer** (https://github.com/IceTank/litematica-printer and forks). The README describes placement by range, rotation handling, an air-place option (described as "a bit buggy") and a tick delay setting. It states a wrong-block rate of about 1 in 1000, higher at high ping. The README does not describe any ordering or hazard model, so it is effectively greedy build-and-hope. I did not read the placement source code. Evidence that printers do not solve ordering.
- **Baritone** builder and **Autobuild** (https://www.curseforge.com/minecraft/mc-mods/autobuild, alpha). Search results gave only repository commit pages and a mod page. I did not verify the builder's placement order or scaffolding logic. Unverified.

### Research agents

- **Voyager**, arXiv 2305.16291. Three parts: an automatic curriculum (GPT-4 picks the next task), a skill library (verified code, stored by embedding, top 5 retrieved), and iterative prompting with environment feedback and a GPT-4 critic. Reported gains over MineDojo baselines: 3.3x more unique items, 2.3x longer distance, up to 15.3x faster tech-tree milestones. The critic is an LLM, which is weaker than the owner's simulator certification. The skill-library idea transfers well (Idea 1).
- **Minecraft Builder benchmark** (arXiv 2407.12734). LLM builders place blocks from text instructions. Table 1 (zero-shot vs. chain-of-thought): absolute addressing 42.98 vs 76.5; relative addressing 82.02 vs 95.8; primitive shapes 59.02 vs 60.3. The model confused axis signs without CoT. The paper does not describe any placement ordering or verification. Lesson: LLMs are unreliable at grid geometry even with CoT, which supports keeping them out of the geometry path.

## Part 3: Ranked top 5

1. **Certified gadget library (macro actions with interface contracts).** Pre-certify small redstone and access modules in the simulator, once each, with declared assumptions about neighbors and declared effects. A schematic is then planned over gadgets, not cells. This is Sokoban's macro moves plus Voyager's verified skill library plus Churchill's macro actions. It cuts search depth by the module size. Speedup is large for repetitive schematics (door arrays, clock chains, piston banks), and close to zero for unique builds. The main cost is writing interface contracts. Confidence: high that it reshapes the search. Unverified for Minecraft specifically.

2. **Placeholder materialization (plan inert, then swap).** Build every hazardous cell first as a non-reactive placeholder block (for example, a plain block that sends and receives no NC updates), then replace each with its final redstone form in a safe order. The Factorio ghost model is the precedent. Because update order is fixed and deterministic (Minecraft Wiki), each swap's effect can be checked once per placeholder neighborhood, not per trial. This turns "build cold, arm last" into a complete method: the arming set becomes the swap set, and each swap is verified against a known neighborhood. Caveat: a swap is still an update event, so the swap order needs its own check. Confidence: medium-high. Speedup is large if most hazards are local.

3. **Benders-style decomposition with nogood learning from simulator failures.** Split the problem into a master (choose a cell order or a gadget order using abstract dependency and hazard constraints) and a subproblem (the simulator certifies one candidate). Each failed trial returns a nogood: the minimal set of (partial state, action) pairs that caused the surprise, which the master must avoid. The Factorio blueprint paper uses this exact loop (stage fails, previous stage regenerates, past candidates stored). Today's local repair search does not remember failures across trials, so it repeats them. This directly addresses the owner's "slow" complaint and reduces LLM debugging. Confidence: medium-high. Speedup is unknown until the nogood extraction cost is measured.

4. **Retrograde deadlock pre-pass (backward search from the goal).** Sokoban's room-level retrograde analysis gives a method to precompute which partial states are unrecoverable. For the builder, compute from the schematic which placements make some other cell unreachable or unsupportable (access deadlock), and which redstone states cannot be reached by clicks (state deadlock), by searching backward from the finished schematic. This is a stronger version of the known "static pre-pass classifying each cell by hazard type": it classifies states, not just cells. The Sokoban paper reports large speedups, but that comes from a different state space. Confidence: medium. Preprocessing cost on 20,000+ blocks is the risk.

5. **Anytime branch and bound with admissible landmark bounds.** Churchill and Buro give a template: depth-first branch and bound, a dependency-landmark lower bound (the owner's support and access dependencies), resource bounds, and an anytime output. Fast-forwarding removes waiting nodes, so the search only branches on events that change state. For a multi-bot build, the output can be a partial, provably-best-so-far plan that is safe to execute, which is a different guarantee from v4.1's "correct or stuck." Confidence: medium. Expect limited gains unless the bound is tight.

## Cross-cutting notes

- **LLM role.** FLE and the Minecraft Builder benchmark both show that LLMs fail on spatial reasoning and produce long debug loops. The owner should keep the LLM for choosing gadgets and reading failure reports, and keep geometry in deterministic code.
- **Simulator role.** The deterministic update order means an offline, fast, exact update model (MCHPRS-style) is a good filter, but only for the update-order part. Entity and access checks still need the real server or a faithful model. Unverified how far MCHPRS covers survival-only block behavior.
- **Claims to check against the owner's data.** The "memory is permanent" claim should be checked case by case, because repeater locks recover (Minecraft Wiki). Piston and rail states may still be permanent.
- **Gaps.** I did not find solver papers for Zachtronics games, Baba Is You or Lemmings, nor usable Baritone placement logic. Those remain unverified.

## Sources

- https://wiki.factorio.com/Entity_ghost
- https://arxiv.org/html/2503.09617v1 (FLE)
- https://arxiv.org/html/2310.01505v1 (Factorio blueprints)
- https://arxiv.org/pdf/2102.04871 (The Factory Must Grow)
- https://dwarffortresswiki.org/index.php/v0.34:Micromanaging_tricks
- https://www.dwarffortresswiki.org/index.php/Dwarf_cancels_construction_of_Wall:_Creature_occupying_site
- https://oxygennotincluded.wiki.gg/wiki/Priority
- https://webdocs.cs.ualberta.ca/%7emmueller/ps/botea-sokoban.pdf
- https://ojs.aaai.org/index.php/AIIDE/article/view/12435
- https://arxiv.org/pdf/2006.10525
- https://minecraft.wiki/w/Block_update
- https://minecraft.wiki/w/Repeater
- https://github.com/IceTank/litematica-printer
- https://arxiv.org/html/2305.16291v2 (Voyager)
- https://arxiv.org/pdf/2407.12734 (Minecraft Builder benchmark)
