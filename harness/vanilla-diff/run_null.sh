#!/bin/bash
# Run null-abilitycheck through the tap with the input log on. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
#
#   run_null.sh --tap-out FILE --input-log DIR [abilitycheck flags, e.g. --scenario A15 --account NAME]
#
# The tap listens on 25569 and forwards to the vanilla server (--target to change); the
# abilitycheck connects to the tap. The tap stops when the run ends.
set -euo pipefail
D="$(cd "$(dirname "$0")" && pwd)"; ROOT="$D/../.."
TARGET=127.0.0.1:25567
while [ $# -gt 0 ]; do case "$1" in
  --tap-out) TAP_OUT="$2"; shift 2;; --input-log) LOGDIR="$2"; shift 2;; --target) TARGET="$2"; shift 2;; *) break;; esac; done
rm -f "$TAP_OUT" "$TAP_OUT.2"
"$ROOT/target/release/vd-tap" --listen 127.0.0.1:25569 --target "$TARGET" --out "$TAP_OUT" 2> "$TAP_OUT.tap.out" &
TAP=$!
trap 'kill $TAP 2>/dev/null || true' EXIT
sleep 1
"$ROOT/target/release/null-abilitycheck" --server 127.0.0.1:25569 --input-log "$LOGDIR" "$@"
