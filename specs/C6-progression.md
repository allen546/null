# C6 Progression

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.6, §6, §7, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** K7 (production graph), K5 (HTN), B1–B5, C5. **Used by:** K7 and K5 when the goal is `KillDragon` or a gear target.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Tech tree as data | Deterministic | The recipe graph and structure requirements are game data |
| 4.2 | Route templates | Deterministic | Speedrun-style routes are well known and testable |
| 4.3 | Gear targets per role | Deterministic | Enchantments and gear per role are a table |
| 4.4 | Nether expedition | Deterministic | Fortress and bastion objectives with known methods |
| 4.5 | Stronghold and End entry | Deterministic | Triangulate, dig down, fill the portal frame |
| 4.6 | Dragon fight plan for a swarm | Measured | Known strategies; crew sizes and timings are measured in the harness |
| 4.7 | After the dragon | Deterministic | End cities, elytra and shulkers follow known methods |
| 4.8 | Readiness gates | Deterministic | Checks against gear and stock thresholds; going is a commander goal |

## Architecture decisions

None open. Beating the game follows well-documented routes.


---

## 1. Purpose

C6 holds the plans for beating the game: the tech tree from wood to the dragon, route templates for the nether, stronghold and End, gear targets per role, and the swarm's dragon fight. It's data and plan templates that K7 and K5 expand into operations and jobs.

## 2. Scope

**Owns:** the tech-tree DAG as data; route templates; gear targets; the multi-bot boss plans for the dragon and wither; readiness checks.

**Does not own:** executing jobs (K5, skills); single-bot fight behaviour (B5); deciding to start (the commander).

## 3. Interfaces

- `TechTree` data: nodes (items, structures, facilities) and edges, per version.
- `RouteTemplate{goal, operations, preconditions, typical crew, measured duration}` for K7.
- `GearTarget{role, items, enchants}`.
- `Readiness(goal)` → checklist with pass/fail and gaps.

## 4. Requirements

### 4.1 Tech tree as data

wood → stone → iron → diamonds → nether → blaze rods and pearls → stronghold → End → dragon (ARCHITECTURE §3.6), with every recipe and structure dependency from F5 data. K7 expands goals over it.

### 4.2 Route templates

Templates for: an overworld economy start, a nether expedition (fortress for blaze rods, bastion or bartering for pearls), stronghold location, End entry. Each lists the operations it becomes and the measured time.

### 4.3 Gear targets per role

Miners (efficiency, fortune or silk touch, unbreaking, mending), fighters (protection, sharpness, totems), builders, scouts. Used by C3 and B3 to equip bots.

### 4.4 Nether expedition

Portal building, fortress location (C5), blaze farming from spawners or hunting, piglin bartering with gold for pearls, return routes. Bots wear gold where piglins matter. *Proposed (AD-K5.2): each of these is a plan over the generic verbs with capabilities derived from profiles, not a separate procedure; the "gold where piglins matter" rule becomes a counter rule (proposals/GENERALIZATION.md §4).*

### 4.5 Stronghold and End entry

Triangulate (C5 §4.3), dig to the portal room, fill missing eyes, enter with the fight crew and supplies.

### 4.6 Dragon fight plan for a swarm

Destroy end crystals (ranged bots for caged crystals), split melee and ranged groups for perch phases, keep bots off the edges, bring blocks, water and slow-falling. Beds are a known damage method but explosive; policy decides. Crew size, timing and losses are measured in the harness.

### 4.7 After the dragon

End gateway travel, End cities for elytra and shulker shells, which unlock shulker-based logistics at scale.

### 4.8 Readiness gates

Before a stage starts, check gear, stock, crew and route knowledge against the template. The result goes into K7's quote; starting is the commander's goal decision.

## 5. Failure handling

A failed stage returns to the previous gate with losses recorded; K7 re-quotes. Repeated wipes go to the commander through K5's escalation with what was tried.

## 6. Scale and performance

A full `KillDragon` program is a handful of operations; most of the bots are in the economy behind it.

## 7. Version-sensitive parts

Recipes, structures, boss mechanics, bartering tables, via F5.

## 8. Tests

- ARCHITECTURE §7 step 3: reach iron and diamonds unattended from a fresh spawn.
- Full `KillDragon` on a harness world, measured against the template.

## 9. Open questions

None at this stage.
