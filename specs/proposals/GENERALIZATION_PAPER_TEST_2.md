# Generalization paper test, run 2

> **2026-10-14.** Second run of the test in [GENERALIZATION.md](GENERALIZATION.md) §7, against spec set v1.2. Paper only: nothing ran. It follows [run 1](GENERALIZATION_PAPER_TEST.md) (method, categories, style) on five different tasks, T6 to T10. The question for this run: do new fields per task keep falling on unseen tasks? Game facts come from vanilla 1.21.x as remembered, not from F5, so each one is marked `[c]` until the harness measures it. Nothing here is adopted. This run did not read or use any earlier attempt at the same question.

## 0. Baseline: what run 1 left in place

The brief says run 1's fields count as existing only if its revisions were folded into GENERALIZATION.md. I checked. In this document "GEN" is GENERALIZATION.md and "P1" is run 1. A citation with no tag is adopted v1.2 spec text. A citation to GEN or P1 is proposal text, so "already provided" there means provided on paper only.

- **Folded in.** The nine-field registry is in GEN §3.2 ("Field registry"). `use` and `trade` are in the verb table (GEN §3.1). The closed grammar list is in GEN §4. The rate store (not F5) for outcomes is GEN §3.4. The two boss exceptions are GEN §5. The five planner features are in GEN §4 (protected conditions, provocation check), §3.2 (costed counter alternatives, exchange rows) and §6 (K7 yield modifiers). The revised pass criterion is GEN §7.
- **Not folded in.** The value sets of the nine fields (`climbs`, `swims`, `channelled`, `area`, `explosive`, the `provoked_by` actions) exist only in P1 §3 and §4. GEN §3.2 names the fields but lists no values. I treat P1's value sets as existing and cite P1.
- **Not folded in.** Predicate terms. GEN §4 says `await(predicate, budget)` waits on "a ledger or belief predicate" and `until` takes "a predicate", but nothing lists the terms. P1 used `ledger(...)`, `bartered(P)` and `sight(c)` without counting them.
- **Not folded in.** Entity state. GEN §3.2's exchange rows need "an adult piglin that isn't angry, a villager with a profession". F2 §4.5 lists the per-entity snapshot (type, position, last seen, vehicle, name, persistence hint, cargo tag) and has no age, anger or profession. P1 used this state in T4 and T5 and counted no field for it.
- **Documented, with no reader.** GEN §3.2's threat-profile table has five columns that the registry does not make interpreted: `immunities`, `hazards it creates`, `habits`, `spawn conditions`, `drops`. Only `spawn conditions` (GEN §3.3, protected conditions for sources) and `drops` (GEN §6, K7 yields) name a reader. This run wants the other two at T6 and T8.
- **Pointers only.** B5 §4.2 and §4.3, F5 §4.3, K5 §4.1 and C6 §4.4 carry "Proposed (AD-K5.2)" notes. No component spec reads any profile field today.

## 1. Method

Same as P1 §1. For each task, write the plan over GEN §3.1's verbs and the GEN §4 grammar, then list every need the specs don't already provide. Each need has one kind: **row**, **value**, **field**, **planner**, **grammar**, **verb**, **method**, as P1 §1 defines them.

Differences from run 1, all aimed at the honest cost:

- **Every "given" has a citation.** No citation means it is a need.
- **A new reader of an existing field is not a field.** I list it as a *reader addition* and count it in a separate row, because P1 counted each field once however many readers it had (P1 §3 registry). The cost is real, so it is shown.
- **Predicate terms are counted** in their own row. P1 left them out. A term is a new belief or ledger read in the predicate evaluator.
- **Field versus value.** A value is a new entry in a set that an existing reader already enumerates, or a new dimension of a parameter whose reader already exists. A field is a column that no existing reader covers. Where a skeptic could argue the other way, the entry says so, and §3 gives a pessimistic recount.
- **Second pass.** Each task is read twice. The second pass asks what the planner, the counters and the targeting would actually read, and what each skill does on its own (reflexes, drops, lighting) that the plan never mentions.
- **Restated run 1.** §3 shows run 1 again on this run's bar, so the comparison is like for like.

## 2. Tasks

### T6. Night perimeter

Hold a 64×64 base area through one full night. No bot deaths, no block damage inside the area.

```
observe(area, seek: spawnable(night)) as dark                  # F2 §4.7 spawnability layer
build(spawn_proof, site: area, keep: all)                      # C2 §3 PrepRequest.spawn_proofing
prepare(counters(zombie, skeleton, creeper, spider, enderman, witch, phantom) + keep_list)
reflexes(combat.defend_self: on):
  hold(area, band: within_32, count: N, until: clock(dawn), budget)
    with engage(profile: hostile_night, standoff: derived, protect: unchanged(area), loss_cap: 0)
retreat(rally: shelter, reason: floor)
```

Given, with citations:

