# Null: index

Null is a swarm of Minecraft bots under one commander, a person or an LLM. It plays on pure vanilla survival servers through legit play, built on a custom Azalea build that speaks 1.21.11. **This folder is the single source of truth for every Null document.** This index says what each document is and whether it's current. Last updated 2026-10-14.

**Start here:** [ARCHITECTURE.md](ARCHITECTURE.md) (the design), then the decision summary below, then the component spec you need.

## How this folder works

| where | what it holds | status |
|---|---|---|
| top level | the architecture, the conventions, the decisions, the 31 component specs, the worked example, the builder design | **Current.** This is the truth, kept up to date. |
| [`proposals/`](proposals/) | designs written but not adopted, each with an open decision | **Proposal.** Not truth until its decision is made and the change is folded into a top-level doc. |
| [`reference/`](reference/) | research, spikes, surveys, and notes on how the builder prototype was built | **Reference.** Accurate where it doesn't conflict with a current doc, but not maintained. |
| [`history/`](history/) | reviews, audits, debates and earlier builder designs that current docs have superseded | **Historical.** Kept as a record. |
| [`logs/`](logs/) | progress logs, escalation logs, stuck reports, run logs and milestone reports | **Log.** A record of what happened. Never a spec, and never edited after the fact. |

Rules:
- **Prose paths are relative to this folder.** For example, "`proposals/GENERALIZATION.md` §7" means `specs/proposals/GENERALIZATION.md`. Markdown links are relative to the file they're in. `ARCHITECTURE §n` and `C1 §n` style citations name a document; this index says where it is.
- **Builder archive copies are read-only.** Every file under `reference/builder/`, `history/builder/` and `logs/builder/` is a byte-identical copy from the builder archive, `null-builder-archive-2026-10-14` at commit `0f5ce4a`, which is kept unchanged. Code paths inside those files (`lib/loop.js`, `planner/ONLINE_BUILDER.md`) are relative to that repo. The tables below give each file's source. The archive keeps the code, test fixtures, plans, certificates and planner data. Only documents and logs were copied.
- **A change to the truth goes into a top-level doc.** A proposal that gets adopted is folded into the docs it changes, and its file moves to `history/`. If it's a contract many specs share, it moves to the top level instead and the specs point to it, as INTERRUPTS did.

## Documents

### Current (top level)

| doc | status | what it is |
|---|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Current (v0.8) | The swarm architecture: 31 components in four groups (F, B, C, K), the control model, HQ, world model, version layer, risks. §11 lists what changed from v0.7 and the proposals made since. |
| [CONVENTIONS.md](CONVENTIONS.md) | Current | Shared rules for all specs: verdicts, claims vs permits vs territory (§3.3a), the decision/mechanism crate rule (§3.5), the reflex switch contract (§4). |
| [DECISIONS.md](DECISIONS.md) | Current | Both sides of each open non-builder decision, and what evidence would tip it. |
| F1–F6, B1–B7, C1–C11, K1–K7 | Current (spec set v1.2) | One spec per component, named by its ARCHITECTURE §10 id. Each starts with a judgment map and its open decisions (⚑). Listed under "Architecture decisions" below. |
| [INTERRUPTS.md](INTERRUPTS.md) | Current (adopted 2026-10-14, AD-K5.3) | The interrupt contract: anything outside a job's expected envelope is contained without being diagnosed (back off, quarantine, requeue elsewhere), priced as a re-plan, escalated only on cost, and fixed from a cost-ranked batch review with token budgets. Proposed as "the surprise contract". |
| [CONTROL_HIERARCHY.html](CONTROL_HIERARCHY.html) | Current (overview, 2026-10-14) | One-page diagram of the control hierarchy, from the person and the commander down to Azalea and the server: what moves down and up at each boundary, how an interrupt moves up, and what skips levels. Drawn from ARCHITECTURE and INTERRUPTS; if they disagree, they win. |
| [WORKED_EXAMPLE_OSC.md](WORKED_EXAMPLE_OSC.md) | Current | The Orbital Skybender walked through the architecture end to end, as a test of the control model. |
| [ONLINE_BUILDER.md](ONLINE_BUILDER.md) | Record (v4.1), replaced by `proposals/WORLD_TRANSFORMATION.md` (Allen, 2026-10-14) | The builder's internal design: simulator-certified plans, trials, witnesses, repair search. Its status block lists the decisions made while prototyping that aren't folded in yet. The builder decisions (AD-C1.1, AD-C1.2, AD-B6.1, AD-F3.1) were decided 2026-10-15 in the world-transformation frame (C1, B6, F3); its §9 open questions are superseded. |

