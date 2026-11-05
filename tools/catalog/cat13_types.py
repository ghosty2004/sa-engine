#!/usr/bin/env python3
"""CAT-13: static receiver types for virtual calls on objects other than `this`.

Three type sources, all from gta-reversed (re/gta-reversed) and checked before use:

  members   Class layouts computed from the gta-reversed headers (vptr, primary base, data members
            in declaration order with MSVC alignment, bit-fields, arrays, std::array). A layout is
            used only when its computed size equals the class's VALIDATE_SIZE (catalog/layouts.csv)
            and every VALIDATE_OFFSET of the class matches. A member `C* m_x` (or an embedded `C m_x`)
            whose class C has a known vtable types the load `mov r, [obj + off]`.
  globals   `StaticRef<T>(0xADDR)`, `T& x = *(T*)0xADDR`, `T* x = (T*)0xADDR` declarations: a global
            holding a `C*`, or an object/array of class C.
  returns   Declarations `C* Name(...)` in a class body (or at file scope) for the gta-reversed hook
            `Class::Name` (functions.csv); overloads with different return classes are dropped.

Used by callgraph.py build (`virtual-typed` edges). Usage (report): cat13_types.py
"""
import collections
import csv
import glob
import os
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from vtables import class_bodies, strip_comments, top_level  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
PRIM = {}
for n in ('bool', 'char', 'int8', 'uint8', 'int8_t', 'uint8_t', 'unsigned char', 'signed char', 'RwUInt8',
          'RwInt8', 'BYTE', 'byte'):
    PRIM[n] = 1
for n in ('int16', 'uint16', 'int16_t', 'uint16_t', 'short', 'unsigned short', 'wchar_t', 'char16_t',
          'RwInt16', 'RwUInt16', 'WORD', 'GxtChar'):
    PRIM[n] = 2
for n in ('int32', 'uint32', 'int32_t', 'uint32_t', 'int', 'unsigned', 'unsigned int', 'long', 'unsigned long',
          'float', 'DWORD', 'RwInt32', 'RwUInt32', 'RwReal', 'RwBool', 'size_t', 'uintptr_t', 'float32',
          'BOOL', 'HANDLE', 'HWND', 'LONG', 'UINT', 'RwFixed', 'RwBlendFunction'):
    PRIM[n] = 4
for n in ('int64', 'uint64', 'int64_t', 'uint64_t', 'double', 'long long', 'unsigned long long'):
    PRIM[n] = 8
SKIP_DECL = re.compile(r'^\s*(?:static\b|using\b|typedef\b|friend\b|template\b|enum\b|struct\s+\w+\s*$|class\s+\w+\s*$|'
                       r'public\s*:|private\s*:|protected\s*:|static_assert|constexpr\b|inline\s+static|'
                       r'NOTSA_|PLUGIN_NO|RH_|VALIDATE_|#)')


