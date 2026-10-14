# C11 Content packs

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.10, §3.12, §3.13, §10; WORKED_EXAMPLE_OSC.md §1.1; REVIEW_RESPONSE.md (DSL creep, arming); research/builder-hard-cases.md §3.2. Conventions: `CONVENTIONS.md`.
**Depends on:** F5 (data, versions), F3 (acceptance runs). **Used by:** K7 (production graphs), C1 (schematics, macros, arming), C4 (facilities), C9 (containment), C10 (site constraints).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Descriptor schema | Deterministic | A small typed schema with no scripting, fixed by the framework |
| 4.2 | Loading and validation | Deterministic | Schema checks plus consistency with the schematic |
| 4.3 | Rates: claimed and measured | Deterministic | Every rate is `[c]` until a run writes `[m]` with conditions |
| 4.4 | Pack contents beyond descriptors | Deterministic | Schematics, macros, containments and acceptance tests, each a known file type |
| 4.5 | Designing new machines and builds | Interface · authored input · person or LLM | Inventing a redstone machine or a decorative structure for a need is the canonical open-ended problem |
| 4.6 | Describing an existing community build | Interface · authored input · person | Writing a descriptor needs someone who understands the machine: its arming steps, presence, inputs and safe insertion |
| 4.7 | Pack acceptance tests | Interface · authored input · person | What "this machine works" means has to be stated by someone who knows what it's for |
| 4.8 | Version compatibility | Deterministic | Acceptance tests run per version; results recorded (fixing failures is F5 §4.8) |
| 4.9 | Trusting third-party packs | Interface · runtime decision · person | Whether to run someone else's pack on a live server is a trust decision |

## Architecture decisions

None open. v0.7 already settled the main question (a typed schema, no scripting).


---

## 1. Purpose

Null is a framework; machines are content (ARCHITECTURE §3.12). A content pack is schematics plus a data descriptor per facility in a small typed schema: outputs and rates, inputs and how each is used, per-container contents, presence, site and concealment constraints, arming steps, precursors. C11 defines that schema, loads and validates packs, and records which numbers are measured. It's also where most of the project's open-ended design work enters: every new machine, build or containment arrives as a pack.

## 2. Scope

**Owns:** the descriptor schema; loading and validation; claimed and measured rates; the pack format for schematics, macros, containments and acceptance tests; version compatibility records; the pack trust list.

**Does not own:** designing machines (people and LLMs; their output arrives through 4.5); operating machines (the operator, ARCHITECTURE §3.12); building (C1).

## 3. Interfaces

```
pack/
  pack.yaml            name, author, version, minecraft versions claimed, provenance
  facilities/*.yaml    descriptors (4.1)
  schematics/*.litematic
  macros/*             builder macros (C1 §4.6), e.g. safe-insertion procedures
  containments/*       mob containment schematics tagged with mob types (B7 §4.8)
  tests/*              acceptance scenarios (4.7)
```

- `load(pack)` → validated pack or errors.
- `descriptor(facility)` → typed descriptor for K7, C4, C10, C1.
- `record_measurement(pack, key, value, conditions, run_id)`.

## 4. Requirements

### 4.1 Descriptor schema

Typed fields only, no scripting (REVIEW_RESPONSE "DSL creep"):

| field | content |
|---|---|
| `facility`, `schematic`, `footprint` | identity and geometry |
| `site` | spawn conditions, depth, range constraints, concealment budget (traffic and dwell time included), spawn-proofing |
| `inputs` | items, exact per-container contents, entities; each with `use: consumed | installed | feeds_facility`, and counts |
| `outputs` | items with rates, `[c]` or `[m]` |
| `facilities_inside` | e.g. dupers, chunk loaders |
| `presence` | the fixed form: `at`, `band`, `count`, `phase`, `outside` (ARCHITECTURE §3.10) |
| `arming` | circuit break location and arming steps, in order (policy class) |
| `precursors` | facilities needed first |
| `operation` | data for the operator's tooling only (markers, charge times); Null doesn't interpret it |

The annotation effect set (`keep_out`, `defend`, `rally`, `presence_target`) is closed and grows only by changing the framework (ARCHITECTURE §3.12); packs can't add effects.

