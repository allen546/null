# Companion mode brief: a second bot in every ability-check row

Implementer brief for one Sonnet task (written 2026-10-25, Allen: "do option 2 after patch 40"). Runs after fork patch 40 (`pick-skips-own-id`) is committed locally.

## Why

Many Null bots run in one process and share one Azalea `World`. Four of the 13 upstream bugs the fork fixed come from that (patches 1, 1b, 2, 40); patch 40 was found only in S6's live run, because nearly every ability-check row runs one bot alone. The shared-world bugs show only when another bot of the same swarm is near: each bot then holds a copy of the other's player, and code that confuses "this bot" with "an entity that has this bot's id" goes wrong. Companion mode puts a second bot of the same swarm next to the main bot in every row, so this class shows up in the ability check instead of in later live runs.

## Read first

- `/Users/allen/Desktop/null/CLAUDE.md` (the rules bind you, the live-run rules most of all).
- `specs/plans/ABILITY_CHECK.md` "How to run" and "Which modules a change reruns".
- `crates/null-abilitycheck/src/main.rs` (the swarm: the main account, `<account>2` for A8, A18 and A19, spares for A16 and A17), `crates/null-abilitycheck/src/scenarios.rs` (`Ctx::scene_in`, `Ctx::reset_player`, the per-row server-log and Grim check).
- `specs/reference/AZALEA_PATCHES.md` rows 1, 1b, 2 and 40 (the shared-world bugs).

## What to build

- **`--companion`:** adds the account `<account>c` to the same `SwarmBuilder` (the same swarm and the same `World`; a separate process would test nothing). It has no skill, is never op, and plays survival.
- **`--companion-first`:** the companion joins before the main bot. Patch 40 failed only on the bot that joined second, so join order matters; both orders must be testable.
- **Placement:** at each row's `scene_in` and `reset_player`, RCON teleports the companion to stand on solid floor about 2–3 blocks from the main bot's start: inside entity-tracking range and in view, clear of every cell the row uses and of the main bot's path, and at least 1.5 blocks away so no push can happen. The default spot is computed from the scene area. A row may name its own spot.
- **Opt-out with a reason:** a row where a nearby player changes what vanilla does (mob targeting, a push, a player in the crosshair line, attackers lists) parks the companion further away but still in tracking range, with a one-line reason in code. Parking it offline or out of range is not allowed, because it would hide the class this mode is for. List every opt-out in your report.
- **Checks:** the companion's Grim alerts and server warnings count against the row, like the main bot's. If the companion dies or disconnects, revive or reconnect it before the next row and note it in the row output.
- **The Pi suite:** `scripts/pi/full_suite.sh` and `run_server_suite.py` pass the flags through (no Pi run by you).
- Docs: ABILITY_CHECK "How to run" gets the two flags and what they are for.

## Live runs

Each live run runs strictly once; no repeat runs to double-check. Do not touch the Pi. Bots must not be op. Do not run a heavy build while a Grim run is going.

1. **Negative control first** (proves the mode catches the class). With fork patch 40 reverted locally (a temporary revert of that one commit, restored afterwards; the pushed head and the pin do not change), run AT and A4 with `--companion --companion-first` once on the vanilla server. Done when at least one crosshair row fails with a ray hit on the main bot's own entity id. If none fails, report it: the mode does not reproduce patch 40's case, and say why.
2. **Restore patch 40** and check that the clone's head equals the pinned `rev` again.
3. **Selective runs:** AT, A4, A13, A14 and A19 with `--companion --companion-first` once on each server (vanilla 25567, Paper + Grim 25568). Then the same set with `--companion` (main bot first) once on the vanilla server.
4. Any new failure is a finding: report the row, the evidence and your best reading of the cause. Do not fix fork code in this task. Fix only harness faults (a companion in the way, a bad spot), and rerun only that module, once.

## Rules

- Every major function gets a one-line spec link (`// Spec: specs/plans/COMPANION_BRIEF.md` or the ability-check section); the verifier fails a missing or dead link. Comments state the why in a line or two. Code reads like the surrounding code.
- `scenarios.rs` is a hub file: by the map, a change there reruns all modules. The full suite runs only on the Pi pair, which the lead launches. Your runs are the selective set above.
- Python: use uv, never pip. Never read or change network configuration on any device. Do not commit. Build with plain `cargo` (the pinned nightly).
- Time box: 60 minutes.

## Done when

Companion mode is built; the negative control fails a crosshair row with patch 40 reverted; with patch 40 restored, the selective set passes on both servers in companion-first order and on vanilla in main-first order with zero Grim alerts, or each failure is reported as a finding with evidence. The workspace tests and `scripts/check_boundary.sh` pass.

Report: files and lines, the spot rule and each opt-out with its reason, the negative-control result, the run logs per server and order, and the findings.
