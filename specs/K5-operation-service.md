# K5 Operation service

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.3, §3.6, §3.7, §3.8, §3.11, §3.14, §4.3, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** K7 (quotas, sources), K6 (squads), F2 (shard), C3 (ledger), F1 (accounts). **Used by:** K1.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | HTN planner | Deterministic | Expands steps that existing searches solve; the LLM plans steps with no model (AD-K5.2); replans on failure |
| 4.2 | Scheduler | Deterministic | Jobs to squads by role, location and claims |
| 4.3 | Presence scheduling | Deterministic | Location constraints over time, as a constraint scheduler |
| 4.4 | Ledger and world shard | Deterministic | Per-operation state with an event log |
| 4.5 | Claims registry | Deterministic | Leases with owners, priority and expiry |
| 4.6 | Recovery rules | Deterministic | Known failure classes map to defined recoveries; anything else is an interrupt, contained the same way every time |
| 4.7 | Escalation | Deterministic | Escalate only what the parent can act on, with remedies tried |
| 4.8 | Gates | Deterministic | Handover preconditions re-checked against fresh belief |
| 4.9 | Restart from the event log | Deterministic | Rebuild state and resume |
| 4.10 | Operation officers for fuzzy work | Interface · runtime decision · LLM (optional) | Some operations (diplomacy, an unusual build, a raid) have work no HTN method covers |

## Architecture decisions

### AD-K5.2 Scenario knowledge as data, or as method code? (decided)

