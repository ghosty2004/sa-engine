#!/usr/bin/env bash
# All-ports L3 check of the current build on every corpus recording, the recordings spread over several lanes
# in parallel (one port-l2.sh "plain rust" run per recording, each with its own tag).
#
#   tools/e2e/verify-all.sh TAG LANE[,LANE...] [RECORDING...]
#
# Default recordings: the grove walk plus every target/e2e/corpus/*.sarec. Prints one line per recording
# (PASS/FAIL with the comparison line) and exits non-zero if any recording is not IDENTICAL.
# Env: SA_ROOT (checkout holding wine/ and the corpus; default: the repo; the DLL is always built from this
# script's checkout into CARGO_TARGET_DIR or its target/), MEM (systemd scope MemoryMax per run, default 6G).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SRC="$(cd "$HERE/../.." && pwd)"   # the checkout under test (a worktree builds and tests its own code)
ROOT="${SA_ROOT:-$SRC}"            # the checkout holding wine/, the corpus and the shared outputs
TAG="${1:?usage: $0 TAG LANES [REC...]}"
IFS=, read -r -a LANES <<< "${2:?usage: $0 TAG LANES [REC...]}"
shift 2
RECS=("$@")
if [ ${#RECS[@]} -eq 0 ]; then
    RECS=("$ROOT/target/e2e/lane-2/inf4/rec.sarec")
    for r in "$ROOT"/target/e2e/corpus/*.sarec; do RECS+=("$r"); done
fi
# Always test the current tree: build the DLL and launcher first (a stale DLL silently tests old code).
if [ -z "${VERIFY_NO_BUILD:-}" ]; then
    (cd "$SRC" && systemd-run --user --scope -q -p MemoryMax="${BUILD_MEM:-14G}" \
        cargo build -q --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook) \
        || { echo "verify-all: build failed, nothing verified" >&2; exit 2; }
fi
OUT="$ROOT/target/verify-all/$TAG"
mkdir -p "$OUT"
# One worker per lane; each takes the next recording from a shared index file (flock).
echo 0 > "$OUT/next"
git -C "$SRC" rev-parse HEAD > "$OUT/head" 2>/dev/null
sha256sum "${E2E_BIN:-${CARGO_TARGET_DIR:-$SRC/target}/i686-pc-windows-gnu/release}/sa_hook.dll" > "$OUT/dll.sha256" 2>/dev/null
worker() {
    local lane=$1
    while :; do
        local i
        i=$(flock "$OUT/next" sh -c 'n=$(cat "$0"); echo $((n+1)) > "$0"; echo $n' "$OUT/next")
        [ "$i" -lt ${#RECS[@]} ] || break
        local rec=${RECS[$i]} name
        name=$(basename "$(dirname "$rec")")-$(basename "$rec" .sarec)
        LANE=$lane SA_ROOT="$ROOT" systemd-run --user --scope -q -p MemoryMax="${MEM:-6G}" \
            "$HERE/port-l2.sh" ports "$rec" --runs "plain rust" --tag "$TAG-$name" > "$OUT/$name.log" 2>&1
        local line
        line=$(grep -E '^(PASS|FAIL) \(c\)' "$OUT/$name.log" | tail -1)
        [ -n "$line" ] || line="FAIL (c) no comparison line (see $OUT/$name.log)"
        # SAVE-1: a recording that writes save files must write plain's bytes (port-l2 check (f)), except in
        # the stack-residue positions of the 1.0 save structures (ledger SAVE-1, tools/accept/save-diff.py
        # --mask): those differ whenever a save port's caller runs on Rust
        local fl d f
        fl=$(grep -E '^FAIL \(f\)' "$OUT/$name.log" | head -1)
        if [ -n "$fl" ]; then
            d="$SRC/target/e2e/lane-$lane/port-l2-$TAG-$name"
            line="FAIL save files differ: ${fl#FAIL }"
            for f in "$d"/rust.GTASAsf[1-8].b; do
                [ -f "$f" ] || continue
                python3 -I "$SRC/tools/accept/save-diff.py" "$d/plain.${f##*/rust.}" "$f" --mask > "$f.diff.txt" 2>&1
                if [ $? = 3 ]; then
                    line="$(grep -E '^(PASS|FAIL) \(c\)' "$OUT/$name.log" | tail -1); save files identical except stack residue ($(grep -o '[0-9]* differing bytes are residue' "$f.diff.txt"), ledger SAVE-1)"
                fi
            done
        fi
        grep -q -E '^PASS \(f\)' "$OUT/$name.log" && line="$line; save files byte-identical"
        echo "$name: $line" >> "$OUT/summary.txt"
    done
}
: > "$OUT/summary.txt"
for l in "${LANES[@]}"; do worker "$l" & done
wait
sort "$OUT/summary.txt"
! grep -q -v 'IDENTICAL' "$OUT/summary.txt"
