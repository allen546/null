#!/bin/zsh
# Team A glows red, team B blue (glow colour = team colour). Effects clear on death, so
# this re-applies every 3 s until no null-node is left. usage: glow_teams.sh [N=8]
S=${0:A:h:h}; N=${1:-8}
rc() { python3 $S/scripts/rcon.py "$@" >/dev/null 2>&1; }
rc 'team add A' 'team modify A color red' 'team add B' 'team modify B color blue'
sleep 15
while pgrep -f null-node >/dev/null; do
  cmds=()
  for t in a b; do for i in {1..$N}; do
    cmds+=("team join ${(U)t} null_$t$i" "effect give null_$t$i glowing infinite 0 true")
  done; done
  rc "${cmds[@]}"
  sleep 3
done
