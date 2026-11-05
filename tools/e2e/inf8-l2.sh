#!/usr/bin/env bash
# INF-8 acceptance: the L2 oracle (in-process original-vs-Rust comparison at real call sites).
#
#   tools/e2e/inf8-l2.sh [--rec FILE] [--runs "plain l2 selftest rustleave"] [--max N]
#
# Replays an input recording of the grove walk (default: the INF-4 recording
# $SA_ROOT/target/e2e/lane-2/inf4/rec.sarec; recorded first from tools/e2e/programs/grove-walk.txt
# when missing) in determinism mode, from process start, four times:
#   plain     the ported hooks all on the ORIGINAL (SA_HOOK_MODE), frame hashes traced
#   l2        L2 on for the ported functions (CGeneral::LimitAngle, LimitRadianAngle,
#             GetRadianAngleBetweenPoints, GetATanOfXY, SolveQuadratic, CVector::Normalise,
#             CGameLogic::IsCoopGameGoingOn), the game continuing with the original's results;
#             read/write sets from `sa-oracle l2 spec`
#   selftest  L2 with deliberately wrong Rust implementations (SA_L2_MUTANT) of LimitRadianAngle and
#             CVector::Normalise, and L2 on CTimer::Update with CTimer::m_FrameCounter (0xB7CB4C, a
#             global it writes) dropped from its declared write set (SA_L2_DROP)
#   rustleave like l2, but the game continues with the Rust results (SA_L2_LEAVE=rust; after the
#             case budget the hooks stay on Rust): continuation, reported as INFO vs plain
# and checks:
#   (a) l2: every ported function has >= 1000 cases (or every call it got was compared), 0 mismatches
#       (mismatches matching the pending ledger entry for GetATanOfXY are listed separately);
#   (b) selftest: both mutants are caught, with a report;
#   (c) selftest: the write to 0xB7CB4C is reported as outside the declared write set;
#   (d) l2 trace == plain trace for every frame (the oracle does not perturb the run);
#   host: the l2 spool is stored as content-addressed zstd fixtures (fixtures/l2) and replayed on the
#   host without the game: 0 mismatches; the same fixtures replayed with a mutant: mismatches.
#
# Locks: the global game lock (target/locks/game.lock in SA_ROOT; one game lane at a time until INF-15)
# and the lane lock. Env: LANE (default 2), SA_ROOT. Build first:
#   cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
#   cargo build --release -p sa-oracle-format -p sa-oracle
# Output: target/e2e/lane-N/inf8/ (traces, spools, reports, logs, summary.txt). Exit 0 = all passed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
REC=""
RUNS="plain l2 selftest rustleave"
MAX="${L2_MAX:-1000}"
RUN_TIMEOUT="${RUN_TIMEOUT:-1500}"
while [ $# -gt 0 ]; do
    case "$1" in
        --rec) REC="$2"; shift 2 ;;
        --runs) RUNS="$2"; shift 2 ;;
        --max) MAX="$2"; shift 2 ;;
        *) echo "usage: $0 [--rec FILE] [--runs 'plain l2 selftest'] [--max N]" >&2; exit 2 ;;
    esac
done
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
TGT="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release"
SATOOL="$TGT/satool"
ORACLE="$TGT/sa-oracle"
for b in "$SATOOL" "$ORACLE"; do
    [ -x "$b" ] || { echo "inf8: $b missing (cargo build --release -p sa-oracle-format -p sa-oracle)" >&2; exit 1; }
done
[ -n "$REC" ] || REC="$SA_ROOT/target/e2e/lane-2/inf4/rec.sarec"

PORTS="CGeneral::LimitAngle,CGeneral::LimitRadianAngle,CGeneral::GetRadianAngleBetweenPoints-,CGeneral::GetATanOfXY,CGeneral::SolveQuadratic,CVector::Normalise,CGameLogic::IsCoopGameGoingOn"
ALLMODES="CGeneral::LimitAngle=orig,CGeneral::LimitRadianAngle=orig,CGeneral::GetRadianAngleBetweenPoints-=orig,CGeneral::GetATanOfXY=orig,CGeneral::SolveQuadratic=orig,CVector::Normalise=orig,CGameLogic::IsCoopGameGoingOn=orig"
MUTANTS="CGeneral::LimitRadianAngle,CVector::Normalise"
DROP="0xB7CB4C"

