# Shared-world prediction test

Queued 2026-11-02 (Allen). Runs after the VD2b departure fixes, before `mine`.

## Background

Bots of one swarm share one Azalea `World`. A bot writes its predicted block into that shared world: places at `azalea-client/src/plugins/block_actions.rs` (`handle_place_block_queued`, patch 33) and breaks at `mining.rs` (inherited from upstream). The rollback in `interact/mod.rs` (`end_prediction_up_to`) writes there too. So bot B can read bot A's unconfirmed block about one round trip before the server's update could reach B, and for a refused action B reads a state that never existed. No spec records a decision on this; B6 §4.10 treats the bot's world as private.

The companion bot has no skill and stands 2.5 blocks from the main bot, so 447/447 on the Pi pairs says little about this.

## Details

- **Where:** places in `azalea-client/src/plugins/block_actions.rs` (`handle_place_block_queued`, patch 33); breaks in `mining.rs` (inherited from upstream; upstream `main` has the same write, and a search of its GitHub issues and PRs found no decision); rollback in `interact/mod.rs` (`end_prediction_up_to`).
- **What can go wrong:** bot B reads bot A's block about one round trip before the server's update could reach B. For a refused action, B reads a state that never existed, and only A gets the snap-back. B can then collide or fall against Grim's view (`Simulation`), click a block it could not know about (a packet a vanilla client could not send), or decide wrongly. B's server update can also overwrite A's pending prediction, because B has no retained state for that cell.
- **Second leak, same cause:** the first bot to receive a server update writes it for all bots, so B can see a true update before B's own packet arrives. The overlay below does not fix this.
- **Why nothing caught it:** no spec weighed it. B6 §4.10 treats the bot's world as private, and its movement zone covers only the acting bot. `COMPANION_BRIEF.md` defines shared-world bugs as entity identity bugs. The companion has no skill and stands 2.5 blocks away, so the 447/447 pairs say little here. It is the same family as patches 1, 1b, 2, 40, 41, 43 and 44; patch 38's read of the shared chunk map is a related open gap.
- **Why it may not matter:** the window is short, harm needs contact with that cell during the window, most actions end up accepted, and claims may keep bots apart. `mine` and building put bots side by side, so these reasons are weakest there.

## Fix if proven

