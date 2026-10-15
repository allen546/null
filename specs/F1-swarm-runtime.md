# F1 Swarm runtime

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §2, §3.5, §3.8, §3.14, §5, §9, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** F5 (adapter), F6 (bus). **Used by:** every bot skill (B1–B7), K6 squad leader, K5 operations (account reassignment), F4.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Node process model | Deterministic | Azalea swarm mode plus Bevy ECS fixes the shape; nothing to invent |
| 4.2 | Account registry and credentials | Deterministic | Storing, refreshing and leasing accounts is standard secret handling |
| 4.3 | Login, reconnect and backoff | Measured | The logic is fixed; only server throttle limits need measuring |
| 4.4 | Per-bot lifecycle | Deterministic | A closed state machine with known transitions |
| 4.5 | Bot agent runtime (reflex host, skill executor, fallback) | Deterministic | Arbitration and switch resolution follow the reflex contract exactly |
| 4.6 | Placing bots on nodes | Deterministic | Bin packing under stated rules (co-locate by region, squads never span nodes) |
| 4.7 | Moving accounts when a node dies | Deterministic | Lease expiry plus reassignment; cargo resumes from checkpoints |
| 4.8 | Restart reconnect order | Deterministic | Priority order is defined in §3.14 |
| 4.9 | Server deployment profile | Interface · runtime decision · person | Whether the swarm may play on a server, and at what size, is the operator's call (§1.8) |
| 4.10 | Kicks, bans and idle timeouts | Deterministic | Classify the reason, apply the rule, report; the policy on repeat bans is 4.9 |
| 4.11 | Bandwidth budget | Measured | Per-role view distance and placement by host bandwidth; the numbers come from the harness |

## Architecture decisions

### AD-F1.1 What kind of server does the swarm play on? (decided)

Decided by Allen, 2026-10-13: not an architecture concern, because adapting to a server is easy. Per-server limits live in the deployment profile (§4.9). No ⚑. One HQ per deployment, able to span several servers (AD-K1.1, decided by Allen).

---

## 1. Purpose

F1 is the layer that turns accounts into bodies in the world and keeps them there. It runs Azalea swarm processes ("swarm nodes"), logs bots in and out, hosts each bot's agent runtime (reflexes, skill executor, perception, fallback), and moves accounts between nodes when hardware fails. Everything above it sees a bot as an addressable body with a lifecycle state.

## 2. Scope

**Owns:** swarm node processes; the account registry; login, reconnect and backoff; per-bot lifecycle; the bot agent runtime shell (reflex arbitration and switches, the skill executor slot, local perception plumbing, local fallback); bot-to-node placement; account migration; restart reconnect ordering; per-server deployment profiles; kick, ban and idle-timeout handling.

**Does not own:** individual reflexes and skills (B1–B7 register them); squad logic (K6, though it runs inside the node); what a bot should do (K5/K6); the belief layer (F2); the bus (F6); version translation (F5).

## 3. Interfaces

**In:**
- `AssignAccounts{operation, accounts[], region_hint}` from K5: bring these accounts up on a node near the region.
- `ReleaseAccounts{accounts[]}` from K5.
- `SetReflexes{scope, settings, expiry?}` from K2/K5/K6 (CONVENTIONS §4).
- `RunJob{bot, job}` from K6 (in-process ECS event, not a bus message).
- Server restart detection (all connections drop with a shutdown reason, or the server stops answering).

**Out:**
- `BotState{bot, state, node, since}` changes, to K6 (in process) and summarised to K5.
- `JobOutcome{bot, job, outcome, report}` to K6.
- `NodeHealth{node, bots, cpu, ram, tick_lag, bytes_in, bytes_out}` to F4 and HQ, wall time heartbeats. Azalea logs and skips ticks when a GameTick pass falls more than 10 ticks behind; that is the overload signal behind `tick_lag`.
- `Kick{bot, reason_class, raw_reason(quarantined)}` and `Ban{...}` to F4 and K5.
- `ReflexSuppressed{bot, reflex, cause}` to the running skill and K6.

## 4. Requirements

### 4.1 Node process model

