#!/usr/bin/env bash
# INF-9: L3 runner with automatic divergence bisection (and the PLAN §5 hook-off check).
#
#   tools/e2e/l3.sh [--rec FILE] [--rust ports|A,B] [--fixed SET] [--hookoff none|all|A,B] [--margin N] [--mutant A,B]
#   tools/e2e/l3.sh --selftest [--rec FILE]
#   tools/e2e/l3.sh --l2 HOOK --from FRAME --end FRAME [--rec FILE] [--rust ports|A,B]
#   tools/e2e/l3.sh --probe NAME END MODES OUT_TRACE        (one game run; used by sa-oracle l3)
#
# Default mode: `sa-oracle l3` replays the recording (default: the INF-4 grove walk,
# $SA_ROOT/target/e2e/lane-2/inf4/rec.sarec) in determinism mode with every Rust port on the original
# (the reference trace, cached under target/l3/cache per recording + exe + sa_hook.dll + environment)
# and with the --rust set on Rust; on a divergence it bisects the set down to the minimal culprit
# hooks without help and prints, per culprit, where it diverges alone and a suggested L2 run. With
# --hookoff it then switches each hook of the matching set back to the original alone (must still match).
# --fixed SET (e.g. `verified`): those hooks are on Rust in every probe and never bisected; the --rust
# set (e.g. `batch:ID`) is tested on top of them (tools/verify/bisect-batch.sh). SETs also accept
# `verified`, `unverified` and `batch:ID`; unverified hooks need SA_UNVERIFIED (docs/verify/HOWTO.md).
#
# --selftest: (1) the plain L3 run of the --rust set (default every port) with the hook-off check of
# every hook (L3_SELFTEST_HOOKOFF=A,B: only those); (2) the same
# with a deliberately wrong Rust CGeneral::LimitRadianAngle (SA_L2_MUTANT: last bit of the result
# flipped for |angle| > 1) that the bisection must report as a culprit on its own, besides the
# culprits of (1).
#
# --l2: one replay up to --end with the --rust set on Rust (default: every port except HOOK) and HOOK in
# L2 mode from frame --from (game continues with the original's results); prints the L2 report.
#
# Every game run holds the lane lock (and target/locks/game.lock with L3_GAME_LOCK=1).
# Env: LANE (default 3), SA_ROOT (checkout holding wine/; set it from a worktree), RUN_TIMEOUT (s per
# run, default 900), L3_CACHE (trace cache directory, default sa-oracle's target/l3/cache). Build first:
#   cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
#   cargo build --release -p sa-oracle-format -p sa-oracle
# Output: target/e2e/lane-N/l3/ (reports, probe traces and logs, summary.txt). Exit 0 = all passed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
SELF="$HERE/$(basename "$0")"
LANE="${LANE:-3}"
RUN_TIMEOUT="${RUN_TIMEOUT:-900}"

