#!/usr/bin/env python3
"""Virtual-slot names for the gta-reversed vtables (CAT-12).

gta-reversed declares the vtable of a class with `RH_ScopedVirtualClass(C, vtable, n)` and hooks
only the slots it reimplemented out of line (`RH_ScopedVMTInstall(Name, addr)`). Most small
virtuals (CEvent*::GetEventType, ::GetLifeTime, ...) are inline in the headers and never hooked,
so their original addresses are catalogued as `C::vfunc_N`. This module rebuilds the slot layout
of every class from the header declarations and checks it against the binary before any name is
used:

  layout(C) = layout(primary base) with overrides in place, then C's new virtuals in declaration
              order; the destructor is one slot (MSVC scalar deleting destructor).

A class's layout is *validated* when (1) len(layout) equals the `n` of its RH_ScopedVirtualClass,
(2) every RH_ScopedVMTInstall hook of the class or of an ancestor class sits in the slot the
layout gives its name, in this class's vtable, when the slot is not overridden, and in the class
itself always, (3) every inline `{ return <integer literal>; }` body in the class or an
ancestor (for the slot this class does not override) matches the original slot code
`mov eax, imm32; ret` / `xor eax, eax; ret`, (4) no class in the chain declares two virtuals with
the same name (MSVC orders overloads in reverse, so overloads make the order unprovable), and
(5) every ancestor up to the root is parsed. Only validated classes contribute names.

Usage (report): vtables.py [gta_sa.exe] [gta-reversed]
"""
import csv
import glob
import os
import re
import struct
import sys
from pathlib import Path

import pefile

ROOT = Path(__file__).resolve().parents[2]
BASE = 0x400000

RH_VT = re.compile(r'RH_ScopedVirtualClass\(\s*([\w:]+)\s*,\s*(0x[0-9A-Fa-f]+)\s*,\s*(\d+)')
RH_CLS = re.compile(r'\bRH_Scoped(?:Virtual|Named)?Class\s*\(\s*([\w:<>]+)')
RH_VMT = re.compile(r'RH_ScopedVMTInstall\(\s*(\w+)\s*,\s*(0x[0-9A-Fa-f]+)')
CLASS_HEAD = re.compile(r'^[ \t]*(?:class|struct)\s+(?:NOTSA_\w+\s+|alignas\([^)]*\)\s+)*(\w+)\s*(?:final\s*)?'
                        r'(?::\s*((?:public|protected|private)?\s*(?:virtual\s+)?[\w:<>]+)[^{;]*)?\{', re.M)


def strip_comments(t):
    t = re.sub(r'/\*.*?\*/', lambda m: ' ' * len(m.group(0)), t, flags=re.S)
    return re.sub(r'//[^\n]*', '', t)


def class_bodies(text):
    """Yield (name, base, body) for each class/struct definition with a body (top-level of that
    class only: nested braces are skipped when the body is scanned)."""
    for m in CLASS_HEAD.finditer(text):
        name = m.group(1)
        base = None
        if m.group(2):
            base = re.sub(r'^(public|protected|private)\s+', '', m.group(2).strip())
            base = re.sub(r'^virtual\s+', '', base).split('::')[-1]
        i, depth = m.end(), 1
        while i < len(text) and depth:
            c = text[i]
            if c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
            i += 1
        yield name, base, text[m.end():i - 1]


def top_level(body):
    """The class body with nested brace blocks blanked out (function bodies, nested types),
    keeping inline bodies' text available separately: returns [(decl, inline_body_or_None)]."""
    out, i, cur, depth, start_body = [], 0, '', 0, None
    decl = ''
    while i < len(body):
        c = body[i]
        if depth == 0:
            if c == '{':
                depth, start_body = 1, i + 1
            elif c == ';':
                out.append((decl + cur, None))
                cur, decl = '', ''
            else:
                cur += c
        else:
            if c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
                if depth == 0:
                    out.append((decl + cur, body[start_body:i]))
                    cur, decl = '', ''
        i += 1
    return out


VIRT_NAME = re.compile(r'(~?\w+)\s*\(')


