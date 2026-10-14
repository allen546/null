# C1 Builder

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.9, §3.12, §6, §8, §10; ONLINE_BUILDER.md v4.1; reference/builder-hard-cases.md. Conventions: `CONVENTIONS.md`.
**Depends on:** B6 (placement, replay executor), B1 (replay mode), B7 (entity insertion), F3/F5 (simulator, data), C3 (items), K5 (claims, presence). **Used by:** K5 build operations, K7 (bills of materials for quotes).

**Scope note.** Allen's separate builder agent owns the builder's placement and ordering internals: the simulator, trials, certification, ranking and repair search. This spec covers only the **interface** the rest of Null calls, the **survival seams** that interface must carry, and the **judgment zones** where the builder needs a person or an LLM. It deliberately says nothing about how plans are found.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Plan request and plan contract | Deterministic | A typed request and a typed plan or stuck report |
| 4.2 | Survival seams in the contract | Deterministic | Items, break time, drops and presence per step are fields |
| 4.3 | Multi-bot build modes | Deterministic | The interface states regions and exclusivity; how the builder partitions is internal |
| 4.4 | Execution, progress and divergence | Deterministic | Step reports, halts and repair requests over a fixed protocol |
| 4.5 | Pre-build report and unreachable states | Interface · runtime decision · person | Choosing a substitution or a workaround for a state no placement reaches is redstone design |
| 4.6 | Macro library | Interface · authored input · person | Macros are hand-invented mechanisms for inexpressible targets (ONLINE_BUILDER §6) |
| 4.7 | Stuck reports | Interface · runtime decision · either | Search ran out of budget; getting past it needs creative rework, not more search |
| 4.8 | Schematic defects and intent | Interface · runtime decision · person | Whether a non-update-stable state is intended or a bug is the schematic author's intent |
| 4.9 | Hazard model corrections | Interface · authored input · person | ONLINE_BUILDER §7 makes these deliberately human-reviewed |
| 4.10 | Entity acceptance tolerances | Interface · authored input · person | "Close enough" for mobs, frames and carts is the user's intent, not a computable fact |
| 4.11 | Arming steps and approval | Interface · runtime decision · person | Making a machine live is a policy class needing approval (ARCHITECTURE §3.12, §4.2) |
| 4.12 | Schematic intake (the builder never designs) | Deterministic | Schematics come from content packs or the operator; design is upstream (C11 §4.5) |

## Architecture decisions

### ⚑ AD-C1.1 How does building with many bots keep the guarantee?

- **Options:**
  - (a) One certified bot per redstone region, and many bots on inert regions certified independently. Regions count as independent only when nothing in one can reach the other.
  - (b) Certify the interleaving of several bots' packet streams. Exact, but certification gets much more expensive.
  - (c) Single-bot certification only; parallelism comes from suppliers.
- **Recommendation:** (a), with a separation rule from the hazard model (power, fluid, vibration and entity reach) deciding when regions are independent.
- **Why it needs Allen:** It's the open question in ARCHITECTURE §3.9, it sits inside Allen's builder agent's design, and it sets how fast big builds go.
- **Settled by:** Allen together with the builder agent.
- **If decided late:** The plan contract already carries region and exclusivity, but the planner's internals don't change cheaply.

### ⚑ AD-C1.2 How do items reach a builder bot during a certified plan?

- **Options:**
  - (a) Items arrive only at step boundaries, handed over at the entry area by suppliers who never enter the margin. Restock walks are part of the builder's certified moves.
  - (b) Suppliers deliver into the work envelope mid-build. Faster, but a supplier is a foreign entity the replay must pause for, and outside the certificate.
  - (c) The builder carries everything for a module (36 slots plus shulkers), and modules are sized to fit.
