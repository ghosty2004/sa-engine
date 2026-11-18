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
#   bindings the generated `orig!` bindings (crates/*/src/bindings.rs) match catalog/bindings.csv, no module
#            declares a bound address again, and no address is declared in two modules without a binding (C
#            varargs excepted, bindings.VARARGS) (tools/gen/bindings.py --check)
#   conv     every `orig!` declaration of a registered address passes its argument words where the port reads
#            them (the wasm32 dispatch's word order; tools/verify/convcheck.py)
#   state    catalog/state.csv claims implemented/L1/L2/L3 only for registered or documented Rust ports
#            (tools/verify/state_check.py) and lowers no state of BASE (progress.py guard)
#   dupes    no cross-crate copy of a function beyond tools/verify/dupes-allow.txt (tools/verify/dupes.py --check)
#   cfg      platform cfgs only in backends / the binding layer beyond tools/verify/cfg-baseline.txt (cfg_check.py)
#   gen      tools/gen/{patterns,ai_patterns,audio_patterns}.py --check (generated ports match the exe + catalogue)
#   layers   crate dependencies follow ADR-003 §3.4's layers (tools/lib/layers.py, tools/verify/crate_layers.py)
#   clippy   cargo clippy --workspace --all-targets -- -D warnings (host build of every crate and test)
#   i686     cargo build --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook (the in-process
#            DLL, every port's naked entry) + clippy -D warnings of sa-hook for that target
#   wasm32   cargo build --target wasm32-unknown-unknown -p sa-ports (every port crate) and the mod crates
#            (sa-mod-api, sa-mod, sa-mod-host, the hello, trainer-hud and courier example mods, the first-party
#            resolution mod; MOD-2)
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
# ADR-003 §3.2/§3.5: the generated orig! bindings match catalog/bindings.csv and no module repeats one
python3 -I "$ROOT/tools/gen/bindings.py" --check > "$OUT/bindings.log" 2>&1
res bindings "$([ $? = 0 ] && echo 1)" "$(grep -m1 '^bindings:' "$OUT/bindings.log")$(grep -m1 '^FAIL' "$OUT/bindings.log" | sed 's/^/; /')"
# every orig! declaration of a registered address passes its words where the port reads them (the wasm32
# dispatch ignores the convention; a thiscall declaration of a stdcall port shifts every argument)
python3 -I "$HERE/convcheck.py" > "$OUT/conv.log" 2>&1
res conv "$([ $? = 0 ] && echo 1)" "$(tail -1 "$OUT/conv.log")$(grep -m1 '^FAIL' "$OUT/conv.log" | cut -c1-160 | sed 's/^/; /')"
# catalog/state.csv claims a port only where one exists
python3 -I "$HERE/state_check.py" > "$OUT/state.log" 2>&1
rc=$?
# ... and the branch never lowers a state the base had (a state_check.py --fix demotion excepted)
git show "$BASE:catalog/state.csv" > "$OUT/state-base.csv" 2>/dev/null &&
    python3 -I "$ROOT/tools/catalog/progress.py" guard "$OUT/state-base.csv" "$ROOT/catalog/state.csv" --allow-hand >> "$OUT/state.log" 2>&1 || rc=1
res state "$([ $rc = 0 ] && echo 1)" "$(head -1 "$OUT/state.log"); $(grep -m1 '^state guard' "$OUT/state.log")$(grep -m1 '^FAIL' "$OUT/state.log" | cut -c1-120 | sed 's/^/; /')"
# ADR-003 §4: no new cross-crate copies (dupes.py's allow-list only shrinks), platform cfgs only in backends
# (cfg-baseline.txt only shrinks), crate dependencies in layer order (tools/lib/layers.py)
python3 -I "$HERE/dupes.py" --check > "$OUT/dupes.log" 2>&1
res dupes "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/dupes.log")$(grep -m1 '^FAIL' "$OUT/dupes.log" | cut -c1-160 | sed 's/^/; /')"
python3 -I "$HERE/cfg_check.py" > "$OUT/cfg.log" 2>&1
res cfg "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/cfg.log")$(grep -m1 '^FAIL' "$OUT/cfg.log" | sed 's/^/; /')"
# ADR-003 §3.5: the byte-pattern generators' output matches what they produce from the exe and the catalogue
# (capstone from the project venv: this checkout's .venv, else the main checkout's)
PYV="$ROOT/.venv/bin/python"; [ -x "$PYV" ] || PYV="${ROOT%%/.claude/worktrees/*}/.venv/bin/python"
if skip gen; then
    printf 'SKIP %-8s %s\n' gen "generator --check" | tee -a "$SUMMARY"
elif [ ! -x "$PYV" ]; then
    res gen 0 "no project venv (.venv) with capstone"
else
    : > "$OUT/gen.log"; bad=""
    for g in patterns ai_patterns audio_patterns; do
        "$PYV" -I "$ROOT/tools/gen/$g.py" --check >> "$OUT/gen.log" 2>&1 || bad="$bad $g"
    done
    res gen "$([ -z "$bad" ] && echo 1)" "patterns / ai_patterns / audio_patterns --check$([ -z "$bad" ] || echo ":$bad differ; $(grep -m1 '^differs' "$OUT/gen.log"); log $OUT/gen.log")"
fi
python3 -I "$HERE/crate_layers.py" > "$OUT/layers.log" 2>&1
res layers "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/layers.log")$(grep -m1 '^FAIL' "$OUT/layers.log" | sed 's/^/; /')"

# commit messages since BASE follow Conventional Commits (CLAUDE.md; commits older than the rule are not checked)
python3 -I "$HERE/commit_lint.py" --range "$BASE..HEAD" > "$OUT/commits.log" 2>&1
res commits "$([ $? = 0 ] && echo 1)" "$(tail -1 "$OUT/commits.log")$(grep -m1 '^FAIL' "$OUT/commits.log" | cut -c1-160 | sed 's/^/; /')"

# ---- builds and tests ----------------------------------------------------------------------------
cargo_step clippy "host clippy -D warnings, all targets" clippy "$J" --workspace --all-targets -- -D warnings
cargo_step i686 "i686-pc-windows-gnu release build of sa-launcher + sa-hook" \
    build "$J" --release --target i686-pc-windows-gnu -p sa-launcher -p sa-hook
if ! skip i686; then
    cargo_step i686clip "i686 clippy -D warnings (sa-hook and every port crate)" \
        clippy "$J" --release --target i686-pc-windows-gnu -p sa-hook -- -D warnings
fi
cargo_step wasm32 "wasm32-unknown-unknown build of sa-ports (every port crate) and the mod crates" \
    build "$J" --target wasm32-unknown-unknown -p sa-ports -p sa-mod-api -p sa-mod -p sa-mod-host -p sa-mod-hello -p sa-mod-resolution -p sa-mod-trainer-hud -p sa-mod-courier
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
