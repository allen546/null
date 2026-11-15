# Load contract and everyday testing: correct by design, checked without a live suite

Status (2026-11-04): direction set by Allen, 2026-11-03: this becomes the new faster-testing method. The old live tests stay, but they probably stop being part of everyday dev testing. The decisions below are made, the audit is written and the swarm world design is approved (Allen, 2026-11-04), but no test or fix is built. The one-at-a-time pair stays the gate until this method is proven.

## Testing tiers

| Tier | What runs | Answers | When |
|---|---|---|---|
| Everyday | Fork tests with no server: contract tests, timeline tests, trace replays | Did the change break the contract or a recorded behaviour? | Every change |
| Targeted | Selective live modules from the module map | Does this change work on real servers? | When a change touches server-facing behaviour, as a change's verifier decides from the map |
| Occasional | The full pair; mimicked load on Null and a vanilla client; real load | Long-lived state across modules, unknown problems, what the servers really do | To be decided once the everyday tier is proven; the current cadence stands until then |

**The gap this must close.** Contract tests show that behaviour does not change with timing. They do not show that the behaviour is right. Today the live rows show that. So the everyday tier also needs **trace replays**: the vanilla diff already records a vanilla client's packets by client tick (`vd-tap`). A recorded row becomes a fixture: the same inputs and inbound packets go to the bot with no server, and the packets it sends must match the recording. A replay checks what the client decides. It cannot check what a server would do with those packets, so the targeted and occasional tiers keep that job. Whether inbound packets from a recording can drive the bot this way is not yet shown.

**Proof before it replaces anything.** The everyday tier must catch what the old suite caught:
1. Reintroduce known past bugs, one at a time (for example patch 10 off, `craft_took_result` a no-op, patch 53 off, patch 54 off). The everyday tier must fail on each. A bug it misses is listed, and that kind stays in a live tier.
2. Run it beside the one-at-a-time pair for several fork pushes. Every pair failure caused by Null must also fail the everyday tier, or be listed as live-only.
3. Allen then decides what leaves the everyday path. Until then, nothing does.

## Why

Null must behave like vanilla clients with and without load. The ability check and the vanilla diff run on quiet servers, so they say little about load. Parallel runs on the Pi showed this: A23 logged `moved too quickly` in 3 of 6 runs, and Grim drew start-up alerts in 2 of 5 (`SUITE_SPEED.md`, Findings). The rows could not tell a Null bug from a stall that a vanilla client would also suffer.

A complete live load suite is not possible: real load is random, and the possible timings have no end. So this plan states a contract that the design must hold. It tests the contract directly and mimics load where the contract cannot answer.

## The model

A bot's client tick is a function: (state, packets queued since the last tick, inputs) to (new state, packets to send). A vanilla client works the same way. Load does not change this function. It changes only the timeline: when ticks run, and which tick a packet lands in.

| Load | What it becomes under the contract |
|---|---|
| Server stall | Packets arrive late or in bursts |
| Client stall | The tick clock (L1), plus late packets |
| Swarm load | A client stall on all bots at once, if bots are isolated (L4) |

## The contract

| # | Promise | Vanilla method | Status |
|---|---|---|---|
| L1 | Client ticks follow vanilla's clock: the ticks due come from elapsed time, at most 10 run back to back, and the rest are dropped | `Minecraft#runTick`, `DeltaTracker.Timer#advanceGameTime` | Built, fork patch 23, AD-F5.3 |
| L2 | Queued packets are handled in arrival order, before that frame's ticks | `Minecraft#runTick` (`runAllTasks`) | Audited at 40b6af0: the order against ticks holds. Within the queue, chunk packets, block updates and respawn are reordered against other packets (three low breaches; see Audit) |
| L3 | What a bot sends depends on no wall clock, apart from L1 and an allowed list | | Audited at 40b6af0: holds. Wall clocks only in the allowed list (chunk batch rate, chat timestamp and key refresh); pathfinder time limit and reconnect backoff sit above the contract. Per-frame senders not fully read (see Audit) |
| L4 | No bot reads another bot's state | | **Broken**, audited at 40b6af0. The first bot to receive a server update writes it for all bots (OPEN §2), and about ten read sites that decide packets read it. Shared mob updates (`UpdatedBy`) and patch 38 in `travel.rs` are the other breaches (see Audit). Fix approved: [Swarm world design](#swarm-world-design-allen-approved-2026-11-04) |
| L5 | A bot reports success only with server evidence | | Built, fork patch 54 (pushed `40b6af0`) |
| L6 | Code above the contract only chooses inputs. It may choose differently under load, for example the pathfinder's time limit, but every packet stays legal | | Design rule, to confirm |

## How each part is checked

