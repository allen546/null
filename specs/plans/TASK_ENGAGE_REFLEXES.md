# Task: replace DuelLoop's state machine with an engage skill and a retreat reflex

**Written 2026-10-16 (evening debug session).** One implementer task, about 600–800 lines including tests. Uncommitted work; do not commit.

## Why

`crates/null-combat/src/duel_loop.rs` is a three-phase state machine (`Seek` → `Fight` → `Recover`) whose phases stay set until an exit condition fires. B5 §4.9 and CONVENTIONS §4 define combat as per-tick behaviour with reflexes arbitrated by priority, and `null-bot-api`'s `Brain` (`crates/null-bot-api/src/reflex.rs`) already does that arbitration. Every stall found on 2026-10-16 was a phase that stayed set wrongly: a bot frozen after the switch into `Fight`, `Recover` waiting forever for health that never came, seek waiting for a path that never came, and fight/seek flip-flopping at a tree trunk, which restarted the fight every few ticks. Allen's rule: **nothing latches. Each tick decides again from what the bot sees, so a wrong decision costs one tick, not the match.** Memory inside a behaviour (the goto goal in flight, the controller's strafe timers, a smoothing counter) is fine. A mode that only an exit condition can leave is not.

## Build

1. **`Engage` skill** (new file `crates/null-combat/src/engage.rs`, implements `null_bot_api::Skill`, never finishes). Each tick:
   - Pick the target: nearest valid enemy (`FightRules::is_valid_target`, health > 0). Keep the current target unless it is gone, dead or invalid, or another enemy is more than 3 blocks closer (stops target flapping).
   - Choose how to move, from scratch each tick:
     - **Melee** when the target is within 8 blocks (`handoff`) and the line to it is clear. Once in melee, stay while within 14 (`drop_radius`) and the line has not been blocked for `BLOCKED_TICKS` (10) ticks in a row. That is smoothing, not a latch: the inputs are re-read each tick.
     - **Path** otherwise: `goto` toward the target with today's re-plan rules (target moved ≥3 blocks and ≥10 ticks since the last goto; `Idle` and ≥4 ticks; `Calculating` and ≥40 ticks). Use the straight-line push only within `DIRECT_RANGE` (16) on a clear line, after 20 ticks without progress.
     - **Wander** toward the centre when no enemy is known (as `DuelLoop::seek` does now).
   - In melee, run a `FightSkill` built with `melee_basic` and **`retreat_hp` = 0**: retreating is the reflex's job, not the controller's. Keep one `FightSkill` for the current target across ticks so strafe and crit timing survive. Make a new one only on a target change. When melee hands over to path, call `abort` (releases the shield and inputs). When path hands over to melee, call `stop_nav`.
   - Hold still until a sword or axe is in the hotbar (as now).
2. **`combat.flee_low_hp` reflex** (B5 §4.9; first written here as `combat.retreat`, renamed after verification) (new file `crates/null-combat/src/retreat.rs`, implements `null_bot_api::reflex::Reflex`; group `Combat`, `default_on: true`, params listed in its `ReflexSpec`). It is evaluated every tick:
   - **Trigger:** health < `retreat_hp` (6), or it fired last tick and health ≤ `resume_health` (14). That is hysteresis, re-read each tick. It does **not** fire when an enemy is within 2.5 blocks (caught: engage fights it out), or when the bot is **hopeless**: it has been firing for `stall_ticks` (200) with no rise in health. A bot can't eat yet, and health regenerates only with food ≥ 18. Hopeless clears when health rises or goes above `resume_health`.
   - **Act:** run to teammates. Move the rally-point logic from today's `DuelLoop::rally_point` (near teammates, scored by distance minus 4 × friends within 8, skipping any teammate on the far side of the threat). Go there with `goto` and the same re-plan rules. Within 3 blocks of the teammate, stop. While the pathfinder is not `Following` and an enemy is within 12, run away from it on a safe heading (`safe_heading` and `run` from today's `duel_loop.rs`). With no teammate in range, use that away-run on its own. If no safe heading exists, don't fire (engage fights).
   - Allies are `rules.never_names` (team names). Check whether the bot's own entity can appear in `view.entities()` in `mc-adapter`. If it can, exclude it by name, or the bot rallies to itself.
3. **Node wiring** (`crates/null-node/src/main.rs`, the `fight` branch of the factory): register the retreat reflex in `Brain` and pass `Engage` as the skill. Delete `DuelLoop` and `DuelLoopConfig` once nothing uses them.
4. **Telemetry.** It must keep working, and `skeleton-server/scripts/duel_analyze.py` (`~/Desktop/null-spikes/skeleton-server/scripts/`) must keep producing its report. Keep the JSON line format of `DuelLoop::report`, with `phase` now meaning what acted this tick: `"fight"` (engage, melee), `"seek"` (engage, path or wander), `"recover"` (the retreat reflex acted). Keep the existing fields per phase: fight `ctrl/act/jump/drop/step/yaw/ground`, seek `players/valid/enemy/enemy_dist/direct/goal`, recover `best_hp/rose_at/desperate/rally` (`desperate` = hopeless). Keep the events (`phase` with `why`, `target`, `hurt`, `shield`, `attack`, sample every 10 ticks). Use `why` values such as `target_dead`, `target_lost`, `blocked`, `target_far`, `retreat`, `caught`, `hopeless`, `healed`. Exactly one of the two acts each tick, so share one telemetry handle between them (for example `Arc<Mutex<..>>` holding the sender and the last phase). Keep it simple.
5. **Unit tests** (`crates/null-combat/tests/`, `FakeView` and `FakeCommands` from `null_bot_api::fake`; see `tests/combat.rs` for style). At least:
   - melee to path after 10 blocked ticks, and back to melee when the line clears, with no extra state;
   - a single blocked tick does not leave melee;
   - the retreat fires at low health with an enemy near and issues a goto toward a teammate;
   - it skips a teammate behind the threat;
   - it stops firing when caught (enemy ≤ 2.5) and when hopeless (200 ticks with no health rise);
   - with `Brain`, the reflex preempts engage while it fires and engage resumes the tick after.
   - Keep `action::tests::walk_is_resent_every_tick` and all existing tests passing.

## Rules

- Read first: `crates/null-combat/src/duel_loop.rs` (today's logic and its comments; most of it moves over), `fight.rs`, `melee.rs`, `action.rs`, `crates/null-bot-api/src/reflex.rs`, `fake.rs`, and `crates/null-node/src/main.rs`.
- **Spec links (Allen, 2026-10-16):** every major function (`Engage::step` and its move choice, the retreat `trigger` and `act`, the rally-point choice, the node wiring) carries a one-line comment naming the spec it implements, e.g. `// Spec: specs/B5-combat.md §4.9; specs/CONVENTIONS.md §4`. Verification checks that each link exists and points at a real file and section.
- Match the surrounding code: short comments that give the why in a line or two, the same naming and the same idiom. `null-combat` has no dependencies beyond the workspace crates; keep it that way.
- Don't change `mc-adapter`, the Azalea fork or `melee.rs`'s logic. If something there blocks you, stop and report.
- Done when: `cargo build --release -p null-node` and `cargo test -p null-combat -p null-bot-api` pass with no new warnings. Report the files changed, the line counts and any deviation from this spec with the reason.
- Don't run the server or matches; verification does that.
