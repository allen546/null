# K1 HQ

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §2, §3.1, §3.2, §3.6, §3.7, §3.11, §3.12, §4, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** K7 (program service), K5 (operations), K2 (commands in), F2 (coarse map), F6. **Used by:** K2/K3 (digest out), the commander.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Goal tree and programs | Deterministic | Goals map to programs through K7; HQ tracks them |
| 4.2 | Allocation between operations | Deterministic | Allocation by priority and critical path under headcount limits |
| 4.3 | Transfers between operations | Deterministic | Transfer orders executed by carriers (C3) |
| 4.4 | Annotations | Deterministic | Stored, displayed and resolved by name; one closed set of effects honoured |
| 4.5 | Coarse map | Deterministic | Region summaries merged from shards |
| 4.6 | Cross-operation claims | Deterministic | Fixed arbitration (priority, then age) |
| 4.7 | Policy enforcement | Deterministic | Policies are rules; HQ checks actions against them |
| 4.8 | Digest | Deterministic | A fixed-size summary with ranked, merged exceptions |
| 4.9 | Critical path tracking | Deterministic | Computed from the program DAG and operation ETAs |
| 4.10 | Escalations to the commander | Interface · runtime decision · commander | What HQ can't settle by policy (contested priorities, approvals, strategy) is the commander's decision by design |
| 4.11 | Failover | Deterministic | Standby takes over from the shared log |

## Architecture decisions

### AD-K1.1 Does one HQ cover one server world, or several? (decided)

Decided by Allen, 2026-10-13: one HQ per deployment, able to span several servers. Consequences, settled:
- Every location key (coordinates, regions, tiles, ledger entries, convoys, coarse-map rows, permits) carries a `server` field (F2, C3, C9, K1).
- Items and entities can't cross servers, so transfers, handovers and convoys between servers are refused at planning time (K7) rather than failing later.
- Moving a bot between servers is a relog: its account lease moves (F1 §4.2), and it arrives with whatever inventory that account has on the target server.
- One commander, one digest: the digest groups operations by server, so its size stays bounded by operation count, not by server count.
- Server profiles (F1 §4.9) and the Minecraft version (F5) are per server; normally the whole deployment runs one version and ports together; two versions at once only when a server forces it, with a node binary per version (F5 §4.1, §4.7) (Allen, 2026-10-14).

HQ's shape also depends on AD-K2.1 (manifest or imperative commands).


---

## 1. Purpose

HQ is deliberately thin (ARCHITECTURE §3.2). It owns the commander's goals and the programs they become, allocates bots and resources between operations, keeps the operator's annotations and the coarse map, enforces swarm-wide policies, arbitrates cross-operation claims, and writes the digest. It never assigns a job to a bot.

## 2. Scope

**Owns:** the goal tree; program records and their critical paths; allocation of headcount; transfer orders; annotations; the coarse map; cross-operation claims; policy enforcement; the digest; escalation routing to the commander; failover.

**Does not own:** planning programs (K7); planning or scheduling jobs (K5); squads (K6); the command surface (K2); the LLM presentation (K3).

## 3. Interfaces

- **From K2:** goals, priorities, policies, annotations, allocations, approvals, errands, halt and recall.
- **To K7:** `plan(goal)` → program and quote; `replan(program)`.
- **To K5:** create, resize, retire operations; quotas; budgets; regions; transfer orders.
- **From K5:** summaries (fixed fields), exceptions, escalations with remedies tried.
- **To K2/K3:** digest, inspect results, escalation tickets.

## 4. Requirements

### 4.1 Goal tree and programs

Goals from the commander (`Obtain`, `Build`, `Defend`, `KillDragon`, `Explore`, `Hold`, `Errand`) become programs via K7 (§3.6, §3.11). HQ tracks each program's state, stage, critical path and ETA, and shows one row per program in the digest.

### 4.2 Allocation between operations

