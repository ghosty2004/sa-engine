#!/usr/bin/env python3
"""Per-hook L2 / split evidence for whole unverified batches, automated (docs/verify/HOWTO.md §4.3-4.6).

  batch-l2.py plan BATCH|@FILE...           the hooks of those batches and the run kind of each (below);
                                            writes target/verify/batch-l2/plan.tsv
  batch-l2.py run REC LANE [--kind K] [--chunk N] [--only 0xA,...]
                                            port-l2.sh over the planned hooks of kind K (l2 | heap | split)
                                            that still lack evidence and were not run on REC yet, in lists
                                            of at most N hooks (no two of one name: results.tsv keys by name);
                                            a list whose run does not reach the end frame is halved and re-run
                                            until the hook that stops it is alone (verdict CRASH); every
                                            hook's result is appended to target/verify/batch-l2/results.tsv
  batch-l2.py report [--batches]            per-hook verdict over every recording (and per batch: promotable
                                            under HOWTO §4.6 or what is missing)
  batch-l2.py setkind KIND 0xA[,0xB...]    run those hooks as KIND from now on (an in-process double run that
                                            cannot work, e.g. a cache holding stack addresses: split)
  batch-l2.py forget 0xA[,0xB...]           drop the recorded results of those hooks (after a port fix: re-run them)
  batch-l2.py reparse                       re-read every recorded run directory (after a parser change)
  batch-l2.py promote BATCH...              HOWTO §4.6 step 6 for batches `report --batches` marks PROMOTE: `unverified`
                                            leaves the hand-written attributes, generated AI hooks leave
                                            tools/gen/ai_patterns_unverified.txt, gen2 hooks join patterns_verified.txt
                                            (regenerate both generators afterwards); catalog/state.csv gets L2 / L3
                                            for the rows with evidence (progress.py set), NOCALLS rows keep their state
  batch-l2.py record                        queue.py statuses: L2 for every hook with evidence (bulk)

Run kinds (from the queue row; a writer's `route` is a hint, the hazards decide how it can run; route
L1-pure rows run as l2 too: an L2 result is evidence, `sa-oracle l1` stays the route for those without calls):
  split  route split, hazards uninit-locals / stack-residue, or a destroyer / allocator by name (an
         in-process double run frees or allocates twice): `--runs "plain split"`, SPLIT_STACKFILL=4096
  l3     route L3-only (whole-frame and init functions, no usable double run): hook-off evidence,
         `run REC LANE --kind l3` = l3.sh --rust LIST --fixed verified --hookoff all (verdict HOOKOFF-PASS)
  heap   hazards heap-writes: `--runs "plain l2 l2all"` with the list as --window-all (heap arena watched)
  l2     everything else (L3-only rows too: their L2 run is tried, hook-off is the fallback)

A hook has evidence when one recording gave port-l2's `(a)` PASS (>= L2_MAX cases or every call, 0 mismatches,
0 undeclared writes) or a split PASS / PASS-FP with 0 undeclared, and no recording gave a mismatch. A hook that
got no call on any of the 13 recordings needs none (HOWTO §4.6: "every row that has calls").
Env: SA_ROOT (data checkout), L2_MAX (default 200, as VERIFIER-11), L2_PER_FRAME (2), SPLIT_PER_FRAME (20), RUN_TIMEOUT
(default 3000 s: an L2 run snapshots the static window for every case).
"""
import csv
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
import importlib.util  # noqa: E402

# tools/verify/queue.py (the queue rows, recording names, status file); loaded by path, `queue` is a stdlib name
_spec = importlib.util.spec_from_file_location("verify_queue", HERE / "queue.py")
vqueue = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(vqueue)

OUT = ROOT / "target/verify/batch-l2"
PLAN = OUT / "plan.tsv"
RESULTS = OUT / "results.tsv"
RFIELDS = ["tag", "rec", "kind", "addr", "name", "batch", "calls", "cases", "mismatches", "undeclared", "verdict"]
DESTROY = re.compile(r"Destructor|Destroy|Delete|Remove|Shutdown|~|deleting|operator delete|::Free|Release|Clear",
                     re.I)
