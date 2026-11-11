#!/usr/bin/env python3
"""The verification queue of unverified ports (docs/verify/HOWTO.md, docs/PLAN.md "Writer/verifier split").

  queue.py [gen]                 regenerate catalog/verify-queue.csv and docs/verify/QUEUE.md
  queue.py --check               exit 1 when the generated files are out of date (writer gate)
  queue.py set KEY STATUS [EVIDENCE...]
                                 record a verification result in catalog/verify-status.csv, then regenerate;
                                 KEY = 0xADDR or batch:ID; STATUS = written | L1 | L2 | L3-batch | failed
  queue.py list BATCH            the batch's hook addresses, one per line (an @FILE list)
  queue.py show BATCH            the batch's rows, readable
  queue.py gate BASE             writer gate: every hook added since git revision BASE is unverified, has
                                 a batch, route and hazards, and its batch is small and of one subsystem;
                                 prints `L1 0xADDR` for the new L1-pure hooks (writer-gate.sh runs them)

Sources: the `#[hook(...)]` attributes in crates/*/src (tools/verify/hookattrs.py), catalog/procedures.csv
(subsystem), catalog/verify-status.csv (hand/tool maintained status), the per-recording coverage files
catalog/raw/coverage-KEY.csv of the data checkout (SA_ROOT or the main checkout; INF-10) and git (the
commit that introduced each batch). A hook leaves the queue when its `unverified` attribute is removed
(promotion); its status rows are then dropped from verify-status.csv.
"""
import csv
import io
import os
import pathlib
import re
import subprocess
import sys

HERE = pathlib.Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
import hookattrs  # noqa: E402

QUEUE = ROOT / "catalog/verify-queue.csv"
STATUS = ROOT / "catalog/verify-status.csv"
OVERVIEW = ROOT / "docs/verify/QUEUE.md"
STATUSES = ("written", "L1", "L2", "L3-batch", "failed")
BATCH_MAX = 60
COLUMNS = ["address", "name", "subsystem", "batch", "crate", "file", "line", "writer_commit", "writer_branch",
           "status", "status_evidence", "route", "hazards", "recordings", "command"]
GROVE = "grove-walk"


