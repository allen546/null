#!/bin/bash
# One command: chosen modules of the ability check through the vanilla diff on one test server. Spec: specs/plans/VD3_BRIEF.md T1
#
#   run_vd3.sh --server vanilla|grim --modules A1,A2,A3,A4,A22 --out DIR --account NAME [--rows ID,ID] [--skip-null]
#
# 1. null-abilitycheck runs the modules through the tap with the input log on (`run_null.sh`).
# 2. run_rows.py replays each row in a fresh vanilla client (not op) and diffs the packets; on grim it
#    also reads both players' Grim alerts from each row's own window in server.log and compares them.
# Output: DIR/null.log (the ability check), DIR/null_tap.jsonl, DIR/inputlog/, DIR/rows/ (per row files, README "VD3 formats"),
# DIR/summary.txt. --skip-null re-uses DIR's Null run (the replays only). A run of 20+ rows takes over 4 minutes: run it with nohup.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"; ROOT="$D/../.."
SERVER=""; MODULES=""; OUT=""; ACCOUNT=""; ROWS=""; SKIP_NULL=0
while [ $# -gt 0 ]; do case "$1" in
  --server) SERVER="$2"; shift 2;; --modules) MODULES="$2"; shift 2;; --out) OUT="$2"; shift 2;; --account) ACCOUNT="$2"; shift 2;;
  --rows) ROWS="$2"; shift 2;; --skip-null) SKIP_NULL=1; shift;; *) echo "unknown flag $1" >&2; exit 2;; esac; done
case "$SERVER" in
  vanilla) ADDR=127.0.0.1:25567; RCON=25577; SLOG=""; SDIR="$ROOT/harness/skeleton-server/server";;
  grim) ADDR=127.0.0.1:25568; RCON=25578; SDIR="$ROOT/harness/grim-server/server"; SLOG="$SDIR/server.log";;
  *) echo "--server vanilla|grim" >&2; exit 2;; esac
[ -n "$MODULES" ] && [ -n "$OUT" ] && [ -n "$ACCOUNT" ] || { echo "--modules, --out and --account are needed" >&2; exit 2; }
mkdir -p "$OUT"; OUT="$(cd "$OUT" && pwd)"
if pgrep -f "release/null-abilitycheck" > /dev/null; then echo "another null-abilitycheck is running" >&2; exit 1; fi
grep -q "RCON running" "$SDIR/server.log" || { echo "no RCON running line in $SDIR/server.log" >&2; exit 1; }
# Ops are exempt from Grim: the vanilla player has a fresh name (VD<time>), so it is op only if ops.json lists a VD name or all of them.
if grep -q '"name": "VD' "$SDIR/ops.json"; then echo "ops.json lists a VD player: the vanilla player would be exempt from Grim" >&2; exit 1; fi
SLOGFLAG=(); [ -n "$SLOG" ] && SLOGFLAG=(--server-log "$SLOG")
if [ "$SKIP_NULL" = 0 ]; then
  # run_null.sh runs the release binary as it is: a stale one would not test the tree.
  (cd "$ROOT" && cargo build --release -p null-abilitycheck)
  "$D/run_null.sh" --tap-out "$OUT/null_tap.jsonl" --input-log "$OUT/inputlog" --target "$ADDR" \
    --scenario "$MODULES" --account "$ACCOUNT" --rcon-port "$RCON" --server-log "$SDIR/server.log" 2>&1 | tee "$OUT/null.log"
fi
SEL=(--module "$MODULES"); [ -n "$ROWS" ] && SEL=(--rows "$ROWS")
set +e
uv run "$D/run_rows.py" --null-tap "$OUT/null_tap.jsonl" --input-log "$OUT/inputlog/$ACCOUNT.jsonl" --out "$OUT/rows" "${SEL[@]}" \
  --server "$ADDR" --rcon-port "$RCON" ${SLOG:+--server-log "$SLOG"} 2>&1 | tee "$OUT/rows.log"
RC=${PIPESTATUS[0]}
sed -n '/== SUMMARY/,$p' "$OUT/rows.log" > "$OUT/summary.txt"
echo "done: exit $RC, summary $OUT/summary.txt"
exit $RC
