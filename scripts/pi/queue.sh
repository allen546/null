#!/bin/sh
# Runs on the Pi: a queue of repro jobs, in order, one server at a time (diagnosis of flaky rows, not checks).
# Usage: queue.sh <prefix> <kind>:<modules>:<repeats> ...   e.g. queue.sh q1 grim:A17:30 vanilla:A10E:30
# Job i logs as <prefix>_j<i>_<kind>_*; one summary in logs/<prefix>_summary.txt and one Bark push at the end.
# Started detached from the Mac by SCRIPT=queue.sh scripts/pi/launch.sh. Spec: specs/plans/ABILITY_CHECK.md §How to run
H=/mnt/pi-data/null-harness; L=$H/logs; cd $H || exit 1
prefix=$1; shift
notify() { /mnt/pi-data/tools/bark -d mac -t "Null $prefix" "$1" >/dev/null 2>&1 || true; }
if pgrep -x java >/dev/null || pgrep -f '^([^ ]*/)?null-abilitycheck( |$)' >/dev/null; then
  echo "ABORT: a server or a check is already running" > $L/${prefix}_summary.txt; notify "aborted: a server or a check is already running"; exit 1
fi
i=0; : > $L/${prefix}_summary.txt; short=""
for job in "$@"; do
  i=$((i + 1)); kind=${job%%:*}; rest=${job#*:}; mods=${rest%%:*}; n=${rest#*:}; p=${prefix}_j$i
  /home/allen/.local/bin/uv run --no-project python run_server_suite.py $kind $p --repeat $n --scenario $mods > $L/${p}_${kind}_driver.log 2>&1
  runs=$(grep -cE "scenario\(s\)" $L/${p}_${kind}_run.log)
  bad=$(grep -E "scenario\(s\)" $L/${p}_${kind}_run.log | grep -cv " 0 failed")
  {
    echo "== job $i: $kind $mods x$n: $runs runs ended, $bad with a failure"
    grep -E "^FAIL" $L/${p}_${kind}_run.log | cut -c1-300
    grep -E "RCON running|BOT exit|GRIM alerts|server exit|DIED" $L/${p}_${kind}_monitor.log
  } >> $L/${prefix}_summary.txt
  short="$short$kind $mods $((runs - bad))/$n clean; "
done
echo "done $(date '+%F %T')" >> $L/${prefix}_summary.txt
notify "${short}done"
