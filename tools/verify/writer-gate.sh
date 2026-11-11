#!/usr/bin/env bash
# Writer gate (docs/verify/HOWTO.md §2): what a WRITER agent must pass before hand-back. Needs no game lane.
#
#   tools/verify/writer-gate.sh [BASE]
#
# BASE: the git revision the writer started from (default: `git merge-base HEAD master`). Steps, each a
# PASS/FAIL line, all run even after a failure (summary at the end):
#   hooks    every hook added since BASE is `#[hook(unverified = "BATCH", route = ..., hazards = ...)]`,
#            its batch has at most 60 hooks (tools/verify/queue.py gate)
#   queue    catalog/verify-queue.csv and docs/verify/QUEUE.md are regenerated (tools/verify/queue.py)
#   clippy   cargo clippy --workspace --all-targets -- -D warnings (host build of every crate and test)
#   i686     cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook (the in-process
#            DLL, every port's naked entry) + clippy -D warnings of sa-hook for that target
#   wasm32   cargo build --target wasm32-unknown-unknown -p sa-ports (every port crate)
#   hooktest cargo test -p sa-hook --lib (table vs exe and catalogue, callers, patch_refs, keycells, INF-20)
#   l1test   cargo test --release -p sa-oracle --test l1 (harness self-tests, preserve/route checks)
#   l1       for every new hook with route = "L1-pure": sa-oracle l1 0xADDR --backend native (the real CPU
#            under Wine; FAIL when the hook has no spec or the native backend is unavailable)
# Env: GATE_SKIP="wasm32 l1test ..." skips steps (say so in the hand-back), L1_CASES (default 1000000),
# CARGO_TARGET_DIR (default ./target), GATE_CLEAN=1 removes target/debug at the end (disk), MEM (systemd
# scope MemoryMax, default 6G), JOBS (cargo -j, default 8). Output: target/writer-gate/summary.txt + logs.
# Exit 0 = every step passed.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
cd "$ROOT" || exit 2
BASE="${1:-$(git merge-base HEAD master 2> /dev/null)}"
[ -n "$BASE" ] || { echo "writer-gate: no BASE (give the revision you started from)" >&2; exit 2; }
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="$ROOT/target/writer-gate"
mkdir -p "$OUT"
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
FAILS=0
skip() { case " ${GATE_SKIP:-} " in *" $1 "*) return 0 ;; esac; return 1; }
res() { # res STEP OK DETAIL
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %-8s %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
disk_ok() {
    local free
    free=$(df -BG --output=avail /home | tail -1 | tr -dc 0-9)
    [ "$free" -ge 20 ] || { echo "writer-gate: only ${free} GB free on /home (keep >= 20 GB; delete target/debug)" >&2; return 1; }
}
cap() { systemd-run --user --scope -q -p MemoryMax="${MEM:-6G}" -p MemorySwapMax=0 "$@"; }
cargo_step() { # cargo_step STEP DETAIL cargo-args...
    local step="$1" detail="$2"
    shift 2
    if skip "$step"; then printf 'SKIP %-8s %s\n' "$step" "$detail" | tee -a "$SUMMARY"; return; fi
    disk_ok || { res "$step" 0 "not run: disk"; return; }
    cap cargo "$@" > "$OUT/$step.log" 2>&1
    local rc=$?
    res "$step" "$([ $rc = 0 ] && echo 1)" "$detail$([ $rc = 0 ] || echo "; $(grep -m1 -E '^(error|warning)' "$OUT/$step.log"); log $OUT/$step.log")"
}
J="-j${JOBS:-8}"
echo "writer gate: base $(git rev-parse --short "$BASE"), target dir $CARGO_TARGET_DIR" | tee -a "$SUMMARY"

# ---- hooks + queue -------------------------------------------------------------------------------
python3 -I "$HERE/queue.py" gate "$BASE" > "$OUT/hooks.log" 2>&1
rc=$?
res hooks "$([ $rc = 0 ] && echo 1)" "$(grep -m1 '^gate:' "$OUT/hooks.log")$([ $rc = 0 ] || echo "; $(grep -c '^FAIL' "$OUT/hooks.log") problems, e.g. $(grep -m1 '^FAIL' "$OUT/hooks.log" | cut -c6-200)")"
grep -E '^(note|promoted):' "$OUT/hooks.log" | sed 's/^/    /' | tee -a "$SUMMARY"
python3 -I "$HERE/queue.py" --check > "$OUT/queue.log" 2>&1
res queue "$([ $? = 0 ] && echo 1)" "$(tail -1 "$OUT/queue.log")"

# ---- builds and tests ----------------------------------------------------------------------------
cargo_step clippy "host clippy -D warnings, all targets" clippy "$J" --workspace --all-targets -- -D warnings
cargo_step i686 "i686-pc-windows-gnu release build of sa-launcher + sa-hook" \
    build "$J" --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
if ! skip i686; then
    cargo_step i686clip "i686 clippy -D warnings (sa-hook and every port crate)" \
        clippy "$J" --release --target i686-pc-windows-gnu -p sa-hook -- -D warnings
fi
cargo_step wasm32 "wasm32-unknown-unknown build of sa-ports (every port crate)" \
    build "$J" --target wasm32-unknown-unknown -p sa-ports
cargo_step hooktest "sa-hook lib tests" test "$J" -p sa-hook --lib
cargo_step l1test "sa-oracle --test l1" test "$J" --release -p sa-oracle --test l1

# ---- L1 runs of the new L1-pure hooks --------------------------------------------------------------
L1S="$(sed -n 's/^L1 \(0x[0-9a-f]*\) .*/\1/p' "$OUT/hooks.log")"
if [ -z "$L1S" ]; then
    printf 'PASS %-8s %s\n' l1 "no new L1-pure hook" | tee -a "$SUMMARY"
elif skip l1; then
    printf 'SKIP %-8s %s\n' l1 "$(echo "$L1S" | wc -w) L1-pure hooks not run" | tee -a "$SUMMARY"
else
    cap cargo build "$J" --release -p sa-oracle > "$OUT/l1-build.log" 2>&1
    ORACLE="$CARGO_TARGET_DIR/release/sa-oracle"
    "$ORACLE" list > "$OUT/l1-list.txt" 2>&1
    for a in $L1S; do
        if ! grep -qiE "\s$a\s" "$OUT/l1-list.txt"; then
            res l1 0 "$a: route L1-pure but no L1 spec (add one in crates/sa-oracle/src/specs_*.rs, or change the route)"
            continue
        fi
        cap "$ORACLE" l1 "$a" --backend native --cases "${L1_CASES:-1000000}" > "$OUT/l1-$a.log" 2>&1
        rc=$?
        res l1 "$([ $rc = 0 ] && echo 1)" "$a: $(grep -m1 -iE 'mismatch|error|native' "$OUT/l1-$a.log" | cut -c1-200); log $OUT/l1-$a.log"
    done
fi

[ "${GATE_CLEAN:-0}" = 1 ] && rm -rf "$CARGO_TARGET_DIR/debug"
echo "writer gate: $FAILS failure(s); summary $SUMMARY; $(du -sh "$CARGO_TARGET_DIR" 2> /dev/null | cut -f1) in $CARGO_TARGET_DIR, $(df -h --output=avail /home | tail -1 | tr -d ' ') free on /home"
exit $((FAILS > 0))