- Holding a region with a band and a count is the presence form: `at: region`, `band`, `count`, `phase` (ARCHITECTURE §3.10 Presence; K5 §4.3).
- `defend(position | bot, radius)` is already a B5 skill (B5 §3), so `hold … with engage` has a precedent.
- Spawnability from light and block rules is an F2 layer (F2 §4.7). Unexplained block changes in a region are an F2 event (F2 §4.9).
- Phantoms follow B2's sleep policy (B2 §4.3, P1 §4 phantom row).
- Skeleton, spider, witch, enderman and phantom map onto fields (P1 §4). Zombie and slime are in the same shape. Splitting is `splits` (P1 §3).
- The reflex scope modifier is in the grammar (GEN §4, CONVENTIONS §4.4).

Needs:

- **Lighting an area that is already built.** C2 §4.6 lights or slabs floors "as the dig goes… when the request asks for it". A base is not a dig. The algorithm exists inside C2 as a flag, not as a job on a finished area, and it is not in GEN §3.1's table. → **value**: `build` with `spawn_proofing` on a site with `keep: all`. [c] a light level above the version's spawn threshold stops overworld hostile spawns.
- **Creepers turn "no block damage" into a geometry problem.** A creeper that reaches the area and swells is a failed task even if no bot dies [c fuse 30 gt, radius 3]. `attack: explosive(fuse, radius)` (P1 §4) says how it hurts a bot. It does not say what it does to blocks. Endermen take blocks [c]. Zombies break doors on hard difficulty [c]. Burning mobs at dawn may ignite flammable blocks [c, unsure]. The targeting must rank by what a mob does to protected blocks. GEN §3.2 has a `hazards it creates` column and the registry gives it no reader. → **field** `alters_blocks` (explode(radius), take(tag), break(door), ignite). Readers: target ranking (C7 §4.2, B5 §4.6 "ranked by threat and distance"), engage standoff, and the plan validator.
- **The constraint is the task's own, not a source's.** GEN §3.3 derives protected conditions from the plan's sources. Here the task supplies one: "unchanged(area)". F2 §4.9 already detects the violation. → **value**: `protect: unchanged(region)`.
- **Standoff from a region, not from the bot.** Engage must kill an exploding mob at more than its radius from the protected edge. That reads `attack: explosive(radius)` in a place that field has not been read before (the standoff). → **reader addition**, 1.
- **Crew size and expected losses.** `count: N` has no source. K7 §4.7 computes an ETA from rates and gives no arrival rate for hostiles. GEN §3.3's validator reads `spawn conditions` only to protect a source. A forecast of the expected mob mix and rate over a window at a place, from biome, light, time and the mob cap, is what lets the quote price losses and pick N. It could be deferred if the commander sets N, but then "no bot deaths" cannot even be priced. → **planner**: threat forecast.
- **A clock term.** `until: clock(dawn)`. F2 §4.1 keeps game time for `at_gt`. No predicate list names it. → **predicate term**, 1.
- **Loss cap 0.** GEN §3.3 budgets are time, attempts and bots lost. A cap of 0 is reached only after the first death, so it prevents nothing. What prevents deaths is retreat floors (B5 §4.5) and gear from counter rows. The quote shows expected losses as `[c]` (K7 §4.3). This is not a need. It is a limit: the paper plan cannot certify zero deaths.
- **Coverage geometry.** ARCHITECTURE §3.10 and K5 §4.3 turn `at: region` into location constraints. Neither says how N bots spread over 4,096 cells with overlapping bands. Assumed to be K5's constraint scheduler. Not counted, but unspecified.

Second pass:

- The plan never says what the bots do at dawn. Zombies and skeletons burn [c] but stay hostile for some seconds. `until: clock(dawn)` ends too early. A margin is a parameter, not a need.
- Bots are players, so mobs spawn around them (ARCHITECTURE §3.10 Presence). A fully lit area pushes the spawn ring outside the area, which is a benefit and also the reason creepers arrive from outside.
- A wall and moat (C8 §4.5) is the other plan. That section calls defence layout an authored input, with structures from packs. It needs rows (a wall pack) and an author, not framework code. I chose the active plan because the task says "hold".
- Dark cells under the area (caves) are inside the footprint. `observe` must cover the volume, not only the surface. C5 `explore` covers it. The volume's vertical extent is not in the task.

Rows: zombie, skeleton, creeper, spider, witch, phantom (all new as rows), enderman and slime reused from P1, counter rows for `explosive → keep_distance | shield` and `projectile → cover | shield`, the wall pack as an option. About 9.

Tally: verbs 0, methods 0, fields 1, values 2, planner 1, grammar 0, predicate terms 1, reader additions 1.

### T7. Cattle drive

Breed 20 cows from 2, then move all 20 to a fenced pen 2 km away, across a river and a ravine.

```
prepare(wheat: derived)                                        # K7 recipe graph; harvest is B4 §4.4
build(cow_pen, at: source)                                     # pack containment, B7 §4.8
hold(source_pen, until: count(cow, adult, held: source_pen) >= 20 + reserve, budget)
  with use(wheat, on: eligible_pair)                           # breeding exchange row
observe(corridor(source_pen, P), seek: hazard(drop > safe | water > safe)) as gaps
carry(cargo: cows, to: pen P, mode: lead | lure | boat, route: cargo_safe, escort: ring)
  on_fail(plan: bridge(gaps))
```

