# skeleton-server

Local vanilla Minecraft Java 1.21.11 server for the Null skeleton (milestones S4 to S6). Port 25567, RCON 25577 (password `skeleton`), offline mode, normal world, survival, peaceful, view and simulation distance 6, 2G-3G heap. Copied from `combat-server` (same `server.jar`, libraries and scripts); `combat-server` and `azalea-bench` are untouched.

## Run
- Start: `scripts/start_server.sh` (log: `server/server.log`, pid: `server/server.pid`). Wait for `RCON running` in the log.
- RCON: `python3 scripts/rcon.py 'list' 'data get entity null_b1 Pos'` (defaults to port 25577 and the password above).
- Stop: `scripts/stop_server.sh` (RCON `stop`, waits for the PID). Run it before starting a second copy: both copies would share the world, and a second `start_server.sh` overwrites `server.pid`.
- 20-bot patrol check: `scripts/run_patrol.sh TAG [N]` runs `null-node` (release build) with `null_b1..null_bN`, samples server-side positions over RCON and greps the server log. Output goes to `runs/TAG.*`.

## World
- `level-seed=3`, `generate-structures=false` (no mineshaft logs underground), `level-type=minecraft:normal`.
- The natural spawn of seed 3 is in water, so the world spawn was moved: `setworldspawn 416 70 352`. The chunks around it were generated with `forceload` and the force-load list was then cleared (`forceload remove all`). The spawn is stored in `world/level.dat`.
- Around the spawn: grass plains with scattered oak and birch, ground y 66 to 81 within 35 blocks, no water within 30 blocks of spawn. Grass at (416, 352) is at y 69, so standing feet are at y 70.
- If the world is deleted, the same seed regenerates the same terrain and trees (for this jar). Redo `setworldspawn 416 70 352` and the gamerules below.

## Oak trees near spawn (for S5)
Base is the lowest log block of the tree (x, y, z); distance is horizontal from the spawn. Bases sit on grass, so the standing block next to a trunk is at the base y.

| base (x, y, z) | logs | distance |
|---|---|---|
| (401, 69, 349) | 5 | 15 |
| (401, 69, 356) | 5 | 16 |
| (410, 69, 335) | 5 | 18 |
| (400, 69, 343) | 6 | 18 |
| (407, 69, 336) | 6 | 18 |
| (397, 68, 346) | 6 | 20 |
| (394, 68, 347) | 4 | 23 |
| (398, 71, 366) | 5 | 23 |

There are about 38 oak trees with 4 or more logs within 40 blocks of the spawn. Birch trees are mixed in; they use `birch_log`, not `oak_log`.

## Gamerules (1.21.11 snake_case names; persisted in the world)
```
gamerule spawn_mobs false
gamerule spawn_monsters false
gamerule advance_time false
time set noon
gamerule advance_weather false
weather clear
gamerule spawn_phantoms false
gamerule spawn_patrols false
gamerule spawn_wandering_traders false
gamerule spawn_wardens false
gamerule raids false
gamerule show_advancement_messages false
gamerule immediate_respawn true
```

## Notes
- `server.properties` differences from combat-server: normal world, seed, ports and password, peaceful, `generate-structures=false`, `pause-when-empty-seconds=-1` (the server never pauses when empty, so tests do not start slow), no `pvp` flag change.
- `runs/` holds logs of the S4 acceptance runs (state JSON lines, sampled positions, server log slices).
