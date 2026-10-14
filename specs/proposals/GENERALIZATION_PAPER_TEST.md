# Generalization paper test

> **2026-10-14.** Runs the test in [GENERALIZATION.md](GENERALIZATION.md) §7 against spec set v1.2. Paper only: nothing ran. Game facts come from vanilla 1.21.x as remembered, not from F5, so every one is marked `[c]` until the harness measures it. Allen's constraint stands: there is no budget to write code for every scenario.

## 1. Method

For each task, write the plan over §3.1's verbs, then list everything a step needs that the specs don't already provide. Each need is one of:

| kind | what it is | cost |
|---|---|---|
| **row** | a profile, counter rule or exchange entry | data |
| **value** | a new value for a field that code already reads (a new attack kind, say) | a small piece of code in the reader |
| **field** | a new column that some code must read | code, written once, shared by every row that uses it |
| **planner** | a generic feature in K7 or the plan validator | code, once |
| **grammar** | a new construct in the plan format | code, once, and a step towards a language |
| **verb** | a new job kind | an architecture change |
| **method** | code for one goal kind or one scenario | what the rule forbids |

§7's pass criterion only counts verbs and methods. The first review warned that cost would hide in fields, so this run counts all seven.

## 2. Tasks

### T1. 16 blaze rods from a known nether fortress

```
prepare(counters(blaze) + counters(nether_fortress))
travel(to: fortress F, mode: portal)
observe(F, seek: spawner(blaze)) as room
clear(room, profile: blaze, protect: spawner_conditions(room), loss_cap: N)
hold(room, band: within_16, until: ledger(blaze_rod) >= 16, budget)
carry(blaze_rod, to: home)
```

- **Fire resistance is circular.** The counter rule says `fire damage → fire_resistance | shield | cover`. Brewing fire resistance needs a brewing stand, which needs a blaze rod `[c]`. So `prepare` can't just derive a capability. K7 has to treat the alternatives as choices, cost each through the recipe graph, and drop the ones that can't be made. (It may also find the potion another way: piglins barter it, T5.) → **planner**: costed counter alternatives.
- **The generic default would switch the source off.** A blaze spawner stops spawning above block light 11 `[c]`. §3.3's default for structures ("light the way") would shut it down. The validator has to protect conditions that the plan's own sources depend on. Every farm plan needs the same thing. → **planner**: protected conditions.
- Blazes hover and shoot, so melee only lands when one comes close. `engage` reads **movement** (`flying`) and **attack kind** (`ranged_projectile`). → 2 **fields**.
- Snowballs and water hurt blazes `[c]`, which gives a cheap ranged counter. → **field** `hurt_by`.
- Drops are 0–1 rods, and each looting level raises the maximum by 1 `[c]`. That's about 32 kills without looting.

Rows: blaze, nether_fortress. Verbs 0, methods 0.

### T2. 3 wither skeleton skulls from a fortress

Wither skeletons spawn across the fortress, not from spawners `[c]`.

```
prepare(counters(wither_skeleton, zombified_piglin, magma_cube) + counters(nether_fortress)
        + yield(wither_skeleton_skull))
travel(to: fortress F, mode: portal)
observe(F, seek: spawn_floor(wither_skeleton)) as ground
hold(ground, band, until: ledger(wither_skeleton_skull) >= 3, budget)
  with engage(profile: wither_skeleton, spare: [zombified_piglin], loss_cap: N)
carry(skulls, to: home)
```

