# K7 Program service

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.2, §3.3, §3.10, §3.11, §10; WORKED_EXAMPLE_OSC.md §2, §3; REVIEW_RESPONSE.md. Conventions: `CONVENTIONS.md`.
**Depends on:** C11 (descriptors), C6 (tech tree), C1 (bills of materials), C10 (sites), C4 and C9 (rates), F2/K1 (stock and map), F3 (measurements). **Used by:** K1.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Production graph expansion | Deterministic | Recipes, descriptors and acquisitions expanded down to raw sources |
| 4.2 | Choosing sources (make vs build a facility) | Deterministic | Cost comparison over the goal's horizon with stated rates |
| 4.3 | Quotes with measured and claimed numbers | Deterministic | Every number tagged `[m]` or `[c]` with its source |
| 4.4 | Cutting programs into operations by place | Deterministic | Rules from §3.3 |
| 4.5 | Feasibility checks | Deterministic | Presence bands, simulation distance, available sources |
| 4.6 | Gates and cancel paths | Deterministic | Defined preconditions and a defined unwind order |
| 4.7 | Critical path and ETA | Deterministic | DAG scheduling with rate estimates |
| 4.8 | Approving a quote | Interface · runtime decision · commander | "Whether a big machine is worth it on this server is the commander's call" (§3.11) |
| 4.9 | Missing content | Deterministic | Detect and report `needs_content`; design is C11 §4.5 |

## Architecture decisions

None open.


---

## 1. Purpose

The program service is the one production-graph planner (ARCHITECTURE §3.11). It expands a goal and its descriptors into a full production graph, decides which facilities and sources supply each input and how much, produces a quote with every number marked measured or claimed, cuts the program into operations by place, and defines the gates and cancel path. HQ calls it so HQ stays thin.

## 2. Scope

**Owns:** production graphs; source choice; quotes; operation cuts; feasibility checks; gates; cancel unwinds; critical path and ETA; the rate store's use (with C11's measurements).

**Does not own:** running operations (K5); allocation (K1); approving (the commander through K2); designing content (C11 §4.5).

## 3. Interfaces

- `quote(goal, bindings)` → `Quote` (below).
- `plan(goal, approved_quote)` → `Program{operations[], edges (handovers), gates, quotas, cancel_path}`.
- `replan(program, changes)` when rates, stock or losses move.

```
QUOTE  <goal>                         [m] measured  [c] claimed
  Operations (by place): ...
  Critical path: ...
  Entities, presence, items (fully expanded, no "…"), tool wear, spoil haulage
  Lead time, peak bots, bot-hours (each [m] or [c])
  Feasibility: ...
  Risks: ...
  Choices made: sources picked, site candidates (C10 §4.5)
  Needs approval: ...
```

About 300 tokens whatever the swarm size (WORKED_EXAMPLE_OSC §2).

## 4. Requirements

### 4.1 Production graph expansion

Nodes: recipes (items in, items out), facilities (build cost, then a rate per bot-hour of presence), entity acquisitions (source, capture method, route), site searches (§3.10). Every input is expanded to raw sources, including tool wear and spoil haulage; nothing hides behind "…" (§3.11).

### 4.2 Choosing sources (make vs build a facility)

For each input: compare producing by hand against building a facility, over the goal's time horizon, using rates (measured preferred) and current stock. Run several sources in parallel when one is too slow (trial spawners). Operations' HTNs never re-derive sources (§3.3).

### 4.3 Quotes with measured and claimed numbers

Every number carries `[m]` with its run or `[c]` with its claimant. The quote highlights which `[c]` numbers drive the lead time, so the commander knows which measurements (like harness stage S0) would most reduce uncertainty.

### 4.4 Cutting programs into operations by place

One area with one set of claims is one operation, even with many steps in sequence; a new operation only for a different area, a moving region (convoy), or a different danger profile (§3.3).

### 4.5 Feasibility checks

- Presence bands against the server's simulation distance (§3.10).
- Sources exist (known trial chambers, slime chunks or a survey plan).
- Packs pass acceptance on the running version (F5 §4.8) and are allowed on this server (C11 §4.9).

### 4.6 Gates and cancel paths

- Handover edges are gates: before a handover, the receiving side re-checks its preconditions against current belief (§3.11; K5 §4.8).
- Cancel unwinds in order: stop new work, park or release cargo, return stock to HQ, release claims; the cancel report lists what is kept.

### 4.7 Critical path and ETA

ETA from the DAG and rates; expected losses only move the ETA. Only a critical-path slip beyond threshold or a resource below minimum reaches the commander (§3.11).

### 4.8 Approving a quote

> **Interface input (runtime decision).** Who: the commander (a person for large programs or anything with an approval-class step; the LLM within policy limits).
> Why: whether a program is worth its cost, risk and lead time on this server, now, against other goals, is a strategic judgment (§3.11); the quote informs it but can't make it.
> Enters the system as: `approve(quote)` or `reject(quote, reason)` through K2, or a standing policy that pre-approves quotes under a cost threshold.
> Null's side: no program over the approval threshold starts without approval; quotes show the measured/claimed split and the main risks so the decision is informed; the commander can ask for S0-style measurements before deciding.

### 4.9 Missing content

When a goal needs an output no loaded pack, recipe or source provides, the quote says `needs_content(output, constraints)` rather than guessing. Supplying it is C11 §4.5.

## 5. Failure handling

| failure | response |
|---|---|
| Rates change materially after approval | Replan; if the critical path slips beyond threshold, escalate |
| A source is lost (chamber raided) | Replan with remaining sources; escalate if none |
| A gate fails | K5 escalates; K7 offers replan options |

## 6. Scale and performance

A handful of programs at once; quotes for large builds may take seconds to compute.

## 7. Version-sensitive parts

Recipes and mechanics via F5; pack compatibility per version.

## 8. Tests

- Skybender quote from WORKED_EXAMPLE_OSC §2: fully expanded, every number tagged, simulation-distance feasibility computed.
- Cancel mid-program: unwind order followed, cargo parked, cancel report lists kept facilities.
- Measured capture rate lower than claimed: ETA moves, no escalation until the threshold.

## 9. Open questions

- How should quotes express uncertainty from `[c]` numbers: ranges, or a best guess with the drivers listed? Start with drivers listed.