Given, with citations:

- Breeding is a named entity source (ARCHITECTURE §3.10 Sources; C9 §4.5). `breed(animal, n)` is a B4 skill (B4 §3, §4.5) and `breed(type, n, at)` is a B7 skill (B7 §3). Wheat for cows, cooldowns and breeding items are game data (B7 §4.2, B7 §7).
- A convoy is a job with cargo UUIDs, a route, park points, an escort floor and a moving claim (C9 §3, §4.6). Formation is checked every tick: every protected entity within range of a bot (K6 §4.5, C7 §4.3).
- Lure, lead and boat are B7 skills with their limits (B7 §4.2 to §4.4). `mob.keep_in_range` and `mob.cargo_backoff` exist (B7 §4.10).
- Reserves for losses are sized from measured loss rates (C9 §4.11). `blocked(no_containment(type))` is the rule for a pen with no tested containment (B7 §4.8, C9 §4.4).
- Counts carry their age and can be filtered by where an entity is held (C9 §4.1, §3 `count(filter)`).
- `travel` has a `hazard tolerance` parameter (GEN §3.1). `goto` takes a max fall height and an avoid-region constraint (B1 §3).
- Fall hazards and water depth are F2 layers (F2 §4.7).

Needs:

- **Entities are not an item set.** GEN §3.1 gives `carry` the parameter "item set". Its owner column says C3, C9. C9's Convoy job takes `cargo: uuids`, and modes `lead` and `lure` are B7 skills. The widening is: `carry` takes an entity set and a mode. → **value** (contested). A reviewer can call this a new verb. If so, the run has 1 new verb, still within §7's limit of one. B7's other skills (`lure_item`, `leash`, `load`, `persist`, `insert`) are not in the §3.1 table either, so the table is not closed against the B skills.
- **Breeding is a source with a rate that depends on its own stock.** From 2 cows, breeding in pairs on a 6,000 gt cooldown with calves maturing in 24,000 gt `[c]` takes about 10 cycles. That is about 60,000 gt (50 minutes) and 18 breedings, so 36 wheat `[c]`. I simulated this by hand. K7 rates are "per bot-hour of presence" and fixed (C4 §4.2, K7 §4.1). A source whose rate grows with adults, with a maturation delay, has no model. → **planner**: stock-dependent source.
- **The breeding row's output is an entity that grows.** GEN §3.2 exchange rows come out as "the loot table or offer list". A calf is an entity with a timer, and its adult state is the input to the next exchange. → **value**: output kind `entity`, with growth.
- **State the specs don't keep.** Breeding needs `baby`, `in_love` and cooldown. The exchange rows (GEN §3.2 "required state") need them too. F2 §4.5 has no such state (§0). → **field** `entity_state`: age, love, profession, held item, as read from entity metadata. Readers: B7 pair selection, K7 exchange preconditions, the ledger filter. This is the run 1 debt, paid here at first use.
- **Cargo-safe ground.** B7 §4.9 says never path through a cell that would push cargo into a hazard. It does not say which hazards a cow has. Cows do not jump a 1-block step or a fence `[c]`, they take fall damage beyond 3 blocks `[c]`, a leash pull can drag one off a ledge `[c]`, and lure-following cows avoid deep water `[c]`. B1 §3 constraints cover max fall height and avoid-regions and nothing else. `movement: swims` (P1 §3) is categorical. Step height, clearance and ford depth are hand-written numbers, not in the generated data (F5 §4.2 lists mob stats, not AI limits). → **field** `safe_ground`. Readers: B1 path costs for cargo jobs, the route search.
- **Crossing edges in route search.** B1 §4.4's travel graph comes from the coarse map and known portals, roads and rails. Nothing prices "bridge this gap" or "ford here" or "take boats here" as an edge with materials and bot-hours. Rail-laying ahead (C9 §4.6) is the nearest precedent and is only for rails. A plan can instead observe the gaps and compose bridge steps, but then K7 cannot quote the planks, and the choice between a detour and a bridge is made by whoever wrote the plan. → **planner**: cargo-safe route search with priced crossing edges. T8 reuses it for void gaps.
- **A repeating `with`.** `hold … with use(wheat, on: eligible_pair)` must fire each time a pair becomes eligible. GEN §4 defines `with` as a verb that runs inside a hold, which suits `engage`. For a discrete verb it must mean "whenever preconditions hold". → **value** for `with`.
- **Missing moves.** Bridging, doors and gates are not in Azalea's pathfinder (B1 §4.1). Boat riding is not implemented (B7 §4.4). These cost every scenario and are not counted here, as in P1 §5 ("Pathfinding gaps").
- **Containment.** Without a pack pen tested for cows, `insert` returns `blocked(no_containment(cow))` (B7 §4.8). That row has to exist before the task can run.

