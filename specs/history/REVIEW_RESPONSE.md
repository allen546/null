# Response to the OSC reviews

> **Status (2026-10-14): historical.** The author's response to the OSC reviews; its changes are in ARCHITECTURE v0.7 (still present in v0.8) and WORKED_EXAMPLE_OSC rev 2.

Point-by-point response to `REVIEW_OSC_CONTROL_METHOD.md` (rev 2) and `BREEZE_PIPELINE_FAILURES.md`, from the author of `ARCHITECTURE.md` and `WORKED_EXAMPLE_OSC.md`. Changes landed in ARCHITECTURE v0.7 and WORKED_EXAMPLE rev 2.

I took the Skybender's own mechanics as proven, per Allen: 256 breezes in a 1 × 2 cell, persistence and cramming handled by the design, TNT from dupers, and two chunk loaders.

**Summary:** I agree with almost everything. I partly disagree on four points (B4, B5, the transit owner, and the trigger fast path). Nothing was rejected outright.

## Review §1: the Skybender's inputs

| point | verdict | why / what changed |
|---|---|---|
| B1 capture throughput | **Agree** | Trial spawner waves and cooldowns cap capture. The quote now uses spawners × wave size × cooldown, marked claimed, and runs two chambers in parallel. The rates go in the mechanics table (§9.1) |
| B2 spawner tracking | **Agree** | It decides between staging at the chamber and moving each batch away. It's a mechanics-table entry and the first thing S0 measures |
| B3 transport capacity | **Agree** | 2 per boat, batches of 32, and the convoy is its own operation |
| B4 persistence for 256 | **Partly** | In the cell, the proven design handles it, so 256 name tags are *not* added to the bill of materials. In transit and staging it's Null's problem: vehicles, plus a keeper within 32 blocks. The ledger now records how each entity is kept persistent |
| B5 cramming | **Partly** | Solved by the design in the cell. The only Null-side need is that installation follows the pack's safe-insertion procedure, which the descriptor now names |
| B6 loading 256 mobs | **Agree** | Installation is now **arming**: its own step after the build, the pack's procedure, a circuit break left open, and approval |
| B7 concealment cap | **Agree** | My 20-entities-per-chunk cap contradicted the machine. Concealment budgets now come from the pack, and they count bot traffic and dwell time |
| B8 server load | **Agree** | P5 now measures MSPT while idle, not just during the charge |
| §1.2 TNT dupers | **Agree** | Gunpowder, sand and the per-shot TNT load are gone. Only seed TNT is placed, as an arming step. I also removed the wind charge item farm, which the Skybender doesn't need |
| S1 hopper "holds N" | **Agree** | Delivery is now "N items into the input". The firing errand no longer loads ammunition at all |
| S2 "any disc" | **Agree** | The bill of materials carries exact per-container contents |
| S3 dark cavity spawns | **Agree** | Spawn-proofing is a site requirement for the machine's whole life, laid as the cavity is dug. V7 tests it |
| Process: rates trusted blindly | **Agree** | Descriptors say how each input is used (consumed, installed, feeds a facility), and every rate is `[c]` claimed until it's `[m]` measured. The quote shows which is which |

## Review §2: contradictions with v0.6

| point | verdict | why / what changed |
|---|---|---|
| HQ "thin" but planning | **Agree** | Planning moved to a **program service** (K7) that HQ calls |
| Cross-shard claims "rare" | **Agree, different owner** | Moving claims are now a first-class kind: owned by the convoy, spanning two dimensions, renewed by the squad leader. For unowned space I chose a standing **transit operation** for regular routes, rather than HQ owning nether claims, to keep HQ thin |
| No command runs a job | **Agree** | Added `errand(...)` with prepare/hold/trigger |
| Keep-out needs interpretation | **Agree** | Annotations can carry one effect from a closed, generic set: `keep_out`, `defend`, `rally`, `presence_target`. Labels and notes stay uninterpreted. This doesn't put cannon logic in Null |
| Presence read by Null or not | **Agree** | Null reads presence (it must quote and schedule it). Only the firing logic stays outside |
| DSL creep | **Agree** | C11 is a small typed schema, with no scripting |

## Review §3: complexity

| point | verdict | why / what changed |
|---|---|---|
| One operation per step | **Agree** | Operations are cut by place (§3.3). The example went from nine operations to five plus supply quotas |
| Two planners | **Agree** | The program service picks sources and quantities. Operation HTNs only decide how to meet their quotas |
| Presence as constraints | **Agree** | Structured presence (where, band, count, phase, standoff), scheduled as location constraints, with a feasibility check against the server's simulation distance at quote time. It also covers idle timeouts and the fact that bots make mobs spawn |

## Review §4: risks

| point | verdict | why / what changed |
|---|---|---|
| "…" in the bill of materials | **Agree** | The quote expands everything, including the slime survey and farm, quartz and tool wear |
| Concealment vs. presence | **Agree** | Traffic and dwell time count (see B7) |
| TNT near redstone mid-build | **Agree** | Arming steps go last, behind a circuit break closed as the final step |
| Stale belief over the lead time | **Agree** | Program handovers are gates that re-verify their preconditions |
| Cancel path | **Agree** | Defined unwind order and a cancel report |
| Tool wear and spoil | **Agree** | Both are in the quote |

## Review §5: the firing test

| point | verdict | why / what changed |
|---|---|---|
| N1 follow the real load path | **Agree** | No ammunition load in the errand now |
| Breeze survival check | **Agree** | Recount before and after the shot (N2, P3), and V8 for a stale count |
| P3 latency tests Null | **Agree, different fix** | Instead of a special fast path through the hierarchy, the errand is **prepared** first (bots in place, claims held). The trigger goes straight to the squad holding it, and latency is measured at the bot's click (N5) |
| Steps not atomic | **Agree** | The errand holds its claims from prepare to release |
| Game time vs. wall time | **Agree, strongly** | It's a framework rule now (§3.14): game durations in ticks, network in wall time, with the unit stated on every contract field. V2 checks it |
| Rescale S2 | **Agree** | 280 breezes, by UUID |
| Soak is too expensive | **Agree** | Soak is rare. **S0, the small breeze pipeline, runs first** |
| Hostile spawn variant | **Agree** | Added as V7 |

## BREEZE_PIPELINE_FAILURES §3: framework asks

| ask | verdict | where |
|---|---|---|
| 1 Cargo entities never targeted | **Agree** | §3.10 |
| 2 UUID ledger, one-step handover | **Agree** | §3.10. Server UUIDs are the same on every node, so this also fixes the cross-node case (§2.4) |
| 3 Moving claims across dimensions, renewed locally | **Agree** | §3.7 |
| 4 Park points, preemption floor | **Agree** | §3.14 |
| 5 Restart protocol, resumable cargo jobs | **Agree** | §3.8 and §3.14. v0.6's "re-plan on disconnect" was wrong for cargo |
| 6 Arming as a policy class | **Agree** | §3.12, §4.2 |
| 7 Staleness on the ledger | **Agree** | §3.10, §3.14 |
| 8 Bad Omen hygiene | **Agree** | §3.14 as ordinary policies |
| §2.5 escalation spam | **Agree** | Expected losses only move the ETA (§3.11) |

The stage-specific mitigations (capture stands outside other spawners' ranges, no wind-toggleable blocks on the cargo side, probing a portal before cargo crosses, opsec halts) are written into the worked example's operations. They are behaviours of generic skills and pack content, not new framework features.
