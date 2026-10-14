# B7 Mob handling

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.10, §3.14, §10; WORKED_EXAMPLE_OSC.md §3; BREEZE_PIPELINE_FAILURES.md §1; research/builder-hard-cases.md §4.3. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** B1, B3, B5, B6, F2, C9 (ledger and cargo tags). **Used by:** C9 (capture, convoy, handover), C4 (farms needing installed mobs), C1 (entity phase).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Luring by aggro (kiting) | Measured | Follow ranges and attack ranges are game data; success rates are measured |
| 4.2 | Luring with held items | Deterministic | Which items attract which passive mobs is game data |
| 4.3 | Leads | Deterministic | Leashable mobs and lead length are game data |
| 4.4 | Boats and minecarts | Deterministic | Capacity and boarding rules are game data |
| 4.5 | Persistence | Deterministic | Name tags, vehicles and the mechanics table's rules |
| 4.6 | Breeding and villager handling | Deterministic | Breeding items, cooldowns, workstation claiming and trade locking |
| 4.7 | Killing at the right moment | Deterministic | Conditions (a villager zombified for curing, a mob at a farm's kill point) are observable |
| 4.8 | Containment patterns | Interface · authored input · person | A cage that holds a given mob through its whole behaviour set is a build design, like a redstone circuit |
| 4.9 | Cargo protection behaviour | Deterministic | Shield, back off, avoid line of sight; never target |
| 4.10 | Mob-handling reflexes | Deterministic | Registered per the reflex contract, each switchable |

## Architecture decisions

None open. Containment designs are authored inputs (§4.8), not architecture.


---

## 1. Purpose

B7 is how one bot moves living things that can't go in a chest: luring by aggro or held items, kiting into traps, leads, boats and minecarts, name tags, breeding, and killing at the right moment. It's the hands of C9's entity logistics and of any facility that needs a mob installed (a skeleton for a disc farm, villagers for a trading hall, breezes for a machine).

## 2. Scope

**Owns:** the single-bot skills for luring, leashing, loading vehicles, making mobs persistent, breeding, villager job binding and trade locking, timed kills; cargo protection behaviour while handling mobs; mob-handling reflexes.

**Does not own:** the entity ledger, cargo tags, convoys, park points and handover (C9); containment designs (they come from content packs, C11); capture operations at trial chambers (C9 with K5).

## 3. Interfaces

Skills:
- `lure_aggro(mob_uuid, to: position, keep_distance)`, `lure_item(mob_uuid | type, item, to)`
- `leash(mob_uuid)`, `unleash(at: fence)`
- `load(mob_uuid, vehicle)`, `unload(vehicle, into: cell)`
- `persist(mob_uuid, method)`
- `breed(type, n, at)`, `bind_villager(villager_uuid, workstation_cell)`, `lock_trades(villager_uuid)`
- `insert(mob_uuid, into: cell, then_close: cell)` for the builder's entity phase.

Each reports by UUID so C9's ledger updates in one step.

## 4. Requirements

### 4.1 Luring by aggro (kiting)

Keep the mob aggroed but just out of its attack reach, along a path to the trap, accounting for its follow range and speed (F5). Retry from the last good position if it loses interest. Ranged mobs (skeletons) need cover along the path.

### 4.2 Luring with held items

Passive mobs follow held breeding items (wheat for cows and sheep, seeds for chickens, carrots for pigs). Speed and follow range from game data.

### 4.3 Leads

Leash mobs that allow it (per version; villager leashability is a mechanics table entry). Lead length and break distance from game data. Tie to fences at stops.

### 4.4 Boats and minecarts

A boat has 2 seats and a minecart 1 (ARCHITECTURE §3.10), so a boat carries 2 mobs only when no bot rides it; a boat a bot rows carries 1 mob (spec audit). Load by pushing or luring into the vehicle. Bots can row a boat with a mob in it, even over land. Azalea doesn't implement riding: passenger state is a TODO and `move_vehicle` is a stub, so rowing needs physics and packet work in the fork (Azalea reality check, 2026-10-13). Vehicles cross nether portals with their passengers (1.21; with the 1.21.2 vehicle portal cooldown).

### 4.5 Persistence

Hostile mobs despawn when no player is near (instantly beyond 128 blocks, randomly beyond 32). Every held hostile mob must be persistent (vehicle, name tag, or the machine's own design) or have a bot close by at all times (ARCHITECTURE §3.10). B7 applies the method the ledger entry names and reports it.

### 4.6 Breeding and villager handling

- Breed with the right items and cooldowns.
- Villagers: a villager claims an unclaimed job site within ~48 blocks, so the workstation goes in after the villager is confined beside it; trade once to lock the profession; name-tag where persistence needs it (builder-hard-cases §4.3).

### 4.7 Killing at the right moment

Timed kills for farms and processes: a zombie villager for curing (weakness then a golden apple, not a kill), a skeleton or creeper at a disc farm's kill point, a mob at an XP farm's last hit. Conditions are observable in the entity's state.

### 4.8 Containment patterns

> **Interface input (authored).** Who: person (an LLM can draft candidates for harness testing).
> Why: holding a mob safely is a small build that must survive everything the mob does (breezes' wind charges toggle doors, trapdoors and buttons and can open their own cage; endermen teleport; slimes split); there's no general algorithm that turns a mob's behaviour into a safe cage, and each design has to be invented and tested like a redstone circuit.
> Enters the system as: containment schematics in content packs (C11), each tagged with the mob types it holds and the behaviours it was tested against in F3.
> Null's side: uses only containments a pack provides and the harness has passed; for a mob with no tested containment, capture jobs return `blocked(no_containment(type))`.

### 4.9 Cargo protection behaviour

While handling cargo (entities tagged in C9's ledger): never attack them, even when attacked; shield and back off; keep out of their line of sight except while loading; never path through a cell that would push cargo into a hazard. This uses B5's cargo exclusion filter (CONVENTIONS §4.5) plus the reflex below.

### 4.10 Mob-handling reflexes

| id | trigger | action | default | turn it off when | if off |
|---|---|---|---|---|---|
| `mob.cargo_backoff` | a cargo-tagged entity attacks the bot or targets it | raise shield and step out of its line of sight | on | inside a containment where stepping would push cargo; during a pack's insertion procedure that needs the bot to hold still | bot holds position and takes hits; `ReflexSuppressed` to the squad |
| `mob.keep_in_range` | a held mob drifts beyond the job's persistence band | move to re-enter the band | on | in convoy formation, where the squad leader positions bots | bot stays; the squad leader's formation check catches it |

Both switchable at every scope (CONVENTIONS §4.2). Cargo jobs normally switch off `combat.defend_self` (B5) and keep these two on.

## 5. Failure handling

| failure | response |
|---|---|
| Mob escapes | Report to C9 by UUID; recapture if in range; otherwise a loss |
| Mob dies in transit | Ledger loss; C9 re-orders within expected rates |
| Persistence method fails (name tag missing) | Keep a bot within range; `blocked(persistence)` |
| Lure loses aggro | Retry from last position, then `failed(lost_aggro)` |

## 6. Scale and performance

Mob handling is slow and per bot; the throughput constraints are spawners and vehicles, planned by C9 and K7.

## 7. Version-sensitive parts

Leashability, despawn rules, portal behaviour for vehicles and passengers, breeding items, villager job claiming, via F5's mechanics table.

## 8. Tests

- Lure a skeleton by aggro into a disc farm trap and make it persistent; measure success rate.
- Load two breezes into a boat with a cargo tag; the bot is attacked and never hits back.
- `mob.cargo_backoff` off at job scope during an insertion procedure: bot holds still and emits `ReflexSuppressed`.

## 9. Open questions

- Breeze behaviour inside boats (do they attack from a boat, and how often?) should be measured in harness stage S0 before the convoy is designed further.
