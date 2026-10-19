# Null: notes for Claude

Null is a swarm of Minecraft bots (Rust, on a public Azalea fork) with an LLM commander above it. This file holds the standing rules; the current state lives in the docs below.

## Where to start

- [OPEN.md](OPEN.md): decisions waiting, next actions, where things are.
- [HANDOFF.md](HANDOFF.md): the current milestone (now S5, skills and belief), how to run the ability check, open items. SA's record and per-ability gaps: `specs/plans/ABILITY_CHECK.md`.
- [specs/INDEX.md](specs/INDEX.md) for more context. Don't load `specs/logs/` (about 600k tokens of old builder logs).
- Plans: `specs/plans/ABILITY_CHECK.md`, `BUILD_PLAN.md`, `COMBAT_SCAFFOLD.md`. Fork ledger: `specs/reference/AZALEA_PATCHES.md`.

## Architecture rules

- Raw game capability (placing with results, item use, attackers, menus and clicks, pathfinder moves) goes in the Azalea fork as normal Azalea APIs. `mc-adapter` only translates fork components to the version-stable `BotView`/`BotCommands` in `null-bot-api` (F5 §4.1). Only `mc-adapter` and `null-node` import Azalea or Bevy (`scripts/check_boundary.sh`).
- Null targets vanilla servers. The fork and adapter must also run cleanly on Paper + GrimAC, timing included, with zero alerts (F5 §4.6). An ability passes only when it passes on both test servers.
- A bot never sends a packet a vanilla client couldn't send. It refuses on the client, with a reason, and sends nothing.
- Null computes exactly what a vanilla client decides or sends (its own physics, input, timing, clicks and their predicted results), and nothing the server decides. It skips display, and hidden server state is a belief (F5 §4.6, "What Null computes").
- Server-configurable timings are beliefs, not facts (example: the shield's raise-to-block delay).
- Builder work: the product is the safety guarantee, not finishing. A plan is certified (clean full replay, deterministic) or the result is a stuck report; no retries on a live server. Never trade safety for completion (no unproven fallbacks to get past a step).

## The Azalea fork

- Public fork `allen546/azalea`, branch `null-main`; local clone `~/Desktop/null-spikes/azalea` (remote `null`). No upstream PRs or issues: upstream asks for no AI-written contributions.
- One commit per patch, named `null patch: <n>-<name>`, pushed to `null-main`. Each patch gets a row in `specs/reference/AZALEA_PATCHES.md` (files, problem, fix, test, how found, bug fix or Null-only) and a diff in `specs/reference/azalea-patches/`. Then move the pinned `rev` in `Cargo.toml` to the new head.
- A patch that reproduces vanilla names the vanilla method (Mojang names, e.g. `AbstractContainerMenu#doClick`) and the Grim check it must pass. Read the source in `~/Desktop/null-reference/` (`vanilla-1.21.11-src/`, `paper/`, `grimac/`; see its `MANIFEST.md`). Describe behaviour in our own words; never copy Mojang, Paper or Grim code.
- Keep patches small and mostly additive, so rebases onto a new upstream stay cheap.
- `~/.cargo/config.toml` has a `[patch]` that builds Null against the local clone, so local fork edits take effect at once. The pushed head must equal what was tested.
- Vanilla item component hashes: `harness/vanilla-hash/` (`VanillaHash.java`, `ComponentDump.java`) prints vanilla's values; the fork's `azalea-inventory/tests/vanilla_component_hashes.rs` checks them.

## Test servers and the ability check

- Vanilla `harness/skeleton-server` (port 25567, RCON 25577) and Paper + Grim `harness/grim-server` (25568, RCON 25578), RCON password `skeleton`. Start with `scripts/start_server.sh` in each; wait for "RCON running" in `server/server.log`.
- `null-abilitycheck` runs scenarios by id (`--scenario all|A1,...`); commands are in HANDOFF. Server truth comes from RCON readback; every row also checks the server log for warnings and Grim alerts.
- Each scenario module uses its own test area (`Ctx::scene_in`) so runs can share a server; use a distinct `--account` per concurrent run.
- Paper cuts long `data get` replies short with "...": read NBT field by field.
- Grim flags clicks or closes while moving, sprinting or sneaking (`MultiActionsC`/`D`). Bots must not be op (ops are exempt from Grim).
- If another process is running the release binary, build into a separate `CARGO_TARGET_DIR`, then delete that directory.
- A heavy build on this machine during a Grim run can stall the bot's tick loop and draw `Timer` alerts; avoid running both at once.

## Code

- Every major function (skills, reflexes, controllers, entry points, anything a spec defines) has a one-line link to its spec: `// Spec: specs/B3-inventory-crafting.md §4.1`. Put this rule in every implementer brief; the verifier fails on a missing or dead link. (Code drifted from B5 §4.9 once because nothing pointed back at the spec.)
- Comments state the why in a line or two and point to a doc for more. No long design essays in code.
- Write code that reads like the surrounding code.

## Subagents

- Live runs are needed but slow: each runs strictly once (Allen, 2026-10-19). While working, the implementer runs only the rows it adds or changes. At the end, with the code final, it runs the full suite once on each server, keeps the logs, and pushes a fork patch only after that run passes; a failed row is fixed and only its module rerun, once. No repeat runs to double-check. The verifier reruns no suite: it checks the implementer's logs against the code and the server logs, and runs only what nobody ran yet (a negative control on the new rows, when the change needs one). The lead reads summaries and reruns nothing. Put this rule in every implementer and verifier brief.

- Sonnet subagents implement; size each task by importance and lines of code, about 500–800 lines, so it ends well under an hour (about 2,000 lines plus a cold Azalea build took an hour). Budget first Azalea builds and world setup apart from code, and warm the build cache in the first task. Keep work the milestone doesn't need out of the task. Prefer deterministic setup (RCON) over searching. Give each task one done-when.
- Don't verify finished subagent work yourself: hand it to a Haiku subagent, or Sonnet for live-server checks. The bar is correct in-game behaviour, not a line-by-line review.
- If a subagent fails about twice or churns, stop it and solve the problem directly.
