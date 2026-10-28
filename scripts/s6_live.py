#!/usr/bin/env python3
"""S6a live run: two bots collect 64 oak logs through the live spine on the vanilla test server.

Plants 24 oak trees (6 logs each) by RCON in an area no ability-check scenario uses, writes the tree list, starts
`null-node --spine`, sends `set-goal obtain oak_log 64`, waits for the spine to go idle, then reads back by RCON:
each bot's oak_log count, every planted log cell, and the server log. It also checks the operation's event log
(no tree claimed twice at once; every broken cell lies in a claim box that was granted).
During the run a thread polls every planted log cell by RCON about once a second and saves when each cell was first
seen gone (`breaks.json`). Each break is then checked against the hold interval of the box that contains it (a claim
must be held while its box is touched), and the oak_log item entities left in the area are counted (`floor`).
Spec: specs/plans/S6_BRIEF.md (task S6a). Usage: scripts/s6_live.sh [--timeout S] [--out DIR] [--port 25567 --rcon 25577]
"""
import argparse, importlib.util, json, os, signal, socket, subprocess, sys, threading, time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
spec = importlib.util.spec_from_file_location("rcon", ROOT / "harness/skeleton-server/scripts/rcon.py")
rcon = importlib.util.module_from_spec(spec)
spec.loader.exec_module(rcon)

FLOOR_Y = 199          # the harness floor level (scenarios.rs FLOOR_Y)
X0, Z0 = 1200, 700     # no ability-check module uses this area (they sit at z=342, x up to 1090)
COLS, ROWS, SPACING, HEIGHT = 6, 4, 8, 6
BOTS = ["null_s6a1", "null_s6a2"]


POLL_S = 0.25          # poll interval; a full poll of 144 cells takes about 0.05 s
MAP_ERR_S = 0.1        # error of the event-log-to-wall map (one 50 ms spine step plus the RCON round trip)


class Poller(threading.Thread):
    # Spec: specs/plans/S6_BRIEF.md (S6a claim-release fix)
    """Polls every standing planted log cell by RCON (own connection) every POLL_S seconds.

    For each cell it keeps the time of the last poll that saw it standing (`ok`) and of the first poll that saw it
    gone (`gone`), both host wall time when the reply came back. The server runs on this host, so this is the
    server's wall time up to the RCON round trip. The break happened in (ok, gone].
    """

    def __init__(self, cells, port):
        super().__init__(daemon=True)
        self.cells, self.port = cells, port
        self.ok, self.gone, self.polls = {}, {}, []
        self.stop = threading.Event()
        self.error = None

    def run(self):
        try:
            sock = socket.create_connection(("127.0.0.1", self.port))
            sock.settimeout(30)
            sock.sendall(rcon.pkt(1, 3, "skeleton"))
            rcon.rd(sock)
            while True:
                t0 = time.time()
                for c in self.cells:
                    if c in self.gone:
                        continue
                    sock.sendall(rcon.pkt(2, 2, f"execute if block {c[0]} {c[1]} {c[2]} minecraft:oak_log"))
                    out = rcon.rd(sock)[2]
                    t = time.time()
                    if "passed" in out:
                        self.ok[c] = t
                    else:
                        self.gone[c] = t
                self.polls.append((t0, time.time()))
                if self.stop.is_set():
                    break
                time.sleep(max(0.0, POLL_S - (time.time() - t0)))
            sock.close()
        except Exception as e:  # reported in the result, never silent
            self.error = repr(e)


def count_floor(r, area):
    # Spec: specs/plans/S6_BRIEF.md (S6a claim-release fix)
    """oak_log item entities left in the tree area, by stack size (read field by field: Paper cuts long replies)."""
    x0, y0, z0, dx, dy, dz = area
    sel = f"x={x0},y={y0},z={z0},dx={dx},dy={dy},dz={dz}"
    by_n = {}
    outs = r(*[f'execute if entity @e[type=minecraft:item,{sel},nbt={{Item:{{id:"minecraft:oak_log",count:{n}}}}}]' for n in range(1, 65)])
    for n, o in zip(range(1, 65), outs):
        if "count:" in o.lower():  # the server prints "Test passed. Count: N"
            by_n[n] = int(o.lower().rsplit("count:", 1)[1].strip())
    total_entities = r(f'execute if entity @e[type=minecraft:item,{sel}]')[0]
    return {"logs": sum(n * k for n, k in by_n.items()), "stacks_by_size": by_n, "any_item_entities": total_entities}


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
        poller = Poller(cells, a.rcon)
        poller.start()
        time.sleep(2 * POLL_S)  # baseline: every cell seen standing before the goal
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
        t_status = time.time()
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
        if "poller" in locals():
            poller.stop.set()
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

    poller.join(30)
    floor = count_floor(r, (X0, FLOOR_Y, Z0, x1 - X0, 30, z1 - Z0))

    # 5. event log checks
    double, outside, holds = check_events(out, broken)
    breaks = check_breaks(out, holds, poller, (t_goal, t_status))
    floor_logs = floor["logs"]
    breaks["counts"] = {"broken": len(broken), "gathered_by_outcomes": gathered(out), "held_in_inventories": sum(counts.values()),
                        "floor_drops": floor_logs}
    breaks["counts"]["broken_minus_gathered_minus_floor"] = len(broken) - breaks["counts"]["gathered_by_outcomes"] - floor_logs
    (out / "breaks.json").write_text(json.dumps(breaks, indent=2))
    (out / "server-log-slice.txt").write_text("\n".join(log))
    res = {
        "idle": idle, "seconds_to_idle": round(elapsed, 1), "oak_log_by_bot": counts, "oak_log_total": sum(counts.values()),
        "log_cells_planted": len(cells), "log_cells_broken": len(broken), "double_claims": double, "broken_outside_claims": outside,
        "server_log_warnings": warns, "bots_op": bots_op, "out": str(out),
        "polled_breaks": len(breaks["breaks"]), "breaks_outside_hold": breaks["outside_hold"], "breaks_straddling_release": len(breaks["straddling_release"]),
        "poller_error": poller.error, "counts": breaks["counts"], "floor": floor,
        "unexplained_break_difference": breaks["counts"]["broken_minus_gathered_minus_floor"],
    }
    (out / "result.json").write_text(json.dumps(res, indent=2))
    print(json.dumps(res, indent=2))
    ok = idle and sum(counts.values()) >= 64 and not double and not outside and not warns and not bots_op and not breaks["outside_hold"] and not breaks["straddling_release"] and breaks["counts"]["broken_minus_gathered_minus_floor"] == 0 and not poller.error and not set(c for c in broken) - set(breaks_cells(breaks))
    print("S6a done-when:", "PASS" if ok else "FAIL")
    sys.exit(0 if ok else 1)