**Fix if proven (lead's recommendation, not decided):** a per-bot overlay: the shared `World` holds server truth only, each bot keeps its short list of pending cells and reads it first, and the ack deletes the entry. Bots with nothing pending pay one empty-check per read (not measured). The cost is one-time: `azalea-physics` passes `&World` through at least 20 signatures, and collision and `clip.rs` read below `World`, so each read path needs a test. Rejected: per-bot section copies (memory), swapping the overlay in during a bot's tick (parallel systems and pathfinder threads could read it). A claim gap is a stopgap only; it avoids the bug without fixing it.

## Goal (Allen, 2026-11-02)

The test aims to prove that this is a real issue. It is not a neutral survey: build the setup most likely to show harm. It gets 3 chances.

## Chances

- One chance is one live run of one design below, on the server that design names. This count is Allen's exception to "each live run runs strictly once" for this test only.
- Stop at the first chance that proves the issue. Do not spend the rest.
- A run that fails for a harness reason (a row setup bug, a run cut short) still counts, unless the run never reached the action under test. Report any such case.
- After 3 chances without proof, the result is "not proven in 3 chances", not "safe". Record it and go on to `mine`.

Designs, strongest first. An implementer may reorder or replace one only with a reason written in the result section.

1. **Break under a bot (Paper + Grim).** Bot A breaks the block that bot B stands on. B may read air and start to fall before the server's update could reach it. Expected harm: Grim `Simulation` on B.
2. **Refused place in a bot's path (Paper + Grim).** A's place is refused by the server while B walks into the cell. B collides with a block that never existed, and gets no snap-back. Expected harm: Grim `Simulation` on B, or a position that differs from the same walk with no place.
3. **Click on another bot's prediction (Paper + Grim, added latency if it helps).** B aims at and clicks A's predicted block before the server's update could reach B. Expected harm: a packet a vanilla client could not send, a Grim alert, or a wrong refusal by B. Latency may come from the existing tap or from any method that needs no network configuration change.

## Proof

All of these are needed:

- **The harm:** a Grim alert on B, or a packet B sends that a vanilla client could not send, or a wrong action result on B, read from the logs and RCON.
- **The cause:** a log line shows B read the cell while A's prediction for it was pending.
- **The control:** the same setup with A and B in separate swarms, so separate `World`s, does not show the harm. The control run does not use up a chance. It runs once for each chance that shows the harm.

Also write one fork simulation test first: two clients in one `World`, A predicts, no server reply, and B's collision reads the cell. It proves the code fact, not the harm, and uses no chance. Record it with the expected result stated.

Do not weaken a check to get or avoid a result. Do not build the overlay in this task. Fix options are for the lead and Allen once the result is in.

## Result

**Proven (chance 1 of 3, design 1, Paper + Grim, 2026-11-02).** The control did not use a chance. Logs are in `/Users/allen/Desktop/null-spikes/run-logs/shared_world/`.

- Fork code fact (no chance): two sim tests in the fork, commit 62972ef (local). A's unconfirmed break makes B fall; in the control B stays.
- Setup note: the first SW1 try used a slime block as the floor and never reached the action (the bot disconnects on slime, an unmodelled input; `setup_fail_slime.log`). It did not count. The floor is now TNT (hardness 0, full cube).
- **Harm:** `chance1.log`: Grim on B (companion `null_swc`, same swarm as A) drew `Simulation x4` and `GroundSpoof x1` after A broke the TNT under it. A drew none. B's server position ended at y 200.0 (correct), so B was moved down by its own prediction before the server update.
- **Cause:** `SW1_trace.log`, line 12: A has `pending=true` (seq 1/0) and B reads `cell=air` with y 200.92, on_ground=false, vy -0.155. The tick before, B read `cell=tnt` at y 201.0. A's ack arrives one sample later (seq 1/1). So B started to fall in the same tick A wrote its unconfirmed break.
- **Control:** `control_a.log` and `control_b.log` (A and B in separate processes, so separate Worlds): same break, A pending in 1 sample, B fell after the server update (`null_swab_trace.log`), **zero Grim alerts and no warnings**.
- Contention: `chance1_contention.log` and `control_contention.log`: no other null-abilitycheck, cargo or rustc process; CPU idle 64 to 80%. Client longest tick gap 62 ms, no catch-up in chance 1.
- Caveat: the harm is one row, one run each. Grim alerts a fall that starts one tick early (Simulation), not a wrong final position. The setup is the strongest case: B stands on the cell A breaks. Designs 2 and 3 were not run.
- Verifier (Sonnet, logs only, 2026-11-02): pass with caveats. The alert lines, the pending flag and the trace hold (server.log 10163-10167). The evidence bears on A's unconfirmed write, because B fell before any server reply. It does not show the second leak, and a single-millisecond race is not fully excluded.
- The control is not a perfect twin. In chance 1, B logged in 7 s before the action, in the air at the position saved by the slime try, and drew 3 join alerts at 22:04:10 (`Simulation x2`, `GroundSpoof x1`; server.log 10149-10151). The row's "setup-window alerts []" line misses them, because its window starts later. In the control, B logged in 19 s before the action, at spawn, with no join alerts. The control's `first fall` line is B at y 200 before the teleport, so read the trace instead. A cleaner control needs B to log in at the same saved position with the same join age.
- The lead accepts the proof: the fall while A's break was pending is a state a vanilla client cannot reach, with or without the Grim history. The slime disconnect is the safe-envelope guard (patch 30), not a new bug.

## Fix (patch 53, per-bot prediction overlay; Allen approved the design 2026-11-02)

Built in the fork as `null patch: 53-per-bot-prediction-overlay`, pushed and pinned at `f994d4d` (2026-11-03, ABILITY_CHECK "p53_full results"). Details: B6 §4.10 and the ledger row 53.

- **Design.** The shared `World` holds server data only. A bot's predicted block lives in its own `BlockStatePredictionHandler` (entry: retained server state, sequence, position, and `predicted: Option<BlockState>`; `None` = override off, entry kept for the ack). `BotWorld` (shared world + this bot's handler) is what physics, collision, the pick ray, the click and break checks and `mc-adapter`'s `BotView` block reads take. A collision query tests once for an override in its box; with none the old path runs unchanged. A server update always writes the shared world and also updates the retained state of a pending cell. The ack deletes the entry.
- **Edge cases, one fork test each** (`azalea-client/tests/simulation/shared_world_prediction.rs`, unit tests in `azalea-world`/`interact/mod.rs`, `azalea-physics/tests/physics.rs`): own chunk re-sent (the packet's block wins, as `ClientPacketListener#updateLevelChunk`), forgotten chunk (overrides of that chunk turn off at the forget, so the overlay cannot bring the cell back), two clients in one world (**the old SW code-fact test changed its expected result: B now stays on the block; A alone collides with its own prediction**), respawn (below), refused break (B never sees it, A gets the block back at the ack).
- **Respawn bug: real.** Vanilla's `ClientPacketListener#handleRespawn` builds a new `ClientLevel` for another dimension, and the level owns the `BlockStatePredictionHandler`, so pending cells and the sequence number start again. The fork never reset the handler, so it carried pending cells and a growing sequence number into the new dimension (a wire difference: the next `seq` should be 1). Fixed in patch 53; a respawn in the same dimension keeps the handler, as in vanilla.
- **Left on shared truth, by design:** the pathfinder and `find_blocks` (reported if a plan needs the bot's own pending cells), chunk presence, entities, registries.
- **Not fixed, recorded as open:** the second leak (the first bot to receive a server update writes it for all bots) and a delayed packet that overwrites newer shared truth.
- **Vanilla difference left:** if a chunk packet re-sends a cell during its pending window and the packet's block differs from the retained state, vanilla writes the retained state over the packet's block at the ack (`syncBlockState`). The overlay never writes the shared world, so the packet's block stays. Needs a server that changes a block without a block update; not seen.

- **SW1 on the final binary (fork 2a06a48, Paper + Grim, account `null_swf`, one run, 2026-11-02).** Result: no harm. A's break was acknowledged at tick 164, B fell only after the server cell was air (B min y 200.000), and the trace shows zero samples where B read air while A was pending. Alerts: join/setup window on B 0; B after the action 0; A after the action 0; server log no warnings. The row prints FAIL because SW1 expects harm: "NO HARM" is the fixed outcome. Before the fix (`chance1.log`): `Simulation` x4 and `GroundSpoof` x1 on B. CPU idle during the run 80-82%, no other null-abilitycheck, cargo or rustc process. Logs: `~/Desktop/null-spikes/run-logs/shared_world/fix/sw1.log`, `sw1_samples.log`. New Bevy warning in the log: one pair of systems with conflicting access to `BlockStatePredictionHandler` (`apply` and `update_fluid_on_eyes`) has no order; not seen before the patch. Needs an explicit order (follow-up).
- **Ordering fix and SW1 as a pass (2026-11-03).** The Bevy warning is closed in Null: `apply` is the `mc-adapter` host system (it holds the handler mutably but does not use it), so `crates/mc-adapter/src/host.rs` now chains it before `update_fluid_on_eyes` (vanilla handles input, `Minecraft#handleKeybinds`, before the player's `Entity#baseTick` -> `updateFluidOnEyes`). The fork is unchanged (head `2a06a48`). SW1 now passes on zero Grim alerts and zero server-log warnings on B and fails on any. The proof that the bar can fail is the pre-fix log `chance1.log`: `Simulation` x4 and `GroundSpoof` x1 on B (no new negative control). SW1C has no harm bar and is unchanged. Final run, once, Paper + Grim, account `null_swg`: PASS, `NO HARM`; A's break acked at tick 166, B min y 200.000, zero samples of B reading air while A pending, no alerts, no warnings, no ambiguity warning in the log. Log: `~/Desktop/null-spikes/run-logs/shared_world/fix/sw1_final.log`.
- **Verifier fixes and stricter SW1 (2026-11-03, fork head `f994d4d`).** (1) Fork: a dimension respawn reset the handler's sequence to 0 but left pending `BlockActionResults` with old sequence numbers, so a later ack could settle them as acked. `end_unacked_after_reset` now ends pending sent places and breaks as `Refused` with `NoAck` and pending uses as `NoScreen`; new sim test `a_pending_place_ends_unacked_at_a_dimension_change_and_a_new_ack_does_not_settle_it` fails without the call (refusal `None`, settled by the new ack) and passes with it. (2) The SW1 bar now needs all of: no harm, the action outcome `Done`, the server cell air, and at least one sample with A pending while B reads TNT and stands on it (y within 0.05 of the stand height, on ground). SW1C is unchanged. (3) `host.rs`: the unused handler parameter of `apply` is removed; the `.before(update_fluid_on_eyes)` order stays (vanilla order). Final run, once, Paper + Grim, account `null_swg`: PASS, `NO HARM`; break done, ack tick 162, cell air, 1 sample with A pending and B on TNT, B min y 200.000, zero alerts and warnings, no ambiguity warning in the log. Log: `~/Desktop/null-spikes/run-logs/shared_world/fix/sw1_final2.log`. The first launch of the binary exited at 0.0 s because the `--companion` flag was missing; it ran nothing.
