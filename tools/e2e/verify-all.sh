#!/usr/bin/env bash
# All-ports L3 check of the current build on every corpus recording, the recordings spread over several lanes
# in parallel (one port-l2.sh "plain rust" run per recording, each with its own tag).
#
#   tools/e2e/verify-all.sh TAG LANE[,LANE...] [RECORDING...]
#
# Default recordings: the grove walk plus every target/e2e/corpus/*.sarec. Prints one line per recording
# (PASS/FAIL with the comparison line) and exits non-zero if any recording is not IDENTICAL.
# Env: SA_ROOT (default: the repo), MEM (systemd scope MemoryMax per run, default 6G).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="${SA_ROOT:-$(cd "$HERE/../.." && pwd)}"
TAG="${1:?usage: $0 TAG LANES [REC...]}"
IFS=, read -r -a LANES <<< "${2:?usage: $0 TAG LANES [REC...]}"
shift 2
RECS=("$@")
if [ ${#RECS[@]} -eq 0 ]; then
    RECS=("$ROOT/target/e2e/lane-2/inf4/rec.sarec")
    for r in "$ROOT"/target/e2e/corpus/*.sarec; do RECS+=("$r"); done
fi
OUT="$ROOT/target/verify-all/$TAG"
mkdir -p "$OUT"
# One worker per lane; each takes the next recording from a shared index file (flock).
echo 0 > "$OUT/next"
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
        echo "$name: $line" >> "$OUT/summary.txt"
    done
}
: > "$OUT/summary.txt"
for l in "${LANES[@]}"; do worker "$l" & done
wait
sort "$OUT/summary.txt"
! grep -q -v 'IDENTICAL' "$OUT/summary.txt"
