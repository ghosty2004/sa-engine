#!/bin/sh
# First-access traces of the legacy address arena over FRONTIER scenarios (PURE-1, docs/arch/pure-build-census.md).
#
#   tools/pure/trace_scenarios.sh OUTDIR [SCENARIO ...]
#
# Builds tools/pure/arena_trace.c (gcc -m32) into OUTDIR, then runs each scenario of tools/e2e/scenarios/frontier.sh
# (default: the list below) with the trace preloaded; each run leaves OUTDIR/tr-NAME.<pid> and the scenario's
# report in OUTDIR/tr-NAME/. Analyse with
#   .venv/bin/python -I tools/pure/address_census.py --csv OUTDIR/census.csv
#   .venv/bin/python -I tools/pure/arena_usage.py --census OUTDIR/census.csv OUTDIR/tr-*.[0-9]*
# SA_GAME names the i686 sa-game binary (default: target/i686-unknown-linux-gnu/release/sa-game). Runs one at a time.
set -eu
HERE=$(cd "$(dirname "$0")/../.." && pwd)
OUT=$(mkdir -p "$1" && cd "$1" && pwd); shift
[ $# -gt 0 ] || set -- pausemenu crossing solicit sniper hunter gangwar boat melee postfx tank cinema replay copsf flamer driveby samsite
gcc -m32 -O2 -Wall -shared -fPIC -o "$OUT/arena_trace.so" "$HERE/tools/pure/arena_trace.c" -ldl
for s in "$@"; do
    start=$(date +%s)
    rc=0
    WRAP="env LD_PRELOAD=$OUT/arena_trace.so SA_TRACE_OUT=$OUT/tr-$s" \
        sh "$HERE/tools/e2e/scenarios/frontier.sh" "$s" "$OUT/tr-$s" || rc=$?
    echo "$s rc=$rc $(( $(date +%s) - start ))s"
done
