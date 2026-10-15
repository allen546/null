# Generalization paper test, run 3: goal-state decomposition

> **2026-10-15.** Third paper run on the generalization idea. Paper only: nothing ran, no server, no code. Nothing here is adopted. Runs 1 and 2 asked how much framework code new tasks add ([run 1](GENERALIZATION_PAPER_TEST.md), [run 2](GENERALIZATION_PAPER_TEST_2.md)). This run asks a different question: if a task is written as a **goal** (conditions on world state plus constraints) and decomposed by search over the low-level operators, how much of the decomposition does search cover, and how much falls to the LLM or to coded skills? It uses the same ten tasks as runs 1 and 2 (T1 to T5, T6 to T10), as they are defined there. Game facts come from vanilla 1.21.x as remembered, not from F5, so each one is marked `[c]`. One author counted, and that author has read the proposal. The user's skepticism is tested as stated: this is search in an open, changing world, and much of it will fall back to the LLM.

Tags used below. GEN is [GENERALIZATION.md](GENERALIZATION.md). P1 and P2 are runs 1 and 2. WT is [WORLD_TRANSFORMATION.md](WORLD_TRANSFORMATION.md). A citation with only a section sign (for example "K7 §4.1") is adopted spec text. A citation to GEN, P1, P2 or WT is proposal text. Where a spec fact comes to me only through P1 or P2, I say "as cited in P2". A claim with no citation is a gap.

## 0. Method and rules for counting

### 0.1 The idea under test

- A **goal** is conditions on world state, plus constraints (budgets, protected things).
- The vocabulary is the game's own data model, closed per version: block states, block-entity NBT, entities (type, position, NBT), bot inventories, the ledger, and the clock (WT §2, and the brief for this run).
- **Decomposition is search** over operators: the verb set (GEN §3.1), the recipe graph (K7 §4.1), pathfinding (B1 §4.1, §4.4) and finding (C5 §4.2).
- The LLM does three jobs: turn intent into a goal, write a sketch with milestones where operator effects can't be modelled, and propose subgoals as search hints. Good decompositions are cached as methods.

### 0.2 Classes

- **S.** Solved by bounded search that exists in the specs, over a small closed space with known effects. The spec is cited.
- **S\*.** A subset of S that I count as S in the first table but flag. The space is bounded and the effects are known, but the specs have no solver or model for it. Someone must write one. A skeptic can say that this is a method per pattern. Table 2 moves every S\* to H.
- **H.** Expressible as a goal, but the search is intractable or unbounded in an open, changing world. It falls to the LLM as a hint or sketch. I give the size or the hardness.
- **U.** Operator effects can't be modelled (combat, mob AI, players, RNG). It needs a coded skill or an LLM sketch with milestone goals.

### 0.3 Rating a goal

- **Full.** The whole intent is stated in the closed vocabulary.
- **Partial.** Some intent can't be stated. I say what.
- **None.** Needs intent-level generation. No task in this set is "none". That is a selection effect (§6).
- I list predicates the vocabulary lacks. I also list predicates that exist in the game but that F2 can't evaluate today. F2 §4.1 stores block states, container slots, an entity snapshot and a sighting. F2 §4.5 stores type, position, last seen, vehicle, name, persistence hint and cargo tag, and nothing about age, anger or profession (as noted in P2 §0).

### 0.4 Marks for LLM or skill output

- **Checkable.** A milestone exists as a goal predicate over observable state, and a validator can evaluate it before or during execution. I say which.
- **After the fact.** The predicate can only report a failure that has happened, or only after the run ends.
- **Not checkable.** The key variable is hidden from the bot, or no predicate expresses it.

### 0.5 Counting rules

- A **backbone** subgoal is gear preparation, travel, carry-home and the final craft or insert. Every task has some. I count them, but I also give counts without them, because they inflate S. The adjusted figure is called "task-specific".
- The share of the decomposition that falls to the LLM is (H + U) / (S + H + U). U may go to a coded skill (B5, B7) rather than to the LLM. I report H and U apart.
- Constraints and validator checks are not subgoals. A subgoal is a step that needs operators.
- One goal is the intent-to-goal step. It needs the LLM in all ten tasks. I do not count it as a subgoal. It is always H or U in kind.
- T5 refers to T1 for blaze rods. I do not count T1's subgoals twice.
- Missing moves in the operator set (bridging, doors, boats: B1 §4.1; boat riding: B7 §4.4, as cited in P2) are listed in §1 notes. I do not class them. A hole in the operator set blocks any search, so they are a cost on every task.
- Counts weigh a trivial subgoal like a dangerous one. §2 adds a note on that.

### 0.6 What I read

GEN §3.1, §3.2, §3.3, §4, §5, §6; P1 and P2 in full; WT §2, §5; K5 §4.1, §4.6, §4.10; K7 §4.1 to §4.3, §4.7, §4.9; INTERRUPTS §2, §3; F2 §3 to §4.9; and the headings of B1, B4, B5, B7, B3, C5, C9, C2 and C11 for citations. I did not read the implementations. There are none.

## 1. Tasks

Each task has a goal statement, a decomposition table, notes on LLM output and checks, the effect of a changing world, and cache value. In the tables, "Out" is what the LLM or skill must output. "Chk" is the mark from §0.4.

### T1. 16 blaze rods from a known nether fortress

**Goal.** `ledger(blaze_rod, home) >= 16`. Constraints: `bots_lost <= N`, bot-hours <= B, protected: the spawner block unchanged and block light at the spawner floor below 12 `[c]` (spawners stop above that, GEN §3.3; light layer F2 §4.7).
**Rating: full.** The target and the constraints are all state or budget. Lacks: nothing in the vocabulary. The drop rate and the spawn rate are models (K7 §4.3 tags them `[c]`), not predicates.

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear against fire and projectiles (backbone) | S | K7 §4.1 recipe graph. The costed alternatives for circular fire resistance are a planner feature in P1 T1 and GEN §3.2, not in K7 §4.2 | none | n/a |
| 2 | Travel to the fortress (backbone) | S | B1 §4.4 route search, nether portals | none | n/a |
| 3 | Locate the blaze spawner room | S | C5 §4.1 frontier plus F2 §4.6 spawner POI. A fortress is bounded, and chunk data shows the block | none | n/a |
| 4 | Reach the room (traverse the fortress) | S | B1 §4.1 A\* with unknown-cell penalty, F2 §4.7 hazards in path cost (B1 §4.3). Mob exposure is not modelled | none | n/a |
| 5 | Clear the room of hostiles | U | Blaze movement and fire are mob AI. Coded `engage` with a profile row (GEN §3.1, §3.2) | engage parameters: standoff, retreat floor, loss cap | checkable during: hostile entities in the room, from F2 §4.5, with the limit of partial view |
| 6 | Hold at the spawn floor until 16 rods | U | RNG: drop 0 to 1 per kill `[c]`, spawn rate. Only a rate from the rate store (GEN §3.4) | hold position, `with engage`, milestones `ledger >= 4, 8, 12, 16` | checkable during: the ledger (C3) |
| 7 | Carry rods home (backbone) | S | B1 route, C3 | none | n/a |

