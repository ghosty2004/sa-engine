#!/usr/bin/env python3
"""Build the function catalogue (M0) from gta-reversed hook registrations and
verify every address against gta_sa.exe 1.0 US.

Outputs:
  catalog/functions.csv   one row per hooked function
  catalog/callees.csv     addresses still called into the original exe
                          (plugin::Call*, raw function-pointer thunks)
  catalog/summary.md      counts and verification results

Usage: .venv/bin/python -I tools/catalog/build_catalog.py <gta-reversed> <gta_sa.exe> <outdir>
"""
import csv
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import capstone
import pefile

TEXT_LO, TEXT_HI = 0x401000, 0x857000  # first .text section (game code)

HOOK_RE = re.compile(r'\b(RH_Scoped(?:\w*Install\w*|VmtInstall))\s*\((.*)$')
CLASS_RE = re.compile(r'\bRH_Scoped(?:Virtual|Named)?Class\s*\(\s*([\w:<>]+)')
CAT_RE = re.compile(r'\bRH_ScopedCategory\s*\(\s*"([^"]*)"')
NS_RE = re.compile(r'\bRH_ScopedNamespace(?:Name)?\s*\(\s*"?([\w:]+)')
ADDR_RE = re.compile(r'0x([0-9A-Fa-f]{6,8})\b')
CALL_RE = re.compile(r'plugin::Call\w*<\s*0x([0-9A-Fa-f]{6,8})')
THUNK_RE = re.compile(r'\)\)\s*0x([0-9A-Fa-f]{6,8})\)')


def split_args(s):
    """Split the macro argument list at top-level commas; stop at the closing paren."""
    args, depth, cur = [], 0, ''
    for ch in s:
        if ch in '(<{[':
            depth += 1
        elif ch in ')>}]':
            if depth == 0 and ch == ')':
                args.append(cur.strip())
                return args
            depth -= 1
        if ch == ',' and depth == 0:
            args.append(cur.strip())
            cur = ''
        else:
            cur += ch
    args.append(cur.strip())
    return args


def scan_sources(root):
    hooks, callees = [], []
    for path in sorted(Path(root, 'source').rglob('*')):
        if path.suffix not in ('.cpp', '.h', '.hpp'):
            continue
        rel = str(path.relative_to(root))
        cls = ns = cat = ''
        lines = path.read_text(errors='replace').splitlines()
        for ln, line in enumerate(lines, 1):
            if (m := CLASS_RE.search(line)):
                cls, ns = m.group(1), ''
            if (m := NS_RE.search(line)):
                ns = m.group(1)
            if (m := CAT_RE.search(line)):
                cat = m.group(1)
            for m in CALL_RE.finditer(line):
                callees.append((int(m.group(1), 16), 'plugin_call', rel, ln))
            for m in THUNK_RE.finditer(line):
                callees.append((int(m.group(1), 16), 'raw_thunk', rel, ln))
            m = HOOK_RE.search(line)
            if not m or line.lstrip().startswith('#define'):
                continue
            macro, rest = m.group(1), m.group(2)
            # Join continuation lines until the macro closes.
            j = ln
            while rest.count('(') + 1 > rest.count(')') and j < len(lines) and j - ln < 6:
                rest += ' ' + lines[j]
                j += 1
            args = split_args(rest)
            addr = next((int(a.group(1), 16) for a in (ADDR_RE.search(x) for x in args) if a), None)
            if addr is None:
                continue
            fn = args[0]
            if 'Overloaded' in macro and len(args) > 1:
                fn = f'{args[0]}-{args[1].strip(chr(34))}'
            elif 'Named' in macro and len(args) > 1:
                fn = args[1].strip('"')
            owner = cls or ns or ''
            flags = ' '.join(args)
            hooks.append({
                'address': addr,
                'owner': owner,
                'name': fn,
                'macro': macro,
                'category': cat,
                'reversed': 'no' if re.search(r'\.reversed\s*=\s*false', flags, re.I) else 'yes',
                'locked': 'yes' if re.search(r'\.locked\s*=\s*true', flags, re.I) else 'no',
                'file': rel,
                'line': ln,
            })
    return hooks, callees


