#!/usr/bin/env bash
# L2 + L3 verification of a set of hooked ports on a corpus recording (CORE-L2; for every porting batch).
#
#   tools/e2e/port-l2.sh <HOOKS|ports> [RECORDING] [--runs "plain l2 rust hookoff split"] [--max N]
#                        [--window-all HOOKS] [--tag NAME]
#
# HOOKS: comma list of hook names or addresses (any hook of sa-hook's table, see tools/e2e/hook-names.py),
# `@FILE` (one per line, e.g. tools/e2e/lists/core-pilots.txt), `state:STATE[:DOC]` (docs/arch/*.md or DOC.md table rows
# in that state, e.g. state:implemented-needs-L2:core-port = PORT-CORE's 191), or `ports` = every engine-crate
# port (`#[hook]` registrations, sa_core::hooks::ALL, ...).
# RECORDING: an input recording made from process start in determinism mode (default: the grove walk
# $SA_ROOT/target/e2e/lane-2/inf4/rec.sarec, recorded first from tools/e2e/programs/grove-walk.txt
# when missing). Every run replays it from process start with SA_HOOK_PORTS=orig (all ports installed)
# and every hook of the base set (BASE_ORIG below + the list) on the original unless stated:
#   plain    every hook on the original: the all-original reference trace (17 subsystem hashes/frame)
#   l2       L2 on the listed hooks (SA_L2), the game continuing with the original's results; read/write
#            sets from `sa-oracle l2 spec` plus the lines of tools/e2e/l2-extras.txt for these hooks
#            (heap regions such as pool byte maps); SA_L2_WINDOW=data
#   l2all    (only with --window-all HOOKS) L2 on those hooks with SA_L2_WINDOW=all: the used heap arena
#            is watched too, so writes to heap memory outside the declared regions are reported
#   rust     every listed hook on Rust (`ports`: SA_HOOK_PORTS=rust; otherwise SA_HOOK_MODE=NAME=rust)
#   hookoff  (opt-in, one run per hook) every listed hook on Rust except that one (PLAN §5 L3 hook-off)
#   split    (opt-in, INF-17) two split-run recordings: `splito` (SA_L2_SPLIT=orig: every sampled call of
#            the listed hooks runs the original ONCE and is recorded) and `splitr` (SA_L2_SPLIT=rust: the
#            Rust implementations); `sa-oracle l2 split-report` pairs the calls by (function, ordinal),
#            checks that their inputs are identical and compares the outputs (for functions that cannot
#            run twice on the same OS state; docs/re/l2-oracle.md). SPLIT_PER_FRAME (default 1000);
#            SPLIT_DUMP_SEQ=N,… dumps the watched windows at those cases (`sa-oracle l2 dump-diff`)
# and checks:
#   (a) per hook: >= MAX cases or every call compared, 0 mismatches, 0 undeclared writes; a hook that
#       got no call in the run is reported as NOCALLS (not a failure, but no evidence either)
#   (b) l2 trace == plain trace on every frame (the oracle does not perturb the run)
#   (c) rust trace == plain trace on every frame and all 17 subsystems (L3); `randcalls` listed apart
#   (d) hookoff traces == plain trace
#   (e)  split: splito trace == plain, splitr trace == plain, split-report verdict per hook
#   host: `sa-oracle l2 report` of each spool (0 mismatches)
#   spool quota (INF-18): sa-hook stops spooling at L2_SPOOL_MAX bytes (SA_L2_SPOOL_MAX, default 2 GiB);
#   a run that reached it is a FAIL line (cases compared before it stay valid); `df` is checked before
#   every run (>= 20 GB free on /home)
#
# Env: LANE (default 2), SA_ROOT, L2_PER_FRAME (default 4), SPLIT_PER_FRAME (default 1000),
# L2_SPOOL_MAX (bytes, default sa-hook's 2 GiB), L2_EXTRA (extra regions appended to every run's
# SA_L2_EXTRA), RUN_TIMEOUT (s, default 1500),
# L2_SPOOL_DISCARD=1 (drain the binary spool through a FIFO: no host report, bounded disk use).
# Build first (see inf8-l2.sh). Output: target/e2e/lane-N/port-l2[-TAG]/ with summary.txt and
# results.tsv (one row per hook: addr name calls cases mismatches undeclared_cases l2 l3).
# Exit 0 = no failure (NOCALLS rows do not fail the run).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
LIST=""
REC=""
RUNS="plain l2 rust"
MAX="${L2_MAX:-1000}"
WALL=""
TAG=""
RUN_TIMEOUT="${RUN_TIMEOUT:-1500}"
usage() { sed -n '2,5p' "$0" >&2; exit 2; }
while [ $# -gt 0 ]; do
    case "$1" in
        --runs) RUNS="$2"; shift 2 ;;
        --max) MAX="$2"; shift 2 ;;
        --window-all) WALL="$2"; shift 2 ;;
        --tag) TAG="-$2"; shift 2 ;;
        -h | --help) usage ;;
        -*) usage ;;
        *) if [ -z "$LIST" ]; then LIST="$1"; elif [ -z "$REC" ]; then REC="$1"; else usage; fi; shift ;;
    esac