### 4.2 Loading and validation

- Schema validation; every referenced schematic, macro and containment present.
- Consistency: input item counts match the schematic's bill of materials where `count: from schematic`; per-container contents match containers in the schematic; arming circuit break is a real cell in the schematic.
- Presence bands are valid band names; phases reference real phases.
- A pack that fails validation isn't loadable; errors name the field.

### 4.3 Rates: claimed and measured

Every rate and throughput number is `[c]` claimed until a harness or live run measures it and writes `[m]` with its conditions (version, server profile, bot count) (ARCHITECTURE §3.11). K7's quotes show which is which.

### 4.4 Pack contents beyond descriptors

Schematics, macros (C1), containments (B7, C9) and acceptance tests (4.7) are files in the pack, each validated by its own consumer.

### 4.5 Designing new machines and builds

> **Interface input (authored).** Who: person or LLM (an LLM with the builder's simulator can design and iterate; a person judges whether the result is what they wanted).
> Why: this is the canonical case. Designing a redstone machine, a farm, a door, a containment or a decorative structure to meet a need has no optimal answer available up front; it's invention, trial and taste.
> Enters the system as: a new pack (schematic plus descriptor plus acceptance test), validated by 4.2 and tested in F3 before any quote uses it.
> Null's side: never designs anything. When a goal needs a facility no loaded pack provides, K7's quote says so (`needs_content(output, constraints)`) and the commander either supplies a pack, picks another approach, or drops the goal.

Typical design work this covers: new farms for a server's conditions, defensive structures (C8 §4.5), decorative builds, containment for a new mob type, a variant of a machine for a different version.

### 4.6 Describing an existing community build

> **Interface input (authored).** Who: person (an LLM can draft from a schematic and a video transcript; the person checks it).
> Why: turning a proven community machine (like the Orbital Skybender) into a descriptor needs someone who understands it: which steps arm it, what presence it needs, how each input is used, how to insert mobs safely, what its real rates are. The schematic alone doesn't say.
> Enters the system as: the descriptor, macros and acceptance test in a pack, with claimed rates marked `[c]`.
> Null's side: validates consistency with the schematic (4.2), and the builder can suggest a candidate arming set (builder-hard-cases §3.2) for the author to confirm; the author's arming steps are authoritative.

Per Allen's guidance, Null doesn't second-guess a published, proven build's mechanics; the author describes them and the harness measures rates.

### 4.7 Pack acceptance tests

> **Interface input (authored).** Who: person (the pack's author or the operator).
> Why: the pack's own acceptance test (WORKED_EXAMPLE_OSC §5.4 P-checks: impact location, damage, breezes surviving, reusability, server load) states what the machine is for, which only someone who wants the machine can define.
> Enters the system as: test scenarios in the pack, run by F3.
> Null's side: runs them per version, records results, and refuses to quote a facility whose acceptance test hasn't passed on the running version (F5 §4.8).

### 4.8 Version compatibility

Acceptance tests run on every supported version; results are recorded per pack and version as `passes | fails | untested`. Fixing a failure is a redesign (F5 §4.8).

### 4.9 Trusting third-party packs

> **Interface input (runtime decision).** Who: person.
> Why: a pack from someone else may have wrong rates, missing arming steps, or a design that causes problems on a live server (lag, griefing); deciding to run it is a trust decision.
> Enters the system as: a per-server allow-list of packs, set by the operator.
> Null's side: packs not on the allow-list can be loaded and tested in the harness but not quoted for a live server.

## 5. Failure handling

A pack that validates but fails acceptance is loadable for testing only. A pack whose measured rates are far below its claims is reported (C4 §4.7).

## 6. Scale and performance

Tens of packs; validation is cheap.

## 7. Version-sensitive parts

Schematics, macros and acceptance results are per version.

## 8. Tests

- The Skybender pack from WORKED_EXAMPLE_OSC §1.1 validates, including per-container contents and the circuit break cell.
- A pack with a mismatched input count fails validation naming the field.
- A goal needing an output no pack provides yields `needs_content` in the quote.

## 9. Open questions

- Should packs be able to declare optional variants (different farm designs for the same output), so K7 picks one? Probably yes, as separate facilities with the same output.