ALLOC = re.compile(r"operator new|::New\b|Alloc|Create|Constructor|Initialise|Init\b|Load", re.I)
GOOD = ("PASS", "PASS-FP", "HOOKOFF-PASS")


def sa_root():
    return pathlib.Path(os.environ.get("SA_ROOT") or str(pathlib.Path.home() / "sa-engine"))


def recordings():
    """name -> path of the 13 corpus recordings (verify-all's set)."""
    root = sa_root()
    out = {vqueue.GROVE: root / "target/e2e/lane-2/inf4/rec.sarec"}
    for p in sorted((root / "target/e2e/corpus").glob("*.sarec")):
        out[p.stem] = p
    return out


def kind_of(r):
    haz = {x.strip() for x in r["hazards"].split(",")}
    if r["route"] == "L3-only":
        return "l3"
    if r["route"] == "split" or haz & {"uninit-locals", "stack-residue"} or DESTROY.search(r["name"]) \
            or (ALLOC.search(r["name"]) and "heap-writes" in haz):
        return "split"
    if "heap-writes" in haz:
        return "heap"
    return "l2"


def read_tsv(p):
    if not p.exists():
        return []
    return list(csv.DictReader(p.open(), delimiter="\t"))


def write_tsv(p, rows, fields, append=False):
    p.parent.mkdir(parents=True, exist_ok=True)
    new = not p.exists() or not append
    with p.open("a" if append else "w") as f:
        w = csv.DictWriter(f, fields, delimiter="\t", lineterminator="\n", extrasaction="ignore")
        if new:
            w.writeheader()
        w.writerows(rows)


def plan(args):
    want = []
    for a in args:
        want += [x.strip() for x in pathlib.Path(a[1:]).read_text().split()] if a.startswith("@") else [a]
    want = set(want)
    rows = [r for r in csv.DictReader(vqueue.QUEUE.open()) if r["batch"] in want]
    missing = want - {r["batch"] for r in rows}
    if missing:
        sys.exit(f"batch-l2: not in the queue: {', '.join(sorted(missing))}")
    out = [dict(r, kind=kind_of(r)) for r in rows]
    write_tsv(PLAN, out, ["address", "name", "batch", "route", "hazards", "kind"])
    n = {}
    for r in out:
        n[r["kind"]] = n.get(r["kind"], 0) + 1
    print(f"batch-l2: {len(out)} hooks in {len(want)} batches: " + ", ".join(f"{k} {v}" for k, v in sorted(n.items())))


def verdicts():
    """addr -> {rec: result row} (the last run of each hook on each recording counts; only runs of the
    hook's planned kind: `setkind` drops the results of the kind it replaces)."""
    kind = {r["address"].upper(): r["kind"] for r in read_tsv(PLAN)}
    out = {}
    for r in read_tsv(RESULTS):
        k = kind.get(r["addr"].upper())
        if k and r["kind"] != k:
            continue
        out.setdefault(r["addr"].upper(), {})[r["rec"]] = r
    return out


