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
| 4.10 | Gather core and policies | Deterministic | Reach, stands and break order are geometry; a policy only filters blocks |
| 4.11 | Sight mode (no x-ray) | Deterministic | A block is known only after a clear ray from the eye reached it |
| 4.12 | Mining methods (presets) | Measured | Patterns are geometry; yield per block dug is measured per method and height |
| 4.13 | Underground hazards | Deterministic | Fluids, falling blocks and drops are game rules; the miner handles them itself |

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
- `gather(policy, quota, claim, sight)`: the generic skill (§4.10). It finds blocks the policy accepts, plans stands and the break order, breaks, collects drops. It gathers and hunts for a block type; it does not dig to find blocks.
- `mine(target: ore type | volume, quota, method, sight)`: ores and other underground targets (§4.12). A `method` (preset) digs to expose blocks; the targets it exposes go through the gather core.
- `fell(tree_type, quota, replant: bool)`: `gather` with the tree policy (§4.3).
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

*S5 form (Allen, 2026-10-19):* `fell` is the tree policy over `gather` (§4.10), not its own skill. Targets are logs of the type in natural trees inside the claim (§4.8). The policy may clear natural leaves (`persistent=false`) that block a log. A log out of reach is left and reported; nothing is placed to reach it. A target may be the floor of a stand and is then broken last (for example: stand on log 1 to reach log 8). Replant is not supported yet; 2×2 trunks, mangroves and huge fungi are refused.

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
| a log attached to leaves with `persistent=false` | natural tree (placing leaves always sets `persistent=true`, `LeavesBlock#getStateForPlacement`; worldgen trees use the default `false`, `FoliagePlacer#tryPlaceLeaf`; only commands, pistons and structure loads can make non-persistent leaves outside a tree) |
| stone, deepslate, ores, dirt, sand, gravel, with no non-natural block nearby | natural |
| blocks matching one of the jar's structure templates (`data/minecraft/structure/*.nbt`, 1,202 in 1.21.11: villages, bastions, trial chambers, ruins) | natural; mineshafts, strongholds and fortresses are generated by code and are not covered |
| a block that changed since Null last saw it, with no bot changing it (F2 change detection) | someone else's build |
| crafted blocks outside a matched template, torches, signs, beds | someone else's build |
| anything else unclear | skip and report |

### 4.9 Drops and spoil

Collect every drop within reach, report items gathered and items left (inventory full), and the cells changed.

### 4.10 Gather core and policies

Decided by Allen, 2026-10-19. `gather` is one generic skill; what to take is a policy.
- A policy only filters blocks: `target(pos)` (take it) and `may_clear(pos)` (break it because it is in the way). It holds no planning. Example policies: the tree policy (§4.3); "all stone in the claim" for bulk work.
- The core plans stands and the break order by search, not lowest-first. Reach is to the nearest point the crosshair can hit, within the block interaction range, as strict as the client or stricter. A stand may stand on a target, which is then broken last. It never places blocks.
- The core sees only its claim (Allen, 2026-10-20). It scans and reports only cells inside the claim. A tree wholly outside is not reported. A tree across the claim edge is reported and left whole, because felling only part of it is not allowed.
- One break at a time. Each break's result is read from the action mailbox (B6 §4.6); anything but a confirmed break ends the job. No retry.
- Planning is spread over ticks with a counted work budget, so a step never stalls the tick loop and the same world gives the same plan.

### 4.11 Sight mode (no x-ray)

Decided by Allen, 2026-10-19. On servers with anti-x-ray (Paper's engine modes, Orebfuscator), hidden chunk data is fake, and a path straight to hidden ores gets a player banned. Rule: in sight mode, no decision depends on a block the bot could not have seen.
- A block is known by sight when a ray from the bot's eye reached one of its faces with nothing in the way, within render range. Only exposed blocks (a face on a transparent cell) can qualify; this is the rule anti-x-ray uses, so the data for them is real.
- Light is not needed: players routinely use ore-highlighting shader packs.
- A sighting is cached in F2 (`how = seen_sight`, F2 §4.1) and stays valid until the cell changes. Re-check only after the bot moves or the chunk changes, nearest first, within the tick work budget. A block exposed by the bot's own digging counts once the server sends it.
- Visibility is per bot by default: a bot targets only what it saw itself. A swarm-wide option lets bots target each other's sightings.
- `mine` uses sight mode by default. Surface and bulk work (logs, dirt, sand, quarries) can turn it off: those targets are exposed anyway, or chosen by volume, not by hidden content. Choosing where to put a quarry from hidden ore counts is still x-ray.

### 4.12 Mining methods (presets)

Decided by Allen, 2026-10-19: `mine` takes a method. A method is an excavation pattern: it yields the next segment of cells to dig (a few at a time, within the gather core's target cap) and the core breaks them. Target ores visible on the walls within reach go through the target policy and vein following (§4.2). Presets are named methods with parameters, stored as data, for example:
- `fishbone { y, spacing: 6, branch_len, section: 1x2 }`: a main tunnel with side branches; spacing 6 leaves 5 solid blocks between branches.
- `quarry { box, top_down }`: everything in the box (sight mode off).
- `tunnel { dir, y, section, length (or unbounded) }`: a straight tunnel.
- Later: a staircase down to a height, and cave following (walk caves, take exposed ores).

Several bots on one method claim their own branches or segments, spaced so tunnels do not break into each other (§6). Yield per block dug is measured per method and height (§4.1).

### 4.13 Underground hazards

Decided by Allen, 2026-10-19: mining handles underground hazards itself and does not escalate them. Before each break the miner checks the cell it opens and that cell's neighbours: lava or water that would flow in, falling blocks (sand, gravel) above, and a drop or cave below. It handles them as a player does, including placing a block from its inventory to seal a fluid or to cover a hole. This is the first exception to "gather never places": the placed blocks are the bot's own and are reported as changed cells. Every hazard rule has a live row on both test servers.

## 5. Failure handling

| failure | response |
|---|---|
| Tool breaks | Swap (B2); if none, `blocked(no_tool)` |
| Inventory full | Return to drop-off or call a carrier (C3) |
| Lava or water breaks into a tunnel | Seal it (§4.13); if that fails, B2/B1 escape, mark the hazard, re-plan the tunnel |
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
