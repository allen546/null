#!/bin/zsh
# graceful stop via RCON, then wait for the PID
cd "$(dirname $0)/../server"
python3 ../../skeleton-server/scripts/rcon.py --port 25578 stop >/dev/null 2>&1
pid=$(cat server.pid 2>/dev/null)
for i in {1..120}; do kill -0 $pid 2>/dev/null || { echo stopped; rm -f server.pid; exit 0 }; sleep 1; done
echo "still running: $pid" >&2; exit 1
