# Vanilla diff: clean client, tap, driver (VD0), replay and differ (VD1a), A15 and A17 (VD1b), actions A1-A4 (VD2a), A9, A13 and A22 (VD2b)

Spec: `specs/plans/VANILLA_DIFF_PLAN.md` VD0. A real vanilla 1.21.11 client joins `skeleton-server` through a packet tap, and our own minimal mod drives it. The replay driver and the differ are VD1/VD2.

## Parts

- `mod/`: our Fabric mod (Java, Fabric Loom 1.16.3, Yarn `1.21.11+build.6`, build with system `gradle` on JDK 21: `JAVA_HOME=/Library/Java/JavaVirtualMachines/jdk-21.jdk/Contents/Home gradle build`). The jar goes to `~/Desktop/null-spikes/vanilla-diff-client/game/mods/null-vanilla-diff.jar`. One mixin, `ClientTickMixin`, calls `Driver.onClientTick` at the head of `MinecraftClient#tick` (`Minecraft#tick`). `Driver` joins the tap address through vanilla's `ConnectScreen`, then holds `options.forwardKey` for `-Dvd.walkTicks` ticks (default 20) after `-Dvd.settleTicks` player ticks (default 40), then stops the client. Held through the key mapping, so vanilla's `KeyboardInput` builds the movement input and vanilla decides what to send. It sends no packet itself.
- `tap/` (crate `vd-tap`, workspace member): a transparent TCP proxy, default `127.0.0.1:25569` to `127.0.0.1:25567`. It forwards bytes unchanged in the chunks they arrive in, and decodes a copy with `azalea-protocol` (compression threshold taken from `LoginCompression`, protocol state tracked through `LoginAcknowledged` and `FinishConfiguration`). It writes JSON lines `{t_ms, tick, state, dir, name, summary}`; `tick` counts serverbound `ClientTickEnd`, and the tick-end packet carries the tick it ends. A second connection writes `<out>.2`. A harness tool: listed in `scripts/check_boundary.sh`, never runs in a bot.
- `run_client.sh`: launches the client. It expects the layout below. `VD_USER` sets the offline name (default `VDClient`), `VD_JVM` adds JVM flags, `VD_JAVA` sets the Java 21 binary. The mod jar builds into the client's `game/mods` (`-Dvd.mods=<dir>` to change it).

## Client directory (outside the repo, never committed)

`~/Desktop/null-spikes/vanilla-diff-client/`:

- `hmc/`: `headlessmc-launcher-wrapper-2.10.0.jar` and `headlessmc-launcher-2.10.0.jar` (copied from `fearless-pascal`, which is not modified) and `HeadlessMC/config.properties` (`hmc.offline=true`, `hmc.mcdir`, `hmc.gamedir`). HeadlessMC's `-lwjgl` flag stubs GLFW/GL so no window opens. Mac JDK 21.
- `mc/`: the Minecraft dir. `fabric 1.21.11` (typed into the HeadlessMC prompt) wrote the Fabric profile (loader 0.20.1). Mojang's `1.21.11.jar` (sha1 `ba2df812c2d12e0219c489c4cd9a5e1f0760f5bd`, same as `null-reference` R1) and libraries came from HeadlessMC downloads, plus a read-only copy of existing libraries from the launcher dir when the network timed out; copy the vanilla jar to `versions/fabric-loader-0.20.1-1.21.11/fabric-loader-0.20.1-1.21.11.jar` to skip the download. Assets are the real ones (HeadlessMC downloaded them).
- `game/`: `mods/` holds only `null-vanilla-diff.jar`. `options.txt` has `onboardAccessibility:false` (else the first-run accessibility screen blocks the title screen), `tutorialStep:"none"`, `pauseOnLostFocus:false`. Everything else is vanilla default.

Run: start the server (`harness/skeleton-server/scripts/start_server.sh`), then `cargo run -p vd-tap -- --out <log.jsonl>`, then `run_client.sh`. The client quits itself about 1 s after the walk.

## Mod judgements (against vanilla)

Dropped, because they change what the client does: Meteor Client, Baritone, Nora Tweaks, Trouser Streak, the swarm Baritone mod (`swarm-baritone-bot`, whose companion code is replaced by our mod).

- **Krypton: dropped.** It replaces the client connection's compression and encryption handlers, the varint and string encoders, and adds `ConfigurableAutoFlush` (`ClientConnectionMixin`, `microopt/*`). Compression framing, flush timing and the split of writes into TCP chunks can change with it, and the tap and later timing comparison read exactly those. Not provable harmless, so dropped (when unsure, drop).
- **`hmc-specifics`: dropped.** It carries mixins on `Minecraft`, `LocalPlayer`, `BlockableEventLoop`, `InputConstants`, `ChatComponent`, `Font` and widgets, and a command layer (key, click, connect). Several touch tick and input code. The client runs without it; our mod does the connect and key press. Only HeadlessMC's launcher stays, with `-lwjgl`, which stubs the window and GL calls and does not touch game logic.
- **`ChunkLoadProgressMixin`: not carried over.** Vanilla (`LevelLoadTracker`) sends `PlayerLoaded` when the chunk section at the player is compiled and visible, or after a 30 s timeout. Measured here on the GL stub: `PlayerLoaded` goes out at client tick 6 after the first game packets, with no mixin, so the stub renderer compiles the section fast and the mixin is not needed. Null must still match this tick on the server (VD1: compare the `PlayerLoaded` tick, and what the server does before it; the vanilla client does not tick the player before it).
- **`GameRendererMixin`: not carried over.** It cancelled `GameRenderer#render`, which also skips `GameRenderer#pick` (the crosshair pick, run per rendered frame, not per tick) and the section compilation above. Here rendering runs against the GL stub, so `pick` runs each frame. VD1/VD2 must check that the frame rate does not change attack and use packets; `maxFps` is the knob (the frame limiter sleeps in `glfwWaitEventsTimeout`).
- **`HudMixin`, `FontsMixin`, `ReflectInitMixin`:** target Meteor classes only; gone with Meteor.

## Result (2026-10-28, Mac)

Done-when met on the first live run (`logs/tap.jsonl` in the client dir): forward held for client ticks 40 to 59, one `MovePlayerPos` before each of the 20 `ClientTickEnd`s, z 347.60 to 351.56, `PlayerLoaded` at tick 6, no decode errors; 20.0 ticks/s, about 620 MB RSS, 60 to 70 s from launch to join. A second run (memory sample) started where the first ended, since the server keeps player data, and hit a wall at tick 54: VD1 must use a fresh player name or clear the player data so each run starts at the same spot. Open for VD1: the client runs with real rendering against the stub, so frame count and `pick` per frame are unchecked; tick catch-up behaviour under load; `ClientInformation`, `ChunkBatchReceived`, keep-alive and pong timing are the timer packets for the differ's ignore list.

## VD1a: input log, replay driver, differ (2026-10-29)

Spec: `specs/plans/VANILLA_DIFF_PLAN.md` VD1. Same input into Null and into the vanilla client; the serverbound packets are compared per client tick. VD1a builds the three parts and proves them on one row (the A15 control walk). VD1b covers all A15 and A17 rows.

### Parts

