# K3 LLM commander adapter

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §4.3, §4.4, §10; COMMANDER_INTERFACE.md. Conventions: `CONVENTIONS.md`.
**Depends on:** K2 (the only API it touches), K1 (digest data). **Used by:** the LLM commander; optionally operation officers (K5 §4.10) and the diplomacy officer (C8 §4.7).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Digest writer | Deterministic | Bounded, stable-prefix rendering of HQ's digest |
| 4.2 | Wake triggers | Deterministic | Escalations, milestones, heartbeats and people |
| 4.3 | Tool bindings | Deterministic | K2's commands as typed tools |
| 4.4 | Memory outside the context | Deterministic | Standing orders, reasons and the decision log, shown every wake |
| 4.5 | Player-text quarantine | Deterministic | A hard rule, enforced mechanically |
| 4.6 | Per-wake budgets | Deterministic | Token and tool-call caps enforced by the gateway |
| 4.7 | Strategy (the commander's own thinking) | Interface · runtime decision · LLM | This is the one place Null puts open-ended thinking by design |
| 4.8 | Commander prompt and standing guidance | Interface · authored input · person | The prompt encodes the operator's values (aggression, risk, priorities) |
| 4.9 | Evaluation | Deterministic | The bake-off's mechanical scores; its rubric is an authored input, F3 §4.7 |

## Architecture decisions

None of its own. Its presentation depends on AD-K2.1, and officers on AD-K5.1.


---

## 1. Purpose

K3 presents the swarm to an LLM commander and turns its decisions into K2 commands. The LLM sees a bounded digest that doesn't grow with bot count, is woken rather than polled, and acts only at goal level (ARCHITECTURE §4.3, §4.4).

## 2. Scope

**Owns:** rendering the digest for the model; wake scheduling; tool bindings; the commander's external memory; quarantine of player text; per-wake budgets; the commander prompt; evaluation harness integration.

**Does not own:** the command semantics (K2); the digest's content (K1); the model itself (any tool-calling model, ARCHITECTURE §9).

## 3. Interfaces

- **In:** digest and deltas from K1; tickets from K1/K2; wake events.
- **Out:** K2 tool calls only. The adapter has no other path into the swarm.
- **Read tools:** `status`, `inspect`, `query`, `explain` (or, if COMMANDER_INTERFACE's hybrid is adopted, `ls`/`read`/`grep` over a pinned snapshot).

## 4. Requirements

### 4.1 Digest writer

- Order the context from slowest-changing to fastest for prompt caching: system prompt and tool schemas, policies, standing orders, then status, inbox and deltas (COMMANDER_INTERFACE §7.8).
- Budget ~8.5k tokens per wake before reads at 2,000 bots (COMMANDER_INTERFACE §9), independent of bot count: a ~2.5k cached fixed prefix plus a ~6k digest, which sits within ARCHITECTURE §4.3's 4–8k digest budget (K1 §6). The bake-off scores against these numbers (spec audit #14, 2026-10-13).
- One pinned snapshot per wake: every read during a wake answers from the same snapshot, with its timestamp (§7.2).
- Ages shown where they matter ("256, last seen 14 h ago").

### 4.2 Wake triggers

An escalation HQ can't resolve by policy; a milestone; a heartbeat every few minutes; a slower strategic heartbeat (30–60 min) that asks "what should we be doing that we aren't?" (COMMANDER_INTERFACE §7.9); a person talking to it.

### 4.3 Tool bindings

K2's commands as tools with the same schemas. Halt and recall are separate tools, never budgeted.

### 4.4 Memory outside the context

The commander's standing orders with their reasons and its last few decisions with outcomes are shown on every wake, not fetched on demand (COMMANDER_INTERFACE §7.3). Notes and plans are kept by the gateway, so the conversation can be cut at any time.

### 4.5 Player-text quarantine

Chat, signs, books, item and player names never appear inline: only inside a delimited, truncated field, preferably as a summary from a separate model call with no tools; no tool call is ever triggered directly by such text; players appear as stable handles (COMMANDER_INTERFACE §7.1). This is a hard requirement.

### 4.6 Per-wake budgets

A token budget for reads and a cap on tool calls per wake, enforced by the gateway and stated in the prompt; when spent, reads return "budget spent, decide now" (COMMANDER_INTERFACE §7.4).

*Proposed (AD-K5.3, not adopted): there is also a budget per period for wakes caused by surprises. As it is spent, the price threshold for escalating rises. When it runs out, the swarm runs contain-only, and only hard routes reach a person (design/SURPRISES.md §5).*

### 4.7 Strategy (the commander's own thinking)

> **Interface input (runtime decision).** Who: LLM (a person can sit above it, approve its proposals, or take over).
> Why: what the swarm should be doing (expand east, prioritise the castle or the war, build a farm now or later, answer a raid) is strategy with no computable answer; this is where ARCHITECTURE puts open-ended thinking on purpose, above deterministic HQ.
> Enters the system as: K2 commands, within policy, every one audited with its reason.
> Null's side: everything below keeps running on policy if the LLM is slow, silent or wrong; tickets have defaults and deadlines; approval-class actions still need a person if policy says so; halt and recall stay available to a person.

### 4.8 Commander prompt and standing guidance

> **Interface input (authored).** Who: person.
> Why: the system prompt carries the operator's values and style: how aggressive to be, what risks to accept, what matters on this server, when to ask a person. There's no optimal prompt; it's iterated against the bake-off and real runs.
> Enters the system as: a versioned prompt and guidance file per deployment, changed only by a person.
> Null's side: tests every prompt version against the F3 scenario set before it goes live, and logs which version made each decision.

### 4.9 Evaluation

Before committing to a presentation, run the bake-off (COMMANDER_INTERFACE §9): the same scripted scenarios against Option A and the hybrid on two or three models. Mechanical scores (tokens per wake, round trips, duplicate or contradictory commands, flip-flops per hour, injection leading to a tool call) come from F3 §4.6; decision quality uses the rubric in F3 §4.7.

## 5. Failure handling

| failure | response |
|---|---|
| Model unavailable | Swarm runs on policy; tickets resolve by defaults; a person is notified |
| Model issues a malformed or stale command | K2 rejects it with the reason; the adapter shows the rejection next wake |
| Thrash (flip-flopping allocations) | K1's damping plus a "changed N× in 30 min" marker in the digest |

## 6. Scale and performance

Cost grows with escalations, not bot count (ARCHITECTURE §4.4). The same digest budget at 20 and 2,000 bots.

## 7. Version-sensitive parts

None.

## 8. Tests

- Against a fake HQ (K3 can start early, ARCHITECTURE §10): the bake-off scenarios, including an injected sign and a 4-hour quiet run.
- Context cut mid-run: the next wake shows standing orders and reasons, and no goal is re-issued.

## 9. Open questions

None beyond AD-K2.1, which decides K3's presentation.
