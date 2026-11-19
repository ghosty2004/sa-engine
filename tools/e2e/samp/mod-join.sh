#!/usr/bin/env bash
# SA:MP as a mod, natively: sa-game (i686 Linux, `--features mods`) runs the samp mod (mods/build.sh), which joins a
# private copy of our open.mp 1.6.7 server on 127.0.0.1, spawns, and the server then drives the player
# (tools/e2e/samp/sampcov.pwn: chat, game text, health, weapon, clock, weather, position, a vehicle) and an NPC walks
# up to it (a remote player for the client). No Wine, no gta_sa.exe.
#
#   tools/e2e/samp/mod-join.sh [OUT_DIR]       (default target/samp-mod-join)
#
# Inputs (gitignored, never committed): $SA_ROOT/re/samp/server (open.mp), $SA_ROOT/wine/gta-sa (the game data),
# the auth table AUTH_TABLE (default $SA_ROOT/re/samp/auth_table.txt, third-party data, read only), which goes into
# the mod's storage (OUT/moddata/samp/, `SA_MOD_DATA`) with the server and the player name.
# By default the mod joins at once (`auto_connect`) and the server is already approved in the host's list
# (OUT/moddata/sa-engine-host/net.samp). UX=1 goes through the connect screen and the host's approval prompt
# instead, with scripted keys (Enter on the screen at iteration UX_ENTER, default 400; Y at UX_ENTER + 40): the
# server is the one favourite, so Enter joins it. UX_APPROVED=1 approves it beforehand (no prompt; the screen
# queries it from the start). The Internet tab's list is a fixture served on 127.0.0.1:LIST_PORT (default 7820,
# OUT/list/servers.json: our server and a few rows at documentation addresses that are never contacted), never the
# public list: the mod's copy in OUT/mods/samp gets that prefix in its `http` permission. RESOLUTION=WxH also loads
# the resolution mod (from MODS) at that video mode.
# SA:MP's UI files (sampgui.png, mouse.png, sampaux3.ttf from SAMP_CLIENT, default $SA_ROOT/re/samp/client, and the
# game's gtaweap3.ttf; third-party, never committed) are copied into the samp mod's folder when they exist.
# EXTRA_CMDS: more server commands after the scripted ones, `;`-separated (`sleep|S` waits, `shot|NAME` copies the
# newest complete presented frame to OUT/shots/NAME.png), then the lines of the file CMDS (tools/e2e/samp/objects.cmds), e.g.
# 'td|~y~textdraw;dialog|1|0|Welcome|Hello;sleep|8'; SA_GAME_KEYS adds scripted keys (`tab:1500:1502`
# opens the scoreboard). BOT=1: samp-rs's samp-bot (SAMP_BOT, default samp-rs/target/release/samp-bot) joins as
# a second, real player once ours spawned, stands at the spawn point, ahead of our player (its nametag), and leaves
# after BOT_SECONDS (default 30).
# WEB=1: the same join from the browser: the web build with mods (FEATURES=mods tools/web/build.sh) in headless
# Chromium (tools/web/headless.mjs, WEB_SECONDS, default 150, WEB_GPU its --gpu, default swiftshader), its UDP through
# sa-udp-proxy (UDP_PROXY, default
# target/release/sa-udp-proxy, on 127.0.0.1:PROXY_PORT, default 7791) to the server, the page served by
# tools/web/serve.sh (WEB_PORT, default 8655). The page's storage starts empty: the mod reads its settings and the
# auth table from files in its served folder (OUT/webmods/samp), and the scripted Y presses answer the game's
# approval prompt. The server waits 60 s for a silent client (software rendering can hold one frame for seconds).
# Screenshots in OUT/web/.
# SCENARIO=world: instead of the commands above, the world scenario of world-scenario.sh (pickups, checkpoints, map
# icons, a gang zone, an actor, then gmx and a server restart), as samp-rs-ab.sh SCENARIO=world runs it on the DLL
# client; screenshots (the newest present frame at each step) in OUT/shots/. Give it FRAMES for about 4 minutes.
# SA_GAME = the sa-game binary (default target/i686-unknown-linux-gnu/release/sa-game), MODS = the built mods
# (default target/mods). FRAMES (default 1800). Port SAMP_PORT (default 7813).
# Output: server.log, sa-game.log, report.md, present/*.png (every 20th in-game frame), server-events.txt, and
# goal.txt: samp-rs's goal scenario (tools/e2e/goal.sh on the DLL host) as this host can show it: connected, spawned,
# the server's commands acknowledged, our own walk seen by the server (>= 1 m from the teleport point), and with
# BOT=1 the other player streamed in as a ped and removed when it leaves. Exit status 0 only if every step holds.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
SA_ROOT="${SA_ROOT:-$ROOT}"
OUT="$(mkdir -p "${1:-$ROOT/target/samp-mod-join}" && cd "${1:-$ROOT/target/samp-mod-join}" && pwd)"
AUTH="${AUTH_TABLE:-$SA_ROOT/re/samp/auth_table.txt}"
PORT="${SAMP_PORT:-7813}"
GAME_BIN="${SA_GAME:-$ROOT/target/i686-unknown-linux-gnu/release/sa-game}"
MODS="${MODS:-$ROOT/target/mods}"
FRAMES="${FRAMES:-1800}"
# W held from iteration 1000 for 450 iterations (after the server's commands): the server sees the walk in the
# on-foot sync.
export SA_GAME_WALK="${SA_GAME_WALK:-1000:450}"
SERVER_SRC="$SA_ROOT/re/samp/server"
SRV="$OUT/server"
SLOG="$OUT/server.log"
[ -x "$SERVER_SRC/omp-server" ] || { echo "mod-join: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "mod-join: auth table $AUTH missing" >&2; exit 1; }
WEB="${WEB:-0}"
PROXY_BIN="${UDP_PROXY:-$ROOT/target/release/sa-udp-proxy}"
if [ "$WEB" = 1 ]; then
    [ -f "$ROOT/web/pkg/sa_game_bg.wasm" ] || { echo "mod-join: web/pkg missing (FEATURES=mods tools/web/build.sh)" >&2; exit 1; }
    [ -x "$PROXY_BIN" ] || { echo "mod-join: $PROXY_BIN missing (cargo build --release -p sa-udp-proxy)" >&2; exit 1; }
else
    [ -x "$GAME_BIN" ] || { echo "mod-join: $GAME_BIN missing (cargo build --release -p sa-game --features mods --target i686-unknown-linux-gnu)" >&2; exit 1; }
fi
[ -f "$MODS/samp/samp.wasm" ] || { echo "mod-join: $MODS/samp/samp.wasm missing (mods/build.sh)" >&2; exit 1; }
free_kb="$(df -Pk /home | awk 'NR == 2 { print $4 }')"
[ "$free_kb" -ge $((20 * 1024 * 1024)) ] || { echo "mod-join: less than 20 GB free on /home" >&2; exit 1; }

mkdir -p "$SA_ROOT/target/locks"
exec {LOCK_FD}> "$SA_ROOT/target/locks/${SAMP_LOCK:-samp-mod.lock}"
flock "$LOCK_FD"
if ss -lun | grep -q ":$PORT "; then echo "mod-join: UDP $PORT busy" >&2; exit 1; fi

rm -rf "$OUT/present" "$OUT/shots" "$OUT/report.md" "$OUT/moddata"
mkdir -p "$SRV" "$OUT/present" "$OUT/shots" "$OUT/moddata/samp"
printf '127.0.0.1:%s' "$PORT" > "$OUT/moddata/samp/server"
printf 'ModCJ' > "$OUT/moddata/samp/name"
printf '1' > "$OUT/moddata/samp/auto_spawn"
LIST_PORT="${LIST_PORT:-7820}"
printf 'http://127.0.0.1:%s/servers.json' "$LIST_PORT" > "$OUT/moddata/samp/server_list_url"
if [ "${UX:-0}" = 1 ]; then
    E="${UX_ENTER:-400}"
    export SA_GAME_KEYS="enter:$E:$((E + 2)),y:$((E + 40)):$((E + 42))${SA_GAME_KEYS:+,$SA_GAME_KEYS}"
    printf '127.0.0.1:%s' "$PORT" > "$OUT/moddata/samp/favourites"
    if [ "${UX_APPROVED:-0}" = 1 ]; then
        mkdir -p "$OUT/moddata/sa-engine-host"
        printf 'allow 127.0.0.1:%s\n' "$PORT" > "$OUT/moddata/sa-engine-host/net.samp"
        printf '127.0.0.1:%s' "$PORT" > "$OUT/moddata/samp/query_allowed"
    fi
else
    printf '1' > "$OUT/moddata/samp/auto_connect"
    mkdir -p "$OUT/moddata/sa-engine-host"
    printf 'allow 127.0.0.1:%s\n' "$PORT" > "$OUT/moddata/sa-engine-host/net.samp"
fi
cp "$AUTH" "$OUT/moddata/samp/auth_table.txt"
CLIENT="${SAMP_CLIENT:-$SA_ROOT/re/samp/client}"
rm -rf "$OUT/mods"
mkdir -p "$OUT/mods" "$OUT/list"
cp -r "$MODS/samp" "$OUT/mods/samp"
sed -i "s|^http = \[|http = [\"http://127.0.0.1:$LIST_PORT/\", |" "$OUT/mods/samp/mod.toml"
MOD_DIRS="$OUT/mods/samp"
if [ -n "${RESOLUTION:-}" ]; then
    cp -r "$MODS/resolution" "$OUT/mods/resolution"
    printf 'resolution = "%s"\nfullscreen = false\nfollow_resize = false\n' "$RESOLUTION" > "$OUT/mods/resolution/config.toml"
    MOD_DIRS="$OUT/mods/resolution,$MOD_DIRS"
fi
for f in "$CLIENT/sampgui.png" "$CLIENT/mouse.png" "$CLIENT/sampaux3.ttf" "$SA_ROOT/wine/gta-sa/gtaweap3.ttf"; do
    [ -f "$f" ] && cp "$f" "$OUT/mods/samp/"
done
python3 -I - "$OUT/list/servers.json" "$PORT" << 'PY'
import json, sys
rows = [
    {"ip": f"127.0.0.1:{sys.argv[2]}", "hn": "sa-engine test server (open.mp)", "pc": 0, "pm": 50, "gm": "sampcov",
     "la": "English", "pa": False, "vn": "omp 1.6.7", "omp": True, "pr": False},
]
names = [("Los Santos Roleplay", "Roleplay", "English", True), ("San Fierro Freeroam", "Freeroam", "English", False),
         ("Las Venturas Stunts", "Stunt/DM", "Romana", False), ("Grove Street Families", "Gang wars", "English", False),
         ("Drift Kings", "Drift", "Polski", False), ("Old school DM", "Deathmatch", "Espanol", True)]
for i, (hn, gm, la, pa) in enumerate(names):
    rows.append({"ip": f"192.0.2.{10 + i}:7777", "hn": hn, "pc": (i * 37) % 180, "pm": 200, "gm": gm, "la": la,
                 "pa": pa, "vn": "0.3.7-R2", "omp": i % 2 == 0, "pr": False})
json.dump(rows, open(sys.argv[1], "w"), indent=1)
PY
rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$HERE/sampcov.pwn" -iqawno/include \
    -ogamemodes/sampcov.amx -d0 > "$OUT/pawncc.log" 2>&1 ) || { cat "$OUT/pawncc.log"; exit 1; }
python3 -I - "$SRV/config.json" "$PORT" "$WEB" << 'PY'
import json, sys
p, port, web = sys.argv[1], int(sys.argv[2]), sys.argv[3] == "1"
d = json.load(open(p))
d["announce"] = False
d["max_bots"] = 4
d["network"]["port"] = port
d["network"]["bind"] = "127.0.0.1"
# software rendering in the browser can hold a frame (streaming the spawn area) longer than the default 10 s
if web:
    d["network"]["player_timeout"] = 60000
d["artwork"]["enable"] = False
d["rcon"]["enable"] = False
d["pawn"]["main_scripts"] = ["sampcov 1"]
d["pawn"]["side_scripts"] = []
json.dump(d, open(p, "w"), indent=4)
PY
: > "$SLOG"
server_start() {
    (cd "$SRV" && exec ./omp-server >> "$SLOG" 2>&1) &
    SRV_PID=$!
}
server_kill() {
    kill -KILL "$SRV_PID" 2> /dev/null
    wait "$SRV_PID" 2> /dev/null
}
server_start
# the fixture list, with the CORS header a page's fetch needs
python3 -I - "$OUT/list" "$LIST_PORT" > "$OUT/list.log" 2>&1 << 'PY' &
import functools, http.server, sys
class H(http.server.SimpleHTTPRequestHandler):
    def end_headers(self):
        self.send_header("Access-Control-Allow-Origin", "*")
        super().end_headers()
http.server.ThreadingHTTPServer(("127.0.0.1", int(sys.argv[2])), functools.partial(H, directory=sys.argv[1])).serve_forever()
PY
LIST_PID=$!
GAME_PID=
BOT_PID=
WEB_PIDS=
cleanup() {
    [ -n "$GAME_PID" ] && kill "$GAME_PID" 2> /dev/null
    [ -n "$BOT_PID" ] && kill "$BOT_PID" 2> /dev/null
    for p in $WEB_PIDS; do kill "$p" 2> /dev/null; done
    kill "$LIST_PID" 2> /dev/null
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
wait_log 'Legacy Network started' 20 || { echo "mod-join: server did not start" >&2; cat "$SLOG"; exit 1; }
say() { echo "[$(date +%T)] $*" | tee -a "$OUT/steps.log"; }
: > "$OUT/steps.log"
say "server pid $SRV_PID on 127.0.0.1:$PORT"

# The exploration stubs of native-join.sh (render callbacks, skin node, effects, a vehicle's LOD render and sparks).
STUBS='0x6a6dc0=0/12,0x7331e0=0/0,0x7c7b90=1/0,0x80e270=1/0,0x455720=0/0,0x4f6420=0/2,0x732f30=0/0,0x5d9900=0/0,0x4a2c40=0/4,0x701780=0/0,0x733f80=0/0,0x733240=0/0'
if [ "$WEB" = 1 ]; then
    PROXY_PORT="${PROXY_PORT:-7791}"
    WEB_PORT="${WEB_PORT:-8655}"
    rm -rf "$OUT/webmods" "$OUT/web"
    mkdir -p "$OUT/webmods" "$OUT/web"
    cp -r "$OUT/mods/samp" "$OUT/webmods/samp"
    cp "$OUT/moddata/samp/"* "$OUT/webmods/samp/"
    "$PROXY_BIN" "127.0.0.1:$PORT" --bind "127.0.0.1:$PROXY_PORT" > "$OUT/proxy.log" 2>&1 &
    WEB_PIDS="$!"
    "$ROOT/tools/web/serve.sh" --port "$WEB_PORT" --game "$SA_ROOT/wine/gta-sa" --mods "$OUT/webmods" \
        > "$OUT/serve.log" 2>&1 &
    WEB_PIDS="$WEB_PIDS $!"
    sleep 2
    url="http://127.0.0.1:$WEB_PORT/?autostart=1&mods=samp&proxy=ws://127.0.0.1:$PROXY_PORT"
    node "$ROOT/tools/web/headless.mjs" --url "$url" --out "$OUT/web" --seconds "${WEB_SECONDS:-150}" --every 5 \
        --gpu "${WEB_GPU:-swiftshader}" \
        --walk-after 3 --walk "${WEB_WALK:-y:1,x:3,y:1,x:3,y:1,x:45,w:10}" > "$OUT/headless.log" 2>&1 &
    GAME_PID=$!
    say "headless Chromium pid $GAME_PID ($url)"
else
    SA_MOD_DATA="$OUT/moddata" SA_GAME_STUB="${SA_GAME_STUB:-$STUBS}" SA_GAME_SCRIPT_SKIP=1 SA_GAME_PRESENT_GAME=1 \
        SA_GAME_PRESENT_EVERY="${SA_GAME_PRESENT_EVERY:-20}" \
        "$GAME_BIN" --game "$SA_ROOT/wine/gta-sa" --frames "$FRAMES" --start-in-world --mods "$MOD_DIRS" \
        --present-dir "$OUT/present" --report "$OUT/report.md" > "$OUT/sa-game.log" 2>&1 &
    GAME_PID=$!
    say "sa-game pid $GAME_PID ($FRAMES frames)"
fi

SEQ=0
CMDF="$SRV/scriptfiles/sampcov.txt"
cmd() {
    SEQ=$((SEQ + 1))
    echo "$SEQ $1" >> "$CMDF"
    wait_log "\[cov\] $SEQ (done|unknown)" 10 && say "server: $1" || say "command $SEQ ($1) not acknowledged"
}
if wait_log '\[cov\] connect id=[0-9]+ name=ModCJ' 150; then say "connected"; else say "FAIL: no connect"; fi
if [ "${SCENARIO:-join}" = world ] && wait_log '\[cov\] spawn id=' 150; then
    say "spawned"
    GLOG_LIVE="$OUT/sa-game.log"
    mkdir -p "$OUT/shots"
    rm -f "$OUT"/shots/*.png
    wait_count() { # REGEX N SECS
        local i
        for ((i = 0; i < $3 * 2; i++)); do
            [ "$(grep -cE "$1" "$SLOG" 2> /dev/null)" -ge "$2" ] && return 0
            [ -n "$GAME_PID" ] && ! kill -0 "$GAME_PID" 2> /dev/null && return 1
            sleep 0.5
        done
        return 1
    }
    client_wait() { # REGEX SECS
        local i
        for ((i = 0; i < $2 * 2; i++)); do
            grep -qE "$1" "$GLOG_LIVE" 2> /dev/null && return 0
            sleep 0.5
        done
        return 1
    }
    shot() {
        local f
        f="$(ls -t "$OUT"/present/*.png 2> /dev/null | head -1)"
        [ -n "$f" ] && cp "$f" "$OUT/shots/$1.png" && say "shot $1: $(basename "$f")"
    }
    spawn_again() { wait_count '\[cov\] spawn id=' "$1" 120; }
    # shellcheck source=world-scenario.sh
    source "$HERE/world-scenario.sh"
    sleep 4
    world_scenario
    kill "$GAME_PID" 2> /dev/null
elif wait_log '\[cov\] spawn id=' 150; then
    say "spawned"
    if [ "${BOT:-0}" = 1 ]; then
        "${SAMP_BOT:-$ROOT/samp-rs/target/release/samp-bot}" "127.0.0.1:$PORT" --name BotPlayer --mode idle \
            --seconds "${BOT_SECONDS:-30}" --auth-table "$AUTH" > "$OUT/samp-bot.log" 2>&1 &
        BOT_PID=$!
        say "samp-bot pid $BOT_PID"
    fi
    sleep 4
    cmd "msg|Welcome, the SA:MP mod: chat from open.mp"
    cmd "gametext|3|6000|~g~SA:MP ~w~mod"
    cmd "hp|60"
    cmd "armour|40"
    cmd "money|5000"
    cmd "weapon|24|100"
    cmd "time|22"
    cmd "weather|4"
    sleep 3
    # facing north: the scripted walk goes up the street, not into the safe house (its save marker opens a menu)
    cmd "pos|2495|-1680|13.5|0"
    POS_SEQ=$SEQ
    sleep 3
    cmd "npc|Bot|2492|-1684|13.5|105|0"
    sleep 3
    cmd "npcmove|$(grep -oE '\[cov\] [0-9]+ npc -?[0-9]+' "$SLOG" | tail -1 | awk '{print $4}')|2497|-1678|13.5"
    sleep 4
    cmd "time|12"
    cmd "weather|10"
    cmd "veh|560|2502|-1690|13.4|90"
    sleep 6
    IFS=';' read -ra EXTRA <<< "${EXTRA_CMDS:-}"
    [ -n "${CMDS:-}" ] && mapfile -t -O "${#EXTRA[@]}" EXTRA < <(grep -vE '^(#|$)' "$CMDS")
    for c in "${EXTRA[@]}"; do
        case "$c" in
            sleep\|*) sleep "${c#sleep|}" ;;
            shot\|*) shot="$(ls -t "$OUT/present" | grep -E '^present-[0-9]+[.]png$' | sed -n 2p)"
                [ -n "$shot" ] && cp "$OUT/present/$shot" "$OUT/shots/${c#shot|}.png" && say "shot ${c#shot|} ($shot)" ;;
            ?*) cmd "$c" ;;
        esac
    done
    if [ -n "$BOT_PID" ]; then
        wait "$BOT_PID" 2> /dev/null
        BOT_PID=
        say "samp-bot left"
    fi
else
    say "FAIL: no spawn"
fi
wait "$GAME_PID"
say "game exit $?"
GAME_PID=
grep -E '\[(join|part|connection)\]|\[cov\] (connect|class|spawn|state|where|npc|pickup|cp-enter|rcp-enter|loaded|[0-9]+ (done|obj)|disconnect)' "$SLOG" > "$OUT/server-events.txt"
say "server events: $(wc -l < "$OUT/server-events.txt") lines; PNGs: $(ls "$OUT/present" | wc -l)"
GLOG="$OUT/sa-game.log"
[ "$WEB" = 1 ] && GLOG="$OUT/web/game-log.txt"
python3 -I - "$SLOG" "$GLOG" "${POS_SEQ:-0}" "${BOT:-0}" "$SEQ" > "$OUT/goal.txt" << 'PY'
import math, re, sys
slog, glog, pos_seq, bot, seqs = open(sys.argv[1], errors="replace").read(), sys.argv[2], int(sys.argv[3]), sys.argv[4] == "1", int(sys.argv[5])
try:
    game = open(glog, errors="replace").read()
except OSError:
    game = ""
steps = [("connected", bool(re.search(r"\[cov\] connect id=\d+ name=ModCJ", slog))),
         ("spawned", "[cov] spawn id=" in slog),
         ("commands acknowledged", all(re.search(rf"\[cov\] {i} done", slog) for i in range(1, seqs + 1)))]
after = slog.split(f"[cov] {pos_seq} done", 1)[1] if pos_seq and f"[cov] {pos_seq} done" in slog else ""
far = max((math.dist((float(x), float(y)), (2495.0, -1680.0)) for x, y in re.findall(r"where st=1 pos=(-?[\d.]+),(-?[\d.]+),", after)), default=0.0)
steps.append((f"walk seen by the server ({far:.1f} m from the teleport point)", far >= 1.0))
if bot:
    m = re.search(r"player (\d+) \(skin \d+\) streamed in", game)
    steps.append(("the other player streamed in as a ped", m is not None))
    steps.append(("its ped removed when it left", m is not None and f"player {m.group(1)} left" in game))
for name, ok in steps:
    print(f"{'PASS' if ok else 'FAIL'} {name}")
sys.exit(0 if all(ok for _, ok in steps) else 1)
PY
GOAL=$?
cat "$OUT/goal.txt" | tee -a "$OUT/steps.log"
exit "$GOAL"