def summary(per_rec, nrecs):
    """(verdict, evidence) of one hook from its per-recording results."""
    rs = list(per_rec.values())
    bad = [r for r in rs if r["verdict"] in ("MISMATCH", "FAIL")]
    if bad:
        r = bad[0]
        return "MISMATCH", f"{r['tag']} ({r['rec']}) {r['kind']} {r['verdict']} {r['cases']} cases {r['mismatches']} mismatches"
    good = [r for r in rs if r["verdict"] in GOOD]
    und = [r for r in rs if r["verdict"] == "UNDECL"]
    if und:
        # an undeclared write on any recording is a finding (PLAN §5 L2: read/write sets identical)
        return "open", f"UNDECL on {', '.join(r['rec'] for r in und)}"
    if good:
        r = max(good, key=lambda r: int(r["cases"] or 0))
        if r["verdict"] == "HOOKOFF-PASS":
            return "L3", f"{r['tag']} ({r['rec']}) on Rust on top of the verified set and hook-off: MATCH"
        what = "split" if r["kind"] == "split" else "L2"
        return "L2", (f"{r['tag']} ({r['rec']}) {what} {r['verdict']} {r['cases']} cases of {r['calls'] or '?'} calls "
                      f"0 mismatches 0 undeclared")
    # several recordings together reach the case count (each one sampled every call it could)
    few = [r for r in rs if r["verdict"] == "FEW"]
    if few and sum(int(r["cases"] or 0) for r in few) >= int(os.environ.get("L2_MAX", "200")):
        return "L2", "; ".join(f"{r['tag']} ({r['rec']}) {r['cases']} cases" for r in few) + \
            " (L2 0 mismatches 0 undeclared, together >= L2_MAX cases)"
    if len(per_rec) >= nrecs and all(r["verdict"] in ("NOCALLS", "NOCASES") for r in rs):
        return "NOCALLS", f"no call on any of the {nrecs} recordings"
    # an L3-only hook's calls are known only from the INF-10 coverage files (pause-restart, grove walk): a
    # hook that is called in none of them and matched with hook-off there has no call to compare
    if rs and all(r["kind"] == "l3" and r["verdict"] == "NOCALLS" for r in rs):
        return "NOCALLS", f"{rs[0]['tag']}: hook-off MATCH; no call in the INF-10 coverage of any covered recording"
    opens = sorted({r["verdict"] for r in rs if r["verdict"] not in ("NOCALLS", "NOCASES")})
    return "open", ",".join(opens) or f"run on {len(per_rec)} of {nrecs} recordings"


def parse_run(outdir, kind, tag, rec, hooks):
    """Per-hook result rows of one port-l2.sh run directory."""
    res = {r["addr"].upper(): r for r in read_tsv(outdir / "results.tsv")}
    log = (outdir / "summary.txt").read_text() if (outdir / "summary.txt").exists() else ""
    reached = {"l2": "PASS l2 reached the end frame" in log, "split": "PASS splitr reached the end frame" in log
               and "PASS splito reached the end frame" in log}
    reached["heap"] = reached["l2"] and "PASS l2all reached the end frame" in log
    ok = reached["split" if kind == "split" else kind]
    split = {}
    rep = outdir / "split-report.txt"
    if rep.exists():
        for line in rep.read_text().splitlines():
            f = line.split()
            if len(f) >= 4 and f[0] == "SPLIT":
                kv = dict(x.split("=", 1) for x in f[4:] if "=" in x)
                split[int(f[2], 16)] = (f[1], kv.get("undeclared_cases", ""), kv.get("compared", ""),
                                        kv.get("plain_cases", ""), kv.get("mismatches", ""))
    rows = []
    for h in hooks:
        a = int(h["address"], 16)
        r = res.get(f"0X{a:X}") or res.get(f"{a}") or {}
        row = {"tag": tag, "rec": rec, "kind": kind, "addr": h["address"].upper().replace("0X", "0x"), "name": h["name"],
               "batch": h["batch"], "calls": r.get("calls", ""), "cases": r.get("cases", ""),
               "mismatches": r.get("mismatches", ""), "undeclared": r.get("undeclared_cases", "")}
        if kind == "split":
            v, und, cases, calls, mism = split.get(a, ("NOCASES", "", "", "", ""))
            row.update(cases=cases, undeclared=und, calls=calls, mismatches=mism)
            if v in GOOD and und not in ("", "0"):
                v = "UNDECL"
            elif v == "FAIL":
                v = "MISMATCH"
            row["verdict"] = v if ok or v == "MISMATCH" else "CRASH"
        else:
            v = r.get("l2", "norun")
            if v == "FAIL":
                if (r.get("mismatches") or "0") != "0":
                    v = "MISMATCH"
                elif (r.get("undeclared_cases") or "0") != "0":
                    v = "UNDECL"
                else:
                    v = "FEW"
            if not ok and v != "MISMATCH":
                v = "CRASH"
            row["verdict"] = v
        rows.append(row)
    return ok, rows


