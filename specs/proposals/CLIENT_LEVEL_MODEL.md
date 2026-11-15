# Client level model: the bot's `World` is its `ClientLevel`

> **Status (2026-11-15): built in fork patches 76 and 77** (stack `patch-stack-burst`, not pushed). Allen adopted the design on 2026-11-10. Patch 76 is the level model; patch 77 keeps the level on a same-dimension respawn. The rules now live in B6 §4.10 and §4.6; this file is the record of the design. Offline tests only, no live run yet.

## 1. Why

Fork patch 53 keeps a bot's predicted block in a `predicted` field of its `BlockStatePredictionHandler`. A reader sees the prediction only if it reads through `BotWorld`. Patch 53 needed this, because all bots shared one `World` and a prediction written there leaked to other bots.

AD-F5.6 gives each bot its own `World`. The reason for the overlay is then gone, but the own-worlds patch keeps it. The overlay has three costs:

- **It is not vanilla's model.** OPEN §4 "Chunk re-sent during a pending click" is a departure that exists only because of this difference.
- **A reader can forget it.** Patches 59 (`auto_tool`, `container`) and 62 (unloaded-chunk gravity check), and the `travel.rs` gap, are each one reader that read the world directly. Each was found one at a time.
- **Neighbour reactions need a second path.** See §5.

## 2. What vanilla does

The handler (`BlockStatePredictionHandler`) holds a sequence number and one entry per pending cell: the sequence, the last server state, and the player position at the first write. The level holds the predicted block. Four rules cover every case.

| # | Event | Rule | Vanilla method |
|---|---|---|---|
| 1 | A block write inside a prediction scope | Read the old state. Write the level. If the write changed the cell, keep the old state for that cell with the current sequence. If the cell already has an entry, change only its sequence. | `ClientLevel#setBlock`, `BlockStatePredictionHandler#retainKnownServerState` |
| 2 | A server block update or section update | If the cell has an entry, change only the entry's state. The level is not written. If not, write the level. | `ClientLevel#setServerVerifiedBlockState`, from `ClientPacketListener#handleBlockUpdate` and `#handleChunkBlocksUpdate` |
| 3 | Ack N | For each entry with sequence ≤ N: remove it. If the level's state differs from the entry's state, write the entry's state. Then, if the player's box meets that state's collision shape, move the player to the kept position. | `BlockStatePredictionHandler#endPredictionsUpTo`, `ClientLevel#syncBlockState` |
| 4 | A chunk packet, or a chunk forget | Replace or drop the chunk. The handler does not change. | `ClientPacketListener#updateLevelChunk` |

**Prediction scopes.** `MultiPlayerGameMode#startPrediction` has six call sites in four methods: `startDestroyBlock` (two), `continueDestroyBlock` (two), `useItemOn` and `useItem`. Every block write inside a scope follows rule 1. That includes a write the click causes on a block that is not the target: a door, a lever, a trapdoor, a fence gate.

**Corners that follow from the rules.**

- *Chunk re-sent while a cell is pending.* Rule 4 leaves the entry. At the ack, rule 3 writes the kept state over the packet's block. This is OPEN §4 edge a. The design reproduces it with no special code.
- *Chunk forgotten while a cell is pending.* At the ack, the level reads `void_air` for the cell, which differs from the kept state. The write does nothing, because the client's empty chunk refuses it (`EmptyLevelChunk#setBlockState`). The collision test and the snap still run.
- *The same cell predicted twice.* The entry keeps the first old state and takes the newer sequence. So the ack of the first action does not settle the cell.
- *A new level* (dimension change): a new handler, sequence from 0. The fork has this already.

## 3. The design

After the own-worlds patch, the fork follows the four rules, and nothing else.

- A prediction writes the bot's own `World`.
- `BlockStatePredictionHandler` holds the sequence, the acked sequence, and per cell: sequence, server state, player position. The `predicted` field goes.
- `set_predicted`, `predicted`, `clear_predictions_in_chunk` and `has_override_in` go.
- `BotWorld` goes. Its about 28 call sites in the fork read the `World`. The `BlockView` trait can stay as the read interface, with one kind of reader.
- In Null, the `pending` field in `mc-adapter/src/view.rs` and its plumbing in `host.rs` go.
- Block updates for a pending cell no longer write the world (rule 2). Today `block_update.rs` writes the world and the entry.

