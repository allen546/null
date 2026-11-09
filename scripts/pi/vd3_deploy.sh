#!/usr/bin/env bash
# Runs on the Mac: cross-compiles null-abilitycheck and vd-tap, copies them with the vanilla-diff harness and the client's mod to the Pi.
# The Pi's bin/ stays as it is, so a pair is not affected. Spec: specs/plans/VD3_BRIEF.md
# Usage: scripts/pi/vd3_deploy.sh. PI_HOST as in deploy.sh. The client lives in vd-client on the Pi: HeadlessMC fetched it there, because
# the link is too slow for its 400 MB (20-60 KB/s over Tailscale, 2026-11-04). Its run_client.sh is the Pi's own copy, so it is not sent.
set -euo pipefail
export COPYFILE_DISABLE=1  # no macOS ._ files in the tars
PI=${PI_HOST:-192.168.1.2}
H=/mnt/pi-data/null-harness
cd "$(dirname "$0")/../.."
T=aarch64-unknown-linux-gnu
cargo zigbuild --release -p null-abilitycheck -p vd-tap --target $T.2.36
ssh -n "$PI" "mkdir -p $H/vd/target/release $H/vd/harness $H/vd-logs; ln -sfn $H/grim-server $H/vd/harness/grim-server; ln -sfn $H/skeleton-server $H/vd/harness/skeleton-server"
tar -C target/$T/release -cf - null-abilitycheck vd-tap | zstd -q -3 | ssh "$PI" "zstd -dc | tar -C $H/vd/target/release -xf -"
tar --no-xattrs -C harness -cf - --exclude __pycache__ --exclude vanilla-diff/mod --exclude vanilla-diff/tap --exclude vanilla-diff/inputs vanilla-diff | ssh "$PI" "tar -C $H/vd/harness -xf -"
scp -q scripts/pi/vd3_leg.sh scripts/pi/vd3_chain.sh "$PI":$H/vd/
ssh -n "$PI" "chmod +x $H/vd/vd3_leg.sh $H/vd/vd3_chain.sh"
scp -q "$HOME/Desktop/null-spikes/vanilla-diff-client/game/mods/null-vanilla-diff.jar" "$PI":$H/vd-client/game/mods/
echo "$(git rev-parse --short HEAD) plus working tree, $(date '+%F %T')" | ssh "$PI" "cat > $H/vd/DEPLOYED"
for f in null-abilitycheck vd-tap; do
  [ "$(ssh -n "$PI" "sha256sum $H/vd/target/release/$f" | cut -d' ' -f1)" = "$(shasum -a 256 target/$T/release/$f | cut -d' ' -f1)" ] || { echo "CHECKSUM MISMATCH $f" >&2; exit 1; }
done
echo "deployed, checksums ok"