- **Fork patch 45 `input-log`** (`azalea-client/src/plugins/input_log.rs`, row in `AZALEA_PATCHES.md`). Off unless the app has an `InputLogDir`. One JSON line per client tick, `{"tick":N,"f","b","l","r","jump","sneak","sprint","yaw","pitch","slot"}`, written where vanilla's `LocalPlayer#aiStep` has just read the keys. `N` counts client ticks of the connection from 0, as the tap counts `ClientTickEnd`. `{"mark":"start:<id>:<row>","tick":N}` lines carry the row windows. `mc-adapter` has `InputLogPlugin(dir)` and `mark_input`; `null-abilitycheck --input-log DIR` turns it on and `Ctx::run` marks `start:` and `end:` for every row (a no-op without the flag).
- **Replay** (`mod/.../Replay.java`, started by `Driver` when `-Dvd.input` is set). Per tick it sets the key mappings (forward, back, left, right, jump, sneak, sprint) and the player's yaw, pitch and hotbar slot at the head of `Minecraft#tick`, so `KeyboardInput#tick` builds the input and vanilla decides what to send. Look is set directly (the mouse path is per frame, not per tick). Properties: `vd.input` (log file), `vd.row` (the window, from its two marks; else the whole file), `vd.control` (the replay waits until this file exists and the player rests on the ground for 10 ticks), `vd.marks` (output: `replay_start_tick` is the client tick of the first replayed input, counted as the number of `ClientTickEnd` sent before it).
- **Scene for the vanilla player**: `null-abilitycheck --prepare-scene A15 --player NAME` waits for the player to join, then runs the module's own scene code (`prepare_control`: `scene_in` and lane 0) with that name, with no bot. Only A15 so far. A fresh name per run (`VD<MMDDhhmmss>`) gives a fresh start.
- **`run_replay.sh`**: starts the tap, the client and the scene, touches the control file, waits for the client to quit. **`run_null.sh`**: tap plus `null-abilitycheck` with the input log. **`differ/vd_diff.py`** (`uv run`, tests: `cd differ && uv run python -m unittest test_vd_diff`, 29 tests with a negative control): aligns the windows by tick (`replay_start_tick - first_input_tick`), compares serverbound game packets in order (summary strings, so coordinates must be equal), reports the first difference with both packet lists, the clientbound arrivals and the inputs around it. A difference at or after the first different clientbound arrival (timer packets excluded) is labelled "server timing". Ignored serverbound packets, each with its vanilla sender: `KeepAlive` (`ClientCommonPacketListenerImpl#handleKeepAlive`), `Pong` (`#handlePing`), `ClientInformation` (`Options#broadcastOptions`), `ChunkBatchReceived` (`ClientPacketListener#handleChunkBatchFinished`). `ClientTickEnd` is checked as one per tick.

### Hand-written check (`inputs/hand_written.jsonl`, 66 ticks, vanilla server, ~/Desktop/null-spikes/run-logs/vd1a/hand_vtap.jsonl)

Read by hand against the input (tick = replay tick): forward from 5 gives one `PlayerInput{forward}` and `MovePlayerPos` the same tick; yaw 18, 36, 54 with movement give `MovePlayerPosRot` at 15 to 17, then `MovePlayerRot` at 18 and 19 when the player stands against a wall (`horizontal_collision` true); pitch 20 at 23 gives one `MovePlayerRot`; jump at 25 gives `PlayerInput{jump}` with y 71.42, released at 26; sneak at 34 gives `PlayerInput{shift}`, released at 42; sprint at 44 gives `PlayerInput{sprint}` and `PlayerCommand StartSprinting` the same tick, and `StopSprinting` at 54 when the key goes; back at 58 gives `PlayerInput{backward}`; the idle ticks send nothing until the 20-tick position reminder at 86 and 106. Every packet is where the input puts it. The player started against a wall and stepped up a block while jumping; that is the world there, not the replay.

### Result (A15 control row, vanilla server)

