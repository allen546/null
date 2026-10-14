# C4 Production

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.5, §3.10 (facilities, presence), §3.11, §10; WORKED_EXAMPLE_OSC.md §1.1. Conventions: `CONVENTIONS.md`.
**Depends on:** C11 (descriptors), C1 (to build), C3 (inputs and outputs), K5 (presence scheduling), F2. **Used by:** K7 (rates for quotes), K1 (rates in the digest).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Facility lifecycle | Deterministic | States and transitions are defined (§3.10) |
| 4.2 | Rates: claimed and measured | Measured | Rates start as claims and become measurements |
| 4.3 | Presence | Deterministic | Location constraints from the descriptor, scheduled by K5 |
| 4.4 | Inputs and outputs | Deterministic | Feed and collect on schedules from the descriptor |
| 4.5 | Upkeep | Deterministic | Fuel, tools, replacement mobs and block repairs |
| 4.6 | Health checks against the schematic | Deterministic | Compare the built facility with its schematic and repair diffs |
| 4.7 | Diagnosing an underperforming facility | Interface · runtime decision · either | When the build matches its schematic and the rate is still low, the cause is an open question |
| 4.8 | Interaction between facilities | Measured | Shared mob caps and spawn suppression near bots are measured, not guessed |

## Architecture decisions

None open.


---

## 1. Purpose

Facilities are capital (ARCHITECTURE §3.10): farms, smelting arrays, mob farms, storage systems, built once and then producing a rate while a player is near. C4 runs them: brings them into service, keeps them supplied and attended, measures what they actually produce, and notices when they stop producing.

## 2. Scope

**Owns:** facility state; running facilities (presence, inputs, outputs, upkeep); rate measurement; health checks against the schematic; diagnosis tickets.

**Does not own:** choosing to build a facility (K7's build-vs-buy); building it (C1); descriptors (C11); the presence scheduler (K5, which C4 feeds constraints).

## 3. Interfaces

- `Facility{id, pack, descriptor, location, state, rate_claimed, rate_measured, presence, inputs, outputs, last_checked}` registered with K5 and visible to HQ.
- From K7: production quotas per facility.
- To K5: presence constraints (§3.10 form), input delivery needs, output collection jobs.
- To K1: rates and state per facility, summarised.

## 4. Requirements

### 4.1 Facility lifecycle

`planned → building → running ⇄ degraded → destroyed`. Transitions come from C1 (build complete), C4's checks (degraded), and observation (destroyed). A facility in `running` with no presence is reported as `idle`, not `degraded`.

### 4.2 Rates: claimed and measured

- Every rate starts as the descriptor's claim (`[c]`). Once a facility has run for a measurement window with its presence met, the measured rate (`[m]`) replaces the claim for this facility and is written back to the pack's rate store with conditions (F3 §4.10).
- Rates are per bot-hour of presence, in game time.

### 4.3 Presence

- Descriptors state presence in the fixed form (`at`, `band`, `count`, `phase`, `outside`) (ARCHITECTURE §3.10). C4 turns each into a constraint for K5.
- Presence bots can do other work nearby (sorting, crafting, guarding).
- Facilities with their own chunk loaders need only the descriptor's visit presence (`visit_after_restart`, periodic check).

### 4.4 Inputs and outputs

Feed inputs (fuel, bones, items) and collect outputs on the descriptor's schedule, or when the facility's output containers are believed near full. Outputs go into the ledger with exact item types (discs per type, not "any disc").

### 4.5 Upkeep

Fuel, replacing worn tools in the facility, replacing lost mobs (via C9 or B7), and repairing broken blocks found by health checks.

### 4.6 Health checks against the schematic

On each visit, compare visible blocks against the facility's schematic. Differences go to C1 for a repair plan (redstone regions are repaired with the builder's guarantees, not patched by hand).

### 4.7 Diagnosing an underperforming facility

> **Interface input (runtime decision).** Who: either (an LLM can work through likely causes from the descriptor, the server profile and measurements; a person for anything needing a redesign).
> Why: if the facility matches its schematic and has its presence, but the rate is well below its claim, the cause could be server settings (simulation distance, gamerules), mob caps shared with nearby bots, chunk boundaries, a version change, or a wrong claim; finding it is investigation, not lookup.
> Enters the system as: a diagnosis on the ticket, resolved as a corrected rate (accept the lower number), a descriptor change, a presence change, a pack update, or decommissioning.
> Null's side: keeps running at the measured rate (K7 quotes with the measured number), files one ticket per facility with the measurement history, the schematic diff (empty), presence logs and nearby facilities, and never "tweaks" the build itself.

### 4.8 Interaction between facilities

Mob farms near each other share mob caps, and bots near a farm (as players) change where mobs spawn. Placement rules (C10) take measured interference into account; C4 measures it.

## 5. Failure handling

| failure | response |
|---|---|
| Facility destroyed | State `destroyed`; HQ informed; K7 re-quotes dependent programs |
| Presence lost | `idle`; K5 restores presence |
| Output containers full | Collection job priority raised |
| Installed mob lost | Upkeep job to replace via C9/B7 |

## 6. Scale and performance

Dozens to hundreds of facilities at large scale. Each is cheap to track; presence is the costly resource, scheduled by K5.

## 7. Version-sensitive parts

Farm mechanics change between versions (spawning, AI, dupers). Facilities from packs that fail acceptance on the running version are not started (F5 §4.8).

## 8. Tests

- Disc farm from a pack: built, skeleton installed, presence met, rate measured over a window, `[c]` replaced by `[m]`.
- Harness removes a block from a farm: health check detects it; C1 repairs it.
- Harness duper facility with one mistimed piston: near-zero rate, one diagnosis ticket, no repeated rebuilds.

## 9. Open questions

- Should C4 automatically decommission facilities that cost more upkeep than they produce, or always ask? Default to reporting in the digest.
