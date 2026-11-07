#!/usr/bin/env bash
# L2 + L3 verification of a set of hooked ports on a corpus recording (CORE-L2; for every porting batch).
#
#   tools/e2e/port-l2.sh <HOOKS|ports> [RECORDING] [--runs "plain l2 rust hookoff"] [--max N]
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
# and checks:
#   (a) per hook: >= MAX cases or every call compared, 0 mismatches, 0 undeclared writes; a hook that
#       got no call in the run is reported as NOCALLS (not a failure, but no evidence either)
#   (b) l2 trace == plain trace on every frame (the oracle does not perturb the run)
#   (c) rust trace == plain trace on every frame and all 17 subsystems (L3); `randcalls` listed apart
#   (d) hookoff traces == plain trace
#   host: `sa-oracle l2 report` of each spool (0 mismatches)
#
# Env: LANE (default 2), SA_ROOT, L2_PER_FRAME (default 4), RUN_TIMEOUT (s, default 1500),
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
extras_for() {
    local want="$1" line doc rest n hit
    local -A docset=()
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
    done | sed 's/,$//'
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
cp "$OUT/l2.spec" "$DET/l2.spec"

run_game() { # run_game NAME VAR=VALUE... : one determinism-mode run from process start to the end frame
    local name="$1" start last="" st kv
    shift
    rm -f "$DET/$name.trace" "$DET/$name.spool" "$DET/$name.spool.txt" "$GTA_DIR/sa-hook.log"
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
    [ -n "$drain" ] && { kill "$drain" 2> /dev/null; rm -f "$DET/$name.spool"; }
    for f in trace spool spool.txt; do
        [ -f "$DET/$name.$f" ] && mv -f "$DET/$name.$f" "$OUT/$name.$f"
    done
    printf '%s\n' "$last" > "$OUT/$name.det"
    e2e_say "run $name finished in $(($(date +%s) - start)) s"
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

for run in $RUNS; do
    case "$run" in
        plain)
            run_game plain "$REPLAY" SA_HOOK_MODE="$(modes)"
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
for r in l2 l2all; do
    [ -f "$OUT/$r.trace" ] || continue
    ok="$(same_as_plain "$r")"
    check "(b) $r trace == plain trace on all $END_FRAME frames" "$ok" "$(tail -1 "$OUT/plain-vs-$r.txt" 2> /dev/null)"
done
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
printf 'addr\tname\tcalls\tcases\tmismatches\tundeclared_cases\tl2\tl3\thookoff\n' > "$OUT/results.tsv"
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
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$addr" "$fn" "${calls:-}" "${cases:-}" "${mism:-}" "${und:-}" \
        "$l2" "$l3" "$ho" >> "$OUT/results.tsv"
done
for r in l2 l2all; do
    [ -f "$OUT/$r.spool" ] || continue
    "$ORACLE" l2 report "$OUT/$r.spool" > "$OUT/$r-report.txt" 2>&1
    check "host report of the $r spool: 0 mismatches" "$([ $? = 0 ] && echo 1)" "$(head -1 "$OUT/$r-report.txt")"
done
crash="$(grep -E -c 'Unhandled exception|page fault|PANIC|dispatch failed|FAILED' "$OUT"/*.log 2> /dev/null | awk -F: '{ s += $2 } END { print s + 0 }')"
check "no crash / panic / dispatch or install failure in logs" "$([ "$crash" = 0 ] && echo 1)" "$crash lines"
note "results: $OUT/results.tsv"
column -t -s $'\t' "$OUT/results.tsv" | tee -a "$SUMMARY"
exit $((FAILS > 0))
