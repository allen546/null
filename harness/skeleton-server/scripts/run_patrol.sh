#!/bin/zsh
# usage: run_patrol.sh TAG [N]  -- runs N (default 20) null_bN bots through the patrol and
# samples server-side positions over RCON. Output in runs/TAG.*
S=${0:A:h:h}; R=$S/runs; mkdir -p $R
TAG=$1; N=${2:-20}
rm -f $R/$TAG.pos.jsonl.stop $R/$TAG.exit
wc -l < $S/server/server.log > $R/$TAG.baseline
( ${NODE:-/Users/allen/Desktop/null/target/release/null-node} --server 127.0.0.1:25567 --accounts null_b1..null_b$N \
  --exit-after-skills --state-log $R/$TAG.state.jsonl > $R/$TAG.stderr 2>&1
  echo "node exit $?" > $R/$TAG.exit; touch $R/$TAG.pos.jsonl.stop ) &
python3 $S/scripts/watch_positions.py $R/$TAG.pos.jsonl null_b 1 $N 400
cat $R/$TAG.exit
B=$(cat $R/$TAG.baseline); tail -n +$B $S/server/server.log > $R/$TAG.serverlog
echo "skills done: $(grep skill_ended $R/$TAG.state.jsonl | grep -c '"done"') failed: $(grep skill_ended $R/$TAG.state.jsonl | grep -c '"failed"')"
echo "server log: moved-warnings $(grep -ciE 'moved too quickly|moved wrongly' $R/$TAG.serverlog) kicks $(grep -ciE 'kicked' $R/$TAG.serverlog) WARN/ERROR $(grep -cE 'WARN|ERROR' $R/$TAG.serverlog) joined $(grep -c 'joined the game' $R/$TAG.serverlog) left $(grep -c 'left the game' $R/$TAG.serverlog)"
python3 -I - $R/$TAG.pos.jsonl <<'PY'
import json, math, sys
rows=[json.loads(l) for l in open(sys.argv[1])]
home={}
for r in rows:
    for n,p in r['pos'].items(): home.setdefault(n,p)
res=[]
for n in home:
    ds=[math.hypot(r['pos'][n][0]-home[n][0], r['pos'][n][2]-home[n][2]) for r in rows if n in r['pos']]
    st='home'; c=0
    for d in ds:
        if st=='home' and d>12: st='out'
        elif st=='out' and d<4: st='home'; c+=1
    res.append((n,round(max(ds),1),c,round(ds[-1],1)))
print('server-side: bots',len(res),'with 2 out-and-back cycles',sum(1 for *_,c,_ in [(a,b,c,d) for a,b,c,d in res] if c==2),
      'max-dist',min(m for _,m,_,_ in res),'..',max(m for _,m,_,_ in res),'final-dist<=2:',sum(1 for *_,f in res if f<=2))
PY
