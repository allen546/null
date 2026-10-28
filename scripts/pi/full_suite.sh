#!/bin/sh
# Runs on the Pi: the ability-check full suite on vanilla, then on Paper + Grim, one server at a time.
# Usage: full_suite.sh <prefix> [--only vanilla|grim] [null-abilitycheck args, e.g. --shuffle 20261021];
# summary in logs/<prefix>_summary.txt. --only reruns one half after the other half passed.
# Started detached from the Mac by scripts/pi/launch.sh. Spec: specs/plans/ABILITY_CHECK.md §How to run
H=/mnt/pi-data/null-harness; L=$H/logs; cd $H || exit 1
prefix=$1; shift
kinds="vanilla grim"
if [ "${1:-}" = --only ]; then kinds=$2; shift 2; fi
# One Bark push to Allen's Mac when the pair ends; a failed push never fails the run.
# Bark's reply and exit code go to logs/<prefix>_notify.log (p39q's push was lost with no trace).
notify() {
  { echo "$(date '+%F %T') push: $1"; /mnt/pi-data/tools/bark -d mac -t "Null $prefix" "$1" 2>&1; echo "exit $?"; } >> $L/${prefix}_notify.log 2>&1 || true
}
if pgrep -x java >/dev/null || pgrep -f '^([^ ]*/)?null-abilitycheck( |$)' >/dev/null; then
  echo "ABORT: a server or a check is already running" > $L/${prefix}_summary.txt; notify "aborted: a server or a check is already running"; exit 1
fi
for k in $kinds; do /home/allen/.local/bin/uv run --no-project python run_server_suite.py $k $prefix --scenario all "$@" > $L/${prefix}_${k}_driver.log 2>&1; done
{
  for k in $kinds; do
    echo "== $k"
    grep -E "scenario\(s\)|^FAIL|server settle|TIMING total" $L/${prefix}_${k}_run.log | cut -c1-300
    grep -E "RCON running|WARMUP|BOT exit|GRIM alerts|server exit|DIED" $L/${prefix}_${k}_monitor.log
    grep TICK $L/${prefix}_${k}_monitor.log | sed -E 's/.*Average time per tick: ([0-9.]+)ms.*P99: ([0-9.]+)ms.*/\1 \2/' |
      sort -k2 -n | tail -1 | sed 's/^/worst P99 sample (mean, P99 ms): /'
  done
  echo "done $(date '+%F %T')"
} > $L/${prefix}_summary.txt
notify "$(for k in $kinds; do
  r=$(grep -E "scenario\(s\)" $L/${prefix}_${k}_run.log | tail -1 | sed -E 's/^([0-9]+) scenario\(s\), ([0-9]+) passed.*/\2\/\1/')
  a=$(grep -o "GRIM alerts in the suite: [0-9]*" $L/${prefix}_${k}_monitor.log | grep -o "[0-9]*$")
  printf '%s %s, alerts %s; ' $k "${r:-no result}" "${a:-?}"
done)done"
