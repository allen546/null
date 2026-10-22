#!/usr/bin/env bash
# Runs on the Mac: starts scripts/pi/full_suite.sh on the Pi, detached, and returns at once.
# Usage: scripts/pi/launch.sh <prefix> [null-abilitycheck args]; read the result later with
#   ssh 100.95.33.78 cat /mnt/pi-data/null-harness/logs/<prefix>_summary.txt
# Deploy the binary first (scripts/pi/deploy.sh). Spec: specs/plans/ABILITY_CHECK.md §How to run
set -eu
[ $# -ge 1 ] || { echo "usage: $0 <prefix> [args]" >&2; exit 2; }
# No `cd ... &&` before the `&`: that keeps a subshell holding the SSH channel open.
ssh -n 100.95.33.78 "cd /mnt/pi-data/null-harness; setsid nohup ./full_suite.sh $* > logs/$1_wrapper.log 2>&1 < /dev/null &"
echo "started on the Pi; summary: /mnt/pi-data/null-harness/logs/$1_summary.txt"
