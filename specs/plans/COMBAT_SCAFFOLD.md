# Combat scaffold: milestones M1–M4

**Plan, 2026-10-16; a track of [BUILD_PLAN.md](BUILD_PLAN.md).** M1 is the build plan's S1. M3 needs S4's adapter, and M3–M4 wait until the spine works. The build plan's layout and crate rows supersede the ones below where they differ. Implements the first two steps of AD-B5.1 (Allen, 2026-10-16): written doctrine first, then tuning of its named parameters. Training from scratch (step 3) is not planned; the duel-controller seam and the traces keep it possible. Specs this plan follows: B5 (combat), F5 §3–§4.1 (`null-bot-api`, decision vs mechanism crates), CONVENTIONS §3.5 and §4 (version boundary, reflexes), F3 §4.4 (adversaries), K2 / AD-K2.2 (no attacking a player first), `reference/SPIKE_F5_BOUNDARY.md` (what the boundary looked like in the spike, and its gotchas).

Combat is step 8 of the build order (ARCHITECTURE §7). This scaffold starts early by Allen's choice. M1 and the adapter part of M3 are step-1 foundations that every component needs anyway.

## Layout (default; easy to move while it is small)

- Code lives in this repo: a Cargo workspace at the repo root, crates under `crates/`, tools under `tools/`. `specs/` stays the single source of truth for documents.
- Root `rust-toolchain.toml` pins `nightly-2026-10-12`, the Azalea fork's toolchain. Decision crates must also build with `cargo +stable`; `scripts/check_boundary.sh` checks it.
- The Azalea fork is a path dependency: `~/Desktop/null-spikes/azalea` (1.21.11 branch plus 38 cherry-picks).
- The combat test server is outside the repo: `~/Desktop/null-spikes/combat-server/`, a copy of the bench server's vanilla 1.21.11 jar with its own world. Port 25566, RCON 25576 (the bench server keeps 25565 / 25575).

## Crates

| crate | kind | depends on | holds |
|---|---|---|---|
| `null-bot-api` | decision | nothing | `BotView`, `BotCommands`, `EntitySnapshot`, `Skill` and `Reflex` traits, per-bot arbitration, `fake` test doubles |
| `null-mechanics` | decision | nothing | the mechanics table stub: `mechanic(key)` → `{value, unit, confidence, source}` (F5 §3) |
| `null-combat` | decision | `null-bot-api`, `null-mechanics` | duel-controller interface, observation and action types, vanilla clamps, parameter spaces, doctrine files, `melee_basic` doctrine, the `fight` skill with target filtering, trace records |
| `null-harness` | decision | `null-bot-api` | adversary scripts (F3 §4.4). Never depends on `null-combat`: fake players are not Null's code |
| `mc-adapter` | mechanism | Azalea fork, Bevy, `null-bot-api` | `BotView` and `BotCommands` over Azalea, the per-tick host system |
| `null-duel` | mechanism (binary) | `mc-adapter`, `null-combat`, `null-harness` | runs duels on the combat server, resets arenas by RCON, writes results and traces |

Only `mc-adapter`, `mc-nav`, `mc-act` and binaries that link them may depend on `bevy*` or `azalea*` (F5 §4.1). `cargo-deny` is not installed, so `check_boundary.sh` checks this with `cargo metadata` until it is.

## M1: workspace and the decision API slice

Build: the workspace, `null-bot-api`, `null-mechanics`, `scripts/check_boundary.sh`.

- `BotView` covers what combat needs plus F5 §3's basic reads: tick, own position, velocity, look, on-ground, in-water, health, food, attack cooldown progress (0–1), hotbar and selected slot, offhand, using-item state, entities in view as `EntitySnapshot` (id, kind, is-player, position, velocity, look, on-ground, health if known, held item, blocking with shield), recent attackers, solid block at a position.
- `BotCommands` is intent-shaped: walk (forward and strafe in −1, 0, 1), sprint, jump, sneak, look (absolute yaw and pitch), select slot, attack entity, use item, release use, swap offhand, stop.
- `Reflex` registry entries per CONVENTIONS §4.1 (id, group, priority, default, params). Arbitration: the highest-priority enabled reflex that fires preempts the skill for that tick; a disabled reflex that fires emits `ReflexSuppressed`. Scope resolution (CONVENTIONS §4.2) only needs bot and job scope now.
- `fake::FakeView` and `fake::FakeCommands` for tests.

