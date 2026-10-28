# S6 brief: the skeleton live

Implementer briefs for two Sonnet agents, run one after the other (written 2026-10-25). Milestone: `BUILD_PLAN.md` S6. It needs S3 (the spine on simulated bots) and S5 (`gather` on real bots), both done.

S6 is wiring, but it touches five crates and a live server, so it is two tasks, each with one done-when:

- **S6a:** a real bot speaks the bus (jobs, leases, halt, outcomes, state), `null-node` hosts HQ, the operation and the squad, and two real bots collect 64 logs on the vanilla server from a `set-goal` line.
- **S6b:** `halt` within a second, a kicked bot reconnects and its job is requeued, `status` while it runs, the S3 open items S6 needs, one Grim run.

## What S3 leaves for S6 (lead's check, 2026-10-25; paste into both briefs)

From `BUILD_PLAN.md` S3 "Done 2026-10-24":

- A real bot must honour `Lease` exactly as `null-simbot`'s `SimBot` does: refuse a job with no lease for its (job, attempt), stop by itself `LEASE_MARGIN` (10) ticks before the lease ends, take the last lease per (job, attempt) even when it is shorter. `crates/null-simbot/src/bot.rs` is the reference for the whole bot side of the protocol (subjects, halt handling, outcome shape, `BotState`).
- The bot subject helpers (`bot_jobs`, `bot_up`, `bot_summary`, `bot_cmd`) live in `null-simbot`; move them to `null-proto` or `null-bus` (re-export from `null-simbot` so its tests do not change).
- Renews must not block the squad's tick (today each renew is awaited, up to 2 s). S6b fixes it; S6a only must not make it worse.
- No un-hold command: if every bot is held, the op escalates `quota_infeasible`. Leave it; note it if a live run hits it.

What the live world changes:

- **Trees come from the server, set up by RCON.** The operation takes a tree list (`Vec<(Loc, u32)>`). For S6 the run script plants the trees by RCON in a known area and writes the list to a JSON file; `null-node` reads it (`--trees FILE`). Finding trees from belief is later work. Reuse A14's planting (`crates/null-abilitycheck/src/scenarios_gather.rs` `tree`) so the tree policy accepts them.
- **Claims must cover what the bot breaks.** Today the operation claims `ClaimKind::BlockSet` of the tree's base cell only. A real `gather` breaks every log and maybe leaves, so the operation must claim a `ClaimKind::Region` box around each tree, and the job must carry that box (a `fell` param, for example `"claim": Region`). The bot gives `gather` exactly that box as its `Claim`. The sim keeps working (its trees get a one-tree box).
- **Clock.** The spine steps once per game tick (50 ms). Real bots tick in the Bevy schedule. The bot's tick loop must never wait on the bus: read with `try_recv` (sync), and send outcomes and state without blocking (a channel to the spine task, or a spawned send).
- **Boundary.** The bus side of a bot (jobs, leases, halt, outcome, `BotState`, which skill to start) is decision code with no Azalea: put it in a decision crate, tested offline with a fake skill. Only the glue that starts the skill on `BotBrain` and reads its result lives in `null-node`/`mc-adapter` (`scripts/check_boundary.sh`).
- **CLI.** `null-cli` cannot import Azalea. Make the live spine reuse `null-cli`'s library (`parse_line`, `render`, `Audit`, the HQ/op/squad stepping in `spine.rs`, with the bots made pluggable) and let `null-node --spine` read the same command lines from stdin. Keep `null-cli`'s simulated mode and its tests unchanged.

---

## Task S6a: real bots on the bus, 64 logs live

You work in /Users/allen/Desktop/null (Null: a swarm of Minecraft bots in Rust with an LLM commander above it). Read CLAUDE.md first; its rules bind you, the live-run rules most of all. Your task is the first half of milestone **S6: skeleton live**.

### Read first

- `specs/plans/BUILD_PLAN.md` "### S6: skeleton live" and S3 "Done 2026-10-24".
- This brief's section "What S3 leaves for S6" (above).
- `crates/null-simbot/src/bot.rs` (the bot side of the protocol), `crates/null-cli/src/spine.rs` (the spine in one process), `crates/null-op/src/op.rs` (the `fell` params and claims), `crates/null-squad/src/squad.rs`.
- `crates/null-node/src/main.rs`, `crates/mc-adapter/src/host.rs` (`BotBrain::install_skill`, `SkillEnded`), `crates/mc-adapter/src/lifecycle.rs` (`NodePlugin`, reconnects).
- `crates/null-skills/src/gather.rs` (`GatherConfig`, `GatherResult`) and how A14 runs it live (`crates/null-abilitycheck/src/scenarios_gather.rs`, `skill_gather.rs`).
- Specs, only the sections named: `specs/K6-squad-leader.md` §3 and §4.8 (lease rule); `specs/CONVENTIONS.md` §3.1 (job contracts, outcome shape); `specs/F1-swarm-runtime.md` §4.4 (death and reconnect); `specs/B4-gathering.md` §3.

