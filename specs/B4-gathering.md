# B4 Gathering

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.5, §3.10, §4.2, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** B1, B2, B3, B5 (mob drops), B6, F2. **Used by:** C3, C4, C6, K5 supply operations.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Mining strategies | Measured | Ore distributions by height are known; yields are measured |
| 4.2 | Vein following and ancient debris | Deterministic | Flood search on ore blocks; debris by known methods |
| 4.3 | Tree felling | Deterministic | Logs, leaves and replanting follow tree shapes |
| 4.4 | Crop farming | Deterministic | Growth stages, harvest and replant |
| 4.5 | Animal farming | Deterministic | Breeding items and cooldowns are game data |
| 4.6 | Fishing | Deterministic | Cast, wait for the bite, reel |
| 4.7 | Mob drops | Deterministic | Kill with B5 and collect |
| 4.8 | Ownership check before breaking | Interface · authored input · person | Telling a player's build from natural terrain has no exact test, and the cost of a mistake is diplomatic |
| 4.9 | Drops and spoil | Deterministic | Collect, count, hand to the ledger |

## Architecture decisions

None open.


---

## 1. Purpose

B4 turns the world into items: mining, tree felling, crop and animal farming, fishing and mob drops, done by one bot as a job with a quota.

## 2. Scope

**Owns:** gathering skills and their yields; drop collection; reporting what was taken from where.

**Does not own:** where to gather and how much (K5, C3); facility farms that run as machines (C4); large excavations (C2, which uses B4's mining skills).

## 3. Interfaces

Skills:
- `mine(target: ore type | volume, quota, method)`
- `fell(tree_type, quota, replant: bool)`
- `harvest(farm_region, crop, replant: bool)`
- `breed(animal, n)`, `cull(animal, keep: n)`
- `fish(duration_gt)`
- `hunt(mob, drop, quota)`

Each returns yield, tool wear, time, and changed cells (for F2's expected changes).

## 4. Requirements

### 4.1 Mining strategies

- Strip and branch mining at the best height for the target ore from F5 distribution data; caves when safer and faster.
- Claim the tunnel region (block set claim) before mining, so two bots don't dig the same vein.
- Yields per hour by method and height are measured and fed to K7's rate store.

### 4.2 Vein following and ancient debris

- When an ore is found, flood-search connected ore blocks within reach before moving on.
- Ancient debris: strip mining at its height band; bed or TNT mining only where policy allows explosives.

### 4.3 Tree felling

Fell whole trees including high logs, leave or clear leaves per job, replant saplings when `replant`. Mega trees and huge fungi have their own patterns.

### 4.4 Crop farming

Harvest fully grown crops, replant, bone meal when the job provides it. Avoid trampling farmland (no jumping on it).

### 4.5 Animal farming

Breed with the right items, respect breeding cooldowns, cull to a keep count, collect drops.

### 4.6 Fishing

Cast, detect the bite from the bobber, reel. Treasure and junk counted.

### 4.7 Mob drops

Kill the target mob through B5's PvE skills, collect drops. Looting tools where available.

### 4.8 Ownership check before breaking

> **Interface input (authored).** Who: person (an LLM can propose rules from examples).
> Why: on an SMP a gathering bot can't know for certain whether a block is natural or part of someone's build (a log cabin, a farm, a path), and getting it wrong starts conflicts the commander didn't choose; how conservative to be is a judgment about the server's culture and the faction's relationships.
> Enters the system as: a rule set the operator approves (signals that mark a block as player-made: unnatural block combinations, crafted blocks, torches, signs, known bases from C8), with a conservatism setting per server profile.
> Null's side: applies the rules before every break outside a permit area (CONVENTIONS §3.3a); anything matching is skipped and reported, and breaking a block marked player-made outside a permit needs approval (K2 §4.4). Natural blocks can be broken anywhere without a permit (spec audit #8; Allen, 2026-10-13).

*Changed by AD-K2.2 (Allen, 2026-10-15):* this check is a default permission check on breaking, set once by a person. It needs no runtime approval: a matching block is skipped and reported. Without the world seed nobody can tell natural generation from a build for every block, and people make mistakes too. So the rule is asymmetric (Allen, 2026-10-15): **break only what is clearly natural; skip and report everything else.** A wrong skip costs little, because resources repeat. A wrong break starts a conflict. Rough signals, accurate enough:

| signal | reading |
|---|---|
| a log attached to leaves with `persistent=false` | natural tree (placed leaves get `persistent=true` `[c]`) |
| stone, deepslate, ores, dirt, sand, gravel, with no non-natural block nearby | natural |
| blocks matching one of the jar's structure templates (`data/minecraft/structure/*.nbt`, 1,202 in 1.21.11: villages, bastions, trial chambers, ruins) | natural; mineshafts, strongholds and fortresses are generated by code and are not covered |
| a block that changed since Null last saw it, with no bot changing it (F2 change detection) | someone else's build |
| crafted blocks outside a matched template, torches, signs, beds | someone else's build |
| anything else unclear | skip and report |

### 4.9 Drops and spoil

Collect every drop within reach, report items gathered and items left (inventory full), and the cells changed.

## 5. Failure handling

| failure | response |
|---|---|
| Tool breaks | Swap (B2); if none, `blocked(no_tool)` |
| Inventory full | Return to drop-off or call a carrier (C3) |
| Lava or water breaks into a tunnel | B2/B1 escape; mark hazard; re-plan the tunnel |
| Ore already gone (someone else mined it) | Update F2; `failed(target_gone)` |

## 6. Scale and performance

Hundreds of miners in one district: tunnels claimed per bot, spaced to avoid breaking into each other, and spoil handled as part of the district operation.

## 7. Version-sensitive parts

Ore distributions, crop and breeding items, tree shapes, loot tables, via F5.

## 8. Tests

- Obtain 64 iron ore with two bots in one area: no shared vein, no collision.
- Harness world with a player-built wooden hut in a forest: the felling job leaves the hut alone and reports it.

## 9. Open questions

None at this stage.
