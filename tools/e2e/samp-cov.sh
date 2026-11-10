#!/usr/bin/env bash
# SAMP-1D: run the original gta_sa.exe 1.0 US with samp.dll 0.3.7-R4 (and sa_hook.dll for coverage)
# against a private open.mp server, drive the docs/samp/README.md §6 scenarios, collect coverage.
#
#   LANE=4 SA_ROOT=~/sa-engine tools/e2e/samp-cov.sh [--no-cov] [--steps LIST] [--name NAME]
#
# --steps: comma list of scenario steps (default: all); see `steps_all` below.
# Inputs (gitignored, copied from ~/samp-rs, never used in place): $SA_ROOT/re/samp/client/ (samp.dll,
# samp.saa, SAMP/, bass.dll, sampgui.png, mouse.png, sampaux3.ttf), $SA_ROOT/re/samp/server/ (open.mp
# 1.6.7), $SA_ROOT/re/samp/gta-patches-r4-annotated.csv (samp.dll's patch list).
# The server copy runs from target/e2e/lane-N/samp/server on UDP 7810 with tools/e2e/samp/sampcov.pwn
# as gamemode. The lane's game dir gets symlinks to the client files (nothing in wine/gta-sa changes).
# Output: target/e2e/lane-N/samp/ (server.log, sa-hook.log, shots, cov-NAME.csv) and, with coverage,
# catalog/raw/coverage-samp-NAME.hits.csv.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
COV=1
STEPS=all
NAME=samp
while [ $# -gt 0 ]; do
    case "$1" in
        --no-cov) COV=0; shift ;;
        --steps) STEPS="$2"; shift 2 ;;
        --name) NAME="$2"; shift 2 ;;
        *) echo "usage: $0 [--no-cov] [--steps LIST] [--name NAME]" >&2; exit 2 ;;
    esac
done
LANE="${LANE:-4}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
SAMP_PORT="${SAMP_PORT:-7810}"
CLIENT_SRC="$SA_ROOT/re/samp/client"
SERVER_SRC="$SA_ROOT/re/samp/server"
PATCHES="$SA_ROOT/re/samp/gta-patches-r4-annotated.csv"
OUT="$E2E_OUT/samp"
SRV="$OUT/server"
SLOG="$OUT/server.log"
ORACLE="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release/sa-oracle"
PY="$SA_ROOT/.venv/bin/python"
mkdir -p "$OUT"
for f in samp.dll samp.saa bass.dll SAMP; do
    [ -e "$CLIENT_SRC/$f" ] || { echo "samp-cov: $CLIENT_SRC/$f missing" >&2; exit 1; }
done
[ -x "$SERVER_SRC/omp-server" ] || { echo "samp-cov: $SERVER_SRC/omp-server missing" >&2; exit 1; }

e2e_lane_lock
e2e_disk_check || exit 1
e2e_lane_setup || exit 1
e2e_install || exit 1
say() { e2e_say "$*"; echo "[$(date +%T)] $*" >> "$OUT/steps.log"; }
: > "$OUT/steps.log"

# ---- client files: lane-private symlinks (never inside wine/gta-sa) --------------------------------
for f in samp.dll samp.saa bass.dll sampgui.png mouse.png sampaux3.ttf SAMP; do
    [ -e "$CLIENT_SRC/$f" ] || continue
    if [ -e "$GTA_DIR/$f" ] && [ ! -L "$GTA_DIR/$f" ]; then
        echo "samp-cov: $GTA_DIR/$f exists and is not a symlink; refusing" >&2; exit 1
    fi
    ln -sfn "$CLIENT_SRC/$f" "$GTA_DIR/$f"
done
USERDIR="$WINEPREFIX/drive_c/users/$USER/Documents/GTA San Andreas User Files"
mkdir -p "$USERDIR/SAMP"
printf 'fpslimit=50\r\nmulticore=1\r\naudioproxyoff=0\r\ntimestamp=0\r\n' > "$USERDIR/SAMP/sa-mp.cfg"

# ---- server ------------------------------------------------------------------------------------------
mkdir -p "$SRV"
rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/samp/sampcov.pwn" -iqawno/include \
    -ogamemodes/sampcov.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$SAMP_PORT" <<'PY'
