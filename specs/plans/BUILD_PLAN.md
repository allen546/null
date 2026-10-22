# Build plan: the Null swarm

**Plan, 2026-10-16.** Scaffolds the whole swarm in the order of ARCHITECTURE §7. Step 1 (the skeleton) is broken into milestones S1–S6; later steps are outlined and get milestones when they come up. Implementation is done by Sonnet subagents. Each milestone is cut into tasks that each finish well under an hour (for example: test server, one crate and its tests, the binary, the live run); no single subagent runs for an hour or more (Allen, 2026-10-16). Tasks are sized by importance and estimated lines of code, aiming for 500–800 lines each. S4 was one task: about 2,000 lines plus a cold Azalea build took 67 minutes. Each milestone is checked by a separate verifier subagent: Haiku for builds and unit tests, Sonnet for anything live. The bar is that bots act correctly in-game (Allen, 2026-10-16). If the current docs disagree with this plan, the docs win.

## Layout

- Code lives in this repo: a Cargo workspace at the root, crates under `crates/`, tools under `tools/`, scripts under `scripts/`. `specs/` stays the single source of truth for documents.
- The root `rust-toolchain.toml` pins `nightly-2026-10-12`, the Azalea fork's toolchain. Decision crates must also build with `cargo +stable`. `scripts/check_boundary.sh` checks that, and checks that no crate outside the allowlist (`mc-adapter` and the binaries that link it: `null-node`, `null-abilitycheck`, `null-duel`; the planned `mc-nav` and `mc-act` were dropped 2026-10-17, their raw capability goes in the fork) depends on `bevy*` or `azalea*` (F5 §4.1).
- The Azalea fork is a git dependency on the public fork `allen546/azalea`, pinned by commit in `Cargo.toml` (local clone `~/Desktop/null-spikes/azalea`).
- Test servers: `harness/skeleton-server/` (vanilla, S4; port 25567, RCON 25577) and `harness/grim-server/` (Paper + GrimAC; port 25568, RCON 25578) keep their scripts and READMEs in the repo, with worlds untracked; `~/Desktop/null-spikes/combat-server/` (flat, PvP; port 25566, RCON 25576) stays outside. The bench server keeps 25565.
- Nothing is committed without Allen asking.

## Step 1: skeleton (ARCHITECTURE §7.1)

The goal: all four levels (HQ, one operation, one squad, two bots) run in one process and talk through the real message types. A CLI commander sets `Obtain(oak_log, 64)`, and both bots split the work with no collisions.

| id | milestone | crates | depends on | status |
|---|---|---|---|---|
| S1 | Workspace and decision API | `null-bot-api`, `null-mechanics`, `scripts/check_boundary.sh` | | done 2026-10-16 (combat M1) |
| S2 | Message types and transport | `null-proto`, `null-bus` | S1 | not started |
| S3 | Control spine on simulated bots | `null-hq`, `null-op`, `null-squad`, `null-cli`, `null-simbot` | S2 | not started |
| S4 | Adapter, host and login | `mc-adapter`, `null-node`, `skeleton-server/` | S1 | done 2026-10-16, verified live |
| SA | Ability check: basic abilities proven live, fixed in the fork where needed ([ABILITY_CHECK.md](ABILITY_CHECK.md)) | `null-abilitycheck`, `mc-adapter`, fork | S4 | closed 2026-10-19: A1–A12, A10E, A4C, AT and the shape families pass on both servers (358/358 each, verified) |
| S5 | Skills and belief for the goal | `null-skills`, `null-belief` | S4, SA | in progress: mailbox and `null-belief` done; `gather` pure code done, live phase open (below) |
| S6 | Skeleton live | wiring only | S3, S5 | not started |

S3 and S4 can run in parallel. **First target (Allen, 2026-10-16): S4, a swarm that logs in and moves around.** It is done, and so is the combat track's M2 with the swarm duel test. **Next (Allen, 2026-10-17): SA, the ability check, before S5 and S2.** SA closed 2026-10-19; S5 is under way. S5 needs S4 and SA, not S2: its claims can be held locally until S3's squad hands them out in S6. S2 and S3 feed S6 only.

### S2: message types and transport

