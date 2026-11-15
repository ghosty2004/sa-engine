#!/bin/sh
# FRONTIER-1 scenarios: native sa-game runs (deterministic, --fixed-step 40, offscreen) that reach the blockers the
# user hit. Each must end with an empty blocker list in OUT/report.md and no crash.
#
#   tools/e2e/scenarios/frontier.sh pausemenu|solicit|crossing|melee OUTDIR [FRAMES]
#
# pausemenu (400)  Esc, down x3, Enter (pause-menu item 4: Stats), down, Enter, Enter: the Stats pages draw
#                  (was a SIGSEGV at iteration ~171: CStats::GetStatValue's X87 result dispatched as cdecl)
# solicit  (4000)  PEDHIT's car run without leaving the car: the prostitutes' CTaskComplexProstituteSolicit
#                  (ControlSubTask 0x6669D0) and IsEntityBlockingTarget 0x5F3970 run
# crossing (1200)  the Greenwood taken to the level crossing south of Grove Street (2200, -1895; traincross1/2) and
#                  driven over the tracks: CObject::ProcessTrainCrossingBehaviour 0x5A0B50 for the barriers; the
#                  trailers there (CTrailer 0x6CF1A0 / 0x6CF590 / 0x6CF6A0 / 0x6CFAC0, UpdateMovingCollision 0x6A1460)
#                  and the damage the car does (CVehicle::InflictDamage 0x6D7C90) bring the police: FRONTIER-2 stops on
#                  `update_car_ai_original (0x0)` (CCarAI::UpdateCarAI's law-enforcer path, not ported yet)
# melee    (2600)  on foot at Grove Street, CJ sprints into the peds and punches (LMB): CTaskSimpleFight::FightHitPed,
#                  the damage anim / hit sounds (0x4B3FC0, 0x4E1CC0) and the victims' fight back (0x62A0A0, 0x69D170):
#                  0 blocked since FRONTIER-2
# WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier2-counts.gdb --args" counts the FRONTIER-2 ports' calls.
# Env: SA_ROOT (checkout holding wine/gta-sa; default ~/sa-engine), SA_GAME (binary; default this checkout's
# i686-unknown-linux-gnu release build), WRAP (a command prefix, e.g. WRAP="gdb -q -batch -x tools/e2e/scenarios/frontier-counts.gdb --args"
# to count the ports' calls), plus any SA_GAME_* (SA_GAME_PRESENT_GAME=1 + PRESENT_DIR for frames).
set -eu
HERE=$(cd "$(dirname "$0")/../../.." && pwd)
NAME=$1; OUT=$2
B=${SA_GAME:-$HERE/target/i686-unknown-linux-gnu/release/sa-game}
GAME=${SA_ROOT:-$HOME/sa-engine}/wine/gta-sa
mkdir -p "$OUT"
export GLIBC_TUNABLES=glibc.malloc.mmap_max=0
run() { # run FRAMES [args]
    n=$1; shift
    if [ -n "${PRESENT_DIR:-}" ]; then set -- "$@" --present-dir "$PRESENT_DIR"; fi
    # shellcheck disable=SC2086
    exec systemd-run --user --scope -q -p MemoryMax=6G setarch i686 -R ${WRAP:-} "$B" --game "$GAME" --fixed-step 40 \
        --frames "$n" --report "$OUT/report.md" "$@" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
}
case $NAME in
pausemenu)
    export SA_GAME_SCRIPT_SKIP=1
    export SA_GAME_KEYS="esc:100:102,down:130:132,down:136:138,down:142:144,enter:170:172,down:200:202,enter:210:212,enter:240:242"
    run "${3:-400}" --start-in-world ;;
solicit)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2470.0:-1665.0:13.6:-2.246;400:2470.0:-1665.0:13.6:-2.364;530:2497.35:-1678.6:13.6:3.1416"
    export SA_GAME_KEYS=f:100:103,w:301:360,w:401:450,w:531:553
    run "${3:-4000}" --start-in-world ;;
crossing)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2200.6:-1850.0:13.6:3.1416"
    export SA_GAME_KEYS=f:100:103,w:330:420,s:520:560
    run "${3:-1200}" --start-in-world ;;
melee)
    # the PEDHIT foot run's sprint into the Grove Street peds (they are next to CJ at ~905), then punches every 25
    # iterations: CTaskSimpleFight::FightHitPed lands (7 hits by 2600); stops on HandlePedHit 0x4E1CC0 and
    # CEventDamage::ComputeDamageAnim 0x4B3FC0 (not ported yet)
    export SA_GAME_SCRIPT_SKIP=1 SA_GAME_KEYS="w:100:910,space:120:900"
    export SA_GAME_MOUSE="15:0:200:400,-20:3:500:700,25:-2:800:1000"
    export SA_GAME_LMB="${SA_GAME_LMB:-$(i=905; s=; while [ $i -lt 2400 ]; do s="$s${s:+,}$i:$((i + 4))"; i=$((i + 25)); done; echo "$s")}"
    run "${3:-2600}" --start-in-world --system-heap ;;
*)
    echo "usage: $0 pausemenu|solicit|crossing|melee OUTDIR [FRAMES]" >&2
    exit 2 ;;
esac