class Layouts:
    def __init__(self, gr, cat):
        gr = Path(gr)
        self.vsize, self.voff = {}, collections.defaultdict(dict)
        for r in csv.DictReader(open(Path(cat) / 'layouts.csv')):
            try:
                v = int(r['value'], 0)
            except ValueError:
                continue
            if r['kind'] == 'size':
                self.vsize[r['type'].strip()] = v
            elif r['kind'] == 'offset':
                self.voff[r['type'].strip()][r['member'].strip()] = v
        self.decl = {}          # class -> (base, [(decl, inline)], file, has_virtual)
        self.enums = {}         # enum -> size
        self.alias = {}         # alias -> type text
        self.consts = {}        # constexpr name -> int
        self.const_expr = {}    # constexpr name -> expression text (evaluated on use)
        self.packed = set()     # files with #pragma pack
        for f in sorted(glob.glob(str(gr / 'source' / '**' / '*.h'), recursive=True)):
            raw = open(f, errors='replace').read()
            text = strip_comments(raw)
            rel = os.path.relpath(f, gr)
            if '#pragma pack' in text:
                self.packed.add(rel)
            for m in re.finditer(r'\benum\s+(?:class\s+|struct\s+)?(\w+)\s*(?::\s*([\w ]+?))?\s*\{', text):
                u = (m.group(2) or 'int32').strip()
                self.enums.setdefault(m.group(1), PRIM.get(u, 4))
            for m in re.finditer(r'\busing\s+(\w+)\s*=\s*([^;{}()]+);', text):
                self.add_alias(m.group(1), m.group(2))
            for m in re.finditer(r'\btypedef\s+([^;{}()]+?)\s+(\w+)\s*;', text):
                self.add_alias(m.group(2), m.group(1))
            # enumerator values (explicit or implicit), for array bounds such as TASK_PRIMARY_MAX
            for m in re.finditer(r'\benum\s+(?:class\s+|struct\s+)?\w*\s*(?::\s*[\w ]+?)?\s*\{([^{}]*)\}', text):
                v = -1
                for item in m.group(1).split(','):
                    item = item.strip()
                    mm = re.match(r'^(\w+)\s*(?:=\s*(.+))?$', item)
                    if not mm:
                        continue
                    if mm.group(2) is not None:
                        x = self.number(mm.group(2).strip())
                        if x is None:
                            v = None
                            continue
                        v = x
                    elif v is None:
                        continue
                    else:
                        v += 1
                    self.consts.setdefault(mm.group(1), v)
            for m in re.finditer(r'\bconstexpr\s+(?:(?:static|inline)\s+)*[\w:]+\s+(\w+)\s*=\s*(0x[0-9A-Fa-f]+|\d+)u?\s*;', text):
                self.consts.setdefault(m.group(1), int(m.group(2), 0))
            for m in re.finditer(r'\bconstexpr\s+(?:(?:static|inline)\s+)*[\w:]+\s+(\w+)\s*=\s*([^;{}]+);', text):
                self.const_expr.setdefault(m.group(1), m.group(2))
            for m in re.finditer(r'^\s*(?:static\s+)?const\s+[\w:]+\s+(\w+)\s*=\s*(0x[0-9A-Fa-f]+|\d+)u?\s*;', text, re.M):
                self.consts.setdefault(m.group(1), int(m.group(2), 0))
            for m in re.finditer(r'NOTSA_WENUM_DEFS_FOR\(\s*(\w+)\s*\)', text):
                for k in ('S8', 'S16', 'S32', 'U8', 'U16', 'U32'):
                    self.alias.setdefault(m.group(1) + k, f'notsa::WEnum{k}<{m.group(1)}>')
            for m in re.finditer(r'^\s*#\s*define\s+(\w+)\s+\(?(0x[0-9A-Fa-f]+|\d+)\)?\s*$', text, re.M):
                self.consts.setdefault(m.group(1), int(m.group(2), 0))
            for name, base, body in class_bodies(text):
                if name in self.decl:
                    continue
                items = top_level(body)
                virt = any(re.search(r'\bvirtual\b|\boverride\b', d) for d, _i in items)
                self.decl[name] = (base, items, rel, virt)
        self.cache = {}
        self.why = {}

    def add_alias(self, a, t):
        t = ' '.join(re.sub(r'\b(?:struct|class|enum|union)\b', ' ', t).split())
        if t and t != a:
            self.alias.setdefault(a, t)

    # ------------------------------------------------------------------ sizes
    def type_info(self, t, depth=0):
        """(size, align, class name or None) of a type text, or None."""
        t = re.sub(r'\b(?:const|volatile|mutable|struct|class|enum|union|typename)\b', ' ', t)
        t = ' '.join(t.split())
        if not t or depth > 8:
            return None
        if t.endswith('*') or t.endswith('&'):
            return (4, 4, None)
        if t in PRIM:
            return (PRIM[t], PRIM[t], None)
        m = re.fullmatch(r'(?:notsa::)?EntityRef<\s*(\w*)\s*>', t)
        if m:   # gta-reversed wrapper around a plain entity pointer (4 bytes in the original)
            return (4, 4, None)
        m = re.fullmatch(r'(?:notsa::)?WEnum([SU])(8|16|32)<.*>', t)
        if m:
            return (int(m.group(2)) // 8, int(m.group(2)) // 8, None)
        m = re.fullmatch(r'(?:std::)?array<(.+)>', t)
        if m:
            parts = split_top(m.group(1), ',')
            if len(parts) != 2:
                return None
            n = self.number(parts[1])
            ti = self.type_info(parts[0], depth + 1)
            if n is None or ti is None:
                return None
            return (ti[0] * n, ti[1], None)
        if '<' in t or '::' in t and t.split('::')[-1] not in self.decl and t.split('::')[-1] not in self.enums:
            t2 = t.split('::')[-1]
            if '<' in t or t2 not in self.decl and t2 not in self.enums and t2 not in self.alias:
                return None
            t = t2
        t = t.split('::')[-1]
        if t in self.enums:
            return (self.enums[t], self.enums[t], None)
        if t in self.alias and self.alias[t] != t:
            return self.type_info(self.alias[t], depth + 1)
        lay = self.layout(t, depth + 1)
        if lay:
            return (lay['size'], lay['align'], t)
        return None

    def number(self, s, _depth=0):
        """Integer value of a literal, a known constant or a +/-/* expression of those."""
        s = s.strip()
        try:
            return int(s.rstrip('uU'), 0)
        except ValueError:
            pass
        s = re.sub(r'static_cast<[^<>]*>', '', s)
        s = re.sub(r'\b(\d+)[uU]\b', r'\1', s)
        toks = re.findall(r'[\w:]+|[-+*()]', s)
        if not toks or ''.join(toks) != re.sub(r'\s+', '', s):
            return None
        out = []
        for t in toks:
            if t in '+-*()':
                out.append(t)
                continue
            try:
                out.append(str(int(t.rstrip('uU'), 0)))
            except ValueError:
                k = t.split('::')[-1]
                v = self.consts.get(k)
                if v is None and k in self.const_expr and _depth < 6:
                    v = self.number(self.const_expr[k], _depth + 1)
                if v is None:
                    return None
                out.append(str(v))
        try:
            return int(eval(' '.join(out), {'__builtins__': {}}))  # digits and + - * ( ) only
        except Exception:
            return None

    # ------------------------------------------------------------------ class layout
    def layout(self, c, depth=0):
        """{'size','align','vptr','members': [(off, size, kind, class, name)]} (flattened over the
        primary base) or None. Unvalidated layouts are kept for nested use (their parent is then
        validated by its own size); `validated(c)` tells whether c itself matched VALIDATE_SIZE."""
        if c in self.cache:
            return self.cache[c]
        self.cache[c] = None   # recursion guard
        if c not in self.decl or depth > 12:
            self.why[c] = 'not parsed'
            return None
        base, items, rel, virt = self.decl[c]
        if rel in self.packed:
            self.why[c] = '#pragma pack in the header'
            return None
        members, off, align, vptr = [], 0, 1, False
        if base:
            bl = self.layout(base.split('<')[0].split('::')[-1], depth + 1)
            if not bl:
                self.why[c] = f'base {base} has no layout'
                return None
            members = list(bl['members'])
            off, align, vptr = bl['size'], bl['align'], bl['vptr']
        if virt and not vptr:
            if base:
                self.why[c] = 'introduces a vptr over a non-virtual base'
                return None
            vptr = True
            off, align = 4, 4
        bits = None     # (unit size, bits used)
        last_ty = ''    # type of the previous member list (for `T a{}, b{};`)
        in_init = False
        after_anon = False
        parens, pending, skip_name = 0, None, False
        for d, inline in items:
            d = ' '.join(d.split())
            d = re.sub(r'^(?:public|private|protected)\s*:\s*', '', d)
            d = re.sub(r'\b(?:public|private|protected)\s*:', '', d).strip()
            if parens > 0:
                parens += d.count('(') - d.count(')')
                continue   # rest of a parameter list split at a brace default argument
            if d.count('(') > d.count(')'):
                parens = d.count('(') - d.count(')')
                continue
            if after_anon and re.fullmatch(r'\w+', d):
                after_anon = False
                continue   # name of the preceding `struct { ... } name;`
            after_anon = False
            if d.startswith(')'):
                continue   # tail of a declaration split at a brace default argument
            if in_init and (d.startswith(',') or not d):
                continue   # rest of a constructor's member initialiser list
            in_init = False
            if re.search(r'\)\s*(?:noexcept\s*)?:\s*[\w:]+$', d) and inline is not None:
                in_init = True   # `C(...) : m_a{x}` -> following `, m_b{y}` chunks belong to it
                continue
            nested = re.fullmatch(r'((?:static\s+|inline\s+|constexpr\s+)*)(struct|union|class)\s+(\w+)(?:\s*:.*)?', d)
            if inline is not None and nested:
                # named nested type: a definition, maybe followed by a member of that type
                if not nested.group(1):
                    self.decl.setdefault(nested.group(3), (None, top_level(inline), rel, False))
                skip_name = bool(nested.group(1))
                pending = None if skip_name else nested.group(3)
                continue
            if pending is not None or skip_name:
                if re.fullmatch(r'\w+(?:\s*\[[^\]]*\])*', d) and pending is not None:
                    d = f'{pending} {d}'
                elif re.fullmatch(r'\w+(?:\s*\[[^\]]*\])*', d) and skip_name:
                    skip_name = False
                    continue
                pending, skip_name = None, False
            if not d or SKIP_DECL.match(d):
                bits = None
                continue
            if inline is not None and re.fullmatch(r'(?:union|struct)', d):
                # anonymous union / struct: members in place (union: all at one offset)
                sub = self.anon(inline, d == 'union')
                if sub is None:
                    self.why[c] = f'anonymous {d} not parsed'
                    return None
                ssz, sal, smem = sub
                off = (off + sal - 1) // sal * sal
                members += [(off + o, sz_, k_, c_, n_) for o, sz_, k_, c_, n_ in smem]
                off += ssz
                align = max(align, sal)
                bits = None
                after_anon = True
                continue
            if inline is not None:
                if re.search(r'\bunion\b|\bstruct\b|\bclass\b', d.split('(')[0]):
                    self.why[c] = f'nested aggregate member `{d[:40]}`'
                    return None
                if '(' in d:
                    continue   # inline member function
                # `T m_x{...}`: brace-initialised member
            if re.search(r'\boperator\b', d):
                continue
            # function pointer member `R (*name)(args)`
            fp = re.match(r'^[\w:<>\s\*&]+\(\s*\*\s*(\w+)\s*\)\s*\(', d)
            if '(' in d and not fp:
                if '=' in d and d.index('=') < d.index('('):
                    d = d[:d.index('=')]
                else:
                    continue   # member function declaration
            d = re.sub(r'\s*=\s*.*$', '', d) if not fp else d
            d = re.sub(r'\{.*$', '', d).strip()
            if fp:
                sz, al, kind, cls_, name = 4, 4, 'fptr', None, fp.group(1)
                arr = 1
            else:
                bm = re.match(r'^(.+?)\s+(\w+)\s*:\s*(\w+)$', d)
                if bm:
                    ti = self.type_info(bm.group(1))
                    nb = self.number(bm.group(3))
                    if ti is None or nb is None:
                        self.why[c] = f'bit-field type `{d[:40]}`'
                        return None
                    if bits and bits[0] == ti[0] and bits[1] + nb <= 8 * ti[0]:
                        bits = (bits[0], bits[1] + nb)
                        continue
                    al_ = ti[1]
                    off = (off + al_ - 1) // al_ * al_
                    members.append((off, ti[0], 'bits', None, bm.group(2)))
                    off += ti[0]
                    align = max(align, al_)
                    bits = (ti[0], nb)
                    continue
                bits = None
                if d.startswith(',') and last_ty:
                    d = last_ty + ' ' + d[1:].strip()
                parts = [p.strip() for p in split_top(d, ',')]
                m = re.match(r'^(.*?[\s\*&>])\s*(\w+)\s*((?:\[[^\]]*\])*)$', parts[0])
                if not m:
                    self.why[c] = f'cannot parse member `{d[:50]}`'
                    return None
                ty, first, dims = m.group(1).strip(), m.group(2), m.group(3)
                last_ty = re.sub(r'[\*&]+$', '', ty).strip() if not re.search(r'[\*&]$', ty) else ''
                names = [(first, dims)]
                for p in parts[1:]:
                    mm = re.match(r'^([\*&]*)\s*(\w+)\s*((?:\[[^\]]*\])*)$', p)
                    if not mm or mm.group(1):
                        self.why[c] = f'cannot parse member list `{d[:50]}`'
                        return None
                    names.append((mm.group(2), mm.group(3)))
                ti = self.type_info(ty)
                if ti is None:
                    self.why[c] = f'unknown member type `{ty}`'
                    return None
                for name, dims in names:
                    arr = 1
                    for dm in re.findall(r'\[([^\]]*)\]', dims):
                        n = self.number(dm)
                        if n is None:
                            self.why[c] = f'array bound `{dm}`'
                            return None
                        arr *= n
                    sz, al = ti[0] * arr, ti[1]
                    off = (off + al - 1) // al * al
                    tcls = re.sub(r'\b(?:const|class|struct)\b', ' ', ty).strip()
                    if tcls.endswith('*') and not tcls.endswith('**'):
                        kind, cls_ = 'ptr', tcls[:-1].strip().split('::')[-1]
                    elif ti[2] and arr == 1:
                        kind, cls_ = 'obj', ti[2]
                    else:
                        kind, cls_ = 'data', None
                    members.append((off, sz, kind, cls_, name))
                    off += sz
                    align = max(align, al)
                continue
            off = (off + al - 1) // al * al
            members.append((off, sz, kind, cls_, name))
            off += sz
            align = max(align, al)
        size = max(1, (off + align - 1) // align * align)
        lay = {'size': size, 'align': align, 'vptr': vptr, 'members': members}
        self.cache[c] = lay
        return lay

    def anon(self, body, is_union):
        """(size, align, members) of an anonymous union/struct body, or None."""
        def one(items):
            key = f'__anon_{id(items)}_{len(self.cache)}'
            self.decl[key] = (None, items, '', False)
            lay = self.layout(key)
            del self.decl[key]
            self.cache.pop(key, None)
            return lay
        if not is_union:
            lay = one(top_level(body))
            return (lay['size'], lay['align'], lay['members']) if lay else None
        size, al, mem = 0, 1, []
        prev_aggr = False
        items = top_level(body)
        for k, (d, inline) in enumerate(items):
            d = ' '.join(d.split())
            if not d or (prev_aggr and re.fullmatch(r'\w+', d)):
                prev_aggr = False
                continue
            prev_aggr = inline is not None and bool(re.fullmatch(r'(?:struct|union)(?:\s+\w+)?', d))
            group = [(d, inline)]
            if prev_aggr and k + 1 < len(items) and re.fullmatch(r'\w+', ' '.join(items[k + 1][0].split())):
                group.append(items[k + 1])   # `struct X { ... } name;` inside the union
            lay = one(group)
            if not lay:
                return None
            size, al = max(size, lay['size']), max(al, lay['align'])
            mem += [m for m in lay['members'] if m[0] == 0 and m[2] in ('ptr', 'obj')]
        return (size + al - 1) // al * al, al, mem

    def validated(self, c):
        lay = self.layout(c)
        if not lay:
            return False
        if self.vsize.get(c) != lay['size']:
            if c in self.vsize:
                self.why[c] = f'computed size {lay["size"]:#x} != VALIDATE_SIZE {self.vsize[c]:#x}'
            else:
                self.why[c] = 'no VALIDATE_SIZE'
            return False
        own = {n: o for o, _s, _k, _c, n in lay['members']}
        for mname, v in self.voff.get(c, {}).items():
            if own.get(mname) != v:
                self.why[c] = f'VALIDATE_OFFSET {mname} {v:#x} != {own.get(mname)}'
                return False
        self.why[c] = 'validated'
        return True

    def member_at(self, c, off, depth=0):
        """(kind, class, name) of the member at exactly `off` in validated class c (descending into
        embedded objects), or None."""
        if depth > 6 or not self.validated(c):
            return None
        for o, s, k, mc, n in self.layout(c)['members']:
            if o == off and k in ('ptr', 'obj'):
                return (k, mc, n)
            if k == 'obj' and o < off < o + s:
                r = self.member_at(mc, off - o, depth + 1)
                return r if r is None else (r[0], r[1], f'{n}.{r[2]}')
        return None


def split_top(s, sep):
    out, depth, cur = [], 0, ''
    for ch in s:
        if ch in '<([':
            depth += 1
        elif ch in '>)]':
            depth -= 1
        if ch == sep and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += ch
    out.append(cur)
    return out


# ---------------------------------------------------------------------------- globals, returns
GLOBAL_RES = [
    re.compile(r'StaticRef<\s*(?P<t>[^>]+?)\s*(?:,\s*(?P<a1>0x[0-9A-Fa-f]{6,7}))?\s*>\s*\(\s*(?P<a2>0x[0-9A-Fa-f]{6,7})?'),
    re.compile(r'^\s*(?:static\s+|inline\s+|extern\s+)*(?P<t>[\w:<>]+\s*\**)\s*&\s*[\w:]+\s*=\s*\*\s*\(\s*(?P<t2>[\w:<>]+\s*\**)\s*\*\s*\)\s*'
               r'\(?\s*(?P<a2>0x[0-9A-Fa-f]{6,7})'),
    re.compile(r'^\s*(?:static\s+|inline\s+|extern\s+)*(?P<t>[\w:<>]+)\s*\*\s*[\w:]+\s*=\s*\(\s*(?P<t2>[\w:<>]+)\s*\*\s*\)\s*'
               r'\(?\s*(?P<a2>0x[0-9A-Fa-f]{6,7})'),
]


def global_types(gr, psdk, classes):
    """{address: ('ptr'|'obj', class)} for globals whose class is in `classes`."""
    out = {}
    roots = [(Path(gr) / 'source', '*.[ch]*'), (Path(psdk) / 'plugin_sa' / 'game_sa', '*.[ch]*')]
    for root, pat in roots:
        for f in sorted(glob.glob(str(root / '**' / pat), recursive=True)):
            for line in open(f, errors='replace'):
                if '0x' not in line or line.lstrip().startswith('//'):
                    continue
                for i, rx in enumerate(GLOBAL_RES):
                    m = rx.search(line)
                    if not m:
                        continue
                    a = m.groupdict().get('a1') or m.groupdict().get('a2')
                    if not a:
                        continue
                    t = re.sub(r'\b(?:const|class|struct)\b', ' ', m.group('t')).strip()
                    t = re.sub(r'^(?:std::)?array<\s*([^,]+),.*$', r'\1', t).strip()
                    t = re.sub(r'\[[^\]]*\]', '', t).strip()
                    if i == 2:   # T* x = (T*)addr : an array of T at addr
                        kind, c = 'obj', t
                    elif t.endswith('*'):
                        kind, c = 'ptr', t[:-1].strip()
                    else:
                        kind, c = 'obj', t
                    c = c.split('::')[-1]
                    if '*' in c or c not in classes:
                        continue
                    out.setdefault(int(a, 16), (kind, c))
                    break
    return out


def return_types(gr, classes):
    """{(owner, name): class} for declarations returning `C*` (C in classes), unambiguous only."""
    seen = collections.defaultdict(set)
    rx = re.compile(r'^(?:\s*(?:static|virtual|inline|const|NOTSA_\w+|\[\[nodiscard\]\])\s+)*(?:const\s+)?'
                    r'(?:class\s+|struct\s+)?(\w+)\s*\*\s*(\w+)\s*\(')
    for f in sorted(glob.glob(str(Path(gr) / 'source' / '**' / '*.h'), recursive=True)):
        text = strip_comments(open(f, errors='replace').read())
        spans = []
        for name, _base, body in class_bodies(text):
            spans.append(body)
            for d, _inline in top_level(body):
                d = ' '.join(d.split())
                d = re.sub(r'^(?:public|private|protected)\s*:\s*', '', d)
                m = rx.match(d)
                if m and m.group(1) in classes:
                    seen[(name, m.group(2))].add(m.group(1))
                elif m:
                    seen[(name, m.group(2))].add(None)
        for line in text.splitlines():
            if line[:1].isspace():
                continue
            m = rx.match(line)
            if m:
                seen[('', m.group(2))].add(m.group(1) if m.group(1) in classes else None)
    return {k: next(iter(v)) for k, v in seen.items() if len(v) == 1 and None not in v}


def main():
    gr = ROOT / 're' / 'gta-reversed'
    L = Layouts(gr, ROOT / 'catalog')
    ok = [c for c in sorted(L.decl) if c in L.vsize and L.validated(c)]
    print(f'{len(L.decl)} classes parsed, {len(L.vsize)} with VALIDATE_SIZE, {len(ok)} layouts validated')
    why = collections.Counter(re.sub(r'`[^`]*`|0x[0-9a-f]+|\w+::\w+', '#', L.why.get(c, '?'))[:50]
                              for c in L.vsize if c in L.decl and c not in ok)
    for k, v in why.most_common(15):
        print(f'  {v:4d}  {k}')


if __name__ == '__main__':
    main()