### Proposals

| doc | status | what it is |
|---|---|---|
| [proposals/GENERALIZATION.md](proposals/GENERALIZATION.md) | Decided in the shape of its §0 (AD-K5.2, Allen, 2026-10-15); §1–§8 are the earlier proposal kept as detail | Scenarios can't be listed: facts as data from the jar, observation and the LLM prior (no handwritten per-row fields); skill code only for cost-ranked important parts; goals to state and check tasks; the LLM plans steps with no model in validated, quoted stretches, with the existing searches as subroutines; plans that work saved as routines. |
| [proposals/GENERALIZATION_PAPER_TEST.md](proposals/GENERALIZATION_PAPER_TEST.md) | Evidence for AD-K5.2 | The paper test on five unseen tasks (2026-10-14): passed on verbs and methods; the cost moved into 9 fields and 5 planner features. |
| [proposals/GENERALIZATION_PAPER_TEST_2.md](proposals/GENERALIZATION_PAPER_TEST_2.md) | Evidence for AD-K5.2 | The blind second paper test on five new tasks (2026-10-14): 0 verbs and 0 methods; fields fell from 9 to 4, but total framework items rose from 18 to 24, and each new domain still cost 1–2 fields. |
| [proposals/GENERALIZATION_PAPER_TEST_3.md](proposals/GENERALIZATION_PAPER_TEST_3.md) | Evidence for AD-K5.2 | The ten tasks written as goals (2026-10-15): existing searches solve 54 of 74 steps (42 pessimistic); the LLM or a coded skill carries the rest, mostly steps with no model to search; no hard step can be confirmed before it runs. Basis for the decided shape. |
| [proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md) | Proposal, in design with Allen (2026-10-14) | Replaces the role of ONLINE_BUILDER v4.1: turning a region into a target over blocks and entities, with no simulator on the build path, a descriptor in Minecraft's data model, a condensed ordering rule set as data, and an isolation shell. Its §8 records tests not yet started. |

### Reference

| doc | status | what it is |
|---|---|---|
| [reference/AZALEA_REALITY_CHECK.md](reference/AZALEA_REALITY_CHECK.md) | Current reference | What Azalea really provides, and the recipe for the 1.21.11 fork (branch tip plus 38 cherry-picks). Its §7 corrections are applied in the specs. |
| [reference/BENCH_AZALEA_2026-10-15.md](reference/BENCH_AZALEA_2026-10-15.md) | Current reference (measured 2026-10-14 and 2026-10-15) | The Azalea benchmark on one laptop, up to 100 valid bots: the vanilla server's tick thread is the limit past that; completed searches level off whatever the pool size; 99% of section building is repeated work; the packet observer costs almost nothing; byte comparison is not change detection. Basis for AD-B1.1 and AD-F2.2. |
| [reference/SPIKE_F5_BOUNDARY.md](reference/SPIKE_F5_BOUNDARY.md) | Current reference | The AD-F5.2 spike (2026-10-14): the decision/mechanism split tested on three Azalea builds in the simulator, then on a live vanilla 1.21.11 server (§6, passed except mining and many-bot load). |
| [reference/COMMANDER_INTERFACE.md](reference/COMMANDER_INTERFACE.md) | Reference, partly adopted | Options for showing the swarm to an LLM commander, written against v0.4. What was adopted is in ARCHITECTURE §4 and K2/K3; the rest is AD-K2.1. K3 still cites its §7. |
| [reference/BREEZE_PIPELINE_FAILURES.md](reference/BREEZE_PIPELINE_FAILURES.md) | Reference | How the breeze capture and transport pipeline fails. Its fixes are in ARCHITECTURE §3.14 and C9. |
| [reference/builder-hard-cases.md](reference/builder-hard-cases.md) | Reference | Hard cases for the universal schematic builder: placing without redstone firing, and getting entities into place. |
| [reference/minecraft-tech-inventory.md](reference/minecraft-tech-inventory.md) | Reference | Survey of vanilla 1.21.11 technical builds (common, digital redstone, SMP-specific), with what each means for a bot builder. |
| [reference/BUILDER_ALTERNATIVES.md](reference/BUILDER_ALTERNATIVES.md) | Reference (research proposal, written for the builder agent; now input to `proposals/WORLD_TRANSFORMATION.md`) | Faster, less LLM-dependent ways to build schematics while keeping v4.1's certificate. Built on six scout reports in [reference/builder-alternatives-scouts/](reference/builder-alternatives-scouts/) (games, learning, live systems, mechanics, NP-hardness, real-world construction). |

