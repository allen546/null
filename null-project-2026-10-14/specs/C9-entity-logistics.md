# C9 Entity logistics

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §3.7 (moving claims), §3.10, §3.14, §10; BREEZE_PIPELINE_FAILURES.md; WORKED_EXAMPLE_OSC.md §3; REVIEW_RESPONSE.md. Conventions: `CONVENTIONS.md`.
**Depends on:** B7, B1, C7 (escort), K5, K6, F1 (restart order), F2. **Used by:** K7 (entity acquisitions in production graphs), C4 (installed mobs), C1 (entity phase handovers).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | UUID entity ledger | Deterministic | Server UUIDs are identical on every node; entries have fixed fields |
| 4.2 | Cargo tagging | Deterministic | A tag that drives B5's targeting filter |
| 4.3 | Persistence tracking | Deterministic | Each entry names how it stays persistent, checked against the mechanics table |
| 4.4 | Containment and holding cells | Deterministic | Uses pack-provided, harness-tested containments only (designs are authored inputs, B7 §4.8) |
| 4.5 | Capture at sources | Measured | Spawner waves, cooldowns and tracking are measured in stage S0 |
| 4.6 | Convoys | Deterministic | Moving claim, formation check, escort, capacity planning |
| 4.7 | Portal routing | Deterministic | Probe first, cross in groups that arrive to a waiting bot |
| 4.8 | Park points and preemption floor | Deterministic | Rules from §3.14 |
| 4.9 | Handover | Deterministic | One step by UUID, new owner first |
| 4.10 | Checkpoints and restarts | Deterministic | Resume from saved state; keepers reconnect first |
| 4.11 | Losses and top-ups | Deterministic | Expected losses move the ETA; reserves sized from measured loss rates |
| 4.12 | Convoy opsec | Deterministic | A fixed rule: followed → halt at a park point until clear |

## Architecture decisions

### ⚑ AD-C9.1 Can one convoy's escort span two swarm nodes?

- **Options:**
  - (a) Never. A convoy is capped at what one node can escort, and long trains split into several convoys.
  - (b) Allowed, with the operation coordinating escorts on two nodes and cargo tracked by UUID across them.
- **Recommendation:** (a). It keeps the per-tick formation check in one process, which is what protects the cargo, and 256 breezes already move in batches of 32.
- **Why it needs Allen:** It trades convoy size against complexity, and it's an exception to the rule that squads never span nodes.
- **Settled by:** Allen's call.
- **If decided late:** Low cost if (a) is chosen first.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-c91-can-one-convoys-escort-span-two-nodes).


---

## 1. Purpose

Entities can't go in chests; they have AI and physics, they despawn, and they must be lured, caught, contained and moved, often a long way (ARCHITECTURE §3.10). C9 is the swarm's machinery for that: a ledger by UUID, cargo protection, persistence, convoys with moving claims, park points, handovers, and recovery after restarts.

## 2. Scope

**Owns:** the entity ledger; cargo tags; persistence tracking; holding cells; capture operations' job templates; convoys and their moving claims; portal routing for cargo; park points and the preemption floor; handovers; checkpoints and restart resumption; loss accounting; the standing transit operation's routes.

**Does not own:** per-bot mob skills (B7); containment designs (C11 packs); the builder's insertion procedures (C1 macros from packs); approval of arming (K2).

## 3. Interfaces

- Ledger API: `register(uuid, type, holder, owner_op, persistence, loader)`, `seen(uuid, pos, at)`, `handover(uuids, from_op, to_op)`, `lost(uuid, cause)`, `count(filter)` → count with oldest and newest age.
- Convoy job: `Convoy{cargo: uuids, route, park_points, escort_floor, vehicles, moving_claim}`.
- Capture job: `Capture{source, target_count, containment, staging, batch_size}`.
- Summaries up: per program, counts with ages, positions, ETA, losses against expected rate.

## 4. Requirements

### 4.1 UUID entity ledger

- Keyed by UUID, which every node sees identically (§3.10, REVIEW_RESPONSE). Entry: type, held where (pen, cell, boat, minecart), owner operation, persistence method, what keeps its chunk loaded, last seen (time and observer), cargo tag.
- Counts are derived from entries and carry their age: "256, last seen 14 h ago" is never shown as "256".

### 4.2 Cargo tagging