- HQ allocates bots to operations as quotas by role and gear, never jobs to bots (§3.2). "Move 30 bots from mining to defence" is one decision.
- Order: higher-priority goals first; within a program, critical-path operations first; preemption floors respected (§3.14).
- Allocation changes are damped (minimum dwell time per operation) so headcount doesn't flip-flop.

### 4.3 Transfers between operations

Items move between operations' ledgers as transfer orders carried out by carrier bots (§3.7, C3 §4.7). HQ sees only aggregates.

### 4.4 Annotations

- Operator markers and regions with label, notes and optional effect from the closed set: `keep_out`, `defend`, `rally`, `presence_target` (§3.12).
- Labels and notes are free text that Null doesn't interpret. Goals can refer to annotations by name.
- Effects propagate to operations (pathing, claims, rally points, presence targets).

### 4.5 Coarse map

Regions, owners, territory, threats, known structures, per-region stock hints, headcount and danger, merged from operation shards' summaries (§3.7). Stored in Postgres on a VPS.

### 4.6 Cross-operation claims

Big regions, territory borders, and disputes between shards. Arbitration: higher priority wins; equal priority, older claim wins; still tied, escalate (4.10).

### 4.7 Policy enforcement

Swarm-wide policies (§4.2): always allowed, allowed by policy, needs approval. HQ checks every action that needs approval and routes it as a ticket; it applies standing policies set by the commander, including reflex policies (CONVENTIONS §4).

### 4.8 Digest

- One row per program or operation (goal, headcount, progress, stock shortfalls, health, top issue), ranked and merged exceptions (top K), swarm aggregates, deltas since the last wake, and one line when any scope has survival reflexes off.
- Fixed size whatever the bot count (§4.3). K3 renders it for the LLM; K4 renders it for a person.

### 4.9 Critical path tracking

From the program DAG and operation ETAs. HQ moves bots onto the critical path within policy (§3.11).

### 4.10 Escalations to the commander

> **Interface input (runtime decision).** Who: the commander (a person or the LLM; a person for anything in the "needs approval" class).
> Why: by design HQ only settles what policy covers; contested priorities with no rule to break the tie, approvals (attacking a player, arming, large builds, scarce spending), critical-path slips beyond threshold and strategic choices have no computable answer and belong to the commander (ARCHITECTURE §3.1, §4.2).
> Enters the system as: answers to tickets, approvals, or new policies and priorities through K2.
> Null's side: every ticket is merged across affected bots and operations, carries the remedies already tried and the options with consequences, and has a default from policy with a deadline, so the swarm keeps going if the commander is slow (COMMANDER_INTERFACE §5, §9). Expected losses never escalate (§3.11).

### 4.11 Failover

HQ runs with a standby; the standby takes over from the shared log. While HQ is down, operations keep running their current goals and nothing new is allocated (§3.8).

## 5. Failure handling

| failure | response |
|---|---|
| HQ down | Standby takeover; operations continue |
| Operation silent | Mark stale in the digest; after a timeout, treat as down and reallocate its bots |
| Escalation flood | Merge by cause, cap at K, resolve the rest by policy defaults |

## 6. Scale and performance

Decides every ~10 s (§2). Sees ~10–30 operations, never bots. Digest (manifest, inbox and deltas) within ARCHITECTURE §4.3's 4–8k tokens, about 6k at 2,000 bots (COMMANDER_INTERFACE §9), regardless of bot count (spec audit #14, 2026-10-13).

## 7. Version-sensitive parts

None; HQ is version independent (F6 §4.2).

## 8. Tests

- `Obtain(oak_log, 64)` across two bots in the one-process skeleton (ARCHITECTURE §7 step 1).
- Digest size at 20 and 2,000 simulated bots stays under budget.
- Two equal-priority operations contest a region: escalation with options and a default.

## 9. Open questions

- The commander interface's recommended manifest (COMMANDER_INTERFACE §9) would make HQ reconcile toward a desired-state document rather than apply imperative commands. Not adopted in v0.7; see AD-K2.1.