**Decided by Allen, 2026-10-15** in the shape of [proposals/GENERALIZATION.md](proposals/GENERALIZATION.md) §0. No ⚑. Neither option as first framed scales, because scenarios and high-level tasks can't be listed. Facts are data from the jar, from observation and from the LLM's prior. Skills are code only where the cost-ranked interrupt review shows a need. Goals state and check tasks. The LLM plans the steps that have no model, in validated and quoted stretches, with the existing searches as subroutines. The options were (a) method code per goal kind and a module per mob, and (b) a closed verb set with profile rows and operator data per goal kind. Both sides and the three paper tests are in [DECISIONS.md](DECISIONS.md#ad-k52-scenario-knowledge-as-data-or-as-method-code).

### AD-K5.3 Interrupts: contain and price, or classify and resolve? (decided)

**Decided by Allen, 2026-10-14: contain and price.** Adopted as the interrupt contract ([INTERRUPTS.md](INTERRUPTS.md)); it was proposed as "the surprise contract". No ⚑. The options were (a) known failure classes plus one triage ticket per new signature, each kind fixed by a new class, and (b) contain anything outside a job's expected envelope, price it as a re-plan, escalate only on cost, and fix in cost-ranked batches with token budgets. Both sides are kept in [DECISIONS.md](DECISIONS.md#ad-k53-interrupts-contain-and-price-or-classify-and-resolve).

### AD-K5.1 LLM officers at the operation level in the first version? (decided)

**Decided by Allen, 2026-10-15: the commander alone (a).** In v1 the only LLM is the commander. It plans the steps that have no model for every operation (AD-K5.2). The officer interface stays identical to the commander's, so officers can be added per operation if the commander's context or wake budget overflows. No ⚑. The options were (a) no officers, and (b) optional officers for fuzzy operations (ARCHITECTURE §4.3). Both sides are kept in [DECISIONS.md](DECISIONS.md#ad-k51-llm-officers-at-the-operation-level-in-v1).

---

## 1. Purpose

An operation does most of the scheduling, scoped to one goal in one area (ARCHITECTURE §3.3). It's cut by place, not by step: one area, one set of claims. It runs an HTN planner to meet the quotas the program service gave it (since AD-K5.2: deterministic expansion plus LLM-planned stretches, no methods per goal kind), schedules jobs onto squads, keeps its own ledger, world shard and claims, and applies recovery rules.

## 2. Scope

**Owns:** the HTN planner (expansion through existing searches, validation and running of LLM-planned stretches, saved routines; AD-K5.2); the scheduler; presence scheduling; its ledger and world model shard (with C3, F2); its claims registry; recovery rules; escalation to HQ; handover gates; restart from its log; optional officer integration.

**Does not own:** where inputs come from (K7 decides sources); allocation between operations (K1); per-bot execution (K6, skills).

## 3. Interfaces

- **From K1:** create with sub-goal, quotas, budget, region; resize; retire; transfer orders.
- **From K7:** quotas and source assignments; gate definitions.
- **To K6:** job batches; reflex settings per job (CONVENTIONS §4.4); errand preparation and holding.
- **From K6:** job outcomes, merged observations, exceptions.
- **To K1:** summary (fixed fields), exceptions, escalations with remedies tried.
- **To F1:** account headcount requests by role and region.

## 4. Requirements

### 4.1 HTN planner

- Decomposes the operation's sub-goal into jobs over the skill catalogue and recipe graph (§3.6). Decides *how*, never *where from* (sources are K7's).
- Replans when jobs fail rather than planning everything up front. Never re-derives sources.
- *Changed by AD-K5.2 (Allen, 2026-10-15):* there is no method per goal kind. Steps that an existing search solves (K7's recipe graph, B1, C5, B4) expand deterministically. The LLM commander (alone in v1, AD-K5.1) plans the steps that have no model, as a stretch of goals and a plan over the closed verb set. The stretch is validated and quoted before it runs (proposals/GENERALIZATION.md §0). Plans that work are saved as routines with parameters (C11 §4.4), and these routines act as a cache of methods. Before the decision, this bullet read: "Methods are code per goal kind (build, mine district, base, convoy, chamber capture, defence), versioned with the skill library."

### 4.2 Scheduler

Hands jobs to squads (and through them to bots) by role, location, gear and claims held. Respects preemption floors for cargo jobs. Uses job contracts' `reflexes_off` and `reflexes_assumed_on` and returns `blocked(reflex_policy)` when policy conflicts with a job's assumptions.

### 4.3 Presence scheduling

Presence constraints (§3.10: at, band, count, phase, outside) from facilities, builds and errands become location constraints over time. Bots meeting presence can do other nearby work. Idle timeouts are handled by periodic action (F1 §4.10). Restarts drop presence at once; the restart protocol restores it (F1 §4.8).

### 4.4 Ledger and world shard

The operation's item ledger (C3) and its world model shard (F2), with an append-only event log shipped to a VPS (§3.7).

### 4.5 Claims registry

Region, block, container, item, entity and bot claims for the operation's region; cross-shard claims owned by one side and forwarded; moving claims registered as right of way when a convoy passes (§3.7). Every claim expires; leaked claims are found by audit.

### 4.6 Recovery rules

Per job kind: requeue, reassign, release claims, recover items, mark a region dangerous, checkpoint resume for cargo jobs. Anything that matches no known class is an interrupt (INTERRUPTS §2). The bot backs off to its last safe position, the place is quarantined with an expiry, claims are released, and the job ends `interrupted(incident)` and is placed elsewhere. The operation prices the quarantine against its quotas.

### 4.7 Escalation

Escalate only what HQ can act on (more bots, resources from other operations, approvals), after trying local remedies, with those remedies listed (§3.1). Merge similar exceptions ("37 bots blocked: no pickaxes" is one line). An interrupt escalates only when its price makes the operation's quotas infeasible within budget (INTERRUPTS §2.3).

### 4.8 Gates

Before a handover into this operation (a site to build on, cargo to install), re-check preconditions against current belief: claims held, no intruder sightings since the survey, counts fresh, verification passed. A failed gate escalates instead of proceeding (§3.11).

### 4.9 Restart from the event log

A crashed operation restarts and rebuilds its state from its log; meanwhile its bots finish their current safe step and hold (§3.8).

### 4.10 Operation officers for fuzzy work

> **Interface input (runtime decision).** Who: LLM (an officer scoped to one operation), optional; a person can sit above it.
> Why: some operations involve work no HTN method covers well: diplomacy, an unusual one-off build sequence, a raid plan against a particular base. ARCHITECTURE §4.3 allows an LLM officer with the same bounded interface, scoped to the operation.
> Enters the system as: the officer's decisions through the same typed commands an operation accepts (quotas within budget, job priorities, escalation answers), never per-bot control.
> Null's side: the default is a deterministic operation; officers are opt-in per operation, see only their ~10 squads, and their memos pass the quarantine rule (COMMANDER_INTERFACE §7.1, open question on officer injection).

## 5. Failure handling

| failure | response |
|---|---|
| Operation crash | Restart from log; bots hold |
| Node hosting its bots dies | F1 re-places accounts; jobs replanned or resumed |
| Gate fails | Escalate with the failed precondition |
| Quota infeasible | Escalate to HQ with the shortfall and what was tried |

## 6. Scale and performance

50–300 bots per operation, with the active headcount in one area capped by measured density (F1 §4.11; spec audit #2); decides every ~1–2 s (§2). Runs preferably on the host with most of its bots (§3.8).

## 7. Version-sensitive parts

Expansion and saved routines use features through F5 queries.

## 8. Tests

- Kill an operation mid-build: restart from log, no claims lost, no duplicate jobs.
- Gate with a fresh intruder sighting: handover refused and escalated.
- Presence: a facility needing `within_32` during arming is staffed continuously, with the bot doing nearby sorting.

## 9. Open questions

- How a new goal is matched to a saved routine, and when a saved routine is stale in a changed world (AD-K5.2; proposals/GENERALIZATION.md §0.5). The earlier question, how large a method set gets before splitting into goal-kind plugins, no longer applies.
