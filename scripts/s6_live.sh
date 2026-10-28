#!/usr/bin/env bash
# S6a live run on the vanilla test server (port 25567, RCON 25577): two bots, 64 oak logs, through null-node --spine.
# Needs the server up ("RCON running" in harness/skeleton-server/server/server.log) and target/release/null-node built.
# Spec: specs/plans/S6_BRIEF.md (task S6a). Options: see scripts/s6_live.py.
set -euo pipefail
cd "$(dirname "$0")/.."
exec uv run --quiet python -I scripts/s6_live.py "$@"
