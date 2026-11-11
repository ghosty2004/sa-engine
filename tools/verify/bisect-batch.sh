#!/usr/bin/env bash
# Automatic narrowing of an unverified batch (docs/verify/HOWTO.md §4).
#
#   tools/verify/bisect-batch.sh BATCH RECORDING LANE [--margin N] [--hookoff]
#
# Enables BATCH (SA_UNVERIFIED) on top of the verified set and runs the all-ports L3 compare on
# RECORDING: every verified port on Rust in every probe (`--fixed verified`), the batch's hooks on Rust
# on top. On a divergence sa-oracle l3 bisects the batch's hooks (halving; the SA_HOOK_MODE lists go to
# sa-hook as @FILE when long) down to the single hook, or the minimal set that only diverges together,
# and keeps going until the rest of the batch matches. This script reuses tools/e2e/l3.sh (game runs,
# lane lock, trace cache) and prints, per culprit, the hook, its catalogue subsystem and source line,
# the first divergent frame with the hash subsystems that differ, and the next command (an L2 run from
# that frame).
#
# --hookoff: on a match, also the hook-off check of every batch hook (one run each).
# RECORD=1: record the result in catalog/verify-status.csv (tools/verify/queue.py set): culprits ->
# failed with the bisect line as evidence. A match is only printed (L3-batch needs every recording).
# Env as l3.sh: SA_ROOT (checkout holding wine/, set it from a worktree), RUN_TIMEOUT, L3_CACHE, CARGO_TARGET_DIR.
# Output: target/verify/BATCH/bisect-RECNAME.txt (report) and .log (the l3.sh output).
# Exit: 0 = the batch is identical with the verified set, 1 = culprits found, 2 = usage/setup error.
set -uo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
usage() { sed -n '4p' "$0" >&2; exit 2; }
[ $# -ge 3 ] || usage
BATCH="$1" REC="$2" LANE_ARG="$3"
shift 3
MARGIN="" HOOKOFF=none
while [ $# -gt 0 ]; do
    case "$1" in
        --margin) MARGIN="$2"; shift 2 ;;
        --hookoff) HOOKOFF=all; shift ;;
        *) usage ;;
    esac