done
[ -n "$LIST" ] || usage
[ -n "$WALL" ] && case " $RUNS " in *" l2all "*) ;; *) RUNS="$RUNS l2all" ;; esac
LANE="${LANE:-2}"
# shellcheck source=lib.sh
source "$HERE/lib.sh"
TGT="${CARGO_TARGET_DIR:-$E2E_SCRIPT_ROOT/target}/release"
SATOOL="$TGT/satool"
ORACLE="$TGT/sa-oracle"
for b in "$SATOOL" "$ORACLE"; do
    [ -x "$b" ] || { echo "port-l2: $b missing (cargo build --release -p sa-oracle-format -p sa-oracle)" >&2; exit 1; }
done
[ -n "$REC" ] || REC="$SA_ROOT/target/e2e/lane-2/inf4/rec.sarec"

# ---- the hook set ------------------------------------------------------------------------------------
HOOKTAB="$(python3 -I "$HERE/hook-names.py" "$E2E_SCRIPT_ROOT" "$LIST")" || exit 2
mapfile -t NAMES < <(printf '%s\n' "$HOOKTAB" | cut -f3)
mapfile -t ADDRS < <(printf '%s\n' "$HOOKTAB" | cut -f1)
join_by() { local IFS="$1"; shift; printf '%s' "$*"; }
L2NAMES="$(join_by , "${NAMES[@]}")"
# hooks are passed to sa-hook by address (unique, short environment)
L2LIST="$(join_by , "${ADDRS[@]}")"
# sa-hook's own Core ports that start on Rust (or are ports of earlier batches): the reference runs
# them on the original too, so `plain` is all-original. CTimer::Update stays the frame-boundary
# passthrough (it is not a port).
BASE_ORIG=(CGeneral::LimitRadianAngle CGameLogic::IsCoopGameGoingOn CGeneral::LimitAngle
    CGeneral::GetRadianAngleBetweenPoints- CGeneral::GetATanOfXY CGeneral::SolveQuadratic CVector::Normalise)
