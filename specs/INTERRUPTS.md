# Interrupts: detect, contain, price, escalate

> **Status: current. Adopted by Allen, 2026-10-14 (AD-K5.3, [DECISIONS.md](DECISIONS.md#ad-k53-interrupts-contain-and-price-or-classify-and-resolve)).** Proposed the same day as "the surprise contract", and renamed on adoption. Written after Allen's constraint that there aren't the tokens to resolve interrupts by trial and error, one kind at a time. It replaces the generic recovery and the per-ticket triage that F4 §4.8 used to have. Known failure classes (F4 §4.7) stay as they are. The specs it touches point here (§8). Nothing here is measured. Where a number is a guess it says so.

## 0. Why "interrupts"

The name follows the computing term, and the parts line up:

| computing | here |
|---|---|
| an interrupt stops whatever is running, whatever caused it | an observation outside the running job's expectations stops the job (§2.1) |
| a fixed handler runs, and it doesn't need to know the cause | containment, the same steps every time (§2.2) |
| the scheduler resumes other work | K5 places the job somewhere else |
| masking holds back low-priority interrupts | interrupts priced inside the quote's limits are only recorded (§2.4); contain-only mode when the token budget is spent (§5) |
| non-maskable interrupts always get through | the hard routes: staff messages, kicks and bans, setbacks, approvals (§3) |

**Interrupted, not preempted.** A job is *preempted* when Null stops it on purpose, for higher-priority work (ARCHITECTURE §1.2). It is *interrupted* when the world does something the job didn't expect. The job outcome is `interrupted(incident)` (CONVENTIONS §3.1).

## 1. Why classifying interrupts doesn't converge

The specs handle a failure by its class (F4 §4.7). A failure with no class gets "release claims, requeue once, then `blocked(unknown_failure)`" and a triage ticket. The fix is a new class with its own recovery rule (F4 §4.8). So every new kind of interrupt costs a diagnosis by an LLM or a person, and an open world has no end of kinds: every trap design, every player's habits, every mechanic we modelled wrong.

The builder shows what that costs. F7 runs 2 to 10 each stopped on one new problem. Each took a session to diagnose and added one named rule ([`logs/builder/online/PROGRESS.md`](logs/builder/online/PROGRESS.md)). At the swarm's scale that loop doesn't finish. Response tables per encounter type, gesture menus and per-trap failure classes would have the same problem, so this design has none of them.

The contract below never needs to understand an interrupt in order to survive it. Understanding is bought only where avoiding the interrupt is expensive. It is bought in batches, ranked by what the interrupts cost, and with a fixed token budget.

## 2. The contract

### 2.1 Detect: outside what the job expected

An **interrupt** is any observation outside the running job's expected envelope:

- damage from a source the job didn't expect (its targets and their threat profiles say what is expected);
- block changes the bot didn't cause and F2 didn't predict (B4 already reports its own changes to F2);
- an entity or player where belief had none, acting towards the bot (approaching, watching, attacking);
- no progress against the job's success test within its budget (CONVENTIONS §3.1).

Known failure classes (F4 §4.7) are matched first and keep their own recovery. Everything else is an interrupt. There is no list of interrupt kinds.

### 2.2 Contain: one response for every interrupt

In order, each step at its own time scale:

1. **Reflexes act** (1 tick): flee lava, put out fire, land on water, defend self only while attacked.
2. **Safe stop and back off.** The verb stops at a safe point. The bot goes back along the path it came by, skipping cells that changed, to its last safe position: `goto(last_safe)`, the term C7 §4.4 already uses.
3. **Quarantine.** An F2 hazard mark covers the cells involved plus a margin, and carries the evidence id and an expiry. Size and expiry are policy, and a mark is renewed if it triggers again. B1 path costs treat it as `keep_out`. A mark caused by a player follows that player's last-seen positions until it expires.
4. **Release and requeue.** The job releases its claims and ends `interrupted(incident)`, and K5 places it somewhere else.
5. **Record.** An evidence bundle goes to F4's incident log: the replay window in F3 §4.5's format (packets, inputs, block changes, entities, damage events), the job, and what was expected against what was observed.

Talking to people is not part of containment. A bot that is noticed steps away, which is the cheapest response and commits to nothing. Chat and gestures are messages sent on the operator's behalf, so they are opt-in through C8 §4.7's message gate.

| special case | containment |
|---|---|
| can't back off (trapped, cornered) | local fallback (ARCHITECTURE §5): hold and defend; `blocked(trapped)`. A rescue is a job and is priced like any other. Often the answer is to accept the loss: a death costs items and time, not an account. |
| cargo job | move to the next park point and hand over to a keeper (ARCHITECTURE §3.14). If that fails, escalate at once, because cargo can't be recreated. |
| builder replay | halt on divergence, as the builder already does (ARCHITECTURE §3.9) |
| at a unique place | contain as usual; its price (2.3) is high by definition |

### 2.3 Price: what does avoiding it cost?

The lowest level that can find an alternative prices the quarantine:

- **K5 first.** Can the operation still meet its quotas, within its budget, with the place quarantined? The price is the difference in bot-hours, finish time and losses.
- **If not, K5 escalates to HQ** (ARCHITECTURE §3.1), and K7 re-plans the program with the quarantine as `keep_out`: other sources, other places. The price is the difference in the quote.

K7 marks the **unique places** in each program, the ones with no alternative: the stronghold, the build site, home base, a convoy route. Their price is known in advance.

### 2.4 Escalate, or only record

An interrupt reaches the commander in only three cases:

- **The re-plan leaves the approved quote's limits** (ARCHITECTURE §3.11): a critical-path slip beyond its threshold, a resource below its minimum, or a step that needs approval.
- **Containment is failing.** The same source (one player handle, one entity) triggers quarantines in N places within a window, so it is following the swarm.
- **Containment cost across the swarm rises above its baseline** (an F4 §4.5 alert), so the interrupt is everywhere.

Everything else is only recorded. Expected losses never escalate (§3.11), and an interrupt whose price stays inside the quote's limits counts as an expected loss.

## 3. Escalation: who, in what order, and who in the end

| level | does | passes it up when | to |
|---|---|---|---|
| bot | reflexes, safe stop, back off, evidence bundle | always reports | squad |
| squad (K6) | quarantine; reassigns the job within its batch | the job can't be placed elsewhere in the batch | operation |
| operation (K5) | re-plans locally; prices | its quotas become infeasible within budget | HQ |
| HQ and K7 | re-plans the program; prices | the re-plan leaves the quote's limits; containment is failing; swarm cost is rising | commander |
| commander (LLM, or a person) | picks one of K7's options, changes a policy, buys evidence, or asks a person | approval classes; anything about people or server rules | person |
| person (the operator, Allen) | approvals, relations, server rules, permits | — | — |

A ticket follows K1 §4.10. It is merged across bots and operations, lists what was already tried (the containment), and gives K7's options with their costs, a default from policy and a deadline. **The default is to stay contained,** so a slow commander costs throughput, never safety.

**Buying evidence** is one of the commander's options: a careful `observe` job at the quarantined place. It spends bot time instead of tokens, and a look on the spot often settles what a long read of logs can't.

*Changed by AD-K2.2 (Allen, 2026-10-15):* the LLM commander is in full control and the person holds halt and recall. Approval classes are replaced by permission checks on low-level skills that a person sets once (K2 §4.4), so nothing below waits on a runtime approval. "Attacking needs a person" now reads: attacking a player first is blocked by the default check unless the person disabled it.

**Hard routes** skip pricing:

| evidence | goes to |
|---|---|
| a message from server staff, a rules change | the operator (C8 §4.9), never the LLM alone |
| a kick or a ban | the operator; F1 stops joining that server |
| an anti-cheat setback or a server position correction | engineering (F4 §4.6): it is our bug |
| any response that would attack a player first or break someone else's build | blocked by the default permission checks (K2 §4.4, AD-K2.2) unless the person disabled them |

**In the end:**
- **Containment** needs nobody. It runs automatically.
- **Runtime decisions** end with the commander. The hard routes and approval classes end with a person.
- **Every fix to data or code** (§4.2, rungs 2–4) ends with Allen, because each one changes something static. Policy fixes (rung 1) are the commander's, within the approval-class limits a person set.

## 4. Fixes

### 4.1 Batch review

On a schedule (daily, or every N run-hours, a guess), with a fixed token budget:

1. **Cluster.** F4 groups incidents by a signature computed mechanically: the verb, the kind of mismatch (damage source type, block types changed, entity type, player present, stall), the place profile, and the bot's last action. Region is not part of the signature, so the same trap design in 40 places is one cluster. (Today F4 §4.8 merges by skill, error, region and time window.)
2. **Rank** the clusters by their total price.
3. **Propose.** The reviewer (an LLM) takes clusters from the top until the budget is spent. For each one it reads a bounded summary: signature, count, cost, and 2–3 evidence bundles. It proposes the cheapest fix on the ladder (4.2) that removes the cost, together with a harness scenario built from one of the bundles.
4. **Approve.** A person approves.

Clusters below the cut wait. Clusters that stop recurring drop out of the log.

### 4.2 The fix ladder

Cheapest rung first. A fix is accepted at a rung only if no cheaper rung removes the cost.

| rung | change | example | written by | approved by | gate |
|---|---|---|---|---|---|
| 0 | none: stays contained | a trap in a random cave | — | — | — |
| 1 | policy | longer quarantine for marks caused by players; a bigger search budget | commander | audit; a person sets the approval-class limits | none |
| 2 | data | a hazard field on a place profile, an interaction row, a corrected `[c]` value, a concealment weight | LLM drafts | person | an F3 scenario from the evidence passes; game facts probed (F5 §4.4) |
| 3 | a failure class with its own recovery | "damage after stepping on a pressure plate among player-made blocks: mark the trigger cell" | engineer | person | F3 scenario plus regression; a framework change under GENERALIZATION §1 |
| 4 | verb or skill code | a physics mismatch, a path into lava | engineer | person | F3 scenario, version matrix (F3 §4.8), live gate (F3 §4.9) |

Every fix ships with its scenario, which joins the F3 library so the cluster can't come back unnoticed. The scenario is replayed from recorded evidence, so the trial and error happens in the harness. It doesn't happen on a live server, and it doesn't happen in the commander's context.

## 5. Token budgets

- **Commander wakes from interrupts get a budget per period.** K3 §4.6 only caps each wake. As the period's budget is spent, the price threshold for escalating rises. When it is gone, the swarm runs contain-only: nothing escalates except the hard routes, which go to a person and cost no LLM tokens.
- **The batch review gets a budget per period.** It caps how many clusters get a proposal.
- **Engineering time follows the same ranking.** Rungs 3 and 4 are worked in order of cluster cost.

So tokens grow with what the interrupts cost and with the budget Allen sets. They don't grow with the number of kinds of interrupt.

## 6. Worked through

| interrupt | containment | price | escalates? | fix |
|---|---|---|---|---|
| pressure-plate trap in a random cave | reflexes, back off, quarantine, mine elsewhere | low (another cave) | no | rung 0 |
| a player sees a bot mining | step away, quarantine around the player, work elsewhere | low | no | rung 0 |
| the same player turns up at three sites | as above, three times | rising | yes: containment failing | the commander decides: relocate, change posture, or ask a person about relations |
| trap or griefing at home base | contain | high (unique place) | yes | buy evidence, then a rung 1–2 fix; anything against a player needs a person |
| the same trap design in 40 places | 40 cheap quarantines | high in total | no wake; top of the batch review | rung 3 if one rule removes the cost |
| a bot in a pit it can't climb out of | hold; `blocked(trapped)` | the bot's items | no, unless a rescue is worth it | rung 0, or rung 4 if B1 should have pillared out (B1 §4.5) |
| convoy ambushed | park point, hand to a keeper | high (cargo) | yes, at once | the commander decides; attacking needs a person |
| repeated position corrections | — | — | hard route to engineering | rung 4 |

## 7. What it gives up

- **Efficiency:** detours and abandoned spots. The price measures this cost.
- **Social play by default.**
- **It can be steered.** An opponent who sets off quarantines on purpose can push the swarm out of an area. This shows up as containment cost in that region and escalates as "containment failing". It isn't prevented.
- **Over-avoidance on a busy server.** Swarm containment cost rises, and the commander makes one posture decision instead of one decision per incident.
- **False interrupts from our own model gaps.** They cost efficiency until a batch review fixes them, and they make the incident log the main source of bug reports.
- **The first costly interrupt at a unique place** still needs the commander's thinking at that moment. The contract doesn't make that cheap. It makes it rare.

## 8. Where it lives in the specs

Each of these specs carries the part it owns and points here.

| spec | change |
|---|---|
| ARCHITECTURE §1.7, §11 | containment is the generic recovery |
| CONVENTIONS §3.1 | the job outcome `interrupted(incident)`, distinct from `preempted` |
| F4 §4.7, §4.8 | unclassified failures are contained instead of "requeue once, then blocked"; per-signature tickets become the cost-ranked batch review; signatures leave out region; an evidence bundle for every interrupt |
| K5 §4.6, §4.7 | containment and pricing; escalation gated by price |
| K1 §4.10 | escalation triggers: the quote's limits broken, containment failing, swarm containment cost rising; the default answer is "stay contained" |
| K7 | prices a quarantine as a re-plan difference; marks unique places |
| K3 §4.2, §4.6 | interrupt wakes only through price-gated tickets; a budget per period; contain-only mode |
| F2 | quarantine marks with an evidence id and an expiry; marks caused by players follow that player |
| B1 §4.3 | quarantines are `keep_out`; `goto(last_safe)` |
| B6 §5 | a break that can't resume ends `interrupted(incident)` |
| F3 §4.3, §4.5 | scenarios built from evidence bundles; every fix adds one |
| C8 §4.4, §4.7 | the default response to being noticed is to step away; chat and gestures are opt-in through the message gate |
| proposals/GENERALIZATION §3.3, §3.4 | a generic default that runs out of budget ends `interrupted`; the ratchet for interrupts is the batch review |

## 9. How to test it

Run F3 scenarios with interrupts injected that no class covers. F4 §8 already has "a block that vanishes every 100 gt". Add:
- a pressure-plate trap in a cave, and the same trap at a unique place;
- a follower adversary (F3 §4.4);
- a griefer at the base;
- a mechanic deliberately modelled wrong.

Measure deaths beyond what reflexes absorb, commander wakes and tokens per run-hour, and containment cost. Also check whether one batch review with a fixed budget picks the costliest cluster.

**Pass** (not yet run):
- no interrupt at a place with alternatives wakes the commander;
- each interrupt at a unique place wakes it once;
- the review's top cluster is the one the scenarios made costliest.

## 10. Open points

- Default quarantine size and expiry. Whether marks caused by players expire in wall time, since players act in wall time, not game time.
- How "unexpected damage" is judged while threat profiles are still `[c]`. A profile that is wrong turns every fight into an interrupt until it is measured.
- Whether the batch review can run on a smaller model.
- **Decided (Allen, 2026-10-15):** a convoy has right of way through a quarantine, and a quarantine blocks only new claims, never claims already held.
- **Decided (Allen, 2026-10-15):** a trapped bot is rescued only when K7 prices its cargo and gear above the expected losses of the rescue; otherwise it is written off.

## 11. Note for the builder

The builder can't move its site, so its containment is skipping. A stuck step is set aside together with every step that depends on it, the rest of the build is finished, and the stuck clusters from the whole run are reviewed in one batch. One F7 run would then show every stuck class at once instead of one per run. Whether skipping is safe for redstone is open in the builder redesign (`proposals/WORLD_TRANSFORMATION.md`), because a skipped cell can change what later steps power. The builder decisions (AD-C1.1, AD-C1.2, AD-B6.1, AD-F3.1) were decided 2026-10-15 in that design's frame; failure handling is its §5.6.
