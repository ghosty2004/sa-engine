#!/bin/sh
# The speed blur (CPostEffects::SpeedFX) on the frames of the native build, at any screen size:
#
#   tools/e2e/scenarios/speedfx.sh OUTDIR [SA-GAME ARGS...]
#
# e.g. `--mods target/mods/resolution` with its config.toml at "2455x1512" for a large non-power-of-two screen.
# The quad cheat AKJJYGLC, CJ gets on the quad and stays there (a vehicle camera, a still scene); one offscreen run
# (--fixed-step 40) presents the in-game frames from 320 to the end. CPostEffects::m_bSpeedFXTestMode (0xC402C7) is
# poked on at 380 (it takes effect at in-game frame ~337), so from there every frame draws SpeedFX at speed 1.0
# (5 passes, jitter 4, shake 3: the shake path). The blur's onset may darken the frame evenly (the front buffer is
# copied before the colour filter) but must not change its structure: with every frame's 32x24 grey thumbnail
# scaled to a mean of 100, each quarter's mean absolute difference between two consecutive frames stays below LIMIT
# (default 8; a working blur ~4). Caught: UVs far outside the front buffer (one quarter painted with a single
# texel, ~18) and a front buffer the screen copy does not reach (the frame blended with black, ~14).
# Env: SA_ROOT (checkout holding wine/gta-sa; default ~/sa-engine), SA_GAME (binary; default this checkout's
# i686-unknown-linux-gnu release build), LIMIT.
set -eu
HERE=$(cd "$(dirname "$0")/../../.." && pwd)
OUT=$1; shift
B=${SA_GAME:-$HERE/target/i686-unknown-linux-gnu/release/sa-game}
GAME=${SA_ROOT:-$HOME/sa-engine}/wine/gta-sa
keys=; i=150; for c in a k j j y g l c; do keys="$keys${keys:+,}$c:$i:$((i + 2))"; i=$((i + 6)); done
mkdir -p "$OUT"
env GLIBC_TUNABLES=glibc.malloc.mmap_max=0 SA_GAME_SCRIPT_SKIP=1 SA_GAME_KEYS="$keys,f:260:263" \
    SA_GAME_POKE="380:0xC402C7:1:1" SA_GAME_PRESENT_GAME=1 SA_GAME_PRESENT_SKIP=320 \
    systemd-run --user --scope -q -p MemoryMax=6G setarch i686 -R "$B" --game "$GAME" --fixed-step 40 --frames 400 \
    --start-in-world --present-dir "$OUT" --report "$OUT/report.md" "$@" > "$OUT/stdout.txt" 2> "$OUT/stderr.txt"
python3 -I - "$OUT" "${LIMIT:-8}" <<'EOF'
import glob, sys
from PIL import Image, ImageChops, ImageStat
out, limit = sys.argv[1], float(sys.argv[2])
frames = sorted(glob.glob(f'{out}/present-[0-9]*.png'))
if len(frames) < 2:
    sys.exit(f'FAIL: {len(frames)} frames in {out}')


def thumb(f):
    """The frame's size and its 32x24 grey thumbnail scaled to a mean of 100."""
    img = Image.open(f).convert('L')
    small = img.resize((32, 24))
    m = max(ImageStat.Stat(small).mean[0], 1.0)
    return img.size, small.point(lambda v: max(0, min(255, int(v * 100 / m))))


prev, worst, size = None, (0.0, '', []), None
for f in frames:
    size, t = thumb(f)
    if prev is not None:
        d = ImageChops.difference(prev, t)
        q = [ImageStat.Stat(d.crop(b)).mean[0] for b in ((0, 0, 16, 12), (16, 0, 32, 12), (0, 12, 16, 24), (16, 12, 32, 24))]
        if max(q) > worst[0]:
            worst = (max(q), f.rsplit('/', 1)[1], q)
    prev = t
ok = worst[0] < limit
print(f"{'PASS' if ok else 'FAIL'} {len(frames)} frames {size[0]}x{size[1]}: largest quarter difference between consecutive "
      f"frames {worst[0]:.1f} at {worst[1]} (TL TR BL BR {' '.join(f'{v:.1f}' for v in worst[2])}; limit {limit})")
sys.exit(0 if ok else 1)
EOF
