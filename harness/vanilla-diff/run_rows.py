#!/usr/bin/env python3
"""Replays rows of a Null run in the vanilla client and diffs them. Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b

  uv run harness/vanilla-diff/run_rows.py --null-tap T.jsonl --input-log DIR/ACCOUNT.jsonl --out DIR [--rows A15:x,A17:y | --module A15]

For each row of rows.json: the replay phase from Null's packets, `run_replay.sh` (scene, acts, mod,
tap), then the differ. Rows with "skip" are listed with their reason and not run. One replay per row.
"""
import argparse
import json
import pathlib
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
sys.path.insert(0, str(HERE / "differ"))
import vd_diff  # noqa: E402


# Spec: specs/plans/VANILLA_DIFF_PLAN.md VD1b
def main():
    p = argparse.ArgumentParser()
    p.add_argument("--null-tap", required=True)
    p.add_argument("--input-log", required=True)
    p.add_argument("--out", required=True)
    p.add_argument("--rows", help="comma-separated row ids (default: all)")
    p.add_argument("--module", help="only rows of this module, e.g. A15")
    p.add_argument("--server", default="127.0.0.1:25567")
    p.add_argument("--diff-only", action="store_true", help="do not replay; diff the logs already in --out (the differ changed)")
    a = p.parse_args()
    rows = json.load(open(HERE / "rows.json"))
    want = a.rows.split(",") if a.rows else list(rows)
    out = pathlib.Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    null_recs = vd_diff.load_tap(a.null_tap)
    samples, marks = vd_diff.load_input(a.input_log)
    for row in want:
        cfg = rows[row]
        if a.module and not row.startswith(a.module + ":"):
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
                                "--row", row, "--scene", cfg["scene"], "--phase", str(phase), "--server", a.server],
                               capture_output=True, text=True)
            (out / f"{stem.name}_replay.out").write_text(r.stdout + r.stderr)
        args = [sys.executable, str(HERE / "differ" / "vd_diff.py"), "--null", a.null_tap, "--vanilla", f"{stem}_vtap.jsonl",
                "--input", a.input_log, "--row", row, "--marks", f"{stem}_marks.json"]
        for name in cfg.get("anchors", []):
            args += ["--anchor", name]
        if "stop_at_anchor" in cfg:
            args += ["--stop-at-anchor", cfg["stop_at_anchor"]]
        if r.returncode != 0 or not pathlib.Path(f"{stem}_marks.json").exists():
            print(f"REPLAY FAILED (exit {r.returncode}); see {stem.name}_replay.out", flush=True)
            continue
        d = subprocess.run(args, capture_output=True, text=True)
        (out / f"{stem.name}_diff.txt").write_text(d.stdout + d.stderr)
        print(d.stdout + d.stderr, flush=True)


if __name__ == "__main__":
    main()