# ---------------------------------------------------------------------------------------------------
# One game run (probe). Locks, lane setup and the headless display are taken here unless the caller
# (this script's top level, exporting L3_LOCKED=1) already holds them.
# Extra inputs through the environment: L3_REC (recording), L3_COV_POINTS / L3_COV_CAP (coverage,
# INF-10), L3_L2="HOOK FROM" (L2 on HOOK from FROM), L3_HOOK_PORTS (SA_HOOK_PORTS, default orig).
# Outputs next to OUT_TRACE: .log (sa-hook.log), .det (last det status), .cov.csv, .l2.spool(.txt).
probe_main() {
    local name="$1" end="$2" modes="$3" out="$4"
    # shellcheck source=lib.sh
    source "$HERE/lib.sh"
    if [ "${L3_LOCKED:-0}" != 1 ]; then
        e2e_disk_check || return 1
        take_game_lock
        e2e_lane_lock
        e2e_lane_setup || return 1
        e2e_install || return 1
        trap e2e_cleanup EXIT
        e2e_headless_start || return 1
    fi
    local DET="$GTA_DIR/det"
    mkdir -p "$DET" "$(dirname "$out")"
    [ -n "${L3_REC:-}" ] && [ -s "$L3_REC" ] || { echo "l3 probe: L3_REC not set" >&2; return 1; }
    cmp -s "$L3_REC" "$DET/l3.sarec" || cp -f "$L3_REC" "$DET/l3.sarec"
    win() { e2e_winpath "$1"; }
    wineserver -k 2> /dev/null || true
    # never leave an earlier probe's trace at OUT (it would be read as this run's)
    rm -f "$DET/$name".* "$GTA_DIR/sa-hook.log" "$out"
    (
        export SA_DET=1 SA_DET_EXIT=1 SA_DET_TRACE="$(win "$DET/$name.trace")" SA_DET_REPLAY="$(win "$DET/l3.sarec")"
        # Engine-crate ports are installed in original mode (L3_HOOK_PORTS overrides SA_HOOK_PORTS; set
        # it empty to leave the ports uninstalled); MODES switches them.
        export SA_DET_END="$end" SA_HOOK_PORTS="${L3_HOOK_PORTS-orig}"
        # a mode list over every port exceeds a Windows environment variable: passed as @FILE then
        hm="$(e2e_list_env SA_HOOK_MODE "$modes" "$DET/$name.modes")" || exit 1
        export "${hm?}"
        e2e_unverified_env
        if [ -n "${L3_COV_POINTS:-}" ]; then
            cp -f "$L3_COV_POINTS" "$DET/$name.points"
            export SA_COV="$(win "$DET/$name.points")" SA_COV_OUT="$(win "$DET/$name.cov.csv")" SA_COV_CAP="${L3_COV_CAP:-1}"
        fi
        if [ -n "${L3_L2:-}" ]; then
            set -- $L3_L2
            export SA_L2="$1" SA_L2_FROM="$2" SA_L2_MAX="${L2_MAX:-200}" SA_L2_PER_FRAME="${L2_PER_FRAME:-1000}"
            export SA_L2_OUT="$(win "$DET/$name.l2.spool")"
            [ -f "$DET/l2.spec" ] && export SA_L2_SPEC="$(win "$DET/l2.spec")"
        fi
        e2e_game_start
        wait "$E2E_GAME_PID"
    ) &
    local pid=$! start last="" st
    start=$(date +%s)
    e2e_say "probe $name: end $end, $modes${L3_COV_POINTS:+, coverage}${L3_L2:+, l2 $L3_L2}${SA_L2_MUTANT:+, mutant $SA_L2_MUTANT}"
    while kill -0 "$pid" 2> /dev/null; do
        st="$(e2e_ctl det)"
        [ -n "$st" ] && last="$st"
        if [ $(($(date +%s) - start)) -gt "$RUN_TIMEOUT" ]; then
            e2e_say "probe $name: timeout after $RUN_TIMEOUT s; $last"
            e2e_ctl stack > "$out.stack"
            break
        fi
        sleep 3
    done
    wineserver -k 2> /dev/null || true
    wait "$pid" 2> /dev/null
    cp -f "$GTA_DIR/sa-hook.log" "$out.log" 2> /dev/null
    printf '%s\n' "$last" > "$out.det"
    [ -f "$DET/$name.trace" ] && mv -f "$DET/$name.trace" "$out"
    [ -f "$DET/$name.cov.csv" ] && mv -f "$DET/$name.cov.csv" "$out.cov.csv"
    [ -f "$DET/$name.cov.csv.last" ] && mv -f "$DET/$name.cov.csv.last" "$out.cov.csv.last"
    [ -f "$DET/$name.l2.spool" ] && mv -f "$DET/$name.l2.spool" "$out.l2.spool"
    [ -f "$DET/$name.l2.spool.txt" ] && mv -f "$DET/$name.l2.spool.txt" "$out.l2.spool.txt"
    rm -f "$DET/$name".*
    e2e_say "probe $name finished in $(($(date +%s) - start)) s: $(grep -o 'frame=[0-9]* ticks=[0-9]*' "$out.det" | head -1)"
    [ -s "$out" ] || { echo "l3 probe $name: no trace written" >&2; return 1; }
    # A start-mode list sa-hook could not read leaves every port on the original: the probe would
    # compare the original with itself (BISECT-CARAI). Never accept such a trace.
    if grep -q -E 'SA_HOOK_MODE ignored|l2: bad configuration' "$out.log" 2> /dev/null; then
        echo "l3 probe $name: $(grep -m1 -o -E '(SA_HOOK_MODE ignored|l2: bad configuration).{0,120}' "$out.log")" >&2
        rm -f "$out"
        return 1
    fi
}