done
case "$LANE_ARG" in '' | *[!0-9]*) echo "bisect-batch: LANE must be a number" >&2; exit 2 ;; esac
[ -s "$REC" ] || { echo "bisect-batch: no recording $REC" >&2; exit 2; }
REC="$(cd "$(dirname "$REC")" && pwd)/$(basename "$REC")"
HOOKS="$(python3 -I "$HERE/queue.py" show "$BATCH")" || exit 2
N="$(printf '%s\n' "$HOOKS" | wc -l)"
# the batch on top of whatever the caller already selected
export SA_UNVERIFIED="${SA_UNVERIFIED:+$SA_UNVERIFIED,}$BATCH"
RECNAME="$(basename "$(dirname "$REC")")-$(basename "$REC" .sarec)"
case "$REC" in */inf4/rec.sarec) RECNAME=grove-walk ;; */corpus/*) RECNAME="$(basename "$REC" .sarec)" ;; esac
OUT="$ROOT/target/verify/$BATCH"
mkdir -p "$OUT"
LOG="$OUT/bisect-$RECNAME.log"
REPORT="$OUT/bisect-$RECNAME.txt"
echo "bisect-batch: batch $BATCH ($N hooks) on top of the verified set, recording $RECNAME, lane $LANE_ARG"
# l3.sh removes the lane's run.txt and copies the new one to RUN while it holds the lane lock, so
# another agent's run on the same lane cannot race this report.
RUN="$OUT/l3-$RECNAME.txt"
rm -f "$RUN"
L3_REPORT_COPY="$RUN" LANE="$LANE_ARG" "$ROOT/tools/e2e/l3.sh" --rec "$REC" --rust "batch:$BATCH" --fixed verified --hookoff "$HOOKOFF" \
    ${MARGIN:+--margin "$MARGIN"} > "$LOG" 2>&1
RC=$?
if ! grep -q '^L3 ' "$RUN" 2> /dev/null; then
    echo "bisect-batch: l3.sh failed (exit $RC) before a verdict; see $LOG" >&2
    tail -5 "$LOG" >&2
    exit 2
fi
{
    echo "batch $BATCH ($N hooks), recording $REC, lane $LANE_ARG, $(date -Is)"
    grep -m1 '^FIXED' "$RUN"
    grep -m1 '^L3 ' "$RUN" | cut -c1-400
} > "$REPORT"
if grep -q '^L3 MATCH' "$RUN"; then
    echo "IDENTICAL: batch $BATCH with the verified set matches the all-original reference on every frame of $RECNAME" | tee -a "$REPORT"
    grep -E '^(HOOKOFF|probes)' "$RUN" | tee -a "$REPORT" > /dev/null
    [ "$HOOKOFF" = all ] && grep '^HOOKOFF' "$RUN"
    echo "next: the same on every other recording (L3-batch), then promote; queue: python3 -I tools/verify/queue.py set batch:$BATCH L3-batch \"...\""
    exit 0
fi
# per culprit: the suggest lines carry the address, name and the frame where it diverges (alone on top
# of the fixed set); the `alone` line the hash subsystems.
python3 -I - "$RUN" "$HOOKS" "$BATCH" "$RECNAME" "$REC" "$LANE_ARG" "$ROOT" >> "$REPORT" <<'EOF'
import re, sys
run, hooks, batch, recname, rec, lane, root = sys.argv[1:]
info = {}
for line in hooks.splitlines():
    a, name, route, haz, where = line.split("\t")
    info[int(a, 16)] = (name, route, haz, where)
subs = {}
try:
    import csv
    for r in csv.DictReader(open(f"{root}/catalog/procedures.csv")):
        subs[int(r["address"], 16)] = r["subsystem"]
except OSError:
    pass
lines = open(run).read().splitlines()
first = next((l for l in lines if l.startswith("L3 DIVERGED")), "")
m = re.search(r"first divergence at (.*)$", first)
print(f"DIVERGED: batch {batch} with the verified set, first divergence at {m.group(1) if m else '?'}")
i = 0
n = 0
while i < len(lines):
    l = lines[i]
    if l.startswith("CULPRIT"):
        n += 1
        alone = lines[i + 1] if i + 1 < len(lines) else ""
        sugg = []
        j = i + 2
        while j < len(lines) and lines[j].startswith("  suggest"):
            sugg.append(lines[j])
            j += 1
        kind = "single hook" if len(sugg) == 1 else f"minimal set of {len(sugg)} hooks (diverges only together)"
        print(f"CULPRIT {n}: {kind}")
        am = re.search(r"diverges at frame (\d+): (.*)$", alone)
        for s in sugg:
            a = int(re.search(r"SA_L2=(0x[0-9a-fA-F]+)", s).group(1), 16)
            fr = re.search(r"at frame (\d+)$", s).group(1)
            name, route, haz, where = info.get(a, ("?", "?", "?", "?"))
            cmd = re.search(r"\((tools/e2e/l3.sh [^)]*)\)", s).group(1)
            print(f"  hook {a:#x} {name}  [catalogue subsystem {subs.get(a, '?')}; {where}; route {route}; hazards {haz}]")
            print(f"    first divergent frame {fr}" + (f", hash subsystems {am.group(2)}" if am else
                  " (no divergence alone in the window: only together with the others)"))
            print(f"    next: SA_UNVERIFIED={batch} LANE={lane} {cmd} --rec {rec} --rust verified")
            print(f"    queue: python3 -I tools/verify/queue.py set {a:#x} failed \"bisect {recname}: frame {fr}"
                  + (f" {am.group(2)}" if am else "") + "\"")
        i = j
        continue
    if l.startswith("rest of the set"):
        print(l[:300] + ("..." if len(l) > 300 else ""))
    i += 1
EOF
cat "$REPORT"
if [ "${RECORD:-0}" = 1 ]; then
    grep -o 'queue: python3 -I tools/verify/queue.py set .*' "$REPORT" | sed 's/^queue: //' | while IFS= read -r c; do
        (cd "$ROOT" && eval "$c")
    done
fi
echo "report: $REPORT"
exit 1
