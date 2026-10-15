# C3 Logistics

**Spec v1 · 2026-10-13.** Sources: ARCHITECTURE.md v0.7 §1.5, §3.3, §3.7, §3.9, §3.10, §6, §10. Conventions: `CONVENTIONS.md`.
**Depends on:** B1, B3, F2, K5 (ledger lives in the operation), K1 (transfers between operations). **Used by:** every operation, C1 (staging), C2 (spoil), C4 (outputs), K7 (stock for quotes).

## Judgment map

| § | subsection | verdict | why |
|---|---|---|---|
| 4.1 | Stock ledger | Deterministic | Per-operation records with locations, counts and ages |
| 4.2 | Reservations | Deterministic | Soft and hard reservations by priority and expiry |
| 4.3 | Supply jobs from a bill of materials | Deterministic | Expand shortfalls into fetch, craft and gather jobs |
| 4.4 | Carrier routing and batching | Measured | A vehicle-routing heuristic; good enough beats optimal, and quality is measured |
| 4.5 | Staging and delivery to stands | Deterministic | Deliver per step to the stand the plan names |
| 4.6 | Storage layout and sorting | Deterministic | Ad-hoc chest grids follow fixed rules; built storage systems are content packs |
| 4.7 | Transfer orders between operations | Deterministic | HQ orders, carriers execute, both ledgers update |
| 4.8 | Death drop recovery | Deterministic | Race the despawn timer with the nearest free bot |
| 4.9 | Reconciliation and loss | Deterministic | Compare ledger with observations; record shrinkage |

## Architecture decisions

### ⚑ AD-C3.1 Central depots, or storage per operation?

- **Options:**
  - (a) v0.7: each operation keeps its own stock, and HQ moves goods with transfer orders.
  - (b) Standing depot operations (one per base or hub) hold most stock; operations keep only working stock.
  - (c) (a) for small swarms, switching to (b) once several operations share a base.
- **Deferred (Allen, 2026-10-15):** not on the early path. The recommendation below stays as the default until this is taken up.
- **Recommendation:** (c).
- **Why it needs Allen:** It trades walking distance against fragmentation and decides where carriers spend their time, which is most of the swarm's effort at build scale.
- **Settled by:** Allen's call; a harness comparison of bot-hours per delivered stack can inform it.
- **If decided late:** Storage gets built in the world, which is expensive to move.
- **Both sides:** [DECISIONS.md](DECISIONS.md#ad-c31-central-depots-or-storage-per-operation).


---

## 1. Purpose

The swarm is an economy (ARCHITECTURE §1.5): every goal becomes a supply chain. For big builds, logistics throughput, not placement speed, sets the rate (§6). C3 tracks stock, reserves it, moves it, stages it for builders, and recovers it when bots die.

## 2. Scope

**Owns:** the item ledger (per operation); reservations; turning bills of materials into supply jobs; carrier routing; staging; ad-hoc storage and sorting; executing transfer orders; death drop recovery; reconciliation.

**Does not own:** the production graph and source choice (K7); facilities and their rates (C4); entities (C9); HQ's aggregate view (K1, fed by C3's summaries).

## 3. Interfaces

- `need(items, by_gt, deliver_to: stand | chest, priority)` from K5 or C1 plans.
- `reserve(items, for: job, kind: soft | hard, expiry)` / `release`.
- `transfer(from_op, to_op, items)` from K1.
- Summaries up: stock by item (top and short), reservations, carrier utilisation, shortfalls with ETA.
- Jobs down: `fetch`, `deliver`, `haul`, `sort`, `recover_drops`.

## 4. Requirements

### 4.1 Stock ledger

- Per operation (ARCHITECTURE §3.7). Entries: item, count, location (bot inventory, chest at pos, shulker in chest, ender chest of bot X), last verified time and by whom.
- Counts carry their age; a chest's count is only as fresh as its last opening (F2 §4.4).

### 4.2 Reservations

- Soft: stock earmarked for a pending job; others may take it at higher priority.
- Hard: stock in a carrier's inventory for a delivery, or claimed via a container claim.
- Every reservation expires (CONVENTIONS §3.3). Priority comes from the job.

### 4.3 Supply jobs from a bill of materials

For each step's needs: use local stock, then request transfer from HQ, then (if K7 assigned this operation a production quota) craft or gather. Shortfalls report with their source and ETA.

### 4.4 Carrier routing and batching

- Carriers batch deliveries by route and inventory capacity (36 slots, shulkers multiply).
- A heuristic vehicle-routing solver is sufficient; quality (bot-hours per delivered stack) is measured and tuned.
- Long hauls use roads, rails, nether routes and the transit operation (C9 for routes).

### 4.5 Staging and delivery to stands

- Builder steps name their stand and items (C1 §4.2). Materials are staged in chests near the site before the step, then handed to the builder bot.
- For concealed sites, carriers stay outside the concealment radius until the last leg (WORKED_EXAMPLE_OSC §3).

### 4.6 Storage layout and sorting

- Ad-hoc storage: chest grids by item category near each operation's base, labelled in our own data (not signs, which other players read).
- Large automated storage (sorters, shulker loaders) is a facility built from a content pack and run by C4.

### 4.7 Transfer orders between operations

HQ issues `transfer`; the sending operation's carriers deliver to the receiver's staging; both ledgers update on handover, receiver first (the same new-owner-first rule as entity handover).

### 4.8 Death drop recovery

On a death (B2 §4.7), dispatch the nearest free bot within the despawn window (6,000 gt if the chunk stays loaded). Items recovered go back into the ledger; unrecovered items are recorded as lost.

### 4.9 Reconciliation and loss

- When a container is reopened and differs from the ledger, record the difference as shrinkage with location and time window, and pass unexplained losses to C8 (possible theft).
- Periodic audits of high-value stock (diamonds, netherite, shulkers) by visiting.

## 5. Failure handling

| failure | response |
|---|---|
| Chest looted | Shrinkage record; reservations broken → jobs `blocked(container_changed)`; C8 notified |
| Carrier dies with cargo | Recovery job; delivery re-planned from other stock |
| Shortfall can't be met locally | Escalate to HQ as a merged shortage ("short 3.1k deepslate tiles") with remedies tried |

## 6. Scale and performance

At 2,000 bots most are suppliers (ARCHITECTURE §6). Ledgers are per operation; HQ sees aggregates. Carrier routing runs per operation every few seconds on batches.

## 7. Version-sensitive parts

Stack sizes, shulker and bundle behaviour, item despawn, via F5.

## 8. Tests

- Two operations need the same 32 iron; reservation priority decides, the loser reports a shortfall.
- Harness empties a staging chest before a build step: the step is blocked, not attempted.
- Death with a full inventory 200 blocks away: items recovered before despawn.

## 9. Open questions

- Ender chests give each account a private, portable 27 slots. Worth using as per-bot buffers, or too much state to track? Measure in the harness.
