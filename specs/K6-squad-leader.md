# K6 Squad leader

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.4, §3.8, §3.10, §4.1 (errands), §9, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** F1 (runs inside the node), C7 (plays), skills. **Used by:** K5.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Job batch assignment | Deterministic | Assign jobs to 5–20 bots by position, role and inventory |
| 4.2 | Local retries | Deterministic | Rerun or reassign failed jobs before reporting |
| 4.3 | Tactics loop | Deterministic | Executes authored plays at 4–10 Hz (doctrine is an authored input, C7 §4.5) |
| 4.4 | Observation merging | Deterministic | Deduplicate and merge before passing up |
| 4.5 | Formation checks | Deterministic | Every tick for convoys and escorts |
| 4.6 | Errand holding and trigger | Deterministic | Hold prepared bots and claims; trigger with no planning in between |
| 4.7 | Reflex application | Deterministic | Apply job and play reflex settings and restore after |
| 4.8 | Moving claim renewal | Deterministic | Renew locally for convoys, never through HQ |

## Architecture decisions

None of its own. A war-time battle group (AD-C7.1) would reuse its code.


---

## 1. Purpose

A squad is 5–20 bots with a leader process, not a leader bot (ARCHITECTURE §3.4). It takes job batches from its operation and assigns them, retries locally before reporting, runs the fast tactics loop in combat, keeps convoy formations, holds prepared errands, and merges its bots' observations. It runs inside the swarm node, so it never waits on the network. Whether its tactics are decision code over a `SquadView` (the AD-F5.2 judge's lean) or mechanism code is still open (F5 AD-F5.2).

## 2. Scope

**Owns:** job assignment within the squad; local retries; the tactics loop's execution; observation merging; formation checks; errand holding and triggering; applying reflex settings per job and per play; renewing moving claims for convoys it escorts.

**Does not own:** which jobs exist (K5); plays and doctrine (C7); reflex definitions (B specs).

## 3. Interfaces

- **From K5:** job batches; errand preparation; reflex settings at squad scope.
- **To bots (in-process ECS events):** `RunJob`, `SetReflexes`, play role assignments.
- **From bots:** outcomes, `ReflexSuppressed`, observations, stuck reports.
- **To K5:** batch outcomes, merged observations, exceptions.
- **From the commander gateway (errands only):** `trigger(errand, step)` routed straight to the holding squad.

## 4. Requirements

### 4.1 Job batch assignment

Assign each job to the bot that can start it soonest: nearest, with the right items and tools, in the right role. Rebalance within the squad as jobs finish.

### 4.2 Local retries

A failed job is retried once by the same bot if the failure is transient, then reassigned within the squad, then reported. Reports carry what was tried (§3.1).

### 4.3 Tactics loop

At 4–10 Hz in combat: select the play (C7 §4.6), assign roles, choose targets, keep formation, call retreat. No network round trip inside the loop.

### 4.4 Observation merging

Deduplicate and merge observations from the squad's bots (the same chunk seen by five bots is one observation) before writing to the operation's shard.

### 4.5 Formation checks

For convoys and escorts: every tick, every protected entity must be within range of a bot. When the check fails, the column stops (ARCHITECTURE §3.10).

### 4.6 Errand holding and trigger

- An errand is prepared first: bots allocated and in position, claims taken and held until release (§4.1).
- `trigger(errand, step)` goes straight to the squad holding it, with no planning in between; the step's reflex settings (usually `all: off`) apply for the step.
- Latency is measured at the bot's click (WORKED_EXAMPLE_OSC N5: within 1 game tick of the squad receiving the trigger).

### 4.7 Reflex application

On job start, apply the job's `reflexes_off`; on play start, apply the play's settings at squad scope; restore inherited settings when either ends (CONVENTIONS §4.2, §4.4). Pass `ReflexSuppressed` events to the running skill and handle the ones the skill doesn't (pull a starving bot, cover a bot in lava with reflexes off).

### 4.8 Moving claim renewal

For convoys, the escort's squad leader renews the moving claim locally on its heartbeat, so a partition from HQ doesn't drop it (§3.7).

## 5. Failure handling

| failure | response |
|---|---|
| A bot disconnects | Reassign its jobs; for cargo, apply the preemption floor and park if below it |
| Leader process lag | Bots continue their current job; in combat, they hold defensively |
| Squad below minimum size | Report to K5 for merge or reinforcement |

## 6. Scale and performance

~100–200 squads at 2,000 bots. Squads never span nodes (§3.8).

## 7. Version-sensitive parts

None directly; skills handle version differences.

## 8. Tests

- Errand trigger latency measured at the bot's click.
- Formation check stops a column within one tick of a vehicle leaving range.
- Job with `reflexes_off: [combat.defend_self]` restores the bot's inherited setting after it ends.

## 9. Open questions

None at this stage.
