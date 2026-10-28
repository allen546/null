# Vanilla diff: clean client, tap, driver (VD0)

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