Entries tagged `cargo` are never targets for reflexes or combat, even when they attack; bots shield and back off and keep out of their line of sight where they can (B5 §4.6, B7 §4.9). Near cargo, escorts also follow B5 §4.6's cargo-proximity weapon rule, so losses aren't self-inflicted; harness losses are split into self-inflicted and hostile, and only hostile losses size the reserves in §4.11 (spec audit #7, 2026-10-13). Untagging is a ledger operation by the owner.

### 4.3 Persistence tracking

Every held hostile mob is persistent (vehicle, name tag, or the machine's own design, per the mechanics table) or has a bot within range at all times (§3.10). The ledger records which, and a held entity whose method lapses (keeper leaves, vehicle breaks) becomes a high-priority reassignment.

### 4.4 Containment and holding cells

- Containment comes from packs, tagged with the mob types it holds and tested in F3. A capture job with no tested containment returns `blocked(no_containment(type))` (B7 §4.8).
- Holding cells at the destination decouple convoy timing from build timing (BREEZE_PIPELINE_FAILURES P4).

### 4.5 Capture at sources

- Sources: natural spawning, spawners, trial spawners, breeding (§3.10).
- Trial spawners cap throughput (waves, cooldowns, wave size rising with players in range). Rates come from the mechanics table and measured runs; K7 runs several sources in parallel when one is too slow.
- Capture stands are placed outside other spawners' ranges, or those spawners are walled off first.
- Spawner tracking (whether a spawner waits for its mobs before cooling down) decides between staging at the chamber and moving each batch away; it's a mechanics-table value measured in S0.
- Hygiene: no bot with Bad Omen or Trial Omen takes chamber jobs (policy).

### 4.6 Convoys

- A convoy is its own operation with a **moving claim** covering the train plus a margin, spanning two dimensions while crossing portals, renewed by the escort's squad leader (not through HQ), registering right of way with each shard it passes (§3.7).
- Formation: every vehicle within range of a bot, checked every tick by the squad leader; the train stops when the check fails.
- Capacity: a boat carries 2 mobs when pushed or towed with no bot aboard, 1 when a bot rows it, and a minecart 1 (B7 §4.4); batches sized for exposure (small batches often, not one big convoy).
- Rail-laying ahead, repair behind, spare vehicles carried.
- Regular routes through unowned space belong to the standing transit operation.

### 4.7 Portal routing

- Long hauls go through the nether (distance / 8).
- A bot crosses first and confirms where the link comes out (guards against portal hijacks). Cargo then crosses in groups that each arrive with a bot already waiting. The link is re-probed after any gap.
- Both portals are claimed and guarded while cargo uses them.

### 4.8 Park points and preemption floor

- Routes have park points: enclosed holding spots (§3.14).
- A cargo job's minimum crew can't be preempted. Preempting above the floor is normal. Preempting the job itself means moving to the next park point, handing cargo to a keeper, then releasing.

### 4.9 Handover

One step, by UUID: both sides see the same UUIDs, the new owner's claim exists before the old one is released, and a count mismatch is resolved by UUID, not by guessing (§3.10).

### 4.10 Checkpoints and restarts

- Cargo jobs are `checkpoint: resumable`: position, UUIDs, crew and route progress saved.
- After a server restart, keepers and escorts reconnect first (F1 §4.8), and convoys resume from their park state, not re-planned (§3.14).
- After a node failure, the same applies on the new node.

### 4.11 Losses and top-ups

- Losses within the measured rate only move the ETA; only a critical-path slip beyond threshold or a count below minimum escalates (§3.11).
- Reserves at the destination's holding cell are sized from measured losses, so top-ups don't need a new convoy (BREEZE_PIPELINE_FAILURES P7).

### 4.12 Convoy opsec

If a player is seen following the train (sightings at more than one route point, C8), the convoy halts at the next park point and continues only when clear for a set time. The final leg to a concealed site is short and unmarked.

## 5. Failure handling

All from BREEZE_PIPELINE_FAILURES, by stage: escape (recapture or loss), train split across loaded and unloaded chunks (formation check stops it), train split across dimensions (claim allows both; cross with a waiting bot), portal hijack (probe), vehicle failure (spares, repair squad), cargo death (expected loss), keeper loss (immediate reassignment), restart (resume from park state), count mismatch at handover (by UUID).

## 6. Scale and performance

256 breezes in batches of 32 is the design stress case. Ledger operations are cheap; the cost is bots in presence bands, which K5 schedules.

## 7. Version-sensitive parts

Despawn rules, vehicle capacities and portal cooldowns (1.21.2), trial spawner behaviour, leashability, via F5's mechanics table.

## 8. Tests

- WORKED_EXAMPLE_OSC S0 first: a few dozen breezes, one chamber, a nether convoy and a test cell, measuring wave size, cooldown, spawner tracking, persistence in vehicles and loss per km.
- Variants: restart mid-convoy, escort preemption, broken exit portal, a player following the train (BREEZE_PIPELINE_FAILURES §3).
- Handover with three breezes missing: resolved by UUID, losses attributed to the right stage.

## 9. Open questions

None beyond AD-C9.1.
