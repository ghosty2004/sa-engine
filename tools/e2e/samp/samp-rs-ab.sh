#!/usr/bin/env bash
# SAMP-2/4 A/B join: the ORIGINAL gta_sa.exe 1.0 US with a samp-rs build (samp_client.dll + samp-launcher.exe from
# DLL_DIR) against our private open.mp 1.6.7 copy, running a fixed join scenario (connect, class selection, spawn,
# walk, server commands: message, game text, position, weapon, money, clock, weather, vehicle enter/drive/exit,
# interior, death and respawn, /q). Used to show that an adapter step of samp-rs is a pure refactor: run it once
# with the build before the step (A) and once after (B) and diff the two `sequence.txt` files.
#
#   LANE=3 SA_ROOT=~/sa-engine tools/e2e/samp/samp-rs-ab.sh DLL_DIR NAME
#
# Inputs (gitignored, never committed): $SA_ROOT/re/samp/client (samp.saa, SAMP/, ... - samp.dll is NOT used),
# $SA_ROOT/re/samp/server (open.mp), the auth table AUTH_TABLE (default ~/samp-rs/data/auth_table.txt, third-party
# data, read only). The lane's game dir gets lane-private copies/symlinks that are removed afterwards.
# Output: target/e2e/lane-N/samp-ab/NAME/ (server.log, samp-rs.log, shots, sequence.txt).
# sequence.txt = the server's [cov] event lines + the client's RPC/state lines, with timestamps removed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
DLL_DIR="$(cd "${1:?usage: samp-rs-ab.sh DLL_DIR NAME}" && pwd)"
NAME="${2:?usage: samp-rs-ab.sh DLL_DIR NAME}"
LANE="${LANE:-3}"
# shellcheck source=../lib.sh
source "$HERE/../lib.sh"
SAMP_PORT="${SAMP_PORT:-7813}"
CLIENT_SRC="$SA_ROOT/re/samp/client"
SERVER_SRC="$SA_ROOT/re/samp/server"
AUTH="${AUTH_TABLE:-$HOME/samp-rs/data/auth_table.txt}"
OUT="$E2E_OUT/samp-ab/$NAME"
SRV="$E2E_OUT/samp-ab/server"
SLOG="$OUT/server.log"
CLIENT_FILES="samp.saa bass.dll sampgui.png mouse.png sampaux3.ttf SAMP"
rm -rf "$OUT"
mkdir -p "$OUT"
for f in samp_client.dll samp-launcher.exe; do
    [ -f "$DLL_DIR/$f" ] || { echo "samp-rs-ab: $DLL_DIR/$f missing" >&2; exit 1; }
