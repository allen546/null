# Implementer brief: fork patch 31, interpolation and the bot-to-bot view

Written 2026-10-20 by the lead from the read-only research of that day. Launch it only when Allen's full-suite pair for patches 27–30 and the harness timing task have ended, and nothing else is building or running. Delete this file when patch 31 is committed.

## Brief

You implement Azalea fork patch 31 for Null (a Minecraft bot swarm in Rust on an Azalea fork). Repo: /Users/allen/Desktop/null. Read /Users/allen/Desktop/null/CLAUDE.md first and follow it exactly. Then read `specs/plans/PATCH_30_TIER1.md` §2.3 (the push rules, the bot-to-bot view and the split), `specs/DECISIONS.md` AD-F5.5 (tier 1 details), F5 §4.6 (the interpolation bullet) and B1 §4.2. Your scope is interpolation and the bot-to-bot view only. Pushes and teams are patch 32; do not build them.

### State

- Fork clone ~/Desktop/null-spikes/azalea, branch null-main, head <current head; e591089 is patch 30>, pinned in Null's `Cargo.toml`. Check `git log` there first: if patches 27–30 have been pushed, work on top of the pushed head.
- The Null working tree may hold uncommitted work. Do not commit, stash, reset or revert anything in the Null repo. Add your changes on top.

### Vanilla (1.21.11, Mojang names; read ~/Desktop/null-reference/vanilla-1.21.11-src/ to confirm)

- `InterpolationHandler#interpolateTo(pos, yRot, xRot)`: with 0 steps it snaps; a new target different from the current one starts N steps (default 3) from the entity's current position; a target equal to the active one is ignored. `#interpolate`, once per tick: move by 1/stepsLeft of the remaining gap per axis (1/3, then 1/2, then the rest), then decrement. It also shifts the target by any displacement something else caused since the last tick, but only when the shifted box is free; that matters only for riders and carried entities. Smooth position only (Allen, 2026-10-20): no client decision reads another entity's rotation.
- Steps by type: `LivingEntity`, `AbstractBoat`, `ExperienceOrb`, `FishingHook` and `OldMinecartBehavior` 3; `Display` 0 (snap); `Shulker` has no handler (set at once); item entities and projectiles have no override (set at once). `NewMinecartBehavior` uses `ClientboundMoveMinecartPacket` lerp steps; if the fork does not handle that packet, leave new-behaviour minecarts at once and note it.
- When it runs: `LivingEntity#aiStep` near the start, after `noJumpDelay`, before travel and `pushEntities`; `RemotePlayer#aiStep` (interpolate, head lerp, motion lerp, `pushEntities`; no `super.aiStep`); `AbstractBoat#tick` after `super.tick()`; `OldMinecartBehavior#tick` client branch.
- Packets (`ClientPacketListener`): `handleMoveEntity` uses `moveOrInterpolateTo`, then `setOnGround`; `handleEntityPositionSync` interpolates, but snaps when the entity is not in the tick list or the jump is over 64 blocks; `handleTeleportEntity` interpolates when the entity is ticking, is not locally authoritative or the teleport is relative, otherwise sets at once, and also snaps over 64 blocks; `handleAddEntity` snaps (`recreateFromPacket`) and sets the codec base. The fork's `VecDeltaCodec` already matches vanilla (`azalea-entity/src/vec_delta_codec.rs`); the base is the decoded target, not the smoothed position.
- Packets are applied at the start of the client tick, before `tickEntities`, so step 1 runs in the tick the packet arrives (confirm the order in `Minecraft#tick`).

### The fork today (head e591089)

- Other entities' positions are set at once: `move_entity` (`packet/game/mod.rs` ~1933), `entity_position_sync` (~1686), `teleport_entity` (~898). No interpolation exists.
- The swarm shares one world. One ECS entity has one `Position`, written only by its `UpdatedBy` client (`relative_updates.rs` ~86). A bot (`LocalEntity`) is never moved by another bot's packets, so bot A sees bot B at B's exact simulated position. A vanilla client would see the server's position for B, smoothed.
- Readers that will see the smoothed positions: `LastBoundingBox` (`azalea-physics/.../entity_collisions.rs`; refreshed in `Update` after `update_bounding_box`, `azalea-entity/src/plugin/mod.rs` ~221; it must also be refreshed in `GameTick` after interpolation, or the box lags a tick), `pick_entity` (`interact/pick.rs`), `entity_actions.rs` (`aim_at_entity`, `handle_look_at_entity_queued`, `handle_use_entity_queued`, `interact_at_location`), and physics collision with boats and shulkers.