**Tally:** S 5 (backbone 3), H 0, U 2. Total 7. Falls to LLM or skill: 29%. Task-specific: 2 of 4, 50%.

**LLM output and checks.**
- Before running, a validator can check only form: that the plan uses the closed grammar, that it keeps the spawner dark (protected condition, GEN §3.3), and that the quote exists (K7 §4.3). It can't check that the fight will work.
- During running, the ledger milestones and "no blaze alive in the room" are real predicates. The first is exact. The second depends on what the bots can see.
- After the fact: whether the standoff and loss cap were good. That shows in losses per rod in the rate store.

**World change.**
- A ghast or a wandering mob: an interrupt (INTERRUPTS §2.1). Containment, quarantine, requeue (§2.2). The decomposition holds. No search. The LLM is asked only if the price leaves the quote's limits (§2.4).
- A player takes or destroys the spawner: F2 §4.9 reports `unexplained_change`. The source is lost, so K7 re-plans with other sources (K7 §5). This is a re-search, in K7, not in the LLM, unless no source is left.
- A chunk unload: the spawner works only with a player near `[c]`. The `hold` band already requires presence (GEN §4 `hold`; ARCHITECTURE §3.10, as cited in P2). The decomposition holds.
- Terrain change in front of the bot: B1 re-plans paths incrementally (B1 §4.1). The decomposition holds.

**Cache value.** Pattern: **kill-for-drop at a known source with a ledger target** (T1, T2, T5a). Cache value is high for the skeleton (`prepare, travel, observe, hold with engage, carry`). The `hold` position and the engage parameters depend on the room, so they come from the cache only as defaults.

### T2. 3 wither skeleton skulls from a fortress