def virtuals(body):
    """[(name, is_override_or_virtual_redecl, inline body)] in declaration order."""
    res = []
    for decl, inline in top_level(body):
        d = ' '.join(decl.split())
        d = re.sub(r'^(?:public|protected|private)\s*:\s*', '', d)
        d = re.sub(r'\b(?:public|protected|private)\s*:', '', d).strip()
        if 'operator' in d:
            if re.search(r'\bvirtual\b', d):
                res.append(('operator?', True, inline))  # never used as a name source
            continue
        is_virtual = bool(re.search(r'\bvirtual\b', d))
        is_override = bool(re.search(r'\)\s*(?:const\s*)?(?:noexcept\s*)?(?:override|final)\b', d))
        if not (is_virtual or is_override):
            continue
        if d.startswith(('template', 'friend', 'using', 'typedef')):
            continue
        # name = last identifier before the parameter list's opening paren at depth 0
        pm = None
        depth = 0
        for k, ch in enumerate(d):
            if ch == '(' and depth == 0:
                pm = k
                break
            if ch == '<':
                depth += 1
            elif ch == '>':
                depth -= 1
        if pm is None:
            continue
        nm = re.search(r'(~?\w+)\s*$', d[:pm])
        if not nm:
            continue
        res.append((nm.group(1), is_override and not is_virtual, inline))
    return res


def int_literal_return(inline):
    if inline is None:
        return None
    m = re.fullmatch(r'\s*return\s+(-?(?:0x[0-9A-Fa-f]+|\d+))\s*;\s*', inline)
    return int(m.group(1), 0) if m else None


class VTables:
    def __init__(self, exe, gr):
        self.pe = pefile.PE(str(exe), fast_load=True)
        self.img = self.pe.get_memory_mapped_image()
        gr = Path(gr)
        self.vt = {}       # class -> (vtable, n)
        self.vmt = {}      # class -> {name: addr}
        for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.cpp'), recursive=True)):
            text = open(f, errors='replace').read()
            for m in RH_VT.finditer(text):
                self.vt.setdefault(m.group(1), (int(m.group(2), 16), int(m.group(3))))
            cls = None
            for line in text.splitlines():
                if (m := RH_CLS.search(line)):
                    cls = m.group(1)
                for m in RH_VMT.finditer(line):
                    if cls:
                        self.vmt.setdefault(cls, {}).setdefault(m.group(1), int(m.group(2), 16))
        self.decl = {}     # class -> (base, [(name, override, inline)])
        for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.h'), recursive=True)):
            text = strip_comments(open(f, errors='replace').read())
            for name, base, body in class_bodies(text):
                v = virtuals(body)
                if name not in self.decl or (v and not self.decl[name][1]):
                    self.decl[name] = (base, v)
        self.layout, self.owner, self.inline, self.why = {}, {}, {}, {}
        for c in sorted(self.vt):
            self._validate(c)

    def u32(self, va):
        return struct.unpack_from('<I', self.img, va - BASE)[0]

    def code(self, va, n):
        return bytes(self.img[va - BASE:va - BASE + n])

    def chain(self, c):
        out, seen = [], set()
        while c and c not in seen:
            seen.add(c)
            out.append(c)
            c = self.decl.get(c, (None, []))[0]
        return list(reversed(out))   # root first

    def _layout(self, c):
        """(slots [name], inline {slot: body text of the most-derived inline def}, owner {slot: class
        that last (re)defined the slot}) or a string reason."""
        slots, inline, owner = [], {}, {}
        for k in self.chain(c):
            if k not in self.decl:
                return f'class {k} not parsed'
            names = [n for n, _o, _i in self.decl[k][1]]
            dup = {n for n in names if names.count(n) > 1 and n != 'operator?'}
            if dup:
                return f'{k} declares overloaded virtuals {sorted(dup)}'
            for n, is_ovr, body in self.decl[k][1]:
                key = '~' if n.startswith('~') else n
                if key in slots:
                    i = slots.index(key)
                elif is_ovr:
                    return f'{k}::{n} is `override` but no base declares it'
                else:
                    slots.append(key)
                    i = len(slots) - 1
                owner[i] = k
                if body is not None:
                    inline[i] = body
                else:
                    inline.pop(i, None)
        return slots, inline, owner

    def _validate(self, c):
        vt, n = self.vt[c]
        r = self._layout(c)
        if isinstance(r, str):
            self.why[c] = r
            return
        slots, inline, owner = r
        if len(slots) != n:
            self.why[c] = f'header layout has {len(slots)} slots, RH_ScopedVirtualClass says {n}'
            return
        checks = 0
        for k in self.chain(c):
            for name, addr in sorted(self.vmt.get(k, {}).items()):
                if name not in slots:
                    if k == c:
                        self.why[c] = f'hook {k}::{name} {addr:#x} not a virtual in the layout'
                        return
                    continue
                i = slots.index(name)
                if k == c or owner[i] == k:
                    got = self.u32(vt + 4 * i)
                    if got != addr:
                        self.why[c] = f'hook {k}::{name} {addr:#x} != slot {i} ({got:#x})'
                        return
                    checks += 1
        for i, body in inline.items():
            v = int_literal_return(body)
            if v is None:
                continue
            got = self.code(self.u32(vt + 4 * i), 6)
            if v == 0 and got[:3] in (b'\x33\xc0\xc3', b'\x31\xc0\xc3', b'\x32\xc0\xc3'):
                checks += 1
            elif got[:1] == b'\xb8' and struct.unpack('<i', got[1:5])[0] == v and got[5:6] == b'\xc3':
                checks += 1
            elif 0 <= v <= 1 and got[:2] == b'\xb0' + bytes([v]) and got[2:3] == b'\xc3':
                checks += 1
            else:
                self.why[c] = f'inline slot {i} ({slots[i]}) returns {v}, original starts {got.hex()}'
                return
        self.layout[c] = slots
        self.owner[c] = owner
        self.why[c] = f'validated ({checks} checks)'

    def slot_names(self):
        """{implementation address: [(class, slot, name, vtable, declaring class)]} for validated
        classes. The declaring class is the last class in the chain whose header (re)declares the
        slot; when the slot is inherited it names the base whose code is in the vtable."""
        out = {}
        for c in sorted(self.layout):
            vt, n = self.vt[c]
            for i, name in enumerate(self.layout[c]):
                own = self.owner[c][i]
                nm = f'~{own.split("::")[-1]}' if name == '~' else name
                out.setdefault(self.u32(vt + 4 * i), []).append((c, i, nm, vt, own))
        return out

    def ancestors(self, c):
        return set(self.chain(c)) - {c}


