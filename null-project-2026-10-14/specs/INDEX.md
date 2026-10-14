# Null component specs: index

Spec set v1.2, 2026-10-14. One spec per component in `design/ARCHITECTURE.md` §10 (31 components), plus `CONVENTIONS.md` for the shared rules, the verdicts and the reflex switch contract. Design only, no code. ARCHITECTURE v0.8 (2026-10-14) adopted the specs' proposals and kept v0.7's section numbers, so every `ARCHITECTURE §n` reference here still holds.

**What's flagged (⚑).** Only architecture points that are still open: they can't be settled yet, they're open to other opinions, or someone else owns them, and getting them wrong is costly to undo (Allen, 2026-10-13). Everything else, 283 subsections, is implementable as written, and details like stage order or thresholds can change later without harm. Decisions a person or an LLM makes while the swarm runs are part of each component's interface, not flags. They're listed at the end.

## Summary

- **18 open architecture decisions** across 13 components (AD-K5.2, scenario knowledge as data, and AD-K5.3, the surprise contract, were proposed 2026-10-14; see `design/GENERALIZATION.md` and `design/SURPRISES.md`). Allen decided AD-F1.1, AD-F5.1, AD-K1.1 and AD-F5.2 (below). Each has options, a recommendation, why it needs you, what settles it, and what changes if it's decided late.
- **Both sides of the 14 open non-builder decisions are in [`DECISIONS.md`](DECISIONS.md)**, with what evidence would tip each one.
- **3 should be settled before any code is written:** AD-F2.1, AD-F2.2 and AD-C1.2.
- **4 belong with Allen's builder agent:** AD-C1.1, AD-C1.2, AD-B6.1 and AD-F3.1.
- **3 can be settled by a spike or bake-off** before Allen chooses: AD-F2.2, AD-B1.1 and AD-K2.1.
- **Decided (Allen, 2026-10-13 and 2026-10-14):** AD-F1.1, the server isn't an architecture question because adapting to a server is easy. AD-F5.1, Null runs on a specialized Azalea build with the latest features that speaks 1.21.11 natively, not through ViaVersion. AD-K1.1, one HQ per deployment, able to span several servers, with a server field in every location key. AD-F5.2 (2026-10-14), decision code (skills, reflexes, arbitration, squad tactics) has no Bevy or Azalea dependency and named mechanism crates import them directly, enforced by a cargo-deny allowlist (F5 §4.1); the live join check passed.
- **Capacity (Allen, 2026-10-13):** one laptop may carry about 1,000 bots at roughly 1 MB of extra memory each, so CPU and above all bandwidth are the limits (F1 §4.1, §4.11). Azalea's own guidance is a few hundred per swarm, and a swarm is one ECS that must finish each 50 ms tick, so 1,000 is a target to benchmark. Local testing is per component, not a full 2,000-bot run.
- **Spec audit** (`review/SPEC_AUDIT.md`, 2026-10-13): its two open questions became AD-F5.2 and AD-K1.1, both now decided. Its other findings are fixed in the specs, each marked "(spec audit #n)", except #6 (builder bot state during replay), which is noted in C1 §9 for the builder agent.
- **Azalea reality check** (`research/AZALEA_REALITY_CHECK.md`, 2026-10-13): its §7 factual corrections are applied in F1, F2, F5, B1, B2, B3, B6 and B7, each marked "(Azalea reality check, 2026-10-13)".
- **AD-F5.2 spike** (`research/SPIKE_F5_BOUNDARY.md`, 2026-10-14): the decision-code split held across three Azalea builds, reflexes act in the same tick, and the host costs no measurable CPU. Applied in F1 §4.1, F5 and CONVENTIONS §3.5. A live vanilla-server run (spike §6) then passed too; it found that Azalea sends no `UseItem` for a block-targeted click.
- **18 components have none open:** F1, F4, F5, K1, B2, B3, B4, B7, C2, C4, C5, C6, C10, C11, K3, K4, K6 and K7. K3, K4 and K6 depend on decisions owned by other specs, noted in each.

**Reflexes can be turned off** (Allen, 2026-10-13). The contract is in `CONVENTIONS.md` §4, and it's settled, not flagged. Every reflex can be switched off at swarm, operation, squad, job or bot scope. "Off" still reports the trigger to the skill and squad. Jobs declare which reflexes they need off or assume on. Switch tables: B1 §4.9, B2 §4.9, B5 §4.9, B7 §4.10. Runtime side: F1 §4.5. Commands: K2 §4.5. Squads: K6 §4.7 and C7 §4.7. UI: K4 §4.7.