**Goal.** `ledger(wither_skeleton_skull, home) >= 3`. Constraints: `bots_lost <= N`, bot-hours <= B, spare set: zombified piglins not angered. Skulls drop at 2.5% plus 1% per looting level `[c]` (P1 T2).
**Rating: full.** Anger is entity NBT, so the vocabulary has it. **F2 can't evaluate it.** F2 §4.5 has no anger. A client also can't read it (WT §1: bots can't read hidden state of entities).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear, counters for wither effect, looting choice (backbone) | S | K7 §4.1 and §4.2. The make-or-buy over a yield modifier is a new term (GEN §6). Enchanting by table is RNG | none | n/a |
| 2 | Travel to the fortress (backbone) | S | B1 §4.4 | none | n/a |
| 3 | Locate spawn floors for wither skeletons | S | F2 §4.7 spawnability by block rules, F2 §4.6 structure recognition. The spawn rules are a place-profile row | none | n/a |
| 4 | Choose where to hold, and how (kill lane, sweep room) | H | Candidate cells are the fortress floor, about 10^4 cells `[c]`. The objective (kills per hour) depends on spawn dynamics and mob paths, which have no model. Even a perfect search has nothing to optimise | hold site, lane layout, `band` | after the fact: kills per hour in the rate store |
| 5 | Engage without angering the zombified piglins | U | Group anger on hit `[c]` is mob AI. Provocation check is a validator feature (GEN §4) | spare set, `engage` parameters, sweep policy | after the fact: anger is not observable. It shows when piglins attack |
| 6 | Hold until 3 skulls | U | RNG with a low rate: about 120 kills without looting `[c]` | milestones: kill count, `ledger(skull)` | checkable during |
| 7 | Carry skulls home (backbone) | S | B1, C3 | none | n/a |

**Tally:** S 4 (backbone 3), H 1, U 2. Total 7. LLM or skill: 43%. Task-specific: 3 of 4, 75%.

**LLM output and checks.** The hold site and lane (row 4) is a plan the LLM must write from a map. Nothing checks it before running. The kill count milestone is checkable during. The anger constraint can only be seen after it fails.

**World change.**
- A group of piglins turns hostile: an interrupt. Containment backs the bots off. The decomposition needs the spare set changed, for example a `clear` of the piglins first. The validator's provocation check (GEN §4) can insert that, but only if the piglins are believed in range. Otherwise it needs the LLM again.
- Another player in the same fortress: interrupt, quarantine (INTERRUPTS §2.2). K7 may re-plan to another fortress.
- Chunk unload: spawning stops. The `hold` band covers it.

**Cache value.** Same pattern as T1. The `spare set` and `hold` lane are parameters. A cached method transfers if the next task has the same structure type. Low transfer across different structures, because row 4 depends on the layout.

### T3. 10 ancient debris from a nether region

**Goal.** `ledger(ancient_debris, home) >= 10`. Constraints: no cell adjacent to lava opened, tool tier, bot-hours <= B.
**Rating: full.** Lacks: nothing in the vocabulary. A server with anti-xray hides the true block states from the client `[c]`. Then the "observed state" part of the goal degrades (§6).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear: diamond pickaxe, food, fire resistance if wanted (backbone) | S | K7 §4.1 | none | n/a |
| 2 | Travel to the region (backbone) | S | B1 §4.4 | none | n/a |
| 3 | Find debris | S | F2 §3 `find_blocks` on loaded chunks. C5 §4.1 to load more. B4 §4.1 strip-mining by height as the fallback | none | n/a |
| 4 | Dig a route to each target without opening lava | S | B1 §4.1 with break moves, B1 §4.3 hazards, B4 §4.2 vein following (flood search), B2 §4.6 lava-face check (as cited in P1 T3). The space is the known blocks | none | n/a |
| 5 | Order the targets | S | A small route order over about 10 to 20 candidates. No solver is named in the specs, but any greedy order is bounded | none | n/a |
| 6 | Mine and collect | S | B4 §4.2 | none | n/a |
| 7 | Handle nether mobs met on the way | U | Ghast and piglin behaviour. The generic default and the interrupt contract (GEN §3.3, INTERRUPTS §2) | none new | checkable during: hp, hostile entities |
| 8 | Carry debris home (backbone) | S | B1, C3 | none | n/a |

**Tally:** S 6 (backbone 3), H 0, U 1. Total 7. LLM or skill: 14%. Task-specific: 1 of 4, 25%.

**World change.** A lava flow or a new mob: B1 re-plans the path (B1 §4.1), or the interrupt contract contains it. The decomposition holds. No LLM. A chunk unload changes nothing, since the blocks are mined from loaded chunks while the bot is there.

**Cache value.** Pattern: **mine a known block type from a region**. Very high, but B4 already is this skill. The cache adds nothing that B4 and K7 don't already do.

**Remark.** T3 is the clean case for the idea. The state is static, the effects are deterministic, and the goal is a ledger count. This is the one task where search covers nearly everything. It was also already specified.

### T4. All chests in a bastion, with gold armour

**Goal.** For each container `c` in bastion B: `contents(c)` moved to `ledger(home)`. Constraints: piglins not hostile, `bots_lost <= N`, bot-hours <= B. Gold armour worn (pacifies piglins `[c]`; opening chests still angers them `[c]`).
**Rating: partial.**
- "All chests" is a universal over a set that is discovered during the run. The state has no predicate that says "no chest was missed". A coverage rule (every room explored, or the structure's bounding box seen) must stand in, and F2 §4.6 gives no structure extent.
- "Piglins not hostile" needs anger and line of sight (`sight(c)` was a predicate term in P1 T4, and P2 §0 notes it was not counted).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear including gold armour (backbone) | S | K7 §4.1. Armour comes from the piglin row `pacified_by` (P1 §3) | none | n/a |
| 2 | Travel to the bastion (backbone) | S | C5 §4.2 to find it, B1 §4.4 | none | n/a |
| 3 | List the chests | S | F2 §3 `find_blocks(chest, ...)` on loaded chunks. Bounded, but completeness is not a state | none | n/a |
| 4 | Order the visits | S | B1 §4.4 plus a small route order, as T3 row 5 | none | n/a |
| 5 | Make each chest safe to open | H | The risk is piglins that see the chest being opened. Options: clear them, block the line of sight, lure them, skip the chest. Piglin positions change every tick and they are only partly visible. Each chest has about 4 options, over 5 to 10 chests with moving watchers. No model of piglin motion exists | per chest: `clear`, cover, or skip, plus order | after the fact: anger shows when it happens |
| 6 | Fight what attacks anyway (brutes, hoglins) | U | Always-hostile mobs `[c]` (P1 T4). Coded `engage` | `engage` parameters, retreat floor | after the fact |
| 7 | Open and take the loot | S | B3 §4.7 container transactions. Contents are unknown until opened (F2 §4.4) | none | n/a |
| 8 | Carry loot home (backbone) | S | B1, C3 | none | n/a |

**Tally:** S 6 (backbone 3), H 1, U 1. Total 8. LLM or skill: 25%. Task-specific: 2 of 5, 40%.

**World change.** New piglins or hoglins: the validator's provocation check runs on every plan (GEN §4) and can insert a `clear`. That is a local re-plan, not an LLM. A brute that appears: interrupt. Another player loots the chest first: F2 §4.4 already assumes contents may have changed. The decomposition holds, with fewer items. A chunk unload while the bots are away: the structure stays, so nothing breaks.

**Cache value.** Pattern: **container sweep under watching mobs** (bastion, fortress, mansion). The `over(chests)` shell transfers. The per-chest safety choice does not, because it depends on the room.

### T5. Eye-of-ender supply

**Goal.** `ledger(ender_eye, home) >= n`. Constraints: bot-hours <= B, `bots_lost <= N`. Each eye needs 1 pearl and half a rod (P1 T5). Two sources for pearls: endermen, or piglin barter.
**Rating: full.** The target is a ledger count. Lacks: `bartered(P)`, an event, not a state (P2 §0). Endermen are provoked by gaze, which the look controller must read (P1 T5).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Expand the recipe: eye, pearl, powder, rod | S | K7 §4.1 | none | n/a |
| 2 | Choose pearl source and rod source | S | K7 §4.2 make vs build over rates. The rates are `[c]` (K7 §4.3) and can differ by 10x, so the choice is S in form and weak in substance | none | n/a |
| 3 | Path (a): travel to a warped forest | S | C5 §4.2 biome rules, B1 §4.4 | none | n/a |
| 4 | Path (a): hunt endermen | U | Gaze provocation, teleport on projectile (P1 T5). Coded `engage` | `engage` parameters, look policy, pumpkin or none | checkable during: `ledger(ender_pearl)` |
| 5 | Path (b): gold ingots and gold armour (backbone) | S | K7 §4.1 | none | n/a |
| 6 | Path (b): barter loop | U | RNG: about 0.065 pearls per ingot `[c]`. The distribution is known, the outcome is not | batch size, `await(bartered(P), 200 gt)` | checkable during: ledger |
| 7 | Blaze rods | not counted | Same as T1 | | |
| 8 | Craft eyes and carry (backbone) | S | K7 §4.1, C3 | none | n/a |

**Tally:** S 5 (backbone 3: rows 3, 5, 8), H 0, U 2. Total 7. LLM or skill: 29%. Task-specific: 2 of 4, 50%.
A planner evaluates both paths, so I count both. Only one runs.

**World change.** An enderman teleports away or a piglin gets angry: the loop continues or an interrupt contains it. The decomposition holds. A chunk unload: endermen despawn and respawn. The `hold` band covers it.

**Cache value.** Pattern: **obtain a drop with a known rate by hunting or bartering**. High. The exchange source kind (P1 T5) is the part that transfers, and it is the part that doesn't exist yet in K7 §4.1.

### T6. Night perimeter

**Goal.** At `clock = dawn + margin`: all bots alive, and for every cell in the 64 x 64 area, `block == snapshot`. Constraints: `bots_lost = 0`, bots present in the area band.
**Rating: full.** Every part is state, budget or clock. Two limits. A `bots_lost = 0` cap stops nothing, because it fires after the first death (P2 T6). And the invariant "unchanged" is detected after a change, not before.
Lacks: an arrival forecast for hostiles. This is a model, not a state predicate (P2 T6 planner feature).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear, torches, counters (backbone) | S | K7 §4.1 | none | n/a |
| 2 | Survey light and spawnability | S | F2 §4.7 | none | n/a |
| 3 | Light the area | S | C2 §4.6 (spawn-proofing logic, a flag on a dig, not a job on a finished area: P2 T6). Placing torches so that no cell is at spawn light is a covering problem. Greedy on a grid is bounded and enough | none | n/a |
| 4 | Choose crew size N and station layout | H | The state is up to about 70 hostiles `[c]` per player cap with random arrival from outside a 64 x 64 area, over about 12,000 gt `[c]`. Layout is a stochastic game, not a search. There is no arrival model in K7 §4.7 (rates are per bot-hour of presence) | N, station cells, band, shelter point | after the fact: deaths and damage show only at dawn |
| 5 | Engage ordinary hostiles | U | Zombie, skeleton, spider, witch, phantom AI. Coded `engage` | `engage` parameters per profile | checkable during: hostile entities within a radius, from F2 §4.5 (partial view) |
| 6 | Stop creepers and block-takers before they damage blocks | U | Creepers (fuse, radius `[c]`), endermen take blocks `[c]`. Kill at range greater than blast radius from the protected edge | standoff from the area edge, target ranking by what a mob does to blocks | after the fact: `unchanged(area)` (F2 §4.9) reports the damage after it happens |
| 7 | Repair any damage | S | WT §2 (repair after damage, target = last good state), B6 place | none | n/a |
| 8 | Hold presence until dawn | S | K5 §4.3 turns `at: region`, `band`, `count` into location constraints over time. How N bots cover 4,096 cells with overlapping bands is unspecified (P2 T6) | none | n/a |

**Tally:** S 5 (backbone 1), H 1, U 2. Total 8. LLM or skill: 38%. Task-specific: 3 of 7, 43%.

**World change.** This task is made of world change. A creeper arrives: engage. A raid or a player: interrupt (INTERRUPTS §2.1); the unexplained-change event (F2 §4.9) triggers containment. A zombie breaks a door `[c]`: repair (row 7). A thunderstorm or rain changes nothing here. Chunk unload at the edge: if bots stand at the edge, the area's outer chunks may fall outside simulation distance, and mobs spawn in loaded chunks only `[c]`. The layout in row 4 may need an LLM again if losses exceed the quote (INTERRUPTS §2.4). Repeated damage is the case where the first sketch was wrong and a new sketch is needed.

**Cache value.** Pattern: **defend a region for a time window**. This is the "defence" goal kind in K5 §4.1. The cache is useful when terrain and mob mix are alike. The wall-and-moat alternative (C8 §4.5, as cited in P2) is a different cached method.

### T7. Cattle drive

**Goal.** `count(cow, adult, held: pen P) >= 20`, pen P closed (tested containment, B7 §4.8), `cows_lost <= r`. The start: 2 cows at the source.
**Rating: full.** Age is entity NBT. **F2 can't evaluate it** (F2 §4.5; P2 §0). "Closed pen" is a test by the harness (B7 §4.8), not a block pattern, so it rests on a pack.

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Wheat supply (backbone) | S | K7 §4.1, B4 §4.4 harvest (as cited in P2) | none | n/a |
| 2 | Build the source pen | S | C1 and B7 §4.8 containment patterns from a pack | none | n/a |
| 3 | Breed from 2 to 20 plus reserve | S\* | Timers are known: cooldown 6,000 gt, maturation 24,000 gt `[c]`. The model is small and exact (about 10 cycles, 60,000 gt: P2 T7). K7 §4.1 rates are fixed per bot-hour (C4 §4.2 as cited in P2) and have no stock-dependent source | none | n/a |
| 4 | Observe the corridor and find gaps | S | C5 explore, F2 §4.7 fall and water layers | none | n/a |
| 5 | Choose route and crossing (detour, bridge, boat) | S\* | A route search where crossing edges carry material and bot-hour cost. B1 §4.4 prices none (P2 T7). Bridging is also a missing move (B1 §4.1) | none | n/a |
| 6 | Herd the cows over the route | U | Cows follow wheat, wander and panic `[c]`. B7 §4.2 (lure), §4.3 (leads), §4.4 (boats), C9 §4.6 (convoy) are coded | lure or lead mode, formation, park points | checkable during: cow positions near the convoy (F2 §4.5) |
| 7 | Escort against hostiles on the way | U | As T6 rows 5 and 6, with cargo exclusion (B5 §4.6) | escort floor | after the fact |
| 8 | Insert into the pen P (backbone) | S | B7 §4.8, C9 | none | n/a |

**Tally:** S 6 (S\* 2, backbone 2), H 0, U 2. Total 8. LLM or skill: 25%. Task-specific: 2 of 6, 33%.

**World change.** A cow dies: C9 §4.11 reserves cover it. A hostile near the convoy: B7 §4.10 `mob.cargo_backoff` and the interrupt contract (cargo jobs move to the next park point, INTERRUPTS §2.2 table). The bridge or the river state changes: re-run row 5 with the new map. That is a re-search, not an LLM. A chunk unload while cows are away from bots: leashes can break `[c]`. B7 `keep_in_range` limits it.

**Cache value.** Pattern: **move live entities across terrain** (T7, T9 villager transport, any relocation of animals). High. C9's convoy already is this as coded skill. Pattern 2: **breed to a count**. High and narrow.

### T8. Elytra

**Goal.** `ledger(elytra, home) >= 1`. Constraints: `bots_lost <= N`, blocks for bridging within budget. The dragon is dead, the end gateway is known.
**Rating: full.** The elytra generates in an item frame on an end ship, not in the city, and not every city has a ship `[c]` (P2 T8). So the LLM must fix the user's wording ("from an end city") when it writes the goal. This is the intent-to-goal job doing real work. If no ship exists in the explored area the goal is infeasible. The goal is state-only.
Lacks (F2): the item in an item frame (entity NBT; F2 §4.5 doesn't store it).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Gear: bridging blocks, counters for shulker levitation (backbone) | S | K7 §4.1. Counter alternatives are GEN §3.2 | none | n/a |
| 2 | Travel through the gateway (backbone) | S | B1 §4.4 has no gateway mode (P2 T8): a travel mode value | none | n/a |
| 3 | Find an end ship | S | C5 §4.2 generation rules. Fails with `not_found` if there is none (as cited in P2) | none | n/a |
| 4 | Cross void between islands | S\* | Route with place moves, priced by blocks and bot-hours. Same missing component as T7 row 5. B1 §4.1 lists bridging as a missing move | none | n/a |
| 5 | Approach the ship under shulker line of sight | S\* | Path cost by exposure to stationary shooters. B1 §4.3 prices cargo line of sight and enemy sightings, and reads no profile field (P2 T8) | none | n/a |
| 6 | Survive end-city hostiles (shulkers, endermen) | U | Homing bullets, levitation, teleport `[c]`. A closed shulker is armoured `[c]`. Coded `engage`, with the conditional-state problem (P2 §4) | `engage` parameters, counter set | after the fact: levitation arrives as `effect_gained` (B2 §3, as cited in P2) after the hit |
| 7 | Take the elytra from the frame | S | B6 `use` or hit. Needs `collect` from an entity slot (a value, P2 T8) | none | n/a |
| 8 | Return and carry (backbone) | S | B1, C3. The outer gateway return link is `[c]` unsure | none | n/a |

**Tally:** S 7 (S\* 2, backbone 3), H 0, U 1. Total 8. LLM or skill: 13%. Task-specific: 1 of 5, 20%.

**World change.** A shulker appears or a bridge piece is lost: B1 re-plans the path (B1 §4.1). A fall into the void is a death. The reflex `move.mlg_bucket` is irrelevant over void (B1 §4.9). Chunk unload: the outer islands are far from the home chunks. The bots are the only loaders. No change to the decomposition. No LLM, unless the infeasible case occurs (no ship), when the goal itself changes.

**Cache value.** Pattern: **go to a far structure and take a fixed item** (elytra, shulker shells, heart of the sea). Medium. The route and hazards are different each time, so only the shell transfers.

### T9. Mending librarian

**Goal.** Exists villager `v`: `profession(v) = librarian` and `offers(v) includes enchanted_book{mending}`, and `v` in the cell, trade locked. Constraints: attempts <= K, the villager must be unemployed and untraded at the start.
**Rating: full.** All is entity NBT (profession, offers) and a count of attempts.
Lacks (F2): profession, level and offers (F2 §4.1 has no payload kind for offers; P2 T9). A client reads offers only after opening the trade window (B3 §4.9, as cited in P2). So the predicate is evaluable only by an action that has side effects.

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Find an unemployed, untraded villager | S | F2 §3 `entities(type, region)`. The profession is not stored (P2 T9). Once F2 holds it this is a query | none | n/a |
| 2 | Capture and move the villager into the cell | U | Villager pathing and boat or minecart push. Coded B7 §4.4, §4.6 | load, unload, insert | checkable during: the villager is in the cell (entity position) |
| 3 | Build the cell | S | B7 §4.8 pack containment | none | n/a |
| 4 | Reroll until the offer shows | U | Per-iteration RNG, about 1 in 35 to 40 for Mending `[c]`. Villager job-site timing is mob AI. The loop body is fixed and the exit test is a predicate (P2 T9 grammar need) | routine: break lectern, await no profession, place, await librarian, read offers | checkable during: `profession` and `offers` milestones |
| 5 | Lock the trade | S | B7 `lock_trades` (B7 §4.6), B3 §4.9. The cost is known only after reading offers | none | n/a |

**Tally:** S 3 (backbone 0), H 0, U 2. Total 5. LLM or skill: 40%. Task-specific: 2 of 5, 40%.

**World change.** Another villager claims the lectern: the cell's walls stop it (a pack property: B7 §4.8). A nitwit, or a villager that traded already: the loop ends on its attempt budget `[c]` (P2 T9). Then the decomposition needs a new villager: a re-search at row 1, and the LLM only if the cause is not obvious. A zombie or a player near the cell: interrupt. Chunk unload: villagers inside unloaded chunks stop working `[c]`, and the `hold` band `entity_ticking` (ARCHITECTURE §3.10, as cited in P2) keeps the cell loaded.

**Cache value.** Pattern: **reroll a random output until a predicate holds** (trades, enchant reroll, some farms). Very high. It is a named routine in a pack (C11 §4.4, as cited in P2) and the plan is one loop.

### T10. Ancient city loot

**Goal.** For each chest in the city: contents in `ledger(home)`. Constraint: the warden does not appear. Budgets: attempts, bot-hours, `bots_lost`.
**Rating: partial.**
- "Without summoning the warden" is a safety invariant over a **hidden counter**. A shrieker's warning level is stored per player on the server `[c]`. It is not an entity NBT that a client can read. The client sees only effects after the fact (darkness, a shriek sound).
- The vocabulary can state `not exists(entity warden)`. That is the failure, not a guide for the search.
- The vocabulary has state, not events. "No vibration within 8 blocks of a sensor, unless a wool block is on the path" is an event rule over actions.
Lacks: `strikes(bot)`, a vibration-event vocabulary, occlusion (P2 T10).

| # | subgoal | class | solver or reason | out | chk |
|---|---|---|---|---|---|
| 1 | Find an ancient city | S | C5 §4.2 generation rules, F2 §4.6 signature blocks | none | n/a |
| 2 | Gear, quiet kit (backbone) | S | K7 §4.1 | none | n/a |
| 3 | Map the sensor and shrieker field | S | Chunk data shows the blocks `[c]`; F2 §4.6 and §4.7 (a "known trap" layer kind) | none | n/a |
| 4 | Silent paths that avoid every sensor radius | S | B1 §4.3 path costs read F2 hazards; keep-out for the radius `[c]` 8 | none | n/a |
| 5 | Dig an entry that starts outside all radii | S | B6 break, B1 §4.1 | none | n/a |
| 6 | Reach chests that are inside a sensor radius | H | Options: skip, place wool to occlude, accept a limited number of strikes and rotate bots (P2 T10). Every placement is itself a vibration. The warning level is hidden. The vibration rules (event types, ranges, occluders) are a model in `[c]` with no row in F5. The sequence space is small, but the key variable can't be read | ordered plan of occlusions, skips and strikes per bot | the geometry is checkable against the model before running. The warning level is **not checkable** |
| 7 | Loot quietly | S | B3 §4.7. Opening a container is a vibration `[c]`; this only works if row 6 is right | none | n/a |
| 8 | If the warden appears: break contact | U | Warden AI `[c]`. A fight is lost (one or two hits). The interrupt contract can't undo a spawned warden (P2 T10) | flee plan | **not checkable** until it is too late |
| 9 | Carry loot out by the same path (backbone) | S | B1, C3 | none | n/a |

**Tally:** S 7 (backbone 2), H 1, U 1. Total 9. LLM or skill: 22%. Task-specific: 2 of 7, 29%.

**World change.** A bot's strike raises the level and the level decays over minutes `[c]` (P2 T10), so the state of the world depends on the bots' own history. A new player: interrupt. A mob (none in deep dark `[c]`) does not matter. A chunk unload: sculk blocks stay. If anything unexpected makes noise, the contract's response (back off, quarantine, requeue) is correct only before the fourth strike. After a warden spawns, containment can't help. This is the one task where the interrupt contract (INTERRUPTS §2) is the wrong tool: the plan must refuse instead of price (P2 T10), and a refusal needs a model of hidden state.

**Cache value.** Pattern: **quiet traversal of a trap field**. Narrow. It would help for other sculk structures only. The vibration rules table is the value, and it is data, not a method.

## 2. Table 1: base classification

S includes S\*. "LLM or skill" is (H + U) / total. "Task-specific" drops backbone subgoals from every count.

| task | goal rating | S (of which S\*, backbone) | H | U | total | LLM or skill, all | task-specific total | LLM or skill, task-specific |
|---|---|---|---|---|---|---|---|---|
| T1 blaze rods | full | 5 (0, 3) | 0 | 2 | 7 | 29% | 4 | 50% |
| T2 wither skulls | full | 4 (0, 3) | 1 | 2 | 7 | 43% | 4 | 75% |
| T3 ancient debris | full | 6 (0, 3) | 0 | 1 | 7 | 14% | 4 | 25% |
| T4 bastion chests | partial | 6 (0, 3) | 1 | 1 | 8 | 25% | 5 | 40% |
| T5 eyes of ender | full | 5 (0, 3) | 0 | 2 | 7 | 29% | 4 | 50% |
| T6 night perimeter | full | 5 (0, 1) | 1 | 2 | 8 | 38% | 7 | 43% |
| T7 cattle drive | full | 6 (2, 2) | 0 | 2 | 8 | 25% | 6 | 33% |
| T8 elytra | full | 7 (2, 3) | 0 | 1 | 8 | 13% | 5 | 20% |
| T9 mending librarian | full | 3 (0, 0) | 0 | 2 | 5 | 40% | 5 | 40% |
| T10 ancient city | partial | 7 (0, 2) | 1 | 1 | 9 | 22% | 7 | 29% |
| **all ten** | 8 full, 2 partial, 0 none | **54** (4, 23) | **4** | **16** | **74** | **27%** (20 of 74) | **51** | **39%** (20 of 51) |

Reading Table 1:
- By count, S is the majority: 54 of 74 (73%). Without the backbone, S is 31 of 51 (61%).
- H is rare: 4 in all. U is common: 16. So the LLM-or-skill share is mostly **U**, not H. Few things are "expressible but intractable". Many things are "not modelled".
- Of the 16 U subgoals, most are fights, hunts and herding. Those already have coded skills in the specs (B5 §4.2 per-mob modules, B7 §4.2 to §4.4 mob handling, C9 §4.6 convoys). For them the LLM supplies parameters and milestones, and does not search.
- **Counts under-weigh the U and H rows.** A travel step and a 120-kill grind both count 1. In T1, T2, T5, T6 and T7 the U and H rows hold nearly all the bot-hours and nearly all the risk `[c]`. I did not estimate a weighted share, because I have no data. I expect it to be well above the count share.
- **The S column is not a result for the new idea.** Every S row was already solved by a component that exists in the specs (K7, B1, C5, B4, B6) before anyone wrote the goal-state framing. The framing adds no solving power to S. It adds a uniform way to write goals and milestones.
- **Four S rows are S\*.** They need a component the specs lack: stock-dependent breeding (T7), priced crossing edges (T7, T8) and exposure pricing (T8).

### Predicates the vocabulary lacks, all ten tasks

| task | missing or unevaluable predicate |
|---|---|
| T1 | none in the vocabulary |
| T2 | piglin anger (entity NBT, hidden from the client; F2 §4.5 lacks it) |
| T3 | none (anti-xray would hide true block states `[c]`) |
| T4 | "no chest missed" (a universal over a set found at run time); line of sight; anger |
| T5 | `bartered(P)` (an event, not a state) |
| T6 | an arrival forecast (a model, not a state) |
| T7 | age and love state (entity NBT; F2 §4.5 lacks it); "pen tested closed" (a harness result, B7 §4.8) |
| T8 | item in a frame (entity NBT; F2 §4.5 lacks it) |
| T9 | profession and offers (F2 §4.1 has no payload; offers readable only on opening: B3 §4.9, as cited in P2) |
| T10 | warning level (server-side hidden state `[c]`), vibration events, occlusion |

The vocabulary is closed per version. The bot's **observable subset** is smaller. Three kinds of gap show up. (a) State that exists in the game, is client-visible `[c]`, and that F2 doesn't store (T2 partly, T7, T8, T9 profession). These are cheap to fix. (b) State the client can't read (T2 anger, T10 warning level, T9 offers before opening). These can't be fixed, so a goal over them can only be checked by side effects or by outcome. (c) **Events and invariants** ("never", "without", "not before"). The vocabulary describes states, and these describe trajectories. T6 and T10 need them. The constraint list in GEN §3.3 (budgets, protected conditions) is the nearest thing, and it covers T6's "unchanged" but not T10's "no strike".

### Checkability of the 20 H and U outputs (base case)

| mark | count | where |
|---|---|---|
| checkable during (milestone is a predicate over observable state) | 10 | T1 rows 5 and 6; T2 row 6; T3 row 7; T5 rows 4 and 6; T6 row 5; T7 row 6; T9 rows 2 and 4 |
| only after the fact | 8 | T2 rows 4 and 5; T4 rows 5 and 6; T6 rows 4 and 6; T7 row 7; T8 row 6 |
| not checkable | 2 | T10 rows 6 (the warning level) and 8 |

- **None of the 20 can be verified before it runs**, except the geometry part of T10 row 6, and only against a model that is `[c]`. What a validator can check before running is form: the grammar, the vocabulary, the budgets, the protected conditions, the provocation list and the quote (GEN §4; K7 §4.3).
- The 10 "checkable during" marks are **progress watchdogs**. They show that a ledger count is rising or that a hostile is gone. They don't show that the plan is a good one. A good milestone catches a stall, and it does not prevent a loss.
- So the LLM's output on H and U is validated mainly by running it, with caps. That is how the contract in INTERRUPTS §2 and the caps in GEN §3.3 already work.

## 3. Table 2: pessimistic recount

A reasonable skeptic would make these moves. I give the reason for each.

**Rule A. Every S\* becomes H.** The solver does not exist. A solver written for one pattern is a method per goal kind (K5 §4.1) under a new name. Moves: T7 rows 3 and 5; T8 rows 4 and 5. Total 4.

**Rule B. Gear with a choice among counters becomes H.** The counter alternatives are a planner feature that exists only in GEN §3.2 and P1 T1. K7 §4.2 compares making by hand with building a facility, and no more. Until threat rows and costed alternatives exist, someone must pick the kit. Moves: T1 row 1, T2 row 1, T6 row 1, T8 row 1. Total 4. I did not move T3, T4, T5 or T10 gear, where the kit follows one row without a real choice.

**Rule C. "Find all" under partial visibility becomes H.** T4 row 3: the set must be complete, and F2 §4.6 gives no structure extent. Total 1.

**Rule D. Coverage geometry becomes H.** T6 row 8: P2 T6 says the spread of N bots over 4,096 cells is unspecified in K5 §4.3. Total 1.

**Rule E. Silent paths become H.** T10 row 4: the model of vibrations is `[c]`, has no row in F5, and requires five skills to obey `quiet` (P2 T10). Total 1.

**Rule F. S whose safety depends on mob state becomes U.** T1 row 4: the path is a path search, and "reach the room alive" depends on ghasts and blazes. Total 1.

**Rule G. H with no model of the objective becomes U.** No search can optimise an objective that has no model. T2 row 4, T4 row 5, T6 row 4, T10 row 6. Total 4.

| task | S (of which backbone) | H | U | total | LLM or skill, all | task-specific total | LLM or skill, task-specific | moves |
|---|---|---|---|---|---|---|---|---|
| T1 | 3 (2) | 1 | 3 | 7 | 57% | 4 | 75% | B (row 1), F (row 4) |
| T2 | 3 (2) | 1 | 3 | 7 | 57% | 4 | 75% | B (row 1), G (row 4 to U) |
| T3 | 6 (3) | 0 | 1 | 7 | 14% | 4 | 25% | none |
| T4 | 5 (3) | 1 | 2 | 8 | 38% | 5 | 60% | C (row 3), G (row 5 to U) |
| T5 | 5 (3) | 0 | 2 | 7 | 29% | 4 | 50% | none |
| T6 | 3 (0) | 2 | 3 | 8 | 63% | 7 | 57% | B (row 1), D (row 8), G (row 4 to U) |
| T7 | 4 (2) | 2 | 2 | 8 | 50% | 6 | 67% | A (rows 3, 5) |
| T8 | 4 (2) | 3 | 1 | 8 | 50% | 5 | 60% | A (rows 4, 5), B (row 1) |
| T9 | 3 (0) | 0 | 2 | 5 | 40% | 5 | 40% | none |
| T10 | 6 (2) | 1 | 2 | 9 | 33% | 7 | 43% | E (row 4), G (row 6 to U) |
| **all ten** | **42** (19) | **11** | **21** | **74** | **43%** (32 of 74) | **51** | **55%** (28 of 51) | 11 S to H, 1 S to U, 4 H to U |

Check: 42 + 11 + 21 = 74. Moves: 11 S to H (Rule A 4, B 4, C 1, D 1, E 1), 1 S to U (Rule F), 4 H to U (Rule G). The H column ends at 11 because the moved S rows replace the H rows that Rule G sends to U.

Reading Table 2:
- The LLM-or-skill share rises from 27% to 43% of all subgoals, and from 39% to 55% of task-specific subgoals.
- H grows from 4 to 11. U grows from 16 to 21. The mass moves from "solved by search" to "not modelled".
- The skeptic's moves are defensible. Rule A rests on a missing component, Rule B on a proposal that isn't in K7, Rule G on a missing model. None rests on a hard-to-defend argument.
- Even here, T3 is untouched at 14%, and T9 and T5 hold. The tasks that stay mostly S are those with static state and deterministic effects: mining, crafting, rerolling a fixed loop.

## 4. Cross-check against WORLD_TRANSFORMATION §2 and §5

The user's NP-hardness point is aimed at construction and redstone. None of T1 to T10 is a construction-ordering task. T6 lighting and T7's pen are printable work (WT §5.1 first row: any order that obeys support works). So **this set does not test the hardest case.** I did not add a task, because the brief fixes the ten.

What WT itself says is still evidence:
- WT §2 states the problem as "observed state + target → plan". For a printable target a rule set suffices (WT §5.2: R1 is a topological order of the flow graph, a linear-time computation).
- For an order-sensitive target WT does **not** search. It orders by rules, breaks cycles with an arming cell (R5), and "if no order is found, the pack is refused at load with the conflict attached" (WT §5.1; open point 4 in WT §7 asks whether to allow some search at build time).
- WT §5.2 names weak spots with no rule: location-dependent wire update order, zero-tick pulses, piston timing, comparator subtract mode, sculk at range. These are the places where a search would need a simulator, and WT §1 rules out a simulator on the build path (about 50x real time at best).
- So for the builder the design is already "rules plus refusal", with no general search. That agrees with the user's position for the hard case. It is not a counter-example to this run's counts, and it is not a case that this run's counts cover.

## 5. Conclusions

**What the generic search really covers.**
- Recipe expansion and source choice (K7 §4.1, §4.2), route search over known cells (B1 §4.1, §4.4), structure and block finding (C5 §4.2, F2 §3), mining (B4 §4.2), place and break, and static covering problems such as lighting (C2 §4.6). In my count this is 54 of 74 subgoals, or 31 of 51 without the backbone.
- All of it exists in the specs already, and none of it needs the goal-state framing. K5 §4.1's methods already call these components. The framing does not add solving power here. It adds one way to write the end condition, and one way to attach milestones.
- It covers a task well when state is static and effects are deterministic (T3, T9's fixed loop, the recipe parts of T5 and T8).
- It needs four components the specs lack (S\*): stock-dependent breeding, priced crossing edges, exposure pricing. And it relies on three proposal-only features: costed counters, protected conditions, the provocation check (GEN §3.2, §3.3, §4).

**What the LLM really carries.**
- In all ten tasks: intent to goal. T8 shows real work here (the elytra is on a ship, not in the city). T4 and T10 show that some intents can't be fully stated.
- In the 20 H and U subgoals (base case): a sketch or parameter set that no validator can verify before it runs. Ten have a milestone that can show progress while it runs. Two can't be checked at all (T10).
- The LLM does very little **search**. In H and U there is nothing to search with: no model of piglin motion, no arrival model, no model of the hidden warning level. The LLM writes a plan from experience, and the system runs it under caps. In practice the LLM is the planner for these rows.

**Does the user's expectation hold?**
- Short answer: yes, for the parts that matter, and with a correction to the words.
- By count, the base share is 27% of all subgoals and 39% of the task-specific ones. That is "much", not "most". The pessimistic share is 43% and 55%.
- By weight, I expect it to be higher than the count, because the U and H rows hold the long, risky parts of T1, T2, T5, T6, T7 and T10. I have no data to put a number on it.
- The correction: the fallback is not "search that falls back to the LLM". For 20 to 32 subgoals no search is possible, because the effects have no model. The LLM does not take over a search. It writes a plan where search does not exist. Many of those rows are filled by coded skills (B5, B7, C9), which exist or are planned, and these need no LLM at run time.
- The user is also right that goal-state search "adds layers". It does so in four places: S\* components, the F2 gaps (age, anger, profession, offers, frame items), the predicate vocabulary (P2 §5 item 2: no list of terms exists), and the validator. I did not count the cost of those layers in days. Runs 1 and 2 counted them in items.
- One thing does not fit the user's frame. Most subgoals by count are S and always worked without an LLM. The system doesn't lose there. It just doesn't gain either.

**The LLM's role in the architecture.**
- It must be the **main planner for everything outside the S backbone**, not a fallback. A fallback implies that deterministic search tries first and fails. For H and U rows search doesn't try, and there is nothing to fail.
- It needs a **deterministic validator** around it. Its jobs are: check the plan's form against the closed grammar and vocabulary (GEN §4); derive protected conditions (GEN §3.3); run the provocation check (GEN §4); quote the plan (K7 §4.3); enforce budgets and caps (GEN §3.3); and watch the milestones during the run. This validator is a guard and a watchdog. It is not a verifier of correctness. The 20-row table in §2 shows why: none can be verified in advance.
- Containment and pricing (INTERRUPTS §2) is what makes the LLM's unverified plans acceptable. It limits the cost of a bad plan. T10 is the exception where it can't, and there the plan must be refused without a model of the hidden state.
- Search (K7, B1, C5) is a **subroutine** that the plan calls for the S rows. It is not the driver.

**Recommendation.** Use the goal as the specification and check language: end condition, constraints, milestones, protected sets. Do not build a general goal-state planner. Make the LLM (commander or officer) the planner for the non-backbone rows, behind a deterministic validator and a milestone watchdog, with search (K7, B1, C5) called as subroutines for the S rows. Build an S\* component only when a second task needs it. Cache successful plans as parameterised routines (C11 §4.4, as cited in P2) and not as an inferred method base, because §1's cache notes show transfer only at the shell level (T9's reroll loop and T1's hunt skeleton transfer well, T2's hold lane, T4's per-chest choice and T10's occlusions do not). Before adopting any part of this, add the F2 state gaps (age, anger, profession, frame items) and a closed predicate list. Cache-hit matching is itself a job: deciding whether a new goal is an instance of a cached pattern needs the LLM or a structural match on goal parameters, and I did not count it.

## 6. What a paper test can't tell

- **Single author.** I classified, drew the S/H/U lines, chose the granularity and counted. I have read the proposal. The U/H line and the S/S\* line are judgements. Someone who did not read GEN should recount. The pessimistic table shows how far the numbers move with the rules (27% to 43%).
- **Granularity bias.** Split "travel" into five steps and S grows. Merge the backbone and S shrinks. I gave the count both ways. The result does not change direction but the exact shares do.
- **Counts are not cost.** A subgoal is a subgoal. I can't weigh it without data on bot-hours, losses and tokens. F3 runs with fixed seeds would give those.
- **Task selection.** All ten tasks are acquisition or relocation tasks with a ledger or count target, chosen in runs 1 and 2 (T8 and T9 were nearly specified already). No diplomacy, no multi-operation program, no raid, no builds, no redstone. I found no "none" rating, and that is a selection effect. The user's NP-hardness point is aimed at construction, which this set doesn't contain (§4).
- **Spec versus reality.** S rows assume that spec text works. Many B1 moves are missing (bridging, doors, boats: B1 §4.1; boat riding: B7 §4.4 as cited in P2), and these block S rows in T7, T8 and T10. Nothing here is implemented.
- **Game facts.** Every `[c]` needs probing before anything depends on it (F5 §4.4). The facts that move the classification most: client visibility of entity state, anti-xray behaviour, the vibration rules, sensor radius, and spawn caps.
- **LLM quality.** I assume the LLM can write a plausible sketch. I did not test it. Whether it writes sketches with usable milestones, and how often a validator rejects them, is the key unknown. Run it on a harness with fixed seeds.
- **Validator cost.** I assume a validator that checks form, protected conditions, provocation and quote. Its cost, and its false-reject rate on creative plans, is unmeasured.
- **Cache matching.** Whether a cached method is found, matched to a new goal, and still valid in a changed world is untested.
- **World change.** I traced each task by hand. Real runs find interactions I did not see. T10's hidden state is the case I trust least.
