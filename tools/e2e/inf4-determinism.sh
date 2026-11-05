#!/usr/bin/env bash
# INF-4/5/6 acceptance: determinism mode + input recorder/replayer + frame-state hasher.
#
#   tools/e2e/inf4-determinism.sh [--program FILE] [--perturb-frame N] [--runs LIST]
#
# Runs gta_sa.exe 1.0 US headless on a lane, always in determinism mode (SA_DET=1), from process start:
#   record   the input program (default tools/e2e/programs/grove-walk.txt: boot, menu, new game, then
#            > 2 min of walking/running/turning around Grove Street through injected pad states) is
#            injected and recorded (rec.sarec) while the frame-state hashes are traced (record.trace)
#   replay1  the recording is replayed (replay1.trace)
#   replay2  again (replay2.trace)
#   perturb  replayed again with 1 ulp added to the player's position x at frame N (perturb.trace)
# and checks: replay1 == replay2 == record over every frame; perturb == replay1 before N and differs
# first at frame N in exactly the `peds` hash.
#
# Env: LANE (default 2), SA_ROOT (checkout holding wine/; set it from a worktree). Build first:
#   cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
#   cargo build --release -p sa-oracle-format          (host tool satool)
# Output: target/e2e/lane-N/inf4/ (traces, recording, logs, summary.txt). Exit 0 = all passed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PROGRAM="$HERE/programs/grove-walk.txt"
PERTURB=""
RUNS="record replay1 replay2 perturb"
RUN_TIMEOUT="${RUN_TIMEOUT:-900}"
while [ $# -gt 0 ]; do
    case "$1" in
        --program) PROGRAM="$2"; shift 2 ;;
        --perturb-frame) PERTURB="$2"; shift 2 ;;
        --runs) RUNS="$2"; shift 2 ;;
        *) echo "usage: $0 [--program FILE] [--perturb-frame N] [--runs 'record replay1 replay2 perturb']" >&2; exit 2 ;;
    esac
done
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
SATOOL="${SATOOL:-${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release/satool}"
[ -x "$SATOOL" ] || { echo "inf4: $SATOOL missing (cargo build --release -p sa-oracle-format)" >&2; exit 1; }
"$SATOOL" program "$PROGRAM" > /dev/null || exit 1
END_FRAME="$(sed -n 's/^end \([0-9]*\).*/\1/p' "$PROGRAM" | tail -1)"
[ -n "$END_FRAME" ] || { echo "inf4: program has no 'end FRAME' line" >&2; exit 1; }
[ -n "$PERTURB" ] || PERTURB=$((END_FRAME - 1500))

OUT="$E2E_OUT/inf4"
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
e2e_lane_lock
e2e_lane_setup || exit 1
e2e_install || exit 1
trap finish EXIT
wineserver -k 2> /dev/null || true

DET="$GTA_DIR/det"
mkdir -p "$DET"
cp "$PROGRAM" "$DET/program.txt"
win() { winepath -w "$1"; }

# run_game NAME VAR=VALUE... : one determinism-mode run from process start to the end frame.
run_game() {
    local name="$1" kv start last="" st
    shift
    rm -f "$DET/$name.trace" "$DET/$name.rand" "$DET/$name.heap" "$GTA_DIR/sa-hook.log"
    (
        export SA_DET=1 SA_DET_EXIT=1 SA_DET_TRACE="$(win "$DET/$name.trace")"
        # INF4_RANDLOG=1: log every main-thread rand/srand call (frame, caller, value) per run.
        [ "${INF4_RANDLOG:-0}" = 1 ] && export SA_DET_RANDLOG="$(win "$DET/$name.rand")"
        [ "${INF4_HEAPLOG:-0}" = 1 ] && export SA_DET_HEAPLOG="$(win "$DET/$name.heap")"
        for kv in "$@"; do export "${kv?}"; done
        e2e_game_start
        wait "$E2E_GAME_PID"
    ) &
    local pid=$!
    e2e_say "run $name ($*)"
    start=$(date +%s)
    local shot=0 f
    while kill -0 "$pid" 2> /dev/null; do
        st="$(e2e_ctl det)"
        [ -n "$st" ] && last="$st"
        f="$(printf '%s' "$st" | sed -n 's/.* frame=\([0-9]*\) .*/\1/p')"
        if [ "$shot" = 0 ] && [ -n "$f" ] && [ "$f" -ge $((END_FRAME - 400)) ]; then
            e2e_shot "inf4-$name-late" > /dev/null && shot=1
        fi
        if [ $(($(date +%s) - start)) -gt "$RUN_TIMEOUT" ]; then
            e2e_say "run $name: timeout after $RUN_TIMEOUT s; $last"
            e2e_ctl stack > "$OUT/$name.stack"
            break
        fi
        sleep 3
    done
    wineserver -k 2> /dev/null || true
    wait "$pid" 2> /dev/null
    cp -f "$GTA_DIR/sa-hook.log" "$OUT/$name.log" 2> /dev/null
    cp -f "$DET/$name.trace" "$OUT/$name.trace" 2> /dev/null
    [ -f "$DET/$name.rand" ] && mv -f "$DET/$name.rand" "$OUT/$name.rand"
    [ -f "$DET/$name.heap" ] && mv -f "$DET/$name.heap" "$OUT/$name.heap"
    printf '%s\n' "$last" > "$OUT/$name.det"
    e2e_say "run $name finished in $(($(date +%s) - start)) s; last status: $last"
}

