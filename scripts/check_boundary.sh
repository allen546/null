#!/usr/bin/env bash
# Enforces the version boundary (F5 §4.1) until cargo-deny is installed.
# (a) decision crates build on stable; (b) only mechanism crates reach bevy*/azalea*.
set -euo pipefail
cd "$(dirname "$0")/.."

# vd-tap: harness tool (specs/plans/VANILLA_DIFF_PLAN.md VD0), a packet proxy that never runs in a bot.
ALLOW="mc-adapter null-abilitycheck null-duel null-node vd-tap"

meta=$(cargo metadata --format-version 1 --locked 2>/dev/null || cargo metadata --format-version 1)

# Workspace members not on the allowlist are the decision crates.
decision=$(ALLOW="$ALLOW" python3 -I -c '
import json, os, sys
m = json.load(sys.stdin)
allow = set(os.environ["ALLOW"].split())
ids = set(m["workspace_members"])
print(" ".join(p["name"] for p in m["packages"] if p["id"] in ids and p["name"] not in allow))
' <<<"$meta")

echo "decision crates: ${decision:-none}"
args=()
for c in $decision; do args+=(-p "$c"); done
if [ ${#args[@]} -gt 0 ]; then
  cargo +stable build "${args[@]}"
fi

ALLOW="$ALLOW" python3 -I -c '
import json, os, sys
m = json.load(sys.stdin)
allow = set(os.environ["ALLOW"].split())
name = {p["id"]: p["name"] for p in m["packages"]}
deps = {n["id"]: [d["pkg"] for d in n["deps"]] for n in m["resolve"]["nodes"]}
bad = False
for wid in m["workspace_members"]:
    if name[wid] in allow:
        continue
    seen, stack, via = set(), [wid], {wid: None}
    while stack:
        cur = stack.pop()
        for d in deps.get(cur, []):
            if d in seen:
                continue
            seen.add(d); via[d] = cur; stack.append(d)
            if name[d].startswith(("bevy", "azalea")):
                path, x = [], d
                while x is not None:
                    path.append(name[x]); x = via[x]
                print("BOUNDARY VIOLATION: " + " -> ".join(reversed(path)))
                bad = True
sys.exit(1 if bad else 0)
' <<<"$meta"
echo "boundary ok"