Second pass:

- The unknowns. A river and a ravine 2 km out are not known in advance. `observe(corridor …)` needs the chunks, so C5 `explore` runs first (C5 §3). Terrain beyond view is unknown (B1 §4.1).
- Night. A 2 km drive at walking speed is about 8 minutes `[c]` and mostly daytime. If it runs into night, T6's engage rows apply, with cargo exclusion (B5 §4.6). No new need.
- The count. "All 20" means breeding to 20 plus a reserve (C9 §4.11), then culling extras (B4 §4.5 `cull`). Both exist.
- Persistence. Passive animals are not despawned in the usual case `[c]`. B7 §4.5 covers hostile mobs only. A mechanics-table row says so.

Rows: cow (passive profile), cow breeding exchange, cow pen containment, river and ravine place profiles, a bridge segment pack, a wheat farm descriptor. About 6.

Tally: verbs 0 (1 contested), methods 0, fields 2, values 3, planner 2, grammar 0, predicate terms 0, reader additions 1 (B1 reads `swims` for cargo).

### T8. Elytra

Get one elytra from an end city. The dragon is dead and the end gateway is known.

```
prepare(counters(shulker, enderman, void_edge) + gap_blocks: derived)
travel(to: gateway G, mode: gateway)
observe(outer_end, seek: end_ship, until: found, budget) as ship
travel(to: ship.approach, hazard tolerance: void + shulker_los)
collect(elytra, source: item_frame(ship))
travel(to: home, mode: gateway | exit_portal)
carry(elytra, to: home)
```

The elytra generates in an item frame on an end ship, not in the city, and not every city has a ship `[c]`. The task wording says "from an end city", so the search target is the ship.

Given, with citations:

- C6 §4.7 names "End gateway travel, End cities for elytra and shulker shells" and gives no mechanism. This task is named in the specs and specified nowhere.
- `find(structure_type, near, max_distance)` and `explore` use generation rules from F5 and fail with `not_found` (C5 §3, §4.2, §5). Structures are recognised from signature blocks (F2 §4.6). `observe` takes an `until` (GEN §3.1).
- Edge guard, `move.mlg_bucket` and a posture parameter exist (B1 §4.9, §3). The `void edge → no_sprint, edge_guard` counter is GEN §3.2's own example.
- Enderman rows: `provoked_by: gaze`, `evades`, `low_ceiling`, the pumpkin counter (P1 T2, T5). Water does not exist in the End, so `hurt_by: water` is unusable and K7's costed alternatives pick another (GEN §3.2).
- Counter alternatives are costed through the recipe graph (GEN §3.2). The reflex that falls with a bucket is `move.mlg_bucket` (B1 §4.9).
- Dimension is a place-profile column (GEN §3.2).

Needs:

- **Gateway travel.** B1 §4.4 lists walking, roads, boats, nether portals, minecart and elytra "later". It has no gateway. Its portal check ("confirm the exit matches the expected linked portal") does not fit a gateway, whose exit is a position, not a portal `[c]`. → **value**: `travel.mode: gateway`. The return route is `[c]`: the exit portal in the island centre returns to the overworld, and the outer gateway's return link is unsure.
- **Void between islands.** Outer islands are separated by void `[c]`. A bot must bridge. That is the T7 planner feature (priced crossing edges), here for void gaps, plus blocks from `prepare`. B1 bridging is a missing move (B1 §4.1), not counted. → reuse, **planner** 0 new.
- **Shulkers.** Stationary turrets `[c]` with homing bullets `[c]` that inflict levitation `[c]`. They close up and teleport when hit `[c]`. → **values**: `movement: stationary`, `inflicts: levitation`, `evades: teleport_on_hurt`. Counter row `levitation → slow_falling | milk | stay_out_of_los`. Slow falling needs a phantom membrane (T6 phantoms), so the alternatives are really costed.
- **Exposure to turrets.** Avoiding a stationary shooter means pricing its line of sight in path costs. B1 §4.3 prices cargo line of sight and enemy sightings. It reads no profile fields. → **reader addition**: B1 path costs read `attack` range and `movement: stationary`.
- **An item inside an entity.** `collect` sources are block, mob, container, ground (P1 §2 T5, GEN §3.1). An item frame is none of these. F2 does not hold what a frame contains (§0). → **value**: `collect.source: entity_slot`, and `held item` in `entity_state` (T7's field).
- **Ship as part of a city.** The place profile needs a child structure with its own markers and loot kind. That is a schema nesting. → counted as part of the rows.

Second pass:

- **Closed shulker.** A closed shulker takes much less damage `[c armor bonus]` and bounces arrows `[c]`. That is a damage rule that depends on the mob's state. It is the shape of the wither's arrow immunity (P1 §4), which P1 declared a named exception because a conditional field "costs more than a module". The generic loop (melee, retreat at a threshold) still works, slowly. So no module, but this is the second instance of a conditional state in 10 mobs (wither, shulker). See §4.
- **Chorus fruit** teleports the eater `[c]`. B2's food choice must exclude it. A mechanics-table row, not a field.
- **Return trip with the elytra.** Gliding needs fireworks (B1 §4.4, §9 "scope it when the End is reached"). The plan does not use it, so no need. The elytra's durability matters for later use only.
- **Targeting.** No hostile in the End city needs a new target rule beyond endermen (gaze, reused) and shulkers.

Rows: shulker, end_city, end_ship, outer end islands (place profile), end gateway, counter rows for levitation and void (reused edge guard), enderman reused. About 6.

Tally: verbs 0, methods 0, fields 0, values 5, planner 0 (1 reused), grammar 0, predicate terms 0, reader additions 1.

### T9. Mending librarian

Get a librarian villager whose trade offers Mending, by placing and breaking lecterns to reroll trades. Start from one unemployed villager in a village.

```
observe(village, seek: villager(profession: none)) as v
carry(cargo: v, to: cell, mode: vehicle)                       # B7 load, unload, insert; cell is a pack containment
hold(cell, band: entity_ticking, until: offers(v) includes enchanted_book{mending}, budget)
  with routine reroll_librarian(v, lectern_slot):
    break(lectern_slot)                                        # idempotent: air stays air
    await(profession(v) == none, budget)
    place(lectern, at: lectern_slot)
    await(profession(v) == librarian, budget)
    observe(v, offers)
trade(v, lock)                                                  # B7 lock_trades, one cheap trade
```

Given, with citations:

- B7 §4.6 gives the protocol: confine the villager, place the workstation after, trade once to lock the profession, name-tag where persistence needs it. B7 §3 has `bind_villager` and `lock_trades`. F5 §7 lists "villager job claiming" in the mechanics table.
- Offers are visible after opening and can be read (B3 §4.9). `trade(villager_uuid, offer_index, times)` is in B3 §3 and in the verb table.
- `place` and `break` with drop collection exist (B6 §3, §4.5). Capture, load, unload and insert exist (B7 §3, §4.4). A containment must come from a pack and pass F3 (B7 §4.8, C11 §4.1).
- Villagers are entities in the ledger and tagged cargo, so nothing attacks them (C9 §4.2, ARCHITECTURE §3.10 Entities).
- The presence band `entity_ticking` exists (ARCHITECTURE §3.10).
- Budgets on attempts exist (GEN §3.3, CONVENTIONS §3.1).

Needs:

- **Offers are not in the world model.** F2 §4.1's observation payloads are block states, container slots, entity snapshot, sighting. B3 §4.9 reads offers but nothing stores them with an age. → **value**: payload kind `offers`.
- **Profession.** The `await(profession(v) …)` terms read the villager's data from entity metadata. → uses T7's `entity_state` (value `profession`, `level`). **predicate term**, 1.
- **A predicate over an offer's contents.** `offers(v) includes enchanted_book{mending}` matches an item component. → **predicate term**, 1. It needs F5's item components.
- **A loop with a multi-step body.** GEN §4's `hold … with` takes a verb, and `over` is "bounded by the set's size". Rerolling is a sequence of five steps repeated about 40 times with the stop check after the last step, and the lectern must stay when the offer appears. Without a conditional or a mid-body exit, the body only works if `break` on an empty cell is a no-op and the `until` is checked between runs. `on_fail` as a loop is wrong: it counts against K5 §4.6's recovery rules (GEN §4 `on_fail`). → **grammar**: `with` accepts a routine (a named plan, C11 §4.4). Routines already exist as pack content, but GEN lists no `with routine`.
- **Planning a search with a probability.** About 1 in 35 to 40 rerolls shows Mending `[c]`, from the size of the tradeable-enchantment tag. K7 prices an exchange by its expected output. Here the target is a predicate over a random output. → **value**: an exchange row's target may be a predicate, and the expected number of exchanges is 1/p.
- **Timing.** A villager acquires a job site during its work time `[c]` and loses it a short time after the block breaks `[c]`. `clock(day)` is T6's predicate term, reused.

Second pass:

- **Competing claimants.** A placed lectern in a village is claimed by the nearest unclaimed villager that can path to it. The cell's walls stop others. That is a pack property and a design check, not a field.
- **Lock cost.** `lock_trades` buys the cheapest offer. Its inputs are known only after the offers are read, so the quote cannot price them in advance. Quoted as `[c]`. No new need, since the B7 skill picks the offer.
- **The villager must never have traded.** Unemployed and untraded is assumed from "unemployed". If it had traded, rerolling fails silently `[c]`. The profession and level in `entity_state` are enough to tell. XP is visible only in the trade window `[c]`.
- **Nitwits.** A nitwit cannot take a job. `profession == nitwit` is a state value. A reroll that never gets a profession ends on its budget.

Rows: unemployed villager, librarian trade pool, lectern job-site row, villager cell containment. About 4.

Tally: verbs 0, methods 0, fields 0, values 2, planner 0, grammar 1, predicate terms 2, reader additions 0.

### T10. Ancient city loot

Loot the chests of one ancient city without summoning the warden.

```
find(ancient_city, near: region, budget)                       # C5 §3
observe(city, seek: sensor_field | container) as field, chests # chunks are visible from above
prepare(counters(place(ancient_city)) + quiet_kit)             # derived
reflexes(upkeep.eat: off, move.unstuck_jump: off, combat.shield: off):
  travel(to: city.entry, posture: sneak, protect: quiet(field))
  over(chests as c):
    collect(*, source: container(c), protect: quiet(field), budget: strikes <= 2)
carry(loot, to: home)
```

Facts, all `[c]`: the warden spawns when a shrieker, triggered by a player-caused vibration, reaches warning level 4 for the same player; each shriek raises that player's level; the level decays over minutes; walking, sprinting, landing, placing, breaking, opening a container, dropping an item, eating and shooting all emit vibrations; sneaking walking does not; wool blocks stop a vibration along its path; a sensor listens within about 8 blocks.

Given, with citations:

- `find` and structure recognition by signature blocks (C5 §4.2, F2 §4.6). Known traps are an F2 hazard layer (F2 §4.7). B1 path costs read F2 hazards (B1 §4.3). `goto` has a `posture` (B1 §3). Natural blocks, including loot chests, need no permit (B4 §4.8).
- `over(binding)` with a chest list, as in P1 T4 (GEN §4). Container reading and its side effects are B3 §4.7.
- Effects gained are reported as events (B2 §3 `effect_gained`), and darkness would appear as one.
- Reflex switches at job scope (CONVENTIONS §4.2, §4.4).
- A counter table from hazard tags to capability tags with alternatives, including posture-like tags (`no_sprint`) (GEN §3.2). The provocation check on plan steps (GEN §4).
- A `danger class` column on the place profile feeds retreat floors and loss caps (GEN §3.2).
- Interrupt handling after the fact: back off, quarantine, price (INTERRUPTS §2.2, §2.3). It cannot undo a spawned warden.

Needs:

- **A second kind of provoker.** P1's `provoked_by` belongs to a mob row, and the validator checks each step against mobs believed in range (GEN §4). A sensor is a block. It provokes by event type, within a radius, unless a wool block is in the way, and what it causes is a counter that belongs to a player. No profile row holds that. → **field** `triggers` on place and hazard rows: events heard, radius, occluders, consequence. Readers: the F2 hazard layer, B1 path costs, the validator's provocation check.
- **The event vocabulary.** The validator needs a map from actions (and reflex actions) to events. → **value**: `provoked_by` and the check accept vibration events (walk, sprint, land, place, break, open_container, drop, eat, shoot). The warden itself fits `provoked_by: vibration`.
- **Silence is a condition every skill must obey.** GEN §3.3's default for unknown structures is "light the way". Placing a torch is a vibration. Protected conditions today protect light levels for sources. `quiet(zone)` has to be honoured by B1 (sneak only, no jump), B2 (no eating), B3 (no drops, B3 §4.1's "drop the lowest-value items when full" is itself a vibration), B5 (no shield raises) and B6 (no place or break near a sensor). → **value**: protected condition `quiet`. **Reader additions**: 5 (B1, B2, B3, B5, B6). This is the largest hidden cost in the set. It is a value by the rule, and the one a skeptic would call a field.
- **A budget that is not time, attempts or losses.** A chest inside sensor range cannot be silenced by sneaking. The options are a wool occluder, skipping it, or accepting a limited number of strikes per bot. Each bot has its own level `[c]`, so rotating bots spreads the strikes. The strikes are not readable from the client. They are inferred from darkness events and shriek sounds. → **value**: budget kind `strikes(per bot, decay)`, **predicate term** `strikes(bot)`, and **planner**: the quote prices expected strikes against the budget and rotates bots.
- **Refuse, don't price.** The warden is lethal in one or two hits `[c]`. GEN §3.3 says an unknown profile ends at a loss cap. That is far too late. The validator must refuse a plan that cannot show zero unmitigated triggers, using the place's `danger class`. That column exists (GEN §3.2). The refusal rule is part of the `triggers` field's reader. No separate count.