### Work

1. Warm the release build of null-abilitycheck first.
2. Fork, one commit `null patch: 31-entity-interpolation` on null-main (do NOT push):
   - An interpolation component and a per-tick system for non-local entities, at vanilla's point in the tick, with the per-type step table above. Change the packet handlers to interpolate or snap as vanilla does.
   - **Bot-to-bot view (Allen, 2026-10-20):** each bot keeps its own smoothed view of the other bots within 16 blocks, at most its 32 nearest, with its current aim target always kept. Build each view from the server's packets for that bot, which the fork now drops for `LocalEntity`. Reaching the cap is logged once per bot per episode. Outside the radius nothing is kept. Mobs and other non-bot entities keep one shared smoothed copy (the server sends every client the same positions). All the readers above use the observer's view when the target is another bot. Add a counter of the per-observer entries (current and peak) that the ability check and the load tests can read.
   - Name the vanilla methods (Mojang names) and the Grim checks (`Reach`, `Hitboxes`; `MovementTicker#handleEntityCollisions` uses the interpolation range for boxes) in doc comments. Describe vanilla in your own words; never copy Mojang, Paper or Grim code. Keep the patch small and mostly additive.
   - Fork tests: the step sequence (1/3, 1/2, rest); an equal target ignored; snaps at 0 steps, over 64 blocks and on add; the per-type table; the box refreshed in the same tick; the bot-to-bot view (built from packets, radius, cap with the target kept, counter). Run the fork test crates listed in ledger row 30 and report the counts.
   - Write the diff to `specs/reference/azalea-patches/31-entity-interpolation.patch`, and move the pinned `rev` in Null's `Cargo.toml` to the new head. Do not write the ledger row; put a draft row in your report.
3. Null side: a few live rows (a new module, or rows in A17), each with its own test area (`Ctx::scene_in`). Examples: a mob teleported by RCON is reached by the bot's view in 3 steps (the trace of the view against the vanilla sequence); an entity click right after a mob moves uses the smoothed box; two bots near each other each see the other from server packets (the counter shows the entries; a far bot is not tracked). Recheck A8 and A10E, because entity aim now uses smoothed positions: run those two modules once per server. Every major function gets a one-line spec link (`// Spec: specs/... §...`); the verifier fails on a missing or dead link. Update F5 §4.6 and B1 §4.2 from "planned" to built for what you built.
4. Negative-control readiness: say exactly how a verifier turns interpolation off (positions set at once) so that your new rows fail. Do not run it yourself.

### Live-run rule (Allen, 2026-10-19/20)

Live runs are slow: each runs strictly once. Run only the rows you add or change, plus A8 and A10E as named above, once on vanilla (harness/skeleton-server, 25567) and once on Paper + Grim (harness/grim-server, 25568), with a distinct `--account`. A failed row is fixed and only its module rerun, once. No full suites and no repeat runs to double-check. Do not run a heavy build while a Grim row runs. At the end, with the code final, give the full-suite command for Allen: `scripts/full_suite.sh p31_full` (or the manual commands in HANDOFF, if the script is not there).

### Machine notes

- Python, if needed: uv only (`uv run --with`), never pip.
- `stat` is GNU coreutils: `/usr/bin/stat -f %m FILE` for BSD.
- Bots must not be op. Grim flags clicks while moving, sprinting or sneaking.

### Done when

The fork commit exists locally (not pushed), the pin is moved, fork tests pass, and the new rows plus A8 and A10E each passed once on vanilla and once on Paper + Grim with zero Grim alerts.

### Report (under 700 words)

Fork commit hash; files changed; the vanilla points you followed; fork test counts; live result lines from both servers (log paths); any Grim alerts; the peak per-observer entries seen; the negative-control instructions; the draft ledger row; anything left open. If you fail twice on the same problem, stop and report rather than churn.
