# Null: project docs

Null is a swarm of Minecraft bots under one commander (a person or an LLM). It runs on pure vanilla survival servers through legit play, built on a custom Azalea build that speaks 1.21.11. This page says what each doc is and whether it is current. Last updated 2026-10-14.

**Start here:** [design/ARCHITECTURE.md](design/ARCHITECTURE.md) (the design), then [specs/INDEX.md](specs/INDEX.md) (component specs and what's still open).

**Status words.** *Current*: the source of truth, kept up to date. *Reference*: still accurate where it doesn't conflict with a current doc, but not maintained. *Historical*: kept as a record; a current doc supersedes it.

## Design

| Doc | Status | What it is |
|---|---|---|
| [design/ARCHITECTURE.md](design/ARCHITECTURE.md) | Current (v0.8) | The swarm architecture: 31 components in four groups (F, B, C, K), the control model, HQ, world model, version layer, risks. §11 lists what changed from v0.7. |
| [design/WORKED_EXAMPLE_OSC.md](design/WORKED_EXAMPLE_OSC.md) | Current | The Orbital Skybender walked through the architecture end to end, as a test of the control model. |
| [design/GENERALIZATION.md](design/GENERALIZATION.md) | **Proposal**, not adopted (revised after the paper test) | No skill code per scenario: a closed verb set, mob and place knowledge as F5 data rows, interpreted fields counted as code, a closed plan grammar, generic safe defaults, outcomes that ratchet. Decision is AD-K5.2. |
| [design/GENERALIZATION_PAPER_TEST.md](design/GENERALIZATION_PAPER_TEST.md) | Current | The §7 paper test run on five unseen tasks (2026-10-14): passed on verbs and methods; the cost moved into 9 fields and 5 planner features. |
| [design/SURPRISES.md](design/SURPRISES.md) | **Proposal**, not adopted | The surprise contract: detect anything outside a job's expected envelope; contain it (back off, quarantine, requeue elsewhere) without diagnosing it; price it as a re-plan; escalate only on cost; fix from a cost-ranked batch review with token budgets. Decision is AD-K5.3. |
| [design/BREEZE_PIPELINE_FAILURES.md](design/BREEZE_PIPELINE_FAILURES.md) | Reference | How the breeze capture and transport pipeline fails. Its fixes are in ARCHITECTURE §3.14 and C9. |
| [design/COMMANDER_INTERFACE.md](design/COMMANDER_INTERFACE.md) | Reference, partly adopted | Options for showing the swarm to an LLM commander, written against v0.4. What was adopted is in ARCHITECTURE §4 and K2/K3; the rest is AD-K2.1. |
| [design/REVIEW_OSC_CONTROL_METHOD.md](design/REVIEW_OSC_CONTROL_METHOD.md) | Historical | Independent review of the worked example's control method (v0.6). |
| [design/REVIEW_RESPONSE.md](design/REVIEW_RESPONSE.md) | Historical | The response to that review; its changes landed in v0.7. |

## Component specs

| Doc | Status | What it is |
|---|---|---|
| [specs/INDEX.md](specs/INDEX.md) | Current (v1.2) | Entry point: every open architecture decision (⚑), who settles it, and the runtime decisions each component exposes. |
| [specs/CONVENTIONS.md](specs/CONVENTIONS.md) | Current | Shared rules for all specs: verdicts, claims vs permits vs territory (§3.3a), the decision/mechanism crate rule (§3.5), the reflex switch contract (§4). |
| [specs/DECISIONS.md](specs/DECISIONS.md) | Current | Both sides of each open non-builder decision, and what evidence would tip it. |
| specs/F1–F6, B1–B7, C1–C11, K1–K7 | Current | One spec per component, named by its ARCHITECTURE §10 id. |
| [specs/debates/AD-F5.2-bevy-boundary.md](specs/debates/AD-F5.2-bevy-boundary.md) | Historical (experiment run) | The three-advocate debate on whether skills should see Bevy/Azalea. Its experiment is the spike below. |

## Research

| Doc | Status | What it is |
|---|---|---|
| [research/AZALEA_REALITY_CHECK.md](research/AZALEA_REALITY_CHECK.md) | Current reference | What Azalea really provides, and the recipe for the 1.21.11 fork (branch tip plus 38 cherry-picks). |
| [research/SPIKE_F5_BOUNDARY.md](research/SPIKE_F5_BOUNDARY.md) | Current | The AD-F5.2 spike (2026-10-14): the decision/mechanism split tested on three Azalea builds in the simulator, then on a live vanilla 1.21.11 server (§6, passed except mining and many-bot load). |
| [research/builder-hard-cases.md](research/builder-hard-cases.md) | Reference | Hard cases for the universal schematic builder: placing without redstone firing, and getting entities into place. |
| [research/minecraft-tech-inventory.md](research/minecraft-tech-inventory.md) | Reference | Survey of vanilla 1.21.11 technical builds (common, digital redstone, SMP-specific), with what each means for a bot builder. |

## Review

| Doc | Status | What it is |
|---|---|---|
| [review/SPEC_AUDIT.md](review/SPEC_AUDIT.md) | Historical (closed) | Error audit of the spec set; every finding is resolved or handed off. |

## Other

| Path | What it is |
|---|---|
| [ONLINE_BUILDER.md](ONLINE_BUILDER.md) | Allen's online builder design (v4.1). Owned by Allen's builder agent; builder ordering decisions (AD-C1.1, AD-C1.2, AD-B6.1, AD-F3.1) belong there. |
| uploads/ | Files people attach in the project. |
| .notes/inputs.md | Working notes on supplied files, for Claude sessions. |

## Decisions so far

Settled by Allen: pure vanilla survival servers only (no op, no plugins, no creative); one HQ per deployment, able to span several servers (AD-K1.1); the deployment upgrades versions together, with two versions at once only when a wanted server forces it; a specialized Azalea build speaking 1.21.11 natively (AD-F5.1); decision code never imports Bevy or Azalea, only named mechanism crates do (AD-F5.2, live join check passed); reflexes can be switched off; "permit" means operator approval to modify player-made blocks. Open ones are in [specs/INDEX.md](specs/INDEX.md), including AD-K5.2 and AD-K5.3 (both proposed 2026-10-14).