Done when: `cargo test` passes; `cargo +stable build -p null-bot-api -p null-mechanics` passes; `check_boundary.sh` passes, and fails when a decision crate is given an `azalea` dependency.

## M2: combat decision crate

Build: `null-combat`.

- **Duel controller.** `trait DuelController { fn id(&self) -> &str; fn reset(&mut self, seed: u64); fn step(&mut self, obs: &DuelObs) -> DuelStep }`. `DuelStep` is an action, or done with a reason (target dead, target lost, retreat). Every technique, hand-written or learned later, implements it.
- **Observation and action.** `observe(view, target) -> Option<DuelObs>` is a pure function. `DuelObs::to_vec()` and `DuelAction::to_vec()` have a fixed, documented order, for traces and for a later learned policy.
- **Vanilla clamps.** Applied after the controller, never inside it: maximum clicks per second, maximum turn per tick, attack only inside reach. Values come from `null-mechanics` with their confidence. Each clamp that changes an action is counted.
- **Parameters.** Each technique declares a `ParamSpace` of named, bounded parameters with units and defaults, with mapping to and from a unit vector for optimizers.
- **Doctrine file.** One small text file per technique version: technique id, version, parent version, parameter values. Hand-parsed, so the crate keeps zero dependencies. Round-trips exactly.
- **`melee_basic` doctrine.** Approach to a spacing distance; strafe with a period and seeded jitter; attack when cooldown reaches a threshold and the target is in reach; critical hits when enabled and valid (falling, not on ground, not sprinting, not in water); W-tap after a hit; raise the shield when the opponent is about to swing and own cooldown is low; done with retreat below a health threshold. Deterministic for a given seed (own small PRNG).
- **`fight` skill.** Target filtering outside the controller: never cargo, never allies, and no attacking a player first unless that permission is disabled (AD-K2.2). Then it delegates to the chosen controller and applies the clamps.
- **Traces.** `TraceRecord { tick, obs, action, clamped }` as one JSON line, written by hand.

Done when: unit tests on `FakeView` cover approach, reach and click-rate clamps, crit validity, W-tap, retreat, the player-first rule, parameter round-trips, doctrine file round-trips and determinism by seed; stable build and boundary check pass.

**Status (2026-10-16): done, verified** (21 tests; the stable build and the boundary check pass). Known gaps:
- "The opponent is about to swing" is approximated, because a client can't see another player's attack cooldown. The shield goes up when the target is in reach, isn't blocking, and our own cooldown is below 0.5. M3 can add the opponent's swing animation as an observation.
- `BotView` has no sprint flag, so the controller tracks its own sprint command.
- Every `null-mechanics` value is still Claimed: the server jar is obfuscated, so they need probing on the test server.

## Swarm duel test (2026-10-16, ahead of M3)

Allen asked for two swarms to duel while he watched. Two `null-node` processes run 8 bots each, `null_a*` against `null_b*`. Each bot runs `null-combat`'s `DuelLoop`: goto toward the nearest enemy picked by name (`TargetSelector::PlayerNames`), fight it with `melee_basic`, kite away when health is low.

Kits are drawn by seeded coin flips: an iron sword for every bot, then an iron chestplate and a shield at 50% each. Team A glows red and team B blue. Run it with `skeleton-server/scripts/swarm_duel.sh`.

**What Allen saw, and the fixes:**
- **Frozen enemies:** bots circled dead bodies and enemies' old positions, and after a teleport they ran back to where the enemies had been. The cause was in the Azalea fork: its shared-world update counters froze entities, and `Dead` was never cleared on respawn (patches 1, 1b, 2 in `reference/AZALEA_PATCHES.md`).
- **Full speed with a shield up:** Azalea didn't slow movement while an item was in use (patch 3). The bot also judged its shield state from the server's echo, which arrives a tick or two late. Now the bot tracks its own shield state and lowers the shield a tick before swinging, since vanilla drops attack clicks while an item is in use.
- **Retreat:** a retreating bot stopped after 3 s and was killed standing still. It now keeps running while an enemy is within 12 blocks, and fights back if caught.
- **Steps:** bots didn't climb one-block steps when they walked straight at an enemy. A step-up jump (`fight::step_ahead`) now covers that, in the fight and in the direct walk.
- **Slow re-lock:** a bot waited 1–3 s before re-locking on a target. Changes: hand-off to the fight at 8 blocks instead of 5; fall back from goto to a direct walk after 1 s without progress instead of 3; re-plan goto only when the enemy has moved 3 or more blocks.

