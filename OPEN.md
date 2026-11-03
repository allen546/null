# Open for Allen: start here next thread

This page holds open items only. Finished work goes in the docs in §5, not here. Standing rules: [CLAUDE.md](CLAUDE.md). Current milestone: [HANDOFF.md](HANDOFF.md). Don't load `specs/logs/` (about 600k tokens of old builder logs).

## 1. Next actions, in order

1. **Vanilla diff VD3** (`specs/plans/VANILLA_DIFF_PLAN.md`): the diff in the harness, Grim alert lists compared. In full, every module, before `mine` (Allen, 2026-11-03).
2. **`mine`:** methods as presets (fishbone, quarry, tunnel; B4 §4.12), sight mode for anti-x-ray servers (B4 §4.11; belief needs `how = seen_sight`), and underground hazards handled by the miner, placing to seal fluids (B4 §4.13).
3. **Bench** ([BENCH_PLAN](specs/plans/BENCH_PLAN.md)): remote server, the 100-bot felling load test, CPU density across machines, the crowd test.
4. **Shared section cache** in the Azalea fork (99% of section building was repeated), then the same bench ladder again.
5. **Bench gaps:** lock-wait and per-thread timing (why the ECS tick waits), the level-off at 22–28 searches per second, re-run 5 threads at 50 bots (suspect value), why 3% of repeated chunks have different bytes.
6. **World transformation tests** T-ACC, T-WE, T-OSC, T-COB (WORLD_TRANSFORMATION §8).
7. Optional: a Docker topology in F3 for distribution logic (HQ, NATS, 2–3 nodes, injected latency and partitions), once F1 and F6 exist.

## 2. Open problem: bots in a swarm share one world state

Found 2026-11-02. **Harm proven 2026-11-02** on chance 1 of 3, Paper + Grim: A broke the block under B, B fell while A's break was pending, and Grim drew `Simulation x4` and `GroundSpoof x1` on B. With separate swarms, the same break drew zero alerts. One row, strongest setup, control not a perfect twin (Result section of the plan). Null runs many bots in one process on one Azalea `World`, to keep memory low. A vanilla client has its own world and writes its block predictions there. Our bots write theirs into the shared `World`, so every other bot sees them.

So bot B can read bot A's block about one round trip before the server's update could reach B. For a refused action, B reads a state that never existed. B could then draw a Grim `Simulation` alert, click a block it could not know about, or decide wrongly. A second leak has the same cause: the first bot to receive a server update writes it for all bots.

No spec weighed this. It matters most for `mine` and building, which put bots side by side. The overlay (patch 53, pushed 2026-11-03, `f994d4d`) fixes A's unconfirmed write. It does not fix the second leak, or a delayed packet overwriting newer shared truth; both stay open and are tested only if harm shows up. Code locations and the result: [SHARED_WORLD_TEST](specs/plans/SHARED_WORLD_TEST.md).

## 3. Waiting on your call, when it comes up

None of these blocks current work. Each has a default.

| item | when it matters | default now | where |
|---|---|---|---|
| Departures from vanilla that draw no Grim alert: fix each one, or log it and fix only on harm. "Fix them all" (2026-11-02) covered the VD2b findings only | VD3 and later diffs | fix, as the current rule says | CLAUDE.md, F5 §4.6 |
| AD-K2.1 commander API: imperative commands or a desired-state manifest. Bake-off ran 2026-10-14: no decisive difference | before the LLM commander | manifest hybrid | K2, DECISIONS, `specs/reference/BAKEOFF_K21_2026-10-14.md` |
| AD-C3.1 central depots or storage per operation | large logistics | per operation, depots when shared | C3 |
| AD-C7.1 fights bigger than one squad | large fights | bigger squads first | C7 |
| AD-C8.1 bots talk to people in v1 | first public server | silent | C8 |
| AD-C9.1 one convoy's escort across two nodes | multi-node convoys | never; split trains | C9 |
| AD-F6.1 third bus quorum member | several always-on hosts | defer | F6 |
| World transformation §7 items 1–4: which jobs merge into one service; the isolation shell as the basis of the guarantee; trust pack claims or check once per version; refuse unorderable packs or allow some search | builder build-out | proposals as written | proposals/WORLD_TRANSFORMATION.md §7 |
| World transformation §7 item 6: access planning (placement options, strict layers, scaffolding temps) for large technical builds | after test T-ACC | adopt; devise something else if unsafe | same, §5.3, §8 |
| Builder execution model: packet replay, skills or a hybrid | builder build-out | deferred | same, §7 |
| A rule for conditional mob states (wither, shulker) | when a third case appears | none | GENERALIZATION §0.5 |
| AD-B2.1 after a death: recovery against quarantine, and whether a death is a hard interrupt | step 2 design | working rule: go back to work, or escalate | DECISIONS |
| When the commander's load justifies LLM officers | when the commander's context or wake budget overflows | no officers (AD-K5.1) | K5, GENERALIZATION §0.6 |