- **`null-proto`** (decision crate; `serde` allowed). These are the types every level uses, versioned with a `proto_version`:
  - Ids: `ServerId`, `OperationId`, `SquadId`, `BotId` (account name), `JobId`, `ClaimId`, `GoalId`.
  - Location keys carry `server` (AD-K1.1): `Loc { server, dim, pos }`, `Region { server, dim, min, max }`.
  - Typed time: `Gt` (game ticks) and `WallMs` (CONVENTIONS §3.2). No bare integers for durations.
  - **Goals:** `Goal::Obtain { item, n }`, with the other ARCHITECTURE §3.6 kinds as declared variants that the skeleton rejects as unsupported.
  - **Jobs** (CONVENTIONS §3.1): id, kind and parameters, required items and claims, preconditions, success test, budget (with units), `reflexes`, `checkpoint`, `preemption_floor`.
  - **Outcomes:** `Succeeded | Failed(reason) | Blocked(on) | Preempted | Interrupted(incident)`, each with a structured report.
  - **Claims** (CONVENTIONS §3.3): kind (region, block set, container, items, entity, bot), owner, priority and expiry. Requests are take, renew and release, with grant, deny and preempt replies.
  - **Upward messages:**
    - the operation summary, with fixed fields (state, headcount, progress, stock, health, top issue);
    - exceptions, merged with a count;
    - escalations, carrying the remedies already tried.
  - **The digest**, one row per operation.
  - **Bot messages:** `BotState`, `JobOutcome`, `Observation` (a block or entity seen, with observer and tick).
  - **Commands** (K2 §3): status, inspect, set_goal, cancel_goal, halt and recall. Plus `SetReflexes`.
- **`null-bus`** (decision crate): a `Bus` trait using the subject scheme `null.<server>.<level>.<id>.<kind>` and the message classes in F6 §3:
  - intent: at least once, with an idempotency key;
  - summary: latest wins;
  - escalation: deduplicated by key;
  - claim: request and reply with a timeout;
  - event log: durable, append-only;
  - command: highest priority, never queued behind other traffic.

  The skeleton implementation runs in-process on tokio channels. NATS comes in step 4, behind the same trait.
- Done when: every message type round-trips through JSON (unit tests); the in-process bus honours each class's guarantee in tests (a duplicate intent is applied once, summaries keep only the latest, halt overtakes a full queue); the stable build and the boundary check pass.

### S3: control spine on simulated bots

- **`null-simbot`**: a toy world (a grid with trees at known positions) and fake bots that run `fell` jobs in simulated ticks. Fake bots can die and disconnect.
- **`null-hq`** (K1, thin):
  - a goal tree from commands;
  - creating one operation per goal;
  - headcount allocation;
  - the digest;
  - halt and recall, never gated.
- **`null-op`** (K5):
  - **Claims registry:** every claim has an expiry and a priority, preemption sends `Preempted`, and an expired claim frees itself.
  - **Expansion:** `Obtain(oak_log, n)` becomes `fell` jobs, one claim per tree (a block-set claim).
  - **Scheduler, ledger, summaries:** a scheduler that hands job batches to its squad; a ledger counting logs from outcomes; summaries with fixed fields.
  - **Event log:** an append-only JSONL log. A restarted operation rebuilds its claims and ledger from it.
- **`null-squad`** (K6): assigns jobs to idle bots, retries a failed job locally once before reporting, applies job reflex settings, and passes merged observations up.
- **`null-cli`** (K2 subset plus K4): `set-goal obtain oak_log 64`, `status`, `inspect <op>`, `halt [scope]`, `recall [scope]`.
- Done when: in simulation, `Obtain(oak_log, 64)` completes with 2 bots, and the claims log shows no tree claimed by two bots at once. In further runs:
  - a bot killed mid-job has its claim expire and its job requeued;
  - halt stops new work within one squad tick;
  - an operation killed and restarted resumes from its log with the ledger intact;
  - the digest size doesn't grow with bot count (2 bots against 50).

### S4: adapter, host and login

- **`skeleton-server/`**: vanilla 1.21.11 copied from the combat server. Survival, peaceful, offline mode, port 25567 and RCON 25577. A fixed seed with oak trees near spawn (record the seed and tree positions). Gamerules use the 1.21.11 snake_case names in `combat-server/README.md`.
- **`mc-adapter`** (mechanism):
  - **`BotView`:** implemented by borrowing Azalea components inside `GameTick`.
  - **`BotCommands`:** implemented as direct component writes, never `Update`-schedule messages (spike §3.2).
  - **Host system:** one system runs each bot's `Brain` (null-bot-api arbitration) every tick.
  - **Setup rules:** add plugins before the app is built (spike §4.2). Disable or register Azalea's own autonomous behaviours (CONVENTIONS §4.5).
  - **`goto`:** Azalea's pathfinder, behind a command (AD-B1.1).
