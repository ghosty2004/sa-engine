#!/usr/bin/env python3
"""Assembles a `tools/web/headless.mjs --screencast` recording into an MP4 (H.264, yuv420p).

    python3 -I tools/web/make_video.py OUT/cast out.mp4 [--speed 8] [--fps 25]

The screencast frames keep their capture times (a frame lasts until the next one). The loading stretches are played
`--speed` times faster with an on-screen note; everything else is real time. The fast stretches are the boot's
first long iteration (gGameState LOGO) and the world loading (from leaving the menus to the first in-game frame),
unless `--fast FROM:TO[,FROM:TO]` (seconds since the first frame) gives them.
"""
import argparse
import json
import os
import subprocess
import sys


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("cast")
    ap.add_argument("out")
    ap.add_argument("--speed", type=float, default=8.0)
    ap.add_argument("--fps", type=int, default=25)
    ap.add_argument("--fast", default=None)
    ap.add_argument("--end", type=float, default=None, help="stop at this many seconds after the first frame")
    a = ap.parse_args()

    frames = [json.loads(l) for l in open(os.path.join(a.cast, "frames.jsonl")) if l.strip()]
    events = [json.loads(l) for l in open(os.path.join(a.cast, "events.jsonl")) if l.strip()]
    if not frames:
        sys.exit("no frames")
    t0 = frames[0]["ts"]
    if a.end is not None:
        frames = [f for f in frames if f["ts"] - t0 <= a.end]

    # stretches played faster: the boot's first long iteration (gGameState LOGO until the logos play) and the
    # world loading (leaving the menus until the first in-game frame); or --fast FROM:TO[,FROM:TO]
    fast = []
    if a.fast:
        for part in a.fast.split(","):
            x, y = part.split(":")
            fast.append((float(x), float(y)))
    else:
        sts = [(e["ts"] - t0, e["status"]) for e in events if e.get("status")]
        logo = next((t for t, st in sts if st.get("state") == 1), None)
        logo_end = next((t for t, st in sts if logo is not None and t > logo and (st.get("state") or 0) >= 2), None)
        if logo is not None and logo_end is not None and logo_end - logo > 5:
            fast.append((logo, logo_end))
        menu = next((t for t, st in sts if st.get("state") == 7), None)
        load = next((t for t, st in sts if menu is not None and t > menu and (st.get("state") or 0) >= 8), None)
        load_end = next((t for t, st in sts if load is not None and t >= load and (st.get("presents") or 0) > 0), None)
        if load is not None and load_end is not None:
            fast.append((load, load_end))
    for f0, f1 in fast:
        print(f"fast stretch {f0:.1f}..{f1:.1f} s at {a.speed}x", file=sys.stderr)

    def out_time(t):
        # source time (s since the first frame) -> output time
        o = t
        for f0, f1 in fast:
            if t > f0:
                o -= (min(t, f1) - f0) * (1 - 1 / a.speed)
        return o

    lst = os.path.join(a.cast, "concat.txt")
    with open(lst, "w") as f:
        for i, fr in enumerate(frames):
            t = fr["ts"] - t0
            nxt = frames[i + 1]["ts"] - t0 if i + 1 < len(frames) else t + 1.0
            d = max(out_time(nxt) - out_time(t), 0.001)
            f.write(f"file '{fr['file']}'\nduration {d:.4f}\n")
        f.write(f"file '{frames[-1]['file']}'\n")

    note = f"loading - {a.speed:g}x speed"
    enable = "+".join(f"between(t,{out_time(f0):.3f},{out_time(f1):.3f})" for f0, f1 in fast) or "0"
    vf = (
        "pad=ceil(iw/2)*2:ceil(ih/2)*2,"
        f"drawtext=text='{note}':fontcolor=yellow:fontsize=28:box=1:boxcolor=black@0.6:boxborderw=8:"
        f"x=(w-tw)/2:y=h-th-40:enable='{enable}',"
        f"fps={a.fps},scale=in_range=full:out_range=tv,format=yuv420p"
    )
    cmd = [
        "ffmpeg", "-y", "-hide_banner", "-loglevel", "error", "-f", "concat", "-safe", "0", "-i", lst,
        "-vf", vf, "-c:v", "libx264", "-preset", "medium", "-crf", "20", "-pix_fmt", "yuv420p",
        "-movflags", "+faststart", a.out,
    ]
    subprocess.run(cmd, check=True)
    print(a.out)


if __name__ == "__main__":
    main()
