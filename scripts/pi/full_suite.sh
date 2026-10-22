#!/bin/sh
# Runs on the Pi: the ability-check full suite on vanilla, then on Paper + Grim, one server at a time.
# Usage: full_suite.sh <prefix> [null-abilitycheck args, e.g. --shuffle 20261021]; summary in logs/<prefix>_summary.txt.
# Started detached from the Mac by scripts/pi/launch.sh. Spec: specs/plans/ABILITY_CHECK.md §How to run
H=/mnt/pi-data/null-harness; L=$H/logs; cd $H || exit 1
prefix=$1; shift
if pgrep -x java >/dev/null || pgrep -f '^([^ ]*/)?null-abilitycheck( |$)' >/dev/null; then
  echo "ABORT: a server or a check is already running" > $L/${prefix}_summary.txt; exit 1
fi
for k in vanilla grim; do /home/allen/.local/bin/uv run --no-project python run_server_suite.py $k $prefix --scenario all "$@" > $L/${prefix}_${k}_driver.log 2>&1; done
{
  for k in vanilla grim; do
    echo "== $k"
    grep -E "scenario\(s\)|^FAIL|server settle|TIMING total" $L/${prefix}_${k}_run.log | cut -c1-300
    grep -E "RCON running|WARMUP|BOT exit|GRIM alerts|server exit|DIED" $L/${prefix}_${k}_monitor.log
    grep TICK $L/${prefix}_${k}_monitor.log | sed -E 's/.*Average time per tick: ([0-9.]+)ms.*P99: ([0-9.]+)ms.*/\1 \2/' |
      sort -k2 -n | tail -1 | sed 's/^/worst P99 sample (mean, P99 ms): /'
  done
  echo "done $(date '+%F %T')"
} > $L/${prefix}_summary.txt
