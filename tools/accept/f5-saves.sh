#!/usr/bin/env bash
# PLAN F5 (save compatibility), SAVE-1: the save-load recording (tools/e2e/programs/save-load.txt: a save to
# slot 1, then Load Game of slot 1 in the same run) replayed with
#   all    every save and load port on Rust,
#   save   only the ports that run while saving on Rust (the load reads that save on the original),
#   load   only the ports that run while loading on Rust (they read the original's save),
# each against the all-original replay (port-l2.sh "plain rust"): the state trace must be IDENTICAL on every
# frame (the load restores the same state: original save -> Rust load, Rust save -> original load), and the save
# file the run writes is compared with the original's, byte for byte and with tools/accept/save-diff.py --mask
# (bytes the 1.0 code takes from uninitialised stack locals; docs/arch/save-port.md, ledger).
#
#   tools/accept/f5-saves.sh LANE [RECORDING] [SET...]      (SET: all save load; default all three)
#
# Env: SA_ROOT (checkout with wine/ and the corpus), TAG (default f5), as port-l2.sh. Output:
# target/accept/f5/TAG-SET/{summary.txt,save-diff.txt} and target/accept/f5/TAG.txt (one line per set).
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
LANE="${1:?usage: $0 LANE [RECORDING] [SET...]}"
shift
SA_ROOT="${SA_ROOT:-$ROOT}"
REC="$SA_ROOT/target/e2e/corpus/save-load.sarec"
if [ $# -gt 0 ] && [ -f "$1" ]; then REC="$1"; shift; fi
SETS=("$@")
[ ${#SETS[@]} -eq 0 ] && SETS=(all save load)
TAG="${TAG:-f5}"
OUT="$ROOT/target/accept/f5"
mkdir -p "$OUT"
PY="${PYTHON:-python3}"

# Ports that run while saving (C_PcSave::SaveSlot and below) and while loading (GenericLoad and below), by
# address; the helpers both use (CloseFile) belong to "all" only.
SAVE=(0x619060 0x5d0e90 0x618f50 0x5d13e0 0x5d0dd0 0x5d0f80 0x5d1270 0x5d1b80 0x69f910 0x5d0f50
    0x5d4c40 0x5d0880 0x5d4b40 0x5d4940 0x5d5730 0x5d57e0 0x5d43d0 0x5d3160 0x44a170 0x5d3020 0x5d33c0
    0x5d34c0 0x5d3540 0x5d3620 0x5d5860 0x5d2e60 0x5d3a60 0x5d38c0 0x5d3ac0 0x5d24b0 0x5d3b40 0x5d3c70
    0x5d29a0 0x5d3cd0 0x5d3d60 0x5d5420 0x5d3de0 0x5d5530 0x5d2620 0x5d5570 0x5d5970 0x5d3ee0 0x5d4300
    0x5d4760 0x5d47e0 0x5d2330 0x5d4800)
LOAD=(0x5d17b0 0x5d0d20 0x5d10b0 0x5d1300 0x5d1ea0 0x5d1380 0x5d1170 0x6190a0 0x618e90
    0x5d4fd0 0x5d0890 0x5d2d70 0x5d4a40 0x5d2a20 0x5d4640 0x5d46e0 0x5d44b0 0x5d3270 0x5d30c0 0x5d3440
    0x5d3500 0x5d35a0 0x5d3770 0x5d53c0 0x5d2f40 0x5d3a90 0x5d39b0 0x5d3b00 0x5d2560 0x5d3bf0 0x5d3ca0
    0x5d29e0 0x5d3d10 0x5d3da0 0x5d54a0 0x5d3e40 0x5d3eb0 0x5d2740 0x5d5920 0x5d55c0 0x5d40e0 0x5d42e0
    0x5d2900 0x5d2980 0x5d23b0)
BOTH=(0x5d0e30)

: > "$OUT/$TAG.txt"
for set in "${SETS[@]}"; do
    list="$OUT/$TAG-$set.list"
    case "$set" in
        all) printf '%s\n' "${SAVE[@]}" "${LOAD[@]}" "${BOTH[@]}" > "$list" ;;
        save) printf '%s\n' "${SAVE[@]}" > "$list" ;;
        load) printf '%s\n' "${LOAD[@]}" > "$list" ;;
        *) echo "unknown set $set" >&2; exit 2 ;;
    esac
    t="$TAG-$set"
    # the listed hooks are selected even when unverified (SA_UNVERIFIED), every other port stays original
    SA_UNVERIFIED="@$list" LANE="$LANE" SA_ROOT="$SA_ROOT" "$ROOT/tools/e2e/port-l2.sh" "@$list" "$REC" \
        --runs "plain rust" --tag "$t" > "$OUT/$t.log" 2>&1
    d="$ROOT/target/e2e/lane-$LANE/port-l2-$t"
    mkdir -p "$OUT/$t"
    cp -f "$d/summary.txt" "$OUT/$t/summary.txt" 2> /dev/null
    trace="$(grep -E '^(PASS|FAIL) \(c\)' "$d/summary.txt" 2> /dev/null | tail -1)"
    if [ -f "$d/plain.GTASAsf1.b" ] && [ -f "$d/rust.GTASAsf1.b" ]; then
        "$PY" -I "$ROOT/tools/accept/save-diff.py" "$d/plain.GTASAsf1.b" "$d/rust.GTASAsf1.b" --mask > "$OUT/$t/save-diff.txt"
        case $? in
            0) bytes="save bytes IDENTICAL" ;;
            3) bytes="save bytes identical except stack residue ($(grep -o '[0-9]* differing bytes are residue' "$OUT/$t/save-diff.txt"))" ;;
            *) bytes="save bytes DIFFER ($(grep -E '^mask:|data bytes differ' "$OUT/$t/save-diff.txt" | tr '\n' ' '))" ;;
        esac
    else
        bytes="no save file pair"
    fi
    echo "$set: ${trace:-FAIL (c) no comparison line}; $bytes" | tee -a "$OUT/$TAG.txt"
done
! grep -q -v ': PASS (c)' "$OUT/$TAG.txt"
