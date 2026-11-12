#!/usr/bin/env bash
# SAMP-4 native join (docs/samp/native-plan.md): native sa-game (i686 Linux, built with `--features samp`) joins a
# private copy of our open.mp 1.6.7 server on 127.0.0.1, spawns, and the server then drives the player
# (tools/e2e/samp/sampcov.pwn commands: position, health, armour, weapon, money, clock, weather, interior, a vehicle
# the player is put in).
# No Wine, no gta_sa.exe.
#
#   tools/e2e/samp/native-join.sh [OUT_DIR]       (default target/samp4-evidence)
#
# Inputs (gitignored, never committed): $SA_ROOT/re/samp/server (open.mp), $SA_ROOT/wine/gta-sa (the game data
# sa-game reads), the auth table AUTH_TABLE (default ~/samp-rs/data/auth_table.txt, third-party data, read only).
# SA_GAME = the sa-game binary (default target/i686-unknown-linux-gnu/release/sa-game). FRAMES (default 1400).
# Port SAMP_PORT (default 7812), lock target/locks/$SAMP_LOCK (default samp-native.lock, shared with
# native-handshake.sh; give another name together with another port to run beside a running stack).
# Output: server.log, sa-game.log, report.md, present/*.png (every 20th in-game frame).
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
SA_ROOT="${SA_ROOT:-$ROOT}"
OUT="$(mkdir -p "${1:-$ROOT/target/samp4-evidence}" && cd "${1:-$ROOT/target/samp4-evidence}" && pwd)"
AUTH="${AUTH_TABLE:-$HOME/samp-rs/data/auth_table.txt}"
PORT="${SAMP_PORT:-7812}"
GAME_BIN="${SA_GAME:-$ROOT/target/i686-unknown-linux-gnu/release/sa-game}"
FRAMES="${FRAMES:-1600}"
# W held from iteration 1000 for 450 iterations (after the server's commands): the server sees the walk in the
# on-foot sync.
export SA_GAME_WALK="${SA_GAME_WALK:-1000:450}"
# The offscreen run has no keyboard: Shift for class selection is synthetic (samp's 2 s guard still applies).
export SA_SAMP_AUTO_SPAWN="${SA_SAMP_AUTO_SPAWN:-1}"
SERVER_SRC="$SA_ROOT/re/samp/server"
SRV="$OUT/server"
SLOG="$OUT/server.log"
[ -x "$SERVER_SRC/omp-server" ] || { echo "native-join: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "native-join: auth table $AUTH missing" >&2; exit 1; }
[ -x "$GAME_BIN" ] || { echo "native-join: $GAME_BIN missing (cargo build --release -p sa-game --features samp --target i686-unknown-linux-gnu)" >&2; exit 1; }
free_kb="$(df -Pk /home | awk 'NR == 2 { print $4 }')"
[ "$free_kb" -ge $((20 * 1024 * 1024)) ] || { echo "native-join: less than 20 GB free on /home" >&2; exit 1; }

mkdir -p "$SA_ROOT/target/locks"
exec {LOCK_FD}> "$SA_ROOT/target/locks/${SAMP_LOCK:-samp-native.lock}"
flock "$LOCK_FD"
if ss -lun | grep -q ":$PORT "; then echo "native-join: UDP $PORT busy" >&2; exit 1; fi

rm -rf "$OUT/present" "$OUT/report.md"
mkdir -p "$SRV" "$OUT/present"
rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/sampcov.pwn" -iqawno/include \
    -ogamemodes/sampcov.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$PORT" << 'PY'
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
json.dump(d, open(p, "w"), indent=4)
PY
(cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1) &
SRV_PID=$!
GAME_PID=
cleanup() {
    [ -n "$GAME_PID" ] && kill "$GAME_PID" 2> /dev/null
    kill "$SRV_PID" 2> /dev/null
    wait "$SRV_PID" 2> /dev/null || true
}
trap cleanup EXIT
wait_log() { # REGEX SECS
    local i
    for ((i = 0; i < $2 * 2; i++)); do
        grep -qE "$1" "$SLOG" 2> /dev/null && return 0
        [ -n "$GAME_PID" ] && ! kill -0 "$GAME_PID" 2> /dev/null && return 1
        sleep 0.5
    done
    return 1
}
wait_log 'Legacy Network started' 20 || { echo "native-join: server did not start" >&2; cat "$SLOG"; exit 1; }
say() { echo "[$(date +%T)] $*" | tee -a "$OUT/steps.log"; }
: > "$OUT/steps.log"
say "server pid $SRV_PID on 127.0.0.1:$PORT"

# STD-9's exploration stubs (render callbacks, skin node, effects), plus two for the native join's vehicle:
# CVisibilityPlugins::RenderVehicleReallyLowDetailCB 0x7331E0 and the collision sparks 0x6A6DC0 (visual only).
STUBS='0x6a6dc0=0/12,0x7331e0=0/0,0x7c7b90=1/0,0x80e270=1/0,0x455720=0/0,0x4f6420=0/2,0x732f30=0/0,0x5d9900=0/0,0x4a2c40=0/4,0x701780=0/0,0x733f80=0/0,0x733240=0/0'
SA_GAME_STUB="${SA_GAME_STUB:-$STUBS}" SA_GAME_SCRIPT_SKIP=1 SA_GAME_PRESENT_GAME=1 SA_GAME_PRESENT_EVERY="${SA_GAME_PRESENT_EVERY:-20}" \
    "$GAME_BIN" --game "$SA_ROOT/wine/gta-sa" --frames "$FRAMES" --start-in-world \
    --samp "127.0.0.1:$PORT" --name NativeCJ --samp-auth "$AUTH" \
    --present-dir "$OUT/present" --report "$OUT/report.md" > "$OUT/sa-game.log" 2>&1 &
GAME_PID=$!
say "sa-game pid $GAME_PID ($FRAMES frames)"

SEQ=0
CMDF="$SRV/scriptfiles/sampcov.txt"
cmd() {
    SEQ=$((SEQ + 1))
    echo "$SEQ $1" >> "$CMDF"
    wait_log "\[cov\] $SEQ (done|unknown)" 10 && say "server: $1" || say "command $SEQ ($1) not acknowledged"
}
if wait_log '\[cov\] connect id=[0-9]+ name=NativeCJ' 120; then say "connected"; else say "FAIL: no connect"; fi
if wait_log '\[cov\] spawn id=' 120; then
    say "spawned"
    sleep 4
    cmd "msg|Welcome, native sa-game: chat from open.mp"
    cmd "gametext|3|6000|~g~native ~w~sa-game"
    cmd "hp|60"
    cmd "armour|40"
    cmd "money|5000"
    cmd "weapon|24|100"
    cmd "time|22"
    cmd "weather|4"   # 8 (storm) reaches the unported thunder audio event 0x506800, 9 (fog) CMovingFog::Update 0x716B10
    sleep 4
    cmd "pos|2495|-1680|13.5|180"
    sleep 4
    cmd "pos|2440|-1660|13.5|90"
    sleep 4
    cmd "time|12"
    cmd "weather|10"
    cmd "interior|0"
    # A server vehicle (streamed in and created natively). PUTIN=1 also puts the player in as driver; natively the
    # warp task does not run yet (native-plan.md §10.7).
    cmd "veh|560|2445|-1650|13.4|90"
    V="$(grep -oE "\[cov\] $SEQ veh -?[0-9]+" "$SLOG" | tail -1 | awk '{print $4}')"
    [ -n "$V" ] && [ "${PUTIN:-0}" = 1 ] && cmd "putin|$V|0"
    sleep 6
else
    say "FAIL: no spawn"
fi
wait "$GAME_PID"
say "sa-game exit $?"
GAME_PID=
grep -E '\[(join|part|connection)\]|\[cov\] (connect|class|spawn|state|where|[0-9]+ done|disconnect)' "$SLOG" > "$OUT/server-events.txt"
say "server events: $(wc -l < "$OUT/server-events.txt") lines; PNGs: $(ls "$OUT/present" | wc -l)"
