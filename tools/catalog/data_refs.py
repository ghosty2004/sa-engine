#!/usr/bin/env python3
"""CAT-12b: code that the game reaches only through data.

Usage: data_refs.py <catalog-dir> [gta_sa.exe] [gta-reversed]

Scans every aligned dword of the data sections (.rdata, .data, _TEXT_HA, _rwdseg) and every
immediate operand of a catalogued body (catalog/raw/cat7-code-immediates.csv, written by
`callgraph.py build`) for values that point into game code (0x401000-0x858000) and are not
catalogue entries. A value is accepted as a function entry only when

  (a) it is a plausible entry (`entry_check`): it is not inside any Ghidra function body
      (catalog/raw/ghidra/functions.jsonl), it is 16-byte aligned and preceded by padding
      (0x90 / 0xCC) or by a `ret`, and a linear decode from it reaches ret / jmp without an
      invalid, privileged or padding instruction; and
  (b) it has context: a data slot that is part of a pointer table (a run of at least two
      function pointers, catalogue entries or plausible entries, zero slots allowed inside),
      or a single slot that code reads as a variable; or an immediate in a catalogued body.

Accepted entries are appended to catalog/data-code-addresses.csv (a tracked input that
merge_ghidra.py reads, rule `data`). Exit status 3 when rows were added (rerun the pipeline from
build_catalog), 0 when every accepted entry is already in the catalogue.

Generated reports (deterministic, sorted):
  catalog/data-tables.csv   one row per pointer table: start, end, slots, kind, code refs
  catalog/data-refs.csv     one row per non-catalogue game-code value found in data / immediates
                            with its verdict (accepted / rejected + reason)
Method and results: docs/re/cat12b-data-tables.md. Run with .venv/bin/python -I.
"""
import bisect
import collections
import csv
import json
import re
import struct
import sys
from pathlib import Path

import capstone
import pefile
from capstone import x86_const as X

sys.path.insert(0, str(Path(__file__).resolve().parent))
from vtables import VTables  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
BASE = 0x400000
GAME_CODE = (0x401000, 0x858000)
SCAN = [('.rdata', 0x858000, 0x8A4000), ('.data', 0x8A4000, 0xC9E000), ('_TEXT_HA', 0xC9E000, 0xCAF000),
        ('_rwdseg', 0xCAF000, 0xCB0000)]
# counted, not used: SecuROM's data and switch tables inside .text
COUNT_ONLY = [('.data2', 0x1301000, 0x1556000), ('.text', 0x401000, 0x857000)]
MAX_ZERO_RUN = 16
PURECALL = 0x82263A  # MSVC _purecall (excluded:crt)
PRIV = {'in', 'out', 'ins', 'outs', 'insb', 'insd', 'outsb', 'outsd', 'hlt', 'cli', 'sti', 'lgdt', 'lidt',
        'iretd', 'arpl', 'les', 'lds', 'into', 'bound', 'retf', 'int1', 'sysenter', 'icebp', 'salc',
        'aas', 'aaa', 'das', 'daa', 'aam', 'aad'}
# MSVC CRT _cinit (0x823b76) walks __xi_a..__xi_z with `mov ecx, imm; mov edi, imm` and
# __xc_a..__xc_z with `mov esi, imm; ... mov edi, imm`. Checked on the bytes below.
CINIT = [(0x823B83, b'\xb9', 0x823B88, b'\xbf', 'crt-init:__xi'), (0x823BB6, b'\xbe', 0x823BBD, b'\xbf', 'crt-init:__xc')]
# structures known from their accessor; (base, size, kind, accessor address, accessor bytes, evidence)
STRUCTS = [(0x8E2498, 0x30, 'rw-device', 0x7F9C20, bytes.fromhex('b898248e00c3'),
            'RwDevice (gammaCorrection 1.0, fpSystem, zBufferNear/Far, fpRenderStateSet/Get, Im2D/Im3D '
            'render functions) returned by 0x7f9c20 `mov eax, 0x8e2498; ret`')]
