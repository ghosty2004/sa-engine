#!/usr/bin/env python3
"""How the player moves on the floors of tools/e2e/samp/floor.cmds.

    floor-drift.py [--max-slide M] SERVER_LOG [SERVER_LOG ...]
    floor-drift.py --frames SA_GAME_LOG [--min-change C]

From the server's "[cov]" lines, for each floor (the two teleports per floor after `track`, in the order of
floor.cmds) and each log, in metres on the ground plane: land = how far the player moved while landing after the
first teleport (the samples before the 8th, near where it then stood), stand = the farthest it got from there until
the walk started, walk / sprint = from where it stood to where it was when the server cleared the animation
("anim-end"), and the slides after them = how far it went on from there until it came to rest; z = its height while
standing. The server sees the player a sync interval late, so a slide includes the distance covered in that
interval; compare a floor with the street of the same log. With --max-slide the exit status is 1 when, on any floor
of any log, land or stand exceeds M, or a slide exceeds the street's of the same log by more than M.

--frames reads the samp mod's own view instead, frame by frame: sa-game run with
SA_GAME_DUMP='B7CD98>14>30:c,B7CD98>44:c,B7CD98>568:4,B7CB5C:4' (the player's position, move speed, ground entity
and the time step at every present). For each sprint: what the player stood on and the largest change of its move
speed in one frame, per time step. The game limits that change to 0.01 for a ped standing on a vehicle or object
unless `GameOption::StandingSpeedClamp` is off, as the samp mod switches it; the sprint on the street changes by
about 0.03. With --min-change the exit status is 1 when a sprint changes by less than C (0.015 tells the two apart).
"""
import argparse
import math
import re
import sys

FLOORS = ["19377", "19379", "18766", "3095", "street"]
# samples skipped after a teleport while the player falls onto the floor and lands
LANDING = 8
XYZ = r"(-?[\d.]+),(-?[\d.]+),(-?[\d.]+)"


def segments(path):
    """The where samples between the scenario's events: [(event, position or None, [(x, y, z), ...]), ...]."""
    out = [("start", None, [])]
    for line in open(path, errors="replace"):
        m = re.search(r"\[cov\] where st=\d+ pos=" + XYZ, line)
        if m:
            out[-1][2].append(tuple(float(v) for v in m.groups()))
            continue
        m = re.search(r"\[cov\] \d+ done (pos|animfor|track)\b", line)
        if m:
            out.append((m.group(1), None, []))
            continue
        m = re.search(r"\[cov\] anim-end pos=" + XYZ, line)
        if m:
            out.append(("anim-end", tuple(float(v) for v in m.groups()), []))
    return out


def flat(a, b):
    return math.dist(a[:2], b[:2])


def move(segs, i):
    """(distance, slide) of the move whose teleport is segs[i] (then animfor, anim-end)."""
    start = segs[i][2][-1]
    end = segs[i + 2][1]
    rest = segs[i + 2][2][-1] if segs[i + 2][2] else end
    return flat(start, end), flat(end, rest)


def floors(path):
    """(floor, land, stand, walk, walk slide, sprint, sprint slide, z) per floor of the log."""
    segs = segments(path)
    first = next((i for i, s in enumerate(segs) if s[0] == "track"), None)
    if first is None:
        return []
    poses = [i for i, s in enumerate(segs) if s[0] == "pos" and i > first]
    rows = []
    for k, name in enumerate(FLOORS):
        if 2 * k + 1 >= len(poses):
            break
        i, j = poses[2 * k], poses[2 * k + 1]
        if any(n + 2 >= len(segs) or segs[n + 1][0] != "animfor" or segs[n + 2][0] != "anim-end" for n in (i, j)):
            break
        stand = segs[i][2][LANDING:]
        if not stand or not segs[j][2]:
            break
        rest = stand[0]
        landing = [p for p in segs[i][2][:LANDING] if flat(p, rest) < 3.0]
        walk, walk_slide = move(segs, i)
        sprint, sprint_slide = move(segs, j)
        rows.append((name, max((flat(p, rest) for p in landing), default=0.0), max(flat(p, rest) for p in stand),
                     walk, walk_slide, sprint, sprint_slide, sum(p[2] for p in stand) / len(stand)))
    return rows


def server_logs(paths, limit):
    bad = False
    head = ["floor", "land", "stand", "walk", "slide", "sprint", "slide", "z"]
    print(f"{'log':<40} {head[0]:<7}" + "".join(f" {h:>7}" for h in head[1:]))
    for path in paths:
        rows = floors(path)
        if len(rows) < len(FLOORS):
            print(f"{path}: only {len(rows)} of {len(FLOORS)} floors in the log")
            bad = True
        street = next((r for r in rows if r[0] == "street"), None)
        for r in rows:
            print(f"{path[-40:]:<40} {r[0]:<7}" + "".join(f" {v:7.2f}" for v in r[1:]))
            if limit is not None and street is not None and (
                max(r[1], r[2]) > limit or r[4] - street[4] > limit or r[6] - street[6] > limit
            ):
                bad = True
    return bad


def frames(path):
    """[(position, move speed, ground entity, time step)] of every present of the dump."""
    out = []
    cur = {}
    for line in open(path, errors="replace"):
        m = re.match(r"sa-game: dump (\S+) = 0x[0-9a-f]+:(.*)", line)
        if not m:
            continue
        words = re.findall(r"([0-9a-f]{8})\(([^)]*)\)", m.group(2))
        key = m.group(1)
        if key == "B7CD98>14>30":
            cur = {"pos": [float(w[1]) for w in words]}
        elif key == "B7CD98>44":
            cur["speed"] = [float(w[1]) for w in words]
        elif key == "B7CD98>568":
            cur["ground"] = int(words[0][0], 16)
        elif key == "B7CB5C" and "pos" in cur and "speed" in cur and "ground" in cur:
            out.append((cur["pos"], cur["speed"], cur["ground"], float(words[0][1])))
    return out


def sprints(path, minimum):
    """Per sprint (a run of frames faster than 0.1 a step, with the 30 frames before it): the height, whether the
    player stood on an entity, and the largest change of the move speed in one frame per time step."""
    fs = frames(path)
    speed = [math.hypot(f[1][0], f[1][1]) for f in fs]
    bad = False
    count = 0
    i = 0
    print(f"{'z':>7} {'on':<7} {'change':>7}")
    while i < len(fs):
        if speed[i] <= 0.1:
            i += 1
            continue
        j = i
        while j < len(fs) and speed[j] > 0.002:
            j += 1
        span = fs[max(i - 30, 0):j + 1]
        change = max((math.hypot(b[1][0] - a[1][0], b[1][1] - a[1][1]) / b[3] for a, b in zip(span, span[1:])
                      if a[2] == b[2] and b[3] > 0), default=0.0)
        print(f"{fs[i][0][2]:7.2f} {'entity' if fs[i][2] else 'ground':<7} {change:7.4f}")
        count += 1
        if minimum is not None and change < minimum:
            bad = True
        i = j + 1
    if count < len(FLOORS):
        print(f"{path}: only {count} sprints (SA_GAME_DUMP set?)")
        bad = True
    return bad


def main():
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("logs", nargs="+")
    p.add_argument("--max-slide", type=float)
    p.add_argument("--frames", action="store_true")
    p.add_argument("--min-change", type=float)
    a = p.parse_args()
    if a.frames:
        bad = any([sprints(path, a.min_change) for path in a.logs])
    else:
        bad = server_logs(a.logs, a.max_slide)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
