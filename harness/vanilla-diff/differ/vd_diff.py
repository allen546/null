#!/usr/bin/env python3
"""Vanilla-diff differ (VD1). Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1.

Reads the two tap logs (JSON lines from `vd-tap`), the Null input log (fork patch 45) and the
vanilla replay marks, lines the row window up by client tick, and compares the serverbound game
packets tick by tick. It reports the first difference with context.

  uv run harness/vanilla-diff/differ/vd_diff.py --null tap.jsonl --vanilla vtap.jsonl \
      --input null_check.jsonl --row A15:plain_stone_walk_matches_the_model --marks marks.json
  (no markers: --null-window START,END --vanilla-offset N; ticks END exclusive)

  --anchor NAME (repeatable): a clientbound packet that the row's RCON action causes (e.g.
  UpdateMobEffect). The k-th arrival of the anchors in the Null window is matched with the k-th in
  the vanilla run, and each stretch between anchors is compared from its own anchor, so a server
  reply that lands a tick later in one run does not shift everything after it. The shift is printed.

  --phase: print the value for the replay mod's -Dvd.phase (ticks since Null's last position
  reminder reset, at the first window tick) and exit.
"""
import argparse
import json
import re
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


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def load_tap(path):
    out = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


POS = re.compile(r"x: ([-0-9.eE]+), y: ([-0-9.eE]+), z: ([-0-9.eE]+)")


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
def normalize(records):
    """Game-state records with the values that differ between two runs for no reason made equal.

    - The player's own entity id in `PlayerCommand` (the server hands out ids in order): `self`.
    - The id in `AcceptTeleportation` (the server's count of teleports for that player) is
      `ClientPacketListener#handleMovePlayer` echoing the id of the last `PlayerPosition`; it is
      replaced by a word when it does echo it and kept when it does not.
    - A `MovePlayerPos` that repeats the position `LocalPlayer#sendPosition` sent last is the
      20-tick reminder (nothing moved): renamed `MovePlayerPos[reminder]`, so the differ can set
      it apart. The `PosRot` that answers a teleport is not from `sendPosition` and not counted.
    """
    out, last_tp, last_pos, accepted = [], None, None, False
    for r in records:
        if r.get("state") != "Game":
            continue
        r = dict(r)
        if r["dir"] == "C" and r["name"] == "PlayerPosition":
            m = re.search(r"id: (\d+)", r["summary"])
            last_tp = m.group(1) if m else None
        elif r["dir"] == "S":
            if r["name"] == "AcceptTeleportation":
                m = re.search(r"id: (\d+)", r["summary"])
                r["summary"] = "(echoes the last PlayerPosition id)" if m and m.group(1) == last_tp else r["summary"] + " (does NOT echo the last PlayerPosition id)"
                accepted = True
            elif r["name"] == "PlayerCommand":
                r["summary"] = re.sub(r"MinecraftEntityId\(\d+\)", "MinecraftEntityId(self)", r["summary"])
            elif r["name"] in ("MovePlayerPos", "MovePlayerPosRot"):
                m = POS.search(r["summary"])
                pos = m.groups() if m else None
                if accepted:
                    accepted = False  # the answer to a teleport
                else:
                    if r["name"] == "MovePlayerPos" and pos is not None and pos == last_pos:
                        r["name"] = "MovePlayerPos[reminder]"
                    last_pos = pos
        out.append(r)
    return out


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def by_tick(records):
    """{tick: (serverbound [(name, summary)], clientbound [name], tick_ends)} for the Game state."""
    ticks = {}
    for r in normalize(records):
        s, c, e = ticks.setdefault(r["tick"], ([], [], [0]))
        if r["dir"] == "S":
            if r["name"] == "ClientTickEnd":
                e[0] += 1
            elif r["name"] not in IGNORE:
                s.append((r["name"], r["summary"]))
        elif r["name"] not in IGNORE_CLIENTBOUND:
            c.append(r["name"])
    return ticks


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
def phase_at(records, start):
    """Value of vanilla's position-reminder counter (`LocalPlayer#positionReminder`) at the head of
    client tick `start`, read from Null's packets: it is reset to 0 whenever `sendPosition` sends a
    Pos or PosRot packet. The PosRot that answers a teleport (`ClientPacketListener#handleMovePlayer`,
    sent right after AcceptTeleportation) is not from `sendPosition`, so it does not reset it."""
    last = None
    accepted = False
    for r in records:
        if r.get("state") != "Game" or r["dir"] != "S" or r["tick"] >= start:
            continue
        if r["name"] == "AcceptTeleportation":
            accepted = True
        elif r["name"] in ("MovePlayerPos", "MovePlayerPosRot"):
            if accepted:
                accepted = False
            else:
                last = r["tick"]
    if last is None:
        raise ValueError("no position packet before the window")
    return start - 1 - last


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


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
# A packet that reaches the tap this close (ms) before the ClientTickEnd of its tick comes while
# the client is still inside that tick's body. Vanilla handles queued packets once per frame,
# before its ticks (`Minecraft#runTick`: `packetProcessor.processQueuedPackets`, `runAllTasks`),
# so such a packet takes effect in the next tick. Measured in VD1b: arrivals 0, 1, 1, 4 and 5 ms
# before the tick end were handled a tick later (one 5 ms case in Null's bot loop was seen in the
# tap as the answer in the next tick); the next closest arrivals were 19 and more ms before.
LATE_MS = 15


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def anchor_events(records, names, a, b):
    """[(tick, name, note)] of the first arrival of each anchor name per tick, label ticks in [a, b).

    The tick is the one the client handles the packet in: the label, or the next when the packet
    came within LATE_MS of the tick's end (then `note` says so)."""
    ends = {r["tick"]: r["t_ms"] for r in records if r.get("state") == "Game" and r["dir"] == "S" and r["name"] == "ClientTickEnd" and "t_ms" in r}
    seen, out = set(), []
    for r in records:
        if r.get("state") == "Game" and r["dir"] == "C" and r["name"] in names and a <= r["tick"] < b and (r["tick"], r["name"]) not in seen:
            seen.add((r["tick"], r["name"]))
            late = r["tick"] in ends and "t_ms" in r and 0 <= ends[r["tick"]] - r["t_ms"] < LATE_MS
            out.append((r["tick"] + (1 if late else 0), r["name"], f"arrived {ends[r['tick']] - r['t_ms']} ms before tick {r['tick']} ended, taken as tick {r['tick'] + 1}" if late else ""))
    return sorted(out)


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def _no_reminders(packets):
    return [p for p in packets if not p[0].endswith("[reminder]")]


