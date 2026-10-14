# K2 Commander API

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §4.1, §4.2, §4.4, §10; COMMANDER_INTERFACE.md §7, §9. Conventions: `CONVENTIONS.md` (reflex control §4).
**Depends on:** K1, F6. **Used by:** K3 (LLM adapter), K4 (human interface), operator tooling (errands).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Command set | Deterministic | A small typed set (§4.1) |
| 4.2 | Validation and permissions | Deterministic | Schema, feasibility and permission checks |
| 4.3 | Audit log | Deterministic | Every command logged with who, what, why and effect |
| 4.4 | Policy mechanism and autonomy levels | Deterministic | Scoped standing orders with three classes |
| 4.5 | Reflex policy | Deterministic | Reflex settings at swarm and operation scope (CONVENTIONS §4) |
| 4.6 | Policy content | Interface · runtime decision · commander | What the swarm may do unasked is risk appetite and strategy, not a calculation |
| 4.7 | Approvals | Deterministic | Tickets answered by approve or reject, with defaults and deadlines |
| 4.8 | Errands | Deterministic | Prepare, hold, trigger, release |
| 4.9 | Halt and recall | Deterministic | Ungated, never budgeted |
| 4.10 | Shared seat between a person and the LLM | Interface · authored input · person | Who may override whom is a governance choice |

## Architecture decisions

### ⚑ AD-K2.1 Imperative commands, or a desired-state manifest?

- **Options:**
  - (a) v0.7's imperative tools (set_goal, allocate, approve and so on).
  - (b) COMMANDER_INTERFACE's hybrid: a manifest the commander patches, a ticket inbox, and a read-only tree to browse.
- **Recommendation:** (b), which COMMANDER_INTERFACE argues fixes the commander's amnesia and thrashing, but run the bake-off (F3 §4.7) first, since it's cheap against a fake HQ.
- **Why it needs Allen:** It decides how both you and the LLM command the swarm, and v0.7 hasn't adopted it.
- **Settled by:** The bake-off, then Allen's call.
- **If decided late:** K1 (reconciling a manifest versus applying commands), K3 and K4 all change shape.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-k21-imperative-commands-or-a-desired-state-manifest).

### ⚑ AD-K2.2 Should a person and the LLM share the commander seat, and on what terms?

- **Options:**
  - (a) One holder at a time, switched explicitly.
  - (b) Shared: each goal is owned by a person or the LLM, and the LLM can only propose changes to goals a person owns.
  - (c) The LLM commands, and a person approves only approval-class actions.
- **Recommendation:** (b), with (c)'s approval classes on top.
- **Why it needs Allen:** It's a governance choice about how much the LLM may do without you.
- **Settled by:** Allen's call.
- **If decided late:** An ownership field and permissions are cheap to add early and awkward to retrofit into the audit history.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-k22-share-the-commander-seat-and-on-what-terms).


---

## 1. Purpose

The commander talks only to HQ, through one typed command set (ARCHITECTURE §4.1). A person uses it through a CLI or console; an LLM uses it as tools; same validation, same permissions, same audit log. K2 is that API plus policies, approvals, errands and the big red buttons.

## 2. Scope

**Owns:** the command set and its schemas; validation and permissions; the audit log; the policy mechanism and autonomy classes, including reflex policy; approvals; errands; halt and recall.

**Does not own:** what policies say (the commander, 4.6); the LLM's presentation (K3); human UIs (K4); executing commands (K1 and below).

## 3. Interfaces

**Read:** `status()`, `inspect(target)`, `query(question)`, `explain(goal_or_operation)`.

**Write:** `set_goal` / `cancel_goal` / `reprioritise`, `quote(goal)`, `annotate` / `remove_annotation`, `errand(...)`, `trigger(errand, step)`, `allocate`, `set_policy(scope, ...)`, `approve` / `reject`, `halt(scope)` / `recall(scope)`.

Targets are goals, operations, regions and selectors, never lists of bots. Per-bot control exists only as a person's debugging tool (§4.1).

## 4. Requirements

### 4.1 Command set

As in §3 (ARCHITECTURE §4.1). Every write returns what HQ will actually do in concrete terms before or as it takes effect (COMMANDER_INTERFACE §7.6, echo before effect); writes above a cost threshold wait for confirmation.

### 4.2 Validation and permissions

- Schema validation, reference resolution (names must exist; no invented ids), feasibility (an allocation that exceeds available bots is rejected with the arithmetic).
- Writes carry the state generation they were based on; stale writes are rejected, and re-sending the same write is harmless (COMMANDER_INTERFACE §7.5).
- Permissions per principal (person, LLM, operator tooling): which commands, which scopes, which approval classes.