import json, sys
p, port = sys.argv[1], int(sys.argv[2])
d = json.load(open(p))
d["announce"] = False
d["network"]["port"] = port
d["artwork"]["enable"] = False
d["rcon"]["enable"] = False
d["pawn"]["main_scripts"] = ["sampcov 1"]
d["pawn"]["side_scripts"] = []
d["game"]["use_entry_exit_markers"] = True
d["max_bots"] = 4
json.dump(d, open(p, "w"), indent=4)
PY
SRV_PID=
server_stop() {
    [ -n "$SRV_PID" ] && kill "$SRV_PID" 2> /dev/null
    [ -n "$SRV_PID" ] && wait "$SRV_PID" 2> /dev/null
    SRV_PID=
}
wait_log() { # REGEX SECS (gives up early when the game has exited)
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        grep -qE "$1" "$SLOG" 2> /dev/null && return 0
        if [ -n "${E2E_GAME_PID:-}" ] && ! kill -0 "$E2E_GAME_PID" 2> /dev/null; then return 1; fi
        sleep 0.5
    done
    return 1
}
if ss -lun | grep -q ":$SAMP_PORT "; then echo "samp-cov: UDP $SAMP_PORT busy" >&2; exit 1; fi
( cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1 ) &
SRV_PID=$!
wait_log 'Legacy Network started' 20 || { echo "samp-cov: server did not start" >&2; cat "$SLOG"; server_stop; exit 1; }
say "server pid $SRV_PID on UDP $SAMP_PORT"

finish() {
    say "teardown"
    e2e_cleanup
    server_stop
    cp -f "$GTA_DIR/sa-hook.log" "$OUT/sa-hook.log" 2> /dev/null
    cp -f "$GTA_DIR/sa-launcher.log" "$OUT/sa-launcher.log" 2> /dev/null
    [ -f "$GTA_DIR/coverage-samp.csv" ] && cp -f "$GTA_DIR/coverage-samp.csv" "$OUT/cov-$NAME.csv"
    [ -f "$GTA_DIR/coverage-samp.csv.last" ] && cp -f "$GTA_DIR/coverage-samp.csv.last" "$OUT/cov-$NAME.csv.last"
    [ -f "$GTA_DIR/coverage-samp-scm.csv" ] && cp -f "$GTA_DIR/coverage-samp-scm.csv" "$OUT/scm-$NAME.csv"
    rm -f "$GTA_DIR/coverage-samp.csv" "$GTA_DIR/coverage-samp.csv.last" "$GTA_DIR/coverage-samp-scm.csv"
    audio_restore
    # Leave the lane as found: drop the client symlinks again.
    for f in samp.dll samp.saa bass.dll sampgui.png mouse.png sampaux3.ttf SAMP; do
        [ -L "$GTA_DIR/$f" ] && rm -f "$GTA_DIR/$f"
    done
}
trap finish EXIT