done
[ -x "$SERVER_SRC/omp-server" ] || { echo "samp-rs-ab: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "samp-rs-ab: auth table $AUTH missing" >&2; exit 1; }

e2e_lane_lock
e2e_disk_check || exit 1
e2e_lane_setup || exit 1
say() { e2e_say "$*"; echo "[$(date +%T)] $*" >> "$OUT/steps.log"; }

# ---- client files (lane-private, removed in finish) ----------------------------------------------------
for f in $CLIENT_FILES samp.dll; do
    if [ -e "$GTA_DIR/$f" ] && [ ! -L "$GTA_DIR/$f" ]; then
        echo "samp-rs-ab: $GTA_DIR/$f exists and is not a symlink; refusing" >&2; exit 1
    fi
done
[ -L "$GTA_DIR/samp.dll" ] && rm -f "$GTA_DIR/samp.dll"
for f in $CLIENT_FILES; do
    [ -e "$CLIENT_SRC/$f" ] && ln -sfn "$CLIENT_SRC/$f" "$GTA_DIR/$f"
done
e2e_put "$DLL_DIR/samp_client.dll" samp_client.dll
e2e_put "$DLL_DIR/samp-launcher.exe" samp-rs-launcher.exe
rm -f "$GTA_DIR/samp-rs.log" "$GTA_DIR/samp-rs-crash.txt"
USERDIR="$WINEPREFIX/drive_c/users/$USER/Documents/GTA San Andreas User Files"
mkdir -p "$USERDIR/SAMP"
printf 'fpslimit=50\r\nmulticore=1\r\naudioproxyoff=0\r\ntimestamp=0\r\n' > "$USERDIR/SAMP/sa-mp.cfg"

# ---- server --------------------------------------------------------------------------------------------
mkdir -p "$SRV"
rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/sampcov.pwn" -iqawno/include \
    -ogamemodes/sampcov.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$SAMP_PORT" << 'PY'
import json, sys
p, port = sys.argv[1], int(sys.argv[2])
d = json.load(open(p))
d["announce"] = False
d["network"]["port"] = port
d["network"]["bind"] = "127.0.0.1"
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
wait_log() { # REGEX SECS
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        grep -qE "$1" "$SLOG" 2> /dev/null && return 0
        if [ -n "${E2E_GAME_PID:-}" ] && ! kill -0 "$E2E_GAME_PID" 2> /dev/null; then return 1; fi
        sleep 0.5
    done
    return 1
}
if ss -lun | grep -q ":$SAMP_PORT "; then echo "samp-rs-ab: UDP $SAMP_PORT busy" >&2; exit 1; fi
( cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1 ) &
SRV_PID=$!
wait_log 'Legacy Network started' 20 || { echo "samp-rs-ab: server did not start" >&2; server_stop; exit 1; }
say "server pid $SRV_PID on UDP $SAMP_PORT"

# Steam 3.0 audio data under the 1.0 US exe: see samp-cov.sh (private audio/CONFIG with aliased stub tracks).
AUDIO_FIX="$SA_ROOT/re/samp/tools/audio-stub-tracks.py"
audio_private() {
    [ -f "$AUDIO_FIX" ] && [ -L "$GTA_DIR/audio" ] || return 0
    local shared
    shared="$(readlink "$GTA_DIR/audio")"
    rm "$GTA_DIR/audio" && mkdir -p "$GTA_DIR/audio/CONFIG" || return 1
    echo "$shared" > "$GTA_DIR/audio/.shared"
    ln -s "$shared/sfx" "$GTA_DIR/audio/sfx"
    ln -s "$shared/streams" "$GTA_DIR/audio/streams"
    cp "$shared"/CONFIG/* "$GTA_DIR/audio/CONFIG/"
    python3 -I "$AUDIO_FIX" patch "$GTA_DIR" > "$OUT/audio-fix.log" 2>&1 || cat "$OUT/audio-fix.log"
}
audio_restore() {
    [ -f "$GTA_DIR/audio/.shared" ] || return 0
    local shared
    shared="$(cat "$GTA_DIR/audio/.shared")"
    rm -rf "${GTA_DIR:?}/audio" && ln -s "$shared" "$GTA_DIR/audio"
}

finish() {
    say "teardown"
    e2e_cleanup
    server_stop
    cp -f "$GTA_DIR/samp-rs.log" "$OUT/samp-rs.log" 2> /dev/null
    cp -f "$GTA_DIR/samp-rs-crash.txt" "$OUT/" 2> /dev/null
    rm -f "$GTA_DIR/samp-rs.log" "$GTA_DIR/samp-rs-crash.txt" "$GTA_DIR/samp_client.dll" "$GTA_DIR/samp-rs-launcher.exe"
    audio_restore
    for f in $CLIENT_FILES; do [ -L "$GTA_DIR/$f" ] && rm -f "$GTA_DIR/$f"; done
    # The comparable sequence: server events (ids kept, they are deterministic for one client) and the
    # client's log without timestamps, addresses, frame ticks and network counters.
    {
        grep -oE '\[cov\] .*' "$SLOG" | grep -vE '^\[cov\] (where|sync|keys|upd|aim) '
        echo '---- probes'
        cat "$OUT/probes.txt" 2> /dev/null
        echo '---- client'
        sed -E 's/^\[[^]]*\] \[[^]]*\] //' "$OUT/samp-rs.log" 2> /dev/null |
            grep -vE '^(tick [0-9]+|control: client|client: net phase|net: connect to|net: auth challenge|net: open-connection)' |
            sed -E 's/0x[0-9a-f]{5,}/0xADDR/g; s/after [0-9.]+s/after Ns/; s/(physical RAM) [0-9]+ MB/\1 N MB/' |
            grep -v '^gGameState = ' || true
        # The init thread's game-state lines race with the game thread's log lines: compared as a set.
        echo '---- game states'
        grep -oE 'gGameState = [7-9] \([A-Z_]+\)' "$OUT/samp-rs.log" 2> /dev/null | sort
    } > "$OUT/sequence.txt"
    say "sequence: $(wc -l < "$OUT/sequence.txt") lines in $OUT/sequence.txt"
}
trap finish EXIT
audio_private || exit 1

# ---- game ------------------------------------------------------------------------------------------------
e2e_headless_start || exit 1
export SAMP_RS_AUTH_TABLE="$(e2e_winpath "$AUTH")"
export SAMP_RS_CTL_PORT=$((47790 + LANE))
(
    cd "$GTA_DIR" || exit 1
    exec env -u WAYLAND_DISPLAY DISPLAY="$E2E_DISPLAY" wine "$GTA_DIR/samp-rs-launcher.exe" \
        --gta "$(e2e_winpath "$GTA_DIR/gta_sa.exe")" --dll "$(e2e_winpath "$GTA_DIR/samp_client.dll")" \
        --name AbPlayer --host 127.0.0.1 --port "$SAMP_PORT"
) > "$OUT/wine.log" 2>&1 &
E2E_GAME_PID=$!
say "game started (pid $E2E_GAME_PID)"

SEQ=0
CMDF="$SRV/scriptfiles/sampcov.txt"
: > "$CMDF"
cmd() {
    SEQ=$((SEQ + 1))
    echo "$SEQ $1" >> "$CMDF"
    wait_log "\[cov\] $SEQ (done|unknown)" 10 || say "command $SEQ ($1) not acknowledged"
}
cmd_id() { grep -oE "\[cov\] $SEQ [a-z]+ -?[0-9]+" "$SLOG" | tail -1 | awk '{print $4}'; }
hold() { e2e_keydown "$1"; sleep "$2"; e2e_keyup "$1"; }
alive() { e2e_game_alive || { say "GAME EXITED"; exit 1; }; }
shot() { e2e_shot "samp-ab/$NAME/$1" > /dev/null; }
mark() { say "== $*"; alive; }
# Game state the steps change, read through samp-rs's control channel (`peek`): weather (forced/new/old),
# clock, CClock::Update's first byte, the HUD clock format, gravity, current area, money.
PEEKS="0xC81318:12 0xB70152:2 0x52CF10:1 0x859A6C:12 0x863984:4 0xB72914:4 0xB7CE50:4"
probe() {
    local p
    for p in $PEEKS; do
        echo "probe $1 $(python3 -I - "$SAMP_RS_CTL_PORT" "peek ${p%%:*} ${p##*:}" << 'PY'
import socket, sys
try:
    s = socket.create_connection(("127.0.0.1", int(sys.argv[1])), timeout=3)
    s.settimeout(5)
    s.sendall((sys.argv[2] + "\n").encode())
    print(s.makefile().readline().strip())
except OSError as e:
    print("error", e)
PY
)" >> "$OUT/probes.txt"
    done
}

wait_log "\[cov\] connect id=[0-9]+ name=AbPlayer" 180 || { shot fail-connect; say "FAIL: no connect"; exit 1; }
mark "connected"
wait_log "\[cov\] class id=" 90 || { shot fail-class; say "FAIL: no class selection"; exit 1; }
mark "class selection"
sleep 3
e2e_key Right; sleep 1.5; e2e_key Right; sleep 1.5; e2e_key Left; sleep 1.5
shot class
spawned=0
for _ in 1 2 3 4 5 6 7 8; do
    e2e_key shift
    if wait_log "\[cov\] spawn id=" 4; then spawned=1; break; fi
done
[ "$spawned" = 1 ] || { shot fail-spawn; say "FAIL: no spawn"; exit 1; }
mark "spawned"
sleep 4
shot spawn
probe spawned
hold w 2; sleep 1
mark "server commands"
cmd "msg|ab message"
cmd "gametext|3|2000|~r~ab ~w~game text"
cmd "pos|2495|-1680|13.5|180"
sleep 1
cmd "weapon|24|100"
cmd "money|5000"
cmd "time|22"
cmd "weather|8"
sleep 2
shot night
probe night
cmd "time|12"
cmd "weather|10"
sleep 1
probe day
cmd "veh|560|2500|-1660|13.4|0"
V="$(cmd_id)"
sleep 2
if [ -n "$V" ]; then
    cmd "putin|$V|0"
    wait_log "\[cov\] state id=[0-9]+ [0-9]+->2 veh=$V" 8 || say "not driver"
    hold w 3
    shot veh
    hold f 0.4; sleep 3
    cmd "vdel|$V"
fi
cmd "interior|17"
cmd "pos|-25.9|-185.9|1003.6|0"; sleep 3
shot interior
probe interior
cmd "interior|0"
cmd "pos|2495|-1680|13.5|0"; sleep 2
probe outside
mark "death"
cmd "kill"
wait_log "\[cov\] death id=" 10 || say "no death"
sleep 8
for _ in 1 2 3; do [ "$(grep -c "\[cov\] spawn id=" "$SLOG")" -ge 2 ] && break; e2e_key shift; sleep 3; done
[ "$(grep -c "\[cov\] spawn id=" "$SLOG")" -ge 2 ] && mark "respawned" || say "no respawn seen"
shot respawn
probe respawned
mark "quit (/q)"
e2e_key t; sleep 0.5; DISPLAY="$E2E_DISPLAY" xdotool type --delay 60 "/q"; e2e_key Return
wait_log "\[cov\] disconnect id=" 15 || say "no disconnect"
for ((i = 0; i < 20; i++)); do e2e_game_alive || break; sleep 1; done
say "scenario done"
exit 0
