#!/usr/bin/env bash
# Runs the ability-check full suite on vanilla, then on Paper + Grim, with one command.
# Usage: scripts/full_suite.sh <prefix> [extra null-abilitycheck args, e.g. --scenario A1,AT]
# Logs: <prefix>_vanilla.log, <prefix>_grim.log. Spec: specs/plans/ABILITY_CHECK.md §How to run
set -u
cd "$(dirname "$0")/.."

notify() { osascript -e "display notification \"$1\" with title \"null full suite\"" >/dev/null 2>&1 || true; }
abort() { echo "ABORT: $1" >&2; notify "aborted: $1"; exit 1; }

[ $# -ge 1 ] && [[ "$1" != -* ]] || { echo "usage: $0 <prefix> [extra args]" >&2; exit 2; }
prefix=$1; shift

pgrep -x null-abilitycheck >/dev/null && abort "another null-abilitycheck is running"

# name dir port rcon-port
servers=("vanilla harness/skeleton-server 25567 25577" "grim harness/grim-server 25568 25578")
for s in "${servers[@]}"; do
  read -r name dir port rport <<<"$s"
  if ! grep -q "RCON running" "$dir/server/server.log" 2>/dev/null \
     || ! nc -z 127.0.0.1 "$port" 2>/dev/null || ! nc -z 127.0.0.1 "$rport" 2>/dev/null; then
    echo "The $name server is not up. Start it and wait for \"RCON running\" in $dir/server/server.log:" >&2
    echo "  $dir/scripts/start_server.sh" >&2
    abort "$name server is not up"
  fi
done

cargo build --release -p null-abilitycheck || abort "build failed"

t_all=$(date +%s)
summary=""
notes=""
for s in "${servers[@]}"; do
  read -r name dir port rport <<<"$s"
  log="${prefix}_${name}.log"
  slog="$dir/server/server.log"
  mark=$(wc -c <"$slog" | tr -d ' ')
  t0=$(date +%s)
  ./target/release/null-abilitycheck --server "127.0.0.1:$port" --rcon-port "$rport" \
    --server-log "$slog" --account null_check --scenario all "$@" 2>&1 | tee "$log"
  secs=$(( $(date +%s) - t0 ))
  line=$(grep -E '^[0-9]+ scenario\(s\), [0-9]+ passed, [0-9]+ failed' "$log" | tail -1)
  alerts=n/a
  if [ "$name" = grim ]; then
    alerts=$(tail -c +"$((mark + 1))" "$slog" | sed $'s/\x1b\\[[0-9;]*m//g' | grep -c 'Grim.* failed ')
  fi
  failed_rows=$(grep -E '^FAIL ' "$log" | sed 's/ — .*//' || true)
  summary+="$name: ${line:-no summary line (run aborted?)}; Grim alerts in window: $alerts; time ${secs} s"$'\n'
  [ -n "$failed_rows" ] && summary+="  failed rows:"$'\n'"$(sed 's/^/    /' <<<"$failed_rows")"$'\n'
  p=$(sed -E 's/^[0-9]+ scenario\(s\), ([0-9]+) passed.*/\1/' <<<"$line")
  f=$(sed -E 's/.*, ([0-9]+) failed.*/\1/' <<<"$line")
  notes+="$name ${p:-?} passed ${f:-?} failed, ${alerts} alerts; "
done

echo
echo "==== full suite summary ($prefix) ===="
printf '%s' "$summary"
total=$(( $(date +%s) - t_all ))
echo "total time ${total} s (build included)"
notify "${notes}${total} s"