**Result:** kills keep coming. Before the shield fix it was 85 kills in 150 s. After it, shields block, and it was 14–20 kills per 90 s with no gap over 21 s. The server log showed no kicks and no movement warnings.

### Debug session 3 (2026-10-16, night): no more latching phases

**Why:** Allen watched a match. A bot retreated straight away along a cliff edge, hit a tree, was cornered and died. He then pointed out that combat was meant to be reflexes, not a state machine. Every stall in sessions 1–2 was a `DuelLoop` phase that stayed set wrongly. **Rule (Allen): nothing latches; each tick decides again from what the bot sees.**

**Built** (task spec: `specs/plans/TASK_ENGAGE_REFLEXES.md`; a Sonnet agent implemented it, then fixes came from live verification):
- `crates/null-combat/src/engage.rs`, the `Engage` skill. Each tick it picks a target (switching only for an enemy 3 blocks closer) and chooses melee, path or wander. Melee needs the target within 8 blocks and a clear line, and is left only after 10 blocked ticks in a row. `melee_basic` runs with `retreat_hp` 0.
- `crates/null-combat/src/retreat.rs`, the `combat.flee_low_hp` reflex (B5 §4.9; first written as `combat.retreat`). It fires below 6 HP and keeps firing up to 14. It doesn't fire when caught (enemy within 2.5), hopeless (no health rise in 200 ticks), or with no threat (12 to enter, 14 to stay) and no healthy teammate. It runs to a healthy teammate through the pathfinder, skipping the bot's own copy in the shared world and any teammate past the threat. Otherwise it runs away on a safe heading.
- `crates/null-combat/src/telemetry.rs` is shared by both. The format is unchanged: `phase` = what acted that tick.
- `Goto` progress check: a goal with under 0.5 blocks of movement in 40 ticks, or 20 ticks with no progress toward the target, gets a force stop and a new plan. A plain re-goto doesn't free a path stuck on a node.
- `DuelLoop` is deleted. The node registers the reflex in `Brain`. 12 engage tests.
- `// Spec:` links in every major function (new rule, memory `spec-links-in-code`). A Haiku agent checked them, which caught the reflex-name mismatch.

**Verified** in 11 matches across seeds 7, 11 and 31. All ended with a winner in 15–45 s. No flip-flops; no attacks on teammates; no kicks or movement warnings. The last three: `runs/match-20261016-212857`, `-212953`, `-213036`.

**Follow-up (Allen, same night):**
- **Drop rule built:** `fight::drop_worth`/`drop_depth` (B5 §4.5). A fighter drops onto a target more than 2 blocks below and out of reach when it survives the fall and lands healthier than the target. No MLG bucket. Telemetry `drop_in`. Tests: `drop_worth_needs_survival_and_a_lead`, `fighter_drops_onto_a_weaker_enemy_below`. Not yet seen in a live match.
- **Pit escape:** it waits on the pathfinder being allowed to break and place blocks (B1).
- **Healing and fighting at low HP are out of this milestone.** They wait on an item-use skill (eating, with shield use moved into it).
- **The "false beliefs" were analyzer artifacts:**
  - Target health: the bots' value matched a raw server sample in 336 of 339 cases. The analyzer had interpolated health across a hit.
  - Own position: the 2–3 block errors were at the end of the data, compared with a server sample 0.65 s old.
  - `duel_analyze.py` now compares health with the samples on either side and position only between two samples. The warnings disappear on four past matches.

