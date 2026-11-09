"""Grim alert lists of one row, from the row's own window in server.log, and their comparison.
Spec: specs/plans/VD3_BRIEF.md T1. Formats: harness/vanilla-diff/README.md "VD3 formats".

A window is (log path, start byte, end byte). Null's comes from the `logoff:` marks of its input
log (written by `Ctx::run_allowing`); the vanilla replay's comes from `run_replay.sh` (`.logwin`).
"""
import json
import re

ANSI = re.compile(r"\x1b\[[0-9;]*[A-Za-z]")


# Spec: specs/plans/VD3_BRIEF.md T1
def parse_alerts(text, player):
    """{check: lines} for the Grim alert lines of `player` in `text` (a log slice).

    A line is "Grim » <player> failed <Check> (x3) ...": colour codes are stripped, the player must
    be a whole word (`VD1` is not `VD12`), and each line counts once as the ability check does
    (`scan_log` in scenarios.rs); the (xN) violation count is not used."""
    out = {}
    for line in ANSI.sub("", text).splitlines():
        if "Grim" not in line or " failed " not in line:
            continue
        head, rest = line.split(" failed ", 1)
        if not re.search(r"(?<![\w])" + re.escape(player) + r"(?![\w])", head):
            continue
        check = rest.split()[0] if rest.split() else "?"
        out[check] = out.get(check, 0) + 1
    return out


# Spec: specs/plans/VD3_BRIEF.md T1
def read_window(path, start, end):
    """The text of server.log bytes [start, end). Raises if the log is shorter than `end` (it was
    replaced since the run)."""
    with open(path, "rb") as f:
        f.seek(0, 2)
        if f.tell() < end:
            raise ValueError(f"{path} has {f.tell()} bytes, the window ends at {end}: the log was replaced")
        f.seek(start)
        return f.read(end - start).decode("utf-8", "replace")


# Spec: specs/plans/VD3_BRIEF.md T1
def null_window(marks, row):
    """(start, end) log bytes of `row` from the `logoff:start:ROW:N` and `logoff:end:ROW:N` marks, or None."""
    got = {}
    for kind in ("start", "end"):
        pre = f"logoff:{kind}:{row}:"
        for label in marks:
            if label.startswith(pre) and label[len(pre):].isdigit():
                got[kind] = int(label[len(pre):])
    return (got["start"], got["end"]) if len(got) == 2 else None


# Spec: specs/plans/VD3_BRIEF.md T1
def compare_alerts(null, vanilla, allowed=None):
    """Classifies the checks of both lists. `allowed` is {check prefix: reason} of the row.

    Each check gets: `null_only` (a finding, unless allowed), `shared` (both drew it, recorded),
    or `vanilla_only` (reported). A check the row allows carries `allowed` with the reason: it is
    labelled, never dropped. `finding` is true when an alert is on Null only and not allowed."""
    allowed = allowed or {}
    checks = {}
    for c in sorted(set(null) | set(vanilla)):
        kind = "shared" if c in null and c in vanilla else "null_only" if c in null else "vanilla_only"
        why = next((w for p, w in allowed.items() if c.startswith(p)), None)
        checks[c] = {"class": kind, "null": null.get(c, 0), "vanilla": vanilla.get(c, 0), **({"allowed": why} if why else {})}
    # VD3 T2, additive: alert lines of checks only the vanilla client drew (identical packets with these point at the replay).
    van_only = sum(v["vanilla"] for v in checks.values() if v["class"] == "vanilla_only")
    return {"checks": checks, "finding": any(v["class"] == "null_only" and "allowed" not in v for v in checks.values()), "vanilla_only_alerts": van_only}


# Spec: specs/plans/VD3_BRIEF.md T1
def describe_list(alerts):
    return ", ".join(f"{c} x{n}" for c, n in sorted(alerts.items())) or "none"


# Spec: specs/plans/VD3_BRIEF.md T1
def report(cmp, null, vanilla):
    lines = [f"  Grim alerts, Null:    {describe_list(null)}", f"  Grim alerts, vanilla: {describe_list(vanilla)}"]
    for c, v in cmp["checks"].items():
        label = {"null_only": "FINDING (Null only)", "shared": "shared (both drew it)", "vanilla_only": "vanilla only"}[v["class"]]
        if "allowed" in v:
            label = f"ALLOWED by the row ({v['allowed']}), {label}"
        lines.append(f"  alert {c}: {label} (Null {v['null']}, vanilla {v['vanilla']})")
    return "\n".join(lines)


# Spec: specs/plans/VD3_BRIEF.md T1
def load_logwin(path):
    return json.load(open(path))
