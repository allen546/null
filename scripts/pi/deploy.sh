#!/usr/bin/env bash
# Runs on the Mac: cross-compiles null-abilitycheck for the Pi (ARM64, glibc 2.36), copies it with the Pi scripts,
# and checks the checksum. Keeps the Pi's previous binary as bin/null-abilitycheck.prev.
# Usage: scripts/pi/deploy.sh. Do not run during a Grim run on this machine. Spec: specs/plans/ABILITY_CHECK.md §How to run
set -euo pipefail
PI=${PI_HOST:-100.95.33.78}  # override while Tailscale is down, e.g. PI_HOST=192.168.1.2
cd "$(dirname "$0")/../.."
T=aarch64-unknown-linux-gnu
cargo zigbuild --release -p null-abilitycheck --target $T.2.36
bin=target/$T/release/null-abilitycheck
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
zstd -q -19 "$bin" -o "$tmp/nac.zst"
scp -q "$tmp/nac.zst" scripts/pi/run_server_suite.py scripts/pi/full_suite.sh scripts/pi/queue.sh "$PI":/mnt/pi-data/null-harness/
ssh -n "$PI" 'cd /mnt/pi-data/null-harness && chmod +x full_suite.sh queue.sh && mv -f bin/null-abilitycheck bin/null-abilitycheck.prev &&
  zstd -q -d nac.zst -o bin/null-abilitycheck && chmod +x bin/null-abilitycheck && rm nac.zst && sha256sum bin/null-abilitycheck' | cut -d' ' -f1 > "$tmp/remote"
[ "$(cat "$tmp/remote")" = "$(shasum -a 256 "$bin" | cut -d' ' -f1)" ] && echo "deployed $(git rev-parse --short HEAD), checksum ok" || { echo "CHECKSUM MISMATCH" >&2; exit 1; }
