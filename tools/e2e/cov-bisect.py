#!/usr/bin/env python3
"""INF-10 diagnostic: find the coverage point(s) whose breakpoint stops a replay.

    tools/e2e/cov-bisect.py POINTS REC END [--timeout S] [--once]

Each probe is one replay (tools/e2e/l3.sh --probe, every port on the original) with a subset of the
points armed, up to frame END; it is "bad" when the trace does not reach END (hang or crash). Binary
splitting over the address-sorted points (keeps a half that is bad on its own; stops and reports both
halves when only their union is bad). --once: one probe with every point, prints the calibration and
the last first-hits (POINTS.last) and exits. Env: LANE, SA_ROOT (as for l3.sh).
"""
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(HERE))


def main():
    args = sys.argv[1:]
    if len(args) < 3:
        print(__doc__)
        return 2
    points_path, rec, end = args[0], os.path.abspath(args[1]), int(args[2])
    timeout = int(args[args.index("--timeout") + 1]) if "--timeout" in args else 180
    once = "--once" in args
    lines = [l for l in open(points_path) if l.strip() and not l.startswith("#")]
    oracle = os.path.join(ROOT, "target/release/sa-oracle")
    modes = subprocess.run([oracle, "l3", "--print-modes", "ports", "--except", "ports"],
                           capture_output=True, text=True, check=True).stdout.strip()
    work = tempfile.mkdtemp(prefix="cov-bisect-", dir=os.path.join(ROOT, "target"))
    n = [0]

    def bad(sub):
        n[0] += 1
        pf = os.path.join(work, f"p{n[0]:02d}.points")
        with open(pf, "w") as f:
            f.writelines(sub)
        out = os.path.join(work, f"p{n[0]:02d}.trace")
        env = dict(os.environ, L3_REC=rec, L3_COV_POINTS=pf, RUN_TIMEOUT=str(timeout))
        subprocess.run([os.path.join(HERE, "l3.sh"), "--probe", f"covb{n[0]:02d}", str(end), modes, out],
                       env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        log = open(out + ".log", errors="replace").read() if os.path.exists(out + ".log") else ""
        ok = f"end frame {end} reached" in log
        print(f"probe {n[0]:2d}: {len(sub):6d} points {sub[0].split()[0]}..{sub[-1].split()[0]}: "
              f"{'reaches' if ok else 'STOPS before'} frame {end}", flush=True)
        return not ok, out, log

    if once:
        b, out, log = bad(lines)
        for l in log.splitlines():
            if "coverage:" in l:
                print(l)
        last = out + ".cov.csv.last"
        if os.path.exists(last):
            print("".join(open(last).readlines()[-24:]))
        return 0
    cand = lines
    if not bad(cand)[0]:
        print("all points: the replay reaches the end; nothing to bisect")
        return 0
    while len(cand) > 1:
        a, b = cand[: len(cand) // 2], cand[len(cand) // 2:]
        if bad(a)[0]:
            cand = a
        elif bad(b)[0]:
            cand = b
        else:
            print(f"neither half alone stops the run: interaction between {a[0].split()[0]}..{a[-1].split()[0]} "
                  f"and {b[0].split()[0]}..{b[-1].split()[0]}")
            return 1
    print(f"CULPRIT POINT: {cand[0].strip()}  (work dir {work})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