# Since INF-15 (CdStreamSync fix) lanes run concurrently: the lane lock is enough. L3_GAME_LOCK=1 also
# takes the global target/locks/game.lock (one game at a time, e.g. for timing-sensitive runs).
take_game_lock() {
    [ "${L3_GAME_LOCK:-0}" = 1 ] || return 0
    mkdir -p "$SA_ROOT/target/locks"
    exec {GAME_LOCK_FD}> "$SA_ROOT/target/locks/game.lock"
    if ! flock -n "$GAME_LOCK_FD"; then
        e2e_say "waiting for target/locks/game.lock"
        flock "$GAME_LOCK_FD"
    fi
}

if [ "${1:-}" = --probe ]; then
    [ $# = 5 ] || { echo "usage: $0 --probe NAME END MODES OUT_TRACE" >&2; exit 2; }
    probe_main "$2" "$3" "$4" "$5"
    exit $?
fi

# ---------------------------------------------------------------------------------------------------
REC=""
RUST="ports"
HOOKOFF="none"
FIXED=""
# --selftest: the hook-off list (default: every hook of the matching set; with hundreds of engine
# ports registered, pass a shorter list).
SELFTEST_HOOKOFF="${L3_SELFTEST_HOOKOFF:-all}"
MARGIN="${L3_MARGIN:-100}"
MUTANT=""
MODE=run
L2_HOOK="" L2_FROM="" L2_END=""
while [ $# -gt 0 ]; do
    case "$1" in
        --rec) REC="$2"; shift 2 ;;
        --rust) RUST="$2"; shift 2 ;;
        --hookoff) HOOKOFF="$2"; shift 2 ;;
        --fixed) FIXED="$2"; shift 2 ;;
        --margin) MARGIN="$2"; shift 2 ;;
        --mutant) MUTANT="$2"; shift 2 ;;
        --selftest) MODE=selftest; shift ;;
        --l2) MODE=l2; L2_HOOK="$2"; shift 2 ;;
        --from) L2_FROM="$2"; shift 2 ;;
        --end) L2_END="$2"; shift 2 ;;
        *) echo "usage: $0 [--rec F] [--rust ports|A,B] [--hookoff none|all|A,B] [--margin N] [--mutant A,B] | --selftest | --l2 HOOK --from N --end N | --probe ..." >&2; exit 2 ;;
    esac
done
# shellcheck source=lib.sh
source "$HERE/lib.sh"
TGT="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release"
ORACLE="$TGT/sa-oracle"
SATOOL="$TGT/satool"
for b in "$ORACLE" "$SATOOL"; do
    [ -x "$b" ] || { echo "l3: $b missing (cargo build --release -p sa-oracle-format -p sa-oracle)" >&2; exit 1; }
done
[ -n "$REC" ] || REC="$SA_ROOT/target/e2e/lane-2/inf4/rec.sarec"
[ -s "$REC" ] || { echo "l3: no recording $REC (run tools/e2e/inf4-determinism.sh first)" >&2; exit 1; }
REC="$(cd "$(dirname "$REC")" && pwd)/$(basename "$REC")"

OUT="$E2E_OUT/l3"
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
take_game_lock
e2e_lane_lock
e2e_lane_setup || exit 1
e2e_install || exit 1
trap finish EXIT
wineserver -k 2> /dev/null || true
e2e_headless_start || exit 1
export L3_LOCKED=1 L3_REC="$REC" E2E_DISPLAY E2E_GAMESCOPE

DLL="$GTA_DIR/sa_hook.dll"
EXE="$(readlink -f "$GTA_DIR/gta_sa.exe")"
# l3 NAME [extra sa-oracle args...]: one sa-oracle l3 session, report in OUT/NAME.txt
l3() {
    local name="$1"
    shift
    "$ORACLE" l3 --rec "$REC" --runner "$SELF --probe" --exe "$EXE" --dll "$DLL" --margin "$MARGIN" \
        --work "$OUT/probes" ${L3_CACHE:+--cache "$L3_CACHE"} --report "$OUT/$name.txt" "$@" 2>&1 | tee "$OUT/$name.log"
    return "${PIPESTATUS[0]}"
}
culprits() { sed -n 's/^CULPRIT \({[^}]*}\).*/\1/p' "$OUT/$1.txt" | sort; }