- **Recommendation:** (a) combined with (c): modules sized to fit an inventory, restocked at module boundaries at the entry area.
- **Why it needs Allen:** It's the seam between Allen's builder agent's certified replay and Null's logistics, and both sides must agree before the survival builder exists.
- **Settled by:** Allen together with the builder agent.
- **If decided late:** The creative prototype hides this completely, so it's the likeliest surprise when switching to survival.


---

## 1. Purpose

The builder is a capability service: schematic in, certified plan or stuck report out (ARCHITECTURE §3.9, ONLINE_BUILDER §0). Operations call it to get ordered steps with per-step bills of materials, stands, temps and claims, then hand those steps to builder squads, which replay them on the live server. Its product is a guarantee: no step on the live server causes lasting unintended change.

## 2. Scope

**Owns (this spec):** the request and plan contract; survival fields in the contract; multi-bot build modes as seen from outside; the execution protocol; the reports that need human or LLM input and the channels that input comes back through.

**Owned by Allen's builder agent (not specified here):** the simulator and fake player, trials, the classification of changes, ranking, access witnesses, repair search, certification replays, the pre-pass compiler, arming-set computation.

**Does not own:** materials (C3 stages them); site preparation (C2); deciding what to build (the commander); designing schematics (C11 §4.5).

## 3. Interfaces

### 3.1 Request

```
PlanRequest {
  schematic: ref (content pack or operator upload), version, rotation/mirror,
  site: { origin, dimension, work_envelope, margin },
  tier: C (vanilla survival only),
  mode: certified | decorative_parallel,        # 4.3
  bots_available: n, items_available: summary,   # for parallel partitioning and early BOM checks
  constraints: { arming: from descriptor, keep_out, presence_bands, max_bots_in_redstone_region: 1 },
  budget: { planning_wall_s, trials? }           # builder interprets trials
}
```

### 3.2 Plan

```
Plan {
  certificate: { version, server_profile, assumptions, variants_passed },
  steps: [ {
     id, region, exclusive: bool,
     items_needed: [{item, count}], items_returned: [{item, count}],   # temps and drops
     break_time_gt, settle_time_gt,
     stand_sequence, packet_stream_ref,
     claims: [cells, region], presence: [band, at],
     expected_changes: per-tick change sequence ref,
     depends_on: [step ids]
  } ],
  arming_steps: [step ids, last],
  bill_of_materials: totals and per step,
  unreachable: [],          # empty for a certified plan
  stuck: null
}
```

or a `PreBuildReport` (4.5) or a `StuckReport` (4.7).

### 3.3 Execution protocol

- K5 assigns step ids to builder squads; B6 replays the packet stream.
- Bots report `step_completed(id, readback)` or `step_halted(id, divergence)`.
- On a halt, K5 calls `repair(plan, from_state)` and the builder returns a repaired suffix or a stuck report.

## 4. Requirements

### 4.1 Plan request and plan contract

- A request either produces a certified plan, a pre-build report (if anything is unreachable or the schematic is defective), or a stuck report. Never a partial, uncertified plan sent to live.
- Plans are per Minecraft version and server profile; certificates never carry across versions (ARCHITECTURE §9.1).

### 4.2 Survival seams in the contract

The prototype builds in creative; the final system doesn't (ARCHITECTURE §3.9, builder-hard-cases §5.1). The contract carries survival needs from the start so the switch doesn't change it:

- **Items per step**, including temps, so logistics stages them and K7 can quote.
- **Break time per step** in game ticks, for temps and repairs.
- **Drops** from the bot's own breaks are intended changes the bot collects, listed per step.
- **Presence**: the site plus margin must stay loaded by a bot or a permitted chunk loader for the whole build, since there is no `/forceload`. Each step states the band it needs.
- **Container readback** is only possible by opening the container, which is itself an action the plan includes.
- **Start state** is what the client can scan; plans state whether they require an inert margin.

### 4.3 Multi-bot build modes

