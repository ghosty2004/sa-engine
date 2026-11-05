#!/bin/bash
# RE-SPEC: decompile a list of functions (reading aid for the spec bundles).
# usage: tools/specs/decomp.sh ADDRS.txt OUT.c   (run from anywhere; needs the analysed project in target/ghidra)
# Runs Ghidra headless read-only under the PLAN §8 cap (8G scope, 5G Java heap).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$ROOT"
MAXMEM=5G systemd-run --user --scope -q -p MemoryMax=8G -p MemorySwapMax=0 \
  /opt/ghidra/support/analyzeHeadless target/ghidra gta_sa_10us -process gta_sa.exe -readOnly -noanalysis \
  -scriptPath "$ROOT/tools/catalog/ghidra" \
  -postScript DecompileList.java "$1" "$2" > "$2.log" 2>&1
tail -3 "$2.log"
grep -c '// ====' "$2"