### Reference: the builder prototype as built

Copied from the builder archive. They describe the archived JavaScript prototype of ONLINE_BUILDER v4.1.

| doc | source in the archive | what it is |
|---|---|---|
| [reference/builder/IMPLEMENTATION.md](reference/builder/IMPLEMENTATION.md) | `online/IMPLEMENTATION.md` | The implementation plan for v4.1: tasks A1–J1, done-when tests, appendices. Task status is in the progress log. |
| [reference/builder/LOOP.md](reference/builder/LOOP.md) | `online/LOOP.md` | The step loop as built, including the decisions K1, M1, support temps and the J1 interim rule. |
| [reference/builder/TRIAL.md](reference/builder/TRIAL.md) | `online/TRIAL.md` | The trial runner as built. |
| [reference/builder/REPAIR.md](reference/builder/REPAIR.md) | `online/REPAIR.md` | The repair search as built (G1). |
| [reference/builder/MODULES.md](reference/builder/MODULES.md) | `online/MODULES.md` | The other modules as built: classifier, replay core, trace. |

### History

| doc | status | what it is |
|---|---|---|
| [history/REVIEW_OSC_CONTROL_METHOD.md](history/REVIEW_OSC_CONTROL_METHOD.md) | Historical | Independent review of the worked example's control method (v0.6). |
| [history/REVIEW_RESPONSE.md](history/REVIEW_RESPONSE.md) | Historical | The response to that review; its changes landed in v0.7. |
| [history/SPEC_AUDIT.md](history/SPEC_AUDIT.md) | Historical (closed) | Error audit of the spec set; every finding is resolved or handed off. |
| [history/AD-F5.2-bevy-boundary.md](history/AD-F5.2-bevy-boundary.md) | Historical (experiment run) | The three-advocate debate on whether skills should see Bevy/Azalea. Its experiment is the spike in `reference/`. |

### History: earlier builder designs

Copied from the builder archive. ONLINE_BUILDER v4.1 is a clean-slate design that supersedes all of these.