- **Three more bugs found in live verification and fixed:**
  - **Path searches were killed before they finished.** Azalea searches for 1–5 s (`PathfinderOpts` min/max timeout). Re-aiming at a moving target every 10 ticks, the 20-tick no-progress re-plan and the 40-tick force-stop all restarted or killed the search, so on a gentle slope a path never arrived (seed 11: a stand-still of up to 86 s). Now nothing interrupts a search: re-aim and force-stop happen only while `Following`, and a search may run 110 ticks (`SEARCH_TICKS`). Test: `a_path_search_is_never_interrupted`.
  - **The hopeless clock restarted every time the retreat stopped firing.** Two hurt bots at the 12/14-block edge took turns and never committed (seed 11 timed out at 240 s). Each switch also started a new goto, restarting the search (64 s in `Calculating`). The clock now runs whenever health is ≤ 14 and resets only when health rises. Test: `hopeless_clock_runs_while_the_threat_comes_and_goes`.
  - **The drop decision in mid-air now adds the fall distance so far.**
  - Leaf litter was ruled out as a pathfinder blocker: Azalea treats it as passable and not solid.
- **Last verification** (`runs/match-20261016-223755`, `-223833`, `-223909`): winners in 21, 24 and 16 s. No oscillation; no stand-still over 3 s after the opening; no fall damage from drops; no deaths other than kills by enemies; no belief anomalies.

- **20 v 20 in formation (Allen watching):** `duel_match.py --n 20 --formation 4x5 --wait-for allen546 --delay 10`. Teams stand 4 rows of 5, facing each other 24 blocks apart, on ground that isn't leaves or logs; the swords go out 10 s after the named player is online. In the first run, chasers stood about 5 s at a time (b11, b18, and the chase over a small hill), waiting on Azalea path searches with 40 bots searching at once. Fix: `Engage::path` walks toward the target on a safe heading while the search runs (test `heads_for_the_target_while_the_path_search_runs`). Rerun `runs/match-20261016-230859`: team A won in 19.9 s, with no stand-still over 3 s and an enemy known. `duel_analyze.py` now counts samples only from the start signal, so a formation hold no longer counts as stuck.
- `null_a11`'s formation spot (412, 336) has no standable ground by the script's test, so it starts at its spawn point instead.

**Open leads:**
- **Low hit rates in the 20 v 20 crowd** (0.18–0.31 for four bots): not looked at.
- **Opening search stalls of 4–5.5 s** (about ticks 220–510), while 16 bots search at once on the shared pool. Shortening Azalea's search timeouts would cut them, but that is a B1 pathfinder setting shared by every skill: Allen's call.
- **Pit standoff** (seed 7, `null_a6`, about 12 s): a bot in a pit 5 blocks down, with enemies on the rim. It can't get out without digging or placing (B1, a job decision). The enemies stay in melee at the rim: out of reach vertically, and the drop guard blocks the jump. Possible fix: melee also hands over to the pathfinder when the target is more than 2.5 blocks below or above and no hit lands for N ticks.
- **The `hopeless` exit fires 3–6 times per match.** Until bots can eat, a hurt bot eventually fights at low health.
- **Rallies to teammates 27–50 blocks away sometimes lose ground**, probably because the teammate is moving away.
- From before: hits taken with the shield up.

### Debug session 2 (2026-10-16, evening): the three stalls fixed

All three causes of the match 2 stall are fixed. Four matches since then all ran to a winner with no stall: `match-20261016-205313` (seed 7, before the applier fix), `-205527` (seed 7), `-205617` (seed 11) and `-205714` (seed 23). The analyzer flags no stuck or idle bots in the last three. Server and nodes are stopped. Nothing is committed.

