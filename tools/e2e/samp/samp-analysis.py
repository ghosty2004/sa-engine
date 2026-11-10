#!/usr/bin/env python3
"""SAMP-1D: what gta_sa.exe executes under SA:MP, against the static closure and single-player coverage.

    samp-analysis.py --points P --excluded X --hits H1,H2,... --out-dir DIR [--catalog CAT_DIR]
                     [--required docs/samp/required-functions.csv] [--aggregates docs/samp/aggregates.json]
                     [--sp catalog/raw/coverage-all.csv] [--state catalog/state.csv] [--scm SCM.csv,...]
                     [--priority catalog/samp-priority.csv]

Function executed = its entry point was hit; for a function whose entry was not armed (samp patch
range, see samp-points.py), any hit block counts. Hits files are merged (sum). Writes:
* DIR/samp-functions.csv: every live catalogue function with samp/E/SP flags;
* DIR/report.md: the per-subsystem tables, zero-hit checks of samp's disables, SCM opcodes;
* --priority: functions executed under SA:MP whose state is below L1 (unknown / specified /
  implemented): address,name,subsystem,state,hits,blocks_hit,blocks_total,in_E,in_sp.
"""
import argparse
import collections
import csv
import json
import os
import re

VERIFIED = {"L1", "L2", "L3"}


