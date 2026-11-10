# The control spine on real bots: leases, holds and claims for physical work

> **Status (2026-11-10): proposal, decisions open (§9).** Design level only: the exact code is decided when each part is built (Allen, 2026-11-10). Nothing here is built. The code facts were read in `crates/null-squad`, `crates/null-botlink`, `crates/null-node/src/spine.rs` and `crates/null-proto` on branch `burst-2026-11-10`. Open items it answers: BUILD_PLAN S3 "Done 2026-10-24", `plans/S6_BRIEF.md`, and the two questions in `plans/BURST_2026-11-10.md`.

## 1. Why now

`mine` puts several bots side by side under ground. The control spine was proven on simulated bots, where every part shares one clock and a "stop" is instant. Real bots break both assumptions. This proposal states the rules that keep one invariant true on real bots:

**No two bots ever change the same cells at the same time, and no bot changes a cell it holds no valid lease for.**

Already closed by S6 and not repeated here: a real bot honours `Lease`, renews do not block the squad tick, the bot subject helpers moved to `null-bus`.

## 2. How it runs today

| Fact | Where |
|---|---|
| The operation holds a claim per job, with a time to live of 100 ticks. The squad renews it every 20 ticks and passes the new end to the bot as a `Lease` | `null-squad/src/squad.rs`, BUILD_PLAN S3 |
| The bot's side of the protocol (`BotLink`) refuses work with no lease and stops 10 ticks before the lease ends | `null-botlink/src/link.rs` `lease_ok` |
| In the live node, `BotLink` runs on the spine thread, on the spine's clock. The skill runs on the ECS thread. A stop crosses between them as a flag that the ECS reads on its next pass | `null-node/src/spine.rs` (`NodeBot`, `halt_actions`) |
| A lease end is an absolute tick number of the sender | `null-proto` `Lease.expires_at` |
| The first claim check at job start is awaited, up to 2 s | `squad.rs` `assign` |
| Retiring an operation waits 40 ticks for the bots' last outcomes | `squad.rs` `RETIRE_WAIT` |
| A bot-scope halt puts the bot in the squad's `held` set. Nothing takes it out, and the operation is not told | `squad.rs` `read_commands` |

## 3. Leases on real bots: four rules

**Rule 1: the lease is checked where the packet is made.** Today the check and the action are on two threads. If the ECS pass is late, the skill can act once more after the link has decided to stop. If the spine thread is late, nobody stops the skill at all. So the node keeps, per bot, a permit with a deadline, and the adapter checks it before every command that changes the world (break, place, use on a block or entity, container click). After the deadline the command is refused on the client with a reason, as any refusal is. The permit fails closed in both directions: a late spine and a late ECS both end in "no change". Movement needs no permit (§5).

**Rule 2: a lease is a length of time, counted by the holder from the moment it asked.** An absolute tick number only works while both sides share a clock. That is true in one process and false across the bus (BUILD_PLAN step 4). The holder counts from when it sent the request; the grantor counts from when it granted, which is later. So the holder always stops first, with no clock agreement. The permit's deadline is on the machine's steady clock, not the bot's tick count: a bot that drops ticks under load counts too slowly, which is the unsafe direction.

**Rule 3: a claim is released only when no bot can still act under it.** That is when the bot's outcome has arrived, or when the longest lease ever sent for that job has ended. The squad's stop path already follows this (S6b1). Retire does not: it drops the operation after 40 ticks, while a silent bot can hold a lease for up to 100. Retire must wait for the outcome or the lease end. The cost is at most 5 s on a path that is rare.

**Rule 4: a start is a renew.** The squad sends the job at once and asks for the lease in the background, with the same machinery that renews. The bot accepts the job and may travel, but its permit is closed until the lease arrives. If the claim is gone, the squad stops the bot and reports, as it does today. This removes the 2 s wait, keeps "a batch starts in one tick", and needs no second mechanism. It replaces both options in the burst note (one start per tick, or a map of checks in flight).

## 4. Stop, hold and resume