def port_l2(hooks, kind, recpath, lane, tag):
    lst = OUT / "lists" / f"{tag}.txt"
    lst.parent.mkdir(parents=True, exist_ok=True)
    lst.write_text("".join(h["address"] + "\n" for h in hooks))
    env = dict(os.environ, LANE=str(lane), SA_ROOT=str(sa_root()), L2_SPOOL_MAX=os.environ.get("L2_SPOOL_MAX", "1G"),
               L2_MAX=os.environ.get("L2_MAX", "200"), L2_PER_FRAME=os.environ.get("L2_PER_FRAME", "2"),
               RUN_TIMEOUT=os.environ.get("RUN_TIMEOUT", "3000"))
    cmd = [str(ROOT / "tools/e2e/port-l2.sh"), f"@{lst}", str(recpath), "--tag", tag]
    if kind == "split":
        cmd += ["--runs", "plain split"]
        env.setdefault("SPLIT_STACKFILL", "4096")
        env.setdefault("SPLIT_PER_FRAME", "20")
    elif kind == "heap":
        cmd += ["--runs", "plain l2", "--window-all", f"@{lst}"]
        env.setdefault("L2_SPOOL_DISCARD", "1")
    else:
        cmd += ["--runs", "plain l2"]
        env.setdefault("L2_SPOOL_DISCARD", "1")
    log = OUT / "logs" / f"{tag}.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("w") as f:
        subprocess.run(["systemd-run", "--user", "--scope", "-q", "-p", f"MemoryMax={os.environ.get('MEM', '6G')}"]
                       + cmd, env=env, stdout=f, stderr=subprocess.STDOUT, cwd=ROOT)
    outdir = ROOT / f"target/e2e/lane-{lane}/port-l2-{tag}"
    ok, rows = parse_run(outdir, kind, tag, recpath.stem if recpath.stem != "rec" else vqueue.GROVE, hooks)
    # the binary spools are only needed for the split report (already written): bounded disk use
    for p in outdir.glob("*.spool"):
        p.unlink()
    for p in outdir.glob("*.trace"):
        if p.name != "plain.trace":
            p.unlink()
    return ok, rows


def l3_hookoff(hooks, recname, recpath, lane, tag):
    """L3-only hooks: the list on Rust on top of the verified set, then each one back on the original alone
    (tools/e2e/l3.sh --hookoff all). A hook that the recording's INF-10 coverage shows uncalled is NOCALLS."""
    rep = OUT / "l3" / f"{tag}.txt"
    rep.parent.mkdir(parents=True, exist_ok=True)
    rep.unlink(missing_ok=True)
    addrs = ",".join(h["address"] for h in hooks)
    env = dict(os.environ, LANE=str(lane), SA_ROOT=str(sa_root()), L3_REPORT_COPY=str(rep),
               SA_UNVERIFIED=",".join(sorted({h["batch"] for h in hooks})))
    log = OUT / "logs" / f"{tag}.log"
    log.parent.mkdir(parents=True, exist_ok=True)
    with log.open("w") as f:
        subprocess.run(["systemd-run", "--user", "--scope", "-q", "-p", f"MemoryMax={os.environ.get('MEM', '6G')}",
                        str(ROOT / "tools/e2e/l3.sh"), "--rec", str(recpath), "--rust", addrs, "--fixed", "verified",
                        "--hookoff", "all"], env=env, stdout=f, stderr=subprocess.STDOUT, cwd=ROOT)
    text = rep.read_text() if rep.exists() else ""
    off = {}
    for m in re.finditer(r"^HOOKOFF (.*) back on the original, rest on Rust: (MATCH|DIVERGED)", text, re.M):
        off[m.group(1)] = m.group(2)
    culprits = re.findall(r"^CULPRIT (.*)$", text, re.M)
    cov = vqueue.coverage(sa_root())
    has_cov = (sa_root() / f"catalog/raw/coverage-{recname}.csv").exists()
    rows = []
    for h in hooks:
        a = int(h["address"], 16)
        v = off.get(h["name"])
        if v == "MATCH":
            v = "HOOKOFF-PASS" if not has_cov or recname in cov.get(a, []) else "NOCALLS"
        elif v == "DIVERGED":
            v = "MISMATCH"
        else:
            v = "MISMATCH" if any(h["name"] in c for c in culprits) else "CRASH"
        rows.append({"tag": tag, "rec": recname, "kind": "l3", "addr": h["address"], "name": h["name"],
                     "batch": h["batch"], "calls": "", "cases": "", "mismatches": "", "undeclared": "", "verdict": v})
    return bool(off), rows