### What to build

- **Bot side on the bus** (decision crate, no Azalea): one struct per bot that does what `SimBot` does on the bus, but instead of simulating, it asks the host to start a `gather` with the job's claim box and quota and turns the `GatherResult` into a `JobOutcome` (yield from `yield_items`, `left` and `drops_left` in the note). Lease rule exactly as `SimBot`. Unit tests with a fake skill: a job with no lease is refused; the bot stops 10 ticks before its lease ends; a halt ends the job as `Preempted` and drops jobs read in the same tick.
- **Operation:** `Region` claims and a claim param on `fell` jobs (above). Existing `null-op`, `null-squad`, `null-cli` tests still pass.
- **Node:** `null-node --spine --trees FILE --accounts A,B --server HOST:PORT` hosts HQ, the operation, the squad and the two bots in one process and reads command lines from stdin. A bot that dies or disconnects publishes the matching `Life` in `BotState`, so the squad requeues its job (the reconnect itself is S6b's check).
- **Run script:** `scripts/s6_live.sh` (or a small `null-abilitycheck` scenario if that is simpler; say which and why): plants enough trees for 64 logs plus margin by RCON in an area no ability-check scenario uses, gives the bots nothing (no axe needed), writes the tree list, starts the node, sends `set-goal obtain oak_log 64 --reason "S6a live"`, waits until the operation is done or a time limit passes, then reads back by RCON: each bot's `oak_log` count, every planted log cell, and the server log for warnings. The bots must not be op.

### Live runs

Each live run runs strictly once. Run the done-when once on the vanilla server (`harness/skeleton-server`, port 25567, RCON 25577); start it with `scripts/start_server.sh` if it is not up, and wait for "RCON running". Do not run Grim (S6b does). Do not touch the Pi. If the run fails, fix the cause and run once more; report both runs. Run the selective ability-check modules your change affects, from the map in `specs/plans/ABILITY_CHECK.md` "Which modules a change reruns" (`scripts/affected_modules.sh`), once on each server, in the foreground if they take minutes.

### Rules

- Every major function gets a one-line spec link (`// Spec: specs/K6-squad-leader.md §4.8`); the verifier fails a missing or dead link. Comments state the why in a line or two.
- Write code that reads like the surrounding code.
- Python: use uv (`uv run --with`), never pip. Never read or change network configuration on any device.
- No fallback that trades safety for finishing: a bot never breaks outside its claim box; if `gather` cannot run, the job fails with a reason.

### Done when

On the vanilla server, `null-node --spine` with two bots and the `set-goal obtain oak_log 64` line ends with the operation Done; RCON shows the two bots hold at least 64 `oak_log` between them; the operation's event log shows no tree claimed by two bots at once; every broken log cell lies inside a claim box that was held; the server log has no warnings from the run. `cargo build` (the pinned nightly; `+stable` fails on `simdnbt` once Azalea crates are in the build), the workspace tests and `scripts/check_boundary.sh` pass.

Report: what you built (files, lines), the claim-param names, the run log paths and the RCON readback, the selective module results, and any S3 open item you hit.

---

## S6a result (2026-10-25)

Built (Sonnet, 27 min): `crates/null-botlink` (`BotLink`, the bot side of the protocol behind a `Host` trait, mirroring `SimBot`; `FellRun`, `gather` with a stop switch; 11 offline tests, lease tests checked to fail with the margin removed), `crates/null-node/src/spine.rs` (the live spine on its own thread, one step per 50 ms; the Bevy side takes skills from an inbox and never waits on the bus), `scripts/s6_live.py`/`.sh`, `null-node --spine --trees --audit --op-log-dir --job-budget` (default 2400 ticks). `Spine` in `null-cli` is generic over `SpineBot`. `fell` params: `trees`, `quota`, `tree_type`, `claim` (Region: 3 blocks out from the trunk, base log to top log + 2; `OpConfig.tree_box`, sim `(0, 0)`). Subject helpers moved to `null-bus`, `LEASE_MARGIN` to `null-proto`. Trees: 24 oaks of 6 logs at x 1200-1248, z 700-732. Live run 2 (`harness/skeleton-server/runs/s6a-20261025-115241`): op Done 64/64 in 457 s, RCON `null_s6a1` 67 logs, `null_s6a2` 0; 68 of 144 log cells broken, all inside held boxes; no double claims, no warnings. **Done-when half met:** `null_s6a2` failed every job with `crosshair does not hit it` on its first break (in run 1 it worked). Run 1's readback was taken after the node quit (script fixed). Selective modules: none (`Cargo.lock` adds only workspace entries; `null-abilitycheck` depends on no spine crate). A fix task for the a2 failure started 2026-10-25 12:05 (Sonnet). **Cause found (fix task, 2026-10-25):** a fork bug. With two bots in one swarm the shared `World` holds the other bot's view of a2's player: a second ECS entity with a2's own server id (2116). The pick (`azalea-client/src/plugins/interact/pick.rs` `is_pickable`) skips only the bot's own ECS entity, so the ray (starting inside that box) hits a2 itself first. Evidence: run 3 (`runs/s6a-20261025-121006`) has 40 refusals "ray hit entity 2116 (minecraft:player)" and the server log gives a2 id 2116; a1 71 logs, a2 2, op Done 73. Not traced: why run 1 worked (likely the duplicate's lagging position). Also kept: `gather` now puts the client's click evidence in a refused break's reason (`describe_evidence`, test `a_refused_break_reports_what_the_client_saw`, fails without it). Fork patch 40 `pick-skips-own-id` started 12:21 (Sonnet; local, not pushed).

