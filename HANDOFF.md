# Hand-off: SA, the ability check (2026-10-17)

The next session starts here for SA. The plan and per-ability bar are in [specs/plans/ABILITY_CHECK.md](specs/plans/ABILITY_CHECK.md); the fork's patches in [specs/reference/AZALEA_PATCHES.md](specs/reference/AZALEA_PATCHES.md). Older combat leads stay in `specs/plans/COMBAT_SCAFFOLD.md`.

## Where things stand

- **Done and passing on Paper + Grim, zero Grim alerts:** A1–A8 (slots, breaking, pickup, placing, eating, shield, attack, attackers), A2S/A4S (every non-cube shape family: click on it, place it, break it), and a chain of 30 back-to-back breaks. About 200 scenario rows.
- **Fork:** patches 5–10 on `null-main` of `allen546/azalea` (block actions with results, shape-aware clicks, attack cooldown fix, item use, attackers, held-button break timing). Null's `Cargo.toml` pins the head.
- **Rules settled today (Allen):**
  - Raw game capability goes in the fork, as normal Azalea APIs; `mc-adapter` only translates to `BotView`/`BotCommands` (F5 §4.1). Not a change to AD-F5.2: the boundary stands; only the planned `mc-act` and `mc-nav` crates were dropped.
  - Paper + Grim (`harness/grim-server`) is the reference server, timing included, because it is stricter. Vanilla `skeleton-server` is secondary.
  - A bot never sends a click a vanilla client couldn't make; it refuses on the client with a reason.
  - Server-configurable timings are beliefs, not facts (example: the shield's raise-to-block delay).

## How to run

```
harness/grim-server/scripts/start_server.sh     # wait for "RCON running" in harness/grim-server/server/server.log
cargo build --release -p null-abilitycheck
./target/release/null-abilitycheck --server 127.0.0.1:25568 --rcon-port 25578 \
  --server-log harness/grim-server/server/server.log --account null_check --scenario all
harness/grim-server/scripts/stop_server.sh
```

`--scenario A1,A5,...` runs a subset; `SHAPES_ONLY=<text>` reruns the shape rows whose name contains the text. Several runs can share the server with different `--account` names (the log check skips other `null_` accounts) as long as each scenario module uses its own test area (`Ctx::scene_in`).

## Open

**Decisions for Allen** (also in OPEN_FOR_ALLEN.md §2):
- F5 §4.6 still says "pure vanilla servers only; Paper out of scope". Proposed: targets are vanilla and Paper/Spigot servers, possibly with an anticheat; any server-configurable mechanic is a per-server belief (a prior from the item or the mechanics table, refined by observation); zero Grim alerts is the bar.
- Grim server difficulty: peaceful now. Proposed default: normal (mob spawning is already off; duel matches use normal). Peaceful hides the food rise in A5 and its regeneration blurs A5/A6 numbers.

**To do, in order:**
1. **A verifier run over the whole suite** after patches 6–10 (none yet). Include: revert patch 10 locally and confirm `chain_of_30_breaks` then fails on Grim `FastBreak` (proves the test catches it).
2. **Reference sources** (set up later, Allen 2026-10-17), into a local reference folder outside the repo, each pinned by version:
   - vanilla 1.21.11 client and server, decompiled with Yarn mappings;
   - Paper 1.21.11 source;
   - Meteor Client and Baritone sources (more useful than mineflayer's);
   - then each fork patch cites the vanilla method it reproduces.
3. **Shield delay as a belief:** prior from the held shield's `blocks_attacks.block_delay_seconds`; update from hits taken with the shield up; A6 measures the server's delay and checks the bot's belief, plus a shield given a non-default delay by RCON.
4. **A5 food rise** on a non-peaceful server (after the difficulty decision).
5. **A9–A12:** inventory clicks, chests, crafting by clicks, respawn (tasks 3 and 4 in ABILITY_CHECK.md).

**Known gaps and doubts:**
- Interaction-shape boxes for hopper, composter and scaffolding (patch 6) were written from memory; only the cauldron is confirmed by a test. Check against the vanilla source.
- `click_replaces` doesn't cover stacking blocks (candles, sea pickles, turtle eggs).
- Not tested: the "no ack" timeout path, off-hand use, bows and tridents, lily pad and sign placement, break rows for a door's upper half, a bed's head and a double slab, creative mode, and any of A2S/A4S/A5–A8 on the vanilla server.
- The adapter reports `finish_tick` from the tick the break was commanded; when the fork holds the start for the destroy delay (patch 10), the absolute tick is off by that wait (the duration is right).
- Server-side timing (A2 break, A5 eating) is bracketed by two RCON clock reads, so the window is about ±2 ticks; the bot-side counts are exact.
- 28 shape break rows still use Haste 255 (added to dodge `FastBreak` before patch 10); they could now run without it.
- One full run had all A4S rows fail after `null_blk fell from a high place` while another agent's bot was active; never reproduced.
- A1 evidence prints `SelectedItemSlot = (… ms)` without the value (the check compares it).
- Patches 5–10 have no `null/<name>` per-fix branches: they depend on each other (6 on 5, 8 on 3), and the branches only existed for upstream PRs.
- Old combat leads (positions off by up to 2.9 blocks, retreat rate, few shield-ups, low hit rate in crowds, pit standoff) are in COMBAT_SCAFFOLD.md, unchanged.
