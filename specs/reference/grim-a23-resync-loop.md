# Grim's unloaded-chunk resync loop (A23, 2026-10-23)

Proof for the A23 allowance in F5 §4.6: after a server teleport into an unsent chunk mid-air, Grim can draw `Simulation` and `GroundSpoof` alerts that a vanilla client draws too. Rule (Allen, 2026-10-23): identical to vanilla means identical Grim alerts.

## Runs

A23 alone on the Mac Paper + Grim server, release binary at fork 4ee1f8a: 3 failures in 8 runs (`p38diag_grim.log`, `p38diag_grim_1.log`, `p38diag_grim_2.log`, and five scratch runs, two of them kept). Traces, with temporary `TEMPDIAG` lines in the fork (removed after) and `grim consoledebug` on the server:

- `p38diag_grim_trace_fail.log`: FAIL, `Simulation x8`.
- `p38diag_grim_trace_pass.log`: PASS, no alerts.

`TEMPDIAG send` is one position packet from `send_position` (y and speed after the tick). `TEMPDIAG tp applied` is a server teleport after it is applied. `TEMPDIAG knockback Set` is a set-motion packet for the bot.

## The loop, in the fail trace (times 15:57:47)

```
.234 player position packet y 240.0, delta 0      Grim resync (chunk not yet acknowledged)
.234 tp applied y 240.000000 vel_y 0
     (tick: moves 0, ends at y 240 = last sent y: nothing sent)
.334 send y 239.921600 vel_y -0.155232
.353 player position packet y 240.0, delta 0      Grim setback one tick back
.353 knockback Set y -0.0783739                   with that tick's speed
     (tick: moves -0.0784, ends at y 239.92163 = last sent y within 2e-4: nothing sent)
.436 send y 239.766420 vel_y -0.230502            two ticks of fall in one packet
.455 player position packet y 239.9216, delta 0   setback one tick back again
.455 knockback Set y -0.1552219
.523 send y 239.535861 vel_y -0.304307            two ticks again; repeats until the landing
```

Grim's per-tick lines for the same run (`P` predicted move, `A` actual move, `O` offset): `A` is two client steps (-0.0784 - 0.1552 = -0.2336, then -0.1552 - 0.2305 = -0.3857) against a one-tick `P`, and the alerts follow:

```
P: 0.0 -0.0986000015258789 0.0
A: 0.0 -0.23358037291637856 0.0
P: 0.0 -0.1752320045166016 0.0
A: 0.0 -0.38573931963023256 0.0
Grim » null_p38fix failed Simulation (x1) .284333
```

In the pass trace the last setback arrives before the client sent that tick's position, so the next tick ends away from the last sent position, a packet goes out every tick (about 50 ms apart), and Grim's `P` and `A` agree to 1e-14.

## Why a vanilla client does the same

- `ClientPacketListener#handleMovePlayer` → `setValuesFromPositionPacket`: the teleport sets position and speed (delta 0, not relative), then sends the accept and a `PosRot`. It does not touch `LocalPlayer`'s last sent position. The fork does the same (`player_position`, `RelativeMovements::apply`).
- `ClientPacketListener#handleSetEntityMotion`: sets the player's speed. The fork does the same (`handle_knockback`, `Set`).
- `LocalPlayer#sendPosition`: sends a position only when the move since the last sent position is over 2e-4 blocks (or every 20 ticks). The fork's `send_position` uses the same rule.
- Grim: `MovementCheckRunner#processAndCheckMovementPacket` resyncs each move while `SetbackTeleportUtil#insideUnloadedChunk`; `blockMovementsUntilResync` sends the last known good position and speed. A setback one tick back, applied after the client already sent that tick, makes the client repeat the tick and end on its last sent position, so vanilla sends nothing for that tick.

Whether the loop starts depends only on when the resync arrives relative to the client's tick, not on anything the client chooses. Later: replace this proof with a stubbed vanilla client on the same scenario, compared alert for alert (OPEN item 0).

## The floating kick (Pi pair p38b_full, 2026-10-23)

This is not an allowance. A23 now makes sure the case cannot happen, and the row fails with a clear reason if it does.

What happened, from the Pi Grim server log: the teleport to y 240 above the far floor came at 00:50:48. At 00:50:52 the server logged "null_pi was kicked for floating too long!" and "Flying is not enabled on this server". That is 4 s, about 80 ticks, before the bot fell at all. No Grim alerts. (The row's "client chunk seen at 17.1 s" is from after the kick.)

Why a vanilla client gets the same kick:

- `ServerGamePacketListenerImpl#handleMovePlayer`: an accepted move with no block below or around the box, no flight and no Levitation marks the player as floating. The teleport accept at y 240 is such a move.
- `ServerGamePacketListenerImpl#tickPlayer`: each tick while the flag is set, the count goes up by one. At over 80 ticks the server disconnects the player. A teleport does not reset the count.
- Grim (`SetbackTeleportUtil#insideUnloadedChunk`, `blockMovementsUntilResync`) holds the player at the arrival position until the client has the chunk, so the server never sees the fall that would clear the flag.

So any client the server cannot send a chunk to within 4 s gets this kick on Paper + Grim. On the Mac the far chunk arrives in 0.2 to 0.4 s. On the Pi it did not arrive within 4 s. The cause is terrain generation around the far chunk, which the server needs before it sends that chunk. Measured on the Pi Paper + Grim server, idle, by RCON `forceload add` and `execute if loaded` (probe log `logs/chunkprobe.log` on the Pi, 2026-10-23): one fresh chunk about a million blocks out loads in 19.1 s, 17.8 s and 15.0 s (three places); the same chunk again after it was unloaded loads in 0.42 s; A23's saved far chunk loads in 1.95 s (first load after the server start); a fresh square 217 blocks wide (A23's new preparation) loads in 79 s. So a fresh chunk takes 15 to 19 s on the Pi, which matches the 17.1 s in the row, and a saved one takes under 2 s.

Harness fix (`scenarios_unloaded.rs`): before the row, A23 forceloads a square 108 blocks around the far floor and waits until its corners are loaded, so the chunks are generated and saved. Then it builds the floor and removes the forceload. The arrival only reads saved chunks. The row fails with "chunk within 3 s: false" when the client gets the chunk after 3 s.