# frames_in TRACE → number of records and last frame
trace_info() { "$SATOOL" show "$1" | awk 'NR > 1 { n++; f = $1 } END { printf "%d frames, last %d", n, f }'; }

e2e_headless_start || exit 1
for run in $RUNS; do
    case "$run" in
        record)
            rm -f "$DET/rec.sarec"
            run_game record SA_DET_PROGRAM="$(win "$DET/program.txt")" SA_DET_RECORD="$(win "$DET/rec.sarec")"
            cp -f "$DET/rec.sarec" "$OUT/rec.sarec" 2> /dev/null
            check "record run reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/record.log" && echo 1)" "$(trace_info "$OUT/record.trace")"
            check "recording written" "$([ -s "$OUT/rec.sarec" ] && echo 1)" \
                "$("$SATOOL" rec "$OUT/rec.sarec" 2>&1 | tail -1)"
            ;;
        replay1 | replay2)
            run_game "$run" SA_DET_REPLAY="$(win "$DET/rec.sarec")"
            check "$run reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/$run.log" && echo 1)" "$(trace_info "$OUT/$run.trace")"
            check "$run input in sync" \
                "$(grep -q ' desyncs=0 ' "$OUT/$run.det" && echo 1)" "$(grep -o 'ticks=[0-9]* input=[a-z-]*.*desyncs=[0-9]*' "$OUT/$run.det" | head -1)"
            ;;
        perturb)
            run_game perturb SA_DET_REPLAY="$(win "$DET/rec.sarec")" SA_DET_PERTURB="$PERTURB"
            check "perturb run reached the end frame" \
                "$(grep -q "end frame $END_FRAME reached" "$OUT/perturb.log" && echo 1)" "$(grep -o 'perturbed .*' "$OUT/perturb.log" | head -1)"
            ;;
    esac
done

# ---- comparisons -----------------------------------------------------------------------------------
cmp_traces() { # cmp_traces A B → satool diff output into OUT/A-vs-B.txt, echo exit status
    "$SATOOL" diff "$OUT/$1.trace" "$OUT/$2.trace" > "$OUT/$1-vs-$2.txt" 2>&1
    echo $?
}
full_len() { # the trace covers every frame 1..END_FRAME
    "$SATOOL" show "$OUT/$1.trace" | awk -v e="$END_FRAME" 'NR > 1 { n++; if ($1 != n) bad = 1 } END { exit !(n == e && !bad) }'
}
for pair in "replay1 replay2" "record replay1" "record replay2"; do
    set -- $pair
    [ -f "$OUT/$1.trace" ] && [ -f "$OUT/$2.trace" ] || continue
    rc="$(cmp_traces "$1" "$2")"
    ok=0
    [ "$rc" = 0 ] && full_len "$1" && full_len "$2" && ok=1
    check "$1 vs $2 identical for all $END_FRAME frames" "$ok" "$(tail -1 "$OUT/$1-vs-$2.txt")"
    [ "$ok" = 1 ] || sed 's/^/    /' "$OUT/$1-vs-$2.txt" | tee -a "$SUMMARY"
done
if [ -f "$OUT/replay1.trace" ] && [ -f "$OUT/perturb.trace" ]; then
    cmp_traces replay1 perturb > /dev/null
    first="$(sed -n 's/^DIVERGED at frame \([0-9]*\): \(.*\)$/\1 \2/p' "$OUT/replay1-vs-perturb.txt")"
    check "1-ulp player position change at frame $PERTURB detected exactly there in peds" \
        "$([ "$first" = "$PERTURB peds" ] && echo 1)" "$(tail -1 "$OUT/replay1-vs-perturb.txt")"
    sed 's/^/    /' "$OUT/replay1-vs-perturb.txt" >> "$SUMMARY"
fi
crash="$(grep -E -c 'Unhandled exception|page fault|PANIC' "$OUT"/*.log 2> /dev/null | awk -F: '{ s += $2 } END { print s + 0 }')"
check "no crash / panic in logs" "$([ "$crash" = 0 ] && echo 1)" "$crash crash lines"
exit $((FAILS > 0))
