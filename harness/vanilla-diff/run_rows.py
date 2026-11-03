#!/usr/bin/env python3
"""Replays rows of a Null run in the vanilla client and diffs them. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b, specs/plans/VD3_BRIEF.md T1

  uv run harness/vanilla-diff/run_rows.py --null-tap T.jsonl --input-log DIR/ACCOUNT.jsonl --out DIR [--rows A15:x,A17:y | --module A15]
      [--server 127.0.0.1:25568 --rcon-port 25578 --server-log PATH]   (VD3: Paper + Grim; alerts are compared)

For each row of rows.json: the replay phase from Null's packets, `run_replay.sh` (scene, acts, mod,
tap), then the differ. Rows with "skip" are listed with their reason and not run. One replay per row.
"""
import argparse
import json
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "differ"))
import vd_alerts  # noqa: E402
import vd_diff  # noqa: E402


# Spec: specs/plans/VD3_BRIEF.md T1
def null_player(recs):
    """The account of the Null run: the name in the first Login `Hello` of its tap log."""
    for r in recs:
        if r["dir"] == "S" and r["name"] == "Hello":
            m = re.search(r'name: "([^"]+)"', r["summary"])
            if m:
                return m.group(1)
    raise ValueError("no Hello in the Null tap log")


# Spec: specs/plans/VD3_BRIEF.md T1
def row_alerts(cfg, row, marks, player, stem, server_log):
    """Alert lists of a row from its two windows, and their comparison (README "VD3 formats", `alerts`)."""
    nw = vd_alerts.null_window(marks, row)
    if nw is None:
        return {"error": "the Null input log has no logoff marks for this row (run with the VD3 ability check)"}
    lw = pathlib.Path(f"{stem}_vtap.jsonl.logwin")
    if not lw.exists():
        return {"error": f"{lw.name} missing: the replay did not record its log window"}
    vw = vd_alerts.load_logwin(lw)
    null = vd_alerts.parse_alerts(vd_alerts.read_window(server_log, *nw), player)
    van = vd_alerts.parse_alerts(vd_alerts.read_window(vw["log"], vw["start"], vw["end"]), vw["player"])
    allowed = cfg.get("allowed_alerts", {})
    cmp = vd_alerts.compare_alerts(null, van, allowed)
    return {"null_player": player, "vanilla_player": vw["player"], "null_log": [str(server_log), *nw], "vanilla_log": [vw["log"], vw["start"], vw["end"]],
            "null": null, "vanilla": van, "allowed": allowed, **cmp}


# Spec: specs/plans/VD3_BRIEF.md T1
def summary_line(row, v):
    pk = v.get("packets")
    if pk is None:
        return f"{row}: REPLAY FAILED"
    text = pk["verdict"] if pk["verdict"] == "MATCH" else "DIFF (" + ", ".join(f"{n} {k}" for k, n in sorted(pk["classes"].items())) + ")"
    al = v.get("alerts")
    if al is None:
        tail = ""
    elif "error" in al:
        tail = f"; alerts NOT COMPARED: {al['error']}"
    else:
        tail = "; alerts: " + ("FINDING (Null only) " if al["finding"] else "") + f"Null {vd_alerts.describe_list(al['null'])} | vanilla {vd_alerts.describe_list(al['vanilla'])}"
    return f"{row}: packets {text}{tail}"


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
def main():
    p = argparse.ArgumentParser()
    p.add_argument("--null-tap", required=True)
    p.add_argument("--input-log", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--rows", help="comma-separated row ids (default: all)")
    p.add_argument("--module", help="only rows of these modules, e.g. A15 or A1,A2")
    p.add_argument("--server", default="127.0.0.1:25567")
    p.add_argument("--rcon-port", help="RCON port of the server (the scene process)")
    p.add_argument("--server-log", help="server.log of the server: its Grim alerts are read per row and compared (VD3)")
    p.add_argument("--diff-only", action="store_true", help="do not replay; diff the logs already in --out (the differ changed)")
    a = p.parse_args()
    rows = json.load(open(HERE / "rows.json"))
    want = a.rows.split(",") if a.rows else list(rows)
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    null_recs = vd_diff.load_tap(a.null_tap)
    player = null_player(null_recs) if a.server_log else None
    verdicts = {}
    samples, marks = vd_diff.load_input(a.input_log)
    for row in want:
        cfg = rows[row]
        if a.module and row.split(":")[0] not in a.module.split(","):
            continue
        if "skip" in cfg:
            print(f"== {row}\nNOT COMPARED: {cfg['skip']}", flush=True)
            continue
        start, end = marks[f"start:{row}"], marks[f"end:{row}"]
        phase = vd_diff.phase_at(null_recs, start)
        stem = out / row.replace(":", "_")
        print(f"== {row} (Null ticks [{start}, {end}), phase {phase})", flush=True)
        if a.diff_only:
            r = subprocess.CompletedProcess([], 0, "", "")
        else:
            r = subprocess.run([str(HERE / "run_replay.sh"), "--input", a.input_log, "--tap-out", f"{stem}_vtap.jsonl", "--marks", f"{stem}_marks.json",
                                "--row", row, "--scene", cfg["scene"], "--phase", str(phase), "--server", a.server]
                               + (["--server-log", a.server_log] if a.server_log else []) + (["--rcon-port", a.rcon_port] if a.rcon_port else []),
                               capture_output=True, text=True)
            (out / f"{stem.name}_replay.out").write_text(r.stdout + r.stderr)
        args = [sys.executable, str(HERE / "differ" / "vd_diff.py"), "--null", a.null_tap, "--vanilla", f"{stem}_vtap.jsonl",
                "--input", a.input_log, "--row", row, "--marks", f"{stem}_marks.json", "--json", f"{stem}_packets.json"]
        for name in cfg.get("anchors", []):
            args += ["--anchor", name]
        if "stop_at_anchor" in cfg:
            args += ["--stop-at-anchor", cfg["stop_at_anchor"]]
        if r.returncode != 0 or not pathlib.Path(f"{stem}_marks.json").exists():
            print(f"REPLAY FAILED (exit {r.returncode}); see {stem.name}_replay.out", flush=True)
            verdicts[row] = {"row": row}
            continue
        d = subprocess.run(args, capture_output=True, text=True)
        (out / f"{stem.name}_diff.txt").write_text(d.stdout + d.stderr)
        print(d.stdout + d.stderr, flush=True)
        v = {"row": row, "packets": json.load(open(f"{stem}_packets.json"))}
        if a.server_log:
            v["alerts"] = row_alerts(cfg, row, marks, player, stem, pathlib.Path(a.server_log))
            print(vd_alerts.report(v["alerts"], v["alerts"]["null"], v["alerts"]["vanilla"]) if "error" not in v["alerts"] else "  alerts NOT COMPARED: " + v["alerts"]["error"], flush=True)
        (out / f"{stem.name}_verdict.json").write_text(json.dumps(v, indent=1))
        verdicts[row] = v
    print("\n== SUMMARY (packets, then Grim alerts; allowed alerts are labelled in the row's report above)")
    for row, v in verdicts.items():
        print(summary_line(row, v))
    bad = [r for r, v in verdicts.items() if "packets" not in v or v["packets"]["verdict"] != "MATCH" or v.get("alerts", {}).get("finding") or "error" in v.get("alerts", {})]
    print(f"{len(verdicts) - len(bad)} of {len(verdicts)} rows clean (packets MATCH, no Null-only alert)" if verdicts else "no rows run")
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
