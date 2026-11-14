#!/bin/sh
# PEDHIT scenarios: native sa-game runs (deterministic, --fixed-step, offscreen) where vehicles and the player hit
# peds. Each must end with an empty blocker list in OUT/report.md.
#
#   tools/e2e/scenarios/pedhit.sh car|foot|intro OUTDIR [FRAMES]
#
# car    (2900) the Greenwood at Grove Street: two fast runs into the gang group on the south pavement (one bumped
#               at 375, one killed and one bumped at 467), a slow approach (531), out of the car at 620 (so the
#               prostitutes' solicit task, whose ControlSubTask 0x6669D0 is unported, does not start), then idle
# foot   (3500) the main session's repro: CJ sprints into peds with the camera turning (CEventPedCollisionWithPlayer),
#               then stands until the idle camera starts
# intro  (7300) New Game: INTRO-2's keys to the BMX; at 5020 the BMX is put on the pavement north of a standing ped
#               and pedalled into it (killed at 5082; on master a266cc61 INTRO-2's ride ends against a wall)
# Env: SA_ROOT (checkout holding wine/gta-sa; default ~/sa-engine), SA_GAME (binary; default this checkout's
# i686-unknown-linux-gnu release build), plus any SA_GAME_* (SA_GAME_PRESENT_SKIP / _EVERY, SA_GAME_VEHLIST=5 and
# SA_GAME_VEHLIST_PEDS=60 for the ped table).
set -eu
HERE=$(cd "$(dirname "$0")/../../.." && pwd)
NAME=$1; OUT=$2
B=${SA_GAME:-$HERE/target/i686-unknown-linux-gnu/release/sa-game}
GAME=${SA_ROOT:-$HOME/sa-engine}/wine/gta-sa
mkdir -p "$OUT"
export SA_GAME_PRESENT_GAME=${SA_GAME_PRESENT_GAME-1}
export GLIBC_TUNABLES=glibc.malloc.mmap_max=0
run() { # run FRAMES STEP [args]
    n=$1; step=$2; shift 2
    exec systemd-run --user --scope -q -p MemoryMax=6G setarch i686 -R "$B" --game "$GAME" --fixed-step "$step" \
        --frames "$n" --present-dir "$OUT" --report "$OUT/report.md" "$@" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
}
case $NAME in
car)
    export SA_GAME_TELEPORT="60:2505.0:-1666.4:13.4;300:2470.0:-1665.0:13.6:-2.246;400:2470.0:-1665.0:13.6:-2.364;530:2497.35:-1678.6:13.6:3.1416"
    export SA_GAME_KEYS=f:100:103,w:301:360,w:401:450,w:531:553,f:620:623
    run "${3:-2900}" 40 --start-in-world ;;
foot)
    export SA_GAME_SCRIPT_SKIP=1 SA_GAME_KEYS="w:100:1400,space:120:1400"
    export SA_GAME_MOUSE="15:0:200:400,-20:3:500:700,25:-2:800:1000,-10:0:1100:1300"
    run "${3:-3500}" 40 --start-in-world --system-heap ;;
intro)
    export SA_GAME_TELEPORT="5020:2262.6:-1276.0:24.2:3.1416"
    export SA_GAME_KEYS=w:4700:4728,d:4708:4714,f:4732:4738,w:4760:5000,d:4790:4812,w:5021:5090
    run "${3:-7300}" 40.5 ;;
*)
    echo "usage: $0 car|foot|intro OUTDIR [FRAMES]" >&2
    exit 2 ;;
esac