- **Looting changes the cost.** Skulls drop at 2.5%, plus 1% per looting level `[c]`. That's about 120 kills without looting and about 55 with Looting III. Whether to enchant first is a make-or-buy choice over a yield modifier. K7 §4.2 already compares hand production with building a facility, so this adds a new term to that comparison. → **planner**: yield modifiers.
- **Neutral mobs share the area.** Zombified piglins are neutral and anger as a group when one is hit `[c]`. A sweep at a wither skeleton beside one starts a group fight. B5 §4.6 already has this shape in the cargo-proximity weapon rule (no sweeps, no shots with a protected entity in the line of fire). Widening "cargo" to "anything a weapon could provoke" reuses that rule. → **field** `provoked_by` (value `hit`, with group radius).
- Wither skeletons inflict Wither `[c]`. → **field** `inflicts`, plus a counter row `wither → milk_bucket | retreat`.
- They are 2.4 blocks tall `[c]`, so a 2-high tunnel keeps them out. Height is game data, so this needs only a counter row: `height > 2 → low_ceiling`. T5 reuses it.
- Magma cubes split on death `[c]`, which affects kill counts and target choice. → **field** `splits`.
- Hours of presence, food and repair are already covered by K5 §4.3 and B2.

Rows: wither_skeleton, zombified_piglin, magma_cube, two counter rows. Verbs 0, methods 0.

### T3. 10 ancient debris from a nether region

```
prepare(counters(place(nether, y 8..22)), tools: diamond_pickaxe)
travel(to: region R, mode: portal)
collect(ancient_debris, 10, source: block, method: strip | explosive)
carry(debris, to: home)
```

- **It's a place, not a structure.** The region is a height band in the nether, with lava lakes at sea level (y 31) and hidden lava pockets `[c]`. The structure profile becomes a **place profile**: a structure, a biome or a height band. This changes the row type but adds no field.
- Strip mining, vein following and the lava-face check are B4 §4.1–4.2 and B2 §4.6. That's mining quality, written once.
- **Explosive mining needs a click with an item on a block.** It means right-clicking a bed (beds explode in the nether) or lighting TNT with flint and steel. The same click lights a portal in T1 when none exists, and opens doors. It's missing from §3.1's verb table, but B6 has it (click choice) and errands have "press". → **table omission**: `use(item?, on: block | entity)`. Explosives are governed by policy (B4 §4.2), and B2 §4.3 forbids beds in the nether. The two settings already exist; the operator picks.
- The explosive sequence (tunnel, place, ignite, step back, collect) is a **routine**: a named plan over `over(positions)`, not a method. Blast safety is B6 code, written once.

Rows: one place profile. Verbs 0 (plus 1 omission), methods 0.

### T4. All chests in a bastion, with gold armour

```
prepare(counters(piglin, piglin_brute, hoglin) + counters(bastion))   # gold armour is derived
travel(to: bastion B)
observe(B, seek: container) as chests
over(chests as c):
  collect(*, source: container(c))      # validator inserts clear(sight(c), piglin) where needed
carry(loot, to: home)
```

- **Gold armour is derived, not written.** The piglin row says `pacified_by: wearing(gold armour, 1 piece)` `[c]`, and `prepare` reads it. → **field** `pacified_by`.
- **Opening a chest provokes piglins.** Piglins that see a player open a container, or break gold blocks, gilded blackstone or nether gold ore, turn hostile `[c]`. This is T2's `provoked_by` field with action values (`open_container`, `break(tag)`) and a sight condition. The validator checks each step's action against the `provoked_by` of mobs believed to be in range. It either inserts a `clear` or puts the risk in the quote. → **planner**: provocation check. The field is reused from T2.
- Piglin brutes and hoglins are always hostile. Hoglins avoid warped fungus `[c]`. → **field** `repelled_by`.
- **The chests are unknown until observed.** §4 has `over(set)` but no way to name an observation's result. → **grammar**: bindings (`as`).
- Loot chests in generated structures are natural blocks, so they need no permit (ARCHITECTURE §4.2). The structure row's `loot_containers` lets B4 §4.8's ownership rules recognise them.
- Lava in treasure bastions and gaps on bridges are hazards in the structure row. B1 §4.3 path costs already read F2 hazards.

Rows: piglin, piglin_brute, hoglin, bastion. Verbs 0, methods 0.

### T5. Eye-of-ender supply

Each eye needs 1 pearl and 1 blaze powder (half a rod). T1 covers the rods. For pearls, K7 picks a source by cost:

- (a) **Hunt endermen in a warped forest:**
  ```
  prepare(counters(enderman) + counters(place(warped_forest)))
  travel(to: W, mode: portal)
  hold(W, band, until: ledger(ender_pearl) >= n, budget) with engage(profile: enderman)
  ```
- (b) **Barter with piglins** (gold armour from T4):
  ```
  over(batches(gold_ingot)):
    carry(gold_ingot, to: ground_near(piglin P))
    await(bartered(P), 200 gt)
    collect(*, source: ground)
  ```

Findings:

- **Endermen are provoked by being looked at** `[c]`. That isn't a plan step: the look controller (B1, B5) has to read it, the same way B1 §4.3 handles line of sight to cargo. → a new value for `provoked_by` (`gaze`), plus a counter row `gaze → carved_pumpkin | look_away`.
- Endermen teleport away from projectiles `[c]`, so `engage` must drop ranged attacks. → **field** `evades`. Water hurts them (`hurt_by`, from T1). They are 2.9 tall `[c]`, so T2's `low_ceiling` row applies.
- **Bartering is an exchange.** Items go in, an entity takes time, and items come out from a loot table. Ender pearls have weight 10 of 459, at 2–4 pearls per hit `[c]`, which is about 0.065 pearls per ingot `[c]`. K7's source kinds (recipes, facilities, entity acquisitions, site searches) have no such kind. Villager trading (B3 §4.9) has the same shape. → **planner**: an exchange source kind, with rows for piglin bartering and villager trades. Piglins also barter fire resistance potions `[c]`, so once exchanges are a source, K7 finds the way around T1's circular counter by itself.
- `carry` to ground near an entity and `collect(source: ground)` are parameter values. B4 §4.9 already picks up drops.
- A standing supply is a facility (an enderman farm, a barter pen). That's C11 content.

Rows: enderman, warped_forest, piglin_barter. Verbs 0, methods 0.

## 3. Totals

| | T1 | T2 | T3 | T4 | T5 | total |
|---|---|---|---|---|---|---|
| new verbs | 0 | 0 | 0 | 0 | 0 | **0** |
| verbs missing from the §3.1 table that the specs already have | | | `use` | | | **1** (`trade` in B3 is a second) |
| methods | 0 | 0 | 0 | 0 | 0 | **0** |
| rows | 2 | 5 | 1 | 4 | 3 | **15** |
| new fields | 3 | 3 | 0 | 2 | 1 | **9** |
| new values for existing fields | | | | 2 | 1 | 3 |
| planner features | 2 | 1 | 0 | 1 | 1 | **5** |
| grammar | 0 | 0 | 0 | 1 | 0 | **1** |
| routines (named plans) | 0 | 0 | 1 | 0 | 0 | 1 |

**Verdict: pass on §7's criterion** (no new verb, no new method), **with costs §7 didn't count.** The cost moved into 9 fields, 5 planner features and 1 grammar construct. All of these are framework code, written once and shared. None of them is tied to one scenario.

The field registry:

| field | read by | used by (this test) |
|---|---|---|
| `movement` (walks, flying, climbs, swims) | engage, B1 pursuit | blaze, phantom (§4) |
| `attack` (melee, ranged_projectile, explosive, area, channelled) | engage, B5 §4.5 shields | all |
| `hurt_by` | engage weapon choice, prepare | blaze, enderman |
| `provoked_by` (hit+group, gaze, open_container, break(tag), light) | B5 §4.6 filter, look controller, plan validator | zombified piglin, piglin, enderman, spider |
| `pacified_by` | prepare | piglin |
| `repelled_by` | prepare, clear | hoglin |
| `inflicts` | prepare, B2 §4.2 | wither skeleton, stray, witch |
| `splits` | engage target and kill count | magma cube, slime |
| `evades` | engage weapon choice | enderman, breeze |

