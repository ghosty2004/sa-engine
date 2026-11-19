#!/bin/sh
# The C-bug with a sideways strafe on the native build, keyboard and mouse: one offscreen run (--fixed-step 40, 25
# frames a second as the frame limiter's default) where CJ takes the desert eagle (KJKSZPJ, E twice), holds the right
# button (aim) and D (strafe right), and every 0.8 s fires (left button) and taps C 0.12 s later, ten times.
#
#   tools/e2e/scenarios/cbug-strafe.sh OUTDIR [SA-GAME ARGS...]
#
# Each presented frame prints CPad::Pads[0]'s NewState (0xB73458) and the player's position. Checks: the pad holds
# LeftStickX 128 and RightShoulder1 (aim) through every frame of the strafe, every shot reaches the pad as a Circle
# press and every C tap as a ShockButtonL press, and CJ strafes at least MIN units (default 12; 17.6 on master) from
# the first shot on.
# Env: SA_ROOT (checkout holding wine/gta-sa; default ~/sa-engine), SA_GAME (binary; default this checkout's
# i686-unknown-linux-gnu release build), MIN.
set -eu
HERE=$(cd "$(dirname "$0")/../../.." && pwd)
OUT=$1; shift
B=${SA_GAME:-$HERE/target/i686-unknown-linux-gnu/release/sa-game}
GAME=${SA_ROOT:-$HOME/sa-engine}/wine/gta-sa
keys=; i=100; for c in k j k s z p j; do keys="$keys${keys:+,}$c:$i:$((i + 3))"; i=$((i + 6)); done
keys="$keys,e:162:165,e:172:175,d:217:440"
aim=207:440 shots=
for n in 0 1 2 3 4 5 6 7 8 9; do
    s=$((227 + 20 * n)); c=$((s + 3))
    shots="$shots${shots:+,}$s:$((s + 2))"; keys="$keys,c:$c:$((c + 3))"
done
mkdir -p "$OUT"
env GLIBC_TUNABLES=glibc.malloc.mmap_max=0 SA_GAME_SCRIPT_SKIP=1 SA_GAME_KEYS="$keys" SA_GAME_LMB="$shots" \
    SA_GAME_RMB="$aim" SA_GAME_DUMP="B7CD98>14>30:c,B73458:30" \
    systemd-run --user --scope -q -p MemoryMax=6G setarch i686 -R "$B" --game "$GAME" --fixed-step 40 --frames 460 \
    --start-in-world --report "$OUT/report.md" "$@" > "$OUT/run.txt" 2>&1
python3 -I - "$OUT/run.txt" "${MIN:-12}" <<'EOF'
import math, re, struct, sys
log, least = sys.argv[1], float(sys.argv[2])
frames, it = [], None
for line in open(log, errors='replace'):
    m = re.match(r'frame (\d+):', line)
    if m:
        it = int(m.group(1))
        continue
    m = re.match(r'sa-game: dump (\S+) = 0x[0-9a-f]+:(.*)', line)
    if not m or it is None:
        continue
    words = b''.join(struct.pack('<I', int(w.split('(')[0], 16)) for w in m.group(2).split())
    if m.group(1) == 'B7CD98>14>30':
        frames.append({'it': it, 'pos': struct.unpack('<3f', words)})
    elif frames and frames[-1]['it'] == it:
        lx, r1, circle, shock = (struct.unpack_from('<h', words, o)[0] for o in (0x00, 0x0C, 0x22, 0x24))
        frames[-1].update(lx=lx, r1=r1, circle=circle, shock=shock)
frames = [f for f in frames if 'lx' in f]
held = [f for f in frames if 219 <= f["it"] < 440]
fails = []
if not held:
    sys.exit(f'FAIL: no in-game frames in {log}')
bad = [f['it'] for f in held if f['lx'] != 128 or f['r1'] != 255]
if bad:
    fails.append(f'LeftStickX / RightShoulder1 not held at iterations {bad[:10]}')
edges = lambda k: sum(1 for a, b in zip(held, held[1:]) if b[k] and not a[k])
if edges('circle') != 10 or edges('shock') != 10:
    fails.append(f"{edges('circle')} shots and {edges('shock')} crouch presses reached the pad (want 10 each)")
moved = [f for f in held if f['it'] >= 227]
path = sum(math.dist(a['pos'][:2], b['pos'][:2]) for a, b in zip(moved, moved[1:]))
if path < least:
    fails.append(f'strafed {path:.2f} units from the first shot (want >= {least})')
print(f'{len(held)} frames with D held, strafe {path:.2f} units from the first shot')
for f in fails:
    print('FAIL:', f)
sys.exit(1 if fails else 0)
EOF