- **Certified mode:** redstone and update-sensitive regions; one builder bot in a region at a time unless the builder certifies more.
- **Decorative parallel mode:** inert regions (shell, walls, floors) partitioned for many bots at once. At hundreds of bots, the shell needs this (ARCHITECTURE §3.9).
- The plan marks each step's region and whether it is exclusive. How the builder partitions is internal.

### 4.4 Execution, progress and divergence

- Live replay halts on the first unexpected block change, position correction, or foreign entity in range (ONLINE_BUILDER §0 S3; B6 §4.7).
- Pauses at step boundaries are safe while the plan's assumptions hold; divergence halts are not covered by the guarantee and always go to repair.
- Progress reports to K5: steps done, items consumed, time, divergences.

### 4.5 Pre-build report and unreachable states

> **Interface input (runtime decision).** Who: person (an LLM can propose options, tested in the simulator).
> Why: some target states can't be produced by any ordering of survival placements (budded pistons, unpowered lit lamps, update-trick states, piston-locked boats); choosing a substitution, a different mechanism with the same function, or a macro is redstone design with no derivable answer.
> Enters the system as: an answer to a pre-build report ticket: approve a listed substitution, supply a macro (4.6), supply a revised schematic, or cancel.
> Null's side: the builder produces the report before anything is built, listing each unreachable cell, why, and candidate options; the build operation stays at `blocked(prebuild_report)` with its site claim held and materials staging allowed to continue.

### 4.6 Macro library

> **Interface input (authored).** Who: person (ONLINE_BUILDER §6: "a person writes each macro, usually from a stuck report they solved in a sandbox").
> Why: macros exist precisely for targets the builder's vocabulary can't express; each one is an invented mechanism (temporary pistons, update tricks) with its own cleanup.
> Enters the system as: a macro file (precondition pattern, action script with temporary mechanisms, postcondition), versioned per Minecraft version, added to the library and tried by the builder only on cells it classifies as inexpressible.
> Null's side: every macro is trialled and certified like one compound step; macros are never trusted untested.

Content packs may ship macros for their machines, for example a safe-insertion procedure for installing mobs (WORKED_EXAMPLE_OSC §1.1).

### 4.7 Stuck reports

> **Interface input (runtime decision).** Who: either (an LLM can triage common causes; a person for genuine redesign).
> Why: a stuck report means the bounded search found no clean plan (ONLINE_BUILDER P4: budgets guarantee stopping, not success); time-critical pairs and never-quiet builds also end here (ONLINE_BUILDER §9). Getting past it needs a new idea: a macro, a schematic change, an accepted risk, or a different site.
> Enters the system as: an answer to the stuck ticket, through the same options as 4.5, plus "retry with a wider budget" for the cases the triage judges budget-limited.
> Null's side: the builder attaches the state, the failing step, every plan tried with its first dirty diff, and the vetoes in force; the build operation keeps the already-certified, already-built prefix in place (it's inert at step boundaries) and holds.

### 4.8 Schematic defects and intent

> **Interface input (runtime decision).** Who: person (the schematic's author or the operator).
> Why: if the finished build isn't stable under a forced neighbour update (a BUD-extended piston, a rail in a shape neighbours would change), it's either an intended trick that needs a macro or a bug in the schematic; only the author's intent decides which (builder-hard-cases §3.3).
> Enters the system as: a decision on the report: "intended, use macro X", "bug, use corrected schematic", or "accept as-is".
> Null's side: the builder reports the cells and the change a forced update makes; it never "fixes" a schematic on its own.

### 4.9 Hazard model corrections

> **Interface input (authored).** Who: person.
> Why: when a trial is dirty in a way the hazard model didn't predict, turning recurring misses into model rules is deliberately human-reviewed in ONLINE_BUILDER §7; an automatic rule could encode a coincidence.
> Enters the system as: a reviewed change to the per-version hazard labels data (builder-hard-cases §5.3).
> Null's side: the builder logs every unpredicted dirty trial with its provenance; the simulator remains the judge regardless, so a missing rule costs speed, never safety.

### 4.10 Entity acceptance tolerances

> **Interface input (authored).** Who: person (the operator, or the content pack's author).
> Why: entities can't be placed exactly in survival (mob positions are floats, villager offers can't be reproduced, minecarts settle where they settle); what counts as "built correctly" for each entity is the user's intent (builder-hard-cases §4).
> Enters the system as: tolerance fields per entity class in the request or the content pack ("on this rail cell", "in this 1×1 cell", "offers: don't care").
> Null's side: defaults from builder-hard-cases §4 (accept by cell, offers don't-care), stated in the certificate; mob insertion results are reported per UUID with retries, since they can't be certified like blocks.

