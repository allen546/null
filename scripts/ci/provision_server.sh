#!/usr/bin/env bash
# Runs on a GitHub Actions runner (Linux): builds the test server for one kind from nothing, because the
# server directories are not in the repo. Downloads the jars (checked against the hashes of the Mac's copies),
# writes the settings of harness/*/README.md, makes the seed-3 world on vanilla and, for grim, copies that
# world to Paper (as the Mac's grim world was made). Leaves the server stopped; run_server_suite.py starts it.
# Usage: scripts/ci/provision_server.sh vanilla|grim   (NULL_HARNESS = the harness directory)
# Spec: specs/plans/ABILITY_CHECK.md §How to run
set -euo pipefail
kind=$1
H=${NULL_HARNESS:?set NULL_HARNESS}
RCON="uv run --no-project python $H/skeleton-server/scripts/rcon.py"

VANILLA_URL=https://piston-data.mojang.com/v1/objects/64bb6d763bed0a9f1d632ec347938594144943ed/server.jar
VANILLA_SHA1=64bb6d763bed0a9f1d632ec347938594144943ed
PAPER_URL=https://fill-data.papermc.io/v1/objects/5ffef465eeeb5f2a3c23a24419d97c51afd7dbb4923ff42df9a3f58bba1ccfba/paper-1.21.11-132.jar
PAPER_SHA256=5ffef465eeeb5f2a3c23a24419d97c51afd7dbb4923ff42df9a3f58bba1ccfba
GRIM_URL=https://cdn.modrinth.com/data/LJNGWSvH/versions/nKI7MWZj/grimac-bukkit-2.3.74-f5bbe9c.jar
GRIM_SHA256=a6768ab769ac8801b7d03567528424b0a8e7310cecd31dc70d100b521ab959e3

fetch() { # url file algorithm hash
  curl -fsSL --retry 3 -A null-ci -o "$2" "$1"
  echo "$4  $2" | "${3}sum" -c - >/dev/null || { echo "hash mismatch: $2" >&2; exit 1; }
}

settings() { # dir port rcon-port motd
  mkdir -p "$1"
  echo eula=true > "$1/eula.txt"
  # Protection needs one op; no bot uses this name (harness/skeleton-server/README.md, Notes).
  echo '[{"uuid":"e8d63ec0-d237-3314-8bfe-17538f36724b","name":"allen546","level":4,"bypassesPlayerLimit":false}]' > "$1/ops.json"
  cat > "$1/server.properties" <<EOF
server-port=$2
query.port=$2
enable-rcon=true
rcon.port=$3
rcon.password=skeleton
motd=$4
online-mode=false
enforce-secure-profile=false
level-name=world
level-seed=3
level-type=minecraft\:normal
generate-structures=false
gamemode=survival
force-gamemode=true
difficulty=normal
pvp=true
max-players=200
view-distance=6
simulation-distance=6
spawn-protection=4
pause-when-empty-seconds=-1
max-tick-time=-1
sync-chunk-writes=false
EOF
}

start() { # dir jar ; waits for RCON
  ( cd "$1"; nohup java -Xms1G -Xmx2G -XX:+UseG1GC -jar "$2" nogui > server.log 2>&1 < /dev/null & echo $! > server.pid )
  for _ in $(seq 600); do
    grep -q "RCON running" "$1/server.log" 2>/dev/null && return 0
    kill -0 "$(cat "$1/server.pid")" 2>/dev/null || { tail -40 "$1/server.log" >&2; echo "server died" >&2; exit 1; }
    sleep 1
  done
  echo "no RCON in 600 s" >&2; exit 1
}

stop() { # dir rcon-port
  $RCON --port "$2" stop >/dev/null
  for _ in $(seq 180); do kill -0 "$(cat "$1/server.pid")" 2>/dev/null || { rm -f "$1/server.pid"; return 0; }; sleep 1; done
  echo "server did not stop" >&2; exit 1
}

# Vanilla first, for both kinds: it makes the world.
V=$H/skeleton-server/server
settings "$V" 25567 25577 "A Minecraft Server"
fetch "$VANILLA_URL" "$V/server.jar" sha1 "$VANILLA_SHA1"
start "$V" server.jar
# World setup of harness/skeleton-server/README.md (World, Gamerules). The values stay in level.dat.
$RCON --port 25577 \
  'setworldspawn 416 70 352' \
  'forceload add 336 272 495 431' \
  'gamerule spawn_mobs false' 'gamerule spawn_monsters false' \
  'gamerule advance_time false' 'time set noon' \
  'gamerule advance_weather false' 'weather clear' \
  'gamerule spawn_phantoms false' 'gamerule spawn_patrols false' \
  'gamerule spawn_wandering_traders false' 'gamerule spawn_wardens false' \
  'gamerule raids false' 'gamerule show_advancement_messages false' \
  'gamerule immediate_respawn true'
sleep 30  # the forced chunks generate over the next ticks
$RCON --port 25577 'forceload remove all' 'save-all flush'
stop "$V" 25577
[ "$kind" = vanilla ] && { echo "vanilla server ready"; exit 0; }

G=$H/grim-server/server
settings "$G" 25568 25578 "Null skeleton + GrimAC"
cp -r "$V/world" "$G/world"; rm -f "$G/world/session.lock"
mkdir -p "$G/plugins"
fetch "$PAPER_URL" "$G/paper.jar" sha256 "$PAPER_SHA256"
fetch "$GRIM_URL" "$G/plugins/grimac-bukkit-2.3.74-f5bbe9c.jar" sha256 "$GRIM_SHA256"
# First start: Paper converts the world and Grim writes its default files.
start "$G" paper.jar
$RCON --port 25578 'difficulty normal' 'save-all flush'  # Paper keeps the world's stored difficulty
stop "$G" 25578
# Every alert prints to the console (harness/grim-server/README.md, Grim settings).
cp "$H/grim-server/config/GrimAC/punishments.yml" "$G/plugins/GrimAC/punishments.yml"
echo "grim server ready"