- One swarm node is one Azalea swarm process hosting bots and the squad leaders for those bots (ARCHITECTURE §3.8, §9). Azalea's own guide says ~20–40 MB for a lone bot, usually up to ~1 MB extra per bot in a swarm (more when bots are spread apart), and a few hundred bots per swarm by default. Allen estimates one laptop can run around 1,000; treat that as a target to confirm by benchmark, plausible with low view distance and clustered bots. A swarm is one ECS under one lock: every bot's physics, packets and path execution must fit in the 50 ms game tick, so the per-ECS limit is CPU. A host can run several swarm processes (each its own ECS) at the price of not sharing world memory between them. Node size is set per host by CPU and network bandwidth, not by a fixed number (Azalea reality check, 2026-10-13).
- Bots are ECS entities. Skills and reflexes are decision code (AD-F5.2): plain Rust against Null's `BotView` and `BotCommands`, called once per bot per tick by one host system in `mc-adapter` that runs inside GameTick after packet handling and before physics, borrowing Azalea's components without copying. `Skill` and `Reflex` are `Send + Sync`. The host owns the tick counter. Squad leaders run in the same process, so squad-to-bot traffic never crosses the network. The AD-F5.2 spike confirmed reflexes act in the same tick this way (`reference/SPIKE_F5_BOUNDARY.md`).
- A node serves one Minecraft version (one binary per version, §9.1) and one server. A node never mixes servers, so its shared world memory is coherent.
- Per-node capacity is configured by the host (CPU, bandwidth, RAM, roughly in that order of importance) and measured under load (F6 load tests). The node refuses assignments beyond capacity rather than degrading every bot.

### 4.2 Account registry and credentials

- Registry entry: account id, auth method, current lease (node, operation), last login, last kick/ban, skin and name (for opsec, names are not chosen to identify the faction).
- Credentials are stored encrypted on a VPS and leased to a node only while that node hosts the account. Tokens refresh before expiry.
- An account is leased to exactly one node at a time. The lease is a claim on a bot (CONVENTIONS §3.3) with a wall-time heartbeat, so a dead node's accounts free themselves.
- **Self-fencing.** A node that can't renew its leases acts before they expire, by its own clock: its bots finish the current safe step and disconnect cleanly by `lease − margin`. A node cut off from the bus is often still connected to the Minecraft server, so without this two nodes would log the same account in and kick each other in turn (spec audit #5, 2026-10-13).

### 4.3 Login, reconnect and backoff

- Joins are paced to the server profile's join rate (4.9). Reconnects use exponential backoff with jitter, capped, in wall time.
- A failed login is classified: auth failure, server full, throttled, banned, version mismatch, network. Only network and throttled retry automatically.
- Constants to measure per server: safe join rate, reconnect throttle window.
- Also measure the account side (unverified limits): Microsoft/Xbox token refresh rates and the session server's per-IP login limits. A server restart reconnecting hundreds of accounts, or a node's accounts moving at once, may hit them, so 4.8's reconnect order also paces logins per source IP (spec audit #11, 2026-10-13).

### 4.4 Per-bot lifecycle

```
offline → connecting → spawned → idle ⇄ on_job
                         │          │
                         ├→ dead → respawning → idle
                         ├→ kicked → (backoff) → connecting
                         └→ banned (terminal until a person clears it)
```

- Every transition is an event with a timestamp. `on_job` carries the job id.
- On death the runtime reports the death position and inventory snapshot (for recovery by B2/C3), then respawns.
- On disconnect mid-job, the job returns `failed(disconnected)` unless it is `checkpoint: resumable`, in which case it resumes from its checkpoint (ARCHITECTURE §3.8, §3.14).

### 4.5 Bot agent runtime (reflex host, skill executor, fallback)

Per bot, every tick, in this order:

1. **Perception:** turn client events into observation events (throttled, deduplicated) for F2.
2. **Reflex evaluation:** evaluate every registered reflex trigger. For each reflex that fires, resolve its effective switch (CONVENTIONS §4.2). If on, it becomes a candidate; if off, emit `ReflexSuppressed`. Highest priority candidate wins the tick; ties break by registry priority.
3. **Cargo filter:** strip any action whose target is a cargo-tagged entity (CONVENTIONS §4.5). This runs after reflex selection and also on skill actions.
4. **Skill step:** if no reflex took the tick, advance the active skill one tick. A skill that is overridden is told so and resumes next tick.
5. **Outcome check:** if the skill finished, report the outcome.