mapfile -t GROUPS_ < <(printf '%s\n' "$HOOKTAB" | cut -f2)
modes() { # modes RUST_NAME... → SA_HOOK_MODE value: BASE_ORIG and listed sa-hook-local hooks on orig,
    # the arguments on rust (port hooks start on orig with SA_HOOK_PORTS=orig and are not repeated)
    local -A rust=() seen=()
    local out=() n k
    for n in "$@"; do rust[$n]=1; done
    for n in "${BASE_ORIG[@]}"; do seen[$n]=1; out+=("$n=$([ -n "${rust[$n]:-}" ] && echo rust || echo orig)"); done
    for k in "${!NAMES[@]}"; do
        n="${NAMES[$k]}"
        [ -n "${seen[$n]:-}" ] && continue
        if [ -n "${rust[$n]:-}" ]; then out+=("${ADDRS[$k]}=rust")
        elif [ "${GROUPS_[$k]}" = local ]; then out+=("${ADDRS[$k]}=orig"); fi
    done
    join_by , "${out[@]}"
}
# The extra regions (tools/e2e/l2-extras.txt) of the hooks in a list. `HOOK@REGION` lines apply to that
# hook; `doc:DOC@REGION` lines to the hooks of a docs/arch/DOC.md function table: passed once without a
# hook prefix (so they apply to every listed hook; a per-hook expansion for a few hundred hooks would
# exceed the Windows environment limit) when at least one listed hook is in DOC.
# L2_EXTRA (env) is appended as is (self-tests, e.g. the INF-18 quota test's huge region).
extras_for() {
    local want="$1" line doc rest n hit
    local -A docset=()
    {
    grep -v '^\s*#' "$HERE/l2-extras.txt" | grep -v '^\s*$' | while IFS= read -r line; do
        case "$line" in
            doc:*@*)
                doc="${line%%@*}"; doc="${doc#doc:}"; rest="${line#*@}"
                if [ -z "${docset[$doc]+x}" ]; then
                    docset[$doc]=",$(python3 -I "$HERE/hook-names.py" "$E2E_SCRIPT_ROOT" "doc:$doc" | cut -f3 | paste -sd,),"
                fi
                hit=0
                IFS=, read -r -a wl <<< "$want"
                for n in "${wl[@]}"; do
                    case "${docset[$doc]}" in *",$n,"*) hit=1; break ;; esac
                done
                [ "$hit" = 1 ] && printf '%s,' "$rest"
                ;;
            *) case ",$want," in *",${line%@*},"*) printf '%s,' "$line" ;; esac ;;
        esac
    done
    [ -n "${L2_EXTRA:-}" ] && printf '%s,' "$L2_EXTRA"
    } | sed 's/,$//'
}

OUT="$E2E_OUT/port-l2$TAG"
mkdir -p "$OUT"
FAILS=0
SUMMARY="$OUT/summary.txt"
: > "$SUMMARY"
check() { # check NAME OK? DETAIL
    local r=PASS
    [ "$2" = 1 ] || { r=FAIL; FAILS=$((FAILS + 1)); }
    printf '%s %s: %s\n' "$r" "$1" "$3" | tee -a "$SUMMARY"
}
note() { printf '%s\n' "$*" | tee -a "$SUMMARY"; }
finish() {
    e2e_cleanup
    e2e_say "$FAILS failure(s); summary $SUMMARY"
}

e2e_disk_check || exit 1
e2e_lane_lock
e2e_lane_setup || exit 1
e2e_install || exit 1
trap finish EXIT
wineserver -k 2> /dev/null || true

DET="$GTA_DIR/det"
mkdir -p "$DET"
win() { winepath -w "$1"; }
"$ORACLE" l2 spec --out "$OUT/l2.spec" > /dev/null || exit 1
# The spec comes from the host sa-oracle's hook table: a host binary older than the DLL has no entry
# for newer hooks, so their globals would be missing and every write to them reported as undeclared
# (seen 2026-11-10: CPhysical::AddToMovingList's 0xB9ACC8 with a spec of 1,139 instead of 1,201 hooks).
missing="$(for a in "${ADDRS[@]}"; do grep -qi "^fn $(printf '0x%x' "$a") " "$OUT/l2.spec" || printf '%s ' "$a"; done)"
[ -z "$missing" ] || { echo "port-l2: $ORACLE has no spec entry for: $missing(stale host build? cargo build --release -p sa-oracle)" >&2; exit 1; }
cp "$OUT/l2.spec" "$DET/l2.spec"