SLACK = 40  # ticks past the shifted window in which a late vanilla anchor is still looked for


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
def compare_anchored(null_recs, vanilla_recs, window, offset, anchors, samples=None):
    """As `compare`, with each stretch between anchors compared from its own anchor.

    Returns (diffs, ticks_compared, tick_end_problems, notes). `notes` lists the anchors with the
    tick each run saw them at, and the ticks left out because one run's stretch was shorter.
    """
    null, vanilla = by_tick(null_recs), by_tick(vanilla_recs)
    a, b = window
    names = set(anchors)
    ne = anchor_events(null_recs, names, a, b)
    ve = anchor_events(vanilla_recs, names, a + offset, b + offset + SLACK)
    notes, pairs = [], []
    for i, (t, name, why) in enumerate(ne):
        if i < len(ve) and ve[i][1] == name:
            pairs.append((t, ve[i][0]))
            extra = "".join(f"; {who}: {w}" for who, w in (("Null", why), ("vanilla", ve[i][2])) if w)
            notes.append(f"anchor {name}: Null tick {t}, vanilla tick {ve[i][0]} (shift {ve[i][0] - t - offset:+d} against the start offset{extra})")
        else:
            got = f"{ve[i][1]} at {ve[i][0]}" if i < len(ve) else "nothing"
            notes.append(f"anchor {name} at Null tick {t} has no vanilla match (vanilla has {got}); the rest is compared with the last offset")
            break
    starts = [(a, a + offset)] + pairs
    diffs, total, problems, skipped, dropped = [], 0, [], 0, 0
    for k, (s, v) in enumerate(starts):
        e = starts[k + 1][0] if k + 1 < len(starts) else b
        ve_ = starts[k + 1][1] if k + 1 < len(starts) else None
        length = e - s if ve_ is None else min(e - s, ve_ - v)
        skipped += (e - s) - length
        if length <= 0:
            continue
        seg_null, seg_van = null, vanilla
        if v - s != offset:
            # The reminder counts client ticks, not server replies: after a shifted anchor its
            # tick is not comparable, so reminders are left out of this stretch (counted below).
            seg_null, seg_van = ({t: (_no_reminders(x[0]), x[1], x[2]) for t, x in tk.items()} for tk in (null, vanilla))
            dropped += sum(1 for t in range(s, s + length) for pk in null.get(t, ([],))[0] if pk[0].endswith("[reminder]"))
        d_, n_, p_ = compare(seg_null, seg_van, (s, s + length), v - s, samples)
        diffs += d_
        total += n_
        problems += p_
    if dropped:
        notes.append(f"{dropped} Null reminder packet(s) not compared in stretches shifted by an anchor (the reminder follows the client's ticks, not the server's reply)")
    if skipped:
        notes.append(f"{skipped} Null tick(s) not compared: the vanilla stretch to the next anchor was shorter")
    return diffs, total, problems, notes


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


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1
def main(argv=None):
    p = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    p.add_argument("--null", required=True, help="tap log of the Null run")
    p.add_argument("--vanilla", required=True, help="tap log of the vanilla replay")
    p.add_argument("--input", help="Null input log (fork patch 45)")
    p.add_argument("--row", help="row label in the input log, e.g. A15:plain_stone_walk_matches_the_model")
    p.add_argument("--marks", help="the replay's marks file (vd.marks)")
    p.add_argument("--null-window", help="START,END (END exclusive) when there are no marks")
    p.add_argument("--vanilla-offset", type=int, help="vanilla tick minus Null tick, when there are no marks")
    p.add_argument("--anchor", action="append", default=[], help="clientbound packet name that the row's RCON action causes (repeatable)")
    p.add_argument("--stop-at-anchor", help="end the window at the first Null arrival of this clientbound packet (rows that diverge by chance at it)")
    p.add_argument("--phase", action="store_true", help="print the -Dvd.phase value for the window start and exit")
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
    if a.phase:
        print(phase_at(load_tap(a.null), s))
        return 0
    cut = None
    if a.stop_at_anchor:
        hits = anchor_events(load_tap(a.null), {a.stop_at_anchor}, s, e)
        if hits:
            cut = (e, hits[0])
            e = hits[0][0]
    if a.vanilla_offset is not None:
        off = a.vanilla_offset
    elif a.marks:
        m = json.load(open(a.marks))
        off = m["replay_start_tick"] - m["first_input_tick"]
    else:
        p.error("give --marks or --vanilla-offset")
    notes = []
    if a.anchor:
        diffs, n, problems, notes = compare_anchored(load_tap(a.null), load_tap(a.vanilla), (s, e), off, a.anchor, samples)
    else:
        diffs, n, problems = compare(by_tick(load_tap(a.null)), by_tick(load_tap(a.vanilla)), (s, e), off, samples)
    print(f"window: Null ticks [{s}, {e}), vanilla ticks [{s + off}, {e + off})")
    if cut:
        print(f"window cut before the first {a.stop_at_anchor} (Null tick {cut[1][0]}); the row's end was tick {cut[0]}")
    for note in notes:
        print(note)
    print(report(diffs, n, problems, a.all))
    return 1 if diffs or problems else 0


if __name__ == "__main__":
    sys.exit(main())