---

## Architecture decisions

| decision | recommendation | settled by | needed before |
|---|---|---|---|
| [AD-F2.1](F2-world-model.md) Shard belief by operation, or by fixed map tiles? Includes where belief physically lives (spec audit #3) | Fixed tiles, with operations as the current writers; operation service inside the node | Allen's sign-off | any code: the shard key is hard to migrate |
| [AD-F2.2](F2-world-model.md) Extend Azalea's world storage, or keep a separate store? | Separate store, plus a chunk-storage plug-in; confirmed by the Azalea reality check | mostly settled; a spike measures observer cost | any code |
| [AD-C1.2](C1-builder.md) How do items reach a builder bot during a certified plan? | Modules sized to fit an inventory, restocked at the entry area between modules | Allen together with the builder agent | the survival builder; the creative prototype hides it |
| [AD-C1.1](C1-builder.md) How does building with many bots keep the guarantee? | One certified bot per redstone region; many bots on independent inert regions | Allen together with the builder agent | builder integration (§7 step 6) |
| [AD-B6.1](B6-placement-breaking.md) Precompute placement recipes, or probe on demand? | Table for common states, probing for the rest | Allen's builder agent | builder integration |
| [AD-F3.1](F3-test-harness.md) One server pool for the harness and the builder's simulator? | One pool, two job types | Allen together with the builder agent | the harness build-out |
| [AD-K2.1](K2-commander-api.md) Imperative commands, or a desired-state manifest? | The manifest hybrid, after a bake-off against a fake HQ | the bake-off, then Allen's call | the LLM commander (§7 step 5) |
| [AD-K2.2](K2-commander-api.md) Share the commander seat between Allen and the LLM, and on what terms? | Shared, with each goal owned by a person or the LLM, plus approval classes | Allen's call | the LLM commander |
| [AD-K5.1](K5-operation-service.md) LLM officers at operation level in v1? | No; keep the interface ready for them (reopened by AD-K5.2) | Allen's call | the LLM commander |
| [AD-K5.2](K5-operation-service.md) Scenario knowledge as data (profile rows, closed verb set) or as method and mob-module code? | Data; the paper test in `design/GENERALIZATION.md` §7 passed (2026-10-14) | the paper test, then Allen's call | K5 methods and B5 modules (before they are written) |
| [AD-K5.3](K5-operation-service.md) Surprises: contain and price, or classify and resolve? | Contain and price; escalate on cost; fix in cost-ranked batches with token budgets (`design/SURPRISES.md`) | F3 runs with injected surprises, then Allen's call | K5 recovery rules and F4 triage (before they are written) |
| [AD-B1.1](B1-movement.md) Reuse Azalea's pathfinder, or write our own? | Reuse Azalea's, confirmed by the Azalea reality check; widen its compute pool, add a shared cache later | a CPU benchmark, then Allen's call | survival core (§7 step 2) |
| [AD-F6.1](F6-bus-deployment.md) Where does the bus's third quorum member live? | Defer; add a third small VPS once several always-on hosts run | Allen's call | distribution across hosts (§7 step 4) |
| [AD-C3.1](C3-logistics.md) Central depots, or storage per operation? | Per operation for small swarms, depots once several operations share a base | Allen's call | the first base with several operations |
| [AD-C9.1](C9-entity-logistics.md) Can one convoy's escort span two nodes? | No; split long trains into several convoys | Allen's call | the first convoy |
| [AD-C7.1](C7-squads-tactics.md) What coordinates fights bigger than one squad? | Bigger squads first, capped by density; battle groups only if measured fights need them | Allen's call | faction play (§7 step 8) |
| [AD-B5.1](B5-combat.md) Hand-written combat doctrine, or learned policies? | Hand-written, with a learned-micro seam built in from day one | Allen's call | faction play |
| [AD-C8.1](C8-faction.md) Should bots talk to people in v1? | Silent; keep the policy gate for later | Allen's call | faction play |

---

## Interface inputs (not flags)

These are the places where a component takes input from a person, the commander or an LLM, either while running (runtime decision) or as something written ahead of time (authored input). They're settled architecture: each spec names the channel the input arrives through and what Null does meanwhile.

| spec | subsection | kind · who |
|---|---|---|
| [F1](F1-swarm-runtime.md) | 4.9 Server deployment profile | runtime decision · person |
| [F3](F3-test-harness.md) | 4.4 Adversary models | authored input · person |
| [F3](F3-test-harness.md) | 4.7 Commander evaluation rubric | authored input · person |
| [F3](F3-test-harness.md) | 4.9 Live gate | runtime decision · person |
| [F4](F4-ops-telemetry.md) | 4.8 Novel failure triage | runtime decision · either |
| [F5](F5-version-layer.md) | 4.8 Content compatibility across versions | runtime decision · person |
| [B4](B4-gathering.md) | 4.8 Ownership check before breaking | authored input · person |
| [B5](B5-combat.md) | 4.7 Rules of engagement and attribution | runtime decision · either |
| [B5](B5-combat.md) | 4.8 PvP against real players | authored input · person |
| [B7](B7-mob-handling.md) | 4.8 Containment patterns | authored input · person |
| [C1](C1-builder.md) | 4.5 Pre-build report and unreachable states | runtime decision · person |
| [C1](C1-builder.md) | 4.6 Macro library | authored input · person |
| [C1](C1-builder.md) | 4.7 Stuck reports | runtime decision · either |
| [C1](C1-builder.md) | 4.8 Schematic defects and intent | runtime decision · person |
| [C1](C1-builder.md) | 4.9 Hazard model corrections | authored input · person |
| [C1](C1-builder.md) | 4.10 Entity acceptance tolerances | authored input · person |
| [C1](C1-builder.md) | 4.11 Arming steps and approval | runtime decision · person |
| [C2](C2-site-prep.md) | 4.8 Demolishing unknown structures | runtime decision · either |
| [C2](C2-site-prep.md) | 4.9 Edge and terrain blending | authored input · either |
| [C4](C4-production.md) | 4.7 Diagnosing an underperforming facility | runtime decision · either |
| [C7](C7-squads-tactics.md) | 4.5 Plays against players | authored input · person |
| [C7](C7-squads-tactics.md) | 4.8 After-action review | runtime decision · either |
| [C8](C8-faction.md) | 4.3 Threat assessment | runtime decision · LLM |
| [C8](C8-faction.md) | 4.5 Defence layout for a base | authored input · either |
| [C8](C8-faction.md) | 4.6 Relations, war and peace | runtime decision · person |
| [C8](C8-faction.md) | 4.7 Diplomacy and chat with humans | runtime decision · LLM with person oversight |
| [C8](C8-faction.md) | 4.9 Server rules and conduct | runtime decision · person |
| [C10](C10-site-selection.md) | 4.4 Concealment model | authored input · person |
| [C10](C10-site-selection.md) | 4.5 Trade-offs with no dominant site | runtime decision · either |
| [C11](C11-content-packs.md) | 4.5 Designing new machines and builds | authored input · person or LLM |
| [C11](C11-content-packs.md) | 4.6 Describing an existing community build | authored input · person |
| [C11](C11-content-packs.md) | 4.7 Pack acceptance tests | authored input · person |
| [C11](C11-content-packs.md) | 4.9 Trusting third-party packs | runtime decision · person |
| [K1](K1-hq.md) | 4.10 Escalations to the commander | runtime decision · commander |
| [K2](K2-commander-api.md) | 4.6 Policy content | runtime decision · commander |
| [K2](K2-commander-api.md) | 4.10 Shared seat between a person and the LLM | authored input · person |
| [K3](K3-llm-commander.md) | 4.7 Strategy (the commander's own thinking) | runtime decision · LLM |
| [K3](K3-llm-commander.md) | 4.8 Commander prompt and standing guidance | authored input · person |
| [K5](K5-operation-service.md) | 4.10 Operation officers for fuzzy work | runtime decision · LLM (optional) |
| [K7](K7-program-service.md) | 4.8 Approving a quote | runtime decision · commander |

## Not covered here

- **The builder's internals** belong to Allen's builder agent. C1 specifies only its interface, survival seams and inputs.
- **ARCHITECTURE.md v0.8** (2026-10-14) adopted the specs' proposals beyond v0.7: permits and the narrowed break rule (B4 §4.8, K2 §4.4), self-fencing, the bandwidth and density limits, F3's live gate, and the decision-code host from AD-F5.2 (its §11 lists every change). The reflex contract was already in v0.7 §5. AD-F2.1 is recorded there as open, not adopted.
