# Vanilla diff: compare Null with a real vanilla client

Plan, 2026-10-25 (Allen: "plan option 1"). Status: planned, decisions taken, not started. Open item (g) in OPEN.md.

## Why

Null's rule is "identical to vanilla" (CLAUDE.md, F5 §4.6). Today we find a difference only when a scenario happens to exercise it and Grim or an RCON check complains. Of the 41 fork patches, 21 add a vanilla rule Azalea did not model and 13 fix Azalea bugs; nearly all were found that way, one at a time. An audit finds only the class it looks for. A reference finds differences we did not think of: the same inputs go into a real vanilla client and into Null, and the packets must match.

This also replaces trace proofs. Today an alert a vanilla client "would draw too" is allowed only with a trace that names the vanilla rule (F5 §4.6). With the diff, we measure it: the vanilla client runs the same row and draws the same alert, or not.

## The idea

For one ability-check row:

1. RCON sets up the scene (as today).
2. Null runs the row. The fork records the bot's **input** per client tick: movement keys, jump, sneak, sprint, look (yaw, pitch), hotbar slot, and each click (attack, use, with the hand). This is what a player does, not what the client sends.
3. RCON resets the same scene. A real vanilla 1.21.11 client, with a small Fabric mod, joins and **replays the same input**, tick by tick, through vanilla's own input path (key mappings and `Minecraft#startAttack`, `#startUseItem`, `#continueAttack`). Vanilla itself decides what to send.
4. A packet tap between each client and the server logs every serverbound packet, marked by client tick. 1.21.11 clients send `ServerboundClientTickEndPacket` at the end of each tick, so ticks align without guessing.
5. A differ compares the two logs tick by tick and reports the first difference with context (the tick, both packets, the inputs around it). On Paper + Grim it also compares the two alert lists.

Input level, not packet level, is the point: if Null's physics, click checks or timing differ from vanilla, the packets differ.

## What can differ that is not a bug

- **Server timing.** Block updates, pongs and entity moves arrive at different client ticks in the two runs, and vanilla reacts to them. The tap also logs clientbound packets with the client tick they arrived in; the differ stops at the first difference that follows a different server arrival, and reports it as "server timing", not as a bug.
- **Mobs and randomness.** Rows with mobs, drops or other random server behaviour diverge for reasons outside the client. Start with static-world rows.
- **Packets on timers.** Keep-alive replies, client information, chunk-batch acknowledgements. The differ ignores a short, fixed list of these; each entry names the vanilla method that sends it.

## Phases (each one Sonnet task, one done-when)

- **VD0, reuse the working client.** Allen already runs headless 1.21.11 Fabric clients: `~/Documents/antigravity/fearless-pascal` (Mac; a copy at `/mnt/pi-data/fearless-swarm` on the Pi): HeadlessMC 2.10.0, a companion mod (`mod/`, Fabric Loom, Yarn `1.21.11+build.6`), memory work (about 550 MB RSS per client, `reports/MEMORY_OPTIMIZATION_REPORT.md`), offline login (its server runs `online-mode=false`, as ours do). For the diff the client must behave as vanilla, so VD0 builds a clean game directory from it:
  - **Drop** every mod that changes what the client does: Meteor Client, Baritone, Nora Tweaks, Trouser Streak, the swarm Baritone mod.
  - **Check, then keep or drop:** Krypton (network-stack changes; must not change packet content or flush timing per tick), `hmc-specifics` (HeadlessMC's helper).
  - **Judge each companion-mod mixin against vanilla:** `ChunkLoadProgressMixin` sends `PlayerLoaded` at tick 1, where vanilla waits for its chunks, so it changes what is sent and must go or be matched on the Null side; `GameRendererMixin` cancels the render passes, so VD0 must confirm the crosshair pick still runs each tick (vanilla `GameRenderer#pick`), or the click rows are wrong; `HudMixin` and `FontsMixin` are display only.
  - **Replace** the HTTP control with the replay driver (VD1) and the tap.
  - Done when the clean client logs in to `skeleton-server`, a minimal driver walks forward 20 ticks, and the tap log shows 20 tick ends and the expected `MovePlayer` packets. Note how fast it runs on the Pi.
- **VD1, movement.** A Null-only fork patch `input-log` (records the input per tick, adds nothing to what is sent). The record format (JSON lines). The replay mod for movement, look, jump, sneak and sprint. The differ. Done when A15 (block movement effects) and A17 (Levitation, Slow Falling) rows match vanilla packet for packet, or each difference is a reported finding.
- **VD2, actions.** Clicks, hotbar, item use, block break and place, containers and crafting. Done when A1–A4, A9, A13 and A22 rows match, or each difference is a reported finding.
- **VD3, Grim and the harness.** `null-abilitycheck --vanilla-diff` runs the diff after each chosen row. The Grim alert lists are compared, and an allowed alert in F5 §4.6 is backed by the vanilla run instead of a trace. Done when every static-world module runs under the diff on both servers.

Later: rows with mobs and other players (needs the server-timing rule to be good), and a run of the diff in each Pi full pair.

## Decisions (Allen, 2026-10-25: "go with your defaults")

1. The mod and the tap live in `harness/vanilla-diff/` in the Null repo (Fabric mod in Java, like `harness/vanilla-hash/`; the tap in Rust). The mod is our own code. The Mojang client jar is used locally and never committed.
2. The tap may import `azalea-protocol`, as a harness tool listed in `scripts/check_boundary.sh`; it never runs in a bot.
3. Mac first; the Pi pairs later.
4. After S6, before `mine`.
5. The login plugin was for a real server; the offline test servers do not need it.

## Risks

- The headless client already runs (Allen's setup). The risk left is the changes it makes to stay light: each one must leave the packets vanilla sends unchanged (VD0).
- Vanilla's crosshair pick and some input handling run per rendered frame, not per tick. The mod must drive input at tick boundaries and render as few frames as it can. VD1 checks whether frame count changes the packets.
- The replay is only as good as the input record. If the fork records the wrong input, both sides agree on the wrong thing. VD1 includes a hand-written input file (walk, jump, turn) with packets checked by hand.
