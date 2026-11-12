#!/usr/bin/env bash
# Browser networking check (docs/samp/web-proxy.md): the native-handshake.sh join, but every datagram goes
# through sa-udp-proxy's WebSocket path:
#
#   samp-rs native_handshake test --UDP--> shim 127.0.0.2:PORT --WS--> sa-udp-proxy --UDP--> open.mp 127.0.0.1:PORT
#
# The shim listens on the server's port number on 127.0.0.2 because SA:MP keys its datagram cipher on the
# server port. Starts a private copy of our open.mp 1.6.7 server (re/samp/server, gitignored), the proxy (only
# the server allowed) and the shim, then runs samp-rs's `native_handshake` host test (32-bit Linux) against
# the shim. samp-rs is only built and run, never edited.
#
#   tools/e2e/samp/proxy-handshake.sh SAMP_RS_DIR [AUTH_TABLE]
#
# SAMP_RS_DIR: a samp-rs checkout that has crates/samp-client/tests/native_handshake.rs (branch
# sa-engine/adapter with step 4). AUTH_TABLE defaults to ~/samp-rs/data/auth_table.txt (third-party data, never
# committed). Port: SAMP_PORT (default 7812), WebSocket port WS_PORT (default 7781). SA_ROOT: the sa-engine
# checkout holding re/ and the shared lock (default: the main checkout of this worktree, else this tree).
# Lock: $SA_ROOT/target/locks/samp-native.lock. Logs: target/samp-proxy/.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
MAIN="$(git -C "$ROOT" worktree list --porcelain 2> /dev/null | awk 'NR==1 {print $2}')"
SA_ROOT="${SA_ROOT:-${MAIN:-$ROOT}}"
SAMP_RS="${1:?usage: proxy-handshake.sh SAMP_RS_DIR [AUTH_TABLE]}"
AUTH="${2:-$HOME/samp-rs/data/auth_table.txt}"
PORT="${SAMP_PORT:-7812}"
WS_PORT="${WS_PORT:-7781}"
SERVER_SRC="$SA_ROOT/re/samp/server"
OUT="$ROOT/target/samp-proxy"
SRV="$OUT/server"
[ -x "$SERVER_SRC/omp-server" ] || { echo "proxy-handshake: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "proxy-handshake: auth table $AUTH missing" >&2; exit 1; }
[ -f "$SAMP_RS/crates/samp-client/tests/native_handshake.rs" ] || {
    echo "proxy-handshake: $SAMP_RS has no native_handshake test" >&2
    exit 1
}

(cd "$ROOT" && cargo build -j6 --release -p sa-udp-proxy)
PROXY="$ROOT/target/release/sa-udp-proxy"

mkdir -p "$SA_ROOT/target/locks" "$OUT"
exec {LOCK_FD}> "$SA_ROOT/target/locks/samp-native.lock"
flock "$LOCK_FD"
if ss -lun | grep -qE ":$PORT\b"; then echo "proxy-handshake: UDP $PORT busy" >&2; exit 1; fi
if ss -ltn | grep -qE ":$WS_PORT\b"; then echo "proxy-handshake: TCP $WS_PORT busy" >&2; exit 1; fi

rsync -a --delete --exclude log.txt "$SERVER_SRC/" "$SRV/"
python3 -I - "$SRV/config.json" "$PORT" << 'PY'
import json, sys
p, port = sys.argv[1], int(sys.argv[2])
d = json.load(open(p))
d["announce"] = False
d["network"]["port"] = port
d["network"]["bind"] = "127.0.0.1"
d["artwork"]["enable"] = False
d["rcon"]["enable"] = False
json.dump(d, open(p, "w"), indent=4)
PY
SLOG="$OUT/server.log"
PLOG="$OUT/proxy.log"
HLOG="$OUT/shim.log"
PIDS=()
cleanup() {
    for p in "${PIDS[@]}"; do kill "$p" 2> /dev/null || true; done
    for p in "${PIDS[@]}"; do wait "$p" 2> /dev/null || true; done
}
trap cleanup EXIT
(cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1) &
PIDS+=($!)
"$PROXY" --bind "127.0.0.1:$WS_PORT" "127.0.0.1:$PORT" > "$PLOG" 2>&1 &
PIDS+=($!)
"$PROXY" shim --listen "127.0.0.2:$PORT" --proxy "ws://127.0.0.1:$WS_PORT" --target "127.0.0.1:$PORT" > "$HLOG" 2>&1 &
PIDS+=($!)
for _ in $(seq 40); do
    grep -q 'Legacy Network started' "$SLOG" 2> /dev/null && break
    sleep 0.5
done
grep -q 'Legacy Network started' "$SLOG" || { echo "proxy-handshake: server did not start" >&2; cat "$SLOG"; exit 1; }
grep -q 'listening on' "$PLOG" || sleep 1

cd "$SAMP_RS"
SAMP_TEST_SERVER="127.0.0.2:$PORT" SAMP_AUTH_TABLE="$AUTH" \
    cargo test -p samp-client --target i686-unknown-linux-gnu --test native_handshake -- --nocapture
sleep 1
cleanup
trap - EXIT
echo "--- proxy log"
cat "$PLOG"
echo "--- shim log"
cat "$HLOG"
echo "--- server log (connection lines)"
grep -iE 'connect|join|NativeLinux' "$SLOG" || true
