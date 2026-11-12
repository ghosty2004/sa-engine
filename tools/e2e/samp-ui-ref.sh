#!/usr/bin/env bash
# SAMP-UI reference screenshots: the original gta_sa.exe 1.0 US + samp.dll 0.3.7-R4 (no sa_hook) against the
# private open.mp server with tools/e2e/samp/sampui.pwn, which shows fixed UI content (chat, nametag + bars of
# an NPC, a 3D label, textdraws, game text, kill list, every dialog style, scoreboard).
#
#   LANE=2 SA_ROOT=~/sa-engine E2E_RES=1280x720 tools/e2e/samp-ui-ref.sh [OUTDIR]
#
# Inputs as samp-cov.sh (re/samp/client, re/samp/server; gitignored). Screenshots (gamescope's own, of the
# lane's headless display) go to OUTDIR (default target/samp-ui2-evidence/ref-WxH). Nothing is committed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
SAMP_PORT="${SAMP_PORT:-$((7820 + LANE))}"
CLIENT_SRC="$SA_ROOT/re/samp/client"
SERVER_SRC="$SA_ROOT/re/samp/server"
OUT="${1:-$E2E_SCRIPT_ROOT/target/samp-ui2-evidence/ref-$E2E_RES}"
SRV="$E2E_OUT/sampui/server"
SLOG="$OUT/server.log"
mkdir -p "$OUT" "$SRV"
[ -x "$SERVER_SRC/omp-server" ] || { echo "samp-ui-ref: $SERVER_SRC/omp-server missing" >&2; exit 1; }

e2e_lane_lock
e2e_disk_check || exit 1
e2e_lane_setup || exit 1
e2e_install || exit 1
say() { e2e_say "$*"; echo "[$(date +%T)] $*" >> "$OUT/steps.log"; }
: > "$OUT/steps.log"

FILES="samp.dll samp.saa bass.dll sampgui.png mouse.png sampaux3.ttf SAMP"
for f in $FILES; do
    [ -e "$CLIENT_SRC/$f" ] || continue
    if [ -e "$GTA_DIR/$f" ] && [ ! -L "$GTA_DIR/$f" ]; then
        echo "samp-ui-ref: $GTA_DIR/$f exists and is not a symlink; refusing" >&2; exit 1
    fi
    ln -sfn "$CLIENT_SRC/$f" "$GTA_DIR/$f"
done
USERDIR="$WINEPREFIX/drive_c/users/$USER/Documents/GTA San Andreas User Files"
mkdir -p "$USERDIR/SAMP"
printf 'fpslimit=50\r\nmulticore=1\r\naudioproxyoff=0\r\ntimestamp=0\r\n' > "$USERDIR/SAMP/sa-mp.cfg"

rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/samp/sampui.pwn" -iqawno/include \
    -ogamemodes/sampui.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$SAMP_PORT" <<'PY'
import json, sys
p, port = sys.argv[1], int(sys.argv[2])
d = json.load(open(p))
d["announce"] = False
d["name"] = "SA-MP UI reference"
d["network"]["port"] = port
d["artwork"]["enable"] = False
d["rcon"]["enable"] = False
d["pawn"]["main_scripts"] = ["sampui 1"]
d["pawn"]["side_scripts"] = []
d["max_bots"] = 4
json.dump(d, open(p, "w"), indent=4)
PY
SRV_PID=
server_stop() {
    [ -n "$SRV_PID" ] && kill "$SRV_PID" 2> /dev/null
    [ -n "$SRV_PID" ] && wait "$SRV_PID" 2> /dev/null
    SRV_PID=
}
wait_log() { # REGEX SECS
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        grep -qE "$1" "$SLOG" 2> /dev/null && return 0
        if [ -n "${E2E_GAME_PID:-}" ] && ! kill -0 "$E2E_GAME_PID" 2> /dev/null; then return 1; fi
        sleep 0.5
    done
    return 1
}
if ss -lun | grep -q ":$SAMP_PORT "; then echo "samp-ui-ref: UDP $SAMP_PORT busy" >&2; exit 1; fi
( cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1 ) &
SRV_PID=$!
wait_log 'Legacy Network started' 20 || { echo "samp-ui-ref: server did not start" >&2; server_stop; exit 1; }
say "server pid $SRV_PID on UDP $SAMP_PORT"

finish() {
    say "teardown"
    e2e_cleanup
    server_stop
    for f in $FILES; do [ -L "$GTA_DIR/$f" ] && rm -f "$GTA_DIR/$f"; done
}
trap finish EXIT

e2e_headless_start || exit 1
(
    cd "$GTA_DIR" || exit 1
    exec env -u WAYLAND_DISPLAY DISPLAY="$E2E_DISPLAY" wine "$GTA_DIR/sa-launcher.exe" \
        --gta "$(winepath -w "$GTA_DIR/gta_sa.exe")" \
        --dll "$(winepath -w "$GTA_DIR/samp.dll")" \
        -- -c -h 127.0.0.1 -p "$SAMP_PORT" -n RefPlayer
) > "$OUT/wine.log" 2>&1 &
E2E_GAME_PID=$!
say "game started (pid $E2E_GAME_PID)"

SEQ=0
CMDF="$SRV/scriptfiles/sampui.txt"
cmd() {
    SEQ=$((SEQ + 1))
    echo "$SEQ $1" >> "$CMDF"
    wait_log "\[ui\] $SEQ (done|unknown)" 10 || say "command $SEQ ($1) not acknowledged"
}
shot() { e2e_shot "sampui-$1" > /dev/null && mv -f "$E2E_OUT/sampui-$1.png" "$OUT/$1.png"; say "shot $1"; }
alive() { e2e_game_alive || { say "GAME EXITED"; exit 1; }; }
mouse() { DISPLAY="$E2E_DISPLAY" xdotool mousemove "$1" "$2"; }

wait_log "\[ui\] connect id=[0-9]+ name=RefPlayer" 180 || { shot fail-connect; say "FAIL: no connect"; exit 1; }
wait_log "\[ui\] class id=" 90 || { shot fail-class; say "FAIL: no class selection"; exit 1; }
sleep 3
spawned=0
for i in 1 2 3 4 5 6 7 8; do
    e2e_key shift
    if wait_log "\[ui\] spawn id=" 4; then spawned=1; break; fi
done
[ "$spawned" = 1 ] || { shot fail-spawn; say "FAIL: no spawn"; exit 1; }
sleep 4
alive
cmd "scene"
sleep 4
shot 01-scene
W="${E2E_RES%x*}"; H="${E2E_RES#*x}"
e2e_key t; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "hello world"; sleep 0.5
shot 02-chat-input
e2e_key Escape; sleep 0.5
for s in 0 1 2 3 4 5; do
    alive
    cmd "dialog|$s"
    sleep 1.5
    mouse $((W - 60)) $((H - 60))
    case $s in
        1) DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "RefPlayer" ;;
        3) DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "secret" ;;
        2 | 4 | 5) e2e_key Down; sleep 0.2; e2e_key Down ;;
    esac
    sleep 1
    shot "03-dialog-style$s"
    e2e_key Escape
    wait_log "dialog-response dialog=1$s " 5 || say "no response to dialog style $s"
    sleep 0.5
done
mouse $((W / 2 + 40)) $((H / 2 - 40))
e2e_key Tab; sleep 1.5
shot 04-scoreboard
e2e_key Tab; sleep 1
e2e_key F7; sleep 1
shot 05-chat-plain
say "done"
