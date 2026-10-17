# grim-server

Paper 1.21.11 (build 132) with GrimAC 2.3.74, for checking that bots send only what a vanilla client could (Allen, 2026-10-17). Same world as `skeleton-server` (a copy made 2026-10-17, seed 3, spawn 416 70 352), same settings except: port 25568, RCON 25578 (password `skeleton`), motd. Jars come from Allen's copy at `~/Documents/antigravity/focused-bardeen` (`paper-1.21.11-132.jar`, `plugins/grimac-bukkit-2.3.74-f5bbe9c.jar`). **This server is the strict check for the ability check, timing included** (Allen, 2026-10-17): Null targets vanilla servers, but the fork and adapter must also run cleanly on Paper with an anticheat (F5 §4.6). A scenario passes only with the right result on both this server (its timing window met, zero Grim alerts) and the vanilla `skeleton-server`.

Difficulty is **normal** (set 2026-10-17 by RCON `difficulty normal`; Paper keeps the world's stored difficulty, so `server.properties` alone doesn't change it). Mob spawning stays off by gamerule. A6 turns `natural_health_regeneration` off for its damage rows and restores it.

## Run
- Start: `scripts/start_server.sh` (log: `server/server.log`). Wait for `RCON running`.
- RCON: `python3 ../skeleton-server/scripts/rcon.py --port 25578 'list'`.
- Stop: `scripts/stop_server.sh`.
- Both servers can run at once (different ports and worlds).

## Grim settings
- `plugins/GrimAC/punishments.yml` (tracked copy: `config/GrimAC/punishments.yml`; copy it in after a fresh install): every `[alert]` is `1:1`, so each violation prints to the console (the defaults are kept in `punishments.default.yml`). No kick commands are set.
- An alert in the log: `Grim » <player> failed <Check> (xN)`, with ANSI colour codes.
- Bots must not be op: Grim exempts players with `grim.exempt`, which ops have.