# Steam 3.0 audio data under the 1.0 US exe: a radio station that starts on one of the zero-sample
# stub tracks divides by zero on the stream thread (0x4F1464, exit code 0xC0000094; samp-rs
# docs/phase3/soak-notes.md; seen here in the first full run). For this run only the lane gets a private
# audio/CONFIG whose TrakLkup.dat aliases the stubs (samp-rs tools/audio-stub-tracks.py, copied to
# re/samp/tools). `finish` puts the lane's audio symlink back, so other runs on the lane see the
# shared data unchanged. SAMPCOV_AUDIO_STUBS=1 keeps the shared data.
AUDIO_FIX="$SA_ROOT/re/samp/tools/audio-stub-tracks.py"
audio_private() {
    [ "${SAMPCOV_AUDIO_STUBS:-0}" != 1 ] && [ -f "$AUDIO_FIX" ] && [ -L "$GTA_DIR/audio" ] || return 0
    local shared
    shared="$(readlink "$GTA_DIR/audio")"
    rm "$GTA_DIR/audio" && mkdir -p "$GTA_DIR/audio/CONFIG" || return 1
    echo "$shared" > "$GTA_DIR/audio/.shared"
    ln -s "$shared/sfx" "$GTA_DIR/audio/sfx"
    ln -s "$shared/streams" "$GTA_DIR/audio/streams"
    cp "$shared"/CONFIG/* "$GTA_DIR/audio/CONFIG/"
    python3 -I "$AUDIO_FIX" patch "$GTA_DIR" > "$OUT/audio-fix.log" 2>&1 || cat "$OUT/audio-fix.log"
    say "private audio/CONFIG: $(tail -1 "$OUT/audio-fix.log")"
}
audio_restore() {
    [ -f "$GTA_DIR/audio/.shared" ] || return 0
    local shared
    shared="$(cat "$GTA_DIR/audio/.shared")"
    rm -rf "${GTA_DIR:?}/audio" && ln -s "$shared" "$GTA_DIR/audio"
}
audio_private || exit 1

# ---- coverage points ---------------------------------------------------------------------------------
if [ "$COV" = 1 ]; then
    if [ -n "${SAMPCOV_POINTS:-}" ]; then
        cp -f "$SAMPCOV_POINTS" "$OUT/points.txt"    # debugging: a given points file
    else
        "$ORACLE" coverage points --out "$OUT/points-all.txt" 2> "$OUT/points.log" || { cat "$OUT/points.log"; exit 1; }
        "$PY" -I "$HERE/samp/samp-points.py" "$OUT/points-all.txt" "$PATCHES" "$CLIENT_SRC/samp.dll" \
            "$OUT/points.txt" "$OUT/excluded.csv" | tee -a "$OUT/steps.log"
    fi
fi
unset SA_COV SA_COV_OUT SA_COV_CAP SA_COV_SCM SA_DET SA_HOOK_PORTS SA_HOOK_MODE

# ---- game ----------------------------------------------------------------------------------------------
# The display first: the first Wine process of the prefix (winepath too) starts the Wine session,
# which keeps the display it saw; without one the game's CreateWindowExA fails and it exits.
e2e_headless_start || exit 1
if [ "$COV" = 1 ]; then
    export SA_COV="$(winepath -w "$OUT/points.txt")"
    export SA_COV_OUT="$(winepath -w "$GTA_DIR/coverage-samp.csv")"
    export SA_COV_SCM="$(winepath -w "$GTA_DIR/coverage-samp-scm.csv")"
    export SA_COV_CAP=1   # cap > 1 (re-arm by single step) hung in early init under samp (measured, cap 64)
fi
rm -f "$GTA_DIR/sa-hook.log" "$GTA_DIR/sa-launcher.log"
(
    cd "$GTA_DIR" || exit 1
    exec env -u WAYLAND_DISPLAY DISPLAY="$E2E_DISPLAY" wine "$GTA_DIR/sa-launcher.exe" \
        --gta "$(winepath -w "$GTA_DIR/gta_sa.exe")" \
        --dll "$(winepath -w "$GTA_DIR/sa_hook.dll")" \
        $([ -n "${SAMPCOV_NO_SAMP:-}" ] || echo --dll "$(winepath -w "$GTA_DIR/samp.dll")") \
        -- -c -h 127.0.0.1 -p "$SAMP_PORT" -n CovPlayer
) > "$OUT/wine.log" 2>&1 &
E2E_GAME_PID=$!
say "game started (pid $E2E_GAME_PID)"

# ---- scenario helpers ---------------------------------------------------------------------------------
SEQ=0
CMDF="$SRV/scriptfiles/sampcov.txt"
cmd() { # "CMD|ARG|..." -> waits for "[cov] SEQ done"
    SEQ=$((SEQ + 1))
    echo "$SEQ $1" >> "$CMDF"
    wait_log "\[cov\] $SEQ (done|unknown)" 10 || say "command $SEQ ($1) not acknowledged"
}
cmd_id() { # prints the id the gamemode printed for command SEQ ("[cov] SEQ veh ID")
    grep -oE "\[cov\] $SEQ [a-z]+ -?[0-9]+" "$SLOG" | tail -1 | awk '{print $4}'
}
hold() { e2e_keydown "$1"; sleep "$2"; e2e_keyup "$1"; }
alive() { e2e_game_alive || { say "GAME EXITED"; exit 1; }; }
shot() { e2e_shot "samp/$1" > /dev/null; }
mark() { say "== $*"; alive; }
step_on() { [ "$STEPS" = all ] || [[ ",$STEPS," == *",$1,"* ]]; }

# ---- scenario -----------------------------------------------------------------------------------------
wait_log "\[cov\] connect id=[0-9]+ name=CovPlayer" 180 || { shot fail-connect; say "FAIL: no connect"; exit 1; }
mark "connected"
wait_log "\[cov\] class id=" 90 || { shot fail-class; say "FAIL: no class selection"; exit 1; }
mark "class selection"
sleep 3
e2e_key Right; sleep 1.5; e2e_key Right; sleep 1.5; e2e_key Left; sleep 1.5
shot class
spawned=0
for i in 1 2 3 4 5 6 7 8; do
    e2e_key shift
    if wait_log "\[cov\] spawn id=" 4; then spawned=1; break; fi
done
[ "$spawned" = 1 ] || { shot fail-spawn; say "FAIL: no spawn"; exit 1; }
mark "spawned"
sleep 4
shot spawn

if step_on walk; then
    mark "walk"
    hold w 3; hold a 0.6; hold w 2; e2e_keydown w; e2e_keydown space; sleep 2; e2e_keyup space; e2e_keyup w
    hold c 0.3; hold w 1.5; hold c 0.3   # crouch
    hold LShift 0.3                       # jump
    sleep 1; shot walk
fi
if step_on ui; then
    mark "chat / command / dialog / textdraws / menu"
    e2e_key t; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "hello coverage"; e2e_key Return; sleep 1
    e2e_key t; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "/covtest"; e2e_key Return; sleep 1
    wait_log "\[cov\] text id=" 5 || say "no chat text seen"
    cmd "msg|coverage message"
    cmd "gametext|3|3000|~r~coverage ~w~game text"
    sleep 1
    cmd "dialog|1|0|Coverage|Message box dialog"
    sleep 2; shot dialog-msgbox; e2e_key Return
    wait_log "\[cov\] dialog-response id=[0-9]+ dialog=1" 5 || say "no dialog 1 response"
    cmd "dialog|2|1|Coverage|Input dialog"
    sleep 2; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "abc"; e2e_key Return
    cmd "dialog|3|2|Coverage|First\nSecond\nThird"
    sleep 2; e2e_key Down; e2e_key Return
    cmd "td|~g~coverage textdraw"
    sleep 2; shot textdraws
    cmd "tdhide"
    cmd "menu"
    sleep 2; shot menu; e2e_key Down; sleep 0.3; hold space 0.3; sleep 1
    e2e_key F7; sleep 1; e2e_key F7; e2e_key F7   # chat window modes
    e2e_key Tab; sleep 1.5; shot scoreboard; e2e_key Escape; sleep 1
fi
if step_on world; then
    mark "world entities: objects, pickups, labels, checkpoints, gang zones, icons, actors"
    cmd "pos|2495|-1680|13.5|180"
    sleep 2
    cmd "obj|1337|2498|-1690|13.5"
    O="$(cmd_id)"
    cmd "obj|3361|2490|-1695|13.0"
    [ -n "$O" ] && cmd "objmove|$O|2498|-1690|16|1.5"
    cmd "label|Coverage label|2495|-1686|14"
    cmd "pickup|1240|2|2495|-1684|13.5"
    cmd "pickup|1242|2|2495|-1676|13.5"
    cmd "icon|1|2500|-1700|13|6"
    cmd "gz|2400|-1750|2550|-1600"
    cmd "actor|120|2492|-1690|13.5|90"
    cmd "attach|18645|2"
    cmd "cp|2495|-1674|13.5|3"
    sleep 1; hold w 2.5; sleep 1; shot pickups
    cmd "rcp|0|2495|-1660|13.5|2495|-1640|13.5|5"
    hold w 2.5
    cmd "cpoff"
    sleep 1
fi
if step_on anim; then
    mark "animations, camera, drunk, weather, time"
    cmd "anim|DANCING|dance_loop"; sleep 3
    cmd "clranim"
    cmd "anim|PED|WALK_drunk"; sleep 2
    cmd "clranim"
    cmd "drunk|5000"; hold w 2
    cmd "drunk|0"
    cmd "time|22"; sleep 2; shot night
    cmd "weather|8"; sleep 3
    cmd "weather|10"
    cmd "time|12"
    cmd "campos|2480|-1700|30|2495|-1687|13"; sleep 2
    cmd "camback"
    cmd "money|5000"
    cmd "wanted|2"; sleep 2
    cmd "wanted|0"
    cmd "sound|1057"
    cmd "spec|1"; sleep 2
    cmd "spec|0"; sleep 3
fi
if step_on weapons; then
    mark "weapons and damage"
    cmd "pos|2495|-1680|13.5|0"
    sleep 1
    cmd "weapon|24|100"; sleep 1
    DISPLAY="$E2E_DISPLAY" xdotool mousedown 3; sleep 0.6
    for i in 1 2 3; do DISPLAY="$E2E_DISPLAY" xdotool click 1; sleep 0.5; done
    DISPLAY="$E2E_DISPLAY" xdotool mouseup 3
    cmd "weapon|31|300"; sleep 1
    DISPLAY="$E2E_DISPLAY" xdotool mousedown 1; sleep 1.5; DISPLAY="$E2E_DISPLAY" xdotool mouseup 1
    hold LCtrl 1                          # alternative fire key
    wait_log "\[cov\] shot id=" 3 || say "no shot seen by the server"
    cmd "weapon|16|3"; sleep 1
    DISPLAY="$E2E_DISPLAY" xdotool mousedown 1; sleep 1; DISPLAY="$E2E_DISPLAY" xdotool mouseup 1
    sleep 3
    cmd "weapon|4|1"; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool click 1; sleep 0.5
    cmd "weapon|0|0"; DISPLAY="$E2E_DISPLAY" xdotool click 1; sleep 0.5; hold f 0.3
    cmd "armour|50"
    cmd "hp|60"
    cmd "explode|2505|-1680|13.5|0|5"; sleep 3
    shot weapons
fi
if step_on death; then
    mark "death and respawn"
    cmd "hp|100"
    cmd "kill"
    wait_log "\[cov\] death id=" 10 || say "no death"
    sleep 8
    for i in 1 2 3; do grep -c "\[cov\] spawn id=" "$SLOG" | grep -qv '^1$' && break; e2e_key shift; sleep 3; done
    shot respawn
    [ "$(grep -c "\[cov\] spawn id=" "$SLOG")" -ge 2 ] && mark "respawned" || say "no respawn seen"
    sleep 3
fi
drive() { # MODEL X Y Z ANGLE SECS KEYS... — create, put the player in, drive
    local model="$1" x="$2" y="$3" z="$4" a="$5" secs="$6" v
    shift 6
    cmd "pos|$x|$((${y%.*} + 4))|$z|$a"
    sleep 2
    cmd "veh|$model|$x|$y|$z|$a"
    v="$(cmd_id)"
    [ -n "$v" ] && [ "$v" -gt 0 ] 2> /dev/null || { say "vehicle $model not created"; return; }
    sleep 2
    cmd "putin|$v|0"
    wait_log "\[cov\] state id=[0-9]+ [0-9]+->2 veh=$v" 8 || say "not driver of $model"
    sleep 1
    local k
    for k in "$@"; do e2e_keydown "$k"; done
    sleep "$secs"
    for k in "$@"; do e2e_keyup "$k"; done
    shot "veh-$model"
    hold h 0.3; hold 2 0.2     # horn, misc
    sleep 1
    hold f 0.4                 # exit
    sleep 3
    cmd "vdel|$v"
}
if step_on vehicles; then
    mark "vehicles"
    drive 560 2500 -1660 13.4 0 6 w            # Sultan (car)
    drive 560 2500 -1660 13.4 0 3 w d          # turning
    drive 522 2500 -1660 13.4 0 5 w            # NRG-500 (bike)
    drive 509 2500 -1660 13.4 0 4 w            # BMX
    drive 487 1950 -2380 13.6 90 6 w           # Maverick (heli)
    drive 593 1780 -2493 13.6 90 8 w           # Dodo (plane)
    drive 452 2950 -2050 0.5 90 5 w            # Speeder (boat)
    drive 432 2500 -1660 13.4 0 3 w            # Rhino (tank)
    mark "train"
    T="$(grep -oE '\[cov\] static train [0-9]+' "$SLOG" | awk '{print $4}')"
    if [ -n "$T" ]; then
        cmd "pos|1700|-1945|14|200"; sleep 3
        cmd "putin|$T|0"; sleep 1; hold w 4; shot train; hold f 0.4; sleep 2
    fi
    mark "enter a car by key, drive-by, damage"
    cmd "pos|2500|-1665|13.5|0"; sleep 1
    cmd "veh|411|2502|-1660|13.4|0"; V="$(cmd_id)"; sleep 2
    hold f 0.4; sleep 4
    wait_log "\[cov\] state id=[0-9]+ [0-9]+->2 veh=$V" 6 || say "did not enter car by key"
    hold w 2
    [ -n "$V" ] && cmd "vhp|$V|300"; sleep 5
    shot car-fire
    hold f 0.4; sleep 2
    [ -n "$V" ] && cmd "vhp|$V|0"; sleep 6
    [ -n "$V" ] && cmd "vdel|$V"
    sleep 8; cmd "hp|100"                 # respawn first if the explosion killed the player
    cmd "pos|2950|-2050|2|90"; sleep 4; hold w 2; shot swim   # swimming
fi
if step_on interior; then
    mark "interiors"
    cmd "pos|2229.5|-1717.0|13.5|180"; sleep 3
    hold w 3; sleep 4; shot enex      # Ganton gym entry marker
    wait_log "\[cov\] interior id=[0-9]+ 0->" 4 || say "no enex interior change"
    cmd "interior|17"                  # 24/7 store interior, set by the server
    cmd "pos|-25.9|-185.9|1003.6|0"; sleep 4; hold w 1.5; shot interior
    cmd "interior|0"
    cmd "pos|2495|-1680|13.5|0"; sleep 3
fi
if step_on remote; then
    mark "remote players (server-side NPCs): on foot, aim/shoot, melee, animation, remote driver, passenger"
    cmd "hp|100"
    cmd "pos|2495|-1660|13.5|180"; sleep 3
    cmd "npc|CovNpcA|2500|-1668|13.5|105|31"; A="$(cmd_id)"
    cmd "npc|CovNpcB|2490|-1668|13.5|106|24"; B="$(cmd_id)"
    sleep 3; shot remote-1
    if [ -n "$A" ] && [ -n "$B" ]; then
        cmd "npcfollow|$A"; sleep 4
        cmd "npcanim|$B|DANCING|dance_loop"; sleep 2
        cmd "npcstop|$A"
        cmd "npcaim|$A|1"; sleep 4; shot remote-shoot
        cmd "npcstop|$A"
        cmd "hp|100"
        cmd "npcmelee|$B"; sleep 2
        cmd "veh|560|2505|-1650|13.4|0"; V="$(cmd_id)"; sleep 2
        if [ -n "$V" ]; then
            cmd "npcveh|$B|$V|0"; sleep 2
            cmd "putin|$V|1"; sleep 2           # passenger of a remote driver
            cmd "npcmove|$B|2505|-1600|13.4"; sleep 6; shot remote-driver
            hold f 0.4; sleep 3
        fi
        cmd "npckill|$A"; sleep 3
        cmd "npcdel|$A"; cmd "npcdel|$B"
        [ -n "$V" ] && cmd "vdel|$V"
    else
        say "NPCs not created"
    fi
    cmd "hp|100"
fi
if step_on quit; then
    mark "quit (/q)"
    e2e_key t; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "/q"; e2e_key Return
    wait_log "\[cov\] disconnect id=" 15 || say "no disconnect"
    for ((i = 0; i < 30; i++)); do e2e_game_alive || break; sleep 1; done
    e2e_game_alive && say "game still running after /q" || say "game exited"
else
    sleep 12   # one more coverage write
fi
grep -c '\[cov\] where' "$SLOG" > /dev/null
say "scenario done"
exit 0
