"""Maps changed paths to ability-check modules. Spec: specs/plans/ABILITY_CHECK.md §Which modules a change reruns"""
import argparse, fnmatch, re, subprocess, sys, tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAP = ROOT / "crates/null-abilitycheck/run_map.toml"
MODULES = "A1 A2 A3 A4 A2S A4S A4C A5 A6 A7 A8 A9 A10 A10E A11 A12 A13 A14 A15 A16 A17 A18 A19 A20 A21 A22 A23 AT".split()


def git(repo, *args):
    r = subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)
    if r.returncode:
        sys.exit(f"git {' '.join(args)} in {repo} failed: {r.stderr.strip()}")
    return [l for l in r.stdout.splitlines() if l]


def pinned_rev():
    m = re.search(r'^azalea\s*=\s*\{[^}]*rev\s*=\s*"([0-9a-f]+)"', (ROOT / "Cargo.toml").read_text(), re.M)
    if not m:
        sys.exit("no azalea rev in Cargo.toml")
    return m.group(1)


def load_map():
    cfg = tomllib.loads(MAP.read_text())
    for e in cfg["entry"]:
        bad = [m for m in e.get("modules", []) if m not in MODULES]
        if bad:
            sys.exit(f"{MAP}: entry {e['path']} names unknown module(s) {bad}")
    return cfg


def ignored(path, patterns):
    for p in patterns:
        if p.endswith("/") and path.startswith(p):
            return True
        if not p.endswith("/") and fnmatch.fnmatch(path, p):
            return True
    return False


def classify(path, cfg):
    """(kind, modules, entry): kind is ignore, all, set or unmapped."""
    if ignored(path, cfg.get("ignore", [])):
        return "ignore", set(), None
    best = None
    for e in cfg["entry"]:
        if path.startswith(e["path"]) and (best is None or len(e["path"]) > len(best["path"])):
            best = e
    if best is None:
        return "unmapped", set(), None
    if best.get("all"):
        return "all", set(MODULES), best
    return "set", set(best.get("modules", [])), best


def affected(paths, cfg):
    """Union of modules; `lines` explains each path."""
    mods, lines, unmapped, is_all = set(), [], [], False
    for p in paths:
        kind, ms, e = classify(p, cfg)
        if kind == "unmapped":
            unmapped.append(p)
            lines.append(f"  {p}: NOT IN THE MAP -> all")
        elif kind == "ignore":
            lines.append(f"  {p}: ignored (no effect on a run)")
        elif kind == "all":
            is_all = True
            lines.append(f"  {p}: all ({e['path']}: {e['why']})")
        else:
            lines.append(f"  {p}: {','.join(sorted(ms, key=MODULES.index)) or 'none'} ({e['path']})")
        mods |= ms
    return mods, is_all or bool(unmapped), unmapped, lines


def tree_changes(repo, base, prefix):
    names = set(git(repo, "diff", "--name-only", base))
    names |= set(git(repo, "ls-files", "--others", "--exclude-standard"))
    return sorted(prefix + n for n in names)


def check_history(cfg, fork):
    bad = 0
    for h in cfg.get("history", []):
        n = h["patch"]
        subs = git(fork, "log", "--format=%H %s", "--grep", f"^null patch: {n}-", "-n", "1")
        if not subs:
            print(f"patch {n}: no commit in {fork}", file=sys.stderr)
            bad += 1
            continue
        sha = subs[0].split()[0]
        files = ["azalea:" + f for f in git(fork, "diff-tree", "--no-commit-id", "--name-only", "-r", sha)]
        mods, is_all, unmapped, _ = affected(files, cfg)
        missing = [m for m in h["modules"] if m not in mods]
        shown = "all" if len(mods) == len(MODULES) else ",".join(sorted(mods, key=MODULES.index))
        print(f"patch {n}: {len(files)} files -> {shown}; tests named {','.join(h['modules'])}: {'MISS ' + ','.join(missing) if missing else 'ok'}"
              + (f"; unmapped {unmapped}" if unmapped else ""))
        bad += bool(missing)
    return bad


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--fork-rev")
    ap.add_argument("--fork", default=str(Path.home() / "Desktop/null-spikes/azalea"))
    ap.add_argument("--explain", action="store_true", help="also print how each path was matched (stderr)")
    ap.add_argument("--check-history", action="store_true")
    a = ap.parse_args()
    cfg = load_map()
    fork = Path(a.fork)
    if a.check_history:
        sys.exit(1 if check_history(cfg, fork) else 0)
    rev = a.fork_rev or pinned_rev()
    paths = tree_changes(ROOT, "HEAD", "")
    if fork.is_dir():
        paths += tree_changes(fork, rev, "azalea:")
    else:
        print(f"fork {fork} not found: its changes are not checked", file=sys.stderr)
    mods, is_all, unmapped, lines = affected(paths, cfg)
    if unmapped:
        print("paths with no entry in run_map.toml (so: all): " + ", ".join(unmapped), file=sys.stderr)
    if a.explain or unmapped:
        print("\n".join(lines), file=sys.stderr)
    if is_all or len(mods) == len(MODULES):
        print("--scenario all")
    elif not mods:
        print("--scenario none")
    else:
        print("--scenario " + ",".join(sorted(mods, key=MODULES.index)))


main()
