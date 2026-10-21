#!/usr/bin/env bash
# Prints the ability-check modules a change affects, as one `--scenario` argument (or `--scenario all`).
# Reads the changed files of the Null tree (against HEAD, plus untracked) and of the fork (against the
# pinned rev, or --fork-rev), and matches them against crates/null-abilitycheck/run_map.toml.
# A path the map does not know makes it print `all` and name the path (stderr).
# Usage: scripts/affected_modules.sh [--fork-rev <rev>] [--fork <dir>] [--explain] [--check-history]
# Spec: specs/plans/ABILITY_CHECK.md §Which modules a change reruns
set -euo pipefail
cd "$(dirname "$0")/.."
exec uv run --quiet python -I scripts/affected_modules.py "$@"
