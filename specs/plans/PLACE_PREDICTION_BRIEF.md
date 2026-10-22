# Implementer brief: fork patch, place prediction stage 1

Written 2026-10-21 by the lead from read-only research of that day. Found by the patch 31 Pi pair: A13 `two_places_on_consecutive_ticks_both_done` failed on vanilla because the second place aimed at a block the bot had not predicted, and the server's update came later than one tick. Allen (2026-10-21): place prediction goes early, before the refusals (patch 33 brief) and walls (patch 34 brief). Launch it when patch 32 task 1 is committed and nothing else builds in the main fork clone. The patch number is the next free one on `null-main` at commit time (`null patch: <n>-place-prediction`). Delete this file when the patch is committed.

## Brief

You implement the first stage of client-side place prediction in the Azalea fork for Null (a Minecraft bot swarm in Rust on an Azalea fork). Repo: /Users/allen/Desktop/null. Read /Users/allen/Desktop/null/CLAUDE.md first and follow it exactly. Then read B6 (placement, and §4.6 the action mailbox), F5 §4.6 ("What Null computes") and ledger rows 20, 24, 25 and 28 in `specs/reference/AZALEA_PATCHES.md`.

**Why.** Vanilla writes a placed block into its own world at once and rolls it back if the server disagrees. The fork writes nothing until the server's update arrives. So for one round trip, the bot's world lacks a block that the server and Grim already have. A bot that places a block it then stands on, lands on or walks into (a pillar jump, a bridge) moves as if the block were missing, and Grim flags `Simulation`. A second place aimed at the new block is refused for nothing.

**The safety rule for this patch.** The bot's world may be wrong only where its physics does not read it. Inside the **movement zone** (the bot's box expanded by two ticks of its largest possible motion this tick, and at least 1 block on each side), a place is either predicted exactly or refused on the client with a reason, nothing sent. Outside the zone, a place whose state stage 1 cannot compute is sent unpredicted, as now (vanilla sends the packet whatever its local result; the cell's truth arrives with the server's update).

Every major function you add gets a one-line spec link, for example `// Spec: specs/B6-placement-breaking.md §4.6`. The verifier fails on a missing or dead link. Put the same rule in any brief you write.

### State

- Fork clone ~/Desktop/null-spikes/azalea, branch null-main. Work on top of the head Null's `Cargo.toml` pins (patch 32 when you start). Check `git log` first. Do not push.
- A worktree `~/Desktop/null-spikes/azalea-p34` (branch `p34-walls`, pathfinder only) may exist: do not touch it.
- The Null working tree may hold uncommitted work. Do not commit, stash, reset or revert anything in the Null repo. Add your changes on top.

### Vanilla (1.21.11, Mojang names; confirm in ~/Desktop/null-reference/vanilla-1.21.11-src/net/minecraft/)

Describe vanilla in your own words. Never copy Mojang, Paper or Grim code.

- `MultiPlayerGameMode#useItemOn` (~317): refuses outside the world border, then `startPrediction` (~287) raises the sequence and runs `performUseItemOn` (~331): block use first unless crouching with an item, else `ItemStack#useOn` (~360; PASS when the player may not build and the adventure rule does not allow it). The packet goes out whatever the local result.
- `BlockItem#place` (~46): `BlockPlaceContext#canPlace` (the clicked cell is replaceable, else the cell beside it; `canBeReplaced(ctx)`), `getPlacementState` → `Block#getStateForPlacement`, then `BlockItem#canPlace`: `canSurvive` and `Level#isUnobstructed(state, pos, CollisionContext.placementContext(player))`. Every non-removed entity with `blocksBuilding` (every living entity, the placer included) whose box meets the state's collision shape blocks it. Then `Level#setBlock` with flags 11, which also runs neighbour shape updates (so the client predicts those too), then `setPlacedBy` and the stack's count drops by one.
- `ClientLevel#setBlock` (~215): while predicting, each write first calls `retainKnownServerState(pos, oldState, player)`: the old state and the player position are stored once per cell; a later write to the same cell only refreshes the sequence. Server block updates for a retained cell go to `updateKnownServerState` and do not touch the world (`setServerVerifiedBlockState` ~193).
- `handleBlockChangedAck` (~2466) → `BlockStatePredictionHandler#endPredictionsUpTo`: for each retained cell with sequence at or below the ack, `ClientLevel#syncBlockState` (~199) writes back the server's state if it differs, and if the player then collides with that cell (`isColliding`), snaps the player to the stored position (`absSnapTo`).
- `SlabBlock#getStateForPlacement`: DOUBLE when the clicked block is the same slab kind; else BOTTOM or TOP from the face and the hit height; waterlogged from the cell's water (never for DOUBLE).
- Grim (`utils/anticheat/update/BlockPlace.java` ~545–612, `events/packets/CheckManagerListener.java` ~215–265) computes the placed state itself, skips it when its collision meets the player's or an entity's box, and writes it into its own world at once. So an unpredicted or wrongly shaped block near the bot costs `Simulation`.

