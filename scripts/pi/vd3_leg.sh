#!/bin/bash
# Runs on the Pi: starts one test server, warms it up, runs run_vd3.sh on it, stops it. Spec: specs/plans/VD3_BRIEF.md
# Usage: vd3_leg.sh vanilla|grim MODULES OUT ACCOUNT [ROWS]. The Null run and the replays share one server start, so one server.log; a copy stays in OUT.
H=/mnt/pi-data/null-harness; V=$H/vd
export PATH=/home/allen/.local/bin:$PATH VD_NO_BUILD=1 VD_CLIENT_DIR=$H/vd-client
KIND=$1; MODULES=$2; OUT=$3; ACCOUNT=$4; ROWS=${5:-}
case "$KIND" in
  vanilla) DIR=$H/skeleton-server/server; JAR=server.jar; PORT=25567; RC=25577;;
  grim) DIR=$H/grim-server/server; JAR=paper.jar; PORT=25568; RC=25578;;
  *) echo "vanilla|grim" >&2; exit 2;; esac
mkdir -p "$OUT"
if pgrep -x java > /dev/null || pgrep -f 'null-abilitycheck( |$)' > /dev/null; then echo "a server or a check is already running" >&2; exit 1; fi
# Same flags as run_server_suite.py.
cd "$DIR" && java -Xms1G -Xmx2G -XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=100 -jar "$JAR" nogui > server.log 2>&1 < /dev/null &
SRV=$!
for _ in $(seq 1 600); do grep -q "RCON running" "$DIR/server.log" && break; kill -0 $SRV 2>/dev/null || { echo "server died" >&2; exit 1; }; sleep 1; done
grep -q "RCON running" "$DIR/server.log" || { kill $SRV; echo "no RCON after 600 s" >&2; exit 1; }
sleep 5
cd "$V"
# A fresh server lags when the first players join and draws TimerLimit alerts (run_server_suite.py); this takes that lag.
target/release/null-abilitycheck --server 127.0.0.1:$PORT --rcon-port $RC --server-log "$DIR/server.log" --account null_warm --scenario A1,A3,A8 > "$OUT/warmup.log" 2>&1
sleep 10
harness/vanilla-diff/run_vd3.sh --server "$KIND" --modules "$MODULES" --out "$OUT" --account "$ACCOUNT" ${ROWS:+--rows "$ROWS"}
RC_LEG=$?
uv run --no-project python -c "import sys; sys.path.insert(0, '$H/skeleton-server/scripts'); import rcon; rcon.run(['stop'], port=$RC)" || kill $SRV
for _ in $(seq 1 180); do kill -0 $SRV 2>/dev/null || break; sleep 1; done
kill $SRV 2>/dev/null
cp "$DIR/server.log" "$OUT/server.log"
exit $RC_LEG