def load_points(path):
    pts = {}
    for line in open(path):
        s = line.split("#")[0].split()
        if len(s) >= 4:
            pts[int(s[0], 16)] = (s[2], int(s[3], 16))
    return pts


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--points", required=True)
    ap.add_argument("--excluded", required=True)
    ap.add_argument("--hits", required=True)
    ap.add_argument("--out-dir", required=True)
    ap.add_argument("--catalog", default="catalog")
    ap.add_argument("--required", default="docs/samp/required-functions.csv")
    ap.add_argument("--aggregates", default="docs/samp/aggregates.json")
    ap.add_argument("--sp", default="catalog/raw/coverage-all.csv")
    ap.add_argument("--state", default="catalog/state.csv")
    ap.add_argument("--scm", default="")
    ap.add_argument("--priority", default="")
    a = ap.parse_args()
    os.makedirs(a.out_dir, exist_ok=True)

    pts = load_points(a.points)
    excl = {int(r["address"], 16): r for r in csv.DictReader(open(a.excluded))}
    hits = collections.Counter()
    for h in a.hits.split(","):
        if h:
            for r in csv.DictReader(open(h)):
                hits[int(r["address"], 16)] += int(r["hits"])

    procs = {int(r["address"], 16): r for r in csv.DictReader(open(f"{a.catalog}/procedures.csv"))}
    state = {int(r["address"], 16): r["state"] for r in csv.DictReader(open(a.state))}
    req = {int(r["address"], 16): r for r in csv.DictReader(open(a.required))}
    sp = {}
    for r in csv.DictReader(open(a.sp)):
        sp[int(r["address"], 16)] = int(r["hits"]) > 0
    agg = json.load(open(a.aggregates))

    # Per function: entry armed/hit, blocks.
    fn = collections.defaultdict(lambda: {"entry": None, "bh": 0, "bt": 0})
    for addr, (kind, f) in pts.items():
        d = fn[f]
        d["bt"] += 1
        h = hits.get(addr, 0)
        d["bh"] += h > 0
        if kind == "F" and addr == f:
            d["entry"] = h
    for addr, r in excl.items():
        f = int(r["function"], 16) if r["function"] else None
        if f is not None:
            fn[f]  # make sure the function is known

    def name(f):
        p = procs.get(f, {})
        return p.get("name") or p.get("gr_name") or p.get("ghidra_name") or ""

    rows = []
    for f, p in procs.items():
        live = p["class"] in ("game", "") and not p["subsystem"].startswith("X-")
        if f not in req and not live:
            continue
        d = fn.get(f, {"entry": None, "bh": 0, "bt": 0})
        measured = d["bt"] > 0
        if d["entry"] is not None:
            ex = d["entry"] > 0
        else:
            ex = d["bh"] > 0
        rq = req.get(f)
        rows.append({
            "address": f"{f:#x}",
            "name": name(f),
            "subsystem": p["subsystem"],
            "role": p.get("role", ""),
            "state": state.get(f, p.get("state", "")),
            "measured": int(measured),
            "entry_armed": int(d["entry"] is not None),
            "samp": int(ex),
            "hits": d["entry"] if d["entry"] is not None else d["bh"],
            "blocks_hit": d["bh"],
            "blocks_total": d["bt"],
            "in_E": int(rq["in_E"]) if rq else 0,
            "in_B": int(rq["in_B"]) if rq else 0,
            "direct": int(rq["direct"]) if rq else 0,
            "cut_by_samp": int(rq["cut_by_samp"]) if rq else 0,
            "sp": int(sp.get(f, False)),
        })
    rows.sort(key=lambda r: int(r["address"], 16))
    with open(f"{a.out_dir}/samp-functions.csv", "w", newline="") as fo:
        w = csv.DictWriter(fo, fieldnames=list(rows[0].keys()))
        w.writeheader()
        w.writerows(rows)

    # Only functions of the SAMP-1 universe (required-functions.csv = live catalogue).
    U = [r for r in rows if int(r["address"], 16) in req]
    subs = sorted({r["subsystem"] for r in U})
    T = collections.defaultdict(collections.Counter)
    for r in U:
        for k in (r["subsystem"], "total"):
            t = T[k]
            t["live"] += 1
            t["measured"] += r["measured"]
            t["E"] += r["in_E"]
            t["samp"] += r["samp"]
            t["samp_E"] += r["samp"] and r["in_E"]
            t["samp_notE"] += r["samp"] and not r["in_E"]
            t["E_not_samp"] += r["in_E"] and not r["samp"]
            t["sp"] += r["sp"]
            t["samp_sp"] += r["samp"] and r["sp"]
            t["samp_not_sp"] += r["samp"] and not r["sp"]
            t["sp_not_samp"] += r["sp"] and not r["samp"]
            t["union"] += r["samp"] or r["sp"]
            t["prio"] += r["samp"] and r["state"] not in VERIFIED
            t["verified"] += r["samp"] and r["state"] in VERIFIED
            t["bh"] += r["blocks_hit"]
            t["bt"] += r["blocks_total"]
    order = sorted(subs, key=lambda s: -T[s]["samp"]) + ["total"]
    out = []
    out.append("### Functions executed under SA:MP per subsystem\n")
    out.append("live = functions of `docs/samp/required-functions.csv` (live catalogue); samp = executed under "
               "SA:MP; E = static closure E; SP = single-player corpus coverage (`catalog/raw/coverage-all.csv`).\n")
    out.append("| subsystem | live | E | samp | samp ∩ E | samp ∖ E | E ∖ samp | SP | samp ∖ SP | SP ∖ samp | samp ∪ SP | blocks hit |")
    out.append("|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|")
    for s in order:
        t = T[s]
        b = f"**{s}**" if s == "total" else s
        out.append(f"| {b} | {t['live']} | {t['E']} | {t['samp']} | {t['samp_E']} | {t['samp_notE']} | {t['E_not_samp']} "
                   f"| {t['sp']} | {t['samp_not_sp']} | {t['sp_not_samp']} | {t['union']} | {t['bh']} / {t['bt']} |")
    out.append("")
    out.append("### Executed under SA:MP but outside E (closure misses), by kind\n")
    out.append("init/dtor = CRT static initialisers and destructors (`role` init/dtor in procedures.csv): the CRT "
               "calls them through its `_initterm` / atexit tables, which the static closure does not follow.\n")
    out.append("| subsystem | samp ∖ E | init/dtor | other |")
    out.append("|---|---:|---:|---:|")
    M = collections.defaultdict(collections.Counter)
    for r in U:
        if r["samp"] and not r["in_E"]:
            k = "initdtor" if r["role"] in ("init", "dtor") else "other"
            M[r["subsystem"]][k] += 1
            M["total"][k] += 1
    for s_ in sorted([x for x in M if x != "total"], key=lambda x: -sum(M[x].values())) + ["total"]:
        m = M[s_]
        b = f"**{s_}**" if s_ == "total" else s_
        out.append(f"| {b} | {m['initdtor'] + m['other']} | {m['initdtor']} | {m['other']} |")
    out.append("")
    out.append("### Porting priority (executed under SA:MP, state below L1) per subsystem\n")
    out.append("| subsystem | samp executed | L1/L2/L3 | priority (unknown/specified/implemented) | of which unknown | specified | implemented |")
    out.append("|---|---:|---:|---:|---:|---:|---:|")
    P = collections.defaultdict(collections.Counter)
    for r in U:
        if r["samp"] and r["state"] not in VERIFIED:
            for k in (r["subsystem"], "total"):
                P[k][r["state"]] += 1
    for s in sorted(subs, key=lambda s: -T[s]["prio"]) + ["total"]:
        t, p = T[s], P[s]
        b = f"**{s}**" if s == "total" else s
        out.append(f"| {b} | {t['samp']} | {t['verified']} | {t['prio']} | {p['unknown']} | {p['specified']} | {p['implemented']} |")
    out.append("")

    # Zero-hit checks of samp's disables.
    by_addr = {int(r["address"], 16): r for r in rows}
    out.append("### samp's disables: hits under SA:MP\n")
    out.append("Cut functions (entry patched to `ret` at init; the entry point itself is not armed, so the "
               "body blocks are the measure):\n")
    out.append("| function | blocks hit / armed | executed |")
    out.append("|---|---:|---|")
    for a_, desc in sorted(agg["cut_functions"].items()):
        r = by_addr.get(int(a_, 16))
        if r:
            out.append(f"| {a_} {desc.split(' (')[0]} | {r['blocks_hit']} / {r['blocks_total']} | {'YES' if r['samp'] else 'no'} |")
    out.append("")
    out.append("Removed call edges (NOPed call sites): the callee may still run through other callers.\n")
    out.append("| edge | callee executed | callee hits (entry) |")
    out.append("|---|---|---:|")
    for e in agg["removed_call_edges"]:
        m = re.search(r"-> (0x[0-9a-f]+)", e)
        r = by_addr.get(int(m.group(1), 16)) if m else None
        if r:
            out.append(f"| {e} | {'YES' if r['samp'] else 'no'} | {r['hits']} |")
    out.append("")

    # SCM opcodes.
    if a.scm:
        ops = collections.Counter()
        for f in a.scm.split(","):
            if f and os.path.exists(f):
                for r in csv.DictReader(open(f)):
                    ops[r["opcode"]] += int(r["count"])
        used = {o.split()[0].upper() for o in agg.get("scm_opcodes_used", [])}
        seen = set(ops)
        out.append("### SCM opcodes executed under SA:MP\n")
        out.append(f"{len(seen)} distinct opcodes, {sum(ops.values())} commands. samp-rs static list (SAMP-1 §4.4): "
                   f"{len(used)}; executed and in the list {len(seen & used)}; executed, not in the list "
                   f"{len(seen - used)}; in the list, not executed {len(used - seen)}.\n")
        out.append("Executed (opcode:count): " + " ".join(f"{o}:{ops[o]}" for o in sorted(seen)) + "\n")
        out.append("Executed, not in the samp-rs list: " + " ".join(sorted(seen - used)) + "\n")
        out.append("In the samp-rs list, not executed: " + " ".join(sorted(used - seen)) + "\n")
        with open(f"{a.out_dir}/scm-opcodes.csv", "w") as fo:
            fo.write("opcode,count,in_samp_rs_list\n")
            for o in sorted(seen | used):
                fo.write(f"{o},{ops.get(o, 0)},{int(o in used)}\n")

    with open(f"{a.out_dir}/report.md", "w") as fo:
        fo.write("\n".join(out) + "\n")
    print("\n".join(out))

    if a.priority:
        pr = [r for r in U if r["samp"] and r["state"] not in VERIFIED]
        with open(a.priority, "w", newline="") as fo:
            w = csv.writer(fo)
            w.writerow(["address", "name", "subsystem", "state", "hits", "blocks_hit", "blocks_total", "in_E", "in_sp",
                        "role"])
            for r in pr:
                w.writerow([r["address"], r["name"], r["subsystem"], r["state"], r["hits"], r["blocks_hit"],
                            r["blocks_total"], r["in_E"], r["sp"], r["role"]])


if __name__ == "__main__":
    main()
