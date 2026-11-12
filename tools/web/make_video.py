#!/usr/bin/env python3
"""Assembles a `tools/web/headless.mjs --screencast` recording into an MP4 (H.264, yuv420p).

    python3 -I tools/web/make_video.py OUT/cast out.mp4 [--speed 8] [--fps 25]

The screencast frames keep their capture times (a frame lasts until the next one). The loading stretch is played
`--speed` times faster with an on-screen note; everything else is real time. The loading stretch runs from the first
status that leaves the menus (gGameState 8 -> anything else) to the first in-game frame (`presents` > 0), unless
`--fast FROM:TO` (seconds since the first frame) gives it explicitly.
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
    a = ap.parse_args()

    frames = [json.loads(l) for l in open(os.path.join(a.cast, "frames.jsonl")) if l.strip()]
    events = [json.loads(l) for l in open(os.path.join(a.cast, "events.jsonl")) if l.strip()]
    if not frames:
        sys.exit("no frames")
    t0 = frames[0]["ts"]

    if a.fast:
        f0, f1 = (float(x) for x in a.fast.split(":"))
    else:
        f0 = f1 = None
        was_menu = False
        for e in events:
            st = e.get("status")
            if not st:
                continue
            if st.get("state") == 8:
                was_menu = True
            elif was_menu and f0 is None:
                f0 = e["ts"] - t0
            if f0 is not None and (st.get("presents") or 0) > 0:
                f1 = e["ts"] - t0
                break
        if f0 is None or f1 is None:
            f0 = f1 = 0.0
    print(f"loading stretch {f0:.1f}..{f1:.1f} s at {a.speed}x", file=sys.stderr)

    def out_time(t):
        # source time (s since the first frame) -> output time
        if t <= f0:
            return t
        if t <= f1:
            return f0 + (t - f0) / a.speed
        return f0 + (f1 - f0) / a.speed + (t - f1)

    lst = os.path.join(a.cast, "concat.txt")
    with open(lst, "w") as f:
        for i, fr in enumerate(frames):
            t = fr["ts"] - t0
            nxt = frames[i + 1]["ts"] - t0 if i + 1 < len(frames) else t + 1.0
            d = max(out_time(nxt) - out_time(t), 0.001)
            f.write(f"file '{fr['file']}'\nduration {d:.4f}\n")
        f.write(f"file '{frames[-1]['file']}'\n")

    o0, o1 = out_time(f0), out_time(f1)
    note = f"loading - {a.speed:g}x speed"
    vf = (
        "pad=ceil(iw/2)*2:ceil(ih/2)*2,"
        f"drawtext=text='{note}':fontcolor=yellow:fontsize=28:box=1:boxcolor=black@0.6:boxborderw=8:"
        f"x=(w-tw)/2:y=h-th-40:enable='between(t,{o0:.3f},{o1:.3f})',"
        f"fps={a.fps},format=yuv420p"
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