- **Fighters frozen: root cause found.** `ActionApplier` sent walk, sprint and look only when they changed. When seek hands over to the fight it stops the pathfinder, and Azalea then queues a `StartWalkEvent(None)` that lands after our walk. A controller asking for a steady "forward" (target 6–14 blocks away) never sent it again, so the bot stood still for good. b4 froze at tick 174 with its target 6.6 blocks away. Fix: the applier sends these inputs every tick (they are component writes, not packets). Regression test: `action::tests::walk_is_resent_every_tick`. The drop guard was not the cause; telemetry showed it never fired.
- **Recovery deadlock: fixed.** `Recover` tracks the best health and when it last rose. After `recover_stall_ticks` (200) with no rise, the bot leaves with `recover_stalled` and is `desperate`: new fights don't retreat until health is above `resume_health`. Eating is still missing (B3/B5).
- **"Healthy seekers see no enemy" was a telemetry artifact.** The sample named a target only in `fight`. a3 and a7 had a target. Their pathfinder kept a goal with no path, which `nav_status` reports as `Calculating`, and seek re-planned only on `Idle`. Fix: re-plan after 40 ticks of `Calculating`. Match 3 also showed the straight-line push driving b3 into a corner while chasing an enemy 85+ blocks away. The push is now limited to `DIRECT_RANGE` (16) with a clear line; otherwise seek re-plans.
- **Telemetry:** fight samples add `ctrl` (controller walk), `act` (after the guards), `jump`, `drop`, `step`, `yaw` and `ground`. Seek samples add `players`, `valid`, `enemy`, `enemy_dist`, `direct` and `goal`. Recover samples add `best_hp`, `rose_at` and `desperate`. `duel_analyze.py`: idle now means no enemy known, and stuck includes seek with an enemy over 4 blocks away.

**Open leads:**
- "Believed target health off by about 4 (p90)" shows for most bots in seed 11. Not looked at; maybe the health update lags the hit.
- Slow relocks (up to 27 s) seen so far are chases: a recovering enemy kites away at sprint speed and the chaser barely gains. A tactics question (chase, or regroup and let it come back), not a stall.
- Team B won the last 4 matches. Check spawn or kit bias before tuning.
- The 200-tick recovery limit is untuned.

### Hand-off for the next debug session (2026-10-16, end of day)

**Where things stand:**
- Stopped and clean: no server, node or script processes. Nothing is committed, in this repo or in the Azalea fork. The fork's changes are listed in `reference/AZALEA_PATCHES.md`.
- Last-team-standing matches with telemetry work: `skeleton-server/scripts/duel_match.py --n 8 --seed 7`. It wipes the bots' player data (allen546's is kept), sets normal difficulty with mob spawning off, kits the bots (sword last, and bots wait for it), and runs with no respawn until one team is all dead. It then writes `runs/match-<ts>/report.md` with `scripts/duel_analyze.py`.
- The analysis compares each bot's beliefs with the server and lists anomalies at the top.
- Match 1 (`runs/match-20261016-195147`, before the latest fixes): A won 8 kills to 3 in 37 s.
- Match 2 (`runs/match-20261016-195614`, with the fixes): 6 deaths, then **everything stalled**. Allen stopped it. Its `match.json` and `report.md` are missing because it was killed. The telemetry, server samples and `server.log` are there.

**Match 2 stall, read from the last telemetry samples.** All 10 live bots were standing still for 20 s or more. Three causes:
1. **Recovery deadlock (a2, a4, a8, b1, b8; health 3–6, phase `recover`, no threat within 12):**
   - The cause: on normal difficulty, health only regenerates with food at 18 or more. Sprinting drains food, and there is no eating skill. So health never rises above `resume_health` (14), and a bot waits forever.
   - Fix: give `Recover` a time limit, or resume when health stops rising. Longer term, add eating (a B3/B5 job) and a "hungry" input.
   - Re-test on peaceful to confirm: there the deadlock should not appear.
2. **Fighters frozen (b2, b4, b7; phase `fight`, full health):**
   - Their targets were bots in recovery 12.4–12.6 blocks away. The belief was correct: b4's believed distance to a4 (12.59) matches the server exactly.
   - Yet they didn't approach for 20 s, with `nav` Idle.
   - Top suspect: the new `fight::drop_ahead` guard sets forward and strafe to 0 every tick (a false positive on slopes; the terrain there goes down, a4 is at y=68 and b4 at y=69).
   - Other suspects: the controller's own output; `line_blocked` didn't fire.
   - Next step: add the action before and after the guards (forward, strafe, `drop_ahead`, `wall_ahead`, `line_blocked`) to the telemetry sample, then rerun, or reproduce with `FakeView` at those positions using the real blocks.
   - Also: a fight against a target 8–14 blocks away should hand back to goto rather than rely on the controller's straight walk.