### 4.11 Arming steps and approval

> **Interface input (runtime decision).** Who: person (or a policy the commander set in advance).
> Why: arming makes a machine live (installing payload or actuator entities, placing TNT, closing the final break in a circuit); whether and when to do that is a decision like attacking a player (ARCHITECTURE §3.12, §4.2).
> Enters the system as: an `approve(arming, plan)` through K2, or a standing policy that pre-approves arming for named packs.
> Null's side: the builder leaves the arming set and the descriptor's arming steps until last, behind a physical circuit break; the build operation passes its handover gate (K7) and then waits at `blocked(needs_approval: arming)`.

### 4.12 Schematic intake (the builder never designs)

The builder takes schematics; it doesn't design them. Designing a machine, a door or a decorative structure to meet a need is upstream, an authored input to C11 (§4.5). The builder can report that a schematic is unbuildable, defective, or needs a macro; it never alters what was asked for.

## 5. Failure handling

| failure | response |
|---|---|
| Divergence halt during replay | Repair from the observed state, or stuck report |
| Materials missing at a step | `blocked(materials)`; K5 asks C3 |
| Site unloaded (presence lost) | Pause at the next step boundary; K5 restores presence |
| Foreign player or mob in range | Pause at the step boundary until clear |
| Simulator and live disagree (certificate assumption broken) | Halt; treat as a bug in the simulator setup, never retry around it |

## 6. Scale and performance

- Planning cost is the builder agent's concern; the interface allows planning to take wall-clock minutes to hours for big builds.
- At build scale most bots are suppliers; a few builder squads place blocks (ARCHITECTURE §6). Logistics throughput, not placement speed, sets the build rate.

## 7. Version-sensitive parts

Certificates, macros, hazard labels and placement recipes are all per version.

## 8. Tests

- The builder test corpus (builder-hard-cases §3.7) per version: observer chains, piston doors, flying machine, rail loop with a cart, sculk network, hopper clock and sorter, copper bulb T flip-flop, item-frame lock, villager hall, water and lava farm, decorative baseline.
- Contract tests: a plan with per-step items and break times feeds K7's quote and C3's staging unchanged.
- A schematic with one unreachable state produces a pre-build report before any live action.

## 9. Open questions (for Allen's builder agent)

- Does the prototype follow `ONLINE_BUILDER.md` v4.1, or a different design? This spec only assumes the interface.
- Can the simulator run with survival rules for the simulated bot (items, break time, drops) while the prototype is still creative?
- **Bot state during open-loop replay** (spec audit #6, 2026-10-13). In survival, the effect of a fixed packet stream depends on state the plan doesn't carry: which slot holds which item (held-item packets name slots), tool durability (a tool breaking mid-stream changes later break times), hunger (at 6 or less the bot can't sprint, so sprint moves end elsewhere) and status effects (Haste, Mining Fatigue). `upkeep.eat` and `upkeep.tool_swap` are off during replay, so nothing fixes these mid-step. The audit proposes a per-step precondition in the plan (slot layout, minimum durability per tool, hunger and saturation floor, no break-time effects) and a loadout action at each step boundary, which fits AD-C1.2's restock. Creative hides all of this, so it surfaces at the creative-to-survival switch.