run_game() { # run_game NAME VAR=VALUE... : one determinism-mode run from process start to the end frame
    local name="$1" start last="" st kv q
    shift
    if ! e2e_disk_check; then
        check "run $name: >= 20 GB free on /home before the run" 0 "$(df -h /home | awk 'NR == 2 { print $4 }') free; run skipped"
        return
    fi
    rm -f "$DET/$name.trace" "$DET/$name.spool" "$DET/$name.spool.txt" "$DET/$name".spool.seq*.win "$GTA_DIR/sa-hook.log"
    local drain=""
    if [ "${L2_SPOOL_DISCARD:-0}" = 1 ]; then
        # the binary spool (several MB per case) goes to a FIFO that is drained; verdicts stay in the
        # log summary and the text report (NAME.spool.txt)
        mkfifo "$DET/$name.spool" && { cat "$DET/$name.spool" > /dev/null & drain=$!; }
    fi
    (
        export SA_DET=1 SA_DET_EXIT=1 SA_DET_TRACE="$(win "$DET/$name.trace")" SA_HOOK_PORTS=orig
        for kv in "$@"; do export "${kv?}"; done
        e2e_game_start
        wait "$E2E_GAME_PID"
    ) &
    local pid=$!
    printf '%s\n' "$*" > "$OUT/$name.env"
    e2e_say "run $name"
    start=$(date +%s)
    while kill -0 "$pid" 2> /dev/null; do
        st="$(e2e_ctl det)"
        [ -n "$st" ] && last="$st"
        # A crash under Wine starts winedbg, which keeps the process (and the control channel)
        # suspended until the timeout: stop at once and keep the fault line.
        if grep -q -E 'Unhandled (page fault|exception)' "$E2E_OUT/wine.log" 2> /dev/null; then
            e2e_say "run $name: crashed: $(grep -m1 -E 'Unhandled (page fault|exception)' "$E2E_OUT/wine.log"); $last"
            break
        fi
        if [ $(($(date +%s) - start)) -gt "$RUN_TIMEOUT" ]; then
            e2e_say "run $name: timeout after $RUN_TIMEOUT s; $last"
            e2e_ctl stack > "$OUT/$name.stack"
            break
        fi
        sleep 5
    done
    wineserver -k 2> /dev/null || true
    wait "$pid" 2> /dev/null
    cp -f "$GTA_DIR/sa-hook.log" "$OUT/$name.log" 2> /dev/null
    # Wine's own output (an unhandled-exception line ends up in the crash check below). A fault after
    # `oracle: exiting` happens inside ExitProcess, after the trace, spool and summary were flushed: it
    # is kept apart (NAME.wine-exitfault.txt, a note) and does not count as a crash of the run.
    if grep -q 'oracle: exiting' "$OUT/$name.log" 2> /dev/null \
        && grep -q -E 'Unhandled (page fault|exception)' "$E2E_OUT/wine.log" 2> /dev/null; then
        cp -f "$E2E_OUT/wine.log" "$OUT/$name.wine-exitfault.txt"
        note "run $name: Wine fault during process exit, after the end frame: $(grep -m1 -E 'Unhandled (page fault|exception)' "$E2E_OUT/wine.log")"
    else
        cp -f "$E2E_OUT/wine.log" "$OUT/$name.wine.log" 2> /dev/null
    fi
    [ -n "$drain" ] && { kill "$drain" 2> /dev/null; rm -f "$DET/$name.spool"; }
    for f in trace spool spool.txt; do
        [ -f "$DET/$name.$f" ] && mv -f "$DET/$name.$f" "$OUT/$name.$f"
    done
    for f in "$DET/$name".spool.seq*.win; do
        [ -f "$f" ] && mv -f "$f" "$OUT/"
    done
    printf '%s\n' "$last" > "$OUT/$name.det"
    e2e_say "run $name finished in $(($(date +%s) - start)) s"
    q="$(sed -n 's/.*l2: \(spool quota reached for [^ ]*\).*/\1/p' "$OUT/$name.log" 2> /dev/null | head -1)"
    if [ -n "$q" ]; then
        check "run $name: $q…, cases compared before it still valid" 0 \
            "$(sed -n 's/.*l2: spool quota reached/spool quota reached/p' "$OUT/$name.log" | head -1)"
    fi
}
reached() { grep -q "end frame $END_FRAME reached" "$OUT/$1.log" 2> /dev/null && echo 1; }
trace_info() { "$SATOOL" show "$1" | awk 'NR > 1 { n++; f = $1 } END { printf "%d frames, last %d", n, f }'; }
full_len() { "$SATOOL" show "$OUT/$1.trace" | awk -v e="$END_FRAME" 'NR > 1 { n++; if ($1 != n) bad = 1 } END { exit !(n == e && !bad) }'; }
same_as_plain() { # same_as_plain RUN → 1 when identical over every frame; diff in OUT/plain-vs-RUN.txt
    [ -f "$OUT/plain.trace" ] && [ -f "$OUT/$1.trace" ] || return
    "$SATOOL" diff "$OUT/plain.trace" "$OUT/$1.trace" > "$OUT/plain-vs-$1.txt" 2>&1 && full_len "$1" && echo 1
}
l2_summary() { sed -n 's/.*l2: summary: //p' "$OUT/$1.log" 2> /dev/null | tail -1; }
l2_field() { printf '%s\n' "$1" | tr ';' '\n' | grep -F " $2 calls=" -m1 | tr ' ' '\n' | sed -n "s/^$3=//p" | head -1; }
l2_fn_line() { printf '%s\n' "$1" | tr ';' '\n' | grep -F " $2 calls=" -m1; }

