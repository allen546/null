#!/usr/bin/env python3
"""S6a live run: two bots collect 64 oak logs through the live spine on the vanilla test server.

Plants 24 oak trees (6 logs each) by RCON in an area no ability-check scenario uses, writes the tree list, starts
`null-node --spine`, sends `set-goal obtain oak_log 64`, waits for the spine to go idle, then reads back by RCON:
each bot's oak_log count, every planted log cell, and the server log. It also checks the operation's event log
(no tree claimed twice at once; every broken cell lies in a claim box that was granted).
Spec: specs/plans/S6_BRIEF.md (task S6a). Usage: scripts/s6_live.sh [--timeout S] [--out DIR] [--port 25567 --rcon 25577]
"""
import argparse, importlib.util, json, os, signal, subprocess, sys, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("rcon", ROOT / "harness/skeleton-server/scripts/rcon.py")
rcon = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rcon)

FLOOR_Y = 199          # the harness floor level (scenarios.rs FLOOR_Y)
X0, Z0 = 1200, 700     # no ability-check module uses this area (they sit at z=342, x up to 1090)
COLS, ROWS, SPACING, HEIGHT = 6, 4, 8, 6
BOTS = ["null_s6a1", "null_s6a2"]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--port", type=int, default=25567)
    ap.add_argument("--rcon", type=int, default=25577)
    ap.add_argument("--timeout", type=int, default=900)
    ap.add_argument("--out", default=None)
    ap.add_argument("--server-dir", default=str(ROOT / "harness/skeleton-server/server"))
    ap.add_argument("--binary", default=str(ROOT / "target/release/null-node"))
    a = ap.parse_args()
    out = Path(a.out or ROOT / f"harness/skeleton-server/runs/s6a-{time.strftime('%Y%m%d-%H%M%S')}")
    out.mkdir(parents=True, exist_ok=True)
    log_path = Path(a.server_dir) / "server.log"

    def r(*cmds):
        return rcon.run(list(cmds), port=a.rcon)

    # 1. plant
    x1, z1 = X0 + COLS * SPACING, Z0 + ROWS * SPACING
    r(f"forceload add {X0} {Z0} {x1} {z1}",
      f"fill {X0} {FLOOR_Y} {Z0} {x1} {FLOOR_Y} {z1} minecraft:stone",
      f"fill {X0} {FLOOR_Y + 1} {Z0} {x1} {FLOOR_Y + 20} {z1} minecraft:air",
      f"kill @e[type=minecraft:item,x={X0},y={FLOOR_Y},z={Z0},dx={x1 - X0},dy=30,dz={z1 - Z0}]")
    trees, cells = [], []
    for i in range(COLS):
        for j in range(ROWS):
            x, z = X0 + 4 + i * SPACING, Z0 + 4 + j * SPACING
            base, top = FLOOR_Y + 1, FLOOR_Y + HEIGHT
            leaves = "minecraft:oak_leaves[persistent=false,distance=1]"
            r(f"fill {x-2} {top-1} {z-2} {x+2} {top} {z+2} {leaves}",
              f"fill {x-1} {top+1} {z-1} {x+1} {top+2} {z+1} {leaves}",
              f"fill {x} {base} {z} {x} {top} {z} minecraft:oak_log")
            trees.append({"x": x, "y": base, "z": z, "logs": HEIGHT})
            cells += [(x, y, z) for y in range(base, top + 1)]
    (out / "trees.json").write_text(json.dumps(trees))
    time.sleep(2)
    planted = sum("passed" in o for o in r(*[f"execute if block {x} {y} {z} minecraft:oak_log" for x, y, z in cells]))
    print(f"planted {planted}/{len(cells)} log cells")
    assert planted == len(cells), "planting failed"
    # Generating the new chunks can overload the server for a moment; the log of the run starts after it settles.
    time.sleep(20)
    log_start = log_path.stat().st_size

    # 2. start the node, wait for both bots
    cmd = [a.binary, "--server", f"127.0.0.1:{a.port}", "--accounts", ",".join(BOTS), "--spine", "--trees", str(out / "trees.json"),
           "--audit", str(out / "audit.jsonl"), "--op-log-dir", str(out), "--state-log", str(out / "state.jsonl")]
    spine_log = open(out / "spine.log", "w")
    node = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=spine_log, stderr=open(out / "node.err", "w"), text=True, bufsize=1)
    try:
        t0 = time.time()
        while True:
            online = r("list")[0]
            if all(b in online for b in BOTS):
                break
            assert time.time() - t0 < 60 and node.poll() is None, f"bots did not join: {online}"
            time.sleep(1)
        time.sleep(2)
        for i, b in enumerate(BOTS):
            r(f"clear {b}", f"gamemode survival {b}",
              f"execute in minecraft:overworld run tp {b} {X0 + 2 + i * 3}.5 {FLOOR_Y + 1} {Z0 + 2}.5 0 0")
        time.sleep(4)
        # 3. goal
        t_goal = time.time()
        node.stdin.write('set-goal obtain oak_log 64 --reason "S6a live"\n')
        node.stdin.flush()
        idle = False
        while time.time() - t_goal < a.timeout:
            if "spine idle" in (out / "spine.log").read_text():
                idle = True
                break
            if node.poll() is not None:
                break
            time.sleep(2)
        elapsed = time.time() - t_goal
        node.stdin.write("status\n")
        node.stdin.flush()
        time.sleep(1)
        # Read the bots' inventories while they are online: `clear ... 0` only counts, it takes nothing.
        counts = {}
        for b in BOTS:
            o = r(f"clear {b} minecraft:oak_log 0")[0]
            counts[b] = int(o.split()[1]) if o.startswith("Found") else 0
            print(f"readback {b}: {o}")
    finally:
        node.send_signal(signal.SIGINT)
        try:
            node.wait(10)
        except subprocess.TimeoutExpired:
            node.kill()

    # 4. readback of the world (the bots are gone; the cells stay)
    states = r(*[f"execute if block {x} {y} {z} minecraft:oak_log" for x, y, z in cells])
    standing = [c for c, o in zip(cells, states) if "passed" in o]
    broken = [c for c in cells if c not in standing]
    log = log_path.read_bytes()[log_start:].decode(errors="replace").splitlines()
    warns = [l for l in log if "WARN" in l or "ERROR" in l or "Can't keep up" in l]
    ops = json.loads((Path(a.server_dir) / "ops.json").read_text())
    bots_op = [o["name"] for o in ops if o["name"] in BOTS]

    # 5. event log checks
    double, outside = check_events(out, broken)
    (out / "server-log-slice.txt").write_text("\n".join(log))
    res = {
        "idle": idle, "seconds_to_idle": round(elapsed, 1), "oak_log_by_bot": counts, "oak_log_total": sum(counts.values()),
        "log_cells_planted": len(cells), "log_cells_broken": len(broken), "double_claims": double, "broken_outside_claims": outside,
        "server_log_warnings": warns, "bots_op": bots_op, "out": str(out),
    }
    (out / "result.json").write_text(json.dumps(res, indent=2))
    print(json.dumps(res, indent=2))
    ok = idle and sum(counts.values()) >= 64 and not double and not outside and not warns and not bots_op
    print("S6a done-when:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


def check_events(out, broken):
    """Two checks on the operation's JSONL log: no overlapping claims held at once; each broken cell is in a granted box."""
    held, double, boxes = {}, [], []

    def box(kind):
        reg = kind.get("Region")
        return None if reg is None else (reg["min"], reg["max"])

    def clash(p, q):
        return all(p[0][k] <= q[1][k] and q[0][k] <= p[1][k] for k in "xyz")

    for f in sorted(out.glob("*.jsonl")):
        if f.name in ("audit.jsonl", "state.jsonl"):
            continue
        for line in f.read_text().splitlines():
            ev = json.loads(json.loads(line)["data"])
            c = ev.get("Claim")
            if not c:
                continue
            if "Granted" in c:
                g = c["Granted"]
                b = box(g["kind"])
                if b is None:
                    continue
                boxes.append(b)
                for oid, ob in held.items():
                    if clash(b, ob):
                        double.append([g["id"], oid])
                held[g["id"]] = b
            else:
                for k in ("Released", "Expired", "Preempted"):
                    if k in c:
                        held.pop(c[k]["id"], None)
    outside = [list(c) for c in broken if not any(all(b[0][k] <= v <= b[1][k] for k, v in zip("xyz", c)) for b in boxes)]
    return double, outside


if __name__ == "__main__":
    main()