class Image:
    def __init__(self, exe):
        self.pe = pefile.PE(exe, fast_load=True)
        self.base = self.pe.OPTIONAL_HEADER.ImageBase
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.secs = [(s.Name.rstrip(b'\0').decode(), self.base + s.VirtualAddress,
                      s.Misc_VirtualSize, s) for s in self.pe.sections]

    def section(self, va):
        for name, lo, size, _ in self.secs:
            if lo <= va < lo + size:
                return name, lo
        return None, None

    def read(self, va, n):
        return self.pe.get_data(va - self.base, n)

    def classify(self, va):
        """Return (status, first instruction text)."""
        name, lo = self.section(va)
        if name is None:
            return 'outside-image', ''
        if not (TEXT_LO <= va < TEXT_HI):
            return f'outside-game-text({name}@{lo:#x})', ''
        try:
            data = self.read(va, 16)
        except Exception:
            return 'unreadable', ''
        ins = next(self.md.disasm(data, va), None)
        if ins is None:
            return 'undecodable', data[:8].hex()
        text = f'{ins.mnemonic} {ins.op_str}'.strip()
        if ins.mnemonic in ('jmp', 'call') and ins.op_str.startswith('0x'):
            tgt = int(ins.op_str, 16)
            if not (TEXT_LO <= tgt < TEXT_HI):
                return f'stub-to-{self.section(tgt)[0] or "outside"}', text
        if data[:4] in (b'\xcc\xcc\xcc\xcc', b'\x00\x00\x00\x00'):
            return 'padding', text
        return 'ok', text


def main():
    gr, exe, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    hooks, callees = scan_sources(gr)
    img = Image(exe)

    status = Counter()
    for h in hooks:
        h['status'], h['first_insn'] = img.classify(h['address'])
        status[h['status'].split('(')[0]] += 1

    dup = defaultdict(list)
    for h in hooks:
        dup[h['address']].append(h)

    cols = ['address', 'owner', 'name', 'category', 'macro', 'reversed', 'locked',
            'status', 'first_insn', 'file', 'line']
    with open(out / 'functions.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, cols)
        w.writeheader()
        for h in sorted(hooks, key=lambda h: h['address']):
            w.writerow({**h, 'address': f"{h['address']:#08x}"})

    hooked = set(dup)
    cal = defaultdict(lambda: {'kinds': set(), 'sites': 0, 'first': ''})
    for addr, kind, rel, ln in callees:
        c = cal[addr]
        c['kinds'].add(kind)
        c['sites'] += 1
        c['first'] = c['first'] or f'{rel}:{ln}'
    with open(out / 'callees.csv', 'w', newline='') as f:
        w = csv.writer(f)
        w.writerow(['address', 'kinds', 'sites', 'also_hooked', 'status', 'first_insn', 'first_site'])
        for addr in sorted(cal):
            st, ins = img.classify(addr)
            c = cal[addr]
            w.writerow([f'{addr:#08x}', '+'.join(sorted(c['kinds'])), c['sites'],
                        'yes' if addr in hooked else 'no', st, ins, c['first']])

    rev = Counter(h['reversed'] for h in hooks)
    multi = {a: hs for a, hs in dup.items() if len(hs) > 1}
    bad = [h for h in hooks if h['status'] != 'ok']
    unhooked_callees = [a for a in cal if a not in hooked]
    with open(out / 'summary.md', 'w') as f:
        f.write('# Catalogue summary (generated)\n\n')
        f.write(f'- Source: `{gr}`\n- Binary: `{exe}`\n\n')
        f.write(f'- Hook registrations: {len(hooks)} ({len(dup)} distinct addresses)\n')
        f.write(f'- Marked reversed: {rev["yes"]}, not reversed: {rev["no"]}\n')
        f.write(f'- Addresses registered more than once: {len(multi)}\n')
        f.write(f'- Callee addresses into the original (plugin::Call / raw thunk): {len(cal)} '
                f'distinct, {len(callees)} sites, {len(unhooked_callees)} not hooked anywhere\n\n')
        f.write('## Address verification against 1.0 US\n\n| Status | Count |\n|---|---|\n')
        for k, v in status.most_common():
            f.write(f'| {k} | {v} |\n')
        f.write('\n## Non-ok hook addresses (first 200)\n\n| Address | Owner::Name | Status | First insn |\n|---|---|---|---|\n')
        for h in bad[:200]:
            f.write(f"| {h['address']:#08x} | {h['owner']}::{h['name']} | {h['status']} | `{h['first_insn']}` |\n")
    print(open(out / 'summary.md').read()[:3000])


if __name__ == '__main__':
    main()