def chunks(hooks, n):
    """Lists of at most n hooks, no name twice in one list."""
    out = []
    for h in hooks:
        for c in out:
            if len(c) < n and all(x["name"] != h["name"] for x in c):
                c.append(h)
                break
        else:
            out.append([h])
    return out


def run(args):
    import argparse
    ap = argparse.ArgumentParser(prog="batch-l2.py run")
    ap.add_argument("rec")
    ap.add_argument("lane", type=int)
    ap.add_argument("--kind", default="l2", choices=("l2", "heap", "split", "l3"))
    ap.add_argument("--chunk", type=int, default=150)
    ap.add_argument("--only", default="")
    ap.add_argument("--rerun", action="store_true", help="also hooks already run on this recording")
    o = ap.parse_args(args)
    recs = recordings()
    rec = o.rec if o.rec in recs else pathlib.Path(o.rec).stem
    if rec not in recs:
        sys.exit(f"batch-l2: unknown recording {o.rec} (one of {', '.join(recs)})")
    v = verdicts()
    only = {int(x, 16) for x in o.only.split(",") if x}
    todo = []
    for h in read_tsv(PLAN):
        a = int(h["address"], 16)
        if only and a not in only:
            continue
        if h["kind"] != o.kind:
            continue
        per = v.get(h["address"].upper(), {})
        if not only and summary(per, len(recs))[0] in ("L2", "L3", "MISMATCH"):
            continue
        if rec in per and per[rec]["kind"] == o.kind and not o.rerun:
            continue
        todo.append(h)
    print(f"batch-l2: {len(todo)} {o.kind} hooks on {rec}, lane {o.lane}", flush=True)
    if o.kind == "l3":
        if todo:
            tag = f"bl3-{rec}-{os.getpid()}"
            ok, rows = l3_hookoff(todo, rec, recs[rec], o.lane, tag)
            print(f"batch-l2: {tag}: {len(todo)} hooks, {'report' if ok else 'NO REPORT'}; "
                  f"{sum(1 for r in rows if r['verdict'] == 'HOOKOFF-PASS')} hook-off pass", flush=True)
            write_tsv(RESULTS, rows, RFIELDS, append=True)
        return
    queue_ = chunks(todo, o.chunk)
    n = 0
    while queue_:
        c = queue_.pop(0)
        n += 1
        tag = f"bl2-{o.kind}-{rec}-{os.getpid()}-{n}"
        ok, rows = port_l2(c, o.kind, recs[rec], o.lane, tag)
        good = sum(1 for r in rows if r["verdict"] in GOOD)
        print(f"batch-l2: {tag}: {len(c)} hooks, {'end frame' if ok else 'STOPPED'}; "
              f"{good} pass, {sum(1 for r in rows if r['verdict'] == 'MISMATCH')} mismatch", flush=True)
        if not ok and len(c) > 1:
            # keep the verdicts the stopped run still proves (mismatches), halve the rest and re-run
            write_tsv(RESULTS, [r for r in rows if r["verdict"] == "MISMATCH"], RFIELDS, append=True)
            rest = [h for h, r in zip(c, rows) if r["verdict"] != "MISMATCH"]
            k = len(rest) // 2
            queue_[:0] = [x for x in (rest[:k], rest[k:]) if x]
            continue
        write_tsv(RESULTS, rows, RFIELDS, append=True)


