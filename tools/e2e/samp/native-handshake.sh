#!/usr/bin/env bash
# Native networking check (docs/samp/native-plan.md §6): starts a private copy of our open.mp 1.6.7 server
# (re/samp/server, gitignored) on a local UDP port and runs samp-rs's `native_handshake` host test against it
# as a 32-bit Linux process (i686-unknown-linux-gnu): cookie exchange, RakNet connection, auth key,
# PlayerConnect, InitGame. No game, no Wine.
#
#   tools/e2e/samp/native-handshake.sh SAMP_RS_DIR [AUTH_TABLE]
#
# SAMP_RS_DIR is a samp-rs checkout with the test (branch sa-engine/adapter); AUTH_TABLE defaults to
# ~/samp-rs/data/auth_table.txt (third-party data, never committed). Port: SAMP_PORT (default 7812; samp-rs
# uses 7777/7790, samp-cov.sh 7810). Lock: target/locks/samp-native.lock. Server log: target/samp-native/.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
SA_ROOT="${SA_ROOT:-$ROOT}"
SAMP_RS="${1:?usage: native-handshake.sh SAMP_RS_DIR [AUTH_TABLE]}"
AUTH="${2:-$HOME/samp-rs/data/auth_table.txt}"
PORT="${SAMP_PORT:-7812}"
SERVER_SRC="$SA_ROOT/re/samp/server"
OUT="$ROOT/target/samp-native"
SRV="$OUT/server"
[ -x "$SERVER_SRC/omp-server" ] || { echo "native-handshake: $SERVER_SRC/omp-server missing" >&2; exit 1; }
[ -f "$AUTH" ] || { echo "native-handshake: auth table $AUTH missing" >&2; exit 1; }

mkdir -p "$ROOT/target/locks" "$OUT"
exec {LOCK_FD}> "$ROOT/target/locks/samp-native.lock"
flock "$LOCK_FD"
if ss -lun | grep -q ":$PORT "; then echo "native-handshake: UDP $PORT busy" >&2; exit 1; fi

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
(cd "$SRV" && exec ./omp-server > "$SLOG" 2>&1) &
SRV_PID=$!
trap 'kill $SRV_PID 2> /dev/null; wait $SRV_PID 2> /dev/null || true' EXIT
for _ in $(seq 40); do
    grep -q 'Legacy Network started' "$SLOG" 2> /dev/null && break
    sleep 0.5
done
grep -q 'Legacy Network started' "$SLOG" || { echo "native-handshake: server did not start" >&2; cat "$SLOG"; exit 1; }

cd "$SAMP_RS"
SAMP_TEST_SERVER="127.0.0.1:$PORT" SAMP_AUTH_TABLE="$AUTH" \
    cargo test -p samp-client --target i686-unknown-linux-gnu --test native_handshake -- --nocapture
sleep 1
echo "--- server log (connection lines)"
grep -iE 'connect|join|NativeLinux' "$SLOG" || true