S6a rerun (2026-10-28): with fork patches 40-44 (`null-main` 9abfaa2) and a fresh release build, one run on the vanilla server (`harness/skeleton-server/runs/s6a-20261028-162005`, no code change) met the whole done-when. The operation ended Done in 218.6 s; RCON shows `null_s6a1` 28 and `null_s6a2` 38 `oak_log` (66 total); 70 of 144 log cells broken, all inside held claim boxes; no double claims, no server-log warnings, no refusals, bots not op. The `crosshair does not hit it` failure of `null_s6a2` is gone.

S6a claim-release fix (2026-10-28): a verifier found that the operation released the claim of a job still running when the goal was met (`check_done`), so for 0.8 s a bot worked in an unheld box. Now a job in flight keeps its claim after goal met; `on_outcome` releases it when the outcome of that attempt arrives (`rec.st == Dead`), and the claim's own expiry (the lease end: the squad stops renewing) frees it when no outcome comes. Only `check_done` released a live attempt's claim; the release in `dispatch` for a re-attempt and in `on_outcome` run only when the claim is already gone or the outcome is in, and the squad and HQ never release. Tests in `null-op` (`goal_met_keeps_the_claim_of_a_running_attempt_until_its_outcome`, `goal_met_keeps_the_claim_until_the_lease_ends_when_no_outcome_comes`, three bots, a box-overlap check on the log) fail with the fix removed. The first live run (`runs/s6a-20261028-163630`) with the new `s6_live.py` poller (every log cell by RCON each second, `breaks.json`, breaks checked against hold intervals, floor item count) showed a second hole: the last log of the last tree broke 0.6 to 1.6 s after the claim was released and after the bot's skill ended, because `FellRun` on a stop only called `stop_nav` and the break in progress finished. `FellRun` now calls `cancel_break` on a stop (test extended; fails without it). Second live run (`runs/s6a-20261028-164341`): op Done, 66 `oak_log` held (31 and 35), 71 cells broken, all 71 polled breaks inside a held box (last break 0.24 s before the release), 66 gathered by outcomes, 5 on the floor, difference 0, no warnings, bots not op. Selective modules: none (`null-abilitycheck` depends on neither `null-op` nor `null-botlink`).

**Verifier and lead follow-up (2026-10-28).** Haiku verifier: both goal-met tests and the stopped-fell test fail without their fixes; run 2 numbers recomputed and confirmed. It found the harness too lenient: `ok` ignored straddles and the break difference, and the fit test allowed one poll interval (1 s) of slack, so run 1's break 0.6 s after the release passed. Lead fix in `scripts/s6_live.py`: poll every 0.25 s; a break fits a hold only when its whole window (last seen standing, first seen gone] lies inside [grant, release] widened by the map error (0.1 s); a straddle, an unfit break or a nonzero broken-minus-gathered-minus-floor fails the run; per-function `// Spec:` links. Rechecked from the saved `breaks.json`: run 1 fails (1 unfit break), run 2 passes (0). Two holes left for S6b: (a) a preemption by a higher-priority owner drops the claim in the registry at once, but the bot works on until the squad's next renew sends lease = now (up to `renew_every`, 20 ticks); one op cannot reach it, but the claim must outlive the bot's stop as at goal met. (b) `null-botlink` `link.rs` `advance`: after `STOP_WAIT` the outcome goes up even when the skill never stepped, so `cancel_break` may not run before the claim is released. Not classified: 42 non-log item entities on the floor of run 2.

## Task S6b: halt, reconnect, status, Grim (not started)

Written after S6a's report. Scope: `halt` stops both bots within a second (measured from the line to the last break on the server); a bot kicked by RCON mid-job reconnects and its job is requeued and finished by someone; `status` shows progress while the run goes; renews no longer block the squad tick; one run of the S6 script on Paper + Grim with zero alerts. Then a Sonnet verifier checks in-game through RCON and the server log.
