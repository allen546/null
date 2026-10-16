#!/usr/bin/env python3
"""Sample bot positions through RCON until the stop file appears or the time limit passes.
usage: watch_positions.py OUT.jsonl PREFIX FIRST LAST [SECONDS]  (bots PREFIX<FIRST>..PREFIX<LAST>)"""
import json, os, re, sys, time
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import rcon
out, prefix, first, last = sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4])
limit = float(sys.argv[5]) if len(sys.argv) > 5 else 300
names = [f"{prefix}{i}" for i in range(first, last + 1)]
t0 = time.time()
with open(out, "w") as f:
    while time.time() - t0 < limit:
        res = rcon.run([f"data get entity {n} Pos" for n in names])
        row = {"t": round(time.time() - t0, 2), "pos": {}}
        for n, r in zip(names, res):
            m = re.search(r"\[(-?[\d.]+)d, (-?[\d.]+)d, (-?[\d.]+)d\]", r)
            if m:
                row["pos"][n] = [float(m.group(i)) for i in (1, 2, 3)]
        f.write(json.dumps(row) + "\n"); f.flush()
        if os.path.exists(out + ".stop"):
            break
        time.sleep(1.0)