- **`null-node`** (F1 shell, binary): brings up a list of accounts on one server, with reconnect and backoff. It emits `BotState` and runs per-bot skills handed to it in-process.
- Done when:
  - simulator tests cover each `BotCommands` variant;
  - live, two bots log in to `skeleton-server` and walk to a given point using the pathfinder;
  - live, a swarm of 20 bots in one node logs in and each walks to its own assigned point and back, twice;
  - the server log shows no kicks and no movement warnings.

**Status (2026-10-16): done, verified live by a separate Sonnet agent through RCON and the server log.**
- Two bots reached the `goto` point. 20 bots made two out-and-back cycles each, with 20 online at once.
- The server logged 0 kicks, 0 movement warnings, 0 WARN or ERROR lines, and 22 joins matched by 22 leaves.
- A bot kicked mid-patrol reconnected with backoff and finished its patrol.
- Simulator tests: 13 in `mc-adapter`, one or more for each `BotCommands` variant.

What it added:
- `null-bot-api` gained `goto`, `stop_nav` and `nav_status`.
- New crate `null-skills-demo` holds the `patrol` and `goto_point` skills.
- `null-node` flags: `--goto`, `--center`, `--state-log`, `--exit-after-skills`, `--join-interval-ms` (default 300).
- Use release builds: debug builds of the pathfinder are slow.

Server: seed 3, world spawn set to 416,70,352, about 38 oaks within 40 blocks, listed in the server README. Gaps:
- `goto` starts one tick late, because Azalea's pathfinder only takes `Update` messages. Every other command applies in the same tick, and the view reports `Calculating` meanwhile.
- `recent_attackers` is always empty, because Azalea doesn't track attackers. It must be filled before combat M3, because the no-attacking-a-player-first rule (AD-K2.2) depends on it.
- `using_item` always reports the main hand and 0 ticks.
- Use on crosshair (`UseItem` with a block in view) is not built; it goes in the fork's item-use API (F5 §4.1). Use on a face comes with SA task 1.
- A patrol point that lands on a tree top or a blocked spot is shifted along the ring, so shifted points may sit near another bot's point.
- S4 ran as one subagent task for 67 minutes. 22 of those went to a seed search for S5's trees. In S5, place test trees with RCON `/place feature` if the natural ones get in the way.

### S5: skills and belief for the goal

Done when:
- unit tests run on `FakeView`;
- live, one bot fells a claimed tree completely and picks up the logs;
- belief shows the tree gone, with the bot as observer.

**Redesign (Allen, 2026-10-19):** `fell` is the tree policy over a generic `gather` skill (B4 §4.10). A policy only filters blocks: `target` and `may_clear`. `gather` takes blocks of a type and never digs to find them; `mine` (later) digs by a method. Allen's choices for the tree policy: skip and report a log out of reach (never place to reach it), break only natural leaves that block a log, no replant yet. Commit each step after its verifier passes.