### 4.3 Audit log

Every command: principal, time, arguments, the stated reason (required for writes), the echoed effect, and the outcome. Append-only, kept long.

### 4.4 Policy mechanism and autonomy levels

Three classes (§4.2): always allowed (eat, sleep, flee, defend self, recover own items), allowed by policy (defend allies, retaliate, move bots within an operation, spend diamonds, enter the nether, hygiene rules), needs approval (attack a player, arm a machine, break player-made blocks outside a permit area, start a build over N blocks, spend scarce items, create an operation). Policies are scoped (swarm, operation) and can set ticket defaults and deadlines.

The break rule is narrowed from ARCHITECTURE v0.7 §4.2's "break blocks outside claimed territory": read literally, it needs a ticket for every tree felled and every block mined on a fresh server, which stalls the first milestone or floods the digest. Natural blocks can be broken anywhere; only blocks B4 §4.8's ownership rules mark as player-made need a permit area or approval. A permit is the operator's approval to modify blocks in an area, nothing more; it doesn't mean the server allocated the land (CONVENTIONS §3.3a). Permits are granted only by a person, through `grant_permit(area, scope)` or a map annotation; the LLM commander can request one, which arrives as an approval ticket. Adopted in ARCHITECTURE v0.8 §1.2 and §4.2 (spec audit #8; Allen, 2026-10-13).

### 4.5 Reflex policy

- `set_policy(scope, reflexes: {id | group | all: on | off, params?, for?})` at swarm and operation scope (CONVENTIONS §4.2).
- The reflex registry is readable through `query(reflexes)` so selectors can name reflexes and groups.
- Turning survival reflexes off swarm-wide is a costly write: echoed and confirmed, never refused (CONVENTIONS §4.3).
- `inspect` shows the effective reflex state for any scope; the digest says when survival reflexes are off anywhere.

### 4.6 Policy content

> **Interface input (runtime decision).** Who: the commander (a person sets the approval-class boundaries; the LLM can tune "allowed by policy" settings within them).
> Why: what the swarm may do without asking (retaliate or not, spend diamonds, enter the nether, which reflexes run, ticket defaults like "enemy at the wall → hold") is risk appetite and strategy; there's no correct setting, only one that fits the commander's goals and the server.
> Enters the system as: `set_policy` commands, audited.
> Null's side: ships conservative defaults (ARCHITECTURE §4.2 as written), enforces whatever is set, and shows policies with their reasons on every commander wake (COMMANDER_INTERFACE §7.3).

### 4.7 Approvals

Approval-class actions become tickets with options, consequences, a default from policy and a deadline. `approve(plan)` or `reject(plan, reason)` resolves them. The default applies when the deadline passes, except for actions whose default must be "don't" (attacking a player, arming), which never happen by default.

### 4.8 Errands

`errand(steps at markers, claims to hold, priority)`: a bundle of generic jobs (deliver into a container, press, hold presence, visit and count). Prepared first (bots allocated and in position, claims taken and held until release), then run step by step with `trigger(errand, step)`, which goes straight to the holding squad (K6 §4.6). Errands are audited like goals and never contain machine-specific logic (§3.12).

### 4.9 Halt and recall

`halt(scope)` and `recall(scope)` are never gated, never budgeted, and never depend on parsing or patches (COMMANDER_INTERFACE §7.7). They travel on their own bus subject (F6 §4.3).

### 4.10 Shared seat between a person and the LLM

> **Interface input (authored).** Who: person.
> Why: once AD-K2.2 settles the model, the person still sets who owns which goals and what each principal may do.
> Enters the system as: an ownership field per goal (`person` or `llm`) and a permission table per principal, set by the person.
> Null's side: enforces it; the proposed default is that the LLM may only propose changes to goals a person owns, and a person can take over any goal at any time.

## 5. Failure handling

| failure | response |
|---|---|
| Malformed or oversized command | Rejected with the reason |
| Stale write | Rejected with the current generation |
| HQ unreachable | Halt and recall still delivered on their own subject; other writes fail fast |

## 6. Scale and performance

Command volume is low (the commander acts in minutes). Halt latency to every bot under one second wall time (F6 §6).

## 7. Version-sensitive parts

None.

## 8. Tests

- Every command through both the CLI and the LLM tool binding gives the same validation and audit entry.
- Halt during a full load reaches every bot within the target.
- A swarm-wide `survival: off` reflex policy is echoed, needs confirmation, and then shows in the digest.

## 9. Open questions

None beyond AD-K2.1 and AD-K2.2. K2's validation, audit, policy and errand mechanisms carry over whichever way AD-K2.1 goes.
