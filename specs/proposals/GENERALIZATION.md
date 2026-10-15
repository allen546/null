# Generalization floor: no skill code per scenario

> **Status (2026-10-15): decided by Allen in the shape of §0 (AD-K5.2, [DECISIONS.md](../DECISIONS.md#ad-k52-scenario-knowledge-as-data-or-as-method-code)).** §1 to §8 are the proposal of 2026-10-14 as revised after the first paper test ([GENERALIZATION_PAPER_TEST.md](GENERALIZATION_PAPER_TEST.md)). They are kept as the detail behind §0. Where §0 and a later section disagree, §0 wins. Evidence: paper tests [1](GENERALIZATION_PAPER_TEST.md), [2](GENERALIZATION_PAPER_TEST_2.md) and [3](GENERALIZATION_PAPER_TEST_3.md). Nothing here is measured. Where a claim is a guess it says so.

## 0. Decided shape (Allen, 2026-10-15)

### 0.1 Why the 2026-10-14 rule was not enough

- Minecraft is an open world. Nobody can list its scenarios in advance. So code per scenario does not scale, even when coding agents write the code. Handwritten data per mob or per structure does not scale either.
- The K5 HTN planner needs a method for each high-level task. Low-level tasks can be listed. High-level tasks can't. §4's "operator data per goal kind" still lists goal kinds, so it breaks in the same way.
- Search toward a goal state in an open world that keeps changing is not a fix. Ordering construction and redstone is already NP-hard. Paper test 3 found that the hard steps of a task mostly have no model to search: fights, mob behaviour, other players. Searchable steps were 54 of 74 by count, or 42 of 74 in the pessimistic recount. The existing components (K7, B1, C5, B4) already solved all of them.

### 0.2 The shape

1. **Facts are data, and nobody writes them per row.** Every profile field gets its values from at least one of three sources:
   - the game jar: entity-type tags, damage-type tags, loot tables, recipes (F5 §4.2);
   - observation: damage events (type and source), distance at hit, entity metadata;
   - the LLM's prior: it is marked unconfirmed, it may only set how careful the bots are, and it never certifies a plan.

   **Field admission rule:** a field whose values must be written by hand for each mob or place is code per scenario under another name, and is not allowed. This replaces §1's "fields per task falling" measure.
2. **Skills are code only for the important parts.** The interrupt contract's cost-ranked batch review ([INTERRUPTS.md](../INTERRUPTS.md)) chooses them: a failure that is expensive and recurring gets a skill. The named boss exceptions (§5) stay. Everything else uses the generic default with a safe floor (§3.3), and the interrupt contract contains and prices its failures.
3. **Goals are the language to state and check a task, not the input to a general planner.** A goal is an end condition, constraints (budgets, the protected set) and milestones. It is written in the closed state vocabulary of the version: block states, block-entity NBT, entities with their NBT, inventories, the ledger and the clock. There is no general goal-state planner.
4. **The LLM is the main planner for the steps that have no model.** In v1 the commander does this alone (AD-K5.1, decided 2026-10-15). The LLM works in stretches:
   1. It writes a stretch as goals and a plan over the closed verb set and the plan grammar (§3.1, §4). It names end states such as "diamond armour for 40 bots". It does not write item chains.
   2. The validator checks the form, the protected conditions, the provocation check and the caps. K7 quotes the stretch. The LLM may revise the stretch after it sees the quote.
   3. Null runs the stretch. The existing searches do the steps they can solve: K7's recipe graph, B1's pathfinding, C5's find, B4's mining. These searches are subroutines of the plan, not the driver.
   4. The LLM wakes on K3 §4.2's triggers: a milestone, an escalation that passes the price gate, a heartbeat, a person. Problems below the price threshold are contained locally and never reach the LLM.

   Several stretches run at the same time as separate operations. Example: squads mine diamonds while another squad scouts for a fortress.
5. **The validator is a guard and a watchdog, not a proof.** In paper test 3, no validator could confirm any hard step before it ran. Milestones show progress or a stall. They don't show that a plan is good. Bad plans are acceptable because the quote, the caps and the interrupt contract limit what they cost.
6. **Plans that work are saved as routines with parameters** (C11 §4.4). Methods become a cache, not a requirement. Nobody infers a method base from the saved plans.

Example, "speedrun with the swarm": the commander writes stretch 1 as "diamond armour and tools for all bots, no deaths, 2 h", and K7 expands it into wood, stone, iron and diamonds. A scouting operation runs in parallel. At the milestone, the commander writes stretch 2: obsidian, portal, fortress, "16 blaze rods". The fortress fight uses the generic `engage`, with the blaze row filled from jar tags and from observation. The next speedrun starts from the saved routines. The commander steps in only where this world is different.

### 0.3 What stays, and what changes, in §1 to §8

- **Stays:** the closed verb set (§3.1), profile rows (§3.2, now under the field admission rule), the generic default (§3.3), the ratchet (§3.4), the closed plan grammar (§4), the named exceptions (§5).
- **Changes:** §4 item 1 no longer adds operator data per goal kind. Deterministic expansion is only the existing searches. §4 item 2, LLM composition, becomes the main path for steps with no model, not a fallback. §7's pass criterion is superseded: the measure is now the field admission rule plus the measurements in §0.4.

### 0.4 Before building, and the first measurement

- Fill the F2 state gaps that paper tests 2 and 3 found: entity age, anger, villager profession and item-frame contents (F2 §4.5).
- Write a closed list of predicate terms and a closed list of budget kinds (paper test 2 §5 items 2 and 5).
- First measurement (F3, fixed seeds): give the LLM the goal "16 blaze rods". Count its milestones, the validator's rejections, the losses and the tokens. The quality of the LLM's plans for the hard steps is the main unknown in this shape.

### 0.5 Still open

- When the commander's load justifies officers (AD-K5.1 keeps them out of v1).
- How the commander submits a plan: a new K2 command, or `errand` extended (§8).
- How a new goal is matched to a saved routine, and whether that routine is still valid in a changed world.
- Construction and redstone. Paper test 3 had no construction task. For order-sensitive builds, [WORLD_TRANSFORMATION.md](WORLD_TRANSFORMATION.md) §5 uses rules and refusal, not search.
- Which entity attributes the server sends to clients (`[c]`: max health and speed are probably sent, attack damage probably not).

## 1. The rule

**A new scenario may add rows, and values that existing code already handles. It may not add a verb, a method, a field, or a plan construct.** Those four are framework code: each is reviewed like a component change and has to serve more than one row, or be a named exception (§5).

"Acquire 16 blaze rods in a nether fortress", "bring back wither skulls", "mine 10 ancient debris" and "loot a bastion" are four scenarios. If each needs new code, the project can't finish. If they are four compositions of the same few verbs over four rows of data, it can.

Verbs alone are the wrong budget unit. The paper test needed no new verb across five tasks, but nine new fields, and every field is code that some component must read. So the measure is how much framework code a new task adds, fields included, and whether that falls from task to task.

This is a budget rule, so it comes with a test (§7). If most tasks add a verb, or new fields per task stop falling, the design has failed and we have per-scenario code under another name.

## 2. Where scenario-specific code lives today

Read off the specs (v1.2):

| Place | What is specific | Cost |
|---|---|---|
| K5 §4.1 HTN methods | "code per goal kind (build, mine district, base, convoy, chamber capture, defence)" | A method per kind of operation |
| B5 §4.2 per-mob modules | creepers, skeletons, endermen, spiders, witches, piglins, hoglins, guardians, phantoms, breezes... | A module per mob |
| B5 §4.3 boss fights | blaze, wither, dragon, each its own paragraph and code | A module per boss |
| C7 plays | doctrine authored per situation | Authored per situation |
| C6 §4.2 route templates, C11 descriptors, F5 mechanics table | **data** already | Cheap |

The first three are the budget sinks. The last row is the pattern to extend.

## 3. The floor

Four parts. Each one exists in a weaker form in the specs already.

### 3.1 A small, closed verb set

A **verb** is a job kind with a typed contract (ARCHITECTURE §1.3: inputs, preconditions, success test, outcomes, budget). The verbs are what B1–B7 and C-skills already expose. The proposal is a discipline on them, not new components:

| Verb | Parameters (data-valued) | Owner |
|---|---|---|
| `travel` | destination, mode, hazard tolerance | B1 |
| `observe` | region, what to look for, until | C5, F2 |
| `engage` | target selector, **threat profile**, standoff, retreat floor, spare set (entities not to provoke) | B5, C7 |
| `clear` | region, what counts as hostile, loss cap, protected conditions (§3.3) | B5, B6, B1 |
| `collect` | item, source (block, mob, container, ground), count | B4, B3 |
| `carry` | item set, from, to (container, stand, or ground near an entity) | C3, C9 |
| `hold` | position or region, presence band, `until` predicate | K6 |
| `retreat` | rally point, reason | B5, B2 |
| `prepare` | capability set (derived from profiles, §3.2) | C6, B3, C3 |
| `use` | item (or empty hand), block or entity: ignite, press, open, sleep | B6 |
| `trade` | exchange row (§3.2), count | B3 |
| `build`, `place`, `break`, `craft`, `smelt`, `brew` | as today | C1, B6, B3 |

`use` and `trade` were missing from the first version of this table. Both already exist (B6 click choice and errand "press"; B3 §4.9), and the paper test needed `use` for portals, TNT and beds.

Rules:

- A verb takes a **profile** as a parameter. `engage(blaze)` and `engage(piglin_brute)` are the same verb with different rows.
- A verb declares **preconditions, effects and a cost model** in data, so K7 can expand and quote any composition of verbs (K7 §4.1, §4.3).
- A new verb is an architecture change, reviewed like a new component. A new row is a data change.

This keeps AD-F5.2 intact: verbs are decision code over named mechanism crates, and they don't import Bevy or Azalea.

### 3.2 Scenario knowledge as rows in F5

Two row types, stored in F5's mechanics table (§4.3) with its confidence field (`claimed`, then `measured`):

**Threat profile** (per mob type)

| field | example (blaze), unverified |
|---|---|
| attack kind | ranged projectile, sets fire |
| damage types | fire, projectile |
| speed, reach, hp | from game data (F5 §4.2) |
| immunities | fire, lava |
| hazards it creates | fire on bot, fire on ground |
| counters (capability tags) | `fire_resistance`, `projectile_cover`, `ranged_attack` |
| habits | hovers, keeps distance, flees at low hp? |
| spawn conditions | spawner, light, cap |
| drops | item, count range, looting effect, `[c]` rate |

**Place profile** (per structure type, biome or height band; called structure profile in the first version, widened because ancient debris lives in a height band, not a structure)

| field | example (nether fortress), unverified |
|---|---|
| dimension, generation rules | from F5 / game data |
| marker blocks (what F2 recognises) | nether bricks, nether wart, spawner |
| interior hazards | lava, open edges, void, ghasts outside |
| mob spawners and what they spawn | blaze spawner (rooms) |
| loot | chests, `[c]` contents |
| entry and approach heuristics | which side, bridges |
| danger class | feeds retreat floors and loss caps |

**Counter rules** are a short table from hazard tags to capability tags (`fire damage → fire_resistance | avoid`, `ranged projectile → cover | shield | close_distance`, `void edge → no_sprint, edge_guard`). The readiness gate (C6 §4.8) and `prepare` derive gear, potions and formation from profiles through these rules. Nobody writes "a blaze fight needs fire resistance"; the profile says blazes cause fire damage and the rule says what answers that.

Counter rules list **alternatives**, and K7 costs each one through the recipe graph and drops those it can't make. This matters: fire resistance needs a brewing stand, which needs a blaze rod `[c]`, so for a first blaze trip the rule must fall through to shield or cover, or to a potion from piglin bartering.

**Exchange rows** (per trading entity): what goes in, the loot table or offer list that comes out, time per exchange, and the entity's required state (an adult piglin that isn't angry, a villager with a profession). Piglin bartering and villager trades are both exchanges, and K7 treats them as a source kind beside recipes and facilities.

Partly derivable from the game's own data (mob attributes, damage types, loot tables via F5 §4.2). The rest is a few hand-written fields per row, not code.

**Field registry.** Every field names the code that reads it, because a field is code. After the paper test (its §3) the threat profile's interpreted fields are `movement`, `attack`, `hurt_by`, `provoked_by`, `pacified_by`, `repelled_by`, `inflicts`, `splits` and `evades`. Notable readers outside `engage`: the plan validator reads `provoked_by` (opening a chest in sight of piglins), the look controller reads it too (endermen and gaze), and B5 §4.6's cargo-proximity weapon rule widens from cargo to anything in the job's spare set. Adding a field is a framework change under §1.

### 3.3 A generic default with a safe floor

An unknown mob or structure gets the generic behaviour, bounded:

- `engage` with an unknown profile: keep standoff, retreat at a hurt threshold, stop at a loss cap, report what hurt the bot (damage type, source). B5 §4.2 already does this for unknown mobs ("generic melee with a retreat threshold"). The change is to make this the main path, with module-specific code reduced to optional tuning.
- `clear` and `observe` in an unknown structure: slow, wall-hugging, light the way, avoid edges, retreat at loss cap.
- Defaults respect **protected conditions**: conditions that the plan's own sources depend on. "Light the way" must not light a blaze spawner room the plan is farming (spawners stop above block light 11 `[c]`). The validator derives these from the sources' spawn conditions and passes them to the verbs.
- Every default is **budgeted**: time, attempts, bots lost. When a budget runs out the job ends `interrupted(incident)`, with what hurt the bot in the evidence and the operation re-plans or escalates (K5 §4.6, §4.7).

What happens next, including escalation and how a recurring failure gets fixed, is the interrupt contract in [INTERRUPTS.md](../INTERRUPTS.md) (AD-K5.3, adopted 2026-10-14): contain, price, escalate only on cost, and fix from a cost-ranked batch review.

The first encounter with something new costs bots. This is the price of not having per-scenario code. K7 quotes it as expected losses `[c]` until measured (§3.4).

### 3.4 Outcomes ratchet the data

Each run writes measurements back: loss per kill, time per kill, damage by type, drop rate, what the generic default struggled with. **Outcomes go to K7's rate store**, keyed by verb, profile, gear and crew size, because losses per kill depend on gear and server, not on the game version. **Game facts go to F5** and stay keyed by version: a drop rate or an attack range measured in the harness turns a `claimed` row field into `measured` (F5 §4.4). The second blaze task is cheaper than the first.

The same loop promotes behaviour: a profile whose generic handling keeps failing in the same way is where someone adds a *tuning* field (not a module), reviewed from the failure reports.

## 4. Who composes

Two composers, same output format.

**Output format: a plan.** A plan is a partial order of verb calls with parameters, preconditions, budgets and `on_fail` handlers naming other plans. It is data. It is what an HTN method produces, and it is what an LLM may submit. It is validated against verb contracts and quoted by K7 before it runs. It is not a language. The grammar is a closed list:

| construct | meaning |
|---|---|
| step | one verb call with data parameters |
| `as name` | binds a step's result (the chests an `observe` found) |
| `over(name)` | runs the steps below once per element, bounded by the set's size |
| `await(predicate, budget)` | waits on a ledger or belief predicate |
| `until` on `hold` | holds until a predicate or the budget |
| `with` | a verb that runs inside a `hold` (engage while holding the spawn floor) |
| `on_fail(plan)` | names another plan; its retries are counted against K5's recovery rules (§8) |
| scope modifiers | reflex and policy settings for the steps they wrap (CONVENTIONS §4) |

A new construct is a framework change under §1. The validator also runs two checks on every plan: **protected conditions** (§3.3) and **provocation**. For provocation, each step's action is checked against the `provoked_by` of mobs believed to be in range, and the validator either inserts a `clear` or puts the risk in the quote.

1. **Deterministic expansion.** K7 and K5 expand goals through the recipe graph and **operator data** (verbs with pre/effects/cost). This replaces "code per goal kind". Goal kinds that need no judgment (obtain items, mine, haul) stay here.
2. **LLM composition.** For tasks the operator data doesn't cover, the commander (or, with AD-K5.1 reversed, an operation officer) submits a plan of verbs. Same validation, same quote, same approval classes, same audit (K2 §4.1–4.3). A plan that succeeds repeatedly can be saved as a named routine in a pack (C11 §4.4), which is a ratchet of reusable plans. Names, not scripts.

Worked example, "16 blaze rods from chunks X,Y in the nether, a fortress is known to be there":

```
prepare(counters(blaze) + counters(nether_fortress))                  # derived; fire resistance falls through (§3.2)
travel(to: fortress(X,Y), mode: portal)                               # B1
observe(fortress, seek: spawner(blaze), budget) as room               # gate: verified in person
reflexes(attack_hostile: on):                                         # scope modifier, restored after
  clear(room, profile: blaze, loss_cap: N)                            # protects the spawner's light condition
  hold(room, band: within_16, until: ledger(blaze_rod) >= 16, budget)
    with engage(profile: blaze)
carry(blaze_rod, to: home)                                            # C3
```

No blaze-specific code. Blaze knowledge is the profile row. Fortress knowledge is the place row.

## 5. What stays code, and what this doesn't solve

- **Verb quality.** `engage` against ranged fire mobs in a corridor, `travel` over bridges and lava edges: hard physics and tactics, written once, verified in F3. This is where the engineering budget goes.
- **Pathfinding gaps** (B1's missing moves: bridging, pillaring, ladders, portals) apply to every scenario here, and are not made easier by this design.
- **First-encounter losses** (§3.3).
- **Composition quality.** An LLM can compose a plan that quotes fine and fails. The budget and loss caps bound this. They don't prevent it.
- **Irreducible hand-written data.** Counter rules and a few profile fields. If the game adds a mob, someone writes a row.
- **Field and value code.** Each interpreted field, and each new value of a field like `attack` (explosive, channelled beam, splash), is code in its reader. This is cheaper than a module per mob because values are shared, but it isn't free. The paper test's check against B5's module list (its §4) mapped every named mob onto the nine fields with new values only.
- **Named exceptions.** The ender dragon (phases tied to crystals and the portal) and the wither (arrow immunity below half health `[c]`, block-breaking explosions) stay as modules. A conditional-field grammar could cover the wither but would cost more than the module. The exception list grows only by a framework change.
- **The builder.** This design covers the swarm. The online builder has the same budget problem in another form (a named loop rule per F7 run). See the paper test §6.

## 6. Effects on existing specs (if adopted)

| Spec | Change |
|---|---|
| ARCHITECTURE §1.6, §3.6, §11 | Pointer to this doc; "methods" become operator data (§3.6) |
| K5 §4.1 | HTN methods are operator data over verbs, with code only for verb mechanisms. AD-K5.2 |
| K5 §4.10 / AD-K5.1 | Reopened: officers are the composer for non-recipe tasks |
| B5 §4.2, §4.3 | Per-mob and boss modules become profile rows plus optional tuning; generic `engage` is the main path; dragon and wither stay modules |
| B5 §4.6 | The cargo-proximity weapon rule widens to the job's spare set (neutral mobs that anger as a group) |
| B1 §4.3, B5 | The look controller reads `provoked_by: gaze` |
| B6, B3 | `use` and `trade` exposed as verbs |
| K2 §4.1 | A command to submit a plan (or `errand` extended to carry one) |
| K5 §4.6 | Recovery rules cover plans; one owner of retries with `on_fail` |
| F5 §4.3 | New key families: threat profile, place profile, counter rules, exchange rows; game-fact fields only |
| C6 §4.2, §4.4 | Route templates become plans over verbs; nether expedition is a composition |
| C5 §4.2 | Structure search uses place profiles' marker blocks |
| C7 | Plays reference verbs and profiles; doctrine for humans stays authored (§4.5) |
| C11 §4.4 | Routines (named plans) as a pack content type; still no scripting |
| K7 §4.1 | Operators carry pre/effects/cost; LLM plans are quoted the same way |
| K7 §4.2 | Costed counter alternatives, yield modifiers (looting), exchange as a source kind; outcome measurements in the rate store |

## 7. How to test the claim

Take five tasks the specs don't name. Count, for each, new verbs, new profile rows, new fields, and hand-written plan steps.

1. 16 blaze rods from a known nether fortress.
2. 3 wither skeleton skulls from a fortress.
3. 10 ancient debris from a given nether region.
4. All chests in a bastion, with gold armour.
5. Eye-of-ender supply: pearls from endermen plus rods.

**Pass:** at most one new verb across all five, no new method, and new fields per task falling across the set. Rows are expected. **Fail:** most tasks add a verb, or each task keeps adding fields. Then the verb set or the profile schema is wrong, and we adjust it before building skills. (The field clause was added after the first run, which showed that verbs alone miss where the cost goes.)

**Result (2026-10-14, [GENERALIZATION_PAPER_TEST.md](GENERALIZATION_PAPER_TEST.md)).** Passed: 0 new verbs, 0 methods, 15 rows. It also needed 9 new fields (3, 3, 0, 2, 1 per task), 5 planner features, 1 grammar construct (bindings) and 2 verbs the table had left out (`use`, `trade`). The revisions in this version come from that run.

This is a paper test against the spec set, cheap to run. A second paper run on five more unseen tasks would test whether new fields keep falling. The real test is F3 scenarios with fixed-seed fortresses (F3 §4.x already seeds them).

## 8. Open points

- Is `engage` one verb with profiles, or does ranged vs melee vs area need separate mechanisms behind it? Decide when B5 is built.
- Whether routines saved from LLM plans need human review before reuse (C11 §4.6 says person-authored for descriptors).
- How plan `on_fail` handlers interact with K5 recovery rules so the two don't both retry.
- Whether counter rules are stable enough across versions to live in F5, or belong in packs. (Profiles' game facts belong in F5; outcome measurements don't, §3.4.)
- How the commander submits a plan: a new K2 command, or `errand` extended. `errand` is already a small plan format (generic steps at markers, prepared, then triggered).
