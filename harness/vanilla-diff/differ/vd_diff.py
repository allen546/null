#!/usr/bin/env python3
"""Vanilla-diff differ (VD1). Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1.

Reads the two tap logs (JSON lines from `vd-tap`), the Null input log (fork patch 45) and the
vanilla replay marks, lines the row window up by client tick, and compares the serverbound game
packets tick by tick. It reports the first difference with context.

  uv run harness/vanilla-diff/differ/vd_diff.py --null tap.jsonl --vanilla vtap.jsonl \
      --input null_check.jsonl --row A15:plain_stone_walk_matches_the_model --marks marks.json
  (no markers: --null-window START,END --vanilla-offset N; ticks END exclusive)
"""
import argparse
import json
import sys

# Serverbound packets that depend on timers or on the server's schedule, not on the input. Each
# names the vanilla method that sends it. ClientTickEnd is not ignored for the match: it is checked
# as "one per tick" and then left out of the lists.
IGNORE = {
    "KeepAlive": "ClientCommonPacketListenerImpl#handleKeepAlive (answers the server's timer)",
    "Pong": "ClientCommonPacketListenerImpl#handlePing (answers a server ping)",
    "ClientInformation": "ClientPacketListener#handleLogin / Options#broadcastOptions (on join or an options change)",
    "ChunkBatchReceived": "ClientPacketListener#handleChunkBatchFinished (answers a chunk batch)",
}
# Clientbound packets that arrive on the server's timer; they do not make a difference "server timing".
IGNORE_CLIENTBOUND = {"KeepAlive", "Ping", "SetTime", "ChunkBatchStart", "ChunkBatchFinished", "BundleDelimiter"}


def load_tap(path):
    out = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def by_tick(records):
    """{tick: (serverbound [(name, summary)], clientbound [name], tick_ends)} for the Game state."""
    ticks = {}
    for r in records:
        if r.get("state") != "Game":
            continue
        s, c, e = ticks.setdefault(r["tick"], ([], [], [0]))
        if r["dir"] == "S":
            if r["name"] == "ClientTickEnd":
                e[0] += 1
            elif r["name"] not in IGNORE:
                s.append((r["name"], r["summary"]))
        elif r["name"] not in IGNORE_CLIENTBOUND:
            c.append(r["name"])
    return ticks


def load_input(path):
    """(samples {tick: line dict}, marks {label: tick})."""
    samples, marks = {}, {}
    with open(path) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            d = json.loads(line)
            if "mark" in d:
                marks[d["mark"]] = d["tick"]
            else:
                samples[d["tick"]] = d
    return samples, marks


def describe(sample):
    if sample is None:
        return "-"
    keys = "".join(k.upper() for k in ("f", "b", "l", "r") if sample[k])
    flags = "".join(n for n, k in (("J", "jump"), ("S", "sneak"), ("R", "sprint")) if sample[k])
    return f"{keys or '.'}{'+' + flags if flags else ''} yaw {sample['yaw']} pitch {sample['pitch']} slot {sample['slot']}"


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def compare(null, vanilla, window, offset, samples=None):
    """Compares Null ticks [a, b) with vanilla ticks shifted by `offset`.

    Returns (diffs, ticks_compared, tick_end_problems). Each diff is a dict with the Null tick,
    both packet lists, the clientbound names that arrived in each run and its label.
    """
    a, b = window
    diffs, problems = [], []
    first_server_diff = None
    for t in range(a, b):
        ns, nc, ne = null.get(t, ([], [], [0]))
        vs, vc, ve = vanilla.get(t + offset, ([], [], [0]))
        if ne[0] != 1 or ve[0] != 1:
            problems.append((t, ne[0], ve[0]))
        if first_server_diff is None and sorted(nc) != sorted(vc):
            first_server_diff = t
        if ns != vs:
            # A difference at or after the first different server arrival may follow from it.
            timing = first_server_diff is not None and first_server_diff <= t
            diffs.append({
                "tick": t, "vanilla_tick": t + offset, "null": ns, "vanilla": vs, "null_clientbound": nc,
                "vanilla_clientbound": vc, "label": "server timing" if timing else "client",
                "first_server_diff": first_server_diff,
                "inputs": [(u, describe(samples.get(u) if samples else None)) for u in range(t - 2, t + 3)],
            })
    return diffs, b - a, problems


def report(diffs, n, problems, show_all=False):
    lines = []
    for t, nn, vv in problems:
        lines.append(f"WARNING tick {t}: ClientTickEnd count Null {nn}, vanilla {vv} (want 1 each; the logs are out of step)")
    if not diffs:
        lines.append(f"MATCH: {n} ticks compared, serverbound packets identical (ignored: {', '.join(sorted(IGNORE))})")
        return "\n".join(lines)
    lines.append(f"DIFF: {len(diffs)} of {n} ticks differ")
    for d in (diffs if show_all else diffs[:1]):
        lines.append(f"tick {d['tick']} (vanilla tick {d['vanilla_tick']}): {d['label']}")
        lines.append("  Null:    " + (" | ".join(f"{x} {y}" for x, y in d["null"]) or "(nothing)"))
        lines.append("  vanilla: " + (" | ".join(f"{x} {y}" for x, y in d["vanilla"]) or "(nothing)"))
        lines.append(f"  clientbound this tick: Null {d['null_clientbound']}, vanilla {d['vanilla_clientbound']}")
        if d["first_server_diff"] is not None:
            lines.append(f"  first different server arrival: tick {d['first_server_diff']}")
        lines.append("  inputs: " + "; ".join(f"{u}: {s}" for u, s in d["inputs"]))
    if not show_all and len(diffs) > 1:
        lines.append(f"  (and {len(diffs) - 1} more ticks; --all lists them)")
    return "\n".join(lines)


def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--null", required=True, help="tap log of the Null run")
    p.add_argument("--vanilla", required=True, help="tap log of the vanilla replay")
    p.add_argument("--input", help="Null input log (fork patch 45)")
    p.add_argument("--row", help="row label in the input log, e.g. A15:plain_stone_walk_matches_the_model")
    p.add_argument("--marks", help="the replay's marks file (vd.marks)")
    p.add_argument("--null-window", help="START,END (END exclusive) when there are no marks")
    p.add_argument("--vanilla-offset", type=int, help="vanilla tick minus Null tick, when there are no marks")
    p.add_argument("--all", action="store_true", help="list every differing tick")
    a = p.parse_args(argv)
    samples = None
    if a.input:
        samples, marks = load_input(a.input)
    if a.null_window:
        s, e = (int(x) for x in a.null_window.split(","))
    elif a.input and a.row:
        s, e = marks[f"start:{a.row}"], marks[f"end:{a.row}"]
    else:
        p.error("give --input with --row, or --null-window")
    if a.vanilla_offset is not None:
        off = a.vanilla_offset
    elif a.marks:
        m = json.load(open(a.marks))
        off = m["replay_start_tick"] - m["first_input_tick"]
    else:
        p.error("give --marks or --vanilla-offset")
    diffs, n, problems = compare(by_tick(load_tap(a.null)), by_tick(load_tap(a.vanilla)), (s, e), off, samples)
    print(f"window: Null ticks [{s}, {e}), vanilla ticks [{s + off}, {e + off})")
    print(report(diffs, n, problems, a.all))
    return 1 if diffs or problems else 0


if __name__ == "__main__":
    sys.exit(main())