NAMED = {0x8A6168: ('script-range-fns', 'CRunningScript::ProcessOneCommand (0x469eb0) '
                    '`call [opcode/100*4 + 0x8a6168]` (CAT-10)')}


def hx(v):
    return f'{v:#x}'


def write_csv(path, header, rows):
    with open(path, 'w', newline='') as f:
        w = csv.writer(f, lineterminator='\n')
        w.writerow(header)
        w.writerows(rows)


class Img:
    def __init__(self, exe):
        self.img = pefile.PE(str(exe), fast_load=True).get_memory_mapped_image()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True

    def rd(self, va, n):
        return bytes(self.img[va - BASE:va - BASE + n])

    def u32(self, va):
        return struct.unpack_from('<I', self.img, va - BASE)[0]


def after_switch_table(im, v):
    """True when the bytes right before v end an MSVC switch table of the preceding function:
    a jump table (dwords pointing into game code just below v), optionally followed by its byte
    index table (bytes < 0x80) and padding."""
    p = v
    while p > v - 0x400 and im.rd(p - 1, 1)[0] in (0x90, 0xCC):
        p -= 1
    q = p
    while q > p - 0x400:
        d = im.u32(q - 4)
        if v - 0x10000 <= d < v and d >= GAME_CODE[0]:
            break
        if im.rd(q - 1, 1)[0] >= 0x80:
            return False
        q -= 1
    else:
        return False
    d1, d2 = im.u32(q - 4), im.u32(q - 8)
    return all(v - 0x10000 <= d < v for d in (d1, d2))


def entry_check(im, v):
    """'' when v looks like a function entry on the bytes, else the reason it does not."""
    if v % 16:
        return 'not 16-byte aligned'
    prev = im.rd(v - 3, 3)
    if not (prev[2] in (0x90, 0xCC, 0xC3) or prev[0] == 0xC2 or after_switch_table(im, v)):
        return f'not preceded by padding, ret or a switch table (byte {prev[2]:#04x})'
    code = im.rd(v, 0x1000)
    if code[:2] == b'\x00\x00' or code[0] in (0x90, 0xCC):
        return 'starts with padding / zero bytes'
    off = 0
    for n, ins in enumerate(im.md.disasm(code, v)):
        if ins.address != v + off:
            break
        off += ins.size
        if ins.mnemonic in PRIV or ins.mnemonic == 'int3':
            return f'decodes {ins.mnemonic} at {ins.address:#x}'
        if ins.mnemonic in ('ret', 'jmp'):
            return ''
        if n > 400:
            break
    return 'linear decode does not reach ret/jmp'


def load_bodies(cat):
    spans = []
    for line in open(cat / 'raw' / 'ghidra' / 'functions.jsonl'):
        j = json.loads(line)
        e = int(j['entry'], 16)
        for lo, hi in j['body']:
            spans.append((int(lo, 16), int(hi, 16) + 1, e))
    spans.sort()
    return spans


class Bodies:
    def __init__(self, spans):
        self.spans = spans
        self.of = collections.defaultdict(list)
        for lo, hi, e in spans:
            self.of[e].append((lo, hi))
        self.los = [s[0] for s in spans]
        # running max of span ends lets the lookup stop early
        self.maxhi = []
        m = 0
        for s in spans:
            m = max(m, s[1])
            self.maxhi.append(m)

    def owner(self, v):
        i = bisect.bisect_right(self.los, v) - 1
        while i >= 0 and self.maxhi[i] > v:
            lo, hi, e = self.spans[i]
            if lo <= v < hi and e != v:
                return e, lo
            i -= 1
        return None, None


