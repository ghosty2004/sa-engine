#!/usr/bin/env bash
# The browser build joins a local open.mp server (WEB-SAMP): starts a private copy of our open.mp server, sa-udp-proxy
# and the web server, all on 127.0.0.1 only, and keeps them running until Ctrl-C.
#
#   FEATURES=samp tools/web/build.sh        once per code change (the wasm32 build with the SA:MP client)
#   tools/web/samp-demo.sh                  then open the URL it prints
#
# Inputs (gitignored, never committed): $SA_ROOT/re/samp/server (open.mp), $SA_ROOT/wine/gta-sa (your own game
# folder), the auth table AUTH_TABLE (default ~/samp-rs/data/auth_table.txt, third-party data, read only).
# Ports: SAMP_PORT (UDP, default 7813), PROXY_PORT (WebSocket, default 7781), WEB_PORT (HTTP, default 8644).
# The server runs tools/e2e/samp/sampcov.pwn: commands go to $OUT/server/scriptfiles/sampcov.txt (one
# "SEQ CMD|ARGS" line each, e.g. `echo "1 msg|hello" >> .../sampcov.txt`), its "[cov]" lines are in $OUT/server.log.
# Lock: $SA_ROOT/target/locks/$SAMP_LOCK (default samp-native.lock, shared with tools/e2e/samp/native-join.sh), held
# while the demo runs; another name with other ports runs beside a running demo.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
SA_ROOT="${SA_ROOT:-$ROOT}"
# a git worktree has no re/ or wine/: use the main checkout's
if [ ! -d "$SA_ROOT/re" ]; then
    SA_ROOT="$(cd "$(git -C "$ROOT" rev-parse --git-common-dir)/.." && pwd)"
fi
OUT="$(mkdir -p "${OUT:-$ROOT/target/web-samp}" && cd "${OUT:-$ROOT/target/web-samp}" && pwd)"
AUTH="${AUTH_TABLE:-$HOME/samp-rs/data/auth_table.txt}"
PORT="${SAMP_PORT:-7813}"
PROXY_PORT="${PROXY_PORT:-7781}"
WEB_PORT="${WEB_PORT:-8644}"
GAME="${GAME_DIR:-$SA_ROOT/wine/gta-sa}"
PROXY_BIN="${SA_UDP_PROXY:-$ROOT/target/release/sa-udp-proxy}"
SERVER_SRC="$SA_ROOT/re/samp/server"
SRV="$OUT/server"
SLOG="$OUT/server.log"
[ -x "$SERVER_SRC/omp-server" ] || { echo "samp-demo: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "samp-demo: auth table $AUTH missing" >&2; exit 1; }
[ -f "$GAME/gta_sa.exe" ] || { echo "samp-demo: $GAME has no gta_sa.exe (GAME_DIR=...)" >&2; exit 1; }
[ -f "$ROOT/web/pkg/sa_game_bg.wasm" ] || { echo "samp-demo: web/pkg missing (FEATURES=samp tools/web/build.sh)" >&2; exit 1; }
if [ ! -x "$PROXY_BIN" ]; then
    (cd "$ROOT" && cargo build --release -p sa-udp-proxy) || exit 1
fi

mkdir -p "$SA_ROOT/target/locks"
exec {LOCK_FD}> "$SA_ROOT/target/locks/${SAMP_LOCK:-samp-native.lock}"
flock -n "$LOCK_FD" || { echo "samp-demo: waiting for target/locks/samp-native.lock" >&2; flock "$LOCK_FD"; }
if ss -lun | grep -q "127.0.0.1:$PORT "; then echo "samp-demo: UDP $PORT busy" >&2; exit 1; fi

mkdir -p "$SRV"
rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
( cd "$SRV" && LD_LIBRARY_PATH=qawno ./qawno/pawncc "$ROOT/tools/e2e/samp/sampcov.pwn" -iqawno/include \
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

PIDS=()
cleanup() {
    for p in "${PIDS[@]}"; do kill "$p" 2> /dev/null; done
    wait 2> /dev/null
}
trap cleanup EXIT INT TERM
(cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1) &
PIDS+=($!)
"$PROXY_BIN" "127.0.0.1:$PORT" --bind "127.0.0.1:$PROXY_PORT" > "$OUT/proxy.log" 2>&1 &
PIDS+=($!)
python3 -I "$HERE/serve.py" --port "$WEB_PORT" --game "$GAME" --samp-auth "$AUTH" > "$OUT/web.log" 2>&1 &
PIDS+=($!)
for _ in $(seq 40); do grep -q 'Legacy Network started' "$SLOG" 2> /dev/null && break; sleep 0.5; done
grep -q 'Legacy Network started' "$SLOG" || { echo "samp-demo: the server did not start" >&2; cat "$SLOG"; exit 1; }
echo "samp-demo: open.mp on udp 127.0.0.1:$PORT (log $SLOG), sa-udp-proxy ws://127.0.0.1:$PROXY_PORT, web http://127.0.0.1:$WEB_PORT/"
echo "samp-demo: open http://127.0.0.1:$WEB_PORT/?autostart=1&backend=gl&samp=127.0.0.1:$PORT&proxy=ws://127.0.0.1:$PROXY_PORT&name=WebCJ"
wait -n "${PIDS[@]}"
echo "samp-demo: a process ended; stopping" >&2