Null: `run_null.sh --scenario A15 --account vd1a_null` (`~/Desktop/null-spikes/run-logs/vd1a/a15_null.log`, 9/9 pass; input log has 892 lines for 892 `ClientTickEnd`, so the counts agree). Vanilla: `run_replay.sh --row A15:plain_stone_walk_matches_the_model --scene A15` (`a15_replay.out`, replay began at client tick 75; Null's window starts at 71). Differ: `MATCH: 55 ticks compared, serverbound packets identical` (30 packets: 28 `MovePlayerPos`, 2 `PlayerInput`, equal down to the last digit of each coordinate). Negative control: the same logs with the offset moved by one tick report `DIFF: 30 of 55 ticks differ`. No finding: Null and vanilla agree on this row. The first replay run failed in `--prepare-scene` (`wait_block` asked the missing bot); fixed and rerun once.

### For VD1b

The scene for the other A15 rows (they need `lane` with their setup) and A17 rows; rows that end in a disconnect or use `goto` (the pathfinder's input is recorded, but the row window includes RCON sleeps); the 20-tick position reminder phase at the window start was equal here by luck of a settled start, check on rows that start moving later; Grim alert lists (VD3).

## VD1b: all A15 and A17 rows (2026-10-31)

Spec: `specs/plans/VANILLA_DIFF_PLAN.md` VD1. The same pipeline as VD1a over every A15 and A17 row. Null runs `--scenario A15,A17` once on the vanilla server through the tap (`~/Desktop/null-spikes/run-logs/vd1b/null_a15_a17.log`, 18/18); each row is then replayed once in a fresh vanilla client and diffed.

### Parts added

- **`run_rows.py`** with **`rows.json`**: for every row of the Null run, reads the row window from the input log marks, takes the position-reminder phase from the tap, runs `run_replay.sh` (fresh player, scene, mod, tap) and then the differ. `--rows`, `--module`, and `--diff-only` (re-diff the logs already on disk when only the differ changed; costs nothing live). `rows.json` gives each row its scene, its anchors, and a `skip` reason for the three A17 disconnect rows (Null refuses on purpose, patch 30; the vanilla client would not disconnect).
- **Acts and `--progress`**: a row's RCON actions (`effect give`, `kill`, `summon`...) run in the scene process, not in the mod. The mod writes its replay tick to the progress file and `--prepare-scene` runs each act when the replay reaches the tick that Null ran it at.
- **`--prepare-scene MODULE:ROW`** (A15 and A17): runs one row's own setup code on the vanilla player, so the vanilla client stands where Null stood.
- **Differ anchors** (`--anchor NAME`, `--stop-at-anchor`): a clientbound packet that the row's RCON action causes (`UpdateMobEffect`, `RemoveMobEffect`, `PlayerPosition`, `Respawn`, `PlayerCombatKill`). The k-th arrival in each run is matched, and each stretch is compared from its own anchor, so a server reply that lands a tick later does not shift the rest. The shift is printed.
- **`ClientPlayerAccessor`** (mixin accessor): sets vanilla's position-reminder counter (`LocalPlayer#positionReminder`) so the 20-tick reminder has the phase Null's client had at the window start. Client state only; it sends nothing.
- **Ability check change**: A15 runs the two powdered-snow rows last (see finding 3). Selective reruns A15 on vanilla and A15 + A17 on Paper + Grim passed.

### Result

| Row | Result |
| --- | --- |
| A15 plain_stone_walk_matches_the_model | MATCH (56 ticks) |
| A15 walk_into_cobweb_is_slowed_like_vanilla | MATCH (55) |
| A15 walk_into_sweet_berry_bush_is_slowed_like_vanilla | MATCH (54) |
| A15 walk_across_soul_sand_is_slowed_like_vanilla | MATCH (54) |
| A15 walk_across_honey_block_is_slowed_like_vanilla | MATCH (54) |
| A15 walk_into_powder_snow_is_slowed_like_vanilla | DIFF 8 of 54, server timing (finding 1) |
| A15 powder_snow_without_boots_does_not_lift_the_bot_at_a_wall | DIFF 36 of 104, server timing (finding 2) |
| A15 goto_over_the_same_ground_without_a_cobweb | first run DIFF 37 of 42, harness artefact; after the fix MATCH (40 ticks; `~/Desktop/null-spikes/run-logs/vd1b/rerun_a15/rows/`, the copy in `rows/` is the pre-fix run) (finding 3) |
| A15 goto_across_a_cobweb_goes_around_it | MATCH (44) |
| A17 control_free_fall_matches_the_model | MATCH (27) |
| A17 levitation_i_rises | MATCH (144) |
| A17 levitation_ii_rises | MATCH (86) |
| A17 slow_falling_fall_20_blocks | MATCH (127) |
| A17 slow_falling_arrow | MATCH (8, window cut before the effect) |
| A17 respawn_keeps_base_attribute | DIFF 4 of 244, server timing (finding 4) |
| A17 disconnect_on_scale, disconnect_on_ride, disconnect_in_bubble_column | NOT COMPARED on purpose |

Every difference is a clientbound packet that reaches the two clients at different client ticks, or state the Null bot carried between rows. No difference is a Null/fork departure from vanilla. Logs: `~/Desktop/null-spikes/run-logs/vd1b/rows/*_diff.txt`, `*_vtap.jsonl`; the Null tap is `~/Desktop/null-spikes/run-logs/vd1b/null_tap.jsonl.3` (the row bot is the third connection, the others are idle spare accounts).

### Findings

1. **walk_into_powder_snow: server timing.** The first 13 moves are equal to the last digit. The server freezes the player (`SetEntityData` index 7 = ticks frozen, then `UpdateAttributes` with the `powder_snow` movement-speed modifier, vanilla `LivingEntity#tryAddFrost`, amount -0.05 * frozen / 140, ADD_VALUE) one tick later in Null's run: vanilla's client has the first packet pair at its tick 109 before it builds the move, Null at 543, one tick after the aligned tick 542 (Null: the pair arrives 5 ms after tick 542 ended, vanilla: 25 ms after tick 108 ended). The packet contents are the same (ticks frozen 1, 2, 3, 4; amounts -0.000357, -0.000714, ...). Vanilla given the packet computes what Null computed: at the tick where vanilla has the first modifier and Null does not, z differs by 0.000315 = 0.000357 (the modifier) * 0.98 (input) * 0.9 (the powdered-snow stuck multiplier), exactly (348.24089839 against 348.24058339). The divergence starts at the arrival. Why the server freezes one tick earlier relative to the move count is the server's tick phase against the client's.
2. **powder_snow_without_boots: server timing.** The bot is thawed (ticks frozen 0) when the row starts in both runs. Freeze packets begin at Null 627 and vanilla 88 (aligned 628) and go equal for 8 ticks; at tick 635 / 95 Null has two server ticks in one client tick (modifiers -0.002857 and -0.003571, 45 ms and 15 ms before the tick end) and vanilla one (-0.003214). z differs by 0.000315, again one freeze step * 0.98 * 0.9. Packets that arrive within 2 ms of a tick end count for the next tick (Null 627, 631, 633: z equal to vanilla one tick later). After the first bunched pair the lag stays.
3. **goto_over_the_same_ground_without_a_cobweb: harness artefact.** The Null bot runs all rows of A15 on one player, and the two powder-snow rows came before this one. At the row start (tick 759) the bot still had 46 frozen ticks, thawing 2 per tick (`LivingEntity#aiStep`), so the server sent `SetEntityData` + `UpdateAttributes` with a decaying `powder_snow` modifier for 23 ticks (until tick 782, ticks frozen 0). The vanilla replay uses a fresh player and gets none. First sprint tick: Null z +0.11011, vanilla +0.1274; vanilla's formula gives (0.1 - 0.0135714) * 1.3 * 0.98 = 0.11011 and 0.1 * 1.3 * 0.98 = 0.1274, with the modifier of the packet the bot had (-0.0135714). Lead's check on the sprint modifier: vanilla `ClientPacketListener#handleUpdateAttributes` does remove every modifier (sprint too), but `LocalPlayer#aiStep` calls `setSprinting(true)` every tick while the sprint key is held, and that puts the modifier back before `travel`; and the fork keeps its sprint modifier. So there is no difference on that point. **Fix:** A15 runs the powdered-snow rows after the goto rows (`scenarios_block_effects.rs`, comment at the call). A15 reran once on vanilla (9/9, `~/Desktop/null-spikes/run-logs/vd1b/rerun_a15/`), the row was replayed once: `MATCH: 40 ticks compared`.
4. **respawn_keeps_base_attribute: server timing.** First run: the `PerformRespawn` command looked one tick off. Vanilla's death screen (`DeathScreen`) enables its button after 20 ticks and the replay presses it then; it comes 19 ticks after `PlayerCombatKill` in both runs (Null 1682 to 1701, vanilla 180 to 199). The command was only misplaced because the kill itself landed one client tick earlier in the vanilla run (a server reply); `PlayerCombatKill` is now an anchor in `rows.json`, and a re-diff (`--diff-only`, nothing live) shows the command equal. What is left, 4 ticks: the server sent the new chunk batch (`ChunkBatchStart`, 20 `LevelChunkWithLight`) in the same client tick as `Respawn` in the vanilla run (200) and one tick later in Null's (1703). Both clients send `PlayerLoaded` in the tick the batch arrives (vanilla method: `LevelLoadTracker`), so Null's `PlayerLoaded` and the fall that follows are one tick behind: Null 1703 `MovePlayerPos` y 200.1, 1704 y 200.0216, 1705 y 200.0 on ground, against vanilla 200 / 201 / 202 with the same values. The sequences are equal shifted by one tick. Not a Null difference; it would need the chunk batch as an anchor (many packets per tick, left for VD2).

### Review of the earlier VD1b edits

No defect needed a code fix beyond the row order above; two functions lacked a spec link (`effect_row`, `Ctx::run_act`) and got one (comment only). `cargo check -p null-abilitycheck` clean, differ 20 tests pass.

### Grim run (Paper + GrimAC, port 25568)

`null-abilitycheck --scenario A15,A17 --account vd1b_grim` once, after the row-order change: 18 scenarios, 18 passed, 0 failed, every row line ends "no Grim alerts" (`~/Desktop/null-spikes/run-logs/vd1b/grim_a15_a17.log`).

### For VD2

- Chunk batch as an anchor, or a rule that treats a one-tick shift of a whole equal sequence as timing, to close finding 4 and 1 without reading by hand.
- A tick-phase check in the differ for packets that arrive within 2 ms of a tick end (finding 2): decide which tick they count for from the time, as the anchors already do.
- Rows still outside the pipeline: the three A17 disconnect rows (Null's refusal has no vanilla counterpart), and Grim alert lists (VD3).

## VD2a: actions, rows A1 to A4 (2026-11-01)

Spec: `specs/plans/VANILLA_DIFF_PLAN.md` VD2. The same pipeline as VD1b over the 15 rows of A1 (select slot, offhand swap), A2 (timed breaks, a client refusal, a chain of 30, a cancel), A3 (break and pick up) and A4 (six placements). Null runs `--scenario A1,A2,A3,A4` once on the vanilla server through the tap (`~/Desktop/null-spikes/run-logs/vd2a/null_a1_a4.log`, 15/15); each row is then replayed once in a fresh vanilla client and diffed. A Grim run of the same modules with the log on follows (`grim_a1_a4.log`).

### Parts added

- **Fork patch 46 `input-log-actions`** (`input_log.rs`, one-line notes in the places that send; commit `19e61ed`; `AZALEA_PATCHES.md` row 46). The sample line has `"attack"`, the attack key held in that tick. Discrete actions are `{"ev":..,"tick":N}` lines before the sample of tick `N`: `use_click`, `use_item`, `attack_click`, `swap_offhand`, `open_inventory`, `click` (`win`, `slot`, `button`, `type`), `close`. They are written where the fork sends the action, so a click the client refuses (out of reach, no screen) leaves no line: the bot sent nothing, and the replay sends nothing either.
  - **Attack key.** The fork models mining as a held button: a break starts from `continueDestroyBlock` and adds no progress in its start tick (`SkipStartTickProgress`). So the log has no click for a break. The key is held in a tick when a break starts, when mining goes on, and while the 5-tick destroy delay counts down (vanilla's `MultiPlayerGameMode#continueDestroyBlock` decrements `destroyDelay` only while the key is held, so a release there would stretch the chain). A cancel (`StopMiningQueued`) lets the key go: vanilla then calls `stopDestroyBlock` and sends the abort, as Null does.
  - **Drop key.** The fork has no drop-key path; the bot drops through container clicks (`Throw`, or a click outside the window), which the `click` line covers. Nothing to record for the key.
  - **Not reproducible by a key.** `use_item` (`UseItemQueued`: Null sends `UseItem` whatever the crosshair shows) is logged as it is; the replay presses the use key, so with a block in view the diff will show the difference. No A1-A4 row uses it (A5 and later do).
- **Replay mod** (`Replay.java`). Per tick: the attack key follows `attack`; a `use_click`, `swap_offhand`, `open_inventory` or `attack_click` adds one click to that key's counter (`KeyBindingAccessor`, vanilla `KeyMapping#clickCount`) and leaves the key up, as a key event between two ticks does, so `Minecraft#handleKeybinds` runs `startUseItem` and the rest itself. `MouseAccessor` sets vanilla's "mouse grabbed" flag: `Minecraft#continueAttack` treats the attack key as up without it, and a window that never had focus (the GL stub) never grabs the mouse. The crosshair needs no extra step: `Minecraft#tick` calls `GameRenderer#pick` itself before `handleKeybinds`, after the mod has set the look at the head of the tick.
  - **Container clicks and the tick.** `click` calls `MultiPlayerGameMode#handleInventoryMouseClick` (`clickSlot`) on the open screen's menu and `close` calls the screen's `close()`, both at the head of the tick, before vanilla's tick body. In vanilla a screen click is frame input between two ticks, so its packet follows the last `ClientTickEnd` and comes before the new tick's input packets; Null's click is sent when its host applies the command, which is expected to be before that tick's position packet (not checked live, no A1-A4 row has a window), and the log writes the event with the tick of the next sample. The open (`open_inventory`) is a key press handled inside the tick, so the first click can come in the tick after, which is Null's `ScreenJustOpened` rule. Untested live: no A1-A4 row opens a window (A9 and A13 in VD2b).
- **Differ.** `load_input` keeps the `ev` lines with the sample that follows (`describe` shows `ATTACK` and the events in the inputs line). New `rebase_seq`: the block-prediction `seq` of `UseItemOn` and `PlayerAction` (vanilla `ClientLevel#getBlockStatePredictionHandler`) is a counter of the connection, and Null's row bot has run other rows before the window while the replay is a fresh connection (the first run of A2 to A4 showed Null seq 3, 5, 7, 67, 68 against vanilla 1). Both sides are counted from the first predicted action of the window; a `seq` of 0 stays 0. The report notes when the clientbound packets of the differing tick are equal in both runs. 23 tests (3 new, with negative controls).
- **Scenes.** `--prepare-scene A1..A4:ROW` runs the row's own setup (`a1_scene`, `a2_scene`, `a3_scene`, `a4_scene`, shared with the scenario code). No A1-A4 row has an RCON action inside its window (RCON only reads there), so there are no acts. `rows.json` has the 15 rows; none is skipped.

### Result

| Row | Result |
| --- | --- |
| A1 select_slot | MATCH (12 ticks) |
| A1 swap_offhand | MATCH (12) |
| A2 oak_log_by_hand | MATCH (69) |
| A2 oak_log_iron_axe | MATCH (17) |
| A2 stone_iron_pickaxe | MATCH (16) |
| A2 refused_by_client_break_out_of_reach | MATCH (6, nothing sent on either side) |
| A2 chain_of_30_breaks | DIFF 145 of 491 ticks: vanilla sends a `Swing` in each destroy-delay tick, Null none (finding 1) |
| A2 cancel_break | MATCH (36) |
| A3 pickup_drop | MATCH (39) |
| A4 place_top_face | MATCH (14) |
| A4 place_side_face | MATCH (11) |
| A4 sapling_on_grass | MATCH (14) |
| A4 refused_by_server_entity_in_the_way | DIFF 1 of 13: Null sends a `Swing` after `UseItemOn`, vanilla none (finding 2) |
| A4 refused_by_client_out_of_reach | MATCH (12, nothing sent on either side) |
| A4 refused_by_server_no_support_sapling_on_stone | DIFF 1 of 13: the same extra `Swing` (finding 2) |

Logs: `~/Desktop/null-spikes/run-logs/vd2a/` (`null_a1_a4.log`, `null_tap.jsonl`, `inputlog/`, `rows/*_diff.txt`, `rows/*_vtap.jsonl`, `rows_A*.log`, `grim_a1_a4.log`). The MATCH rows are not empty: the A1 swap window holds `SetCarriedItem` and two `SwapItemWithOffhand`, the break rows the start, the swings and the stop, the placements the `UseItemOn` with the hit point to the last digit.

### Findings (first run, 2026-11-01; both fixed, see "Fixes" below)

1. **chain_of_30_breaks: Null does not swing during the destroy delay.** All 145 differing ticks are the same: vanilla has a `Swing` that Null lacks (110 ticks with the `Swing` alone, 29 with a `MovePlayerRot`, 6 with a `MovePlayerPos`); the breaks themselves (start and stop packets, positions, sequence order) are equal for the 30 blocks. Cause: while the key is held and `destroyDelay` is above 0, vanilla's `MultiPlayerGameMode#continueDestroyBlock` decrements it and returns true, and `Minecraft#continueAttack` then calls `LocalPlayer#swing`, which sends `ServerboundSwing` on every call. The fork swings in `continue_mining_block` (while mining) and at the start, but `decrement_mine_delay` does not (`mining.rs`). So between two blocks vanilla sends 5 swings and Null none. The differ labels the first tick "server timing" because an earlier tick had a different clientbound list; the clientbound packets of the differing tick are equal, so it is a client difference. Not a Grim alert so far (no alert in the run); a fix is one `SwingArmEvent` in `decrement_mine_delay` when the delay is above 0 and a break was held, with a sim test. For the lead to decide; not changed here.
2. **Placement that fails on the client: Null sends a `Swing`, vanilla does not.** Rows `refused_by_server_entity_in_the_way` and `refused_by_server_no_support_sapling_on_stone`: the `UseItemOn` is identical to the last digit (block, face, hit point, `seq` after rebase), Null follows it with `Swing`. Cause: vanilla's `Minecraft#startUseItem` swings only for `InteractionResult.Success` with the client swing source; the client's own try (`BlockItem#place`: an entity in the cell, or a sapling that cannot stay) fails, so no swing and the next hand is not tried. `handle_place_block_queued` (`block_actions.rs`) sends `SwingArmEvent` after every `UseItemOn`. The patch 33 prediction already knows the verdict for the cells it can read (`Verdict`), so the fix is to swing only when the client-side result would be a success; the entity case needs the entity check of `BlockItem#canPlace`. For the lead to decide; not changed here. In the entity row the server side differed too (Null got `RotateHead`, vanilla `SetEntityMotion`/`MoveEntityPos*` for the entity in the cell), so the differ says "server timing"; the `Swing` finding rests on the vanilla rule and the packet text, not on that label. Verified (Haiku, 2026-11-01).
3. **Harness note.** The row bot carries the prediction counter from earlier rows (see `rebase_seq`). The `UseItemOn`/`PlayerAction` `seq` values are the only values of this kind in A1-A4.


### Fixes: patches 47 and 48 (2026-11-01)

Fork commits `921e023` `47-mine-delay-swings` (`decrement_mine_delay` swings in a delay tick when the crosshair block is not air and no item is in use, as `Minecraft#continueAttack` does) and `9da5e25` `48-place-swing-on-success` (the swing after `UseItemOn` follows the client result of the place: `Predict` swings, `Nothing` does not; saplings and armor stands are now decidable; an `Unknown` verdict keeps the swing, a known gap, see ledger row 48). Local, not pushed. Selective run of 15 modules (A1 A2 A2S A3 A4 A4C A4S A10 A10E A11 A12 A13 A14 A16 AT), chosen from the map entries of `mining.rs` and `block_actions.rs` (the helper answered `all` because `place_prediction.rs` has no entry and patch 46's notes touch `inventory/mod.rs`).

Result: all 15 A1-A4 rows MATCH. Null ran through the tap once (`~/Desktop/null-spikes/run-logs/p47_48/null_van.log`, 328/330: two A16 jump rows off by 0.03, jumps do not touch these patches; A4 + A16 rerun once with the final code, 22/22, `null_van_rerun.log`); the rows were replayed once each (`rows/`). The first diff of the 14 other rows read the wrong tap file: the run has one connection per helper bot and the row bot's is the one whose `Hello` names the account (here `.2`), so `run_rows.py --null-tap` must name that file, not the base file. The replays were redone with the right file (`rows_wrongtap/` is the discarded first attempt). One row, `refused_by_server_entity_in_the_way`, still differed after patch 48: the stand is an armor stand, and the first version treated its box as "unsure"; it blocks building unless its marker flag arrived, so the final code decides it, and the row matches in the rerun (`rows_rerun/`). Paper + Grim: 330/330, zero alerts (`grim.log`).

| Row | Result |
| --- | --- |
| A1 select_slot, swap_offhand | MATCH (12, 13 ticks) |
| A2 oak_log_by_hand, oak_log_iron_axe, stone_iron_pickaxe, refused_by_client_break_out_of_reach, cancel_break | MATCH (69, 18, 15, 7, 37) |
| A2 chain_of_30_breaks | MATCH (492 ticks; was DIFF 145) |
| A3 pickup_drop | MATCH (38) |
| A4 place_top_face, place_side_face, sapling_on_grass, refused_by_client_out_of_reach | MATCH (14, 12, 13, 11) |
| A4 refused_by_server_entity_in_the_way | MATCH (12; was DIFF 1) |
| A4 refused_by_server_no_support_sapling_on_stone | MATCH (13; was DIFF 1) |

Verified (Sonnet, 2026-11-01): claims match the logs; both patches follow vanilla in the source; the new tests fail without their fix; A22 (patch 48 changes the patch-33 prediction) ran once per server, 7/7 each, zero Grim alerts (`p47_48/verify_a22_*.log`). The two A16 failures are most likely the apex sampling of `apex_row` (y polled over RCON can miss the apex tick; both read low, one 0.0004 over the limit; exact on the rerun and on Grim); not proven. Gaps left, none reached by a current row: the fork counts the destroy delay down when the crosshair is on no block or air (vanilla only inside `continueDestroyBlock`), and skips neither vanilla's `missTime > 0` nor the spear (`PIERCING_WEAPON`) case; an `Unknown` place verdict and `handle_use_block_queued` still swing after every `UseItemOn`; `UnknownWhy::EntityUnsure` is now unreachable.

### Grim run (Paper + GrimAC, port 25568)

`null-abilitycheck --scenario A1,A2,A3,A4 --account vd2a_grim --input-log DIR` once, patch on, no tap: 15 scenarios, 15 passed, 0 failed, every row line ends "no Grim alerts" (`grim_a1_a4.log`); the log holds the same 7 `ev` lines (2 swaps, 5 use clicks).

### Tests

Fork: `test_input_log_actions` (sim) and `events_come_before_the_sample_of_their_tick` (lib); azalea-client 50 lib + 85 sim pass; negative control: without the note in `continue_mining_block` the sim test fails. Differ: `uv run python -m unittest test_vd_diff` 23 tests (new: events with the next sample, `rebase_seq` and its negative control). Null: `cargo build --workspace --all-targets` ok, `null-abilitycheck` 16 tests, boundary ok.

### For VD2b

- A9 (inventory window), A13 (result mailbox), A22 (place prediction): the `click`, `close` and `open_inventory` replay is built and untested live. A5 and later use `use_item` (see above) and `attack_click`.
- Findings 1 and 2 are fixed in the fork (patches 47 and 48, above); patches 46-48 are pushed (`9da5e25`, 2026-11-02, after Pi pair `p48_full`).
- The differ's "server timing" label is a coarse rule (any earlier clientbound difference in the row); the report now says when the tick's own clientbound packets are equal.

## VD2b: inventory window, result mailbox, place prediction, rows of A9, A13 and A22 (2026-11-02)

Spec: `specs/plans/VANILLA_DIFF_PLAN.md` VD2. The VD2a pipeline over 57 rows: A9 (46: 15 click cases and 31 custom-item hash rows), A13 (4) and A22 (7). Null ran each module once on the vanilla server through the tap; each row was replayed once in a fresh vanilla client and diffed. One Grim run of the same modules with the input log on followed. The `click`, `close` and `open_inventory` replay worked live at the first try (A9:pickup_and_place matched).

### Parts added

- **Scenes.** `--prepare-scene A9:ROW`, `A13:ROW`, `A22:ROW` (`scenarios_inventory.rs`, `scenarios_mailbox.rs`, `scenarios_prediction.rs`, each `prepare_row`). The scene code of each row moved into functions that the scenario and `--prepare-scene` share. No row has an RCON action inside its window. A22 `rejected_place_rolls_back` sets a world border; `prepare_done` puts it back when the replay ends. `rows.json` has the 57 rows; none is skipped.
- **Tap.** The tap cut every summary at 240 characters. A `ContainerClick` with its item hashes is longer, so two clicks with different hashes looked equal (found when the `...` ended Null's and vanilla's lines at different places). Serverbound summaries now keep 4000 characters, clientbound ones stay at 240. Only `ContainerClick` and `ClientInformation` were cut in the Null run. This forced a rerun of A9 on the vanilla server (the first A9 replays, 18 rows, were discarded: `rows_truncated_tap/`). The first Null run's `null_tap.jsonl`, used for the A13 and A22 Null side, has 90 cut serverbound lines, none inside an A13 or A22 compare window (the first window starts at tick 4194, the last cut line is at 4126); the A9 diffs come from `a9/`, which has none, so their Null run stayed.
- **Differ** (29 tests, 6 new, each with a negative control). `rebase_state_id`: the `state_id` of a `ContainerClick` is the server's revision counter of the player's menu (vanilla `AbstractContainerMenu#getStateId`), which the connection keeps; Null's row bot is far on, a fresh client is not. The first click of the window is 0 on both sides. `canon_changed_slots`: `changed_slots`, `added_components` and `removed_components` are hash maps, and their Debug order differs from process to process (Rust's seed; vanilla's identity hashes), so the same click printed in another order. Both are harness artefacts, not findings. The server reads them into maps.

### Result

| Rows | Result |
| --- | --- |
| A9 pickup_and_place, split_half, place_one_at_a_time, swap_with_carried, merge_onto_a_stack, quick_move_main_to_hotbar, quick_move_hotbar_to_main, quick_move_armour, swap_hotbar_key, swap_offhand_key, double_click_gathers | MATCH (27 to 54 ticks each) |
| A9 hash_* (31 rows) | MATCH, each with the item's component checksums equal |
| A9 drop_one, drop_stack, drop_carried_outside, drop_one_carried_outside | DIFF 1 tick each: vanilla sends a `Swing` before the `ContainerClick`, Null none (finding 1) |
| A13 instamine_chain_one_break_per_tick_all_done, two_places_on_consecutive_ticks_both_done | MATCH (38, 19 ticks) |
| A13 place_sent_right_after_a_break_both_results_read | DIFF 1 of 36: a mine-delay `Swing` before the `UseItemOn` in Null, after it in vanilla (finding 2) |
| A13 chain_of_breaks_sent_without_waiting_for_acks | DIFF 11 of 269: from the 14th of 16 breaks on, vanilla restarts one break (Abort and Start) and the later breaks shift by one tick; server timing (finding 3) |
| A22 pillar_jump_places_under_feet, bridge_three_blocks, write_precedes_the_physics_step, unknown_family_in_zone_refused, unknown_family_outside_zone_sent_unpredicted, place_beside_a_fence_in_zone_refused | MATCH (73, 84, 21, 15, 17, 16 ticks) |
| A22 rejected_place_rolls_back | DIFF 1 of 21: Null sends `UseItemOn` and `Swing`, vanilla sends nothing (finding 4) |

Logs: `~/Desktop/null-spikes/run-logs/vd2b/` (`null_a9_a13_a22.log` and `null_tap.jsonl` with `inputlog/` for A13 and A22; `a9/` for the A9 rerun with the tap fix; `rows/*_diff.txt`, `*_vtap.jsonl`; `rows_a9.log`, `rows_a13_a22.log`; `rediff_*.log` are the final diffs with the final differ; `rows_truncated_tap/` and `rows_truncated_tap.log` are the discarded first A9 replays; `grim_a9_a13_a22.log`, `grim_inputlog/`). Null on the vanilla server: 57/57 (first run), 46/46 (A9 rerun).

### Findings

1. **Drop clicks: vanilla swings, Null does not.** Four rows: a `Throw` click (drop key in the window) and a click outside the window. In vanilla the click runs `AbstractContainerMenu#doClick`, which drops through `LivingEntity#drop` (client side), and that calls `swing(MAIN_HAND)`; `LocalPlayer#swing` sends `ServerboundSwing` before the `ContainerClick` leaves. The `ContainerClick` itself is equal to the last hash. Cause: Null departure. A fix is one swing command in the drop paths of `mc-adapter`/the fork click code when the dropped stack is not empty. Not changed here.
2. **Mine-delay swing and a place in one tick: order.** `place_sent_right_after_a_break`, Null tick 4530: the attack key is held and a use click comes. Vanilla's `Minecraft#handleKeybinds` runs the use click (`startUseItem`: `UseItemOn`, then its swing) and then `continueAttack` (the destroy-delay swing of patch 47). Null sends the delay swing first. The three packets are the same set. Cause: Null departure in the order of two fork systems (`decrement_mine_delay` runs before `handle_place_block_queued`). The differ called it "server timing" because earlier clientbound packets differ; the clientbound packets of that tick are equal, so it is not timing. Not changed here.
3. **chain_of_breaks: server timing.** The first 13 breaks match to the tick (all packets, same ticks). At the 14th, the server's update of the axe's damage (`ContainerSetSlot`, from the previous break) arrives one tick after the new break started in vanilla, and 5 ticks before it in Null (window ticks 214 and 208: the server answered the previous stop 6 ticks later in the vanilla run). Vanilla's `MultiPlayerGameMode#sameDestroyTarget` compares the held stack with its components, so a changed damage restarts the break (Abort, Start). The fork has the same rule (`mining.rs`: held stack equal to `MineItem`), so the packets differ because the server answered at another time, not because the rule differs. Not proven with a second run (no repeat runs).
4. **Place outside the world border: vanilla sends nothing, Null sends.** `rejected_place_rolls_back`: the clicked block is outside the border (the row sets a border to make the server refuse). Vanilla's `MultiPlayerGameMode#useItemOn` returns FAIL when the clicked block is outside the client's border, so `startUseItem` sends nothing and tries no other hand. Null sends `UseItemOn` and a `Swing`. This breaks the rule that a bot never sends what a vanilla client cannot send; the row's own check ("Refused by the server") shows the row tests the server refusal path, which a vanilla client never reaches here. Cause: Null departure (the place path lacks the border check; `handle_place_block_queued` / `block_actions.rs`). The clientbound packets of the tick are equal. Not changed here.

### Grim run (Paper + GrimAC, port 25568)

`null-abilitycheck --scenario A9,A13,A22 --account vd2b_grim --input-log DIR` once: 57 scenarios, 57 passed, 0 failed, every row line ends "no Grim alerts" (`grim_a9_a13_a22.log`). The input log holds 91 `click`, 46 `open_inventory`, 46 `close` and 14 `use_click` lines, the same counts as the Null run on the vanilla server.

### Tests

Differ: `cd differ && uv run python -m unittest test_vd_diff` 29 tests. Boundary check and `cargo build --release --workspace --all-targets` ok.

### Verification (Haiku, 2026-11-02)

Passed: 57 rows counted (50 MATCH, 7 DIFF), all four findings checked against the vanilla source and the fork, differ mutation checks, spec links. Notes:

- **Refusal rows.** The MATCH on A22 `unknown_family_in_zone_refused` and `place_beside_a_fence_in_zone_refused` has no action packet in the window. It checks that both clients send nothing, as designed.
- **Log labels.** `rediff_*.log` labels findings 1, 2 and 4 "server timing" through the differ's coarse rule. The classification in "Findings" overrides that label.
- **Finding 3.** One reply came 7 ticks late in vanilla (`ContainerSetSlot`, tick 276), and the other 15 came within 0 to 2 ticks in both runs. The label stands, with moderate strength: one run each, and the cause of the late reply is not shown. The "6 ticks" above is the difference of the two delays (7 and 1).
- **Finding 4.** Ledger row 33 already listed the border check as open. The break path has the same gap (`MultiPlayerGameMode#startDestroyBlock`, `mining.rs` TODOs), and no row tests it.


### Fixes (fork patches 49-51, 2026-11-02)

Spec: `specs/reference/AZALEA_PATCHES.md` rows 49-51. The three Null departures of findings 1, 2 and 4 are fixed in the fork (`b20170b`, `03d2317`, `33b9a94`; pushed 2026-11-02 after Pi pair `p51_full`). Finding 3 needed no fix. Null ran the 22 modules the patches touch once on the vanilla server through the tap (`~/Desktop/null-spikes/run-logs/p49_51/null_van.log`, 407/407, zero alerts), and the changed rows were replayed once each in a fresh vanilla client (`rows.log`, `rows/`).

| Finding | Patch | Replay result |
| --- | --- | --- |
| 1. drop click: no `Swing` | 49 `drop-click-swings` | A9 drop_one, drop_stack, drop_carried_outside, drop_one_carried_outside: MATCH (27, 28, 35, 46 ticks) |
| 2. delay swing before a place | 50 `delay-swing-after-use` | A13 place_sent_right_after_a_break: MATCH (35 ticks) |
| 3. chain_of_breaks | none | MATCH (268 ticks). The same fork rule gave 11 differing ticks in VD2b, so the earlier difference came from when the server answered, as found. This is a second run, not a proof of the reply delay |
| 4. place outside the border | 51 `world-border-client-check` | A22 `place_outside_border_refused_on_client` and `break_outside_border_refused_on_client`: MATCH (20 and 34 ticks; the break row has a control break inside the border) |

Also replayed, to see that nothing else moved: A13 two_places_on_consecutive_ticks and instamine_chain (MATCH), A22 pillar_jump, bridge, write_precedes_the_physics_step, unknown_family_in_zone_refused, unknown_family_outside_zone_sent_unpredicted, place_beside_a_fence_in_zone_refused (MATCH). The 16 rows: 15 MATCH on the first pass, and the break-border row differed (below).

- **A22 `rejected_place_rolls_back` is replaced.** It made the server refuse a place by putting the clicked block outside the border, which a vanilla client refuses itself, so the fork now sends nothing there. The new row `place_in_spawn_protection_refused_by_server` makes the server refuse a place inside spawn protection instead (replay: MATCH, 34 ticks, 2 `UseItemOn`, both acked). Two border rows (`place_outside_border_refused_on_client`, `break_outside_border_refused_on_client`) check the client refusal. Snap-back of an overlapping bot has no live test (a client refuses a place into its own box); the write-and-rollback logic keeps its fork simulation tests.
- **Spawn protection and the border scenes (2026-11-02).** Both servers now run with `spawn-protection=4`. With a world spawn outside the border, the effective spawn is the border centre (`Level#getWorldBorderAdjustedRespawnData`), so the old border scene put the break row's control block (x 1049) inside protection. `border_scene` now uses centre x 1044 and size 13 (border x 1038 to 1051) and brings other bots inside; protection covers x 1040-1048, and the control at x 1049 is free. Final replay of the two border rows (`~/Desktop/null-spikes/run-logs/p49_51/resume/rows.log`): both MATCH (19 and 34 ticks). The replay needed a Null run through the tap, so A22 ran twice on the vanilla server that day (9/9 and 9/9); the first run of the pair had no tap.
- **A wrong fix caught by the replay.** The first version of patch 51 sent one `Swing` for a break outside the border, because `Minecraft#startAttack` swings whatever `startDestroyBlock` answered. The vanilla replay sent none. The replay holds the attack key (as the fork models a break start), so `Minecraft#continueAttack` runs, and it swings only when `continueDestroyBlock` answers true. A mouse click would swing. The patch now sends nothing, the sim test and B6 §4.10 say so, and the patch commit was amended (`d601248` to `33b9a94`). Only A22 was rerun (vanilla 8/8, both border rows MATCH).
- **Fail-without proofs** (test runs only, in scratch worktrees, deleted): patch 49 `a_drop_click_swings_before_the_container_click` fails with `["ContainerClick"]` against `["Swing", "ContainerClick"]`; patch 50 `a_place_goes_out_before_the_destroy_delay_swing_of_the_same_tick` fails with `["Swing", "UseItemOn", "Swing"]` against `["UseItemOn", "Swing", "Swing"]`; patch 51 `world_border_like_vanilla` (2 tests) fail with `block_in_world_border` always true (1 `UseItemOn`; 1 action and 2 swings). A scratch test run that reused another worktree's cached build once showed a false pass; each proof was run again after touching the crates.

## VD3 formats

Spec: `specs/plans/VD3_BRIEF.md` T1. The files a vanilla-diff run leaves, field by field. Other tools (offline trace replays) read them as they are. A change to a field is agreed first. Nothing in the VD0 to VD2 formats changed in T1; additions are marked "VD3".

### Input log (`DIR/<account>.jsonl`, fork patch 45 and 46; JSON lines, UTF-8)

One file per bot account, written by the fork when the app has an `InputLogDir` (`null-abilitycheck --input-log DIR`). Line kinds, in file order. `tick` is the bot connection's client tick counter from 0 (the number of `ClientTickEnd` packets sent before it), the same count the tap uses.

- **Sample**, exactly one per client tick, written where `LocalPlayer#aiStep` has just read the keys: `tick` (int), `f`, `b`, `l`, `r`, `jump`, `sneak`, `sprint`, `attack` (bool, key held in that tick), `yaw`, `pitch` (float, degrees), `slot` (int 0 to 8, hotbar).
- **Event** `{"ev":NAME,"tick":N}`, before the sample of tick `N`, for a discrete action the fork sent that tick. `NAME` is `use_click`, `use_item`, `attack_click`, `swap_offhand`, `open_inventory`, `click` (extra fields `win` int window id, `slot` int, `button` int, `type` int click type) or `close`. An action the client refused leaves no line.
- **Mark** `{"mark":LABEL,"tick":N}`, before the sample of tick `N`. Labels:
  - `start:<ID>:<name>` and `end:<ID>:<name>`: the row `ID:name` runs in ticks [start, end).
  - `act:<label>`: the row ran the RCON act `<label>` at this tick (the replay runs it at the same replay tick).
  - **VD3** `logoff:start:<ID>:<name>:<N>` and `logoff:end:<ID>:<name>:<N>`: `<N>` is the byte length of the server's `server.log` when the row's window opened (before the start mark's 100 ms wait) and when it closed (after the 300 ms settle and the log scan). The row's own window in the server log is bytes [start N, end N). The label is split from the right, so a row name may hold `:`. Written only with `--input-log` (`Ctx::run_allowing`).

### Tap log (`vd-tap --out FILE`, JSON lines; unchanged since VD2b)

One line per decoded packet, both directions: `t_ms` (int, ms since the tap started), `tick` (int, count of serverbound `ClientTickEnd` seen so far; the tick-end packet carries the tick it ends), `state` (`Handshake`, `Status`, `Login`, `Config` or `Game`), `dir` (`S` serverbound or `C` clientbound), `name` (the packet's type name), `summary` (Rust Debug text of the packet; at most 4000 characters serverbound, 240 clientbound). The differ reads only `Game`; the Login `Hello` (`name: "<account>"`) names the player. A second connection through the same tap goes to `FILE.2`; `FILE.tap.out` holds the tap's own stderr.

### Replay marks (`<stem>_marks.json`, written by the mod)

`{"replay_start_tick": int, "ticks": int, "first_input_tick": int}`. `replay_start_tick` is the vanilla client tick of the first replayed input; `first_input_tick` is the input log tick it came from. The offset of a row is `replay_start_tick - first_input_tick` (vanilla tick = Null tick + offset). `ticks` is the number of replayed samples. `<stem>_vtap.jsonl.progress` holds the act labels the mod reached, one per line, then `end`.

### Window of the vanilla replay (VD3, `<stem>_vtap.jsonl.logwin`, JSON, written by `run_replay.sh --server-log`)

`{"player": str, "log": str, "start": int, "end": int}`: the vanilla player's name (`VD<MMDDhhmmss>`, never op), the server log path, and the log's byte length when the control file was touched (scene done, replay about to start) and 1 s after the client quit (Grim writes an alert a moment after the packet).

### Packet verdict (VD3, `<stem>_packets.json`, written by `vd_diff.py --json`)

`verdict` (`MATCH` or `DIFF`), `ticks_compared` (int), `ticks_differing` (int), `classes` ({label: count}, labels `client` or `server timing`), `tick_end_problems` (list of [tick, Null count, vanilla count] where a run did not have one `ClientTickEnd`), `first` (only on DIFF: `tick`, `vanilla_tick`, `label`, `null` and `vanilla` as lists of [packet name, summary]), `null_window` ([start, end) Null ticks), `vanilla_offset` (int), `notes` (anchor lines). `<stem>_diff.txt` is the same in words.

### Alert lists and row verdict (VD3, `<stem>_verdict.json`, written by `run_rows.py`)

`row` (`ID:name`), `packets` (the object above; absent when the replay failed), and `alerts` when `--server-log` is given:

- `null_player`, `vanilla_player`: the names the alerts are counted for (whole-word match).
- `null_log`, `vanilla_log`: [log path, start byte, end byte] of each window.
- `null`, `vanilla`: {check name: number of alert lines of that player in the window}. A line is `Grim » <player> failed <Check> (xN) ...`, colour codes stripped; each line counts once, the `(xN)` violation count is ignored (as `scan_log` in `scenarios.rs`).
- `allowed`: {check prefix: reason} from the row's `allowed_alerts` in `rows.json` (alerts a vanilla client draws too, F5 §4.6); empty for A1 to A4 and A22.
- `checks`: per check seen on either side, {`class`: `null_only` (a finding), `shared` (both drew it, recorded) or `vanilla_only` (reported), `null`: int, `vanilla`: int, `allowed`: reason, only if allowed}. An allowed alert stays in the list and is labelled in the report.
- `finding`: true when some check is `null_only` and not allowed.
- `error` instead of the above when a window is missing (no `logoff` marks, no `.logwin`, or the log was replaced).

`rows.json` row fields: `scene` (the `--prepare-scene` argument), `anchors`, `stop_at_anchor`, `skip` (reason; row not run), and VD3 `allowed_alerts` ({check prefix: reason}).

### Use

`harness/vanilla-diff/run_vd3.sh --server vanilla|grim --modules A1,A2,A3,A4,A22 --out DIR --account null_NAME [--rows ID:name,..] [--skip-null]` (about 15 minutes for 24 rows: run with `nohup`). It checks that no other `null-abilitycheck` runs, that RCON is up, and that `ops.json` lists no `VD` player; runs the modules through the tap; replays and diffs each row; on `grim` it also reads and compares the alerts. `DIR/summary.txt` has one line per row. `run_rows.py` alone replays rows of an earlier Null run (`--diff-only` re-diffs). Exit code 0 only if every row matches and no row has an unallowed Null-only alert.

## VD3: T1, one command for the replay and the alert diff (2026-11-03)

Spec: `specs/plans/VD3_BRIEF.md` T1. Design chosen: extend `run_rows.py` (plus a thin `run_vd3.sh` and `differ/vd_alerts.py`) rather than add `null-abilitycheck --vanilla-diff`, because the replay, the scenes and the differ already live in this directory and the only Rust change is the `logoff:` marks (`Ctx::run_allowing`). Formats: "VD3 formats" above.

- Command: `harness/vanilla-diff/run_vd3.sh --server vanilla|grim --modules A1,A2,A3,A4,A22 --out DIR --account null_NAME` (nohup; each leg about 15 minutes).
- Tests: `cd differ && uv run python -m unittest test_vd_alerts test_vd_diff` (45 tests; every new test was shown to fail on a broken `vd_alerts.py`, `vd_diff.summarize` or `row_alerts`). The negative control for "an alert only one side drew" is `RowAlertsTests` and `CompareTests`: a log with one Grim line inside the Null window only (a finding), inside the vanilla window only (reported), on both (shared), and outside both (not counted). It uses synthetic log lines because the Grim log holds no real alert to reuse.
- Logs: `~/Desktop/null-spikes/run-logs/vd3_t1/` (`vanilla/`, `grim/`, each with `summary.txt`, `rows/`).
- Results: vanilla 24 of 24 rows clean (packets MATCH). Paper + Grim 24 of 24 packets MATCH, zero alerts on either side, in 23 rows compared; the last row was not compared because the process exited before the tick that writes its `logoff:end` mark. Fixed by a 100 ms wait after that mark. A22 rerun on Grim once: 9 of 9 clean, all compared.
- Live control: the bot was stopped with SIGSTOP for 3 s inside `A22:bridge_three_blocks` (22:26:25 to 22:26:28, in a row window of 22:26:24 to 22:26:31). Grim drew no alert and the packets still matched, so a stop of that size draws nothing and the run did not exercise a live alert. The comparison was instead checked on real Grim lines from `server.log` (three `Simulation` lines of the `null_check` run): `parse_alerts` reads them, and `compare_alerts` gives `null_only` and a finding when only Null has them, `shared` and no finding when both do. No row has yet drawn a live alert through the whole run; the first T2-T7 row that draws one is the end-to-end proof.
- Vanilla window: it starts at the replay's control file, so it covers the replay and not the client's login or scene setup. An alert drawn during that setup is not counted.
- `run_vd3.sh` now builds `null-abilitycheck` first (the verifier found it could run a stale binary).
- Verification: Sonnet, 2026-11-03; logs against code, 45 unit tests. Live control logs: `vd3_t1/verify_grim/`.