def git(*args, cwd=ROOT):
    r = subprocess.run(["git", *args], cwd=cwd, capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ""


def subsystems():
    out = {}
    p = ROOT / "catalog/procedures.csv"
    if p.exists():
        for r in csv.DictReader(p.open()):
            out[int(r["address"], 16)] = r["subsystem"]
    return out


def read_status():
    rows = {}
    if STATUS.exists():
        for r in csv.DictReader(STATUS.open()):
            rows[int(r["address"], 16)] = r
    return rows


def write_status(rows):
    buf = io.StringIO()
    w = csv.DictWriter(buf, ["address", "batch", "status", "date", "evidence"], lineterminator="\n")
    w.writeheader()
    for a in sorted(rows):
        w.writerow(rows[a])
    STATUS.write_text(buf.getvalue())


def coverage(data):
    """address -> recordings (INF-10 per-recording coverage) that call the function."""
    out = {}
    for p in sorted((data / "catalog/raw").glob("coverage-*.csv")):
        key = p.name[len("coverage-"):-len(".csv")]
        if key.endswith(".hits") or key == "all" or key.startswith("rust-"):
            continue
        try:
            for r in csv.DictReader(p.open()):
                if int(r.get("hits") or 0) > 0:
                    out.setdefault(int(r["address"], 16), []).append(key)
        except (KeyError, ValueError):
            continue
    return out


def rec_path(name):
    if name == GROVE:
        return "$SA_ROOT/target/e2e/lane-2/inf4/rec.sarec"
    return f"$SA_ROOT/target/e2e/corpus/{name}.sarec"


def l1_specs():
    """Addresses with an L1 spec (`sa-oracle list` of the release build, when present)."""
    for tgt in (os.environ.get("CARGO_TARGET_DIR"), str(ROOT / "target"), str(hookattrs.data_root(ROOT) / "target")):
        if not tgt:
            continue
        exe = pathlib.Path(tgt) / "release/sa-oracle"
        if exe.exists():
            r = subprocess.run([str(exe), "list"], capture_output=True, text=True)
            if r.returncode == 0:
                return {int(m.group(1), 16) for m in re.finditer(r"\s(0x[0-9a-fA-F]+)\s", r.stdout)}
    return None


def command(h, recs, specs):
    """The exact verification command line of one hook (run from the repository root)."""
    a = f"{h['addr']:#x}"
    b = h["unverified"]
    route = h.get("route", "")
    haz = set(x.strip() for x in h.get("hazards", "").split(",")) - {"none", ""}
    rec = rec_path(recs[0] if recs else GROVE)
    env = f"SA_UNVERIFIED={b} LANE=$LANE"
    if route == "L1-pure":
        if specs is not None and h["addr"] not in specs:
            return f"(needs an L1 spec in crates/sa-oracle/src/specs_*.rs, then) target/release/sa-oracle l1 {a} --backend native --cases 1000000"
        return f"target/release/sa-oracle l1 {a} --backend native --cases 1000000"
    if route == "L3-only":
        return f"{env} tools/e2e/l3.sh --rec {rec} --rust {a} --fixed verified --hookoff all"
    pre = []
    if "uninit-locals" in haz or "stack-residue" in haz:
        pre.append("SPLIT_STACKFILL=4096")
    if route == "split":
        runs = "plain split rust"
    else:
        runs = "plain l2 rust"
    extra = f" --window-all {a}" if "heap-writes" in haz else ""
    return " ".join(pre + [env, f"tools/e2e/port-l2.sh {a} {rec} --runs \"{runs}\" --tag {b}-{a}{extra}"])


def batch_commits(batches):
    """batch -> (commit, branch): the oldest commit whose diff adds `unverified = "BATCH"`."""
    out = {}
    head = git("rev-parse", "--abbrev-ref", "HEAD").strip() or "?"
    for b in batches:
        c = git("log", "--format=%h", "--reverse", "-G", f'unverified *= *"{re.escape(b)}"', "--", "crates").split()
        if c:
            br = [x.strip() for x in git("branch", "--contains", c[0], "--format=%(refname:short)").splitlines()]
            branch = "master" if "master" in br else (br[0] if br else "?")
            out[b] = (c[0], branch)
        else:
            out[b] = ("uncommitted", head)
    return out


def build_rows():
    hs = [h for h in hookattrs.hooks(ROOT) if h.get("unverified")]
    sub = subsystems()
    st = read_status()
    cov = coverage(hookattrs.data_root(ROOT))
    specs = l1_specs() if any(h.get("route") == "L1-pure" for h in hs) else None
    commits = batch_commits(sorted({h["unverified"] for h in hs}))
    rows = []
    for h in sorted(hs, key=lambda h: (h["unverified"], h["addr"])):
        a = h["addr"]
        writer_recs = [x.strip() for x in h.get("recs", "").split(",") if x.strip()]
        recs = writer_recs + [r for r in cov.get(a, []) if r not in writer_recs]
        s = st.get(a, {})
        c, br = commits[h["unverified"]]
        rows.append({
            "address": f"0x{a:X}", "name": h["name"], "subsystem": sub.get(a, "?"), "batch": h["unverified"],
            "crate": h["file"].split("/")[1], "file": h["file"], "line": h["line"],
            "writer_commit": c, "writer_branch": br,
            "status": s.get("status", "written"), "status_evidence": s.get("evidence", ""),
            "route": h.get("route", ""), "hazards": h.get("hazards", ""),
            "recordings": " ".join(recs) if recs else "(no coverage data: start with grove-walk)",
            "command": command(h, recs, specs),
        })
    return rows, st


def render(rows):
    buf = io.StringIO()
    w = csv.DictWriter(buf, COLUMNS, lineterminator="\n")
    w.writeheader()
    w.writerows(rows)
    q = buf.getvalue()
    batches = {}
    for r in rows:
        batches.setdefault(r["batch"], []).append(r)
    md = ["# Verification queue (generated by tools/verify/queue.py; do not edit)", "",
          "Unverified ports (`#[hook(unverified = \"BATCH\")]`), one section per batch. Procedure and tools:",
          "[HOWTO.md](HOWTO.md). Per-hook rows with the exact commands: `catalog/verify-queue.csv`.", ""]
    if not rows:
        md += ["The queue is empty: every registered port is verified.", ""]
    md += ["| batch | hooks | subsystems | commit | branch | written | L1 | L2 | L3-batch | failed | routes |",
           "|---|---|---|---|---|---|---|---|---|---|---|"]
    for b, rs in sorted(batches.items()):
        n = {s: sum(1 for r in rs if r["status"] == s) for s in STATUSES}
        subs = sorted({r["subsystem"] for r in rs})
        routes = {}
        for r in rs:
            routes[r["route"]] = routes.get(r["route"], 0) + 1
        md.append(f"| {b} | {len(rs)} | {', '.join(subs)} | {rs[0]['writer_commit']} | {rs[0]['writer_branch']} | "
                  + " | ".join(str(n[s]) for s in STATUSES) + " | "
                  + ", ".join(f"{k} {v}" for k, v in sorted(routes.items())) + " |")
    for b, rs in sorted(batches.items()):
        haz = {}
        for r in rs:
            for x in r["hazards"].split(","):
                x = x.strip()
                if x and x != "none":
                    haz[x] = haz.get(x, 0) + 1
        recs = {}
        for r in rs:
            if r["recordings"].startswith("("):
                continue
            for x in r["recordings"].split():
                recs[x] = recs.get(x, 0) + 1
        best = max(recs, key=lambda k: (recs[k], k == GROVE)) if recs else GROVE
        md += ["", f"## {b}", "",
               f"{len(rs)} hooks in {', '.join(sorted({r['crate'] for r in rs}))}; hazards: "
               + (", ".join(f"{k} ({v})" for k, v in sorted(haz.items())) or "none")
               + "; recordings calling them: "
               + (", ".join(f"{k} ({v})" for k, v in sorted(recs.items())) or "no coverage data") + ".", "",
               "```sh",
               f"# whole batch on top of the verified set, bisected to the culprit hook on divergence",
               f"tools/verify/bisect-batch.sh {b} {rec_path(best)} $LANE",
               f"# L2 of the whole batch (read/write sets; add --runs \"plain split rust\" for split-route hooks)",
               f"SA_UNVERIFIED={b} LANE=$LANE tools/e2e/port-l2.sh batch:{b} {rec_path(best)} --runs \"plain l2 rust\" --tag {b}",
               "```", "",
               "| address | name | status | route | hazards |", "|---|---|---|---|---|"]
        for r in rs:
            md.append(f"| {r['address']} | `{r['name']}` | {r['status']} | {r['route']} | {r['hazards']} |")
    return q, "\n".join(md) + "\n"


def gen(check=False):
    rows, st = build_rows()
    q, md = render(rows)
    live = {int(r["address"], 16) for r in rows}
    dropped = [a for a in st if a not in live]
    if check:
        stale = []
        if not QUEUE.exists() or QUEUE.read_text() != q:
            stale.append(str(QUEUE.relative_to(ROOT)))
        if not OVERVIEW.exists() or OVERVIEW.read_text() != md:
            stale.append(str(OVERVIEW.relative_to(ROOT)))
        if dropped:
            stale.append(f"{STATUS.relative_to(ROOT)} (rows of promoted hooks)")
        if stale:
            print("queue: out of date: " + ", ".join(stale) + " (run tools/verify/queue.py)")
            return 1
        print(f"queue: up to date ({len(rows)} unverified hooks)")
        return 0
    if dropped:
        for a in dropped:
            print(f"queue: 0x{a:X} is no longer unverified (promoted): status row dropped")
            del st[a]
        write_status(st)
    OVERVIEW.parent.mkdir(parents=True, exist_ok=True)
    QUEUE.write_text(q)
    OVERVIEW.write_text(md)
    print(f"queue: {len(rows)} unverified hooks in {len({r['batch'] for r in rows})} batches -> "
          f"{QUEUE.relative_to(ROOT)}, {OVERVIEW.relative_to(ROOT)}")
    return 0


def set_status(key, status, evidence):
    if status not in STATUSES:
        sys.exit(f"queue: status must be one of {', '.join(STATUSES)}")
    hs = [h for h in hookattrs.hooks(ROOT) if h.get("unverified")]
    if key.startswith("batch:"):
        sel = [h for h in hs if h["unverified"] == key[len("batch:"):]]
    else:
        sel = [h for h in hs if h["addr"] == int(key, 16) or h["name"] == key]
    if not sel:
        sys.exit(f"queue: {key}: no unverified hook")
    import datetime
    st = read_status()
    for h in sel:
        st[h["addr"]] = {"address": f"0x{h['addr']:X}", "batch": h["unverified"], "status": status,
                         "date": datetime.date.today().isoformat(), "evidence": evidence}
    write_status(st)
    print(f"queue: {len(sel)} hooks -> {status}")
    return gen()


def base_hooks(base):
    """{addr: hook} of the registrations at git revision BASE."""
    files = [f for f in git("ls-tree", "-r", "--name-only", base, "--", "crates").splitlines()
             if f.endswith(".rs") and "/src/" in f and f.split("/")[1] not in hookattrs.SKIP_CRATES]
    out = {}
    if not files:
        return out
    r = subprocess.run(["git", "cat-file", "--batch"], cwd=ROOT,
                       input="".join(f"{base}:{f}\n" for f in files).encode(), capture_output=True)
    data, pos = r.stdout, 0
    for f in files:
        nl = data.index(b"\n", pos)
        hdr = data[pos:nl].split()
        if len(hdr) != 3:  # "<name> missing"
            pos = nl + 1
            continue
        size = int(hdr[2])
        text = data[nl + 1:nl + 1 + size].decode("utf-8", "replace")
        pos = nl + 1 + size + 1
        if "#[hook(" in text:
            for h in hookattrs.parse_source(text, f):
                out[h["addr"]] = h
    return out


def gate(base):
    if not git("rev-parse", "--verify", base).strip():
        sys.exit(f"queue gate: unknown revision {base}")
    now = hookattrs.hooks(ROOT)
    before = base_hooks(base)
    sub = subsystems()
    fails = []
    new = [h for h in now if h["addr"] not in before]
    for h in new:
        where = f"{h['file']}:{h['line']} {h['name']} ({h['addr']:#x})"
        if not h.get("unverified"):
            fails.append(f"{where}: a new hook must be `unverified = \"BATCH\"` (verifiers promote it)")
        elif not h.get("route") or not h.get("hazards"):
            fails.append(f"{where}: unverified hook without route/hazards hints")
    # a verified hook may lose `unverified` only through a verifier (promotion), never gain new code
    # silently: report hooks that changed from unverified to verified so the reviewer sees them
    promoted = [h for h in now if h["addr"] in before and before[h["addr"]].get("unverified") and not h.get("unverified")]
    batches = {}
    for h in now:
        if h.get("unverified"):
            batches.setdefault(h["unverified"], []).append(h)
    touched = {h["unverified"] for h in new if h.get("unverified")}
    for b in sorted(touched):
        hs = batches[b]
        if len(hs) > BATCH_MAX:
            fails.append(f"batch {b}: {len(hs)} hooks (at most {BATCH_MAX}: split it)")
        subs = sorted({sub.get(h["addr"], "?") for h in hs})
        if len(subs) > 1:
            print(f"note: batch {b} spans subsystems {', '.join(subs)} (one subsystem per batch is preferred)")
        old = [h for h in before.values() if h.get("unverified") == b]
        if old:
            print(f"note: batch {b} already existed at {base} ({len(old)} hooks); new hooks join it")
    print(f"gate: {len(new)} new hooks since {base}, {sum(1 for h in new if h.get('unverified'))} unverified, "
          f"in batches {', '.join(sorted(touched)) or '-'}; {len(promoted)} promoted")
    for h in promoted:
        print(f"promoted: {h['addr']:#x} {h['name']} (batch {before[h['addr']]['unverified']})")
    for h in new:
        if h.get("unverified") and h.get("route") == "L1-pure":
            print(f"L1 {h['addr']:#x} {h['name']}")
    for f in fails:
        print(f"FAIL {f}")
    return 1 if fails else 0


def main(argv):
    if not argv or argv[0] == "gen":
        return gen()
    if argv[0] == "--check":
        return gen(check=True)
    if argv[0] == "set" and len(argv) >= 3:
        return set_status(argv[1], argv[2], " ".join(argv[3:]))
    if argv[0] in ("list", "show") and len(argv) == 2:
        hs = sorted((h for h in hookattrs.hooks(ROOT) if h.get("unverified") == argv[1]), key=lambda h: h["addr"])
        if not hs:
            sys.exit(f"queue: no hook in batch {argv[1]}")
        for h in hs:
            print(f"{h['addr']:#x}" if argv[0] == "list" else
                  f"{h['addr']:#x}\t{h['name']}\t{h.get('route')}\t{h.get('hazards')}\t{h['file']}:{h['line']}")
        return 0
    if argv[0] == "gate" and len(argv) == 2:
        return gate(argv[1])
    print(__doc__, file=sys.stderr)
    return 2


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