OUT="$E2E_OUT/inf8"
mkdir -p "$OUT"
FAILS=0
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
check() { # check NAME OK? DETAIL
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %s: %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
note() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }
finish() {
    e2e_cleanup
    e2e_say "$FAILS failure(s); summary $SUMMARY"
}

e2e_disk_check || exit 1
# One game lane at a time (INF-15): the global lock first, then the lane lock.
mkdir -p "$SA_ROOT/target/locks"
exec {GAME_LOCK_FD}> "$SA_ROOT/target/locks/game.lock"
if ! flock -n "$GAME_LOCK_FD"; then
    e2e_say "waiting for target/locks/game.lock"
    flock "$GAME_LOCK_FD"
fi
e2e_lane_lock
e2e_lane_setup || exit 1
e2e_install || exit 1
trap finish EXIT
wineserver -k 2> /dev/null || true

DET="$GTA_DIR/det"
mkdir -p "$DET"
win() { winepath -w "$1"; }
"$ORACLE" l2 spec --out "$OUT/l2.spec" || exit 1
cp "$OUT/l2.spec" "$DET/l2.spec"

# run_game NAME VAR=VALUE... : one determinism-mode run from process start to the end frame.
run_game() {
    local name="$1" start last="" st kv
    shift
    rm -f "$DET/$name.trace" "$DET/$name.spool" "$DET/$name.spool.txt" "$GTA_DIR/sa-hook.log"
    (
        export SA_DET=1 SA_DET_EXIT=1 SA_DET_TRACE="$(win "$DET/$name.trace")"
        for kv in "$@"; do export "${kv?}"; done
        e2e_game_start
        wait "$E2E_GAME_PID"
    ) &
    local pid=$!
    e2e_say "run $name ($*)"
    start=$(date +%s)
    while kill -0 "$pid" 2> /dev/null; do
        st="$(e2e_ctl det)"
        [ -n "$st" ] && last="$st"
        st="$(e2e_ctl l2)"
        [ -n "$st" ] && printf '%s\n' "$st" > "$OUT/$name.l2-live"
        if [ $(($(date +%s) - start)) -gt "$RUN_TIMEOUT" ]; then
            e2e_say "run $name: timeout after $RUN_TIMEOUT s; $last"
            e2e_ctl stack > "$OUT/$name.stack"
            break
        fi
        sleep 5
    done
    wineserver -k 2> /dev/null || true
    wait "$pid" 2> /dev/null
    cp -f "$GTA_DIR/sa-hook.log" "$OUT/$name.log" 2> /dev/null
    for f in trace spool spool.txt; do
        [ -f "$DET/$name.$f" ] && mv -f "$DET/$name.$f" "$OUT/$name.$f"
    done
    printf '%s\n' "$last" > "$OUT/$name.det"
    e2e_say "run $name finished in $(($(date +%s) - start)) s"
}

trace_info() { "$SATOOL" show "$1" | awk 'NR > 1 { n++; f = $1 } END { printf "%d frames, last %d", n, f }'; }
# l2 summary line of a run (from its log)
l2_summary() { sed -n 's/.*l2: summary: //p' "$OUT/$1.log" | tail -1; }
# field FIELD of function NAME in a summary
l2_field() { printf '%s\n' "$1" | tr ';' '\n' | grep -F " $2 calls=" -m1 | tr ' ' '\n' | sed -n "s/^$3=//p" | head -1; }
l2_fn_line() { printf '%s\n' "$1" | tr ';' '\n' | grep -F " $2 calls=" -m1; }

# ---- the recording ----------------------------------------------------------------------------------
e2e_headless_start || exit 1
if [ ! -s "$REC" ]; then
    e2e_say "no recording at $REC: recording the grove walk first"
    cp "$HERE/programs/grove-walk.txt" "$DET/program.txt"
    REC="$OUT/rec.sarec"
    run_game record SA_DET_PROGRAM="$(win "$DET/program.txt")" SA_DET_RECORD="$(win "$DET/rec.sarec")"
    cp -f "$DET/rec.sarec" "$REC"