Second pass:

- **The generic default is unsafe here.** "Light the way, wall-hugging" (GEN §3.3) places torches. Any default that emits (dropping, jumping to unstick, shield raising) must check `triggers`. That is the `quiet` cost above, seen from the verbs.
- **Digging in.** The tunnel into the city breaks blocks, a vibration. The entry must start outside any sensor's range. The sensor field is visible from above because the client has the chunk data `[c]`, so `observe` finds it before the dig. No new need.
- **Wool as a counter is itself a plan.** Placing wool near a sensor is a placement event. The validator must check the counter's steps recursively. GEN §4 says the validator runs on every plan. Assumed to include inserted steps.
- **Lava and falls** in deepslate caves are F2 hazards (F2 §4.7). Deep dark has no hostile spawns `[c]`, so engage is not used.

Rows: ancient_city, warden, sculk sensor, sculk shrieker, sculk catalyst, counter rows (`vibration → sneak | occlude | stay_out`), the action-to-event map. About 7.

Tally: verbs 0, methods 0, fields 1, values 3, planner 1, grammar 0, predicate terms 1, reader additions 5.

## 3. Totals

Run 1 as recorded, then run 2. Rows past "grammar" are the ones P1 did not count.

| | T1 | T2 | T3 | T4 | T5 | run 1 | T6 | T7 | T8 | T9 | T10 | run 2 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| new verbs | 0 | 0 | 0 | 0 | 0 | **0** | 0 | 0 | 0 | 0 | 0 | **0** |
| verb-table gaps or contested verbs | | | 1 | | 1 | 2 | | 1 | | | | 1 |
| methods | 0 | 0 | 0 | 0 | 0 | **0** | 0 | 0 | 0 | 0 | 0 | **0** |
| rows | 2 | 5 | 1 | 4 | 3 | 15 | 9 | 6 | 6 | 4 | 7 | 32 |
| new fields | 3 | 3 | 0 | 2 | 1 | **9** | 1 | 2 | 0 | 0 | 1 | **4** |
| new values | 0 | 0 | 0 | 2 | 1 | **3** | 2 | 3 | 5 | 2 | 3 | **15** |
| planner features | 2 | 1 | 0 | 1 | 1 | **5** | 1 | 2 | 0 | 0 | 1 | **4** |
| grammar | 0 | 0 | 0 | 1 | 0 | **1** | 0 | 0 | 0 | 1 | 0 | **1** |
| fields + values + planner + grammar | 5 | 4 | 0 | 6 | 3 | **18** | 4 | 7 | 5 | 3 | 5 | **24** |
| predicate terms (not counted by P1) | | | | | | n/a | 1 | 0 | 0 | 2 | 1 | 4 |
| reader additions (not counted by P1) | | | | | | n/a | 1 | 1 | 1 | 0 | 5 | 8 |