# ---- runs ----------------------------------------------------------------------------------------------
e2e_headless_start || exit 1
if [ ! -s "$REC" ]; then
    e2e_say "no recording at $REC: recording the grove walk first"
    cp "$HERE/programs/grove-walk.txt" "$DET/program.txt"
    REC="$OUT/rec.sarec"
    run_game record SA_DET_PROGRAM="$(win "$DET/program.txt")" SA_DET_RECORD="$(win "$DET/rec.sarec")"
    cp -f "$DET/rec.sarec" "$REC"
fi
cp -f "$REC" "$DET/port-l2.sarec"
END_FRAME="$("$SATOOL" rec "$REC" 2> /dev/null | sed -n 's/.*last_frame=\([0-9]*\).*/\1/p' | tail -1)"
END_FRAME=$((END_FRAME + 1))
note "hooks ($LIST): ${#NAMES[@]}: $L2NAMES"
note "recording $REC ($("$SATOOL" rec "$REC" 2>&1 | tail -1)); end frame $END_FRAME"
REPLAY="SA_DET_REPLAY=$(win "$DET/port-l2.sarec")"
L2COMMON=(SA_L2_MAX="$MAX" SA_L2_PER_FRAME="${L2_PER_FRAME:-4}" SA_L2_SPEC="$(win "$DET/l2.spec")")
[ -n "${L2_SPOOL_MAX:-}" ] && L2COMMON+=(SA_L2_SPOOL_MAX="$L2_SPOOL_MAX")
SPLITCOMMON=(SA_L2_MAX="$MAX" SA_L2_PER_FRAME="${SPLIT_PER_FRAME:-1000}" SA_L2_SPEC="$(win "$DET/l2.spec")")
[ -n "${L2_SPOOL_MAX:-}" ] && SPLITCOMMON+=(SA_L2_SPOOL_MAX="$L2_SPOOL_MAX")
[ -n "${SPLIT_DUMP_SEQ:-}" ] && SPLITCOMMON+=(SA_L2_DUMP_SEQ="$SPLIT_DUMP_SEQ")

