#!/bin/bash
# Replay an input log in the vanilla client through the tap. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
#
#   run_replay.sh --input FILE --tap-out FILE --marks FILE [--row LABEL] [--scene A15] [--server 127.0.0.1:25567]
#
# A fresh player name per run (the server keeps a player's position). With --scene the scene is set
# by `null-abilitycheck --prepare-scene` (the module's own scene code) once the client has joined;
# without it the player is left where it spawns (hand-written files). Then the control file is
# touched, the mod replays, the client quits, the tap stops.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"; ROOT="$D/../.."
SERVER=127.0.0.1:25567; SCENE=""; ROW=""
while [ $# -gt 0 ]; do case "$1" in
  --input) INPUT="$2"; shift 2;; --tap-out) TAP_OUT="$2"; shift 2;; --marks) MARKS="$2"; shift 2;;
  --row) ROW="$2"; shift 2;; --scene) SCENE="$2"; shift 2;; --server) SERVER="$2"; shift 2;;
  *) echo "unknown flag $1" >&2; exit 2;; esac; done
INPUT="$(cd "$(dirname "$INPUT")" && pwd)/$(basename "$INPUT")"
mkdir -p "$(dirname "$TAP_OUT")"; TAP_OUT="$(cd "$(dirname "$TAP_OUT")" && pwd)/$(basename "$TAP_OUT")"
MARKS="$(cd "$(dirname "$MARKS")" && pwd)/$(basename "$MARKS")"
NAME="VD$(date +%m%d%H%M%S)"; CONTROL="$TAP_OUT.control"
rm -f "$CONTROL" "$MARKS" "$TAP_OUT" "$TAP_OUT.2"
PORT=25569
"$ROOT/target/release/vd-tap" --listen 127.0.0.1:$PORT --target "$SERVER" --out "$TAP_OUT" 2> "$TAP_OUT.tap.out" &
TAP=$!
trap 'kill $TAP 2>/dev/null || true' EXIT
JVM="-Dvd.input=$INPUT -Dvd.control=$CONTROL -Dvd.marks=$MARKS"
[ -n "$ROW" ] && JVM="$JVM -Dvd.row=$ROW"
VD_USER="$NAME" VD_JVM="$JVM" "${VD_CLIENT_DIR:-$HOME/Desktop/null-spikes/vanilla-diff-client}/run_client.sh" > "$TAP_OUT.client.out" 2>&1 &
CLIENT=$!
echo "vanilla player $NAME, tap log $TAP_OUT"
if [ -n "$SCENE" ]; then
  "$ROOT/target/release/null-abilitycheck" --prepare-scene "$SCENE" --player "$NAME"
fi
touch "$CONTROL"
wait $CLIENT || true
echo "replay marks: $(cat "$MARKS" 2>/dev/null || echo MISSING)"
