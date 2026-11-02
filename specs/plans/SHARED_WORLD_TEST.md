# Shared-world prediction test

Queued 2026-11-02 (Allen). Runs after the VD2b departure fixes, before `mine`.

## Background

Bots of one swarm share one Azalea `World`. A bot writes its predicted block into that shared world: places at `azalea-client/src/plugins/block_actions.rs` (`handle_place_block_queued`, patch 33) and breaks at `mining.rs` (inherited from upstream). The rollback in `interact/mod.rs` (`end_prediction_up_to`) writes there too. So bot B can read bot A's unconfirmed block about one round trip before the server's update could reach B, and for a refused action B reads a state that never existed. No spec records a decision on this; B6 §4.10 treats the bot's world as private.

The companion bot has no skill and stands 2.5 blocks from the main bot, so 447/447 on the Pi pairs says little about this.

## Details

- **Where:** places in `azalea-client/src/plugins/block_actions.rs` (`handle_place_block_queued`, patch 33); breaks in `mining.rs` (inherited from upstream; upstream `main` has the same write, and a search of its GitHub issues and PRs found no decision); rollback in `interact/mod.rs` (`end_prediction_up_to`).
- **What can go wrong:** bot B reads bot A's block about one round trip before the server's update could reach B. For a refused action, B reads a state that never existed, and only A gets the snap-back. B can then collide or fall against Grim's view (`Simulation`), click a block it could not know about (a packet a vanilla client could not send), or decide wrongly. B's server update can also overwrite A's pending prediction, because B has no retained state for that cell.
- **Second leak, same cause:** the first bot to receive a server update writes it for all bots, so B can see a true update before B's own packet arrives. The overlay below does not fix this.
- **Why nothing caught it:** no spec weighed it. B6 §4.10 treats the bot's world as private, and its movement zone covers only the acting bot. `COMPANION_BRIEF.md` defines shared-world bugs as entity identity bugs. The companion has no skill and stands 2.5 blocks away, so the 447/447 pairs say little here. It is the same family as patches 1, 1b, 2, 40, 41, 43 and 44; patch 38's read of the shared chunk map is a related open gap.
- **Why it may not matter:** the window is short, harm needs contact with that cell during the window, most actions end up accepted, and claims may keep bots apart. `mine` and building put bots side by side, so these reasons are weakest there.

## Fix if proven

**Fix if proven (lead's recommendation, not decided):** a per-bot overlay: the shared `World` holds server truth only, each bot keeps its short list of pending cells and reads it first, and the ack deletes the entry. Bots with nothing pending pay one empty-check per read (not measured). The cost is one-time: `azalea-physics` passes `&World` through at least 20 signatures, and collision and `clip.rs` read below `World`, so each read path needs a test. Rejected: per-bot section copies (memory), swapping the overlay in during a bot's tick (parallel systems and pathfinder threads could read it). A claim gap is a stopgap only; it avoids the bug without fixing it.

## Goal (Allen, 2026-11-02)

The test aims to prove that this is a real issue. It is not a neutral survey: build the setup most likely to show harm. It gets 3 chances.

## Chances

- One chance is one live run of one design below, on the server that design names. This count is Allen's exception to "each live run runs strictly once" for this test only.
- Stop at the first chance that proves the issue. Do not spend the rest.
- A run that fails for a harness reason (a row setup bug, a run cut short) still counts, unless the run never reached the action under test. Report any such case.
- After 3 chances without proof, the result is "not proven in 3 chances", not "safe". Record it and go on to `mine`.

Designs, strongest first. An implementer may reorder or replace one only with a reason written in the result section.

1. **Break under a bot (Paper + Grim).** Bot A breaks the block that bot B stands on. B may read air and start to fall before the server's update could reach it. Expected harm: Grim `Simulation` on B.
2. **Refused place in a bot's path (Paper + Grim).** A's place is refused by the server while B walks into the cell. B collides with a block that never existed, and gets no snap-back. Expected harm: Grim `Simulation` on B, or a position that differs from the same walk with no place.
3. **Click on another bot's prediction (Paper + Grim, added latency if it helps).** B aims at and clicks A's predicted block before the server's update could reach B. Expected harm: a packet a vanilla client could not send, a Grim alert, or a wrong refusal by B. Latency may come from the existing tap or from any method that needs no network configuration change.

## Proof

All of these are needed:

- **The harm:** a Grim alert on B, or a packet B sends that a vanilla client could not send, or a wrong action result on B, read from the logs and RCON.
- **The cause:** a log line shows B read the cell while A's prediction for it was pending.
- **The control:** the same setup with A and B in separate swarms, so separate `World`s, does not show the harm. The control run does not use up a chance. It runs once for each chance that shows the harm.

Also write one fork simulation test first: two clients in one `World`, A predicts, no server reply, and B's collision reads the cell. It proves the code fact, not the harm, and uses no chance. Record it with the expected result stated.

Do not weaken a check to get or avoid a result. Do not build the overlay in this task. Fix options are for the lead and Allen once the result is in.

## Result

Not run yet.
