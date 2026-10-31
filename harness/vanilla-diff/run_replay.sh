#!/bin/bash
# Replay an input log in the vanilla client through the tap. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
#
#   run_replay.sh --input FILE --tap-out FILE --marks FILE [--row LABEL] [--scene A15:ROW] [--phase N] [--server 127.0.0.1:25567]
#
# A fresh player name per run (the server keeps a player's position). With --scene the scene is set
# by `null-abilitycheck --prepare-scene` (the module's own scene code) once the client has joined;
# without it the player is left where it spawns (hand-written files). Then the control file is
# touched, the mod replays, the client quits, the tap stops. The scene process stays up while the
# replay runs: it does the row's RCON actions when the mod reports their marks (`$TAP_OUT.progress`).
# --phase N sets vanilla's position-reminder counter at the first replayed tick (the differ's --phase).
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"; ROOT="$D/../.."
SERVER=127.0.0.1:25567; SCENE=""; ROW=""; PHASE=-1
while [ $# -gt 0 ]; do case "$1" in
  --input) INPUT="$2"; shift 2;; --tap-out) TAP_OUT="$2"; shift 2;; --marks) MARKS="$2"; shift 2;;
  --row) ROW="$2"; shift 2;; --scene) SCENE="$2"; shift 2;; --phase) PHASE="$2"; shift 2;; --server) SERVER="$2"; shift 2;;
  *) echo "unknown flag $1" >&2; exit 2;; esac; done
INPUT="$(cd "$(dirname "$INPUT")" && pwd)/$(basename "$INPUT")"
mkdir -p "$(dirname "$TAP_OUT")"; TAP_OUT="$(cd "$(dirname "$TAP_OUT")" && pwd)/$(basename "$TAP_OUT")"
MARKS="$(cd "$(dirname "$MARKS")" && pwd)/$(basename "$MARKS")"
NAME="VD$(date +%m%d%H%M%S)"; CONTROL="$TAP_OUT.control"; PROGRESS="$TAP_OUT.progress"; SCENE_OUT="$TAP_OUT.scene.out"
rm -f "$CONTROL" "$MARKS" "$TAP_OUT" "$TAP_OUT.2" "$PROGRESS" "$SCENE_OUT"
PORT=25569
"$ROOT/target/release/vd-tap" --listen 127.0.0.1:$PORT --target "$SERVER" --out "$TAP_OUT" 2> "$TAP_OUT.tap.out" &
TAP=$!
trap 'kill $TAP 2>/dev/null || true' EXIT
JVM="-Dvd.input=$INPUT -Dvd.control=$CONTROL -Dvd.marks=$MARKS -Dvd.progress=$PROGRESS -Dvd.phase=$PHASE"
[ -n "$ROW" ] && JVM="$JVM -Dvd.row=$ROW"
VD_USER="$NAME" VD_JVM="$JVM" "${VD_CLIENT_DIR:-$HOME/Desktop/null-spikes/vanilla-diff-client}/run_client.sh" > "$TAP_OUT.client.out" 2>&1 &
CLIENT=$!
echo "vanilla player $NAME, tap log $TAP_OUT"
SCENEPID=""
if [ -n "$SCENE" ]; then
  "$ROOT/target/release/null-abilitycheck" --prepare-scene "$SCENE" --player "$NAME" --progress "$PROGRESS" > "$SCENE_OUT" 2>&1 &
  SCENEPID=$!
  trap 'kill $TAP $SCENEPID 2>/dev/null || true' EXIT
  for _ in $(seq 1 600); do
    grep -q " ready for " "$SCENE_OUT" && break
    kill -0 $SCENEPID 2>/dev/null || { cat "$SCENE_OUT" >&2; echo "scene process ended before the scene was ready" >&2; exit 1; }
    sleep 1
  done
fi
touch "$CONTROL"
wait $CLIENT || true
[ -n "$SCENEPID" ] && { sleep 2; kill $SCENEPID 2>/dev/null || true; cat "$SCENE_OUT"; }
echo "replay marks: $(cat "$MARKS" 2>/dev/null || echo MISSING)"