**What a stop means for a real bot.** A stop ends changes, not safety. On a stop the bot aborts a break in progress (a vanilla client can do that at any tick), finishes a hazard step it has begun (§6), and stands. Survival reflexes keep running. The margin before the lease end (10 ticks today) must be longer than the longest step a skill may finish after a stop. Each skill states that length, and a skill with a longer step is rejected at build time, not at run time.

**A hold has an owner.** AD-K2.2 gives the person halt and recall as the kill switch. If the commander could release a person's hold, the switch would be weak. So each hold records who set it (the person or the commander), the reason and the time.

**`resume(scope)`** clears the holds in its scope that the caller may clear: the person clears any hold, the commander only its own. It is audited and needs a reason, like other writes. It is not urgent-class: nothing is unsafe while a bot stands. Resume does not bring jobs back; the operation requeued them when they were reported `Preempted`.

**Holds are visible upward.** The squad already reports lost bots in `Roster`. Held bots go in the same report, as a second state. Then the operation shows "blocked: bots held" and does not escalate `quota_infeasible`, HQ's headcount counts held bots as not available, and the digest shows the hold and its owner to the commander.

## 5. Claims for physical work

A claim today is a set of cells. Physical work needs three things kept apart.

| Need | Rule |
|---|---|
| **Change** | A bot changes only cells in its own claim. The skill already checks this (`gather` claim validation); the permit of rule 1 is the second line |
| **Transit** | Walking needs no claim. A bot may cross any claim on foot. It may not dig or place its way through: the pathfinder's mining and placing moves are off outside the bot's own claim (DECISIONS AD-B1.1 already says B1 turns mining off outside digging jobs) |
| **Effect** | A change can alter cells outside the claim: a fence joins, a torch drops, gravel falls, a fluid flows. Bounded effects are covered by a margin between the work volumes of different bots, set per job kind as data (B4 §4.12 already spaces fishbone branches). Unbounded effects (fluids) are not covered by any margin: the miner seals before it opens (§6), and a failure is an interrupt |

**Shared corridors.** A fishbone's main tunnel is used by all its bots. The operation holds it as a claim that nobody may change, so no bot's job can take it.

**Bodies are checked by sight, not by claim.** Before a break, the bot checks that the cell does not support a player it can see, and that a player is not inside a cell that the break would fill or flood. With own worlds, other bots are ordinary player entities in the bot's world, so one check covers Null's bots and real players. A claim cannot do this: a body is not in the registry, and a person never is.

## 6. Hazard steps in `mine`

B4 §4.13 says the miner handles underground hazards itself, and may place a block to seal a fluid. For the lease rules this means:

- The claim of a mining segment includes a shell one cell thick around the cells to dig, because a seal is a change in that shell.
- "Check, break, seal" is one step for the stop rule in §4: a bot that has opened a cell with a fluid neighbour may finish the seal after a stop. That step sets the minimum margin for `mine`.

## 7. Smaller items

- **`Requeued` written twice for one expiry.** Requeue is keyed by the claim and its expiry, so a second write is a no-op, like the keyed intents the squad already sends.
- **The reason flag fix and the two retire fixes** on branch `s3-open-items` are correct by their tests and independent of this proposal. They can merge.

## 8. Order

1. Rules 3 and 4 and the requeue key: decision crates only, tested in simulation. No live run needed.
2. Holds with owners, `resume`, and holds in `Roster`: decision crates and the CLI grammar; K2 §3 and §4.9 change.
3. Rule 1 (the permit) and the stop rule: `null-node` and `mc-adapter`, with one live row on both servers that ends a lease in the middle of a break.
4. Rule 2 (lease as a length of time): with step 3, since the permit needs it.
5. §5 and §6 with `mine`: the margin data, the corridor claim, the body check and the shell.

## 9. Decisions for Allen

1. **`resume(scope)` and holds with owners.** This changes the K2 command set. Recommended: yes, with the person able to clear any hold and the commander only its own.
2. **Retire waits for the lease end** (up to 5 s) and not 40 ticks. Recommended: yes.
3. **A start is a renew**, and a bot may travel before its lease arrives. Recommended: yes.
4. **The permit at the adapter** as the place where a lease is enforced. Recommended: yes. It adds one refusal reason to the bot API.
5. **The body check before a break** as a rule of the gather core, for all players. Recommended: yes.