for run in $RUNS; do
    case "$run" in
        plain)
            # INF-19: the all-original trace depends only on the recording, the exe, the hook DLL and the
            # end frame, so it is cached (target/l2cache) and reused by later runs. PLAIN_CACHE=0 re-runs.
            pkey="$(cat "$REC" "$GTA_DIR/gta_sa.exe" "$GTA_DIR/sa_hook.dll" | sha256sum | cut -c1-16)-$END_FRAME"
            pdir="$SA_ROOT/target/l2cache"
            if [ "${PLAIN_CACHE:-1}" = 1 ] && [ -s "$pdir/plain-$pkey.trace" ] && [ -s "$pdir/plain-$pkey.log" ]; then
                cp -f "$pdir/plain-$pkey.trace" "$OUT/plain.trace"
                cp -f "$pdir/plain-$pkey.log" "$OUT/plain.log"
                note "plain: reused cached all-original trace $pdir/plain-$pkey.trace"
            else
                run_game plain "$REPLAY" SA_HOOK_MODE="$(modes)"
                if [ "$(reached plain)" = 1 ]; then
                    mkdir -p "$pdir"
                    cp -f "$OUT/plain.trace" "$pdir/plain-$pkey.trace"
                    cp -f "$OUT/plain.log" "$pdir/plain-$pkey.log"
                fi
            fi
            check "plain (all original) reached the end frame" "$(reached plain)" "$(trace_info "$OUT/plain.trace")"
            ;;
        l2)
            ex="$(extras_for "$L2NAMES")"
            note "l2 extra regions: ${ex:-none}"
            run_game l2 "$REPLAY" SA_HOOK_MODE="$(modes)" SA_L2="$L2LIST" "${L2COMMON[@]}" \
                SA_L2_EXTRA="$ex" SA_L2_OUT="$(win "$DET/l2.spool")"
            check "l2 reached the end frame" "$(reached l2)" "$(trace_info "$OUT/l2.trace")"
            ;;
        l2all)
            [ -n "$WALL" ] || continue
            WT="$(python3 -I "$HERE/hook-names.py" "$E2E_SCRIPT_ROOT" "$WALL")" || exit 2
            run_game l2all "$REPLAY" SA_HOOK_MODE="$(modes)" SA_L2="$(printf '%s\n' "$WT" | cut -f1 | paste -sd,)" \
                "${L2COMMON[@]}" SA_L2_WINDOW=all SA_L2_EXTRA="$(extras_for "$(printf '%s\n' "$WT" | cut -f3 | paste -sd,)")" \
                SA_L2_OUT="$(win "$DET/l2all.spool")"
            check "l2all reached the end frame" "$(reached l2all)" "$(trace_info "$OUT/l2all.trace")"
            ;;
        rust)
            if [ "$LIST" = ports ]; then
                run_game rust "$REPLAY" SA_HOOK_MODE="$(join_by , "${BASE_ORIG[@]/%/=orig}")" SA_HOOK_PORTS=rust
            else
                run_game rust "$REPLAY" SA_HOOK_MODE="$(modes "${NAMES[@]}")"
            fi
            check "rust reached the end frame" "$(reached rust)" "$(trace_info "$OUT/rust.trace")"
            ;;
        split)
            ex="$(extras_for "$L2NAMES")"
            note "split extra regions / OS probes: ${ex:-none}"
            for side in orig rust; do
                run_game "split${side:0:1}" "$REPLAY" SA_HOOK_MODE="$(modes)" SA_L2="$L2LIST" "${SPLITCOMMON[@]}" \
                    SA_L2_SPLIT="$side" SA_L2_EXTRA="$ex" SA_L2_OUT="$(win "$DET/split${side:0:1}.spool")"
                check "split${side:0:1} reached the end frame" "$(reached "split${side:0:1}")" \
                    "$(trace_info "$OUT/split${side:0:1}.trace")"
            done
            ;;
        hookoff)
            for k in "${!NAMES[@]}"; do
                others=()
                for j in "${!NAMES[@]}"; do [ "$j" = "$k" ] || others+=("${NAMES[$j]}"); done
                run_game "hookoff-$k" "$REPLAY" SA_HOOK_MODE="$(modes "${others[@]}")"
            done
            ;;
    esac