fi
cp -f "$REC" "$DET/inf8.sarec"
END_FRAME="$("$SATOOL" rec "$REC" 2>/dev/null | sed -n 's/.*last_frame=\([0-9]*\).*/\1/p' | tail -1)"
END_FRAME=$((END_FRAME + 1))
note "recording $REC ($("$SATOOL" rec "$REC" 2>&1 | tail -1)); end frame $END_FRAME"
REPLAY="SA_DET_REPLAY=$(win "$DET/inf8.sarec")"

for run in $RUNS; do
    case "$run" in
        plain)
            run_game plain "$REPLAY" SA_HOOK_MODE="$ALLMODES"
            check "plain replay reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/plain.log" && echo 1)" "$(trace_info "$OUT/plain.trace")"
            ;;
        l2)
            run_game l2 "$REPLAY" SA_HOOK_MODE="$ALLMODES" SA_L2="$PORTS" SA_L2_MAX="$MAX" \
                SA_L2_PER_FRAME="${L2_PER_FRAME:-4}" SA_L2_SPEC="$(win "$DET/l2.spec")" SA_L2_OUT="$(win "$DET/l2.spool")"
            check "l2 replay reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/l2.log" && echo 1)" "$(trace_info "$OUT/l2.trace")"
            ;;
        selftest)
            run_game selftest "$REPLAY" SA_HOOK_MODE="$ALLMODES" SA_L2="CGeneral::LimitRadianAngle,CVector::Normalise,CTimer::Update" \
                SA_L2_MAX=200 SA_L2_PER_FRAME=2 SA_L2_MUTANT="$MUTANTS" SA_L2_DROP="$DROP" \
                SA_L2_SPEC="$(win "$DET/l2.spec")" SA_L2_OUT="$(win "$DET/selftest.spool")"
            check "selftest replay reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/selftest.log" && echo 1)" "$(trace_info "$OUT/selftest.trace")"
            ;;
        rustleave)
            # Continuation test: the game continues with the Rust implementations' results.
            run_game rustleave "$REPLAY" SA_HOOK_MODE="$ALLMODES" SA_L2="$PORTS" SA_L2_MAX="$MAX" SA_L2_LEAVE=rust \
                SA_L2_PER_FRAME="${L2_PER_FRAME:-4}" SA_L2_SPEC="$(win "$DET/l2.spec")" SA_L2_OUT="$(win "$DET/rustleave.spool")"
            check "rustleave replay reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/rustleave.log" && echo 1)" "$(trace_info "$OUT/rustleave.trace")"
            ;;
    esac
done
e2e_cleanup

# ---- (a) l2: cases and mismatches per ported function ------------------------------------------------
if [ -f "$OUT/l2.log" ]; then
    S="$(l2_summary l2)"
    note "l2 summary: $S"
    IFS=',' read -r -a FNS <<< "$PORTS"
    for fn in "${FNS[@]}"; do
        calls="$(l2_field "$S" "$fn" calls)"; cases="$(l2_field "$S" "$fn" cases)"
        mism="$(l2_field "$S" "$fn" mismatches)"; led="$(l2_field "$S" "$fn" ledger)"
        und="$(l2_field "$S" "$fn" undeclared_cases)"
        enough=0
        [ -n "$cases" ] && { [ "$cases" -ge "$MAX" ] || [ "$cases" = "$calls" ]; } && enough=1
        check "(a) $fn: >= $MAX cases or every call compared" "$enough" "calls=$calls cases=$cases"
        check "(a) $fn: 0 mismatches, 0 undeclared writes" \
            "$([ "$mism" = 0 ] && [ "$und" = 0 ] && echo 1)" "mismatches=$mism ledger=$led undeclared_cases=$und"
    done
    "$ORACLE" l2 report "$OUT/l2.spool" > "$OUT/l2-report.txt" 2>&1
    check "(a) host report of the l2 spool: 0 mismatches" "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/l2-report.txt")"
    sed 's/^/    /' "$OUT/l2-report.txt" >> "$SUMMARY"
fi

# ---- (d) the oracle does not perturb the run ----------------------------------------------------------
if [ -f "$OUT/plain.trace" ] && [ -f "$OUT/l2.trace" ]; then
    "$SATOOL" diff "$OUT/plain.trace" "$OUT/l2.trace" > "$OUT/plain-vs-l2.txt" 2>&1
    rc=$?
    full=$("$SATOOL" show "$OUT/l2.trace" | awk -v e="$END_FRAME" 'NR > 1 { n++ } END { print (n == e) }')
    check "(d) l2 trace == plain trace for all $END_FRAME frames" "$([ "$rc" = 0 ] && [ "$full" = 1 ] && echo 1)" \
        "$(tail -1 "$OUT/plain-vs-l2.txt")"
