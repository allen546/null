# Hand-off: SA, the ability check (2026-10-17)

The next session starts here for SA. The plan and per-ability bar are in [specs/plans/ABILITY_CHECK.md](specs/plans/ABILITY_CHECK.md); the fork's patches in [specs/reference/AZALEA_PATCHES.md](specs/reference/AZALEA_PATCHES.md). Older combat leads stay in `specs/plans/COMBAT_SCAFFOLD.md`.

## Where things stand

- **Done and passing on Paper + Grim, zero Grim alerts:** A1–A8 (slots, breaking, pickup, placing, eating, shield, attack, attackers), A2S/A4S (every non-cube shape family: click on it, place it, break it), and a chain of 30 back-to-back breaks. About 200 scenario rows.
- **Verified 2026-10-17 evening, both servers on normal:** 195/195 on Paper + Grim and 195/195 on vanilla, run alone and run in parallel (first vanilla runs of A2S/A4S/A5–A8). An independent Sonnet verifier reran both, spot-checked the evidence against vanilla formulas, checked every `// Spec:` link, and ran the patch-10 control: with patch 10 reverted, `chain_of_30_breaks` on Grim draws 203 `FastBreak` alerts and the bot is kicked. Since then the runner prints that as a FAIL row instead of a panic. `chain_of_30_breaks` passed 5 more runs on vanilla and 14 on Grim.
- **Fork:** patches 5–10 on `null-main` of `allen546/azalea` (block actions with results, shape-aware clicks, attack cooldown fix, item use, attackers, held-button break timing). Null's `Cargo.toml` pins the head.
- **Rules settled today (Allen):**
  - Raw game capability goes in the fork, as normal Azalea APIs; `mc-adapter` only translates to `BotView`/`BotCommands` (F5 §4.1). Not a change to AD-F5.2: the boundary stands; only the planned `mc-act` and `mc-nav` crates were dropped.
  - Null targets vanilla servers only; the fork and adapter must also run cleanly on Paper + Grim (`harness/grim-server`), timing included, zero alerts (F5 §4.6, settled 2026-10-17 evening). A scenario passes only on both servers.
  - A bot never sends a click a vanilla client couldn't make; it refuses on the client with a reason.
  - Server-configurable timings are beliefs, not facts (example: the shield's raise-to-block delay).

## How to run

```
harness/grim-server/scripts/start_server.sh       # wait for "RCON running" in its server/server.log
harness/skeleton-server/scripts/start_server.sh   # the vanilla server, same
cargo build --release -p null-abilitycheck
./target/release/null-abilitycheck --server 127.0.0.1:25568 --rcon-port 25578 \
  --server-log harness/grim-server/server/server.log --account null_check --scenario all
./target/release/null-abilitycheck --server 127.0.0.1:25567 --rcon-port 25577 \
  --server-log harness/skeleton-server/server/server.log --account null_check --scenario all
harness/grim-server/scripts/stop_server.sh; harness/skeleton-server/scripts/stop_server.sh
```

A scenario passes only when it passes on both servers (F5 §4.6). The two runs can go at the same time (separate servers, worlds and logs); each takes about 10 minutes. Rows print as they finish; if the bot stops answering, the runner prints a `RUN <module>_aborted` FAIL row and stops.

`--scenario A1,A5,...` runs a subset; `SHAPES_ONLY=<text>` reruns the shape rows whose name contains the text. Several runs can share the server with different `--account` names (the log check skips other `null_` accounts) as long as each scenario module uses its own test area (`Ctx::scene_in`).

## Open

**Decided 2026-10-17 evening (Allen):**
- F5 §4.6: vanilla servers only as targets; the Azalea-side code must work cleanly on Paper with an anticheat, because those mechanics are well documented. F5 §4.6, ARCHITECTURE, ABILITY_CHECK and the Grim README updated.
- Grim server difficulty: normal (normal or hard allowed; normal leaves damage unscaled). A5 now lowers food with Hunger and checks the +4 food rise exactly; A6 turns natural regeneration off for its damage rows.
- Reference sources: the plan is in [specs/plans/REFERENCE_SOURCES.md](specs/plans/REFERENCE_SOURCES.md) (Mojang names first, Paper source, mineflayer, Meteor Client, Baritone, GrimAC).

**To do, in order:**
1. **Reference sources**: [specs/plans/REFERENCE_SOURCES.md](specs/plans/REFERENCE_SOURCES.md); then each fork patch cites the vanilla method it reproduces.
2. **Shield delay as a belief (A6):** prior from the held shield's `blocks_attacks.block_delay_seconds`; update from hits taken with the shield up; A6 measures the server's delay and checks the bot's belief, plus a shield given a non-default delay by RCON.
3. **A9–A12:** inventory clicks, chests, crafting by clicks, respawn (tasks 3 and 4 in ABILITY_CHECK.md).

**Known gaps and doubts:**
- Interaction-shape boxes for hopper, composter and scaffolding (patch 6) were written from memory; only the cauldron is confirmed by a test. Check against the vanilla source.
- `click_replaces` doesn't cover stacking blocks (candles, sea pickles, turtle eggs).
- Not tested: the "no ack" timeout path, off-hand use, bows and tridents, lily pad and sign placement, break rows for a door's upper half, a bed's head and a double slab, creative mode.
- **For S5 skills:** the adapter keeps one record per action kind (`last_break`, `last_place`), and a break is `Done` only when the server's ack is read. A new break command replaces the record, so a skill that starts the next break before the last ack loses that result. This caused the one hang the verifier saw in `chain_of_30_breaks` on vanilla; the test now waits for the ack. A skill must do the same, or the adapter must keep a short history: decide when S5's `fell` is built.
- The adapter reports `finish_tick` from the tick the break was commanded; when the fork holds the start for the destroy delay (patch 10), the absolute tick is off by that wait (the duration is right).
- Server-side timing (A2 break, A5 eating) is bracketed by two RCON clock reads, so the window is about ±2 ticks; the bot-side counts are exact.
- One full run had all A4S rows fail after `null_blk fell from a high place` while another agent's bot was active; never reproduced.
- **Two intermittent faults, each seen once, both during parallel runs, not reproduced since** (2026-10-17 evening):
  - Grim `Timer x2, TimerLimit x1` on one `chain_of_30_breaks` (the breaks themselves were exact). Not reproduced in 11 more Grim A2 runs beside a vanilla suite, nor in 3 runs with all 10 cores busy (`yes > /dev/null`, which only tests CPU contention, not memory or GC load). Two suspects: the bot's tick loop stalled and then caught up (the fork runs up to 3 ticks per 50 ms until back in step), or the Paper JVM paused (GC) so Grim read queued packets as a burst. Rows with a Grim alert now print the client's longest tick gap and its catch-up ticks: a long gap means the client, steady ticks mean the server (then turn on GC logging for the Grim server).
  - One vanilla full run where every block click from A2 on was refused on the client with "crosshair does not hit it" (174 rows); A1 passed and the bot saw every scene's blocks. A2–A4 alone right after passed 13/13. The bot had logged in mid-air at the end point of the previous run (539.5, 200.96, 352.1, over the A5–A8 area whose floor the cleanup removes); earlier runs started the same way and passed. A failed row with a client refusal now prints the bot's own position, look, ground state, velocity and its view of the target block.
- Patches 5–10 have no `null/<name>` per-fix branches: they depend on each other (6 on 5, 8 on 3), and the branches only existed for upstream PRs.
- Old combat leads (positions off by up to 2.9 blocks, retreat rate, few shield-ups, low hit rate in crowds, pit standoff) are in COMBAT_SCAFFOLD.md, unchanged.