done
e2e_cleanup

# ---- checks --------------------------------------------------------------------------------------------
for r in l2 l2all splito; do
    [ -f "$OUT/$r.trace" ] && [ -f "$OUT/plain.trace" ] || continue
    ok="$(same_as_plain "$r")"
    check "(b) $r trace == plain trace on all $END_FRAME frames" "$ok" "$(tail -1 "$OUT/plain-vs-$r.txt" 2> /dev/null)"
done
if [ -f "$OUT/splitr.trace" ] && [ -f "$OUT/plain.trace" ]; then
    ok="$(same_as_plain splitr)"
    check "(e) splitr (Rust, split-run recording) trace == all-original trace on all $END_FRAME frames" "$ok" \
        "$(tail -1 "$OUT/plain-vs-splitr.txt" 2> /dev/null)"
fi
SPLITREP=""
if [ -f "$OUT/splito.spool" ] && [ -f "$OUT/splitr.spool" ]; then
    # --undeclared: every undeclared write range (`undeclared FN NAME ADDR+LEN xN SYMBOL` lines;
    # tools/e2e/l2-writers.py undeclared lists the instructions that can write them)
    "$ORACLE" l2 split-report "$OUT/splito.spool" "$OUT/splitr.spool" --undeclared > "$OUT/split-report.txt" 2>&1
    check "(e) split-report: every hook PASS, PASS-FP or NOCASES, no trailer" "$([ $? = 0 ] && echo 1)" \
        "$(grep -c "^SPLIT PASS " "$OUT/split-report.txt") PASS, $(grep -c "^SPLIT PASS-FP" "$OUT/split-report.txt") PASS-FP, $(grep -c '^SPLIT FAIL' "$OUT/split-report.txt") FAIL, $(grep -c '^SPLIT DIVERGED' "$OUT/split-report.txt") DIVERGED, $(grep -c '^SPLIT COUNT' "$OUT/split-report.txt") COUNT; $(grep -m1 '^first input divergence' "$OUT/split-report.txt")"
    # PLAN §5: L2 needs 0 undeclared writes too (an incomplete write set is not a pass)
    und_fns="$(grep -E '^SPLIT ' "$OUT/split-report.txt" | grep -v ' undeclared_cases=0 ' | awk '{ print $4 }' | paste -sd, -)"
    check "(e) split-report: 0 undeclared writes" "$([ -z "$und_fns" ] && echo 1)" "${und_fns:-none}"
    SPLITREP="$OUT/split-report.txt"
fi
L3=""
if [ -f "$OUT/rust.trace" ]; then
    L3="$(same_as_plain rust)"
    check "(c) rust trace == all-original trace on all $END_FRAME frames, 17 subsystems" "$L3" \
        "$(tail -1 "$OUT/plain-vs-rust.txt" 2> /dev/null)"
    sed 's/^/    /' "$OUT/plain-vs-rust.txt" >> "$SUMMARY"
    note "    randcalls (Rust vs original): $(grep -E '^\s*randcalls' "$OUT/plain-vs-rust.txt" | sed 's/^ *//')"
