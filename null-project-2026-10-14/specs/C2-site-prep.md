# C2 Site prep and demolition

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.10 (sites), §10; WORKED_EXAMPLE_OSC.md §3 (Site dig); research/minecraft-tech-inventory.md C.5. Conventions: `CONVENTIONS.md`.
**Depends on:** B1, B4, B6, C3, F2, K5. **Used by:** build operations (before C1), C10 handover, K7 (spoil and tool wear in quotes).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Excavation planning | Deterministic | Dig volumes in lanes with safe ordering and claims |
| 4.2 | Fluids | Deterministic | Water and lava removal methods are known and testable |
| 4.3 | Gravity blocks and cave-ins | Deterministic | Sand and gravel behaviour is game data |
| 4.4 | Levelling and fill | Deterministic | Fill to a target surface from spoil |
| 4.5 | Spoil routing | Deterministic | Spoil goes into the ledger and to its destination by policy |
| 4.6 | Spawn-proofing during the dig | Deterministic | Light and block rules for spawning are game data |
| 4.7 | Demolishing our own builds | Deterministic | We have the plan; disarm then reverse |
| 4.8 | Demolishing unknown structures | Interface · runtime decision · either | Taking apart someone else's redstone, possibly trapped, safely has no general method |
| 4.9 | Edge and terrain blending | Interface · authored input · either | How a build meets the surrounding land is an aesthetic choice |

## Architecture decisions

None open.


---

## 1. Purpose

On survival, preparing a site is often as large as the build itself (ARCHITECTURE §10): clearing and levelling, digging out volumes (92k blocks for the Skybender), removing water and lava, and demolishing what's in the way. C2 runs that as site-prep jobs inside the site's operation.

## 2. Scope

**Owns:** excavation plans; fluid removal; gravity and cave-in handling; levelling and fill; spoil routing; spawn-proofing as the dig goes; demolition of our builds and of unknown structures; edge treatment.

**Does not own:** choosing the site (C10); placing the final build (C1); mining for resources (B4, though C2 uses its skills).

## 3. Interfaces

- `PrepRequest{site, target_volume (air), target_surface?, keep: cells, spoil_policy, spawn_proofing: bool, concealment: budget?}` from K5.
- Output: jobs for K5's scheduler (dig lanes, fluid jobs, haul jobs), then `site_ready{verified_volume, changes_outside: none}`.
- Spoil and drops to C3's ledger; tool wear to K7's rate store.

## 4. Requirements

### 4.1 Excavation planning

- Split the volume into lanes and layers sized for squad parallelism, each a block-set claim.
- Dig top-down where possible, so bots stand on unmined ground; leave stairs or ladder shafts for access and exit.
- Estimate time and tool wear per lane from break times (F5) and measured rates.

### 4.2 Fluids

- Water: fill sources with blocks then remove, or sponges where available, working from the edges inwards.
- Lava: cover or convert sources (water to obsidian or cobble) before digging into them; bots keep fire resistance where available.
- Aquifers found during the dig become fluid jobs before digging continues.

### 4.3 Gravity blocks and cave-ins

Sand, gravel and concrete powder ceilings are dropped deliberately or supported before digging under them. Pointed dripstone is cleared from ceilings.

### 4.4 Levelling and fill

Fill low cells to the target surface from spoil, compacting solid; never fill with gravity blocks unless supported.

### 4.5 Spoil routing

- Spoil enters the item ledger as it's mined. Policy decides: keep for casing, store, or haul away.
- For concealed sites, spoil is hauled at least the pack's minimum distance so it doesn't mark the site (WORKED_EXAMPLE_OSC §3).
- Haulage is quoted in stacks and bot-hours (K7).

### 4.6 Spawn-proofing during the dig

The bots digging are players, so mobs spawn near them in the dark (ARCHITECTURE §3.10). Light or slab the floor of dug areas as they're opened, to the version's spawning rules, when the request asks for it.

### 4.7 Demolishing our own builds

We have the plan: disarm first (open the arming set's circuit break, remove TNT and installed entities by the pack's procedure), then reverse the plan, with each removal trialled by the builder for redstone regions. Items return to the ledger.

### 4.8 Demolishing unknown structures

> **Interface input (runtime decision).** Who: either (an LLM can classify the structure from scans; a person approves anything involving another player's build or explosives).
> Why: a structure we didn't build may contain live redstone, TNT, sculk alarms or deliberate traps (common on faction servers); taking it apart without setting it off means understanding an unknown machine, and whether to touch another player's build at all is a diplomatic question.
> Enters the system as: a demolition decision on a ticket: leave it, demolish with a plan the thinker approves (order and disarm points), or route around it; plus the ownership answer from C8 and B4's ownership rules.
> Null's side: scans the structure, classifies blocks with the builder's hazard labels, refuses to break anything in the hazard neighbourhood of an actuator or sensor without an approved plan, and treats any unexpected change during demolition as a halt.

### 4.9 Edge and terrain blending

> **Interface input (authored).** Who: either (an LLM can propose options; the operator picks for builds that matter to them).
> Why: a schematic has a hard boundary; where it meets uneven terrain, someone has to choose foundations, retaining walls, slopes or terraces, and there's no best answer, only taste.
> Enters the system as: an edge treatment chosen in the build request (a named style, or an extra schematic for the surround), defaulting to "level to the schematic's base and slope 1:1 to existing ground".
> Null's side: applies the default unless the request names another; for concealed sites, the edge treatment must also meet the concealment budget.

## 5. Failure handling

| failure | response |
|---|---|
| Unexpected cave, lava or aquifer | Pause the lane; fluid job; re-plan |
| Cave-in onto bots | B2 escape; mark the lane dangerous; re-plan support |
| Something outside the claim changed | Halt and report; it may be our dig or someone else |
| Tool supply runs out | `blocked(tools)`; C3 restocks |

## 6. Scale and performance

Excavation parallelises well: hundreds of bots on lanes. Haulage is the bottleneck for big digs; spoil routing is planned with C3 carriers.

## 7. Version-sensitive parts

Fluid behaviour, gravity blocks, spawning rules, break times, via F5.

## 8. Tests

- Dig a 30×30×20 volume through an aquifer in the harness: final volume matches, no changes outside the claim, spawn-proofed.
- Unknown structure with a tripwire-TNT trap in the way: no break inside the trap's hazard neighbourhood without an approved plan.

## 9. Open questions

- Use spoil as casing automatically when the schematic's block palette allows it, or only when the request says so? Default to the request.
