# Unit of execution: shards inside one process

> **Status (2026-11-10): adopted by Allen, not built (§9).** Nothing here is built or measured for this purpose. The code facts were read on fork branch `patch-stack-burst` (`ee7d418`) and in `crates/null-node`. The numbers come from `plans/LOAD_CONTRACT.md` "Own-world sweep results" and are for harness bots with no skills. Every extrapolation is marked as one.

## 1. The question

AD-F5.6 says: one process per machine, all bots in it, sections stored once. It adds that "a machine can run several batches if that loop becomes the limit". This proposal answers what a batch is, how many bots it holds, and what the bots of one batch share.

## 2. How a process runs today

| Fact | Where |
|---|---|
| One loop on one thread runs every bot. Each pass runs the `Update` schedule (at most 60 times a second), then the game ticks that are due, at most 10 | `azalea-client/src/client.rs` `run_schedule_loop` |
| One tick clock serves all bots. All bots tick in the same pass | `tick_clock.rs` `TickClock`, one per loop |
| That thread reads and decodes every bot's socket until it is empty, once per pass | `plugins/connection.rs` `read_packets` |
| Null's host, node and skills are plugins in the same app, so they run inside the same pass | `null-node/src/main.rs` (`HostPlugin`, `NodePlugin`, `null_bot_plugins`) |
| Socket writes run on Bevy's IO pool: 25% of the cores, at least 1, at most 4. On the Pi that is 1 thread | `plugins/task_pool.rs` |
| Path searches run on the async compute pool, at most 4 threads | `azalea/src/pathfinder/mod.rs` |
| The section store is one map behind one lock for the process | `azalea-world/src/chunk/section_store.rs` `STORE` |

One candidate for the stalls is ruled out: a path search does not hold a world lock for its length. It takes the lock once per section it loads (`pathfinder/world.rs` 278).

## 3. What one loop costs

**3.1 Capacity is about one core.** Measured, own worlds, 50 harness bots:

| Machine | Client CPU per 60 s | CPU per bot per tick | Tick median | Tick time per bot |
|---|---|---|---|---|
| Mac (M1 Pro) | 13.5 s | about 225 µs | 4.1 ms | about 82 µs |
| Pi | 43 s | about 717 µs | 23 ms | about 460 µs |

Extrapolated in a straight line, which is an assumption: the tick reaches 50 ms at about 600 bots on the Mac and about 110 on the Pi. A tick longer than 50 ms drops ticks, so every bot runs slower than the server. ARCHITECTURE's target of about 1,000 bots on a laptop needs about 4.5 cores at the Mac figure. One loop cannot reach it, before any skill runs.

**3.2 One slow pass stalls every bot.** A join burst, a large chunk batch or one slow skill step makes the pass long. Then every bot ticks late and catches up together. The load contract accepts this as "a client stall on all bots at once", and each bot stays legal. But the cost grows with the bot count: one bot's work delays N bots.

**3.3 All bots send in the same millisecond.** One clock means one phase. A crowd of vanilla clients has a random phase per client. Each bot is legal alone. Together they load the server in bursts, and the common phase is a mark that a server owner can see.

**3.4 A bot's chunk rate depends on the other bots.** A vanilla client times its own chunk batch from the start packet to the end packet and tells the server how many chunks per tick it wants (`ChunkBatchSizeCalculator`, 7 ms of work per tick). The fork takes both times in systems that run once per pass, with the chunk work of all bots between them (`plugins/chunks.rs`, the chained systems). So a bot reports the loop's speed, not its own. This is on the load contract's allowed list for wall clocks, but the value is coupled across bots. Not checked: what patch 57 changes here, and what values bots report in a join burst.

## 4. Options

| Option | What it is | Verdict |
|---|---|---|
| A | One loop for all bots (today) | Cannot reach the target (§3.1). Fine for small deployments |
| B | Shards: several loops in one process, each on its own thread with its own app and clock. One section store | Recommended |
| C | Several processes on one machine | Works today with no code: start two nodes. Loses section sharing between them, which is the reason AD-F5.6 rejected one process per bot. Keep as the fallback |
| D | One loop, with the hot systems iterating bots in parallel | Rejected. It changes many fork systems, so every upstream rebase pays for it, and it does not help §3.2 or §3.3 |

