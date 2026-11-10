# After a bot's death: contain always, resume by rule, recover by price

> **Status (2026-11-10): adopted by Allen (§8: yes to all five), not built. This settles AD-B2.1.** Design level only. It gives the answers to the two points AD-B2.1 leaves open: recovery against quarantine, and who decides after a death. The working rule of 2026-10-19 (go back to work, or escalate; one try per cause; log out under spawn camping) stands and is not repeated. Needed for BUILD_PLAN step 2 (survival core), not for `mine`'s first rows.

## 1. The two open points are one question

Both ask who may send a bot back to the place where a bot died. Today three parts can: the skill (`on_death` returns `Keep`, AD-F1.2), the operation (it requeues the job), and logistics (C3 §4.8 sends the nearest free bot for the drops). None of them knows the cause. INTERRUPTS knows the cause and marks the place, but only its quarantine stops anyone.

The answer: a death always makes a mark, and every return to the place is checked against the mark. The skill and logistics propose; the mark decides.

## 2. A death is always reported and contained

At the death tick (AD-F1.2), with no choice by the skill:

1. **Report.** The death goes up with its cause, place, the items carried, and the respawn point. This is a line in the ledger, not an escalation. Escalation stays with the price rules of INTERRUPTS and the working rule.
2. **Classify the cause,** mechanically, from the death message and the attackers the client tracked.
3. **Mark the place** in belief, by class.

| Cause class | Examples | Mark |
|---|---|---|
| Fixed hazard | lava, a fall, the void, drowning, suffocation | The cells involved, as a lasting hazard in belief, with `how` = a death. It ends only when a bot sees that the cells changed |
| Mob | zombie, creeper, skeleton | An area with an expiry. Policy sets the size and time; night and light matter |
| Player | any player kill | The INTERRUPTS quarantine, and C8 attribution starts |
| Unknown | no message, or a cause the table lacks | Treated as the player class until a person or the commander looks |

A harmless death then costs one report and one mark. Nobody is woken for it.

## 3. Resuming the job

The job is suspended at the death tick. It resumes after the respawn only when all three hold:

1. The skill says `Keep`.
2. The mark does not cover the job's work cells or the only way to them. The planner treats the mark as keep-out (INTERRUPTS §2), so this is a path question it can already answer.
3. It is the first death from this cause in this job (the working rule: one try per cause).

If not, the job ends as failed with the cause, the claim is released by the lease rules (CONTROL_SPINE_LIVE §3 rule 3), and the operation requeues it with the mark in place. So the skill decides whether going on is useful, and the mark decides whether it is allowed. A skill cannot walk a bot back into what killed it, and a skill author does not have to get that right.

A bot that a person holds stays held after its respawn.

## 4. Recovering the drops

Recovery is a job like any other. It is made only when it passes four checks, in order:

1. **Are there drops?** Not under keep-inventory (a server setting, so a belief), and not when the cause destroys them (lava, the void).
2. **Can a bot arrive in time?** Dropped items last 5 minutes of time in which the chunk is loaded. Assume the timer runs. If the travel time is longer than what is left, the items are written off at once.
3. **Is it allowed?** Never into an active player quarantine. For the mob and fixed-hazard classes, the recovering bot plans with the mark as a known hazard, not as keep-out, and it must carry the kit the mark's class calls for.
4. **Is it worth it?** The value of the items against the travel time and the risk, priced as INTERRUPTS §2.3 prices any job.

**Who goes.** The nearest free bot that holds the kit. The dead bot has just respawned with nothing, so it goes only after it is re-equipped, and only if it is still the nearest.

**A player kill.** The default is to write the items off and record them as taken by that player, which is evidence for C8. The commander can buy a look first: a careful `observe` job at the edge of the quarantine (INTERRUPTS, "buying evidence"). Recovery follows only if the look shows the killer gone and the items there.

Experience points are ignored in v1.

## 5. Losses have a budget

A goal's quote already states expected losses, and a stretch can say "no deaths". Each death is priced: the items lost, the bot time, the re-equip job. When an operation's losses pass what its quote allows, it escalates, merged, with the causes and the remedies tried. This is the price rule applied to deaths. It replaces nothing in the working rule; it adds the case of many different cheap causes.

## 6. What belief keeps

- A fixed-hazard death teaches the swarm something lasting: this cell has lava behind it, this ledge has a drop. It is stored as a hazard that every later plan sees.
- A mob death teaches something that fades.
- A player death is evidence about a player, kept by C8.

## 7. Order

1. The report, the cause classes and the marks: decision crates and belief, tested in simulation with the simulated bots' deaths.
2. The resume rule in the bot link and the squad. One live row on both servers: a bot dies by a fall inside its job, respawns, and the job ends with the cause because the mark covers the work.
3. The recovery checks in the operation, with C3.
4. The loss budget, when quotes exist (K7).

## 8. Decisions (Allen, 2026-11-10: yes to all five)

1. **A death is always reported and marked,** whatever the skill (§2). Recommended: yes.
2. **A job resumes only when the skill says `Keep`, the mark does not cover the work, and it is the first death from that cause** (§3). Recommended: yes. This settles "the skill, or the interrupt path": both, in that order.
3. **Recovery passes four checks, and never enters an active player quarantine** (§4). Recommended: yes. This settles "recovery against quarantine".
4. **An unknown cause is treated as a player kill** until someone looks. Recommended: yes. It is the safe side, and its cost shows how often the cause table is incomplete.
5. **Losses have a budget in the quote,** and passing it escalates (§5). Recommended: yes.
