# F6 Bus and deployment

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §2, §3.1, §3.8, §9, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** nothing inside Null. **Used by:** every process above the squad.

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | NATS topology | Deterministic | Cluster on VPSes, leaf nodes on home machines, as decided |
| 4.2 | Subjects and message schemas | Deterministic | Versioned, serialisable types shared by all processes |
| 4.3 | Delivery semantics | Deterministic | Each message class gets a fixed guarantee |
| 4.4 | Service discovery and health | Deterministic | Registrations with wall-time heartbeats |
| 4.5 | Deployment and upgrades | Deterministic | One binary per Minecraft version, rolling restarts |
| 4.6 | Collapsed deployment (small swarms) | Deterministic | Same types in one process for ~20 bots |
| 4.7 | Partitions | Deterministic | Leases expire; nothing stays locked |
| 4.8 | Security | Deterministic | Authenticated connections and per-service permissions |
| 4.9 | Load testing and hierarchy tuning | Measured | Span of control, summary sizes and thresholds are found by measuring |

## Architecture decisions

### ⚑ AD-F6.1 Where does the bus's third quorum member live?

- **Options:**
  - (a) A third small VPS.
  - (b) A home machine. Cheaper, but home links are unreliable, which is why §3.8 keeps the bus core off them.
  - (c) Two VPSes only, accepting that losing one stops durable streams until it's back.
- **Deferred (Allen, 2026-10-15):** not on the early path. The recommendation below stays as the default until this is taken up.
- **Recommendation:** Defer. Run JetStream with one replica while the swarm is on a laptop and at most one VPS (operation logs are local SQLite first, so an outage delays shipping but loses nothing), then add (a), the smallest VPS available, when the swarm first runs across several always-on hosts.
- **Why it needs Allen:** It's hosting cost against availability, on your hardware.
- **Settled by:** Allen's call.
- **If decided late:** Easy to change later; low stakes.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-f61-where-does-the-buss-third-quorum-member-live).


---

## 1. Purpose

F6 is the message bus that connects HQ, operations, swarm nodes and the commander gateway across a few home machines and a couple of VPSes, plus the tooling that deploys and upgrades them.

## 2. Scope

**Owns:** the NATS cluster and leaf nodes; subject naming; message schemas and their versioning; delivery guarantees; service discovery and health; deployment; partition behaviour; inter-service authentication; load testing.

**Does not own:** what flows (each component defines its messages); squad-to-bot traffic, which is in-process ECS events (§3.8).

## 3. Interfaces

Subjects follow `null.<server>.<level>.<id>.<kind>`, for example `null.smp1.op.castle.jobs`, `null.smp1.hq.digest`, `null.smp1.node.n07.health`.

Message classes:

| class | examples | guarantee |
|---|---|---|
| intent | goal, quota, allocation, job batch | at least once, idempotency key |
| summary | operation summary, node health | latest wins, may drop |
| escalation | blocked, needs approval | at least once, deduplicated by key |
| claim request | take, renew, release | request/reply with timeout |
| event log | shard events | durable stream (JetStream) |
| command | halt, recall | at least once, highest priority, never queued behind other traffic |

## 4. Requirements

### 4.1 NATS topology

NATS cluster on the VPSes; leaf nodes on home machines (§3.8). HQ and the bus core run on VPSes. JetStream for durable streams (event logs, audit).

### 4.2 Subjects and message schemas

- Every message type is a versioned schema shared by all processes. A process rejects messages with an unknown major version.
- Schemas are the same across Minecraft versions (§9.1). One HQ per deployment can span several servers (AD-K1.1, decided), so it can talk to nodes on different versions, though normally the deployment runs one version (F5 §4.7).
- Every duration field states its unit (CONVENTIONS §3.2).

### 4.3 Delivery semantics

As in §3. Receivers make intent handling idempotent by key. Halt and recall go on a separate subject that every process subscribes to directly.

### 4.4 Service discovery and health

Services register with role, server, version, host and capacity, and heartbeat in wall time. HQ's standby watches HQ's heartbeat and takes over from the shared log (§3.8).

### 4.5 Deployment and upgrades

- One build per supported Minecraft version for swarm nodes; HQ, operations and the gateway are version independent.
- Rolling upgrades: operations upgrade one at a time; nodes drain (finish safe steps, release jobs) before restarting. Cargo jobs stay on their node until they reach a park point.
- Configuration (server profiles, host capacities) is versioned and deployed with the binaries.

### 4.6 Collapsed deployment (small swarms)

At ~20 bots, HQ, one operation and two squads run in one process (§2), using the same message types through an in-process transport. Moving to the bus is a configuration change, not a code change.

### 4.7 Partitions

- A home machine losing its link keeps its node running; bots finish safe steps and hold (F1 local fallback). Before its leases expire it fences itself: bots disconnect by their own clock, except cargo keepers, which hold (F1 §4.2, §4.7). Leases are reassigned only after lease time plus grace, so an expired holder has already stopped acting (spec audit #5, 2026-10-13).
- **Fencing tokens.** Every claim carries a token that increases each time it is granted. The shard and the claims registry reject writes and actions carrying an older token, so a cut-off holder can't act on a claim it lost.
- Moving claims renew locally through the escort's squad leader, not through HQ, so a partition between a convoy's node and HQ doesn't drop the convoy's claim.

### 4.8 Security

Per-service credentials; permissions by subject (a swarm node can't publish to `hq.*` command subjects). The commander gateway is the only entry point from outside.

### 4.9 Load testing and hierarchy tuning

Testing is per component locally, not a full-scale run (Allen, 2026-10-13). Load tests go up to what one host can run (around 1,000 bots on a laptop by Allen's estimate) and measure CPU and bandwidth per bot first, since those are the limits. The first load test compares N bots in one 64×64 area against N bots spread out, measuring bytes per bot as N grows, since bot-to-bot traffic grows with density (F1 §4.11) (spec audit #2, 2026-10-13). Measure: per-node upward traffic (target low KB/s), escalation rates, digest sizes, HQ decision latency, claim latency. Span of control and summary sizes are tuned from these numbers (§8 "Hierarchy tuning").

## 5. Failure handling

| failure | response |
|---|---|
| VPS hosting the bus core fails | Cluster quorum on remaining VPSes; with only two VPSes, plan for a third small quorum member |
| Home leaf node disconnects | Local fallback; leases expire; reconnect resumes |
| Schema mismatch after a partial upgrade | Reject and report; the upgrade order (operations before nodes) avoids it |

## 6. Scale and performance

Per-node upward traffic in the low KB/s regardless of bot count (§3.8). Halt latency from gateway to every bot: under one second wall time.

## 7. Version-sensitive parts

None in the bus. Swarm node builds are per Minecraft version.

## 8. Tests

- Partition a home machine from the bus (but not from the Minecraft server) for longer than the lease time: it self-fences before expiry, no duplicate-login kicks occur, cargo keepers hold, other operations proceed, it rejoins cleanly.
- Halt under load: every bot stops within the latency target while the bus carries a full job-batch load.

## 9. Open questions

None beyond AD-F6.1.