## 5. The design (option B)

**Shard.** One OS thread, one Bevy app, one tick clock, one single-thread async runtime, K bots. A shard is what a "swarm" is today. A process holds S shards.

**What a process shares.** The section store, the IO pool, the path search pool and the allocator. Nothing else. A bot in shard 1 never reads the app of shard 2, so L4 holds between shards with no new rule.

**What a shard shares.** Its loop and its clock. So §3.2 and §3.3 still apply inside a shard, to K bots and not to all bots. K bounds the cost.

**Squads.** A squad never spans shards. ARCHITECTURE already says a squad never spans nodes, for the same reason: the leader reads its bots in the same tick.

**Placement.** The node gives a bot its shard at login, by squad. A bot changes shard only through a reconnect. A live connection is not moved between apps.

**Phase.** Shard i starts its loop i/S of a tick after shard 0. The shards then send at S different phases. Bots of one shard keep one phase; this limit stays and is stated in F5.

**Failure.** A panic in a shard's loop is caught at the thread. That shard's bots disconnect, the node reports them, and the other shards keep running. An abort still ends the process.

**The store's lock.** One lock is right for one loop. With S loops it becomes a point of contention during chunk loads. Split the map by hash into several locks. This is a small change in `section_store.rs`.

**Executor.** Each shard runs Bevy's single-thread executor. The process gets its parallelism from the shards. This may also explain the open bench item "the ECS tick waits" (median tick 24 ms with 3 ms of CPU): Bevy's parallel executor spends time handing many small systems to other threads. That is a guess, and the spike tests it.

**Process-wide counters.** The loop statistics in `tick_clock.rs` are statics for all clients. They become per shard, or the harness reads them per shard.

## 6. Shard size

K is a measured number per machine class, not a constant.

- **Rule:** K is the largest bot count where the 99th percentile pass stays under 40 ms under the felling load. BENCH_PLAN already uses 40 ms as the knee.
- **S:** at most the cores minus a reserve for the IO pool, the search pool and the operating system.
- **First guess, from §3.1 and not from a bench:** about 100 to 150 on the Mac, about 25 on the Pi.

A small deployment is one shard. It runs exactly as today.

## 7. Build order

1. **Spike: two shards in one process.** Two swarm builders, two threads, 25 bots each on the Mac, against one swarm of 50. Measure client CPU, tick median and 99th percentile, memory, and kicks. Run it with the parallel and the single-thread executor. Done when the table shows whether sharing survives (memory near the one-swarm figure) and whether the tick time per shard falls.
2. **Fork changes the spike finds.** Expected: the store's lock, the counters, and how a swarm gets its runtime. Each is a small patch. Done when the simulation suite passes with two apps in one test process.
3. **Node.** `null-node` takes a shard count, starts S swarms and places bots by squad. Done when the skeleton scenario passes with S = 2.
4. **Bench.** The ladder in BENCH_PLAN gains the shard count as a dimension. It replaces the "3 and 5 threads" dimension, which tuned one loop. Done when K is known for the Mac and the Pi.
5. **Phase offset and the per-shard panic guard.** Done when a test shows two shards send at different phases, and a forced panic in one shard leaves the other running.

Steps 1 and 2 touch the fork and follow the patch gate. None of this blocks `mine`, which runs in one shard.

## 8. What this does not solve

- The `Timed out` kicks on the Pi. A shard of 25 bots on the Pi may never reach the overload that shows them, but the cause stays unknown.
- The chunk rate coupling inside a shard (§3.4). It needs its own check and probably its own patch: time each bot's batch around that bot's chunk work.
- The common phase inside a shard.
- Server capacity. The 2026-10-15 bench found that the server's tick thread fills first on one machine.

## 9. Decisions (Allen, 2026-11-10)

1. Shards (option B) are the meaning of "batch" in AD-F5.6, with option C as the fallback. The purpose is to spread the bots over the CPU cores, with the timing gains of §5 on the way.
2. The spike (§7 step 1) runs after the next pair. It needs the swarm patches, and it must not overlap a Grim run.
3. The common phase inside a shard is accepted for v1, as a stated limit.