| part | status |
|---|---|
| Fork patch 23, tick catch-up like vanilla ([AD-F5.3](../DECISIONS.md#ad-f53-how-does-the-bot-catch-up-after-its-tick-loop-stalls)) | done 2026-10-19 |
| Action mailbox ([AD-F5.4](../DECISIONS.md#ad-f54-how-do-skills-read-the-results-of-their-actions)), B6 §4.6 | done and verified 2026-10-19, commit `ec8f492`. Full suite 360/360 on vanilla; on Paper + Grim 357/360 (three Grim Timer/Simulation alerts from machine load, judged by the verifier; A4S and A6 rerun once, 148/148, zero alerts). Negative control failed the A13 chain row as expected. Finish bounds "too long", "no answer" and "not started" got sim tests in patch 25's task. |
| Fork patch 25: settle every in-flight sequenced action by the server's cumulative ack, so no break, place or block open ends `Superseded` (instamine chains send one break per tick) | done 2026-10-20, pinned `70a1295` (commit `e6966ee`); full suite 362/362 on each server |
| `null-belief` (F2 minimal) | done and verified 2026-10-19, commit `ec8f492`: 512×512 tiles; `observe`, `observe_batch`, `observe_scan`, `observe_view`, `block(pos, now)`, `find_blocks(kind, near, radius, now)`, `changes_since`; newest tick wins, unknown is never air; 13 tests. Not yet: dimension key, `how` (F2 §4.1, needed for sight mode), persistence, event log, node merge. |
| `null-skills` pure code: `gather`, `TreePolicy`, `fell(...)` constructor | done 2026-10-19, **uncommitted**; a Haiku verifier failed it 2026-10-20 (no re-check before a break, tolerances, claim validation, drops left, spec links); all fixed in the live phase below. `gather.rs`, `reach.rs`, `policy.rs`; 14 tests on `FakeView` then. Break order by layered search; reach to the nearest crosshair point (fork's `aim_at_block`, 4.5 − 0.05); a stand may be on a target broken last (8-log trunk reached; 9th log left Unreachable); planning sliced by a counted work budget (≈1 ms per step in release on a fast test view). |
| `gather` live phase | done, verified and committed 2026-10-20 (full suites 380/380 on each server with patch 26; verifier PASS, negative control failed the hut row only when both the `persistent` check and the neighbour allowlist were broken: no row isolates the `persistent` check yet). Verifier findings fixed with unit tests (26 in `null-skills`): the target is re-read before each aim and before each break (a log swapped for a chest during a walk is not broken, the job ends with a report); `ARRIVE` is now `STAND_SLACK` (0.3); an invalid claim fails the job before any break; `GatherResult.drops_left` reports item entities left near what was broken; spec links; `map_entry`. Adapter `block_property` (`mc-adapter/src/blocks.rs`, `view.rs`; unit and sim tests). New scenario A14 (`scenarios_gather.rs`, `skill_gather.rs`, area (800, 342)), 9 rows, each run once per server: 9/9 on vanilla (8 rows with `STEP_UNITS` 6000 plus the leaves row) and 9/9 on Paper + Grim (8 rows with `STEP_UNITS` 1000 plus the leaves row), zero Grim alerts. Live step time: at `STEP_UNITS` 6000 the longest step was 14 ms and p99 3-10 ms (11-32 steps over 2 ms per row); lowered to 1000 (p99 1.1-3 ms, 0-4 steps over 2 ms per row, but one 14.8 ms step in the tall-trunk row, 5.1 ms in the quota row). A tree costs about 300-500 ticks instead of 150-350 (the bot stands still while it plans). One target's candidate stands (`GEN_UNITS`) are still built in one piece, so a step can run past the budget. |

**`gather` live phase: to do.**
1. ~~A verifier on the pure `null-skills` code~~ (its findings are fixed, see the table; the final verifier covers everything).
2. ~~Adapter `block_property`~~ done 2026-10-20.
3. ~~Measure the step time on the live bot~~ done 2026-10-20, `STEP_UNITS` 6000 to 1000 (table above).
4. ~~Check live~~ done 2026-10-20: `look_at_block` on a log behind natural leaves (the row `log_behind_natural_leaves`: 3 leaves cleared, 3 logs felled), `is_solid` true for logs and leaves, `persistent` read "false"/"true", item entities listed as `minecraft:item` and picked up at `goto` tolerance 0.8 (every row's log count), the pathfinder's one-block step onto log 1 (tall trunk: the bot reached y 201.25 and felled 8 logs).
5. ~~Live rows~~ done 2026-10-20 (A14, below). `gather` scans only its claim (Allen, 2026-10-20; B4 §4.10): a tree wholly outside is not its business and is not reported; a tree across the edge is reported (OutsideClaim) and left whole. Rows: an oak with an axe; an oak by hand; a birch (expect felled or "unreachable" per the trunk read back first); a hut inside the claim (planks and logs; persistent leaves) left intact and reported; a tree across the claim edge left and reported, and a tree wholly outside left alone; a quota that stops after a tree; a tall trunk built by RCON (leaves `persistent=false,distance=1`) with its top log left Unreachable. Readback by RCON: log cells air, the inventory log count, the hut intact; belief shows air with the bot as observer.
6. **Open, in this order (2026-10-20):** (a) Allen runs the full suite once on each server, offline, from the repo root, vanilla first and nothing else running: `./target/release/null-abilitycheck --server 127.0.0.1:25567 --rcon-port 25577 --server-log harness/skeleton-server/server/server.log --account null_check --scenario all 2>&1 | tee ~/Desktop/null-spikes/s5-logs/s5_full_vanilla.log`, then `./target/release/null-abilitycheck --server 127.0.0.1:25568 --rcon-port 25578 --server-log harness/grim-server/server/server.log --account null_check --scenario all 2>&1 | tee ~/Desktop/null-spikes/s5-logs/s5_full_grim.log`. The release binary is built from the final code. **(a) ran 2026-10-20:** vanilla 370/371, Paper + Grim 369/371. Failures, none in S5 code: vanilla A12 `inventory_drops_with_keep_inventory_off` (the bot's checks passed; the 5 stone were not found as an item entity, and cleanup killed 2 items, not 3: unexplained); Grim A2S `break_lightning_rod` (one Timer alert in a 171 ms stall, the bot's 15 ticks match vanilla); Grim A10 `exact_contents` (the open was refused on the client, nothing sent; the row hid the reason). Harness fixes: A10's `scene()` waits until the bot sees each placed block (`wait_block`) instead of a fixed 500 ms; `open_expect` prints the refusal reason; the A12 drops row reports where a missing drop's item entities are. Reruns, once each, with the binary copied to `~/Desktop/null-spikes/s5-logs/null-abilitycheck-s5`: A10 and A2S on Grim, A12 on vanilla (logs `s5_rerun_grim.log`, `s5_rerun_vanilla.log`). Then fork patch 26 (below) before (b), so the verifier's build does not mix with the patch work. (b) A Sonnet verifier checks both logs, the A14 logs (`~/Desktop/null-spikes/s5-logs/a14_*.log`) and the code, and runs a negative control on the A14 rows only (for example, a tree policy that ignores `persistent` must fail the hut row). It reruns no suite. (c) Commit everything (null-skills, adapter, A14, docs), then update OPEN.md.
7. Known small gaps, not blocking S5: `FakeView::inventory()` caches on first read; item entities carry no stack info, so yield is the inventory change and `drops_left` holds positions only; one target's candidate stands (`GEN_UNITS`) are built in one piece, so a step can run past the budget (14.8 ms seen live); the bot stands still while it plans, so a tree takes 300–500 ticks (plan during the walk later); vanilla ran 8 A14 rows with `STEP_UNITS` 6000, so Allen's full vanilla run is the first vanilla run of the final code.
8. **Fork patch 26, block movement effects (urgent, Allen 2026-10-20: walking through a cobweb at normal speed is an instant ban).** The fork's physics has no stuck-speed multiplier (`Entity#makeStuckInBlock`: cobweb, sweet berry bush, powdered snow) and no block speed factor (`Entity#getBlockSpeedFactor`: soul sand, honey); the powdered snow climb rule ignores the leather-boots condition (`PowderSnowBlock.canEntityWalkOnPowderSnow`); the pathfinder counts a cobweb as passable. A bot in any of these predicts the wrong position, so Grim's Simulation check flags it. Done 2026-10-20: fork `34d54a5`, pushed and pinned; full suites 380/380 on vanilla and on Paper + Grim, zero Grim alerts; verifier PASS (negative control: no stuck multiplier fails the cobweb, berry bush and powdered snow rows); A15 9/9 once on each server, zero warnings and zero Grim alerts (logs `~/Desktop/null-spikes/s5-logs/p26_a15_*.log`). Grim's Simulation alerts in A15 found three more faults, fixed in the patch: attribute modifiers applied in hash order (now vanilla's `AttributeInstance#calculateValue` order), the server's powdered-snow frost slowdown ignored (`ClientboundUpdateAttributes`, only the `powder_snow` modifier is read), and the inside-block check only at the end of the tick (now walked axis by axis, `Entity#checkInsideBlocks`). Cobweb is not passable and has infinite mining cost; soul sand and honey are never planned over (not standable). The amended head was not rerun on A15; the full suites cover it. Reruns of 2026-10-20: A10 `exact_contents`, A2S and A12 pass; A10 `shulker_refuses_shulker_pickup` failed once on Grim (Grim's resync after the refused placement landed in the next click; contents correct), so that row now allows a resync on both clicks. The full suites replace a separate S5 full run. Then two verifiers (S5 per (b), patch 26 with a negative control), then the S5 commit, then the patch 26 push and commit.
9. Follow-ups found 2026-10-20, not blocking: powdered snow collision by context (vanilla gives it a full hitbox to a player in leather boots who stands above it and is not crouching, and a 0.9-high one after a fall over 2.5 blocks; the fork's collision context is unused); path costs for magma (vanilla hurts unless the bot crouches; Frost Walker's immunity is enchantment data, so a datapack can change it: a belief) and powdered snow with boots; Frost Walker's frosted ice is a server change a plan must not count on; the refusal text "crouching with an item in hand would place it" should say the click uses the item, not the block. From patch 26's task: **server-sent attribute modifiers other than `powder_snow` are still ignored** (same class as the cobweb bug: a server or item that changes movement speed, step height or gravity makes the prediction wrong; do this next after S5); `movement_efficiency` (Soul Speed) not applied to the speed factor; cobwebs cannot be broken on a path (no sword or shears rule in the mining model); the 16-step budget in `checkInsideBlocks`; ~~the `azalea` lib tests do not compile~~ (stale: they compile and pass at `1370f1b`, checked 2026-10-21 by the walls task; `cargo check --examples` still fails on the testbot example). From the S5 verifier: no A14 row isolates the leaves' `persistent` check (the hut row fails only when it and the neighbour allowlist are both broken); add a row with persistent leaves on a log and no other non-natural neighbour.

10. **Fork patches 27–30 (2026-10-20), built, local, unpushed.** 27 takes every server-sent attribute (the follow-up in step 9) and gates unmodelled inputs (A16); 28 adds evidence to every client click refusal; 29 casts the crosshair ray once per game tick (a click after a stall was checked against an old hit; AT); 30 is tier 1 of the safe envelope ([AD-F5.5](../DECISIONS.md#ad-f55-how-much-of-the-vanilla-client-does-the-fork-model)): Levitation, Slow Falling, the `Movement` disconnect, attributes across a respawn (A17). Plan `PATCH_30_TIER1.md`. Left: Allen's full-suite pair, the verifier (`VERIFY_P27_P29.md`), the commit and the push; then patches 31 (interpolation, with the bot-to-bot view), 32 (pushes) and 33 (refusals, pathfinder walls). S5 closes after these.

**After S5, not in it (Allen, 2026-10-19):** `mine` with methods (fishbone, quarry, tunnel; B4 §4.12), sight mode for anti-x-ray servers (B4 §4.11, F2 `seen_sight`), and underground hazards handled by the miner itself, placing to seal fluids (B4 §4.13).

### S6: skeleton live

- Wire S3's spine to S4/S5's real bots in one process (`null-node` hosting HQ, the operation and the squad).
- Done when, on `skeleton-server`, the person types `set-goal obtain oak_log 64` in `null-cli` and the two bots collect 64 logs between them. Further conditions:
  - the claims log shows no tree claimed or broken by two bots;
  - no block is broken outside a held claim;
  - `status` shows the operation's progress while it runs;
  - `halt` stops both bots within a second;
  - a bot disconnected by RCON `kick` mid-job reconnects, and its job is requeued.

  The verifier checks this in-game through RCON and the server log, not only through Null's own logs.

## Later steps (milestones planned when each starts)

| step | ARCHITECTURE §7 | main components | notes |
|---|---|---|---|
| 2 | Survival core | B2, B3, B4, B6, C3, F1 (death, reconnect), F4 | gather, craft, smelt, eat, sleep, store; logistics ledger; recovery from death and disconnect |
| 3 | Planner | K7, K5 | tech-tree expansion to iron and diamonds unattended; the recipe graph from game data (F5) |
| 4 | Distribution | F6, F1, K5 | NATS behind `null-bus`; levels across processes and hosts; node failure and account reassignment; a load test with a few hundred bots (the bench hardware plan) |
| 5 | LLM commander | K3, K2 | needs AD-K2.1 decided (the bake-off didn't settle it) |
| 6 | Builder integration | C1, C2, B6 | world transformation (`proposals/WORLD_TRANSFORMATION.md`); its tests T-ACC, T-WE, T-OSC, T-COB; narrow the fork's click-use table to item and state conditions (temporary wide table, B6 §4.2) |
| 7 | Nether and End | B5 (PvE), C5, C6 | the full `KillDragon` goal |
| 8 | Faction | C7, C8, B5 (PvP) | the combat track below joins here |

## Combat track

Combat milestones are in [COMBAT_SCAFFOLD.md](COMBAT_SCAFFOLD.md). Its M1 is S1 here. M2 (the combat decision crate) was started 2026-10-16, before the scope was widened to the whole swarm. M3 (live duel) needs S4. M3 and M4 (parameter tuning) wait until the spine works (ARCHITECTURE §10: combat moves in parallel once the spine works).
