#!/usr/bin/env bash
# Records an input program (tools/e2e/programs/*.txt) as a corpus recording, in determinism mode from
# process start, all hooks on the original (SA_HOOK_PORTS=orig), with screenshots along the way.
#
#   tools/e2e/record-program.sh PROGRAM [--shot-every FRAMES] [--replay]
#
# --replay: replays the new recording once and checks that its trace equals the recording's.
# Env: LANE (default 2), SA_ROOT, RUN_TIMEOUT (s, default 1200). Build first (see inf4-determinism.sh).
# Output: target/e2e/lane-N/rec-NAME/ (rec.sarec, record.trace, record.log, shot-FRAME.png, summary.txt).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
PROGRAM=""
EVERY=200
REPLAY=0
RUN_TIMEOUT="${RUN_TIMEOUT:-1200}"
while [ $# -gt 0 ]; do
    case "$1" in
        --shot-every) EVERY="$2"; shift 2 ;;
        --replay) REPLAY=1; shift ;;
        -*) echo "usage: $0 PROGRAM [--shot-every FRAMES] [--replay]" >&2; exit 2 ;;
        *) PROGRAM="$1"; shift ;;
    esac
done
[ -f "$PROGRAM" ] || { echo "usage: $0 PROGRAM [--shot-every FRAMES] [--replay]" >&2; exit 2; }
PROGRAM="$(cd "$(dirname "$PROGRAM")" && pwd)/$(basename "$PROGRAM")"
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
SATOOL="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release/satool"
"$SATOOL" program "$PROGRAM" > /dev/null || exit 1
END_FRAME="$(sed -n 's/^end \([0-9]*\).*/\1/p' "$PROGRAM" | tail -1)"
NAME="$(basename "$PROGRAM" .txt)"
OUT="$E2E_OUT/rec-$NAME"
mkdir -p "$OUT"
FAILS=0
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
check() {
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %s: %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
finish() { e2e_cleanup; e2e_say "$FAILS failure(s); summary $SUMMARY"; }

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

run_game() { # run_game NAME VAR=VALUE...
    local name="$1" start last="" st f next=0 kv
    shift
    rm -f "$DET/$name.trace" "$GTA_DIR/sa-hook.log"
    (
        export SA_DET=1 SA_DET_EXIT=1 SA_DET_TRACE="$(win "$DET/$name.trace")" SA_HOOK_PORTS=orig
        for kv in "$@"; do export "${kv?}"; done
        e2e_game_start
        wait "$E2E_GAME_PID"
    ) &
    local pid=$!
    e2e_say "run $name"
    start=$(date +%s)
    while kill -0 "$pid" 2> /dev/null; do
        st="$(e2e_ctl det)"
        [ -n "$st" ] && last="$st"
        f="$(printf '%s' "$st" | sed -n 's/.* frame=\([0-9]*\) .*/\1/p')"
        if [ "$name" = record ] && [ -n "$f" ] && [ "$f" -ge "$next" ]; then
            e2e_shot "rec-$NAME/shot-$(printf '%05d' "$f")" > /dev/null
            next=$((f + EVERY))
        fi
        if [ $(($(date +%s) - start)) -gt "$RUN_TIMEOUT" ]; then
            e2e_say "run $name: timeout; $last"
            e2e_ctl stack > "$OUT/$name.stack"
            break
        fi
        sleep "${REC_POLL:-1}"   # REC_POLL=0.2: denser screenshots while tuning a program
    done
    wineserver -k 2> /dev/null || true
    wait "$pid" 2> /dev/null
    e2e_saves_collect "$OUT/$name"
    cp -f "$GTA_DIR/sa-hook.log" "$OUT/$name.log" 2> /dev/null
    mv -f "$DET/$name.trace" "$OUT/$name.trace" 2> /dev/null
    printf '%s\n' "$last" > "$OUT/$name.det"
}

e2e_headless_start || exit 1
rm -f "$DET/rec.sarec"
run_game record SA_DET_PROGRAM="$(win "$DET/program.txt")" SA_DET_RECORD="$(win "$DET/rec.sarec")"
cp -f "$DET/rec.sarec" "$OUT/rec.sarec" 2> /dev/null
check "record run reached the end frame" "$(grep -q "end frame $END_FRAME reached" "$OUT/record.log" && echo 1)" \
    "$(tail -1 "$OUT/record.det")"
check "recording written" "$([ -s "$OUT/rec.sarec" ] && echo 1)" "$("$SATOOL" rec "$OUT/rec.sarec" 2>&1 | tail -1)"
if [ "$REPLAY" = 1 ]; then
    cp -f "$OUT/rec.sarec" "$DET/replay.sarec"
    run_game replay SA_DET_REPLAY="$(win "$DET/replay.sarec")"
    "$SATOOL" diff "$OUT/record.trace" "$OUT/replay.trace" > "$OUT/record-vs-replay.txt" 2>&1
    check "replay == record" "$([ $? = 0 ] && echo 1)" "$(tail -1 "$OUT/record-vs-replay.txt")"
fi
exit $((FAILS > 0))