**One new accessor: the server's state of a cell.** `server_block_state(pos)` returns the entry's state if the cell is pending, else the world's state. One user needs it today: patch 54 judges a sent place or break on the server's data (B6 §4.6), and the cell can hold a later prediction of the same bot at that ack. No vanilla-parity code calls it.

This turns the risk around. Today a reader that forgets `BotWorld` departs from vanilla, and may draw a Grim alert. After the change, a reader that forgets `server_block_state` reads a prediction as fact for at most one round trip, and only Null's own judgement is affected.

## 4. Cost

A prediction copies one section and stores it in the section store. The section is private to the bot until the server's result arrives.

| Case | Section copies |
|---|---|
| Action confirmed by the server | 1 (the prediction; the server's update changes only the entry) |
| Action refused | 2 (the prediction, then the rollback) |
| 16 instant breaks in one tick | 16, because each prediction must be readable in the same tick |

Not measured. The 16-break chain (A13 `instamine_chain_one_break_per_tick_all_done`) is the case to time. If it is too slow, the fix is a per-tick open section, not a return to the overlay.

Memory: one private section per pending cell's section, for about one round trip.

## 5. What it makes possible

`Level#setBlock` runs neighbour shape updates on the client too (`BlockState#updateNeighbourShapes`). A placed fence joins its neighbours; a broken support removes the torch on it. Each of those writes is inside the scope, so rule 1 keeps it. Place prediction stage 3 (B6 §4.10, "reacting neighbours") then needs only the shape rules, not a storage change. This design does not build stage 3.

## 6. What it does not do

- The `Unknown` place families stay `Unknown`.
- It does not add the writes that the fork does not predict today (§7 step 1 lists them).
- It does not change how a result is judged, apart from the accessor.

## 7. Build order

One fork patch, directly after the own-worlds patch. Each step has one done-when.

1. **Audit the scopes.** List every block write the fork makes for a break, a place and a block use, against the vanilla scopes in §2. Done when each vanilla write is marked "the fork writes it", "the fork refuses the action" or "gap", and each gap is in OPEN §4.
2. **Handler and writes.** Remove the `predicted` field, write predictions to the world, and apply rules 2 and 3 as written. Done when the fork's prediction tests pass, with the edge-a test changed to expect the vanilla result.
3. **Readers.** Remove `BotWorld`. Add `server_block_state` and move the patch 54 judgement to it. Done when no `BotWorld` remains in the fork or in Null, and Null builds.
4. **Tests** (§8). Done when each new test fails on its negative control.
5. **Time the 16-break chain** against the stack head before this patch. Done when the number is in the ledger row.

The patch maps to `--scenario all`, so the pair that follows covers it.

## 8. Tests

| Test | Checks | Negative control |
|---|---|---|
| Existing rollback and settle tests in `interact/mod.rs` | Rules 1 to 3 | Skip the keep in rule 1: the rollback tests fail |
| Edge a, changed | A chunk packet lands on a pending cell with a different block; after the ack the cell holds the kept state | Clear the entry on a chunk packet: the test fails |
| Forgotten chunk, new | The ack writes nothing, and the snap test still runs | Skip entries in unloaded chunks: the snap assertion fails |
| Same cell twice, new | The first ack leaves the cell pending; the second settles it against the first old state | Replace the entry on the second write: the test fails |
| Judgement, new | With a later prediction in the cell, the first action is judged on the entry's state | Judge on the world: the test reads the prediction as the result |
| L4 contract tests and `shared_world_prediction.rs` | Bot B never reads bot A's prediction | Already in place: a shared `World` fails them |

Live: the pair. SW1 must draw zero alerts.

## 9. Docs changed when built (2026-11-15)

- B6 §4.10 "Write and roll back": the four rules, the scopes and the respawn rule (patches 76 and 77) replace the overlay text.
- B6 §4.6: the patch 54 judgement reads `server_block_state`.
- Ledger: rows 76 and 77 are new; row 76 supersedes row 53.
- `plans/LOAD_CONTRACT.md` design item 1: the overlay is gone, and the own-world uuid index is named.
- OPEN §4: edge a is closed; the gaps this audit found (break neighbours, break side writes, block use, item use, place second parts) stay listed there.