P1's T3 and T5 columns show `use` and `trade`; P1 §3 listed `use` and said `trade` was a second. The T7 entry in run 2 is the `carry` widening that may be a verb.

### Run 1 restated on this run's bar

| | run 1 as recorded | run 1 restated | note |
|---|---|---|---|
| fields | 9 | 10 | `entity_state` (adult, angry, profession), first needed at T4 |
| values | 3 | 7 | the four values P1's control added (`climbs`, `swims`, `channelled`, `area`) |
| predicate terms | n/a | 2 | `sight(c)` at T4 and `bartered(P)` at T5 |
| fields + values + planner + grammar + predicate | 18 | 25 | |

Run 2 on the same bar is 28 (24 plus 4 predicate terms). Reader additions are not in either figure.

### The new fields, one line each

- **T6 `alters_blocks`.** The task forbids block damage, and nothing in the profile says what a mob does to blocks (creeper craters, endermen taking blocks, zombies breaking doors).
- **T7 `safe_ground`.** Moving cargo needs the cargo's own step, drop, width and water limits, and B1 only knows a max fall height.
- **T7 `entity_state`.** Breeding, trading and frames all need entity state (age, love, profession, held item) that F2 §4.5 does not store. This is P1's unpaid debt.
- **T10 `triggers`.** A sculk sensor is a block that provokes by event and radius, with an occluder and a per-player counter. No mob row can hold it.

T8 and T9 have no new field. T8's frame contents and T9's profession and offers read `entity_state`, which T7 paid for. In the other order they would have paid it. So per-task counts hide an order effect. The sum of distinct fields (4) is the stable figure.

### Pessimistic recount

A skeptic would move these from value to field: T10 `quiet` (five readers), T10 `strikes` (a new budget kind, an inferred counter), T8 `stationary` (exposure pricing is a cost term), T7's `carry` widening (which may be a verb). That gives fields 7, plus 1 verb if the `carry` widening is counted as one. The direction of the result below does not change.

### Is the field count falling?

- **On fields: yes, narrowly.** 9 to 4 as recorded. 10 to 4 restated. The per-task means are 1.8 and 0.8.
- **On total framework items: no.** 18 to 24 as recorded. 25 to 28 on the same bar. The new tasks moved cost from fields into values, planner features and reader additions. Reader additions (8) have no run 1 figure and are not small: T10's `quiet` touches five skills.
- **The fall is mostly a bootstrap effect.** P1's five tasks all ran in one domain (nether mobs), so they built the nine threat-profile fields in one go. Only T6 and T8 fought anything, and they reused those nine fields heavily. T10 reused one (`provoked_by`), and T7 and T9 used none.
- **Cost follows new domains.** T6 (block integrity), T7 (livestock and cargo mobility), T10 (stealth) each opened a domain and cost 1, 2 and 1 fields. T8 and T9 stayed inside domains already paid for, and both were nearly specified already (T8 in C6 §4.7, T9 in B7 §4.6 and B3 §4.9). The rate per unseen domain is about 1 to 2 fields, not near zero.
- **Not monotonic.** The sequence is 1, 2, 0, 0, 1. P1's was 3, 3, 0, 2, 1. Neither falls from task to task.