fi
if [ -f "$OUT/plain.trace" ] && [ -f "$OUT/rustleave.trace" ]; then
    "$SATOOL" diff "$OUT/plain.trace" "$OUT/rustleave.trace" > "$OUT/plain-vs-rustleave.txt" 2>&1
    note "INFO continuation (game continues with Rust's results while L2 samples, leave=rust) vs plain: $(tail -1 "$OUT/plain-vs-rustleave.txt")"
    note "INFO rustleave summary: $(l2_summary rustleave)"
fi

# ---- (b), (c) self-test ---------------------------------------------------------------------------
if [ -f "$OUT/selftest.log" ]; then
    S="$(l2_summary selftest)"
    note "selftest summary: $S"
    IFS=',' read -r -a MS <<< "$MUTANTS"
    for fn in "${MS[@]}"; do
        mism="$(l2_field "$S" "$fn" mismatches)"
        first="$(l2_fn_line "$S" "$fn" | sed -n 's/.*first_mismatch="\([^"]*\)".*/\1/p')"
        check "(b) deliberately wrong $fn caught" "$([ -n "$mism" ] && [ "$mism" -gt 0 ] && echo 1)" \
            "mismatches=$mism; first: $first"
    done
    und="$(l2_field "$S" "CTimer::Update" undeclared)"
    check "(c) CTimer::Update write to $DROP (dropped from its write set) flagged" \
        "$(printf '%s' "$und" | grep -qi "0xb7cb4c+" && echo 1)" \
        "undeclared=$und mismatches=$(l2_field "$S" "CTimer::Update" mismatches)"
    "$ORACLE" l2 report "$OUT/selftest.spool" > "$OUT/selftest-report.txt" 2>&1
    note "selftest host report ($OUT/selftest-report.txt):"
    sed 's/^/    /' "$OUT/selftest-report.txt" | head -40 >> "$SUMMARY"
    note "first mismatch reports ($OUT/selftest.spool.txt):"
    grep -m4 -A3 '^MISMATCH' "$OUT/selftest.spool.txt" | sed 's/^/    /' | tee -a "$SUMMARY"
    grep -m2 -A3 'CTimer::Update' "$OUT/selftest.spool.txt" | sed 's/^/    /' | tee -a "$SUMMARY"
fi

# ---- host: fixtures + replay without the game ---------------------------------------------------------
if [ -f "$OUT/l2.spool" ]; then
    FIX="$E2E_SCRIPT_ROOT/fixtures/l2"
    out="$("$ORACLE" l2 ingest "$OUT/l2.spool" --fixtures "$FIX" 2>&1)"
    check "fixtures stored (content-addressed, zstd)" "$([ $? = 0 ] && echo 1)" "$out"
    "$ORACLE" l2 replay --fixtures "$FIX" > "$OUT/host-replay.txt" 2>&1
    rc=$?
    check "host replay of the fixtures without the game: 0 mismatches" "$([ "$rc" = 0 ] && echo 1)" "$(head -1 "$OUT/host-replay.txt")"
    sed 's/^/    /' "$OUT/host-replay.txt" >> "$SUMMARY"
    "$ORACLE" l2 replay --fixtures "$FIX" --mutant "$MUTANTS" > "$OUT/host-replay-mutant.txt" 2>&1
    n="$(awk '/LimitRadianAngle|CVector::Normalise/ { for (i = 1; i <= NF; i++) if ($i ~ /^mismatches=/) { split($i, a, "="); s += a[2] } } END { print s + 0 }' "$OUT/host-replay-mutant.txt")"
    check "host replay with the mutants: caught" "$([ "$n" -gt 0 ] && echo 1)" "$n mismatches (LimitRadianAngle + Normalise)"
fi
crash="$(grep -E -c 'Unhandled exception|page fault|PANIC|dispatch failed' "$OUT"/*.log 2> /dev/null | awk -F: '{ s += $2 } END { print s + 0 }')"
check "no crash / panic / dispatch failure in logs" "$([ "$crash" = 0 ] && echo 1)" "$crash lines"
exit $((FAILS > 0))
