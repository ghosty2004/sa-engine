#!/usr/bin/env bash
# Soak and crash hunt for the samp mod: long headless runs of sa-game (offscreen, `--features mods`) joined to our
# private open.mp copy (re/samp/server, 127.0.0.1 only, never a public server) through mod-join.sh, one run per
# gamemode, each with a random command list from chaos.py running beside the gamemode. Every crash leaves sa-game's
# report (fault, Rust call chain, last mod log lines) in RUN/crashes/.
#
#   tools/e2e/samp/soak.sh [OUT_DIR]          (default target/samp-soak)
#
# Env: RUNS (default "sampcov gungame derby simpletdm": sampcov is our own gamemode, the others are the server's, with
# sampcov.pwn as a filterscript), MINUTES per run (default 15), SEED (default 1; run k uses SEED + k), CHAOS=0 runs the
# gamemodes without the command list, FILTERSCRIPTS (default npc_trains), SAMP_PORT (default 7923), LIST_PORT (default
# 7924), SAMP_LOCK (default crashhunt.lock), SA_GAME / MODS / SA_ROOT as mod-join.sh takes them.
# Output: OUT/RUN/ (mod-join.sh's files; the present frames and the server copy are removed at the end, the logs
# kept), OUT/summary.txt with one line per run: CLEAN or CRASH (the exit status, the crash reports, panics). Exit
# status 0 only when every run is clean.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
OUT="$(mkdir -p "${1:-$ROOT/target/samp-soak}" && cd "${1:-$ROOT/target/samp-soak}" && pwd)"
MINUTES="${MINUTES:-15}"
SEED="${SEED:-1}"
export SAMP_PORT="${SAMP_PORT:-7923}" LIST_PORT="${LIST_PORT:-7924}" SAMP_LOCK="${SAMP_LOCK:-crashhunt.lock}"
: > "$OUT/summary.txt"
k=0
status=0
for run in ${RUNS:-sampcov gungame derby simpletdm}; do
    k=$((k + 1))
    d="$OUT/$run"
    rm -rf "$d"
    mkdir -p "$d"
    gamemode=$run
    [ "$run" = sampcov ] && gamemode=
    cmds=/dev/null
    if [ "${CHAOS:-1}" = 1 ]; then
        cmds="$d/chaos.cmds"
        python3 -I "$HERE/chaos.py" $((SEED + k)) 40000 > "$cmds"
    fi
    echo "[$(date +%T)] $run: $MINUTES min, seed $((SEED + k))"
    GAMEMODE="$gamemode" FILTERSCRIPTS="${FILTERSCRIPTS:-npc_trains}" CMDS="$cmds" SOAK_SECONDS=$((MINUTES * 60)) \
        KILL_AFTER=1 FRAMES=100000000 SA_GAME_PRESENT_EVERY="${SA_GAME_PRESENT_EVERY:-5000}" \
        "$HERE/mod-join.sh" "$d" > "$d/mod-join.log" 2>&1
    exit_line="$(grep -oE 'game exit [0-9]+' "$d/steps.log" | tail -1)"
    code="${exit_line##* }"
    reports="$(ls "$d/crashes" 2> /dev/null | wc -l)"
    panics="$(grep -c 'panicked' "$d/sa-game.log" 2> /dev/null)"
    commands="$(grep -cE '\[cov\] [0-9]+ (done|unknown)' "$d/server.log" 2> /dev/null)"
    # 143: the SIGTERM KILL_AFTER sends when the time is over
    if [ "$reports" = 0 ] && [ "${panics:-0}" = 0 ] && { [ "$code" = 0 ] || [ "$code" = 143 ]; }; then
        verdict=CLEAN
    else
        verdict=CRASH
        status=1
    fi
    line="$verdict $run: exit ${code:-?}, $reports crash reports, ${panics:-0} panics, $commands commands run"
    echo "$line" | tee -a "$OUT/summary.txt"
    for f in "$d"/crashes/*.txt; do
        [ -f "$f" ] && echo "    $(head -1 "$f")" | tee -a "$OUT/summary.txt"
    done
    rm -rf "$d/present" "$d/server/models" "$d/server/omp-server" "$d/server/components" "$d/server/qawno"
done
exit "$status"