def main():
    cat = Path(sys.argv[1])
    exe = sys.argv[2] if len(sys.argv) > 2 else ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
    gr = sys.argv[3] if len(sys.argv) > 3 else ROOT / 're' / 'gta-reversed'
    im = Img(exe)
    procs = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'procedures.csv'))}
    cls = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'classification.csv'))}
    bodies = Bodies(load_bodies(cat))
    vts = VTables(exe, gr)
    grvt = {vt: (c, n) for c, (vt, n) in vts.vt.items()}

    # code references to data addresses (CAT-7 rwsets: direct / indexed / based / imm)
    dref = collections.defaultdict(set)
    gname = {}
    for r in csv.DictReader(open(cat / 'rwsets.csv')):
        a = int(r['address'], 16)
        dref[a].add((int(r['function'], 16), r['access'], r['mode']))
        if r['name']:
            gname[a] = r['name']
    ext = {}
    for r in csv.DictReader(open(cat / 'extents.csv')):
        ext[int(r['address'], 16)] = [tuple(int(x, 16) for x in rg.split('-')) for rg in r['ranges'].split(';')]

    path = cat / 'data-code-addresses.csv'
    have = {}
    if path.exists():
        have = {int(r['address'], 16): r for r in csv.DictReader(open(path))}
    memo = {}
    absorbed = {}  # plausible entries Ghidra folded into another body -> that body's entry

    def check(v):
        if v not in memo:
            o, span_lo = bodies.owner(v)
            if o is None:
                memo[v] = entry_check(im, v)
            elif v % 16 == 0 and (span_lo == v or im.rd(v - 1, 1)[0] in (0x90, 0xCC)) and \
                    entry_check(im, v) == '' and not branches_to(o, v):
                # Ghidra let o's body run into v, but only by falling through padding (o ends in
                # a call it does not know is non-returning) or as a separate body chunk that
                # starts at v (a computed tail jump); no branch of o targets v
                memo[v] = ''
                absorbed[v] = o
            else:
                memo[v] = f'inside the body of {o:#x}'
        return memo[v]

    def branches_to(o, v):
        for lo, hi in bodies.of[o]:
            for ins in im.md.disasm(im.rd(lo, hi - lo), lo):
                if ins.mnemonic.startswith('j') or ins.mnemonic in ('call', 'loop'):
                    op = ins.operands[0] if ins.operands else None
                    if op is not None and op.type == X.X86_OP_IMM and op.imm & 0xFFFFFFFF == v:
                        return True
        return False

    def is_ptr(v):
        """Code pointer that counts as a table member: catalogue entry or plausible entry."""
        if v in procs:
            return True
        return GAME_CODE[0] <= v < GAME_CODE[1] and (check(v) == '' or is_weak(v))

    def is_weak(v):
        """Plausible entry except that the bytes before it are neither padding, ret nor a switch
        table (e.g. SecuROM-displaced code ends the previous function). Only accepted as a member
        of a table whose other members are catalogue entries or full plausible entries."""
        return v not in procs and check(v).startswith('not preceded')

    # CRT init tables
    crt = {}
    for a1, op1, a2, op2, kind in CINIT:
        assert im.rd(a1, 1) == op1 and im.rd(a2, 1) == op2, f'_cinit bytes changed at {a1:#x}'
        crt[(im.u32(a1 + 1), im.u32(a2 + 1))] = kind

    # ---- RenderWare pipeline node definitions (RxNodeDefinition: name, then 7 RxNodeMethods)
    def cstr(a):
        if not any(lo <= a < hi for _n, lo, hi in SCAN):
            return None
        b = im.rd(a, 128).split(b'\0')[0]
        return b.decode() if b and all(32 <= c < 127 for c in b) else None

    nodedefs, claimed = [], set()
    for _sec, lo, hi in SCAN:
        for s in range(lo, hi, 4):
            name = cstr(im.u32(s))
            if not (name and name.endswith('.csl')):
                continue
            meth = [(x, im.u32(x)) for x in range(s + 4, s + 32, 4)]
            if all(v == 0 or (v in procs or GAME_CODE[0] <= v < GAME_CODE[1] and check(v) == '') for _x, v in meth):
                mem = [(x, v) for x, v in meth if v]
                if mem:
                    nodedefs.append((mem, 'rw-node-def', f'RxNodeDefinition {name!r} at {s:#x}'))
                    claimed.update(x for x, _v in meth)

    for base, size, kind, acc, accb, ev in STRUCTS:
        assert im.rd(acc, len(accb)) == accb, f'accessor bytes changed at {acc:#x}'
        slots = [(x, im.u32(x)) for x in range(base, base + size, 4)]
        mem = [(x, v) for x, v in slots if GAME_CODE[0] <= v < GAME_CODE[1]]
        assert all(is_ptr(v) for _x, v in mem), f'{kind}: a code-range value is not an entry'
        nodedefs.append((mem, kind, ev))
        claimed.update(x for x, _v in slots)

    # ---- pointer tables in the data sections
    found = []   # (slot, value)
    for _sec, lo, hi in SCAN:
        for s in range(lo, hi, 4):
            v = im.u32(s)
            if v and s not in claimed and is_ptr(v):
                found.append((s, v))
    # a table starts at every address code names (vtables are adjacent in .rdata, each named by
    # its constructors), at every gta-reversed vtable and at the CRT init-table bounds
    split = set(dref) | set(grvt) | {vt + 4 * n for vt, (_c, n) in grvt.items()} | {x for b in crt for x in b}
    tables = []
    i = 0
    while i < len(found):
        j = i
        while j + 1 < len(found) and found[j + 1][0] - found[j][0] <= 4 * (MAX_ZERO_RUN + 1) and \
                not any(x in split for x in range(found[j][0] + 4, found[j + 1][0] + 4, 4)) and \
                all(im.u32(x) == 0 for x in range(found[j][0] + 4, found[j + 1][0], 4)):
            j += 1
        tables.append(found[i:j + 1])
        i = j + 1

    def code_refs(lo, hi):
        """CAT-7 code references to [lo, hi): {(function, access, mode, address)}."""
        out = set()
        for a in range(lo, hi, 4):
            for f, acc, mode in dref.get(a, ()):
                out.add((f, acc, mode, a))
        return out

    def vt_install(f, base):
        """True when catalogued body f stores `base` into [reg] (vtable pointer install)."""
        for lo, hi in ext.get(f, ()):
            code = im.rd(lo, hi - lo)
            pat = struct.pack('<I', base)
            k = code.find(pat)
            while k >= 0:
                for back in (2, 3, 6):  # c7 0x /r imm32 forms: [reg], [reg+d8], [reg+d32]
                    st = k - back
                    if st >= 0 and code[st] == 0xC7:
                        for ins in im.md.disasm(code[st:k + 4], lo + st):
                            if ins.size == back + 4 and ins.mnemonic == 'mov' and \
                                    ins.operands[0].type == X.X86_OP_MEM and \
                                    ins.operands[0].mem.base not in (0, X.X86_REG_ESP, X.X86_REG_EBP) and \
                                    ins.operands[0].mem.disp == 0 and ins.operands[1].imm & 0xFFFFFFFF == base:
                                return True
                            break
                k = code.find(pat, k + 1)
        return False

    def classify(t):
        lo, hi = t[0][0], t[-1][0] + 4
        nptr = len(t)
        refs = code_refs(lo, hi)
        base_refs = sorted({f for f, acc, mode, a in refs if a == lo})
        for (clo, chi), k in crt.items():
            if clo <= lo and hi <= chi:
                return k, f'between the _cinit bounds {clo:#x}..{chi:#x}'
        if lo in NAMED:
            return NAMED[lo]
        for s_, _v in t:
            if s_ in grvt:
                return f'vtable-gr:{grvt[s_][0]}', 'gta-reversed RH_ScopedVirtualClass vtable'
        if any(vt_install(f, lo) for f in base_refs):
            return 'vtable', 'base stored into [reg] by a referencing function'
        if nptr >= 2:
            # an abstract class's vtable (`_purecall` slot) or one that shares two or more slots,
            # at the same index, with a gta-reversed vtable (a class derived from it)
            if any(v == PURECALL for _s, v in t):
                return 'vtable', '_purecall slot'
            best = None
            for gvt, (gc, gn) in grvt.items():
                same = sum(1 for s_, v in t if (s_ - lo) // 4 < gn and v in procs and
                           im.u32(gvt + (s_ - lo)) == v)
                if same >= 2 and (best is None or same > best[0] or same == best[0] and gc < best[1]):
                    best = (same, gc)
            if best:
                return 'vtable', f'{best[0]} slots equal to the same slots of {best[1]}'
            return ('fnptr-table', 'base or slots named by code') if refs else ('fnptr-table-unreferenced', '')
        if any(a == lo and mode in ('direct', 'indexed') for f, acc, mode, a in refs):
            return 'fnptr-variable', 'slot read or written by code'
        return 'single', ''

    groups = [(t, *classify(t)) for t in tables]
    # arrays of structs holding one function pointer each: three or more 'single' slots at a
    # constant stride (8..64 bytes) with no other code pointer between them
    merged, i = [], 0
    while i < len(groups):
        t, kind, kev = groups[i]
        j = i
        if kind == 'single':
            d = None
            while j + 1 < len(groups) and groups[j + 1][1] == 'single':
                dd = groups[j + 1][0][0][0] - groups[j][0][0][0]
                if not (8 <= dd <= 64) or (d is not None and dd != d):
                    break
                d = dd
                j += 1
            if j - i + 1 >= 3:
                merged.append(([m for g in groups[i:j + 1] for m in g[0]], 'struct-array',
                               f'{j - i + 1} slots at stride {d:#x}'))
                i = j + 1
                continue
            j = i
        merged.append(groups[i])
        i += 1
    groups = sorted(merged + nodedefs, key=lambda g: g[0][0][0])

    trows, accepted = [], {}
    slot_table = {}
    for t, kind, kev in groups:
        lo, hi = t[0][0], t[-1][0] + 4
        nptr = len(t)
        known = sum(1 for _s, v in t if v in procs)
        refs = code_refs(lo, hi)
        base_refs = sorted({f for f, acc, mode, a in refs if a == lo})
        name = gname.get(lo, '')
        nnew = 0
        nweak = sum(1 for _s, v in t if is_weak(v))
        weak_ok = nptr - nweak >= 2 and nweak <= nptr - nweak and kind not in ('rw-node-def', 'rw-device', 'struct-array')
        for s, v in t:
            slot_table[s] = (lo, kind)
            if v in procs and v not in have:
                continue
            ok = kind != 'single' and (nptr >= 2 or kind in ('fnptr-variable', 'vtable', 'rw-node-def', 'rw-device')) \
                and (weak_ok or not is_weak(v))
            if ok:
                nnew += 1
                accepted.setdefault(v, []).append((s, lo, kind))
        mcls = collections.Counter((procs[v]['class'] or procs[v]['state']) if v in procs else 'new'
                                   for _s, v in t)
        trows.append([hx(lo), hx(hi), (hi - lo) // 4, nptr, known, nnew, kind, kev, name,
                      ';'.join(f'{k} {n}' for k, n in sorted(mcls.items(), key=lambda x: (-x[1], x[0]))),
                      ';'.join(hx(f) for f in base_refs[:8]) + (';...' if len(base_refs) > 8 else ''),
                      len({f for f, *_ in refs})])

    # ---- every non-catalogue game-code value in the scanned sections, with its verdict
    drows = []
    seen_slot = set()
    for sec, lo, hi in SCAN:
        for s in range(lo, hi, 4):
            v = im.u32(s)
            if not (GAME_CODE[0] <= v < GAME_CODE[1]) or v in procs and v not in accepted:
                continue
            seen_slot.add(s)
            why = check(v)
            if v in accepted and any(x[0] == s for x in accepted[v]):
                tlo, kind = slot_table[s]
                verdict = 'catalogue' if v in procs else 'accepted'
                drows.append([hx(v), sec, hx(s), verdict, kind, f'table {tlo:#x}' +
                              (f' (weak: {why})' if is_weak(v) else '') +
                              (f' (was inside the Ghidra body of {absorbed[v]:#x})' if v in absorbed else '')])
            elif s in slot_table:
                tlo, kind = slot_table[s]
                drows.append([hx(v), sec, hx(s), 'rejected', kind,
                              f'{why}; in table {tlo:#x} with too few catalogue / plausible members'
                              if is_weak(v) else f'single slot of table {tlo:#x} with no code read of the slot'])
            else:
                drows.append([hx(v), sec, hx(s), 'rejected', '', why])

    # ---- immediates in catalogued bodies
    imm = collections.defaultdict(list)
    p = cat / 'raw' / 'cat7-code-immediates.csv'
    for r in csv.DictReader(open(p)):
        imm[int(r['value'], 16)].append((int(r['function'], 16), int(r['site'], 16)))
    for v, sites in sorted(imm.items()):
        why = check(v)
        ev = ';'.join(f'{f:#x}@{s:#x}' for f, s in sorted(sites)[:4]) + (';...' if len(sites) > 4 else '')
        if why == '':
            accepted.setdefault(v, []).append((sites[0][1], sites[0][0], 'code-immediate'))
            drows.append([hx(v), 'code-imm', ev, 'accepted', 'code-immediate', f'{len(sites)} site(s)' +
                          (f' (was inside the Ghidra body of {absorbed[v]:#x})' if v in absorbed else '')])
        else:
            drows.append([hx(v), 'code-imm', ev, 'rejected', 'code-immediate', why])
    drows.sort(key=lambda r: (int(r[0], 16), r[1], r[2]))

    write_csv(cat / 'data-tables.csv', ['start', 'end', 'slots', 'pointers', 'in_catalogue_before', 'new',
                                         'kind', 'kind_evidence', 'name', 'members',
                                         'base_code_refs', 'n_referencing_functions'], trows)
    write_csv(cat / 'data-refs.csv', ['value', 'source', 'slot_or_sites', 'verdict', 'kind', 'evidence'], drows)

    # counted-only sections
    counts = collections.Counter()
    for sec, lo, hi in COUNT_ONLY:
        for s in range(lo, hi, 4):
            v = im.u32(s)
            if GAME_CODE[0] <= v < GAME_CODE[1]:
                counts[(sec, 'catalogue' if v in procs else 'other')] += 1
    print('count-only sections:', dict(sorted(counts.items())))

    # ---- data-code-addresses.csv: accepted entries; kinds / evidence of existing rows are refreshed
    rows = dict(have)
    for v, ev in sorted(accepted.items()):
        tab = sorted(x for x in ev if x[2] != 'code-immediate')
        if tab:
            s, tlo, kind = tab[0]
            e = f'slot {s:#x} of table {tlo:#x}' + (f' (+{len(tab) - 1} more slots)' if len(tab) > 1 else '')
        else:
            s, f, kind = sorted(ev)[0]
            e = f'immediate at {s:#x} in {f:#x}' + (f' (+{len(ev) - 1} more)' if len(ev) > 1 else '')
        if v in absorbed:
            e += f'; was inside the Ghidra body of {absorbed[v]:#x} (reached only by falling through padding)'
        rows[v] = {'address': hx(v), 'kind': kind, 'evidence': e}
    new = [v for v in rows if v not in have]
    changed = [v for v in have if (have[v]['kind'], have[v]['evidence']) != (rows[v]['kind'], rows[v]['evidence'])]
    kinds = collections.Counter(r['kind'].split(':')[0] for r in rows.values())
    print('tables by kind:', dict(sorted(collections.Counter(r[6].split(':')[0] for r in trows).items())))
    print('entries by kind (data-code-addresses.csv):', dict(sorted(kinds.items())))
    print('verdicts:', dict(sorted(collections.Counter((r[1], r[3]) for r in drows).items())))
    if new or changed:
        write_csv(path, ['address', 'kind', 'evidence'],
                  [[x['address'], x['kind'], x['evidence']] for x in sorted(rows.values(), key=lambda x: int(x['address'], 16))])
        print(f'data_refs: added {len(new)} data-referenced entries to {path} ({len(changed)} rows re-classified)')
        sys.exit(3)
    missing = [v for v in rows if v not in procs]
    if missing:
        print('data_refs: rows of data-code-addresses.csv missing from procedures.csv:', [hx(v) for v in missing])
        sys.exit(4)
    pending = [r for r in drows if r[3] == 'accepted']
    print(f'data_refs: {len(pending)} accepted data-referenced entries outside the catalogue '
          f'({len(rows)} rows in {path.name}, all in the catalogue)')


if __name__ == '__main__':
    main()
