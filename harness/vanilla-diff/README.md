# Vanilla diff: clean client, tap, driver (VD0), replay and differ (VD1a)

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
- **`run_replay.sh`**: starts the tap, the client and the scene, touches the control file, waits for the client to quit. **`run_null.sh`**: tap plus `null-abilitycheck` with the input log. **`differ/vd_diff.py`** (`uv run`, tests: `cd differ && uv run python -m unittest test_vd_diff`, 8 tests with a negative control): aligns the windows by tick (`replay_start_tick - first_input_tick`), compares serverbound game packets in order (summary strings, so coordinates must be equal), reports the first difference with both packet lists, the clientbound arrivals and the inputs around it. A difference at or after the first different clientbound arrival (timer packets excluded) is labelled "server timing". Ignored serverbound packets, each with its vanilla sender: `KeepAlive` (`ClientCommonPacketListenerImpl#handleKeepAlive`), `Pong` (`#handlePing`), `ClientInformation` (`Options#broadcastOptions`), `ChunkBatchReceived` (`ClientPacketListener#handleChunkBatchFinished`). `ClientTickEnd` is checked as one per tick.

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