**The signal that matters is that new fields per task fall: 3, 3, 0, 2, 1.** `provoked_by` is reused in three tasks and `hurt_by` in two, and by T5 almost every need is a row. If the next five unseen tasks keep adding about one field each, the design is generalising. If they add three each, it isn't, whatever the verb count says.

## 4. Control: B5's per-mob module list

B5 §4.2 names a module per mob. Each maps onto the fields above:

| mob | as rows and fields |
|---|---|
| creeper | `attack: explosive(fuse, radius)`; B5's `combat.dodge` already exists |
| skeleton, stray | `attack: ranged_projectile`; stray `inflicts: slowness` `[c]` |
| enderman | T5 |
| spider | `movement: climbs`; `provoked_by: light(dark)`, neutral in daylight `[c]` |
| witch | `attack: area` (splash), `inflicts` |
| drowned | `movement: swims`; some carry tridents, so `attack: ranged_projectile` |
| piglin, hoglin | T4 |
| guardian | `attack: channelled` (a beam that needs line of sight for about 2 s `[c]`); counter `break_line_of_sight` |
| phantom | `movement: flying`; spawning is B2's sleep policy |
| breeze | `evades: projectiles` `[c]`; wind charge knockback as an `attack` value; cargo handling as now |

**Two don't fit:**
- **The ender dragon.** Its phases depend on the crystals and on the portal, which no row can describe.
- **The wither.** Below half health it becomes immune to arrows `[c]`, and its explosions break blocks. A conditional field (`when hp < 50%: evades projectiles`) would cover the first, but it adds a condition grammar to rows, which costs more than a module.

Both are **named exceptions**, as B5 §4.3's proposed note already allows for the dragon. The control adds new values (`climbs`, `swims`, `channelled`, `area`) but no new field.

## 5. What this changes in the proposal

These are folded into GENERALIZATION.md (revised 2026-10-14):

1. **The budget unit is interpreted fields and values, not verbs alone.** The rule becomes: a new scenario may add rows, plus values that code already handles. A new field, a new interpreted value, a new verb or a new plan construct is framework code. It is reviewed like a component change and must serve more than one row, or be a named exception.
2. **A field registry** (§3 above). Each field names the code that reads it.
3. **Verb table fixes.**
   - `use(item?, on: block | entity)` and `trade` are added. Both already exist in B6 and B3.
   - `collect` sources include `ground`, and `carry` destinations include `ground_near(entity)`.
   - `set_reflexes` is a scope modifier on plan steps, not a step.
4. **The plan grammar is a closed list:** step, `as` binding, `over(binding)`, `await(predicate, budget)`, `until` on `hold`, `with` (a verb run inside a `hold`), `on_fail(plan)`, and scope modifiers (reflexes, policy). Anything else is a grammar change under rule 1.
5. **Five K7 and validator features:** costed counter alternatives, protected conditions, yield modifiers, the provocation check, and an exchange source kind.
6. **Generic defaults respect protected conditions.** "Light the way" doesn't light a spawner the plan farms.
7. **The ratchet writes to K7's rate store,** keyed by verb, profile, gear and crew. It doesn't write to F5. F5 keeps game facts about mobs and places, by version. Outcomes like losses per kill depend on gear and server, not version.
8. **Named exceptions:** the dragon and the wither stay modules.
9. **§7's criterion is extended:** at most one new verb, no method, and new fields per task falling across the set.

## 6. What a paper test can't tell

- **Whether `engage` fights well when driven by these fields.** That's the F3 test with fixed-seed fortresses, and the losses on first encounters it measures.
- **The game facts.** Every `[c]` above needs probing before anything critical depends on it (F5 §4.4).
- **Plan quality** when an LLM writes the plans.
- **The builder.** This test covers Null's swarm specs only. The online builder has the same problem in another form: F7 runs 2–8 each added a named rule (K1 in four versions, K2, K3, L1, M1, the K1 ladder, support temps). The same rule there would read: a new schematic may add mechanics rows, not loop rules. Sorting its existing rules into game facts, the access model and performance is the next step if the builder is in scope. That belongs to Allen's builder agent.