def report(args):
    recs = recordings()
    v = verdicts()
    plan_ = read_tsv(PLAN)
    rows = []
    for h in plan_:
        verdict, ev = summary(v.get(h["address"].upper(), {}), len(recs))
        rows.append(dict(h, verdict=verdict, evidence=ev,
                         recs=" ".join(sorted(k for k, r in v.get(h["address"].upper(), {}).items()
                                              if r["verdict"] not in ("NOCALLS", "NOCASES")))))
    write_tsv(OUT / "verdicts.tsv", rows, ["address", "name", "batch", "route", "hazards", "kind", "verdict",
                                            "evidence", "recs"])
    n = {}
    for r in rows:
        n[r["verdict"]] = n.get(r["verdict"], 0) + 1
    print("batch-l2: " + ", ".join(f"{k} {c}" for k, c in sorted(n.items())) + f" -> {OUT / 'verdicts.tsv'}")
    if "--batches" in args:
        st = {int(r["address"], 16): r["status"] for r in csv.DictReader(vqueue.QUEUE.open())}
        by = {}
        for r in rows:
            by.setdefault(r["batch"], []).append(r)
        for b, rs in sorted(by.items()):
            miss = [r for r in rs if r["verdict"] not in ("L2", "L3", "NOCALLS")
                    and st.get(int(r["address"], 16)) != "L1"]
            print(f"{b}\t{'PROMOTE' if not miss else 'missing ' + str(len(miss))}\t"
                  + " ".join(f"{r['address']}:{r['verdict']}" for r in miss[:8]))


def record(args):
    rows = [r for r in read_tsv(OUT / "verdicts.tsv") if r["verdict"] in ("L2", "L3")]
    st = vqueue.read_status()
    import datetime
    today = datetime.date.today().isoformat()
    for r in rows:
        a = int(r["address"], 16)
        # an L3-only row keeps the queue status L3-batch; its evidence names the hook-off run
        st[a] = {"address": f"0x{a:X}", "batch": r["batch"], "status": "L2" if r["verdict"] == "L2" else "L3-batch",
                 "date": today,
                 "evidence": f"batch-l2: {r['evidence']}"}
    vqueue.write_status(st)
    print(f"batch-l2: {len(rows)} hooks -> L2")
    return vqueue.gen()