Switch resolution is cached per bot and recomputed only when a setting at any enclosing scope changes. The effective settings are exposed for `inspect`.

**Local fallback** (ARCHITECTURE §5): if the squad leader link is lost (only possible for detached bots, since squads never span nodes) or the operation is unreachable, the bot finishes its current safe step and then runs its configured hold behaviour (`hold_position_defensive` by default, configurable per operation). Fallback is not a reflex and is not affected by reflex switches, but `hold_position_defensive` only uses combat reflexes that are on.

### 4.6 Placing bots on nodes

- Rules (ARCHITECTURE §3.8): co-locate bots by region, squads never span nodes, an operation's bots on as few nodes as possible, calm restartable work on home machines, war fronts on VPSes.
- Input: operations' headcount requests with region and role; node capacities and reliability class. Output: account → node assignments.
- A greedy placement with periodic rebalancing is sufficient. Moving a bot between nodes costs a logout and login, so rebalancing only moves idle bots and never moves a squad piecemeal.

### 4.7 Moving accounts when a node dies

- Node heartbeat missed for longer than the lease plus a grace period (wall time, longer than the self-fencing margin in 4.2) → its account leases expire → K5 re-requests headcount → F1 places those accounts on surviving nodes.
- **Cargo keepers are not reassigned automatically.** Self-fencing a keeper drops presence, which can lose cargo. A node holding cargo jobs that loses the bus keeps those bots online and holding at their park point rather than fencing them. On the bus side, a dead node and a cut-off one look the same, so before any account is re-placed, a bot already on that server checks the player list: if the account is still online, its lease is marked `held_by_partition` and nobody logs it in until the node returns or a person releases it; if it's offline, re-placing is safe. This is a default, changeable per operation by the commander (spec audit #5, 2026-10-13).
- Jobs come back `failed(disconnected)`; resumable jobs resume from checkpoint on the new node.
- Bots log in where they logged out. If that position was in an operation's region, the operation re-plans around their actual positions.

### 4.8 Restart reconnect order

After a server restart (ARCHITECTURE §3.14):

1. Cargo keepers and escorts (jobs with `checkpoint: resumable` and cargo claims).
2. Bots assigned to `visit_after_restart` presence jobs.
3. Everything else, by operation priority.

Each tier waits until the previous tier is logged in or has failed, subject to the server's join rate. The order list is built from K5's job records before the restart, so it survives every node restarting too.

### 4.9 Server deployment profile

> **Interface input (runtime decision).** Who: person.
> Why: whether bots are allowed on a server, how many, and what to do when a server pushes back are questions about rules and relationships, not optimisation (ARCHITECTURE §1.8).
> Enters the system as: a per-server profile file the operator writes.
> Null's side: never exceeds the profile; stops joining and reports when a server's behaviour contradicts it (repeated kicks, bans).

The profile holds: server address and version, maximum bots, maximum concurrent connections per source address as stated by the server's rules, join rate, idle timeout, known anti-cheat, and the operator's note on whether automated players are permitted. F1 enforces these limits; it has no feature for getting around a server's limits or rules.

### 4.10 Kicks, bans and idle timeouts