fi
S="$(l2_summary l2)"
SA="$(l2_summary l2all)"
[ -n "$S" ] && note "l2 summary: $S"
[ -n "$SA" ] && note "l2all summary: $SA"
printf 'addr\tname\tcalls\tcases\tmismatches\tundeclared_cases\tl2\tl3\thookoff\tsplit\n' > "$OUT/results.tsv"
for k in "${!NAMES[@]}"; do
    fn="${NAMES[$k]}" addr="${ADDRS[$k]}"
    calls="$(l2_field "$S" "$fn" calls)"; cases="$(l2_field "$S" "$fn" cases)"
    mism="$(l2_field "$S" "$fn" mismatches)"; und="$(l2_field "$S" "$fn" undeclared_cases)"
    unr="$(l2_field "$S" "$fn" unreadable)"
    if [ -n "$SA" ] && [ -n "$(l2_fn_line "$SA" "$fn")" ]; then
        m2="$(l2_field "$SA" "$fn" mismatches)"; u2="$(l2_field "$SA" "$fn" undeclared_cases)"
        c2="$(l2_field "$SA" "$fn" cases)"
        check "(a) $fn (window all): 0 mismatches, 0 undeclared writes" \
            "$([ "$m2" = 0 ] && [ "$u2" = 0 ] && echo 1)" \
            "cases=$c2 mismatches=$m2 undeclared_cases=$u2 undeclared=$(l2_field "$SA" "$fn" undeclared)"
        mism=$((${mism:-0} + ${m2:-0})); und=$((${und:-0} + ${u2:-0}))
    fi
    l2=""
    if [ -z "$S" ]; then
        l2=norun
    elif [ "${calls:-0}" = 0 ]; then
        l2=NOCALLS
        note "NOCALLS (a) $fn: no call in this recording"
    else
        enough=0
        { [ "$cases" -ge "$MAX" ] || [ "$cases" = "$calls" ]; } && enough=1
        check "(a) $fn: >= $MAX cases or every call compared" "$enough" "calls=$calls cases=$cases"
        check "(a) $fn: 0 mismatches, 0 undeclared writes" "$([ "$mism" = 0 ] && [ "$und" = 0 ] && echo 1)" \
            "mismatches=$mism undeclared_cases=$und undeclared=$(l2_field "$S" "$fn" undeclared) unreadable_regions=$unr first=$(l2_fn_line "$S" "$fn" | sed -n 's/.*first_mismatch=\("[^"]*"\).*/\1/p')"
        l2=$([ "$enough" = 1 ] && [ "$mism" = 0 ] && [ "$und" = 0 ] && echo PASS || echo FAIL)
    fi
    ho=""
    if [ -f "$OUT/hookoff-$k.trace" ]; then
        o="$(same_as_plain "hookoff-$k")"
        check "(d) hook-off $fn (others on Rust) == all-original" "$o" "$(tail -1 "$OUT/plain-vs-hookoff-$k.txt" 2> /dev/null)"
        ho=$([ "$o" = 1 ] && echo PASS || echo FAIL)
    fi
    l3=""
    [ -f "$OUT/rust.trace" ] && l3=$([ "$L3" = 1 ] && echo PASS || echo FAIL)
    sp=""
    if [ -n "$SPLITREP" ]; then
        sp="$(awk -v a="$(printf '0x%08x' "$addr")" '$1 == "SPLIT" && $3 == a { print $2; exit }' "$SPLITREP")"
        sp="${sp:-NOCASES}"
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$addr" "$fn" "${calls:-}" "${cases:-}" "${mism:-}" "${und:-}" \
        "$l2" "$l3" "$ho" "$sp" >> "$OUT/results.tsv"
done
for r in l2 l2all; do
    [ -f "$OUT/$r.spool" ] || continue
    "$ORACLE" l2 report "$OUT/$r.spool" --undeclared > "$OUT/$r-report.txt" 2>&1
    check "host report of the $r spool: 0 mismatches" "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/$r-report.txt")"
done
crash="$(grep -E -c 'Unhandled exception|page fault|PANIC|dispatch failed|FAILED' "$OUT"/*.log 2> /dev/null | awk -F: '{ s += $2 } END { print s + 0 }')"
check "no crash / panic / dispatch or install failure in logs" "$([ "$crash" = 0 ] && echo 1)" "$crash lines"
note "results: $OUT/results.tsv"
column -t -s $'\t' "$OUT/results.tsv" | tee -a "$SUMMARY"
exit $((FAILS > 0))