def promote(batches):
    verd = {}
    for r in read_tsv(OUT / "verdicts.tsv"):
        verd.setdefault(r["batch"], []).append(r)
    st = {int(r["address"], 16): r["status"] for r in csv.DictReader(vqueue.QUEUE.open())}
    ai_file = ROOT / "tools/gen/ai_patterns_unverified.txt"
    gen2_file = ROOT / "tools/gen/patterns_verified.txt"
    ai_drop, gen2_add, n = set(), [], 0
    for b in batches:
        rows = verd.get(b)
        if not rows:
            sys.exit(f"batch-l2: {b}: no verdicts")
        miss = [r for r in rows if r["verdict"] not in ("L2", "L3", "NOCALLS") and st.get(int(r["address"], 16)) != "L1"]
        if miss:
            sys.exit(f"batch-l2: {b}: not promotable ({len(miss)} rows without evidence)")
        hooks = {h["addr"]: h for h in vqueue.hookattrs.hooks(ROOT) if h.get("unverified") == b}
        for r in rows:
            a = int(r["address"], 16)
            h = hooks[a]
            path = ROOT / h["file"]
            text = path.read_text()
            if "Generated by `tools/gen/ai_patterns.py`" in text[:300]:
                ai_drop.add(a)
            elif "Generated by" in text[:300]:
                gen2_add.append(f"0x{a:X}  # {r['name']} ({b}, {r['verdict']})")
            else:
                lines = text.split("\n")
                i = h["line"] - 1
                pat = re.compile(r'\s*unverified\s*=\s*"%s"\s*,?' % re.escape(b))
                for j in range(i, min(i + 6, len(lines))):
                    if pat.search(lines[j]):
                        new = pat.sub(" ", lines[j], count=1)
                        new = re.sub(r",\s*\)\]", ")]", new)
                        new = re.sub(r"\(\s+", "(", new).rstrip()
                        if new.strip() == "":
                            del lines[j]
                        else:
                            # keep the attribute's indentation and single spaces
                            lead = len(lines[j]) - len(lines[j].lstrip())
                            lines[j] = lines[j][:lead] + re.sub(r"\s{2,}", " ", new.strip())
                        break
                else:
                    sys.exit(f"batch-l2: {h['file']}:{h['line']}: no `unverified = \"{b}\"`")
                path.write_text("\n".join(lines))
            if r["verdict"] in ("L2", "L3"):
                subprocess.run([sys.executable, "-I", str(ROOT / "tools/catalog/progress.py"), "set", r["address"],
                                r["verdict"], f"batch-l2 promotion of {b}: {r['evidence']}"], check=True, cwd=ROOT,
                               stdout=subprocess.DEVNULL)
            n += 1
    if ai_drop:
        keep = [l for l in ai_file.read_text().splitlines()
                if not (l.strip() and not l.startswith("#") and int(l.split()[0], 16) in ai_drop)]
        ai_file.write_text("\n".join(keep) + "\n")
    if gen2_add:
        gen2_file.write_text(gen2_file.read_text().rstrip("\n") + "\n" + "\n".join(gen2_add) + "\n")
    print(f"batch-l2: {n} hooks of {len(batches)} batches promoted ({len(ai_drop)} generated AI, {len(gen2_add)} gen2: "
          f"regenerate with tools/gen/ai_patterns.py / patterns.py, then queue.py)")


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    cmd, args = sys.argv[1], sys.argv[2:]
    if cmd == "plan" and args:
        return plan(args)
    if cmd == "run":
        return run(args)
    if cmd == "report":
        return report(args)
    if cmd == "setkind" and len(args) == 2:
        want = {int(x, 16) for x in args[1].split(",")}
        rows = read_tsv(PLAN)
        for r in rows:
            if int(r["address"], 16) in want:
                r["kind"] = args[0]
        write_tsv(PLAN, rows, ["address", "name", "batch", "route", "hazards", "kind"])
        return 0
    if cmd == "forget" and len(args) == 1:
        want = {int(x, 16) for x in args[0].split(",")}
        rows = [r for r in read_tsv(RESULTS) if int(r["addr"], 16) not in want]
        write_tsv(RESULTS, rows, RFIELDS)
        return 0
    if cmd == "reparse":
        plan_ = {r["address"].upper(): r for r in read_tsv(PLAN)}
        old = read_tsv(RESULTS)
        out, seen = [], set()
        for r in old:
            if r["tag"] in seen:
                continue
            seen.add(r["tag"])
            rows = [x for x in old if x["tag"] == r["tag"]]
            dirs = sorted(ROOT.glob(f"target/e2e/lane-*/port-l2-{r['tag']}"))
            hooks = [dict(plan_.get(x["addr"].upper(), {}), address=x["addr"], name=x["name"], batch=x["batch"])
                     for x in rows]
            if dirs:
                _, rows = parse_run(dirs[0], r["kind"], r["tag"], r["rec"], hooks)
            out += rows
        write_tsv(RESULTS, out, RFIELDS)
        print(f"batch-l2: {len(out)} result rows re-read")
        return 0
    if cmd == "promote" and args:
        return promote(args)
    if cmd == "record":
        return record(args)
    sys.exit(__doc__)


if __name__ == "__main__":
    sys.exit(main())
