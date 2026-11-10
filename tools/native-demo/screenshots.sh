#!/usr/bin/env bash
# NATIVE-DEMO: headless screenshots of sa-viewer at known locations, plus a scripted fly-through
# benchmark. Usage: tools/native-demo/screenshots.sh [GAME_DIR] [OUT_DIR]
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
game="${1:-${SA_GAME_DIR:-$root/wine/gta-sa}}"
out="${2:-$root/target/native-demo}"
mkdir -p "$out"
cargo build --release -p sa-viewer --manifest-path "$root/Cargo.toml"
bin="$root/target/release/sa-viewer"
run() { systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 "$bin" --game "$game" "$@"; }

# name | position | look direction | time
shots=(
  "grove-street|2440,-1700,32|1,0.55,-0.32|12:00"
  "grove-street-night|2440,-1700,32|1,0.55,-0.32|23:00"
  "downtown-ls|1380,-1600,120|0.45,1,-0.25|17:00"
  "san-fierro|-2400,550,110|1,0.35,-0.18|10:00"
  "las-venturas-strip|2050,900,45|0.02,1,-0.08|21:30"
  "countryside|420,-420,70|-1,0.5,-0.22|08:00"
)
for s in "${shots[@]}"; do
  IFS='|' read -r name pos dir time <<<"$s"
  run --pos "$pos" --dir "$dir" --time "$time" --size 1280x720 --screenshot "$out/$name.png" 2>&1 | grep -E "screenshot|error|panicked" || true
done
run --bench 60 --size 1280x720 --time 14:00 --bench-out "$out/bench.txt"