1. **Audit.** Read the code for L2, L3 and L4, and list every breach. Each breach is fixed or added to an allowed list with its vanilla method. The audit so far is one skim.
2. **Contract tests in the fork, with no server.** A fake clock and injected packets (`RawConnection::injected_clientbound_packets`) run one fixed timeline. The packets sent must be identical when the bot is alone or in a swarm, and stalled or not, as far as L1 allows. A negative control must fail each test.
3. **Timeline tests in the fork.** Quiet runs see only narrow timings, so this code gets bursty timelines: block acks and prediction, item use (`ItemInUse::server_ticks`), teleport acks and respawn.
4. **Mimicked load, on Null and on a real vanilla client.** The same disturbance goes to both, on both servers, for a short list of timing-sensitive rows (the list in step 3). An alert both draw has its proof. An alert only Null draws is a Null bug. A vanilla client has GC pauses of its own, so the comparison allows for them.

| Disturbance | How | Mimics |
|---|---|---|
| Whole server pause | `SIGSTOP` and `SIGCONT` for an exact time | GC stop-the-world on the server |
| Real GC | `jcmd <pid> GC.run` (length not controlled) | The same, as a check on the pause |
| Main thread stall | A forced `fill` (ASTALL), or a small Paper test plugin | Chunk work and other main-thread stalls; network threads keep running |
| Client pause | `SIGSTOP` on the bot's or the vanilla client's process | A client stall |
| Swarm stall | Pause the Null process, or a test hook that makes one bot's tick slow | Swarm load |
| Late or bursty packets | `vd-tap` holds packets, then releases them together | A slow network or a server burst |

The proxy runs as an ordinary program, so no network setting changes on any device.

5. **Real load, now and then.** It finds what nobody thought to mimic, as the shared RCON buffer showed. It is a search for new problems, not a gate.

## Measure first

"Load" may be GC, chunk work or CPU contention. Nothing yet separates them. Two small changes come first:

- Server GC log: `-Xlog:gc*,safepoint` in `run_server_suite.py`. Every pause is then logged with its length.
- A23 prints the longest gap between server replies during the fall (drafted in `.worktrees/faster-suite`, not built).

**Order (Allen, 2026-11-03):** this runs in parallel with VD3 and finishes before `mine`.

## Decisions (Allen, 2026-11-03)

