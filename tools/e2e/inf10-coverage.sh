#!/usr/bin/env bash
# INF-10: execution coverage of the original gta_sa.exe during a replay (first-hit breakpoints).
#
#   tools/e2e/inf10-coverage.sh [--rec FILE] [--name NAME] [--cap N] [--no-blocks] [--rust ports|none]
#
# 1. `sa-oracle coverage points`: every armed catalogue function entry + the basic-block starts decoded
#    inside its Ghidra body ranges, with the exe byte at each point;
# 2. one determinism-mode replay of the recording (default: the INF-4 grove walk) with sa-hook's
#    coverage mode (SA_COV): an int3 on every point, counted and removed at its first execution
#    (SA_COV_CAP > 1: re-armed until CAP hits); the Rust ports on the original (--rust none, default)
#    or on Rust (--rust ports: what the original still executes with the ports on, the F2 view);
# 3. the replay's frame-hash trace must equal the reference trace (breakpoints must not perturb the
#    game): the cached L3 reference of tools/e2e/l3.sh, or a plain replay run here when missing;
# 4. `sa-oracle coverage report`: catalog/raw/coverage-NAME.csv (address,hits,blocks_hit,blocks_total,
#    subsystem,class,state,name per function), catalog/raw/coverage-NAME.hits.csv (every point) and the
#    per-subsystem report; plus the merge of every catalog/raw/coverage-*.hits.csv (all recordings).
#
# Locks: the lane lock (via tools/e2e/l3.sh --probe; L3_GAME_LOCK=1 adds target/locks/game.lock). Env: LANE (default
# 3), SA_ROOT. Build first (see tools/e2e/l3.sh). Output: target/e2e/lane-N/inf10/. Exit 0 = passed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REC=""
NAME="grove-walk"
CAP=1
BLOCKS=""
RUSTSET=none
while [ $# -gt 0 ]; do
    case "$1" in
        --rec) REC="$2"; shift 2 ;;
        --name) NAME="$2"; shift 2 ;;
        --cap) CAP="$2"; shift 2 ;;
        --no-blocks) BLOCKS="--no-blocks"; shift ;;
        --rust) RUSTSET="$2"; shift 2 ;;
        *) echo "usage: $0 [--rec FILE] [--name NAME] [--cap N] [--no-blocks] [--rust none|ports]" >&2; exit 2 ;;
    esac
done
LANE="${LANE:-3}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
TGT="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release"
ORACLE="$TGT/sa-oracle"
SATOOL="$TGT/satool"
for b in "$ORACLE" "$SATOOL"; do
    [ -x "$b" ] || { echo "inf10: $b missing (cargo build --release -p sa-oracle-format -p sa-oracle)" >&2; exit 1; }
done
[ -n "$REC" ] || REC="$SA_ROOT/target/e2e/lane-2/inf4/rec.sarec"
[ -s "$REC" ] || { echo "inf10: no recording $REC" >&2; exit 1; }
REC="$(cd "$(dirname "$REC")" && pwd)/$(basename "$REC")"
END="$("$SATOOL" rec "$REC" | sed -n 's/^frames=//p')"
[ -n "$END" ] || { echo "inf10: recording has no frames= header" >&2; exit 1; }

OUT="$E2E_OUT/inf10"
RAW="$SA_ROOT/catalog/raw"
mkdir -p "$OUT" "$RAW"
FAILS=0
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
check() {
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %s: %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
note() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }

"$ORACLE" coverage points --out "$OUT/points.txt" $BLOCKS 2> "$OUT/points.log"
check "points generated" "$([ -s "$OUT/points.txt" ] && echo 1)" "$(grep '^points:' "$OUT/points.log")"

case "$RUSTSET" in
    none) MODES="$("$ORACLE" l3 --print-modes ports --except ports 2> /dev/null)" ;;
    *) MODES="$("$ORACLE" l3 --print-modes "$RUSTSET" 2> /dev/null)" ;;
esac
export L3_REC="$REC"
L3_COV_POINTS="$OUT/points.txt" L3_COV_CAP="$CAP" "$HERE/l3.sh" --probe "cov-$NAME" "$END" "$MODES" "$OUT/cov.trace"
check "coverage replay reached the end frame" "$(grep -q "end frame $END reached" "$OUT/cov.trace.log" && echo 1)" \
    "$(grep -m1 'coverage: [0-9]* of' "$OUT/cov.trace.log" | sed 's/.*coverage: //')"
note "$(grep 'coverage: final' "$OUT/cov.trace.log" | sed 's/.*coverage: //')"

# Reference: the cached L3 reference (same recording/exe/dll key) if l3.sh made one, else a plain replay.
REF="$OUT/ref-$RUSTSET.trace"
if [ ! -s "$REF" ]; then
    "$HERE/l3.sh" --probe "ref-$NAME" "$END" "$MODES" "$REF"
fi
"$SATOOL" diff "$REF" "$OUT/cov.trace" > "$OUT/ref-vs-cov.txt" 2>&1
rc=$?
n="$(sed -n "s/^frames: .* common=\([0-9]*\) .*/\1/p" "$OUT/ref-vs-cov.txt")"
check "breakpoints do not perturb the game: coverage trace == plain replay trace for all $END frames" "$([ "$rc" = 0 ] && [ "$n" = "$END" ] && echo 1)" "$(tail -1 "$OUT/ref-vs-cov.txt")"

cp -f "$OUT/cov.trace.cov.csv" "$RAW/coverage-$NAME.hits.csv"
"$ORACLE" coverage report --points "$OUT/points.txt" --hits "$RAW/coverage-$NAME.hits.csv" \
    --title "INF-10 coverage of $NAME ($REC, $END frames, ports $RUSTSET, cap $CAP)" \
    --csv "$RAW/coverage-$NAME.csv" --out "$OUT/report-$NAME.txt" > /dev/null
check "report written" "$([ -s "$RAW/coverage-$NAME.csv" ] && echo 1)" "$RAW/coverage-$NAME.csv"
sed 's/^/    /' "$OUT/report-$NAME.txt" | tee -a "$SUMMARY"
all="$(ls "$RAW"/coverage-*.hits.csv | grep -v /coverage-all.hits.csv | tr '\n' ',')"
"$ORACLE" coverage report --points "$OUT/points.txt" --hits "$all" --title "merged coverage: $all" \
    --merged "$RAW/coverage-all.hits.csv" --csv "$RAW/coverage-all.csv" --out "$OUT/report-all.txt" > /dev/null
note "merged over $(echo "$all" | tr ',' '\n' | grep -c .) recording(s): $RAW/coverage-all.csv"
exit $((FAILS > 0))
