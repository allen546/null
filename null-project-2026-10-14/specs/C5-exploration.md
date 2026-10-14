# C5 Exploration

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.8, §3.7, §3.10 (sites), §10. Conventions: `CONVENTIONS.md`.
**Depends on:** B1, B2, B5, F2, K1 (coarse map). **Used by:** C6 (structures), C9 (trial chambers, portals), C10 (scouting candidates), C8 (intel).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Frontier exploration | Deterministic | Cover unexplored chunks by a frontier search |
| 4.2 | Targeted structure search | Deterministic | Generation rules narrow where structures can be |
| 4.3 | Stronghold triangulation | Deterministic | Two or more eye throws and geometry |
| 4.4 | Mapping and coarse map updates | Deterministic | Summaries per region into HQ's map |
| 4.5 | Scout safety | Deterministic | Policy thresholds for retreat and avoidance |
| 4.6 | Opsec while scouting | Deterministic | Rules about routes and behaviour near others, from C8 policy |
| 4.7 | Where to explore next | Deterministic | Value of information against the active goals, by a fixed scoring rule |

## Architecture decisions

None open.


---

## 1. Purpose

C5 finds things: terrain, biomes, villages, fortresses, bastions, trial chambers, strongholds, other players' activity. It fills HQ's coarse map and gives other components candidates to verify.

## 2. Scope

**Owns:** scouting jobs and patterns; structure search; eye-of-ender triangulation; map updates; scout safety and opsec behaviour.

**Does not own:** site scoring (C10); threat assessment (C8); acting on what's found.

## 3. Interfaces

- `explore(region | frontier, budget, purpose)`, `find(structure_type, near, max_distance)`, `locate_stronghold()`.
- Out: POIs and structures to F2 (with "verified in person" flags), region summaries to K1, sightings to C8.

## 4. Requirements

### 4.1 Frontier exploration

Scouts expand the explored area along a frontier, preferring routes that load the most new chunks per minute (fast travel, high ground, boats on water). Every chunk visit stamps the coarse map with time.

### 4.2 Targeted structure search

Use generation rules (F5) to restrict where a structure can occur: biome, dimension, spacing grids, height. Search the most likely cells first.

### 4.3 Stronghold triangulation

Throw an eye of ender from two or more positions far apart, record the directions, intersect them, then search near the estimate. Eyes break sometimes, so the job carries spares.

### 4.4 Mapping and coarse map updates

Summarise each region: biome, terrain, structures, resources seen, danger, player activity, last visited.

### 4.5 Scout safety

Scouts travel light, retreat at policy thresholds, avoid known hostile areas unless the job targets them, and report deaths so regions are marked dangerous.

### 4.6 Opsec while scouting

Policy from C8: don't travel in straight lines from our bases, don't linger near other players' bases unless ordered, never write coordinates in chat.

### 4.7 Where to explore next

Score candidate regions by what active goals need (a stronghold for `KillDragon`, trial chambers for a breeze program, slime chunks for a slime farm) weighted by distance and danger. The weights are configuration; the goals come from the commander.

## 5. Failure handling

| failure | response |
|---|---|
| Scout killed | Mark danger; recovery per C3; re-plan route |
| Eye of ender breaks | Use a spare; if none, `blocked(no_eyes)` |
| Structure not found within budget | `failed(not_found, area_covered)` |

## 6. Scale and performance

Tens of scouts at most; their value is coverage per hour, measured.

## 7. Version-sensitive parts

Structure generation rules and biomes per version, via F5.

## 8. Tests

- Fixed-seed world: find the nearest stronghold by triangulation within a budget.
- Find a trial chamber within N chunks of a point.

## 9. Open questions

None at this stage.
