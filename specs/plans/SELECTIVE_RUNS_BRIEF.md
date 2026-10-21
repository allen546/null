# Implementer brief: the selective-runs map and module isolation

Written 2026-10-21 by the lead, after Allen approved selective runs (CLAUDE.md "Subagents"; ABILITY_CHECK "Which modules a change reruns"). Launch it after Allen's current run ends and the timing task has finished its tests and smoke run, before the patch 31 implementer. Delete this file when the work is committed.

## Brief

You build the tooling that lets a change rerun only the ability-check modules it affects, and you make each module safe to run alone or in any order. Null is a Minecraft bot swarm in Rust on an Azalea fork. Repo: /Users/allen/Desktop/null. Read /Users/allen/Desktop/null/CLAUDE.md first and follow it exactly. Then read `specs/plans/ABILITY_CHECK.md` (the "Which modules a change reruns" section, the Status table and "How to run"), HANDOFF "How to run", and `crates/null-abilitycheck/src/main.rs` and `scenarios.rs` (`Ctx`, `scene_in`, `clean_in`, how modules are chosen by `--scenario`).

### Why

Today every fork push needs a full pair: about an hour per server pair. Most changes touch a few modules. Two things block selective runs: (1) nobody has written down which modules exercise which code, so the choice is a guess; (2) some rows depend on server state left behind by earlier modules. Example from 2026-10-21: A17 `respawn_keeps_base_attribute` killed the bot without setting a spawn point, so its result depended on what had run before it (the lead has since added a `/spawnpoint` line to that row).

### State

- The Null working tree holds uncommitted work from several tasks. Do not commit, stash, reset or revert anything. Add your changes on top.
- Fork clone ~/Desktop/null-spikes/azalea, branch null-main, pinned in Null's `Cargo.toml`. You do not change the fork.
- Scenario modules: `crates/null-abilitycheck/src/scenarios*.rs`; each has its own `AREA`. Skills used by rows: `skill*.rs`.

### Work

1. **The map.** Write a machine-readable file `crates/null-abilitycheck/run_map.toml`. Each entry is a path prefix (in Null, or in the fork as `azalea:<path>`) with the scenario ids it affects. Build it by tracing, not guessing: for each module, follow its `Op`s through the `skill*.rs` code to the `null-bot-api` and `mc-adapter` calls, and from there to the fork APIs and systems they reach (physics and travel, collision, interact and pick, block actions and mining, inventory and clicks, entity interact, packet handlers, attributes and effects, the unmodelled gate, the tick loop). Mark entries that every module reaches (for example the tick loop, `scene_in`, connection and login) as `all = true`. Keep it coarse: one entry per crate area or file, not per function. In ABILITY_CHECK, replace "not written yet" with a short human-readable summary of the map and a pointer to the file. Check the map against history: for each fork patch row in `specs/reference/AZALEA_PATCHES.md` from 20 on, the modules it names as tests (or that failed because of it) must be in the map's set for its files. List any miss and fix the map.
2. **A helper.** `scripts/affected_modules.sh [--fork-rev <rev>]`: reads `git diff --name-only` of the Null tree (against HEAD, plus untracked files) and of the fork (against the pinned `rev`, or `--fork-rev`), matches the paths against `run_map.toml`, and prints one `--scenario A..,A..` argument (or `all`). Paths that match no entry make it print `all` and name them, so an unmapped change never gets a narrow run. Python, if you use it: uv only (`uv run --with`), never pip or system python.
3. **Isolation.** In `Ctx::scene_in` (or a function it calls), reset the shared player and world state that rows rely on: spawn point (to a fixed safe spot in the module's area), effects (`effect clear`), attribute base values and modifiers that rows set (movement_speed, scale, jump_strength, gravity, step_height, block_break_speed, sneaking_speed and the others A16 and A17 touch), gamemode survival, game rules the suite changes (find every `gamerule` call; reset each to the suite's default), time and weather if any row sets them, health, food and saturation, fire and freeze ticks, the held slot and the inventory as `scene_in` already does. Then read every module's rows for state they read but never set, and fix each one in that module. List what you found.
4. **A shuffle option.** `--shuffle <seed>` makes the runner run the chosen modules in a random order from the seed, and prints the order and the seed on the summary line. Rows inside a module keep their order.

Every major function gets a one-line spec link (`// Spec: specs/plans/ABILITY_CHECK.md §Which modules a change reruns` or the module's own spec). The verifier fails on a missing or dead link. Write code that reads like the surrounding code.

### Live-run rule (Allen, 2026-10-19/20/21)

Live runs are slow: each runs strictly once. Your change to `scene_in` reaches every module, so by the rule it needs the full pair, and the full pair is Allen's: do not run `--scenario all` or a list of every module yourself. Run only these, once per server (vanilla harness/skeleton-server port 25567, RCON 25577; Paper + Grim harness/grim-server port 25568, RCON 25578), with a distinct `--account`: A16 and A17 (they set attributes and kill the bot, so they test the reset), A12 (respawn and spawn points), and one `--shuffle` run of those three modules together. A failed row is fixed and only its module rerun, once. No repeat runs to double-check. Do not run a heavy build while a Grim row runs. At the end, with the code final, give Allen the full-pair command with a shuffle seed: `scripts/full_suite.sh <prefix> --shuffle <seed>`. The next fork push (patch 31) may use that pair as its gate.

### Machine notes

- Python: uv only (`uv run --with`), never pip.
- `stat` is GNU coreutils: `/usr/bin/stat -f %m FILE` for BSD.
- Bots must not be op. Grim flags clicks while moving, sprinting or sneaking.
- RCON password `skeleton`. Paper cuts long `data get` replies with "...": read NBT field by field.

### Done when

`run_map.toml` and `scripts/affected_modules.sh` exist and the map passes the history check; `scene_in` resets the shared state; `--shuffle` works; `cargo test -p null-abilitycheck` passes; and A12, A16 and A17 plus one shuffled run of them each passed once on vanilla and once on Paper + Grim with zero Grim alerts.

### Report (under 600 words)

Files changed; the map's summary (areas and their modules, and which are `all`); misses found by the history check; the state that `scene_in` now resets; each isolation bug found in a module and its fix; the helper's output for the current tree (the uncommitted patches 27–30 work plus fork head against the pinned rev); live result lines and log paths; any Grim alerts; the full-pair command for Allen; anything left open.