3. **Healthy seekers see no enemy (a3, a7; phase `seek`, health 20, no target):**
   - Both stood at the exact same position (396.7, 70.0, 371.7) with b8 alive 14.3 blocks away, but `nearest()` found nobody.
   - Suspects:
     - the entity is missing from node A's world;
     - an entity marked `Dead` by mistake, which the adapter now reports as health 0;
     - `is_valid_target` rejecting it.
   - Identical coordinates for two bots is suspicious too: both may be stuck in the same hole, or the wander goto is going nowhere.
   - Next step: add a `known_enemies` count and the nearest enemy's name, distance and health, before filtering, to the sample.

**Other leads from match 1:**
- **Own position off the server's by up to 2.9 blocks,** even with interpolation (a4, a5). It could be timing (the RCON round trip) or real. Check how wall time is stamped on both sides before trusting it.
- **Fight ends were retreat 26, target lost 17:** retreats dominate. Retreat at 6 HP may be wrong for a team fight; tune it in M4.
- **Few shield-ups:** shield up 0–17% of the time.
- **Damage with the shield up:** about once per shield bot. Probably hits from the side or behind with several attackers; not checked.

**Changes made today, untested at match level** (unit tests pass, 26 in `null-combat`):
- engage any enemy the team knows of (`engage_radius` 512, adapter entity radius 256);
- hand-off to the fight only on a clear line;
- leave the fight for the pathfinder when the line is blocked;
- last-stand fight when caught or cornered;
- safe headings when kiting;
- the drop guard in the fight;
- dead bodies stay visible at health 0;
- `TargetDead` reported correctly;
- normal difficulty for matches.

**Still open:**
- No water reflex: a bot drowned.
- Re-lock speed is not measured yet.
- An axe can disable a shield on the server, and the bot wouldn't notice, because it no longer reads the server's echo of its shield state.
- `recent_attackers` is still empty.

## M3: adapter and a live duel

Build: `mc-adapter`, `null-harness` (two adversaries), `null-duel`, and the combat server.

`mc-adapter` already exists from S4 (2026-10-16). M3 must still fill `recent_attackers`, which is always empty now; the no-attacking-a-player-first rule depends on it. It must also make `using_item` report the real hand and duration, and add the opponent's swing animation as an observation.

- `mc-adapter` implements `BotView` and `BotCommands` over the Azalea fork inside `GameTick`. Commands are direct component writes or queued components, never `Update`-schedule messages (spike §3.2). Shield use sends `ServerboundUseItem` itself when a block is under the crosshair (F5 §4.1). Add plugins before the app is built (spike §4.2).
- `null-harness` adversaries: `strafer` (circles, attacks on full cooldown) and `turtle` (shield up, counterattacks). Each has a written note of what real behaviour it stands in for.
- Combat server: survival, PvP on, flat world, no mob spawning, no daylight cycle, arenas spaced so duels can't see each other. Episode reset by RCON: teleport, clear effects and inventory, give a fixed kit, restore health and food.
- `null-duel --a <doctrine file | adversary> --b <adversary> --episodes N --arenas K --seed S --out DIR` writes one JSON line per episode (winner, damage each way, hits, crits, duration, clamp counts) and the traces.

Done when: 20 episodes of `melee_basic` against `strafer` run unattended; no kicks and no movement warnings in the server log; traces show click rate, turn rate and reach inside the clamps; a short report in the run directory.

## M4: parameter tuning

Build: `tools/tune/` (Python with `uv`), driving `null-duel`.

- CMA-ES over `melee_basic`'s unit parameter vector. The objective is the mean episode score against the training adversaries, with the same seeds for every candidate.
- Several arenas at once on one server; the budget is set per run in server-hours.
- Output: a new doctrine file version (parent recorded), and a report comparing it with the default parameters on training adversaries and on a held-out adversary, with confidence intervals.

Done when: one tuning run of 2 server-hours or less completes and reports honestly, including when tuning finds nothing better.

## Not planned yet

More techniques (bow, axe-shield, pearls, mace, spear), PvE `engage`, recorded human play as an opponent, and step 3 of AD-B5.1.
