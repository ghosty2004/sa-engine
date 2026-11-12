#!/usr/bin/env bash
# SAMP-START reference: records the original startup of gta_sa.exe 1.0 US + samp.dll 0.3.7-R4 (no sa_hook)
# from launch to spawn against the private open.mp server running tools/e2e/samp/sampcov.pwn (3 classes,
# class camera at 2495,-1693,15 looking at 2495,-1687,13.5).
#
#   LANE=2 SA_ROOT=~/sa-engine E2E_BIN=~/sa-engine/target/i686-pc-windows-gnu/release \
#       tools/e2e/samp-start-ref.sh [OUTDIR]
#
# Frames (gamescope screenshots of the lane's headless display, about every 200 ms) go to OUTDIR/frames
# (default target/samp-start-evidence/ref), named by seconds since launch. During class selection it
# presses Right twice, Left once and then Shift to spawn; steps.log has the time of each step.
# Nothing is committed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
SAMP_PORT="${SAMP_PORT:-$((7830 + LANE))}"
CLIENT_SRC="$SA_ROOT/re/samp/client"
SERVER_SRC="$SA_ROOT/re/samp/server"
OUT="${1:-$E2E_SCRIPT_ROOT/target/samp-start-evidence/ref}"
SRV="$E2E_OUT/sampstart/server"
SLOG="$OUT/server.log"
rm -rf "$OUT/frames"
mkdir -p "$OUT/frames" "$SRV"
[ -x "$SERVER_SRC/omp-server" ] || { echo "samp-start-ref: $SERVER_SRC/omp-server missing" >&2; exit 1; }

e2e_lane_lock
e2e_disk_check || exit 1
e2e_lane_setup || exit 1
e2e_install || exit 1
T0=$(date +%s%N)
since() { echo $((($(date +%s%N) - T0) / 1000000)); }
say() { e2e_say "$*"; printf '%7d ms %s\n' "$(since)" "$*" >> "$OUT/steps.log"; }
: > "$OUT/steps.log"

FILES="samp.dll samp.saa bass.dll sampgui.png mouse.png sampaux3.ttf SAMP"
for f in $FILES; do
    [ -e "$CLIENT_SRC/$f" ] || continue
    if [ -e "$GTA_DIR/$f" ] && [ ! -L "$GTA_DIR/$f" ]; then
        echo "samp-start-ref: $GTA_DIR/$f exists and is not a symlink; refusing" >&2; exit 1
    fi
    ln -sfn "$CLIENT_SRC/$f" "$GTA_DIR/$f"
done
USERDIR="$WINEPREFIX/drive_c/users/$USER/Documents/GTA San Andreas User Files"
mkdir -p "$USERDIR/SAMP"
printf 'fpslimit=50\r\nmulticore=1\r\naudioproxyoff=0\r\ntimestamp=0\r\n' > "$USERDIR/SAMP/sa-mp.cfg"

rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/samp/sampcov.pwn" -iqawno/include \
    -ogamemodes/sampcov.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$SAMP_PORT" <<'PY'
import json, sys
p, port = sys.argv[1], int(sys.argv[2])
d = json.load(open(p))
d["announce"] = False
d["name"] = "SA-MP startup reference"
d["network"]["port"] = port
d["artwork"]["enable"] = False
d["rcon"]["enable"] = False
d["pawn"]["main_scripts"] = ["sampcov 1"]
d["pawn"]["side_scripts"] = []
json.dump(d, open(p, "w"), indent=4)
PY
SRV_PID=
CAP_PID=
server_stop() {
    [ -n "$SRV_PID" ] && kill "$SRV_PID" 2> /dev/null
    [ -n "$SRV_PID" ] && wait "$SRV_PID" 2> /dev/null
    SRV_PID=
}
wait_log() { # REGEX SECS
    local i
    for ((i = 0; i < $2 * 5; i++)); do
        grep -qE "$1" "$SLOG" 2> /dev/null && return 0
        if [ -n "${E2E_GAME_PID:-}" ] && ! kill -0 "$E2E_GAME_PID" 2> /dev/null; then return 1; fi
        sleep 0.2
    done
    return 1
}
if ss -lun | grep -q ":$SAMP_PORT "; then echo "samp-start-ref: UDP $SAMP_PORT busy" >&2; exit 1; fi
( cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1 ) &
SRV_PID=$!
wait_log 'Legacy Network started' 20 || { echo "samp-start-ref: server did not start" >&2; server_stop; exit 1; }
say "server pid $SRV_PID on UDP $SAMP_PORT"

finish() {
    say "teardown"
    [ -n "$CAP_PID" ] && kill "$CAP_PID" 2> /dev/null
    e2e_cleanup
    server_stop
    for f in $FILES; do [ -L "$GTA_DIR/$f" ] && rm -f "$GTA_DIR/$f"; done
}
trap finish EXIT

e2e_headless_start || exit 1
capture() {
    local f
    while :; do
        f="$OUT/frames/$(printf '%06d' "$(since)").png"
        GAMESCOPE_WAYLAND_DISPLAY="$E2E_GAMESCOPE" gamescopectl screenshot "$f" > /dev/null 2>&1
        sleep 0.2
    done
}
T0=$(date +%s%N)
(
    cd "$GTA_DIR" || exit 1
    exec env -u WAYLAND_DISPLAY DISPLAY="$E2E_DISPLAY" wine "$GTA_DIR/sa-launcher.exe" \
        --gta "$(winepath -w "$GTA_DIR/gta_sa.exe")" \
        --dll "$(winepath -w "$GTA_DIR/samp.dll")" \
        -- -c -h 127.0.0.1 -p "$SAMP_PORT" -n RefPlayer
) > "$OUT/wine.log" 2>&1 &
E2E_GAME_PID=$!
capture &
CAP_PID=$!
say "game started (pid $E2E_GAME_PID)"

wait_log "\[cov\] connect id=[0-9]+ name=RefPlayer" 180 || { say "FAIL: no connect"; exit 1; }
say "server: connect"
wait_log "\[cov\] class id=" 90 || { say "FAIL: no class selection"; exit 1; }
say "server: class selection"
sleep 4
say "key Right"; e2e_key Right; sleep 2.5
say "key Right"; e2e_key Right; sleep 2.5
say "key Left"; e2e_key Left; sleep 2.5
spawned=0
for i in 1 2 3 4 5 6; do
    say "key shift"
    e2e_key shift
    if wait_log "\[cov\] spawn id=" 4; then spawned=1; break; fi
done
[ "$spawned" = 1 ] || { say "FAIL: no spawn"; exit 1; }
say "server: spawn"
sleep 5
say "done"