def check_events(out, broken):  # returns (double claims, broken cells outside every granted box, hold intervals)
    """Two checks on the operation's JSONL log: no overlapping claims held at once; each broken cell is in a granted box."""
    held, double, boxes, holds = {}, [], [], {}

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
                holds[g["id"]] = {"box": b, "from_at": json.loads(line)["at"], "to_at": None, "end": None}
            else:
                for k in ("Released", "Expired", "Preempted"):
                    if k in c:
                        held.pop(c[k]["id"], None)
                        if c[k]["id"] in holds:
                            holds[c[k]["id"]].update(to_at=json.loads(line)["at"], end=k)
    outside = [list(c) for c in broken if not any(all(b[0][k] <= v <= b[1][k] for k, v in zip("xyz", c)) for b in boxes)]
    return double, outside, holds


def gathered(out):
    # Spec: specs/plans/S6_BRIEF.md (S6a claim-release fix)
    """Logs the operation counted: items gained in every outcome of the event log."""
    n = 0
    for line in (out / "op1.jsonl").read_text().splitlines():
        ev = json.loads(json.loads(line)["data"])
        if "Outcome" in ev:
            n += sum(i["n"] for i in ev["Outcome"]["outcome"]["report"]["items_gained"] if i["item"] == "minecraft:oak_log")
    return n


def breaks_cells(breaks):
    return [tuple(b["cell"]) for b in breaks["breaks"]]


def check_breaks(out, holds, poller, anchors):
    # Spec: specs/plans/S6_BRIEF.md (S6a claim-release fix)
    """Each polled break against the hold interval of the box containing it.

    The event log's `at` is node time (ms, 50 ms per spine step). It maps to wall time through two anchors: the
    audit's `at` for `set-goal` and for `status` against the host wall time at which the script wrote each line.
    The line reaches the spine within one 50 ms step; a linear map between the two anchors also absorbs any drift.
    A break lies in (ok, gone] (the last poll that saw the cell and the first that did not). It fits a hold only when
    the whole window lies inside [grant, end], widened by the map error alone: an unproven break fails the run.
    A window that crosses the release is also listed as straddling (the break may have come after it).
    """
    audit = [json.loads(l) for l in (out / "audit.jsonl").read_text().splitlines()]
    at = [e["at"] for e in audit if "SetGoal" in e["command"] or e["command"] == "Status"]
    (w1, w2), (a1, a2) = anchors, (at[0], at[-1])
    k = (w2 - w1) / (a2 - a1)
    wall = lambda t: w1 + (t - a1) * k
    res = {"mapping": {"wall_goal_sent": w1, "wall_status_sent": w2, "at_goal": a1, "at_status": a2, "wall_s_per_at_ms": k},
           "poll_interval_s": POLL_S, "polls": len(poller.polls),
           "longest_poll_s": round(max((b - a for a, b in poller.polls), default=0), 3), "breaks": [], "outside_hold": [], "straddling_release": []}
    inf = float("inf")
    for c, gone in sorted(poller.gone.items(), key=lambda kv: kv[1]):
        ok = poller.ok.get(c)
        owner = [(i, h) for i, h in holds.items() if all(h["box"][0][d] <= v <= h["box"][1][d] for d, v in zip("xyz", c))]
        row = {"cell": list(c), "ok_wall": ok, "gone_wall": gone, "boxes": []}
        fits = False
        for i, h in owner:
            g, e = wall(h["from_at"]), (wall(h["to_at"]) if h["to_at"] is not None else inf)
            lo = ok if ok is not None else -inf  # never seen standing: unproven
            inside = lo >= g - MAP_ERR_S and gone <= e + MAP_ERR_S
            straddle = lo <= e + MAP_ERR_S and gone > e + MAP_ERR_S
            row["boxes"].append({"claim": i, "grant_wall": g, "end_wall": None if e == inf else e, "end": h["end"],
                                 "gone_minus_end_s": None if e == inf else round(gone - e, 3), "ok_minus_end_s": None if e == inf else (None if ok is None else round(ok - e, 3))})
            fits |= inside
            if straddle:
                res["straddling_release"].append(row)
        if not fits:
            res["outside_hold"].append(row)
        res["breaks"].append(row)
    res["cells_still_standing"] = sum(1 for c in poller.ok if c not in poller.gone)
    return res


if __name__ == "__main__":
    main()
