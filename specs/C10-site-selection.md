# C10 Site selection

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.10 (sites, presence), §3.12, §10; WORKED_EXAMPLE_OSC.md §1.1, §3. Conventions: `CONVENTIONS.md`.
**Depends on:** F2, K1 (coarse map), C5 (scouting), C8 (threats, routes), C11 (constraints). **Used by:** K7 (sites in production graphs), C2 (handover to excavation).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Constraint schema | Deterministic | Typed constraints from the pack's descriptor |
| 4.2 | Candidate generation | Deterministic | Filter the coarse map by hard constraints |
| 4.3 | Scoring | Deterministic | Weighted score with weights from the pack and policy |
| 4.4 | Concealment model | Interface · authored input · person | How detectable a site is to human players depends on how they search, which has no model to derive |
| 4.5 | Trade-offs with no dominant site | Interface · runtime decision · either | When candidates trade concealment against distance or cost, picking one is a preference |
| 4.6 | Feasibility checks | Deterministic | Presence bands against simulation distance; space and depth |
| 4.7 | Scout verification | Deterministic | Visit the top candidates and re-check constraints in person |
| 4.8 | Claim and handover | Deterministic | Claim, then hand to the site operation's excavation |

## Architecture decisions

None open. The concealment model is an authored input (§4.4).


---

## 1. Purpose

C10 chooses where things go: a farm location, a hidden machine site, a base. It searches the coarse map and scouting reports for places that meet a pack's constraints, scores them, verifies the best in person, and claims one (ARCHITECTURE §3.10).

## 2. Scope

**Owns:** constraint evaluation; candidate search; scoring; concealment estimates; feasibility checks; scouting verification requests; claims and handover.

**Does not own:** the constraints themselves (C11 packs); excavation (C2); exploring unknown areas (C5).

## 3. Interfaces

- `find_site(constraints, near?, n_candidates)` from K7 → ranked candidates with scores, per-constraint margins, and a verification plan.
- `verify(candidate)` → scouting job (C5) → verified or rejected with reasons.
- `claim(candidate)` → region claim; then `site_ready_for_prep` to C2.

## 4. Requirements

### 4.1 Constraint schema

From the descriptor (C11): spawn conditions (biome, light, space), depth and footprint, distance to supply and to targets (`within_range_of`, `max_range`), concealment budget (including bot visits per day and distance from known routes), territory, and presence bands.

### 4.2 Candidate generation

Filter regions of the coarse map by hard constraints (biome, dimension, space, range), then sample candidate origins. Unknown areas inside the range can trigger exploration (C5) before scoring.

### 4.3 Scoring

Weighted sum of distance to supply, distance to target, excavation volume (C2 estimate), danger, and concealment (4.4). Weights default from the pack, overridable by policy.

### 4.4 Concealment model

> **Interface input (authored).** Who: person (who knows how players search on these servers).
> Why: whether rival players find a site depends on how they look (flying over, tracking traffic, noticing loaded chunks, lag, entity counts, tools they use); there's no derivable model of human search, and a confident-looking number would mislead.
> Enters the system as: a concealment model the operator approves (which signals count and how much: traffic and dwell time, distance from routes, surface marks, spoil, entity and block-entity counts, loaded-chunk signs), plus each pack's own budget, which is authoritative for that facility (ARCHITECTURE §3.10).
> Null's side: computes every signal mechanically, checks the pack's budget, and reports concealment as a set of measured signals against budget, not as a single "hidden" score.

### 4.5 Trade-offs with no dominant site

> **Interface input (runtime decision).** Who: either (the LLM commander for routine farms; a person for programs like a hidden machine).
> Why: when no candidate beats the others on every constraint (one is better hidden but farther, another closer but on a known route), choosing between them is a preference about risk, not a calculation.
> Enters the system as: a choice in the quote (K7 shows the top non-dominated candidates with their trade-offs) or a policy weight set in advance.
> Null's side: if one candidate dominates, it's chosen automatically; otherwise the default is the pack's weights, and the quote shows the alternatives so the commander can override at approval.

### 4.6 Feasibility checks

- Presence bands against the server's simulation distance (reported at login): for example "load the target but stay outside its keep-out" may be impossible at a small simulation distance (ARCHITECTURE §3.10).
- Space and depth against the footprint; spawn-proofing feasible for the cavity.

### 4.7 Scout verification

Scouts visit the top candidates, re-check terrain, structures, player activity and signs of other players, and report. Verification visits count against the concealment budget too.

### 4.8 Claim and handover

The chosen site is claimed (region claim) before anything else happens there, then handed to the site operation's excavation (C2) through a gate that re-checks the claim and recent sightings (K7 §4.6).

## 5. Failure handling

| failure | response |
|---|---|
| No candidate meets hard constraints | `failed(no_site, closest_misses)` in the quote |
| Candidate discovered by others after the claim | Gate fails; escalate with re-search option |
| Claim contested by another operation | HQ arbitrates (K1) |

## 6. Scale and performance

A handful of site searches at a time; each uses the coarse map, not raw chunks.

## 7. Version-sensitive parts

Spawn rules, biomes, simulation-distance behaviour, via F5.

## 8. Tests

- Skybender pack on a harness world: candidates within range, concealment signals within the pack's budget, simulation-distance feasibility correctly computed at distances 8 and 12.
- Two non-dominated candidates: the quote lists both with trade-offs; the pack's weights pick the default.

## 9. Open questions

None at this stage.
