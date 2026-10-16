#!/bin/zsh
# usage: swarm_duel.sh [N=8]  -- team null_a1..aN vs null_b1..bN, both from world spawn.
# Join at localhost:25567 as any other name to watch. Stop: pkill -f null-node (server keeps running).
S=${0:A:h:h}; N=${1:-8}
NODE=${NODE:-/Users/allen/Desktop/null/target/release/null-node}
R=$S/runs/duel-$(date +%Y%m%d-%H%M%S); mkdir -p $R
NORESP=; [[ $RESPAWN == 0 ]] && NORESP=--no-respawn   # RESPAWN=0: dead stays dead (last team standing)
rc() { python3 $S/scripts/rcon.py "$@"; }
if ! rc list >/dev/null 2>&1; then
  $S/scripts/start_server.sh
  until grep -q 'RCON running' $S/server/server.log 2>/dev/null; do sleep 1; done
fi
rc 'gamerule keep_inventory true' 'gamerule immediate_respawn true' 'gamerule natural_health_regeneration true' >/dev/null
wc -l < $S/server/server.log > $R/baseline
for t in a b; do
  o=a; [[ $t == a ]] && o=b
  nohup $NODE --server 127.0.0.1:25567 --accounts null_${t}1..null_${t}$N --center 416,352 \
    --fight $NORESP --enemies null_${o}1..null_${o}$N --state-log $R/$t.state.jsonl > $R/$t.stderr 2>&1 &
  echo $! >> $R/pids
done
nohup $S/scripts/glow_teams.sh $N >/dev/null 2>&1 &
sleep $(( N * 1 + 8 ))
# Kit: sword for all; chestplate and shield by independent seeded coin flips (SEED=n repeats a run).
RANDOM=${SEED:-$$}; : > $R/kits.txt
for t in a b; do for i in {1..$N}; do
  c=$(( RANDOM % 2 )); sh=$(( RANDOM % 2 )); b=null_$t$i
  rc "give $b iron_sword" >/dev/null
  (( c )) && rc "item replace entity $b armor.chest with iron_chestplate" >/dev/null
  (( sh )) && rc "item replace entity $b weapon.offhand with shield" >/dev/null
  echo "$b chest=$c shield=$sh" >> $R/kits.txt
done; done
echo "duel running, logs in $R, pids: $(tr '\n' ' ' < $R/pids)"