- A kick is classified from its reason into: idle, anti-cheat (movement, reach, rate), server full, restart, admin, **superseded** (the same account logged in elsewhere), unknown. A superseded bot never reconnects, and superseded kicks don't count towards the mass-kick halt in §5 (spec audit #5, 2026-10-13). The raw reason string is player- or plugin-authored text and stays quarantined (CONVENTIONS §3.7).
- Anti-cheat kicks are treated as **bugs in our movement or interaction** (the bot behaved in a way vanilla wouldn't) and go to F4 with the bot's last few seconds of inputs.
- Bans are terminal for that account on that server until a person clears them, and are reported immediately.
- Idle timeouts: bots on presence jobs perform a periodic legitimate action (a step, a look) at an interval below the server's idle timeout from the profile.

### 4.11 Bandwidth budget

Bandwidth, not memory, is the main limit when running large swarms (Allen, 2026-10-13). Sharing world memory inside a node doesn't reduce network traffic: the server sends every connection its own chunk data, entity movement and block updates.

- Each node tracks bytes in and out per bot and per host, and reports them in `NodeHealth`. Azalea has no traffic counters, so these are a fork patch (Azalea reality check, 2026-10-13). Placement (4.6) treats a host's bandwidth as a capacity limit alongside CPU.
- **View distance per role.** A bot asks for a view distance in its client settings, and the server sends chunks within the smaller of that and its own view distance (verify the exact rule for 1.21.11 in the harness). Bots that only need their surroundings (miners, builders, keepers) request the minimum; scouts and lookouts request more. Presence is unaffected, because simulation distance is set by the server, not the client.
- **Density costs bandwidth, roughly with the square of bots in one area.** Every bot is a player, so the server sends each bot the movement, rotation, arm swings, equipment and block-break progress of every other player it tracks. View distance doesn't reduce this, because bots in one operation are inside each other's view by design. By the audit's rough estimate (~300 B/s per nearby pair), 100 bots in one area cost a node ~3 MB/s and 300 cost ~27 MB/s. So bots within mutual tracking range is a placement and scheduling constraint alongside CPU and bandwidth: suppliers wait outside the site, idle bots stand still (a stationary player sends almost nothing), big sites are worked in separate sub-areas, and an operation's active headcount in one area is capped by measured density, not by a fixed number (spec audit #2, 2026-10-13).
- **Movement costs bandwidth.** Every new chunk a bot enters is a chunk download. Long-range travel and scouting are the expensive jobs, and the scheduler should prefer the nearest bot for a job partly for that reason.
- The bandwidth per role is measured in the harness and fed to placement and to K7's cost estimates.

## 5. Failure handling

| failure | response |
|---|---|
| Node crash | Leases expire, accounts re-placed (4.7) |
| Node overloaded (tick lag rising) | Refuse new assignments, report; K5/HQ rebalance |
| Auth service outage | Bots already online stay; new logins back off; report |
| Server restart | Restart order (4.8) |
| Mass kick (many bots in seconds, excluding superseded kicks) | Stop all joins to that server, report to the operator; don't reconnect until a person says so |
| Node loses the bus but not the server | Self-fence before leases expire (4.2); cargo keepers hold (4.7) |
| Ban wave | Same as mass kick |

## 6. Scale and performance

- 20 bots: one node, one process with HQ and operations.
- 2,000 bots: a handful of hosts if Allen's ~1,000 per laptop holds, more if bandwidth or the per-ECS tick budget runs out first (Azalea's default guidance is a few hundred per swarm). Per-node upward bus traffic in the low KB/s (§3.8); server-to-bot traffic is the real limit (4.11).
- Reflex evaluation and switch resolution must fit within one tick for the planned node size (a few hundred bots per swarm process by Azalea's guidance, up to ~1,000 per host if Allen's estimate holds), measured, not assumed (spec audit #10, 2026-10-13); switch resolution is cached, so the per-tick cost is the triggers.

## 7. Version-sensitive parts

Login flow, protocol and physics come from Azalea through F5. Kick reason formats vary by server software and version and are parsed through F5.

## 8. Tests

- Node kill with 200 bots: all accounts back online elsewhere within the lease time plus join pacing; no account leased twice.
- Server restart with a convoy, a keeper and 50 miners: keepers and escorts log in first, then the visit job, then miners.
- Reflex switch: set `combat: off` at operation scope, `combat.shield: on` at squad scope; check the effective state on a bot and that a suppressed `defend_self` emits `ReflexSuppressed`.
- Builder replay job with `reflexes_off: all`: no reflex acts during the job; settings restore when it ends.
- Idle timeout: a presence bot survives two timeout windows.

## 9. Open questions

- Can one node hold bots in two dimensions efficiently, or should dimensions split by node?
