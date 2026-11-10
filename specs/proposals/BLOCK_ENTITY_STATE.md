# Block-entity state in the fork

> **Status (2026-11-10): scope adopted by Allen, not built.** Allen, 2026-11-10: do the audit and the storage design now, with the shulker box as the first kind; the piston stays refused until `mine` or building needs it. Nothing here has run. The kind lists in §3 come from a scan of the vanilla block classes and are candidates until the audit (§7 step 1) confirms them. It needs the client level model ([CLIENT_LEVEL_MODEL](CLIENT_LEVEL_MODEL.md)) first.

## 1. Why

The fork keeps no block-entity state. `ClientboundBlockEntityData` and `ClientboundBlockEvent` are read and dropped (`azalea-client/src/plugins/packet/game/mod.rs`, `block_entity_data` and `block_event`), and the block entities in a chunk packet are not kept.

A vanilla client keeps this state and decides from it. So for some blocks Null cannot compute what a vanilla client would send. Today the safe envelope covers the known case (a moving piston disconnects the bot). The other cases are not listed anywhere.

## 2. A correction: the client simulates the piston

The burst note on piston faces said the client cannot know a moving piston's progress, because the fork keeps no block-entity state. The second half is true of the fork. The first half is not true of vanilla.

The server sends one `ClientboundBlockEvent`. `ClientPacketListener#handleBlockEvent` calls `Level#blockEvent`, and `PistonBaseBlock#triggerEvent` runs `moveBlocks` on the client. That writes the moving-piston blocks and creates their block entities in the client's level. The client then ticks each `PistonMovingBlockEntity` itself: progress, the push on entities, and the final block.

So the piston is client-side simulation, which Null computes (F5 §4.6, "What Null computes"). It is not hidden server state. It is also larger than "medium": it needs the piston structure rules (`PistonStructureResolver`), the moving collision shape, and the piston mover in the entity move.

## 3. Principle and classes

Null stores only the block-entity state that a client decision reads. It skips display.

| Class | Candidate kinds | What the client decides from it | Null |
|---|---|---|---|
| Motion | moving piston, shulker box | The collision shape depends on the entity's progress, and its tick pushes entities. Started by a block event. | Store, tick, model. Until a kind is modelled, the envelope refuses it |
| Use result | signs and hanging signs, campfire, chiseled bookshelf, decorated pot, shelf, beehive, vault, jukebox, copper golem statue | The result of a use on the block. The result decides the swing and whether the click falls through to the item or the off hand, so it changes the packets | Store the fields the use reads. Until a kind is modelled, the use is refused on the client |
| Display | chest and ender chest lid, bell, banner, skull, beacon, spawner, and the rest | Nothing the client sends | Store nothing |

Notes on the list:

- A chest is display class. Its menu and contents come from the server.
- Blocks that only operators can use (command block, structure block, jigsaw) are out of scope: bots are not operators.
- The scan matched block methods that read a block entity in a shape, use, step or event path. It can miss a kind that reads the entity through a helper, and it can include a kind whose read is on the server branch only. The audit settles both.

## 4. Storage

- One map per chunk, per bot: cell to a typed value. Only the motion and use-result classes have values.
- It lives beside the chunk's section pointers in the bot's own `World`. It is not in the section store: sections hold block states only, and the store compares sections by content.
- It is not shared between bots in this version. Expected memory is small: motion entities exist for a few ticks, and use-result kinds are few per chunk. Measure before sharing.
- Each value is a parsed struct with only the fields a decision reads, not the raw NBT.

## 5. Lifecycle, as in vanilla

| Event | Effect | Vanilla method |
|---|---|---|
| Chunk packet | The chunk's block-entity list replaces the map | `LevelChunk#replaceWithPacketData` |
| `ClientboundBlockEntityData` | Update one value, if the kind at the cell matches | `ClientPacketListener#handleBlockEntityData` |
| A block write, predicted or from the server | A new block without an entity removes the value; a new block with an entity creates a default one | `LevelChunk#setBlockState` |
| `ClientboundBlockEvent` | The block's event runs on the client (a piston moves its blocks; a shulker box starts to open or close) | `Level#blockEvent`, `BlockBehaviour#triggerEvent` |
| Chunk forget, new level | The map goes with the chunk | |
| Each game tick | Ticking entities advance, after the player's own tick | `Minecraft#tick`: `ClientLevel#tickEntities`, then `Level#tickBlockEntities` |

The third row is why the client level model comes first: a predicted write must create and remove entities in the bot's own world, and the ack must restore them.

## 6. First kind: the shulker box

Bots use shulker boxes for logistics, so a bot stands beside one that opens. In vanilla, `ShulkerBoxBlock#getShape` takes the box from the block entity (`ShulkerBoxBlockEntity#getBoundingBox`), which grows with the lid's progress. The entity's tick moves entities out of the lid's way (`#moveCollidedEntities`). The open and close arrive as a block event (`#triggerEvent`).

Not checked yet: what shape the fork gives a shulker box today, and whether a bot can be inside the lid's path in the A10 `shulker_box` row. If the fork uses a fixed shape, a bot on or against an opening box moves differently from vanilla. That is a departure to fix or to refuse.

## 7. Build order

1. **Audit.** For every block with an entity, read the client-side paths (shape, use, step, event, tick) and class it. Done when §3 is a full table with the vanilla method for each kind, and each unmodelled kind names its refusal.
2. **Envelope check.** Done when every motion and use-result kind that is not modelled has a refusal in the fork and a row, or a gap line in OPEN §4.
3. **Store and lifecycle** (§4, §5), with no kind modelled. Done when a simulation test shows a value arriving by chunk packet, by data packet and by a block write, and leaving by a block write and a chunk forget.
4. **Shulker box:** the shape from progress, the tick, and the push. Done when a simulation test matches the vanilla numbers, and a row passes on both servers with a bot against an opening box.
5. **Signs,** as the first use-result kind, when a skill needs to click near them.
6. **Piston:** only when `mine` or building needs it (Allen, 2026-11-10). Until then the disconnect stays.

Steps 3 to 6 are fork patches under the usual gate. Step 4's row is a new A10 row; the vanilla diff covers it once the row exists.

## 8. Open questions

- Does a block-entity value in a chunk packet for a moving piston give the progress, for a bot that joins mid-move? Read `PistonMovingBlockEntity#getUpdateTag` in step 1.
- Does Grim model the shulker lid as uncertainty, as it does the piston (`PistonData`)? This decides how exact step 4 must be to draw zero alerts.