Nothing else blocks building: all 22 architecture decisions are decided (16) or deferred (6), and unflagged §9 questions in the component specs are measurements or details, deferred to the build by rule (CONVENTIONS, Flags).

## 4. Known gaps and unexplained events

None of these fails a row today. Each stays here until it is fixed or closed.

**Parity gaps**

- A break can report `Done` when another actor replaced the block: `settle_sent_block` (`azalea-client/src/plugins/block_actions.rs`) counts any change from the old block as accepted. Seen live on both servers by the suite-speed session's stall test (run st2): the bot said `Done` while the server held bedrock. Details: `SUITE_SPEED.md` on the `faster-suite` worktree. Not fixed; a fix is a later fork patch, and it is Allen's call.
- An `Unknown` place verdict and a bot not fully set up still swing (ledger row 48). The block-use path has its own swing note there, and the fork tries only the main hand.
- Throw-all in a window (ledger row 49): vanilla loops while the slot holds the same item and stops on `canDropItems` (`AbstractContainerMenu#doClick`); the fork gives one swing and models neither. No A9 row exercises it.
- The `Timer` alerts of the first p49-51 Grim run (404/407) have no proven cause. A2S `break_chain_y` and A10 `component_items` had tick stalls with catch-up ticks. A4S `place_bed_both_parts` had neither (longest gap 65 ms). The rerun on a quieter Mac passed 239/239 with zero alerts, which proves nothing about the cause. Logs: `~/Desktop/null-spikes/run-logs/p49_51/`.
- Chunk re-sent during a pending click (ledger row 53, edge a): vanilla writes the retained server state at the ack (`ClientLevel#syncBlockState`) even over a block that a chunk packet just set; the overlay shows the packet's block. Occurs only when a chunk packet for the bot lands between its click and the ack, and a pending cell's block changed. Fixing it needs an override that outlives the ack.
- `azalea/src/auto_tool.rs:42` and `azalea/src/container.rs:80` still read shared truth; Null does not call them.
- Place prediction stages 2 and 3 (B6 §4.10): 722 block families are `Unknown`.
- Fork gaps listed in ABILITY_CHECK "p44 results": `water_fluid_height` uses own height, `travel.rs` patch 38 reads the shared chunk map, `is_face_sturdy` covers full cubes only.
- `ItemInUse::server_ticks` keeps an inherent error of about one tick either way under load. The shield belief errs long, so it stays safe (F5 §4.6, B5 §4.5).
- Later: a stubbed vanilla client on Paper + Grim, to compare alerts one for one instead of proving each allowed alert by trace (VD3 may cover this; `specs/reference/grim-a23-resync-loop.md`).

**Test gaps**

- `cargo test -p azalea --lib pathfinder::tests` fails 11 of 11 since patch 43 (`6d5d0a4`; 42 passes). Test setup only: `InLoadedChunk` now needs the bot's own chunks, and `azalea/src/pathfinder/simulation.rs` gives the simulated player an empty `PartialWorld`, so it never ticks. Live bots fill theirs from chunk packets, so they and `mine` are not affected (cause found 2026-11-03; Null's own crates not checked). Fix tried on a copy at `f994d4d`: fill that partial from the shared chunks, `azalea` lib pathfinder tests 31 of 31. Not applied: it becomes the next fork patch. Diff: `~/Desktop/null-spikes/run-logs/pathfinder_probe/fix.diff`. Patch 43's check did not include the `azalea` lib tests, which is why this went unseen.
- The A17 `slow_falling_arrow` vanilla-diff replay used the old summon and needs a new replay. The VD1b match has 8 packets, and its window ends before the effect.
- The A14 left-on-leaves accept path has not run live (no drop has landed on leaves).
- `push_tick_order` covers only "bot ticks first" live; the other half is a fork simulation test.

