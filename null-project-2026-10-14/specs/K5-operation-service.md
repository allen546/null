# K5 Operation service

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.3, §3.6, §3.7, §3.8, §3.11, §3.14, §4.3, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** K7 (quotas, sources), K6 (squads), F2 (shard), C3 (ledger), F1 (accounts). **Used by:** K1.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | HTN planner | Deterministic | Methods over the skill catalogue and recipe graph; replans on failure |
| 4.2 | Scheduler | Deterministic | Jobs to squads by role, location and claims |
| 4.3 | Presence scheduling | Deterministic | Location constraints over time, as a constraint scheduler |
| 4.4 | Ledger and world shard | Deterministic | Per-operation state with an event log |
| 4.5 | Claims registry | Deterministic | Leases with owners, priority and expiry |
| 4.6 | Recovery rules | Deterministic | Known failure classes map to defined recoveries |
| 4.7 | Escalation | Deterministic | Escalate only what the parent can act on, with remedies tried |
| 4.8 | Gates | Deterministic | Handover preconditions re-checked against fresh belief |
| 4.9 | Restart from the event log | Deterministic | Rebuild state and resume |
| 4.10 | Operation officers for fuzzy work | Interface · runtime decision · LLM (optional) | Some operations (diplomacy, an unusual build, a raid) have work no HTN method covers |

## Architecture decisions

### ⚑ AD-K5.2 Scenario knowledge as data, or as method code?

- **Options:**
  - (a) Method code per goal kind and module per mob, as written above.
  - (b) A closed verb set, with mob and structure knowledge as profile rows (F5) and goal kinds as operator data. New scenarios add rows, not code. Plans outside the recipe graph are composed (commander or officer) and quoted.
- **Recommendation:** (b). The paper test in GENERALIZATION §7 passed on 2026-10-14 (no new verb or method; 9 new fields, falling per task), with revisions folded into GENERALIZATION.md (`design/GENERALIZATION_PAPER_TEST.md`).
- **Why it needs Allen:** It decides where the skill-code budget goes, and it reopens AD-K5.1 because composition for non-recipe tasks is what officers are for.
- **Settled by:** The paper test, then Allen's call.
- **If decided late:** B5's per-mob modules and K5's methods get written first and then replaced.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-k52-scenario-knowledge-as-data-or-as-method-code).

### ⚑ AD-K5.3 Surprises: contain and price, or classify and resolve?

- **Options:**
  - (a) As written: known failure classes with their recovery rules; anything else is requeued once, then `blocked(unknown_failure)`, with one triage ticket per new signature (F4 §4.8). Each new kind is fixed by a new class.
  - (b) The surprise contract: anything outside a job's expected envelope is contained (back off, quarantine, requeue elsewhere) and priced as a re-plan. It escalates only when the price leaves the approved quote's limits, when containment is failing, or when swarm-wide containment cost rises. Fixes come from a cost-ranked batch review with token budgets.
- **Recommendation:** (b). Diagnosing every kind of surprise costs tokens that grow with the number of kinds, and in an open world that number has no end. Containment costs efficiency instead, and K7 can price that.
- **Why it needs Allen:** It sets where tokens and engineering time go, and it accepts detours and abandoned places as the normal cost of not understanding.
- **Settled by:** Allen's call, informed by F3 runs with injected surprises (design/SURPRISES.md §9).
- **If decided late:** Per-kind failure classes and triage tickets pile up first and are then replaced.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-k53-surprises-contain-and-price-or-classify-and-resolve).

### ⚑ AD-K5.1 LLM officers at the operation level in the first version?

- **Options:**
  - (a) No. Every operation is deterministic, and the only LLM is the commander.
  - (b) Optional officers for fuzzy operations (diplomacy, raids, unusual builds), as ARCHITECTURE §4.3 allows.
- **Recommendation:** (a) for v1, keeping the officer interface identical to the commander's so (b) can be added.
- **Why it needs Allen:** Officers add cost, a new path for injected text, and more autonomy below the commander.
- **Settled by:** Allen's call.
- **If decided late:** Easy to add later if the interface stays the same.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-k51-llm-officers-at-the-operation-level-in-v1).


---

## 1. Purpose

An operation does most of the scheduling, scoped to one goal in one area (ARCHITECTURE §3.3). It's cut by place, not by step: one area, one set of claims. It runs an HTN planner to meet the quotas the program service gave it, schedules jobs onto squads, keeps its own ledger, world shard and claims, and applies recovery rules.

## 2. Scope

**Owns:** the HTN planner and its methods for the operation's goal kind; the scheduler; presence scheduling; its ledger and world model shard (with C3, F2); its claims registry; recovery rules; escalation to HQ; handover gates; restart from its log; optional officer integration.

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
- Methods are code per goal kind (build, mine district, base, convoy, chamber capture, defence), versioned with the skill library.
- *Proposed (AD-K5.2, not adopted):* methods are operator data over a closed verb set (preconditions, effects, cost), so a new goal kind adds rows and not method code. Tasks the operator data doesn't cover are composed as typed plans by the commander or an officer, then validated and quoted (design/GENERALIZATION.md §3, §4).

### 4.2 Scheduler

Hands jobs to squads (and through them to bots) by role, location, gear and claims held. Respects preemption floors for cargo jobs. Uses job contracts' `reflexes_off` and `reflexes_assumed_on` and returns `blocked(reflex_policy)` when policy conflicts with a job's assumptions.

### 4.3 Presence scheduling

Presence constraints (§3.10: at, band, count, phase, outside) from facilities, builds and errands become location constraints over time. Bots meeting presence can do other nearby work. Idle timeouts are handled by periodic action (F1 §4.10). Restarts drop presence at once; the restart protocol restores it (F1 §4.8).

### 4.4 Ledger and world shard

The operation's item ledger (C3) and its world model shard (F2), with an append-only event log shipped to a VPS (§3.7).

### 4.5 Claims registry

Region, block, container, item, entity and bot claims for the operation's region; cross-shard claims owned by one side and forwarded; moving claims registered as right of way when a convoy passes (§3.7). Every claim expires; leaked claims are found by audit.

### 4.6 Recovery rules

Per job kind: requeue, reassign, release claims, recover items, mark a region dangerous, checkpoint resume for cargo jobs. Novel failures fall back to F4 §4.8's generic handling.

*Proposed (AD-K5.3, not adopted): the generic handling is containment. The bot backs off to its last safe position, the place is quarantined with an expiry, claims are released and the job is placed elsewhere. The operation prices the quarantine against its quotas, and escalates to HQ only when its quotas become infeasible (design/SURPRISES.md §2).*

### 4.7 Escalation

Escalate only what HQ can act on (more bots, resources from other operations, approvals), after trying local remedies, with those remedies listed (§3.1). Merge similar exceptions ("37 bots blocked: no pickaxes" is one line).

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

HTN methods use features through F5 queries.

## 8. Tests

- Kill an operation mid-build: restart from log, no claims lost, no duplicate jobs.
- Gate with a fresh intruder sighting: handover refused and escalated.
- Presence: a facility needing `within_32` during arming is staffed continuously, with the bot doing nearby sorting.

## 9. Open questions

- How large can one operation's HTN method set get before it should split into goal-kind plugins? Measure when the third goal kind is written.
