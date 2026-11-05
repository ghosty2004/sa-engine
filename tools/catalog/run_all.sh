#!/usr/bin/env bash
# Regenerate the whole function catalogue (catalog/) deterministically.
#
#   tools/catalog/run_all.sh            # full run
#   SA_STATE_DATE=2026-11-05 tools/catalog/run_all.sh   # date stamped on changed state.csv rows
#
# Order: build_catalog -> merge_ghidra -> classify_unknowns (CAT-5) -> subsystems -> callgraph
# ghidra + build (CAT-7) -> callgraph gaps (CAT-12: closes the catalogue over call targets; when
# it adds rows the loop reruns from build_catalog) -> layouts (CAT-8) -> script_commands (CAT-10)
# -> progress merge + report (CAT-11, catalog/state.csv is merged, never reset).
#
# Inputs: re/gta-reversed, re/plugin-sdk, re/gta/1.0-US/gta_sa.exe, catalog/raw/rea-evidence.json,
# catalog/extra-code-addresses.csv, the analysed Ghidra project in target/ghidra (created by
# `callgraph.py analyse` on first use) and the existing catalog/state.csv.
set -euo pipefail
cd "$(dirname "$0")/../.."
PY=(.venv/bin/python -I)
EXE=re/gta/1.0-US/gta_sa.exe
GR=re/gta-reversed
CAT=catalog
LOG=target/catalog-logs
mkdir -p "$LOG"

step() {  # step NAME CMD... : run quietly, keep the log, fail loudly
    local name=$1; shift
    local rc=0
    "$@" >"$LOG/$name.log" 2>&1 || rc=$?
    if [ $rc -ne 0 ]; then
        echo "run_all: $name failed (exit $rc), see $LOG/$name.log" >&2
        tail -20 "$LOG/$name.log" >&2
        exit $rc
    fi
}

for pass in 1 2 3 4 5; do
    echo "run_all: catalogue pass $pass"
    step build_catalog "${PY[@]}" tools/catalog/build_catalog.py "$GR" "$EXE" "$CAT"
    step merge_ghidra "${PY[@]}" tools/catalog/merge_ghidra.py "$CAT/raw/rea-evidence.json" "$CAT" "$EXE" "$GR"
    step classify_unknowns "${PY[@]}" tools/catalog/classify_unknowns.py "$EXE" "$CAT"
    step subsystems "${PY[@]}" tools/catalog/subsystems.py "$CAT"
    step callgraph-ghidra "${PY[@]}" tools/catalog/callgraph.py ghidra "$CAT"
    step callgraph-build "${PY[@]}" tools/catalog/callgraph.py build "$CAT"
    set +e
    "${PY[@]}" tools/catalog/callgraph.py gaps "$CAT" >"$LOG/callgraph-gaps.log" 2>&1
    rc=$?
    set -e
    cat "$LOG/callgraph-gaps.log"
    [ $rc -eq 0 ] && break
    if [ $rc -ne 3 ]; then echo "run_all: catalogue gaps not closable (exit $rc)" >&2; exit $rc; fi
    if [ "$pass" -eq 5 ]; then echo "run_all: catalogue did not converge in 5 passes" >&2; exit 1; fi
done

step layouts "${PY[@]}" tools/catalog/layouts.py "$GR" "$CAT"
step script_commands "${PY[@]}" tools/catalog/script_commands.py "$GR" "$CAT" "$EXE"
grep 'case annotations' "$LOG/script_commands.log"
step progress "${PY[@]}" tools/catalog/progress.py "$CAT"
head -3 "$CAT/progress.md" | tail -1
echo "run_all: done (logs in $LOG)"