def implementation_name(vts, addr, slots):
    """Name for a code address that is the implementation of the given (class, slot, name, vt,
    declaring class) entries. When every entry has the same declaring class, the name is
    `Declaring::Slot` (an inherited implementation is named after the base that declares it).
    Otherwise (identical code folded across unrelated classes) the most-base vtable class wins,
    tie-break by vtable address. Returns (name, evidence)."""
    classes = {s[0] for s in slots}
    tops = sorted([s for s in slots if not (vts.ancestors(s[0]) & classes)], key=lambda s: s[3])
    c, i, nm, vt, own = tops[0]
    owners = {s[4] for s in slots}
    names = sorted({s[2] for s in slots})
    if len(owners) == 1 and len(names) == 1:
        ev = f'slot {i} {nm} declared by {own}, in the vtable of {c} {vt:#x}' + \
             (f' and {len(slots) - 1} more' if len(slots) > 1 else '') + \
             ' (layout from gta-reversed headers, validated)'
        return f'{own}::{nm}', ev
    ev = f'slot {i} {nm} of {c} vtable {vt:#x} (layout from gta-reversed headers, validated)'
    if len(owners) > 1:
        ev += f'; one body for {len(slots)} vtable slots declared by different classes (identical code ' \
              f'folded): ' + ', '.join(f'{s[4]}::{s[2]}' for s in sorted(slots, key=lambda s: s[3])[:6])
    if len(names) > 1:
        ev += f'; slot names differ: {"/".join(names)}'
    return f'{c}::{nm}', ev


def main():
    exe = sys.argv[1] if len(sys.argv) > 1 else ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
    gr = sys.argv[2] if len(sys.argv) > 2 else ROOT / 're' / 'gta-reversed'
    v = VTables(exe, gr)
    ok = sorted(c for c in v.vt if c in v.layout)
    print(f'{len(v.vt)} gta-reversed vtables, {len(ok)} validated layouts, '
          f'{sum(len(v.layout[c]) for c in ok)} slots')
    reasons = {}
    for c in sorted(v.vt):
        if c not in v.layout:
            reasons.setdefault(re.sub(r'[\w:~]+::|0x[0-9a-f]+|\d+', '#', v.why[c])[:60], []).append(c)
    for r, cs in sorted(reasons.items(), key=lambda kv: -len(kv[1])):
        print(f'  {len(cs):4d} not validated: {r} e.g. {cs[0]}: {v.why[cs[0]]}')


if __name__ == '__main__':
    main()
