#!/bin/zsh
# usage: start_server.sh (background; log in server/server.log). Paper + GrimAC, port 25568, RCON 25578.
cd "$(dirname $0)/../server"
# A second start would fail on the world lock and truncate the running server's log.
if [ -f server.pid ] && kill -0 "$(cat server.pid)" 2>/dev/null; then
  echo "already running (pid $(cat server.pid))" >&2; exit 1
fi
nohup java -Xms2G -Xmx3G -XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=100 -jar paper.jar nogui > server.log 2>&1 &
echo $! > server.pid
