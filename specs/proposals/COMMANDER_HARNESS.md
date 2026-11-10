# The harness for an LLM that drives Null

> **Status (2026-11-10): proposal, decisions open (§10).** Design level only: the exact code is decided when each part is built. Nothing here is built. It extends K3 (LLM commander adapter, v1 of 2026-10-13) with what was decided or learned since: AD-K2.2 (the LLM in full control, the person holds halt and recall), AD-K5.1 (no officers in v1), AD-K5.2 (the LLM plans the steps that have no model, in validated stretches), the bake-off of 2026-10-14, and the control spine built in S3 and S6. It proposes an answer to AD-K2.1, which is still open.

## 1. What K3 has, and what it lacks

K3 already fixes the hard rules: a bounded digest that does not grow with the bot count, wakes in place of polling, K2 as the only way in, memory kept outside the context, player text in quarantine, budgets per wake. They stand.

Four things are missing or out of date:

1. **One kind of session for two kinds of work.** A wake is short and strategic. Since AD-K5.2 the LLM also writes plan stretches for steps that have no model. That work needs local detail, the plan grammar and several rounds with a validator. It does not fit in a wake's budget, and its detail does not belong in the strategic context.
2. **AD-K2.1 is open.** The bake-off was the tie-breaker and came out a tie.
3. **The test bed was a fake.** The bake-off ran against a Python stand-in for HQ, with runs of 3 to 6 wakes. Its report says the scenarios were too short and too easy to separate anything.
4. **Nothing closes the loop.** K3 shows the last decisions with their outcomes. It does not say how an outcome is tied to the decision that expected it.

## 2. Principle: the harness owns the loop, and the model is a guest

A gateway process builds the context for each session, offers the tools, enforces the budgets and ends the session. No model keeps state between sessions. Everything a later session needs is kept by the gateway and shown again.

So any model that can call tools can drive Null, and the model is a setting per kind of session. Two ways to connect one, with the same contract:

- a tool-calling API, with the gateway running the request loop;
- an agent program run without a screen, which reaches the tools through a tool server. The bake-off ran this way.

## 3. Three kinds of session

| | Wake | Plan session | Review |
|---|---|---|---|
| **Purpose** | Strategy: what the swarm should be doing | Write one plan stretch for a goal that has no model | Fix the causes of interrupts, in cost-ranked batches (AD-K5.3) |
| **Started by** | An escalation, a milestone, a heartbeat, the person | A wake, through one tool; or HQ, when a goal has no method | A person, off line |
| **Context** | The digest (K3 §4.1) | The goal and its brief, the verb set and plan grammar, saved routines that match, the validator's replies | Interrupt logs and run logs |
| **Reads** | Budgeted, from one pinned snapshot | Budgeted, local to the goal's region | Unlimited, off line |
| **Writes** | K2 commands at goal level | Only `submit_plan`, after the validator and the quote | Proposals for a person: data, routines, policy |
| **Size** | Small and frequent | Large and rare | Not part of the live system |

- **A plan session is the commander, not an officer.** AD-K5.1 rules out standing LLMs per operation. A plan session is one task with its own context. It ends when the stretch is accepted or given up, and it holds nothing after that. It keeps the wake's context small: the wake gets back one line, "stretch accepted, quote Q" or "no plan found, because R".
- **Review is named so that nobody builds it into a wake.** It is a development loop with a person in it.

## 4. Reads show the state; writes are verbs with a condition

This is the proposed answer to AD-K2.1.

- **Read side.** The digest shows the standing state on every wake: the goal tree, the policies, the standing orders with their reasons. It is the stable front part of the context, so it caches. This is what the manifest was for: the model does not need to remember what it asked for.
- **Write side.** The model uses K2's verbs (`set_goal`, `cancel_goal`, `reprioritise`, `submit_plan`, and the rest). Each write names the snapshot it was decided on. HQ refuses a write whose target changed after that snapshot, and the refusal shows the new state.
- **No reconciler and no document patches.**

Why this, on grounds other than the bake-off:

- A goal is already a desired state. Since AD-K5.2 a goal is a state and a check, and HQ works towards it. A manifest over that adds a second layer of the same thing.
- AD-K2.2 removed the shared seat. Safe edits by two writers were a main reason for conditional patches. The writers that remain (the person's halt and holds, HQ itself, a plan session beside a wake) are covered by the condition on the snapshot.
- The bake-off showed that standing orders and reasons in the digest did the work credited to the manifest, on both interfaces.
- The manifest's digest was larger, and the gap grows with the number of operations.

## 5. Time

- **Nothing below waits for a session.** Every ticket has a default and a deadline (K3 §4.7). A slow or silent model costs quality, never safety.
- **One wake at a time.** A trigger that arrives during a wake goes into the next digest. Two strategic sessions at once would contradict each other.
- **Plan sessions run beside wakes,** one per goal. Their only write passes the validator and the snapshot condition.
- **A wake has a time limit.** Each write is whole and audited when it is made. At the limit the session ends; what was written stands.
- **An empty heartbeat does not call the model.** The gateway compares the digest with the last one shown. With no change above a set size, there is no session. Cost then follows events, not the clock. The slow strategic heartbeat (K3 §4.2) always runs.

## 6. Closing the loop

- **Every write states what it expects, and by when:** "64 logs in the depot within 20 minutes". This is one short field, checked by form only.
- **The gateway checks it at that time** against the ledger and the digest, mechanically.
- **The next wake shows the result** beside the decision: met, not met, or cannot tell. A miss is a wake trigger of its own.
- **Notes.** The model has one notes page of fixed size, shown on every wake, that only it writes. It is for intent that fits neither a standing order nor a goal. The gateway keeps it; the conversation can still be cut at any time.

## 7. What the model must not be trusted with

K3 §4.5 treats player text as hostile, and that stands. The model itself is also not trusted: permission checks on skills (AD-K2.2), the gate on costly changes, the caps per wake, the audit with a reason on every write, and a person's hold that the commander cannot clear (CONTROL_SPINE_LIVE §4).

## 8. The test bed: the real spine on simulated bots

S3 built HQ, the operation and the squad on simulated bots, with simulated time and a seed. The CLI already drives them with the same command lines that the live node reads. So the harness is tested against the real control code:

- **Long runs.** Hours of game time pass in seconds. A run can have hundreds of wakes, context cuts, quiet hours and a lost node.
- **The same result for the same seed,** so two prompts or two models are compared on the same events.
- **Scripted events** that the toy world cannot produce by itself are injected as escalations: a raid, a shortage, a sign with hostile text.
- **Recorded sessions.** The gateway logs each session's full input. A logged session can be run again with a new prompt or model, and checked against properties: no write that echoes the sign, no goal issued twice, no change reversed within N minutes.

Limits: the simulated world is a grid with trees, so it tests the commander's handling of the spine, not its sense of Minecraft. Scores are the mechanical ones of F3 §4.6. The rubric for decision quality (F3 §4.7) is still Allen's to write.

The fake HQ of the bake-off is retired.

## 9. Build order

1. **K2 as a tool server over the simulated spine.** It reuses the CLI's command path. Done when a session can read status and set a goal in simulation, and the audit shows it.
2. **The gateway with wakes:** digest, pinned snapshot, budgets, the snapshot condition on writes, the session log. Done when a seeded run of one hour of game time finishes with every write audited.
3. **The loop:** expectations, their checks, the notes page, the empty-heartbeat rule. Done when a missed expectation causes a wake in a seeded run.
4. **The long test** that the bake-off report asked for: about 30 operations, many wakes with context cuts, hostile text, harder events. Done when the scores exist for two models.
5. **Plan sessions,** once `submit_plan` and its validator exist (K5, K7).
6. **Live,** on the skeleton server, with the person's halt tested first.

Steps 1 to 4 need no Minecraft server and no fork change. They can run beside `mine`.

## 10. Decisions for Allen

1. **Three kinds of session** (§3), with a plan session as the commander in its own context, not an officer. Recommended: yes.
2. **AD-K2.1:** state in the digest, verbs with a snapshot condition, no manifest and no reconciler (§4). Recommended: yes. This closes the decision.
3. **The real spine on simulated bots is the test bed,** and the fake HQ is retired (§8). Recommended: yes.
4. **One wake at a time, and an empty heartbeat does not call the model** (§5). Recommended: yes.
5. **Every write states an expectation that the gateway checks** (§6). Recommended: yes.
6. **The model is a setting per kind of session,** and the harness stays neutral through a tool server (§2). Recommended: yes.
7. **When to start.** BUILD_PLAN puts the LLM commander at step 5. Steps 1 to 4 of §9 could start now, beside `mine`, since they need no server. Recommended: start step 1 after the swarm stack is pushed.