**Seen, cause unknown**

- `Couldn't set chunk data: failed to fill whole buffer. World height: 384` in A12 `death_in_the_nether` on Grim, seen in several runs. The row passes. Likely a nether chunk decoded with the overworld height around the dimension change; not looked into.
- Grim keeping an old floor for over 1 s after a `fill` (`p31_repro`, 6 `Simulation` alerts). Not reproduced in 28 probes; `clean_in` no longer removes the floor. The trace tools are on the Pi (`bin/null-inv`, `run_inv.py`). The INV row `setblock_11` left the bot at y 200.1 on all three servers, not looked into.
- A10E `chest_minecart_behind_a_block_refused` failed once on vanilla; 0/30 in repro. It logs each term now.
- `immediate_respawn_has_no_delay` once hung 60 s on Grim in `revive` (harness race, not examined). A22 flake, not reproduced.
- RCON reply shift: cause probable, not reproduced. A dropped packet prints `rcon: dropped a stale packet`.
- The Grim warm-up sometimes draws alerts (`p45_full` 10, `p39q` 3; not scored). `p48_full` drew 0.

**Open from S3 and S6:** `specs/plans/BUILD_PLAN.md` S3 "Done 2026-10-24" (no un-hold command, renews must not block the squad tick, bot subject helpers move out of `null-simbot`, a real bot must honour `Lease`, four low items), and `specs/plans/S6_BRIEF.md`.

## 5. Where things are

| what | path |
|---|---|
| finished work moved off this page | `specs/history/OPEN_HISTORY.md` |
| spec index and counts | `specs/INDEX.md` |
| both sides of every decision | `specs/DECISIONS.md` |
| decisions log by date | `specs/history/DECISIONS_LOG.md` |
| generalization shape | `specs/proposals/GENERALIZATION.md` §0 |
| builder redesign | `specs/proposals/WORLD_TRANSFORMATION.md` |
| benchmark report | `specs/reference/BENCH_AZALEA_2026-10-15.md` |
| commander bake-off report | `specs/reference/BAKEOFF_K21_2026-10-14.md` (code and logs: `~/Desktop/null-spikes/k21-bakeoff`) |
| bench code and raw data | `~/Desktop/null-spikes/azalea-bench` (`data/`, `scripts/queue.py`) |
| build plans and milestone status | `specs/plans/BUILD_PLAN.md`, `specs/plans/COMBAT_SCAFFOLD.md` |
| current milestone hand-off, how to run the ability check | `HANDOFF.md` |
| swarm code | `crates/` (Cargo workspace at repo root); release build `cargo build --release -p null-node` |
| belief store, skills (`gather`, tree policy) | `crates/null-belief`, `crates/null-skills` |
| action mailbox | `crates/null-bot-api/src/action.rs`, `crates/mc-adapter/src/actions.rs`, spec B6 §4.6 |
| ability check (runner, scenarios, results of each Pi pair) | `crates/null-abilitycheck`; plan `specs/plans/ABILITY_CHECK.md` |
| vanilla diff (plan, results per phase) | `specs/plans/VANILLA_DIFF_PLAN.md`, `harness/vanilla-diff/README.md` |
| run logs | `~/Desktop/null-spikes/run-logs/` |
| skeleton test server | `harness/skeleton-server` (scripts and README; the server world and match runs stay local and untracked); `~/Desktop/null-spikes/skeleton-server` links to it |
| Paper + Grim test server | `harness/grim-server` (README; world untracked) |
| Azalea fork | public `allen546/azalea`, branch `null-main` (1.21.11 + 38 cherry-picks + bench timers + our patches, nightly-2026-10-12); local clone `~/Desktop/null-spikes/azalea`; Null pins a commit in `Cargo.toml`; ledger `specs/reference/AZALEA_PATCHES.md` |