case "$MODE" in
    run)
        [ -n "$MUTANT" ] && export SA_L2_MUTANT="$MUTANT"
        l3 run --rust "$RUST" --hookoff "$HOOKOFF" ${FIXED:+--fixed "$FIXED"}
        rc=$?
        check "L3 $RUST${MUTANT:+ (mutant $MUTANT)}" "$([ "$rc" = 0 ] && echo 1)" "$(grep -m1 '^L3 ' "$OUT/run.txt")"
        grep -E '^(FIXED|CULPRIT|  alone|  suggest|rest|HOOKOFF|probes)' "$OUT/run.txt" | tee -a "$SUMMARY"
        ;;
    selftest)
        unset SA_L2_MUTANT
        l3 clean --rust "$RUST" --hookoff "$SELFTEST_HOOKOFF"
        note "clean: $(grep -m1 '^L3 ' "$OUT/clean.txt")"
        grep -E '^(CULPRIT|  alone|rest|HOOKOFF)' "$OUT/clean.txt" | tee -a "$SUMMARY"
        n_off="$(grep -c '^HOOKOFF' "$OUT/clean.txt")"
        n_bad="$(grep '^HOOKOFF' "$OUT/clean.txt" | grep -vc ': MATCH')"
        check "hook-off check ran on the matching set" "$([ "$n_off" -gt 0 ] && echo 1)" "$n_off hook-off runs, $n_bad diverged"
        check "hook-off check: every hook back on the original alone still matches" "$([ "$n_off" -gt 0 ] && [ "$n_bad" = 0 ] && echo 1)" \
            "$(grep '^HOOKOFF' "$OUT/clean.txt" | tr '\n' ';')"
        SA_L2_MUTANT=CGeneral::LimitRadianAngle l3 mutant --rust "$RUST"
        note "mutant: $(grep -m1 '^L3 ' "$OUT/mutant.txt")"
        grep -E '^(CULPRIT|  alone|  suggest|rest)' "$OUT/mutant.txt" | tee -a "$SUMMARY"
        expect="$( (culprits clean; echo '{CGeneral::LimitRadianAngle}') | sort -u)"
        got="$(culprits mutant | sort -u)"
        check "injected fault found without help: culprits = clean culprits + {CGeneral::LimitRadianAngle}" \
            "$([ "$got" = "$expect" ] && echo 1)" "got [$(echo "$got" | tr '\n' ' ')] expected [$(echo "$expect" | tr '\n' ' ')]"
        grep '^probes' "$OUT/clean.txt" "$OUT/mutant.txt" | tee -a "$SUMMARY"
        ;;
    l2)
        [ -n "$L2_HOOK" ] && [ -n "$L2_FROM" ] && [ -n "$L2_END" ] || { echo "l3: --l2 needs HOOK --from N --end N" >&2; exit 2; }
        [ -n "$MUTANT" ] && export SA_L2_MUTANT="$MUTANT"
        "$ORACLE" l2 spec --out "$GTA_DIR/det/l2.spec" > /dev/null || exit 1
        # the --rust set (default: every port but HOOK) on Rust, HOOK in L2 mode
        export L3_L2="$L2_HOOK $L2_FROM"
        hm="$("$ORACLE" l3 --print-modes "$RUST" --except "$L2_HOOK")" || exit 1
        "$SELF" --probe l2 "$L2_END" "$hm" "$OUT/l2.trace"
        rep="$OUT/l2.trace.l2.spool.txt"
        check "L2 run on $L2_HOOK from frame $L2_FROM" "$([ -s "$rep" ] && echo 1)" "$(sed -n 's/.*l2: summary: //p' "$OUT/l2.trace.log" | tail -1)"
        grep -m6 -A3 '^MISMATCH' "$rep" 2> /dev/null | tee -a "$SUMMARY"
        ;;
esac
exit $((FAILS > 0))
