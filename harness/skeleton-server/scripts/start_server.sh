#!/bin/zsh
# usage: start_server.sh (background; log in server/server.log)
cd "$(dirname $0)/../server"
nohup java -Xms2G -Xmx3G -XX:+UseG1GC -XX:+ParallelRefProcEnabled -XX:MaxGCPauseMillis=100 -jar server.jar nogui > server.log 2>&1 &
echo $! > server.pid
