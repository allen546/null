# Problem statement: bots in one process read each other's world

Written 2026-11-04. This document states the problem and what a fix must do. The chosen design and the measurements are in [LOAD_CONTRACT](LOAD_CONTRACT.md), "Swarm world design". The decision record is [AD-F5.6](../DECISIONS.md#ad-f56-how-do-the-bots-of-one-process-hold-the-world).

## Summary

Many Null bots run in one process and share one Azalea `World`. A vanilla client has a world of its own, filled only from its own packets. So a Null bot can read a block or an entity that its own connection has not told it about. It can then send packets that a vanilla client in the same situation would not send. This has drawn Grim alerts in a live test. It must be fixed before `mine`, which puts bots side by side.

## Background

- **What Null is.** A swarm of Minecraft bots on a public Azalea fork. Null must behave like a vanilla client, with identical Grim alerts (F5 §4.6).
- **Why bots share a world.** Upstream Azalea shares the world between the bots of a swarm to save memory and parsing. The first bot to receive a chunk parses it, and later bots link to it. A shared bot costs about 1 MB on the Pi, measured at 50 and 75 bots.
- **The rule it breaks.** Contract promise L4 in LOAD_CONTRACT: no bot reads another bot's state. Allen made L4 a must-fix before `mine` on 2026-11-03.
- **Why timing matters.** Each bot has its own connection, so one server update reaches bots in different frames. Between those frames, bots should hold different blocks. A shared world cannot show this.

## The problem

| # | Breach | Effect on a bot | State |
|---|---|---|---|
| 1 | A bot's predicted block is written where other bots read it | B reads A's block before the server could tell B, or reads a block that never existed | Fixed by patch 53 (per-bot overlay) |
| 2 | The first bot to receive a block update writes it for all bots (the second leak) | B reads a block ahead of its own packets. About ten read sites that decide packets see this: physics, collision, aim, break and place checks, the crosshair | Open |
| 3 | The first bot to receive a chunk writes it for all, and later bots skip their own packet | B holds data from another bot's packet, which may be newer than its own | Open |
| 4 | One bot applies relative moves to a shared mob or remote player (`UpdatedBy`) | Pushes and picks read the updater's view. These can change a move or an attack packet | Open |
| 5 | Any bot's packet writes the shared entity index | The set of entities that a bot pushes against or hits changes | Open |
| 6 | The chunk-load check reads the shared chunk map (patch 38, `travel.rs`) | A bot falls slowly or fast because another bot holds the chunk | Open |

The full list with file and line is in LOAD_CONTRACT, "Audit (40b6af0)".

## Evidence

- **Harm is proven for breach 1.** On Paper + Grim, A broke the block under B, and B fell while A's break was pending. Grim drew `Simulation` x4 and `GroundSpoof` x1 on B. Separate swarms drew zero alerts. It was one row in the strongest setup, and the control was not a perfect twin ([SHARED_WORLD_TEST](SHARED_WORLD_TEST.md)).
- **Breaches 2 to 6 are shown by reading the code.** No live row has shown harm from them yet. The audit was read by two readers; nothing was run for it.
- **The test suites say little about this.** The companion bot has no skill and stands apart from the main bot, so the pairs pass with these breaches present.

## Why it matters

- A leak is silent. A wrong read raises no error, and it can show as a Grim alert or a wrong decision much later.
- `mine` and building put bots side by side and change many blocks, which makes the leak more likely than it is in today's rows.
- A departure from vanilla is fixed even when it draws no alert (Allen, 2026-11-03).
- L4 is what lets swarm load reduce to "a client stall on all bots at once" in the load contract. The everyday testing method depends on it.

## What a fix must do

| # | Requirement |
|---|---|
| R1 | A bot sends the same packets alone or in a swarm, on the same timeline of its own inbound packets |
| R2 | Everything a bot reads comes from its own packets and its own predictions |
| R3 | A forgotten read site cannot leak unnoticed. The compiler or a test must catch it |
| R4 | The fix covers entities as well as blocks, or names how they are covered |
| R5 | Memory per bot stays near today's in the case where bots overlap. The planning figure is about 1 MB of extra memory per bot and about 1,000 bots on one laptop (Allen's estimate, DECISIONS) |
| R6 | The added CPU is measured and affordable. Allen has set no numeric limit |
| R7 | Grim alerts stay at what a vanilla client would draw, and both test servers pass |

## Constraints

- Raw game capability goes in the Azalea fork as normal Azalea APIs. Patches stay small and mostly additive (CLAUDE.md, "The Azalea fork").
- `null-bot-api` and the crates above it do not change for this problem.
- The one-at-a-time pair stays the gate. Each live run runs once.
- A fix must not trade safety for completion.

## Out of scope

- **How many bots a process can carry.** The contract covers what a bot sends, not speed. A slow bot stalls the other bots of its process; that is a capacity question for the bench.
- **The `Timed out` kicks under overload.** They are tracked in OPEN §4. They could change the design only if the bot's side turns out to hold the reply.
- **Other contract breaches.** L2 packet order inside a frame has its own rows in the audit.

## How we know it is fixed

1. A contract test in the fork runs one fixed timeline for a bot alone and in a swarm, and the packets sent are identical. A negative control on today's code fails it.
2. The SW1 row from SHARED_WORLD_TEST draws zero alerts on Paper + Grim.
3. A full pair passes on both servers, because code that handled the swarm case stops running.
4. Memory and CPU are measured on the Pi and recorded against R5 and R6.

## Unknowns

- Why the Pi kicks bots at 75 bots in own mode and not in shared mode (OPEN §4).
- How CPU grows beyond 75 bots.
- Whether Paper keeps the same update order per section as vanilla.