### The fork today

- `BlockStatePredictionHandler` exists (`azalea-client/src/plugins/interact/mod.rs` ~98–205: `start_predicting`, `retain_known_server_state`, `end_prediction_up_to`); the snap-back is a TODO (~190, commented `is_colliding` / `abs_snap_to`). Mining uses retain (`mining.rs` ~546–552). Block updates: `block_update.rs`.
- `handle_place_block_queued` (`plugins/block_actions.rs` ~705–860) checks the click, sends `ServerboundUseItemOn` and starts a sequence; it never writes the world. `click_replaces` (~701–715) has `canBeReplaced` for snow and slabs; `BlockActionResult.before` is recorded.
- `azalea-block`: `BlockTrait` with `property_map`, `get_property`, `set_property` (`src/lib.rs` ~24–82), typed `BlockState::property::<P>()` (`azalea-block-macros` ~593), default state per `BlockKind` (~642–647). `FluidState::from(BlockState)` gives a cell's water. Collision shapes per state: `azalea-physics/src/collision/blocks.rs`. No `getStateForPlacement`, `updateShape`, `canSurvive` or item-to-block map.
- Patch 20's generator `codegen/gen_click_use_blocks.py` (class to superclass chain → table) is the model for a family table.

### Work

1. Warm the release build of null-abilitycheck first.
2. Fork, one commit on null-main (do NOT push):
   - **Family table.** A generator `codegen/gen_place_families.py` (uv only) and its output in `azalea-block`: for each block item, its block and its placement family, from the vanilla class chain. Stage 1 families: `Plain` (no properties, default state), `Axis` (from the clicked face), `Waterloggable` (default plus water from the cell), `Slab`, `HorizontalFacing` (simple facing from the player yaw, opposite as vanilla does for that class; only classes whose `getStateForPlacement` sets nothing else), and `Unknown` for everything else. Unknown means not predicted.
   - **Placed state.** Compute it as vanilla does for those families: the target cell (`canPlace` replace rule), the state, `canSurvive` only where it is trivially true for the family (full or solid-bottom blocks; else `Unknown`), and entity obstruction against the bot's own box and every living entity's box the bot sees (for other bots use the observer's `PeerView` from patch 31). An obstructed place sends the packet and predicts nothing, as vanilla's local `FAIL` does.
   - **Neighbours.** If any neighbour of the target cell is a shape-reactive block (fences, walls, panes, bars, stairs, redstone wire, tripwire, vines, chorus, multiface, leaves' distance, and other `updateShape` readers; list them from the source) and that neighbour is inside the movement zone, refuse with a reason. Outside the zone, predict the placed cell only.
   - **The movement zone and refusals.** A place in the zone whose family is `Unknown`, or whose prediction precondition cannot be checked (unknown game mode, a missing item `block_state` component the family needs), is refused on the client with a stable reason and patch 28 evidence; nothing sent. Choose and justify the zone size from vanilla's speed limits.
   - **Write and roll back.** Before the write, `retain_known_server_state` the old state and the bot's position; write in the same tick, before the physics step (check the system order against vanilla's `Minecraft#tick`: the click is handled before the player tick). Make server updates for retained cells go to the known state only. Implement the snap-back on the ack: restore the server's state and, if the bot's box then collides with that cell, snap the bot to the stored position.
   - **Game mode.** Find out whether the fork tracks the game mode (`mayBuild`, adventure). If not, add the minimum from the packets that carry it; until known, treat it as unknown (refuse in the zone, no prediction outside).
   - Name the vanilla methods and the Grim checks in doc comments. Keep the patch small and mostly additive.
   - Fork tests: each family's state (axis per face; slab bottom, top and double; waterlogged in water and not; facing per yaw); the replace rule; obstruction by the bot and by another entity; the zone refusal for `Unknown`; the shape-reactive neighbour refusal; retain, server update while retained, ack with the same state, ack with a different state (rollback), and the snap-back when the bot stands in the rolled-back cell. Run the fork test crates listed in ledger rows 30–32 and report the counts.
   - Write the diff to `specs/reference/azalea-patches/<n>-place-prediction.patch`, and move the pinned `rev` in Null's `Cargo.toml`. Do not write the ledger row; put a draft row in your report.
3. Null side: rows in A13 or a new module with its own area (the next free corner after A21 at 1010: 1040; check the `AREA` constants again). Rows, each with RCON readback and zero Grim alerts:
   - A13 `two_places_on_consecutive_ticks_both_done` as it is (the second place aims at the predicted block): both Done. It must pass on the Pi too, so do not loosen it.
   - `pillar_jump_places_under_feet`: the bot jumps and places dirt under itself at the top of the jump, five times; it ends five blocks up, standing; the server agrees.
   - `bridge_three_blocks`: the bot sneaks backward off an edge and places three blocks of a bridge; it never falls.
   - `rejected_place_rolls_back`: a place the server refuses (for example a protected spot, or a block at the target set by RCON in the same tick); the bot's world shows the server's state after the ack.
   - `unknown_family_in_zone_refused`: a block outside the stage 1 families (for example a fence) placed under the bot's feet: refused, nothing sent.
   - Recheck A4, A4S, A4C and A13 (placement): run those modules once per server.
   - Update B6 and F5 §4.6 for what you built; list stage 2 (facing with shape, attachments, chests, stacking, doors and beds) and stage 3 (neighbour `updateShape`) as open.
4. Negative-control readiness: say exactly how a verifier turns prediction off (no write after the packet), and which rows must then fail (the pillar row on Grim with `Simulation`, the two-places row on a slow server). Do not run it yourself.

### Live-run rule (Allen, 2026-10-19/20/21)

Live runs are slow: each runs strictly once. Selective runs: a change reruns only the modules `scripts/affected_modules.sh` picks from `crates/null-abilitycheck/run_map.toml` (`specs/plans/ABILITY_CHECK.md` "Which modules a change reruns"), never by guess. Add a `[[history]]` entry for this patch and a path entry for any new scenario file, and run `scripts/affected_modules.sh --check-history`; it must pass. If the helper prints `--scenario all`, run only your rows plus A4, A4S, A4C and A13, once on vanilla (harness/skeleton-server, 25567, RCON 25577) and once on Paper + Grim (harness/grim-server, 25568, RCON 25578), RCON password `skeleton`, each with a distinct `--account`. Both Mac servers are up; do not restart them. A failed row is fixed and only its module rerun, once. No repeat runs to double-check. Do not run a heavy build while a Grim row runs.

The Pi reproduces slow servers: the lead may ask Allen to run your rows there too. Do not touch the Pi yourself and do not run `scripts/pi/deploy.sh` or `launch.sh`.

No polling (Allen, 2026-10-21): no sleep or wait loops. Run each selective run in the foreground with a timeout and read its summary line once.

### Machine notes

- Python: uv only (`uv run --with`, `uv run --no-project`), never pip or system packages.
- `stat` is GNU coreutils: `/usr/bin/stat -f %m FILE` for BSD.
- Never read or change network configuration on any device (interfaces, IPs, routes, DNS, firewall, Wi-Fi, VPN, Tailscale).
- Bots must not be op. Grim flags clicks while moving, sprinting or sneaking (the bridge row: check `MultiActions` before you design it).

### Done when

The fork commit exists locally (not pushed), the pin is moved, the fork tests pass, and your rows plus A4, A4S, A4C and A13 each passed once on vanilla and once on Paper + Grim with zero Grim alerts.

### Report (under 700 words)

Fork commit hash; files changed; the vanilla points you followed; the zone size and why; whether the fork tracked the game mode; fork test counts; live result lines from both servers (log paths); any Grim alerts; the negative-control instructions; the draft ledger row; what stage 2 and 3 need; anything left open. If you fail twice on the same problem, stop and report rather than churn.