- The contract L1 to L6 is accepted.
- The second leak (L4) is a must-fix before `mine`, no longer "fix only on harm".
- The second leak is fixed by own worlds with shared sections (Allen, 2026-11-04). Problem: [SWARM_WORLD_PROBLEM](SWARM_WORLD_PROBLEM.md). Record: [AD-F5.6](../DECISIONS.md#ad-f56-how-do-the-bots-of-one-process-hold-the-world).
- The per-alert rule stays (F5 §4.6). Mimicked load supplies the proof for the rows it covers. A row failing during a measured stall is reported as "load, row not judged" and rerun, not passed.

## Audit (40b6af0)

Read-only audit of fork commit 40b6af0 (`~/Desktop/null-spikes/azalea`) and the Null adapter, 2026-11-03. "Verified" means I read the code path. "Inferred" means I reasoned from the code and ran nothing. Nothing was built or run. "Matters" says whether the breach could change a packet that a bot sends.

### L2: packets in arrival order, before the frame's ticks

| Finding | Where | Vanilla | Verdict | Matters |
|---|---|---|---|---|
| Frame order: clock advance, then the Update schedule with packets, then the due ticks | `client.rs` `run_schedule_loop` (about 182-202) | `Minecraft#runTick` | Holds (verified) | No |
| `read_packets` drains each socket in order and calls `process_packet` at once; bots run one after another | `plugins/connection.rs` (42, 152, 280) | `runAllTasks` | Holds (verified) | No |
| Most packet handlers act at once inside `process_packet`: teleport (and its accept and move reply), keep-alive, ping (the pong observer), respawn, forget chunk, block-changed ack | `packet/game/mod.rs` 495, 1117, 1509, 1629, 1469, 1310; `pong.rs` 26 | `ClientPacketListener#handleMovePlayer`, `handleKeepAlive`, `handlePing`, `handleRespawn` | Holds (verified) | No |
| Chunk data is not applied at once. The handler writes `ReceiveChunkEvent`; `handle_receive_chunk_event` applies it later in Update. A forget-chunk packet is applied at once. If a chunk and its forget arrive in one frame, the forget runs first and the chunk loads after it | `packet/game/mod.rs` 636-645 and 1469-1481; `chunks.rs` 27-110 | `handleLevelChunkWithLight`, `handleForgetLevelChunk` run in packet order | Breach (verified order; the end state is inferred) | Yes, low. The bot keeps a chunk the server dropped, so physics and picks read stale blocks. Needs two packets in one frame |
| Block updates are queued and applied in Update, after the chunk events, or earlier by the ack handler (`apply_loaded`). An update for an unloaded chunk stays queued in `apply_loaded` but is dropped by `handle_block_update_event` (it calls `set_block_state` and the call finds no chunk). Vanilla drops such an update at once | `block_update.rs` 14-80; `packet/game/mod.rs` 1209-1233, 1318-1326 | `ClientLevel#setServerVerifiedBlockState`, `handleBlockUpdate` | Breach (verified); the two paths differ | Yes, low. The server does not send updates before the chunk |
| Respawn to another dimension swaps the world at once, but chunk events and block updates queued earlier in the frame still run later, against the new world. I found no clear of `QueuedServerBlockUpdates` or of pending chunk events in the respawn handler | `packet/game/mod.rs` 1629-1750; `block_update.rs` 66-79 | `handleRespawn` builds a new `ClientLevel` after the old packets ran | Breach (inferred from reading the handler; I did not trace the event queue) | Yes, low. Old-dimension blocks could land in the new world. Needs a respawn packet in the same frame as chunk or block packets. Bursts make that likely |
| Other readers of `ReceiveGamePacketEvent` (`azalea/src/container.rs` 338, `events.rs` 234) run after the state changed | `azalea` crate | | Probably harmless (checked): `events.rs` forwards a clone of the packet, `container.rs` removes `WaitingForInventoryOpen`. Neither decides a packet | Probably no: both sit above the contract (L6) |
| Entity packets use `commands`, so their writes apply when the command buffer flushes, not at once. Where this reorders against an immediate handler (for example a teleport) was not traced | `packet/game/mod.rs` entity handlers (855-1140) | | Unknown | Low |

### L3: no wall clock in what a bot sends

| Finding | Where | Vanilla | Verdict | Matters |
|---|---|---|---|---|
| Chunk batch rate uses `Instant::now` at batch start and finish, and sends `desired_chunks_per_tick` | `chunks.rs` 120-165, 213 | `ChunkBatchSizeCalculator` | Allowed (verified); the same wall-clock input as vanilla | Yes, by design. The value depends on timing |
| Chat timestamp (`SystemTime::now`), salt, and the signing key refresh after 60 minutes (`Instant`) | `client_chat/handler.rs` 45; `chat_signing.rs` 79, 106 | `ChatComponent`, `LocalPlayer` chat signing | Allowed (verified the clock use; I did not compare the refresh rule with vanilla line by line) | Yes, by design |
| Block-action settle, use-block grace and no-screen timeouts count ticks, not time | `block_actions.rs` 1331-1458; `entity_actions.rs` 777 | | Holds (verified) | No |
| Update-schedule systems that send or choose a packet: `perform_respawn` and `death_event_on_0_health` (respawn request), `handle_chunk_batch_finished_event` (reply to a packet), `handle_sprint`, `handle_walk`, `handle_attack_event`, `handle_start_use_item_event`, `handle_start_mining_block_event` and `handle_stop_mining_block_event` (the last five only insert queued components or events that a GameTick system turns into packets; I checked the attack and use-item ones, the other three I read only up to the schedule line) | `respawn.rs` 345, 369; `attack.rs` 23; `interact/mod.rs` 55, 136; `mining.rs` 52; `movement.rs` 49 | Respawn click: `DeathScreen` button, handled in a frame | Holds for the attack and use-item paths (verified); respawn request is sent in the frame, not the tick (verified, matches a button click); rest unknown | The respawn request: a one-frame shift only |
| Brand, client information, login and join systems run per frame in Update | `brand.rs` 16; `client_information.rs` 16; `join.rs` 36 | `ClientPacketListener` config phase | Allowed (verified the schedule; they send once) | No |
| Everything else that sends (movement, input, mining, use, container, inventory sync, block actions) runs in `GameTick` | the files listed in the grep of `add_systems` | `Minecraft#tick` | Holds (verified by schedule only; I did not check each body) | No |
| Null host systems run in `GameTick`, chained before physics and the action systems | `mc-adapter/src/host.rs` 195-230 | | Holds (verified) | No |
| Null reconnect backoff uses `Instant::now` and a seeded random; it only delays a new login | `mc-adapter/src/lifecycle.rs` 180-215 | | Allowed, above the contract (L6) (verified the clock use) | No |
| Pathfinder time limit (`PathfinderTimeout::Time`, default 1 s) changes which path it finds. The pathfinder does not send packets; it sets inputs (inferred: grep of `azalea/src/pathfinder` found no packet send) | `azalea/src/pathfinder/astar/mod.rs` 57, 151-166, 313 | | Allowed, above the contract (L6) | The path may differ under load; every packet is still a legal input |
| Keep-alive and pong answer in the packet handler, not on a tick | `packet/game/mod.rs` 1117; `pong.rs` 26 | `handleKeepAlive`, `handlePing` | Holds (verified) | No |

### L4: no bot reads another bot's state

"Shared" means `WorldHolder.shared`, the `World` that all bots in a dimension use. A bot's own view is `BotWorld` (patch 53 overlay of its pending cells over shared truth).

| Finding | Where | Vanilla | Verdict | Matters |
|---|---|---|---|---|
| The first bot to get a block update writes it into shared chunks; the others skip nothing and write the same value later. Between the two writes the second bot reads data it has not received. This is the second leak (OPEN §2) | `block_update.rs` 58, 76; `packet/game/mod.rs` 1318-1326 | `ClientLevel#setServerVerifiedBlockState` on the bot's own level | Breach (verified) | Yes. Every read of shared truth below can see a block ahead of the bot's own packets |
| The first bot to get a chunk packet parses it into shared storage. A later bot with no chunk of its own links the shared chunk and skips parsing. It then holds data from another bot's packet, possibly newer than its own packet | `chunks.rs` 83-110 | `handleLevelChunkWithLight` | Breach (verified) | Yes, same as above, for whole chunks |
| `forget_level_chunk` clears only the bot's partial storage, and `Weak` links keep the rest. Not a leak | `packet/game/mod.rs` 1469 | | Holds (verified the code; the link type is inferred from the `limited_set` use) | No |
| Shared mobs: one bot (`UpdatedBy`) applies relative moves to the shared entity; the others skip. The entity then carries the updater's packets, not each bot's. Peer bots use per-observer entries (patch 31, `for_peer`) | `relative_updates.rs` 90-145 | `ClientPacketListener#handleMoveEntity` on the bot's own level | Breach for mobs and remote players (verified) | Yes. `pushes.rs` 226-250 reads these positions for push velocity; the crosshair pick reads entities through `get_entities`/`AabbQuery` (`pick.rs` 225-233). Both can change a move or an attack packet. Peers are fine |
| `UpdatedBy` is inserted with `commands`, so in one frame two bots may both see `None` and both apply the same relative move | `relative_updates.rs` 120-135 | | Unknown (inferred; depends on when the command buffer flushes) | Yes if true: a mob moves twice as far |
| Physics reads the shared chunk map to decide an unloaded-chunk fall (patch 38) | `azalea-physics/src/travel.rs` 145, 147 | `LivingEntity#travel` (hasChunkAt) | Breach (verified) | Yes. A bot can fall slowly or fast because another bot's chunk is present. It changes movement packets |
| Collision reads chunks through the same `World` | `azalea-physics/src/collision/world_collisions.rs` 37-220 | `Entity#collide` | Breach through the shared `World` (verified the read; the bot has no own copy) | Yes, same cause |
| `update_in_loaded_chunk_of_clients` reads shared chunks for `InLoadedChunk` | `azalea-entity/src/plugin/mod.rs` 294 | `LocalPlayer#tick` (hasChunkAt) | Breach (verified); `loading.rs` 65 reads the partial world instead, which is correct | Yes. It gates movement and the "loaded" state |
| Block verdict in settle reads shared truth, on purpose, so a later prediction of this bot is not read. The truth may come from another bot's packet | `block_actions.rs` 1428 | | Breach (verified); closes with the second-leak fix | Yes. It decides what a bot reports (L5), not a packet |
| Block-action aim, break, place and restriction checks read `BotWorld::new(&shared, &prediction)` | `block_actions.rs` 618, 804, 972, 1072, 1185, 1203 | | Breach through shared truth under the overlay (verified) | Yes, same cause |
| Mining, movement and unmodelled-contact reads use `BotWorld` over shared | `mining.rs` 275, 341, 534, 551, 572, 663, 755; `movement.rs` 377, 644; `entity_actions.rs` 583, 703; `unmodelled_disconnect.rs` 105; `entity_actions.rs` 527 | | Breach through shared truth under the overlay (verified the reads) | Yes, same cause |
| Fluid-on-eyes and climbable use `BotWorld` over shared | `azalea-entity/src/plugin/mod.rs` 129, 164 | | Breach through shared truth under the overlay (verified) | Yes, same cause |
| `check_is_interaction_restricted` | `interact/mod.rs` 416-430 | `MultiPlayerGameMode#isInteractionRestricted`-style adventure check | Holds in itself: it only reads the `BotWorld` it receives; its callers fall under the rows above (verified) | Same cause |
| `mine_with_auto_tool` and `open_container_at` read `world().read()` | `azalea/src/auto_tool.rs` 42; `azalea/src/container.rs` 80 | | Breach through shared truth, but above the contract (L6): they pick inputs only (inferred) | Low |
| Pathfinder reads shared chunks to plan | `azalea/src/pathfinder/world.rs` 191, 279; `execute/simulation.rs` 258 | | Breach through shared truth, above the contract (L6) | Low: the plan may differ; every packet is still a legal input |
| Peer view: pushes and picks use the per-observer `peers` entries | `pushes.rs` 214-224; `interact/pick.rs` 74 | | Holds (verified for pushes; pick inferred from the `peers` argument) | No |
| One bot's add-player packet writes the shared entity index, and removes the twin from it. `indexing.rs` moves entries between chunks on the shared `World`, driven by any bot's position updates. Added by the check | `packet/game/mod.rs` 368-375, 400-403; `azalea-entity/src/plugin/indexing.rs` 212-222; read by `pushes.rs` 225-233, `pick.rs` 225-233, `entity_actions.rs` 443-448 | `ClientPacketListener#handleAddEntity` on the bot's own level | Breach (checked by a second reader) | Yes. It changes which entities a bot pushes against or hits. Same family as shared mobs |
| Add-player handling reads the shared `entity_by_id` to choose between a new entity and a `LoadedBy` update. Added by the check | `packet/game/mod.rs` 682-683 | | Breach (checked by a second reader) | Yes, through the row above |
| The crosshair block read goes through `BotWorld` over shared, and feeds attack and use. Added by the check | `interact/pick.rs` 167-169 | `GameRenderer#pick` | Breach through shared truth under the overlay | Yes, same cause as the second leak |
| Protocol-level reads of `registries` through `shared.read()` | `local_player.rs` 153; `interact/mod.rs` 193; `item_use.rs` 149; `equipment_effects.rs` 164 | | Allowed: registries are the same for every bot of one server (inferred) | No |
| Config phase writes shared registries (`shared.write()`) | `packet/config/mod.rs` 74 | | Allowed on the same ground (inferred) | No |

**Summary of breaches, by weight.** (1) The shared block and chunk writes (the second leak) reach about ten read sites that decide packets. Fixing the write fixes all of them. (2) Shared mob updates (`UpdatedBy`) feed pushes and picks. (3) Chunk and forget order in one frame, and a respawn with queued chunk work. (4) Pathfinder and helper reads of shared truth: not in the contract, listed for completeness.

**Not done.** I did not read the body of each GameTick sender. I did not trace when the `commands` buffer flushes relative to `process_packet`. I did not read `container.rs` 338 or `events.rs` 234. I did not compare chat signing refresh with vanilla line by line, and `chat_signing.rs` 120 (`Utc::now` against the certificate expiry) belongs on the allowed list with it. The L1 clock itself (`client.rs` 186-202, `tick_clock.rs` 73-76) is the named exception in L3.

**Check of this audit.** A second reader (Haiku) checked each claim against 40b6af0. Most citations held, some with small line drift, now corrected. Three were wrong and are corrected above: the ack line, `interpolation.rs` 444 (an entity index read, not a block read) and `packet/game/mod.rs` 321 (a read, not a write). Not traced by either reader: the `commands` flush timing, so whether a mob can move twice in one frame is still open. The check was a code reading. Nothing was run.

## Swarm world design (Allen approved 2026-11-04)

**Problem:** [SWARM_WORLD_PROBLEM](SWARM_WORLD_PROBLEM.md). **Record:** [AD-F5.6](../DECISIONS.md#ad-f56-how-do-the-bots-of-one-process-hold-the-world). Nothing is in the fork yet. The spike has items 1 to 4 and a first form of item 5 (see "Spike state and handoff").

**The idea is git's object model, without history.** A section is a blob, stored once by content. A chunk's 24 section pointers are a tree. A bot's world is a branch. One frame's edits to one section are a commit. A packet arriving is a fetch. A section is freed when no bot points to it. Bots never merge, because the server is the only authority.

1. **Own worlds.** Each bot has one `World`, as in the spike (`WorldName::for_bot`, and the uuid index change in `indexing.rs`). The second leak, the shared mob updates and the shared entity index go away by design, and read sites need no change. The patch 53 overlay stays for predictions in the own-worlds patch itself; fork patch 76 removes the overlay and writes predictions into the bot's own `World`, as vanilla does (Allen, 2026-11-10; [CLIENT_LEVEL_MODEL](../proposals/CLIENT_LEVEL_MODEL.md)). Each world also holds its own uuid index, `World::entity_by_uuid` (fork patch 72); `EntityUuidIndex` keeps only the bots' join entities. Other bots become ordinary entities, as on a vanilla client, so the patch 31 and 44 swarm code stops running.
2. **Section store.** One store per process holds each different section once. It looks up a 64-bit hash, compares the full content on a hit, and holds weak references. New file in `azalea-world`.
3. **Chunks.** `Chunk.sections` becomes `Box<[Arc<Section>]>`. A shared section cannot be changed in place, so the compiler rejects any code that tries.
4. **Chunk packets.** Each bot parses its own packet, then stores each section. A bot never skips its own packet. This closes the leak: a bot links a section only when its own packet held the same content.
5. **Block updates.** Edits are grouped per section per frame. The bot copies the section, applies the edits and stores the result, so bots that reach the same content share it again. The live writes are `block_update.rs` 58 and 76.
6. **Readers.** `world_collisions.rs` 78 and 218, `pathfinder/world.rs` 283, `find_blocks.rs` 217 and `heightmap.rs` 95 change. No Null crate reads sections.
7. **Not shared in this version.** Heightmaps (about 0.19 MB per bot), pointer lists (about 0.04 MB per bot) and entities.
8. **One process per machine, for a batch of bots.** The store only shares inside a process, so one process per bot loses the saving. All bots of a process share one tick loop, so a machine can run several batches if that loop becomes the limit.
9. **The full comparison stays** until it really is the limiting factor (Allen, 2026-11-04). It costs about 0.1 µs per section, and a wrong match would be a silent wrong read. A long hash alone would also work in practice.

**Copies per edited section.** About two while an update is arriving (old and new), then one. The count is about the edits in flight plus one, not the bot count. Vanilla keeps one order: `ChunkHolder#broadcast` sends each packet to all players in a loop, and each connection keeps its order. The acting bot also gets direct updates (`ServerPlayerGameMode`, `handleUseItemOn`), but these carry the block's current state, so it can only be ahead on the same line. Paper's order is not checked. Without sharing again, a section would reach N copies, about 139 KB at 75 bots.

**Where sharing can fall short.** These cost memory only, never a wrong read:
- The comparison includes the palette layout, so the same blocks encoded differently do not match. A resent chunk can stay apart from a locally edited copy until it is replaced. This may also explain the 3% of repeated chunks with different bytes (OPEN §1, a guess).
- Plugins, and possibly anti-x-ray, can send players different content.

Biome ids depend on the server's registry. Two servers in one process could share a section with equal ids, and each bot reads it through its own registry, so this cannot give a wrong read.

**Result so far.** The spike reads about 1.2 MB per bot at the end of walking, and at most about 1.8 MB with every page touched, when bots overlap (Mac, one run; see "Spike state and handoff"). Without the store it is 6.2 to 6.6 MB, and shared mode is about 1 MB. Bots far apart overlap less, so the saving falls, but shared mode loses the same overlap. The Pi must confirm this, because Mac RSS can read low.

**Build order.** Each step has one done-when.

1. Contract test for L4 (step 3 of "Order"). Done when it fails on today's code and its negative control is in place.
2. Rerun `own_near_75` on the Pi with the keep-alive log and GC log, once no VD3 leg runs there. Done when it shows which side holds the reply, or that the kicks do not recur.
3. Fork patch: section store, in today's shared mode. Behaviour must not change. Tests: equal content gives one pointer, and one bot's write never changes what another bot reads. Done when the selective runs pass and the tests pass.
4. Fork patch: own worlds, with the spike's switch removed. Done when the contract test passes and a full pair passes. SW1 also draws zero alerts. Memory and CPU are recorded from the Pi.
5. Fork patch: the two per-entity systems run only where a reader exists (see "Where the added CPU goes"). Done when CPU is measured again.

Step 5 may come before step 4, because it changes nothing while entities are few. The pair cadence counts these with the patches VD3 has queued (OPEN §1).

**What would reopen this.** The bot's side holding the keep-alive reply needs a fix first in any design. If entity copies cost far more than expected as bot counts grow, per-observer entries (as in patch 31) come back, with a shared `World` and the same section store.

### Spike state and handoff (2026-11-04)

All of this is uncommitted in worktree `~/Desktop/null-spikes/azalea-own-world` (branch `own-world-spike`), and the fork's `null-main` is unchanged. Build with `CARGO_TARGET_DIR=~/Desktop/null-spikes/own-world-bench/target`. Own mode is the switch `NULL_OWN_WORLDS=1`.

| Item | State | Evidence |
|---|---|---|
| L4 contract test, `azalea-client/tests/simulation/l4_contract.rs` | Written. Blocks and chunks only; A's packets are compared with B's inbound packets changed. Mobs and the entity index have no test yet | Shared mode: both fail, A's moves differ (A falls on B's update). Own mode: both pass. The failing shared run is the negative control |
| Section store, `azalea-world/src/chunk/section_store.rs`; `Chunk.sections` is `Box<[Arc<Section>]>` | Works in the spike | `cargo test -p azalea-client --test main`, shared mode: 105 of 107, and the 2 failures are the L4 tests. `azalea-world` tests pass |
| Edits grouped per frame (design item 5) | Not built. Each edit stores a section | |
| Store unit tests (equal content gives one pointer, a write never changes another holder) | Not written | |
| Own mode against the simulation suite | 100 of 107, the same 7 with and without the store | See below |
| Pi rerun of `own_near_75` with keep-alive and GC logs | Not run: needs a Pi build with `cargo zigbuild`, and the Pi free of VD3 | |

**The 7 tests that fail in own mode.** `login_to_dimension_with_same_name` and `change_dimension_to_nether_and_back` assert the `WorldName`, which the spike suffixes with the bot's entity. The real patch must choose: change these tests, or key `Worlds` another way. `test_bot_b_does_not_read_bot_a_unconfirmed_break` and `test_bot_b_stays_on_the_block_when_a_predicts_nothing` stop at "B stands on the block": setup gives that block only to A, so they look like tests that assume a shared world (inferred). `test_a_refused_break_is_never_seen_by_b_and_a_gets_the_block_back` fails at "B reads the server's block", probably for the same reason. `bot_login_twin` and `test_a_dimension_change_clears_the_pending_cells...` were not read. Each needs reading before step 4: a test that assumes sharing is rewritten, and any other failure is a bug.

**Mac run with the store (`own_near_75`, once, `local/results_store`).** No kicks.

| Measure | Without store | With store |
|---|---|---|
| RSS at the end of walking | 312 MB | 106 MB |
| RSS at the census, every section read | 486 MB | 152 MB, which includes the census's own section copies |
| Different sections alive | 4,427 by content | 4,601 in the store |
| Client CPU, idle, per 60 s | 16.4 s | 14.9 s |
| Client CPU, walking, per 60 s | 16.3 s | 15.5 s |

So the store did not raise CPU in this one run, but one run cannot show that it lowers it. Chunk handling rose from about 1.55 s to 1.76 s in total.

**Next, in order.**
1. Read the 7 failures above and sort them.
2. Group edits per frame and write the store unit tests.
3. Add L4 tests for mobs and the entity index.
4. Limit the two per-entity systems.
5. Run `own_near_75` on the Pi once VD3 has finished there (kick cause, and the memory check).
6. Port to the fork clone as two patches, with selective runs, a ledger row and a diff for each, under the pair cadence.

### Options considered

The block storage comes out the same in the two designs left at the end: own worlds with shared sections, and a shared world with own section updates. The choice was about entities and read sites.

| Option | Verdict |
|---|---|
| A: per-bot list of changed blocks over a shared base | Rejected. The base goes stale, every bot stores the same changes, and collision loses its fast path (Allen, 2026-11-03) |
| B: copy-on-write sections | Kept as the block storage, with sharing by content added |
| C: shared log with a cursor per bot | Rejected. Hardest to build, and rests on all bots receiving the same packets |
| D: shared latest truth, with a per-bot list of cells not yet received | Dropped, not checked against the code. Whole chunk packets were the hard case |
| Shared world with own section updates, and per-observer entries for entities | Rejected for now. It costs less per entity, but mobs and the entity index stay as separate fixes, and every direct `shared` read must move. A missed read is a silent leak |
| One process per bot | Rejected. It loses all sharing |

Spike: worktree `~/Desktop/null-spikes/azalea-own-world`, branch `own-world-spike`, uncommitted, switch `NULL_OWN_WORLDS=1`. Bench: `~/Desktop/null-spikes/own-world-bench`, with `STATE.md` and the local runs in `local/results`.

## Own-world sweep results (2026-11-04)

Two sweeps ran on the Pi, one run per case, view distance 6: 12 cases overnight (`results/`, launched 2026-11-03 22:47) and 4 cases at 75 bots (`results75/`), both in `/mnt/pi-data/own-world`. They measure cost only. The Pi hosted server and bots, so tick times only compare cases. Own mode gave one world per bot: it skipped 0 chunks in every case, and no client crashed.

| Bots | Mode | Server kicks (`Timed out`) | Chunks per bot, total | Process memory, idle | Client CPU, idle, per 60 s | Tick median and p99 | Valid |
|---|---|---|---|---|---|---|---|
| 50, near | shared | 0 | about 270 | 68 MB | 31 s | 14, 41 ms | Yes |
| 50, near | own | 0 | about 260 | 340 MB | 43 s | 23, 53 ms | Yes |
| 75, near | shared | 0 | about 242 | 90 MB | 45 s | 26, 64 ms | Yes, but server about 7 TPS |
| 75, near | own | 149 | about 70 | 200 MB (too low) | 51 s | 36, 88 ms | No |
| 75, spread | shared | 0 | about 250 | 91 MB | 43 s | 26, 63 ms | Yes, but server about 7 TPS |
| 75, spread | own | 161 | about 108 | 225 MB (too low) | 49 s | 38, 94 ms | No |
| 100, all four | both | 47 to 231 | 28 to 75 | | | | No |

**What this shows.** At 50 bots an own world costs about 5.4 MB more per bot, and 25 to 40% more client CPU. The kicks start earlier in own mode: at 75 bots, shared had none and own had 149 and 161. The 75-bot own cases never finished loading chunks, so they give no memory or CPU figure. Those figures come from the Mac runs below.

**The kicks: cause unknown.** Nearly all 75 bots go within about 3 s, at the end of a period when the server fell far behind. Server lag alone does not explain them: `shared_spread_75` fell 77 s behind with zero kicks. The bots' `GameTick` kept about 20 per second with a longest tick of 153 ms, so the client's tick loop did not stall. Its socket reader and writer tasks were not measured. A kick means a keep-alive reply was still unhandled 15 s after it was sent (`ServerCommonPacketListenerImpl#keepConnectionAlive`). The reply is handled without a move to the main thread (`handleKeepAlive`). No log records when the bot received or answered the keep-alive, and the runs have no GC log.

**Mac run (2026-11-04, `own_near_75`, once, `~/Desktop/null-spikes/own-world-bench/local/results`).** Same world, heap and case, with keep-alive times and a GC log. No kicks: 75 of 75 bots stayed, the server held 20 TPS at about 9 ms per tick, and all 750 keep-alives reached the bot's handler within 28 ms of the earliest. The longest GC pause was 47 ms. So own worlds alone do not cause the kicks at 75 bots; they need the Pi's overload. The run did not reproduce them, so it does not say which side holds the reply.

**Where the memory goes (census at the end of the Mac 75-bot run).** Chunk data is 5.1 MB per bot of about 6.2 MB. The rest is the bot itself and its entity copies (3,710 ECS entities at 75 bots).

| Part | All 75 bots | Share |
|---|---|---|
| Block state palettes and storage | 286 MB | 74% |
| `Section` structs (216 bytes each, 383,592 sections) | 83 MB | 22% |
| Heightmaps | 14 MB | 4% |
| Biomes | 1 MB | under 1% |

Only 4,427 of the 383,592 sections are different, and they hold 10.5 MB. 54% of sections have no heap data, but each still costs its 216-byte struct. All bots started in one chunk, so this is the best case for sharing. Mac RSS is not used here: it read 230 MB idle against 384 MB of chunk data, probably because macOS compresses idle pages.

**CPU, own against shared (Mac, 50 bots, near, one run each, other apps running, no kicks).**

| Measure | Shared | Own | Added |
|---|---|---|---|
| Client CPU, idle, per 60 s | 10.7 s | 13.5 s | +26% |
| Client CPU, walking, per 60 s | 10.4 s | 12.9 s | +24% |
| Tick median, idle | 3.0 ms | 4.1 ms | +1.1 ms |
| Tick median, walking | 2.2 ms | 2.9 ms | +0.7 ms |
| ECS entities | 105 | 2,046 | about 20 times |
| Chunks parsed | 602 | 19,617 | about 33 times |

This matches the low end of the Pi's 25 to 40%. The own-mode 75-bot run used 16.4 s idle and 16.3 s walking, with entities up 1.8 times for 1.5 times the bots. No shared 75-bot run exists on the Mac, so how the cost grows is not settled.

**Where the added CPU goes (stack samples, 15 s per window).** `update_in_loaded_chunk` and `update_fluid_on_eyes` running on every copied entity are about 40% of the added samples idle and 38% walking. Parsing chunks is about 10% walking and small idle. Most of the rest, including local physics, is not explained.

Those two systems may not need to run on copies. A grep of the fork finds readers of `InLoadedChunk` and `FluidOnEyes` only on a bot's own entity: `loading.rs` (`With<LocalEntity>`), `events.rs` (`LocalPlayerEvents`), `mining.rs` and `auto_tool.rs`. `update_in_loaded_chunk` runs only on entities that are not clients. Vanilla does run `Entity#updateFluidOnEyes` on remote entities (`baseTick`), but no packet depends on the result, so F5 "What Null computes" allows skipping it. Inferred saving: down to about +15%. Not built, not measured.

**Cost of the section store (real sections, timed in the Mac runs; lock and lookup not included).** Hash about 0.29 µs, full comparison about 0.10 µs, copy about 0.23 µs per section.

| Case | Added cost |
|---|---|
| Walking (4,170 chunks in 60 s) | about 40 ms per 60 s, 0.3% of client CPU |
| Joining (10,650 chunks) | about 100 ms, 2% of that window |
| Heavy edits (an example: 1,000 section edits per second seen by all 50 bots) | about 3% of one core |

The store may also save CPU, because `world_collisions.rs` 78 clones whole sections and would clone only a pointer. Not measured.

## Limits

- VD3 T1 (2026-11-03): a 3 s `SIGSTOP` of the bot on Grim drew no alert, so a stop of that size does not provoke one. Client pause rows need a different length or disturbance.
- `SIGSTOP` freezes all threads. Whether the client sees a main-thread stall and a whole-process pause the same way is unverified.
- The proxy cannot choose the exact client tick a packet lands in. Only the fork tests can.
- A vanilla client takes about 620 MB, so a real crowd needs the Mac or the M5 Airs ([BENCH_PLAN](BENCH_PLAN.md)). One or two clients come first.
- The contract covers what a bot sends. It does not make a swarm fast enough. A slow bot stalls all bots in the process, and that is a capacity question for the bench.

## Order, once accepted

Each step has its own done-when. The steps do not touch the one-at-a-time gate.

1. Measure: the GC log and the A23 gap. Done when one quiet pair and one parallel pair show what the pauses are.
2. Write the accepted contract into F5 §4.6, and audit L2 to L4. Done when every breach is listed.
3. Contract tests and timeline tests in the fork, then a first trace replay, each with a negative control. This plan owns the contract and timeline tests. VD3 owns the replay runner, the Null and vanilla alert comparison and the recording format ([VD3_BRIEF](VD3_BRIEF.md), "Who owns what"), and this plan reuses them. The first trace replay waits for `harness/vanilla-diff/README.md`, section "VD3 formats". A fixture that needs a new field is agreed with VD3 first. Agreed so far (null-5d, 2026-11-03): this plan adds `vd-tap --raw-out FILE`, one line per clientbound `Config` or `Game` packet with its raw bytes. It starts after VD3 T1 is verified, and rebuilds `vd-tap` only when no VD leg runs. The same change adds the field to "VD3 formats" and a test that a line decodes back to the same packet, and changes no existing field or the differ. VD3 runs do not pass the flag, so fixtures choose their own rows. Done when they pass and each control fails, and the time for the whole tier is measured.
4. Fix the breaches, starting with the second leak ([Swarm world design](#swarm-world-design-allen-approved-2026-11-04), which has its own build order).
5. Mimicked load with a vanilla client, using the same client and tap as VD3 and sharing machines with it by agreement. Done when every alert on the short list is shared with vanilla or fixed.

6. The proof above, then Allen's decision on the tiers.

The faster suite (`SUITE_SPEED.md`) no longer leads the speed work. If the old suite stays occasional, its run time matters less. Its barriers may still be worth keeping, after their own proof one server at a time. How parallel failures count depends on the third decision above.