## 4. Control: do new mobs and mechanics fit the fields?

| mob or mechanic | as rows and fields |
|---|---|
| zombie | `movement: walks`, `attack: melee`, `alters_blocks: break(door)` (hard `[c]`) |
| creeper | `attack: explosive(fuse, radius)` (P1 §4), `alters_blocks: explode(radius)`, `repelled_by: cat` `[c]` |
| enderman | P1 T5, plus `alters_blocks: take(tag)` |
| witch, phantom, spider, slime | P1 §4 rows, no change |
| shulker | `movement: stationary`, `attack: ranged_projectile` (homing), `inflicts: levitation`, `evades: teleport_on_hurt`, closed-shell damage rule not expressible (below) |
| warden | `provoked_by: vibration`, melee plus ranged `[c]`, `danger class: lethal`; never engaged, the plan refuses to trigger it |
| cow | passive profile: `lured_by` and leash are mechanics-table keys read by B7 (B7 §4.2, §4.3, §7), `movement: swims`, `safe_ground` |
| villager | `entity_state` (profession, level), `provoked_by: none`, a cargo-tagged entity (C9 §4.2) |
| sculk sensor, shrieker | place and hazard rows with `triggers` |

Result:

- **No module is needed.** Every new mob maps onto fields and values.
- **One field came from the task, not the mob.** `alters_blocks` exists because T6 forbids block damage. P1's control did not see it since P1's tasks never protected blocks. A control that only asks "does the mob fit" misses constraints a task adds.
- **A second conditional.** The shulker's closed shell has the shape of the wither's arrow immunity (P1 §4, GEN §5). Two cases in about 12 mobs is a pattern. If a third shows up (guardian beam only while open `[c]`, breeze wind charges while charging `[c]`), a small condition grammar on rows may cost less than modules. I did not add it. The generic loop copes with the shulker.
- **The warden is a refusal, not a fight.** The generic default (retreat at a hurt threshold) does not survive a one-hit kill. That makes the profile's `danger class` a hard gate, not a tuning input.

## 5. What this changes in the proposal

Recommendations only. This run edits no other file.

1. Fold P1's value sets into GEN §3.2's field registry. They exist only in P1.
2. Add a predicate vocabulary to GEN §4 as a closed list (ledger counts, clock, belief state, offers, profession, strikes), and count new terms.
3. Add `entity_state` to the F2 §4.5 snapshot and to GEN's registry. P1 depended on it without a field.
4. Decide whether `carry` takes entity cargo or a verb is added (`herd`). Both are cheap now and costly later. Also list how each B7 skill maps onto the verb table.
5. Close the list of budget kinds (time, attempts, losses, and now strikes and block damage) the way GEN §4 closes the grammar.
6. Make `quiet(zone)` an explicit constraint in CONVENTIONS §3.1 and §4.4 next to `reflexes_off`, because five skills must honour it.
7. Give `hazards it creates` and `habits` readers or drop the columns. Today they are documented and unread.
8. Revise §7's criterion. "Fields per task falling" is a weak signal: the sequence is not monotonic, it depends on order, and it ignores values, planner features, predicate terms and reader additions. Better: new items per task on a fixed bar, plus the count of new domains each task opens.
9. State whether a conditional-state field is in or out once a third case appears (§4).

## 6. What a paper test can't tell

- **Whether the cost of an item is small.** A field with one reader and a field with five readers count the same here. T10's `quiet` is five skills. The unit is items, not engineering days.
- **Whether the fields make `engage` fight well.** F3 with fixed seeds decides that, and first-encounter losses (GEN §3.3) are unmeasured. T6's zero deaths and T10's zero triggers are not certifiable on paper.
- **The game facts.** Every `[c]` needs probing before anything critical depends on it (F5 §4.4). The ones that matter most: creeper and enderman block damage, shulker closed-shell damage, calf timing, the warden's counter and decay, vibration events and wool, gateway return links.
- **Task selection and counting bias.** The tasks were set by the requester, but the plans, the categories and the field-versus-value calls were made by one author who has read the proposal. Two of the five tasks are close to already specified (T8 and T9), which lowers their cost. §3's pessimistic recount shows how sensitive the field count is. Someone who did not write the proposal should recount.
- **Domain coverage.** Five tasks cannot show the rate of new domains in the real task stream. The honest reading is about 1 to 2 fields per unseen domain, and the number of domains is unbounded.
- **Whether the plan grammar holds.** The `with routine` form and idempotent `break` are chosen to avoid a conditional. Implementing them may show that a conditional is cheaper.
- **Plan quality** when an LLM writes the plans, and whether the validator can check inserted counter steps (T10).
- **Missing moves.** Bridging, gates and boat riding (B1 §4.1, B7 §4.4) bite T7, T8 and T10. They are the same gaps P1 §5 noted, so this design does not make them cheaper.