| doc | source in the archive | what it is |
|---|---|---|
| [history/builder/BUILD_PLANNER_SPEC.md](history/builder/BUILD_PLANNER_SPEC.md) | `BUILD_PLANNER_SPEC.md` | The first build planner spec: a survival litematic paster (2026-10-10). |
| [history/builder/PLAN.md](history/builder/PLAN.md) | `PLAN.md` | Its prototype implementation plan, on the cobble schematic in the simulator. |
| [history/builder/PROTOCOL.md](history/builder/PROTOCOL.md) | `planner/PROTOCOL.md` | The contracts that prototype's workstreams coded against. |
| [history/builder/2026-10-10-mineflayer-sequence-builder-design.md](history/builder/2026-10-10-mineflayer-sequence-builder-design.md) | `docs/superpowers/specs/2026-10-10-mineflayer-sequence-builder-design.md` | The Mineflayer sequence-builder bridge design. |
| [history/builder/2026-10-10-mineflayer-sequence-builder-plan.md](history/builder/2026-10-10-mineflayer-sequence-builder-plan.md) | `docs/superpowers/plans/2026-10-10-mineflayer-sequence-builder.md` (renamed to say it's the plan) | Its implementation plan. |

### Logs: the builder

Copied byte-for-byte from the builder archive. These are logs, not specs: they record what happened and are never edited. Where a log records a decision, the decision belongs in a current doc (see ONLINE_BUILDER's status block).

| log | source in the archive | what it is |
|---|---|---|
| [logs/builder/online/PROGRESS.md](logs/builder/online/PROGRESS.md) | `online/PROGRESS.md` | The online builder's progress log: task status, reviews, Allen's decisions (K1–K3, M1, …), F7 runs 1–8 and what each found. |
| [logs/builder/online/ESCALATIONS.md](logs/builder/online/ESCALATIONS.md) | `online/ESCALATIONS.md` | Problems implementers escalated during the build-out (B4, D4, E2), with what was tried and what was decided. |
| `logs/builder/online/f7.stuck{,2,3,4,5}.md`, `.json`, `.log.gz` | `online/fixtures/plans/` | Stuck reports from F7 runs 1, 2, 6, 7 and 8: readable report, full JSON and compressed run log for each. |
| `logs/builder/online/f7.run{3,4}.stopped.log` | `online/fixtures/plans/` | Logs of F7 runs 3 and 4, stopped by hand. |
| [logs/builder/planner/REPORT.md](logs/builder/planner/REPORT.md) | `planner/REPORT.md` | Milestone report for the earlier planner on cobble.litematic. |
| [logs/builder/planner/report_notes.md](logs/builder/planner/report_notes.md) | `planner/report_notes.md` | The milestone verdict and notes. |
| [logs/builder/planner/correct_report.md](logs/builder/planner/correct_report.md), `correct_actor.log`, `correct_log.jsonl` | `planner/out_layer/` | The earlier planner's correction-loop report and logs. |

## Decisions so far

Settled by Allen:
- pure vanilla survival servers only (no op, no plugins, no creative);
- one HQ per deployment, able to span several servers (AD-K1.1);
- the deployment upgrades versions together, with two versions at once only when a wanted server forces it;
- a specialized Azalea build speaking 1.21.11 natively (AD-F5.1);
- decision code never imports Bevy or Azalea, only named mechanism crates do (AD-F5.2, live join check passed);
- reflexes can be switched off;
- "permit" means operator approval to modify player-made blocks;
- interrupts are contained and priced, not diagnosed, and fixed in cost-ranked batches (AD-K5.3).

The open ones follow.

## Open decisions: summary

Spec set v1.2, 2026-10-14. ARCHITECTURE v0.8 adopted the specs' earlier proposals and kept v0.7's section numbers, so every `ARCHITECTURE §n` reference still holds.

**What's flagged (⚑).** Only architecture points that are still open: they can't be settled yet, they're open to other opinions, or someone else owns them, and getting them wrong is costly to undo (Allen, 2026-10-13). Everything else, 283 subsections, is implementable as written, and details like stage order or thresholds can change later without harm. Decisions a person or an LLM makes while the swarm runs are part of each component's interface, not flags. They're listed at the end.

- **7 open architecture decisions** across 7 components, all deferred by Allen (2026-10-15): 6 not on the early path, and AD-K2.1 until its bake-off. Allen decided AD-F1.1, AD-F5.1, AD-K1.1, AD-F5.2, AD-K5.3, AD-K5.2, AD-F2.1, AD-F2.2, AD-B1.1, AD-C1.1, AD-C1.2, AD-B6.1, AD-F3.1, AD-K2.2 and AD-K5.1 (below). Each has options, a recommendation, why it needs you, what settles it, and what changes if it's decided late.
- **Both sides of every non-builder decision (7 open, 9 decided) are in [`DECISIONS.md`](DECISIONS.md)**, with what evidence would tip each one.
- **Nothing is left that must be settled before any code is written:** AD-F2.1, AD-F2.2 and AD-C1.2 were decided 2026-10-15.
- **No builder decision is open:** AD-C1.1, AD-C1.2, AD-B6.1 and AD-F3.1 were decided 2026-10-15 in the world-transformation frame. The redesign's own open points (`proposals/WORLD_TRANSFORMATION.md` §7 items 1–4) are deferred.
- **Deferred (Allen, 2026-10-15):** AD-B5.1, AD-C3.1, AD-C7.1, AD-C8.1, AD-C9.1 and AD-F6.1. None is on the early path; each recommendation stays as the default.
- **AD-K2.1 is deferred until its bake-off**, which runs before the LLM commander is built. AD-F2.2 and AD-B1.1 were decided from the Azalea benchmark (`reference/BENCH_AZALEA_2026-10-15.md`).
- **Decided (Allen, 2026-10-13 to 2026-10-15):** AD-F1.1, the server isn't an architecture question because adapting to a server is easy. AD-F5.1, Null runs on a specialized Azalea build with the latest features that speaks 1.21.11 natively, not through ViaVersion. AD-K1.1, one HQ per deployment, able to span several servers, with a server field in every location key. AD-F5.2 (2026-10-14), decision code (skills, reflexes, arbitration, squad tactics) has no Bevy or Azalea dependency and named mechanism crates import them directly, enforced by a cargo-deny allowlist (F5 §4.1); the live join check passed. AD-K5.3 (2026-10-14), interrupts: anything outside a job's expectations is contained, priced, escalated only on cost and fixed in cost-ranked batches (`INTERRUPTS.md`). AD-K5.2 (2026-10-15), scenario knowledge: facts as data from the jar, observation and the LLM prior; skill code only where the cost-ranked review shows a need; the LLM plans steps with no model in validated, quoted stretches (`proposals/GENERALIZATION.md` §0). AD-F2.1 (2026-10-15), belief keyed by fixed map tiles, with the operation service inside the node. AD-C1.2 (2026-10-15), a builder bot restocks on need. AD-K2.2 (2026-10-15), the LLM is in full control and the person holds the kill switch (halt and recall); boundaries are permission checks on low-level skills that the person sets once or disables, by default no attacking a player first and no breaking someone else's build, judged by comparing Null's belief with the current world. AD-F2.2 (2026-10-15), a separate belief store fed by a packet observer that forwards packets to a worker thread (`chan`). AD-B1.1 (2026-10-15), reuse Azalea's pathfinder with the default pool, and add a shared section cache first. Both from the Azalea benchmark. AD-C1.1, AD-B6.1, AD-F3.1 (2026-10-15), the builder decisions: parallel work across inert boundaries and any interleaving in printable regions; a placement table plus probing; one server pool. AD-K5.1 (2026-10-15), no officers in v1: the commander alone plans.
- **Capacity (Allen, 2026-10-13):** one laptop may carry about 1,000 bots at roughly 1 MB of extra memory each, so CPU and above all bandwidth are the limits (F1 §4.1, §4.11). Azalea's own guidance is a few hundred per swarm, and a swarm is one ECS that must finish each 50 ms tick, so 1,000 is a target to benchmark. Local testing is per component, not a full 2,000-bot run.
- **Spec audit** (`history/SPEC_AUDIT.md`, 2026-10-13): its two open questions became AD-F5.2 and AD-K1.1, both now decided. Its other findings are fixed in the specs, each marked "(spec audit #n)", except #6 (builder bot state during replay), which is noted in C1 §9 for the builder redesign.
- **Azalea reality check** (`reference/AZALEA_REALITY_CHECK.md`, 2026-10-13): its §7 factual corrections are applied in F1, F2, F5, B1, B2, B3, B6 and B7, each marked "(Azalea reality check, 2026-10-13)".
- **AD-F5.2 spike** (`reference/SPIKE_F5_BOUNDARY.md`, 2026-10-14): the decision-code split held across three Azalea builds, reflexes act in the same tick, and the host costs no measurable CPU. Applied in F1 §4.1, F5 and CONVENTIONS §3.5. A live vanilla-server run (spike §6) then passed too; it found that Azalea sends no `UseItem` for a block-targeted click.
- **24 components have none open:** F1, F2, F3, F4, F5, K1, B1, B2, B3, B4, B6, B7, C1, C2, C4, C5, C6, C10, C11, K3, K4, K5, K6 and K7. K3, K4 and K6 depend on decisions owned by other specs, noted in each.

**Reflexes can be turned off** (Allen, 2026-10-13). The contract is in `CONVENTIONS.md` §4, and it's settled, not flagged. Every reflex can be switched off at swarm, operation, squad, job or bot scope. "Off" still reports the trigger to the skill and squad. Jobs declare which reflexes they need off or assume on. Switch tables: B1 §4.9, B2 §4.9, B5 §4.9, B7 §4.10. Runtime side: F1 §4.5. Commands: K2 §4.5. Squads: K6 §4.7 and C7 §4.7. UI: K4 §4.7.

---

## Architecture decisions

| decision | recommendation | settled by | needed before |
|---|---|---|---|
| [AD-F2.1](F2-world-model.md) Shard belief by operation, or by fixed map tiles? Includes where belief physically lives (spec audit #3) | **Decided** (Allen, 2026-10-15): fixed tiles, with operations as the current writers; operation service inside the node | decided | any code |
| [AD-F2.2](F2-world-model.md) Extend Azalea's world storage, or keep a separate store? | **Decided** (Allen, 2026-10-15): separate store fed by a `chan` packet observer, plus a chunk-storage plug-in; Azalea benchmark | decided | any code |
| [AD-C1.2](C1-builder.md) How do items reach a builder bot during a certified plan? | **Decided** (Allen, 2026-10-15): restock on need | decided | the survival builder |
| [AD-C1.1](C1-builder.md) How does building with many bots keep the guarantee? | **Decided** (Allen, 2026-10-15): parallel work across inert boundaries; any interleaving in printable regions; one bot at a time in order-sensitive regions | decided | builder integration (§7 step 6) |
| [AD-B6.1](B6-placement-breaking.md) Precompute placement recipes, or probe on demand? | **Decided** (Allen, 2026-10-15): a table for common states, probing for the rest | decided | builder integration |
| [AD-F3.1](F3-test-harness.md) One server pool for the harness and the builder's simulator? | **Decided** (Allen, 2026-10-15): one pool, two job types | decided | the harness build-out |
| [AD-K2.1](K2-commander-api.md) Imperative commands, or a desired-state manifest? | **Deferred until the bake-off** (Allen, 2026-10-15); lean: The manifest hybrid, after a bake-off against a fake HQ | the bake-off, then Allen's call | the LLM commander (§7 step 5) |
| [AD-K2.2](K2-commander-api.md) Share the commander seat between Allen and the LLM, and on what terms? | **Decided** (Allen, 2026-10-15): the LLM in full control; the person holds halt and recall; boundaries are permission checks set once (no attacking a player first, no breaking others' builds) | decided | the LLM commander |
| [AD-K5.1](K5-operation-service.md) LLM officers at operation level in v1? | **Decided** (Allen, 2026-10-15): no; the commander alone, interface kept ready | decided | the LLM commander |
| [AD-K5.2](K5-operation-service.md) Scenario knowledge as data (profile rows, closed verb set) or as method and mob-module code? | **Decided** (Allen, 2026-10-15): the shape in `proposals/GENERALIZATION.md` §0 | decided | K5 methods and B5 modules (before they are written) |
| [AD-B1.1](B1-movement.md) Reuse Azalea's pathfinder, or write our own? | **Decided** (Allen, 2026-10-15): reuse Azalea's, default pool, shared section cache first; Azalea benchmark | decided | survival core (§7 step 2) |
| [AD-F6.1](F6-bus-deployment.md) Where does the bus's third quorum member live? | **Deferred** (Allen, 2026-10-15); lean: Defer; add a third small VPS once several always-on hosts run | Allen's call | distribution across hosts (§7 step 4) |
| [AD-C3.1](C3-logistics.md) Central depots, or storage per operation? | **Deferred** (Allen, 2026-10-15); lean: Per operation for small swarms, depots once several operations share a base | Allen's call | the first base with several operations |
| [AD-C9.1](C9-entity-logistics.md) Can one convoy's escort span two nodes? | **Deferred** (Allen, 2026-10-15); lean: No; split long trains into several convoys | Allen's call | the first convoy |
| [AD-C7.1](C7-squads-tactics.md) What coordinates fights bigger than one squad? | **Deferred** (Allen, 2026-10-15); lean: Bigger squads first, capped by density; battle groups only if measured fights need them | Allen's call | faction play (§7 step 8) |
| [AD-B5.1](B5-combat.md) Hand-written combat doctrine, or learned policies? | **Deferred** (Allen, 2026-10-15); lean: Hand-written, with a learned-micro seam built in from day one | Allen's call | faction play |
| [AD-C8.1](C8-faction.md) Should bots talk to people in v1? | **Deferred** (Allen, 2026-10-15); lean: Silent; keep the policy gate for later | Allen's call | faction play |

---

## Interface inputs (not flags)

These are the places where a component takes input from a person, the commander or an LLM, either while running (runtime decision) or as something written ahead of time (authored input). They're settled architecture: each spec names the channel the input arrives through and what Null does meanwhile.

| spec | subsection | kind · who |
|---|---|---|
| [F1](F1-swarm-runtime.md) | 4.9 Server deployment profile | runtime decision · person |
| [F3](F3-test-harness.md) | 4.4 Adversary models | authored input · person |
| [F3](F3-test-harness.md) | 4.7 Commander evaluation rubric | authored input · person |
| [F3](F3-test-harness.md) | 4.9 Live gate | runtime decision · person |
| [F4](F4-ops-telemetry.md) | 4.8 Batch review of interrupts | runtime decision · either |
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

- **The builder's internals** are being redesigned as world transformation ([proposals/WORLD_TRANSFORMATION.md](proposals/WORLD_TRANSFORMATION.md)). Until it is adopted, C1 specifies the interface, survival seams and inputs, and [ONLINE_BUILDER.md](ONLINE_BUILDER.md) v4.1 is the record of the earlier design.
- **ARCHITECTURE.md v0.8** (2026-10-14) adopted the specs' proposals beyond v0.7: permits and the narrowed break rule (B4 §4.8, K2 §4.4), self-fencing, the bandwidth and density limits, F3's live gate, and the decision-code host from AD-F5.2 (its §11 lists every change). The reflex contract was already in v0.7 §5. AD-F2.1 was later decided (2026-10-15, fixed tiles; ARCHITECTURE §3.7 and §11).
