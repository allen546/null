# Azalea benchmark, 2026-10-15

> **Measured on one laptop, 2026-10-14 and 2026-10-15.** Evidence for AD-B1.1 (pathfinder) and AD-F2.2 (belief store), both decided by Allen on 2026-10-15 from these results. Code and raw data: `~/Desktop/null-spikes/azalea-bench` (bench in `bench/`, scripts in `scripts/`, one set of files per run in `data/`). Everything above 100 bots is unmeasured, because on this machine the vanilla server is the limit there (§3).

## 1. Decisions taken from this report

- **AD-B1.1:** reuse Azalea's pathfinder behind B1's interface. Keep the default search pool (3 threads on this machine); don't widen it. Add a shared section cache to the fork first (§4, §6).
- **AD-F2.2:** a separate belief store fed by a packet observer, in the `chan` form: the ECS system only clones the packet's `Arc` and sends it to a worker thread (§5).
- **For F2 §4.9:** change detection must compare decoded block states, not chunk bytes (§5).

## 2. Setup

| part | value |
|---|---|
| machine | Apple M1 Pro, 10 cores (8 P, 2 E), 16 GB. Server and bots on the same machine. |
| server | vanilla 1.21.11 jar, 3–4 GB heap, G1, view distance 8, simulation distance 6, offline mode, mobs off, time and weather frozen. A fresh copy of a pre-generated world for every run. |
| client | `azbench`, built on Null's Azalea fork (1.21.11 branch plus 38 cherry-picks), Rust nightly-2026-10-12. One `SwarmBuilder` with all N accounts, so one ECS; joins 40 ms apart. |
| modes | `idle` (stand still); `path` (continuous A* goals); `walk` (movement, with an observer variant) |
| run shape | 15 s settle, 30 s join burst, 45 s steady window. Figures below are from the steady window. |
| repeats | 3 per configuration |

**Validity.** A run is valid only if the server log has no "Can't keep up" warning and no bot timed out. On localhost a timeout means a process stopped answering keepalives for 15 s, so any run with one has lost bots mid-window and its client figures look better than they are. Invalid runs are left out of every table.

## 3. Where the limit is on one laptop

- **The vanilla server's tick thread saturates first.** During joins one server thread ran at 96–100% of a core, and at 300 bots the server fell 27 s behind and dropped every bot whose keepalive was due. GC is not the cause: 0.7 s in total, no full GC.
- **Every 200-bot run was invalid**, idle runs included (timeouts 0, 13 and 6 in the three idle repeats; 27 to 198 in the others).
- **More pool threads also drop bots on the client side.** At 100 bots, 8 threads dropped 31 to 99 bots per run while the server never fell behind (§4).
- Memory capacity and memory bandwidth are not limits: 83% of memory free, about 158 MB of swap, and the bench's estimated memory traffic is under 1% of the M1 Pro's bandwidth.

## 4. Pathfinding (AD-B1.1)

Steady window, mean of valid repeats. CPU in cores; tick in ms; rates per second.

| config | valid | client CPU | ECS tick p50 | p99 | packet reads | A* CPU | section build CPU | searches done |
|---|---|---|---|---|---|---|---|---|
| idle, 10 | 3/3 | 0.12 | 0.8 | 5.1 | 99 | – | – | – |
| idle, 50 | 3/3 | 0.15 | 1.1 | 3.9 | 100 | – | – | – |
| idle, 100 | 3/3 | 0.18 | 1.4 | 2.0 | 98 | – | – | – |
| path 3 threads, 10 | 3/3 | 0.69 | 2.7 | 7.3 | 98 | 0.47 | 0.12 | 9.1 |
| path 8 threads, 10 | 3/3 | 0.36 | 6.0 | 10.4 | 90 | 0.15 | 0.09 | 13.8 |
| path 3 threads, 50 | 3/3 | 2.27 | 8.4 | 15.1 | 81 | 1.48 | 0.40 | 28.0 |
| path 5 threads, 50 | 3/3 | 3.02 | 14.2 | 22.3 | 69 | 2.13 | 0.42 | 7.6 (suspect) |
| path 8 threads, 50 | 3/3 | 3.66 | 24.4 | 44.0 | 47 | 2.83 | 0.42 | 23.8 |
| path 3 threads, 100 | 3/3 | 3.58 | 15.2 | 33.4 | 59 | 2.21 | 0.72 | 22.2 |
| path 5 threads, 100 | 2/3 | 5.21 | 26.0 | 56.6 | 37 | 3.90 | 0.73 | 23.9 |
| path 8 threads, 100 | 0/3 | – | – | – | – | – | – | – |

Findings:

- **Completed searches level off at about 22–28 per second whatever the pool size.** More threads only use more CPU, slow the ECS tick (8 → 14 → 24 ms median at 50 bots) and cut packet reads (81 → 69 → 47 per second). With fewer reads, keepalives wait behind chunk data and the server drops bots. So the earlier lean, "widen the pool", was wrong.
- **Idle bots are almost free:** 0.18 cores for 100 bots. A linear extrapolation gives about 2 cores for 1,000 idle bots (not measured).
- **Continuous pathing costs about 0.036 cores per bot** at 100 bots with 3 threads. This is a worst case; real bots search now and then.
- **A\* on the main thread** (searches short enough to run at once, about one per second at 100 bots): median 1–10 ms, p99 13–22 ms, maximum 22 ms. A 20 ms search inside a 50 ms tick is a real risk, and the place to look if ticks run long.
- **99% of section building is repeated work** (`uniq` runs: 524,781 section builds for 3,438 unique sections at 50 bots; 1,414,437 for 5,461 at 200 bots). Each search builds its own passability bitsets for sections other searches already built. Building takes 0.4 cores at 50 bots and 0.7 at 100, about a third of A* CPU. Copying the chunks is cheap (1.5–3 s of CPU per 45 s window). A shared section cache, invalidated on block updates, removes most of the building. The worlds here were static, so in real play the duplicate share will be lower, but most terrain doesn't change.
- **Suspect value:** 5 threads at 50 bots shows 7.6 searches per second against 24–28 for 3 and 8 threads. Check before relying on it.

## 5. Packet observer (AD-F2.2)

Walking bots, 100 bots, steady window, mean of valid repeats (the `arc` row has 2 of 3 valid; all others 3 of 3).

| variant | what it does | observer time on the ECS thread (ms per s) | client CPU | ECS tick p50 |
|---|---|---|---|---|
| off | no observer | 0 | 0.31 | 2.35 |
| ref | borrow the packet, count kinds, no store | 0.55 | 0.31 | 2.33 |
| arc | keep the packet's own `Arc` chunk bytes on first sight | 1.66 | 0.31 | 2.26 |
| clone | clone the whole packet, then as `arc` | 2.30 | 0.32 | 2.29 |
| copy | deep-copy chunk bytes on first sight | 1.95 | 0.32 | 2.31 |
| decode | as `arc`, plus decode first-seen chunks into palettes | 2.55 | 0.33 | 2.35 |
| chan | the ECS clones the packet's `Arc` and sends it to a worker thread | **1.24** | 0.32 | 2.25 |

Findings:

- **The observer costs almost nothing.** The most expensive variant (`decode`) uses 2.55 ms per second, about 0.25% of a core, and no variant changes the tick. The same holds at 10 and 50 bots.
- **`chan` has the smallest cost on the tick** among the variants that store anything, because decoding and storing happen off the ECS thread.
- **Deduplication is essential.** At 100 bots the store saw about 2,700 new chunks and 68,000 repeats: 96% of chunk packets show a chunk another bot already sent. At 50 bots: about 2,300 new and 33,500 repeats.
- **Byte comparison is not change detection.** In a world where nothing changes (frozen time, no mobs, no mining), about 3% of repeated chunks arrived with different bytes (2,200 at 100 bots). F2 §4.9 must compare decoded block states. The cause of the byte differences is not checked; light data or heightmaps are guesses.

## 6. Not explained yet

- **The ECS tick waits.** At 50 bots with 8 threads the median tick is 24 ms while the measured ECS thread uses about 3 ms of CPU per tick. Candidates: Bevy systems on other threads (the bench measures one thread), lock waits on the chunk storage the searches read, or scheduling. The bench has no lock-wait timing.
- **Why completed searches level off** at 22–28 per second. Not pool CPU. Candidates: the rate at which the bench issues goals, or the 5 s search timeout.
- **The byte differences** in §5.
- **Anything above 100 bots.** It needs the server on another machine. Plan: borrowed M5 machines, or hourly VPSes as a backup.

## 7. Next measurements

1. Server on a second machine; ladder 100–500 bots with 3 and 5 threads.
2. The shared section cache in the fork, then the same ladder, compared with this report.
3. Lock-wait and per-thread timing in the bench, to explain §6's tick waits.
4. Re-run 5 threads at 50 bots to check the suspect value.
5. Decode and compare the 3% "changed" repeats to find what differs.

## 8. Run log

- 2026-10-14 evening: repeat 1 of the ladder to 200 bots on a loaded machine (load average about 12). The machine rebooted during the 300-bot run.
- 2026-10-15 18:48: restart. 18:48–19:18: repeat 2 and 300-bot runs. At 300 bots the server stalls in every run.
- 19:18: new plan: cap at 200, add a 5-thread pool, `uniq` after the ladder. One broken run (`path-p8-n50-r1`, zero searches) moved to `data/invalid/` and repeated.
- 19:59 ladder done; 20:03 `uniq` done; 22:36 observer variants done (78 runs).
