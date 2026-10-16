# Azalea fork patches

**Record, started 2026-10-16.** These are the changes Null made to the Azalea fork at `~/Desktop/null-spikes/azalea`, on top of the 1.21.11 branch and its 38 cherry-picks. They are uncommitted in the fork for now.

**Plan (Allen, 2026-10-16):** keep each patch small and separate. When Null is finished, send them upstream as pull requests to `azalea-rs/azalea`, following the project's contributing guidelines and code of conduct. Read those before writing a PR.

All three swarm patches were found in the 2026-10-16 swarm duel: two `null-node` processes of 8 bots each, on `skeleton-server`.

| # | file | problem | fix |
|---|---|---|---|
| 1 | `azalea-client/src/plugins/packet/relative_updates.rs` | Bots in one process share a world. Each bot kept its own count of updates per entity, and only a bot whose count matched the shared count could apply the next one. The counts were never synced when a bot loaded an entity. So once the bot that was ahead died, respawned or moved out of range, no bot could update that entity. It froze in place for the whole swarm. Seen as bots chasing enemies' old positions after a teleport, and strafing around "dead bodies". | An `UpdatedBy(Entity)` component names the one bot that applies an entity's updates. When that bot no longer has the entity in its `LoadedBy`, the next bot that gets an update takes over. |
| 1b | `azalea-client/src/plugins/packet/game/mod.rs` (`add_entity`) | When an entity that already exists is re-added (respawn, back in range, teleport), only `LoadedBy` was updated, so its stored position stayed stale. | If no bot that still tracks the entity is driving its updates, take the absolute position from the add packet and make this bot the updater. Never for local (swarm) entities: an early version moved our own bots and piled them up. |
| 2 | `azalea-entity/src/plugin/mod.rs` (`add_dead`) | `Dead` was added at health 0 and never removed. A respawned player keeps its entity id and is re-added onto the same entity, so it stayed `Dead` forever and bots ignored it as a target. | Remove `Dead` from a non-local entity when its health is above 0 again. |
| 3 | `azalea-client/src/plugins/movement.rs`, `lib.rs` | Using an item was a TODO (`using_item = false`). A bot holding up a shield walked and sprinted at full speed, which anticheats flag at once. | A public `ItemInUse` marker, set by whoever sends the use packet. While it is set, input is scaled by 0.2 (vanilla `modifyInput`), sprinting can't start, and an active sprint stops. `mc-adapter` sets it on `UseItem` and clears it on release, offhand swap, slot change and death. |
| 4 | `azalea/src/pathfinder/mod.rs` (`handle_stop_pathfinding_event`) | A stop for a bot with no `ExecutingPath` (its search still running) dropped the search but returned before clearing `goal`, and `is_calculating` stays set until a path arrives. So the bot read "calculating" with nothing running until its next goto. Found 2026-10-16 in the 20 v 20 swarm duel: up to 217 such stops in one match, 5.5 s freezes. | With no `ExecutingPath`, clear `goal`, `opts` and `is_calculating` too. After the fix, three matches had no wait of 2 s or more (before: 7, 0 and 2). |

Before upstreaming:
- Check whether upstream has fixed any of these since.
- Add tests in Azalea's style for 1 and 2 (two simulated clients sharing an entity, the updater leaves; a player dies and respawns).
- For 3, upstream will probably want Azalea to track item use itself instead of a marker.
