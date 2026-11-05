#!/usr/bin/env python3
"""CAT-6: names for catalogue functions that gta-reversed, CAT-5 and the RenderWare names do not name.

Usage: names_extra.py <catalog-dir> [gta_sa.exe] [gta-reversed] [plugin-sdk]

Writes catalog/names-extra.csv (generated, sorted, no timestamps):

    address, name, role, subsystem, source, confidence, evidence

for every live non-RenderWare catalogue function whose base name (gta-reversed hook name, then
catalog/rw-names.csv high/medium, then the CAT-5 name of classification.csv) is empty or a bare
`Class::vfunc_N`. `subsystems.py` merges the file into procedures.csv (`name`, `name_source`,
`name_confidence`) and `progress.py` into state.csv; `low` rows are never used as names (they are
listed in the state evidence as hints only).

Sources, most reliable first (`source` column):
  src-wrapper       a plugin-sdk / gta-reversed function whose body is one statement that calls the
                    address (plugin::Call*<0xADDR>, `((T(__thiscall*)(...))0xADDR)(...)`): the
                    wrapper's own qualified name. high.
  src-converted     plugin-sdk `// Converted from <cc> <ret> Class::Name(args) 0xADDR`. high.
  src-define        plugin-sdk `#define FUNC_Class__Name 0xADDR`. high.
  src-addr-comment  gta-reversed / plugin-sdk `// 0xADDR` (or `@addr 0xADDR`) on the line of a
                    function declaration/definition or directly above a definition. high when the
                    comment is on the same line or the line above, medium otherwise.
  init / dtor       C++ dynamic initialisers of `__xc_a..__xc_z` and the destructor callbacks they
                    register with `_atexit`: `__init_<global>` / `__dtor_<global>`; the global is
                    the `this` passed to the first constructor call, else the first absolute
                    store target. Named globals come from gta-reversed / plugin-sdk declarations
                    (callgraph.global_names); unnamed ones are written `g_<address>`. high (the
                    role is structural; the global's name is as good as its declaration).
  vtable-class      slots of vtables that gta-reversed does not declare in a .cpp: class from
                    RH_ScopedVirtualClass in a header, from the constructor/destructor that installs
                    the vtable (a constructor installs its own vtable last, a destructor first),
                    from the scalar deleting destructor in slot 0 calling `Class::~Class`, or
                    (medium) from the owner of the already named slots that only this vtable has.
                    Slot names come from the validated layout of the class or of its gta-reversed
                    base (vtables.py) when the base's slots agree, else `Class::vfunc_N`.
  hint-string /     low only: `sub_<addr>_<hint>` from the one string the function references, or
  hint-caller       from its only caller. Never used as a name.

Conflicts: a candidate whose name is already the base name of another address (gta-reversed hook,
RenderWare name) is dropped when that other address is a RenderWare/plugin-sdk wrapper target with
a different identity (e.g. plugin-sdk's `RwTextureRead` wrapper of 0x4c7510), otherwise kept as
medium with the other address in the evidence (overloads). Disagreeing candidates for one address
keep the most reliable one at medium and list the others.

Exit status: 3 when names-extra.csv changed (run_all.sh reruns the catalogue loop so that
subsystems.py / callgraph.py pick the names up), 0 when unchanged.
Run with .venv/bin/python -I. Method and counts: docs/re/cat6-names.md.
"""
import bisect
import collections
import csv
import io
import re
import struct
import sys
from pathlib import Path

import capstone
import pefile
from capstone import x86_const as X

sys.path.insert(0, str(Path(__file__).resolve().parent))
import gr_deviations as G  # noqa: E402  (lexer + scope parser)
import subsystems as SUB  # noqa: E402
from vtables import VTables  # noqa: E402

ROOT = Path(__file__).resolve().parents[2]
BASE = 0x400000
GAME_CODE = (0x401000, 0x858000)
ATEXIT = 0x821D1E          # _atexit (CRT, Ghidra FID)
PURECALL = 0x82263A
VEC_CTOR = None            # `eh vector constructor iterator` is found by name below
FSTORE = ('fst', 'fstp', 'fist', 'fistp', 'fisttp', 'fbstp')
COLS = ['address', 'name', 'role', 'subsystem', 'source', 'confidence', 'evidence']
VFUNC_RE = re.compile(r'::vfunc_\d+$')
ADDR_RE = re.compile(r'\b0x([0-9A-Fa-f]{6,7})\b')
DATA = [(0x858000, 0x8A4000), (0x8A4000, 0xC9E000), (0xC9E000, 0xCAF000), (0xCAF000, 0xCB0000)]


def hx(a):
    return f'{a:#08x}'


def in_data(a):
    return any(lo <= a < hi for lo, hi in DATA)


# ============================================================================ catalogue inputs
def load_rw_names(cat):
    p = cat / 'rw-names.csv'
    if not p.exists():
        return {}
    return {int(r['address'], 16): r for r in csv.DictReader(open(p))}


def load_names_extra(cat):
    p = cat / 'names-extra.csv'
    if not p.exists():
        return {}
    return {int(r['address'], 16): r for r in csv.DictReader(open(p))}


def final_name(p, cls_row, rw, extra):
    """(name, source, confidence, low_hint) for a procedures.csv row. Precedence: gta-reversed hook
    name, RenderWare name (rw-names.csv high/medium), CAT-5 name (plugin-sdk, validated vtable
    slot), CAT-6 names-extra (high/medium), CAT-5 bare `vfunc_N` name. Low-confidence names are
    returned as a hint only."""
    hint = ''
    if p.get('gr_name'):
        return p['gr_name'], 'gta-reversed', 'high', hint
    if rw and rw['confidence'] in ('high', 'medium'):
        return rw['name'], 'rw-names', rw['confidence'], hint
    if rw and rw['confidence'] == 'low':
        hint = f"{rw['name']} (rw-names, low)"
    c5 = cls_row['name'] if cls_row else ''
    if c5 and not VFUNC_RE.search(c5):
        return c5, 'cat5', 'high', hint
    if extra and extra['confidence'] in ('high', 'medium'):
        return extra['name'], 'cat6:' + extra['source'], extra['confidence'], hint
    if extra and not hint:
        hint = f"{extra['name']} (names-extra {extra['source']}, low)"
    if c5:
        return c5, 'cat5', 'high', hint
    return '', '', '', hint


class Image:
    def __init__(self, exe):
        self.pe = pefile.PE(str(exe), fast_load=True)
        self.img = self.pe.get_memory_mapped_image()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True

    def rd(self, va, n):
        return bytes(self.img[va - BASE:va - BASE + n])

    def u32(self, va):
        return struct.unpack_from('<I', self.img, va - BASE)[0]

    def insns(self, lo, hi):
        return list(self.md.disasm(self.rd(lo, hi - lo), lo))

    def cstr(self, va, n=96):
        b = self.rd(va, n)
        z = b.find(b'\0')
        if z <= 0:
            return None
        s = b[:z]
        if all(32 <= c < 127 for c in s):
            return s.decode()
        return None


# ============================================================================ source 1: sources
def owner_of(fn):
    q = [x for x in fn['qual'] if x]
    if q:
        return '::'.join(q)
    return fn['scope'][-1] if fn['scope'] else ''


def qual_name(owner, name):
    name = re.sub(r'\s+', '', name)
    if name.startswith('operator'):
        name = 'operator' + (' ' + name[8:] if name[8:9].isalpha() else name[8:])
    return f'{owner}::{name}' if owner else name


def source_candidates(root, dirs, tag, entries):
    """[(addr, name, kind, confidence, location)] from one source tree."""
    out = []
    files = []
    for d in dirs:
        for p in sorted(Path(root, d).rglob('*')):
            if p.suffix in ('.cpp', '.h', '.hpp') and p.is_file():
                files.append(p)
    for p in files:
        rel = f'{tag}:{p.relative_to(root)}'
        text = p.read_text(errors='replace')
        code, comments = G.lex(text)
        active, _ = G.preprocess(code)
        funcs, classes = G.parse_scopes(code, active)
        starts = collections.defaultdict(list)
        for f in funcs:
            starts[f['start']].append(f)

        def class_at(ln):
            best = None
            for c in classes:
                if c['start'] <= ln <= c['end'] and (best is None or c['start'] >= best['start']):
                    best = c
            return best['name'] if best else ''

        # (a) one-statement wrappers
        for f in funcs:
            body = '\n'.join(code[f['open'] - 1:f['end']])
            raw = '\n'.join(text.split('\n')[f['open'] - 1:f['end']])
            k = body.find('{')
            body = body[k + 1:body.rfind('}')] if k >= 0 else body
            if body.count(';') != 1:
                continue
            addrs = {int(m.group(1), 16) for m in ADDR_RE.finditer(body)}
            fa = [a for a in addrs if a in entries]
            if len(fa) != 1 or len(addrs) != 1:
                continue
            a = fa[0]
            if not re.search(r'(?:Call\w*\s*<[^;]*?|\)\s*)0x0*%x\b' % a, body, re.I):
                continue
            nm = f['name']
            if nm.startswith('operator') and re.match(r'operator\s*(new|delete)', nm):
                continue  # allocator forwarders name the allocator, not the target
            own = owner_of(f)
            if own and nm == own.split('::')[-1]:
                pass
            out.append((a, qual_name(own, nm), 'src-wrapper', 'high', f'{rel}:{f["start"]}'))
            del raw
        # (b) address comments
        for ln in sorted(comments):
            for c in comments[ln]:
                m = re.match(r'^\s*(?:\*\s*)?(?:@addr(?:ess)?\s*:?\s*)?0x([0-9A-Fa-f]{6,7})\s*(?:$|[,;\-:(]|\s)', c)
                conv = re.search(r'Converted from .*?([\w:~]+)\s*\((?:[^()]|\([^()]*\))*\)\s*(?:const\s*)?'
                                 r'0x([0-9A-Fa-f]{6,7})\b', c)
                if conv:
                    a = int(conv.group(2), 16)
                    if a in entries:
                        out.append((a, conv.group(1), 'src-converted', 'high', f'{rel}:{ln}'))
                    continue
                if not m:
                    continue
                if len(ADDR_RE.findall(c)) != 1:
                    continue
                a = int(m.group(1), 16)
                if a not in entries:
                    continue
                line = code[ln - 1] if ln - 1 < len(code) else ''
                # same line: a declaration or definition `name(`
                if line.strip():
                    if ln in starts:
                        f = starts[ln][0]
                        out.append((a, qual_name(owner_of(f), f['name']), 'src-addr-comment', 'high',
                                    f'{rel}:{ln}'))
                        continue
                    h = G.strip_template(' '.join(line.split()))
                    pp = G.first_call_paren(h)
                    if pp < 0 or '=' in h[:pp]:
                        continue
                    nm = G.NAME_BEFORE_PAREN.search(h[:pp])
                    if not nm:
                        continue
                    nm = re.sub(r'\s+', '', nm.group(1))
                    nm = re.sub(r'<[^<>]*>', '', nm)
                    parts = nm.split('::')
                    own = '::'.join(parts[:-1]) or class_at(ln)
                    if not own and p.suffix != '.cpp' and not re.search(r'\)\s*[;{]|\)\s*(?:const|override|noexcept|=)', h):
                        continue
                    out.append((a, qual_name(own, parts[-1]), 'src-addr-comment', 'high', f'{rel}:{ln}'))
                    continue
                # comment line(s) above a definition: only comments / blank / template lines between
                for k in range(ln + 1, ln + 13):
                    if k in starts:
                        f = starts[k][0]
                        conf = 'high' if k == ln + 1 else 'medium'
                        out.append((a, qual_name(owner_of(f), f['name']), 'src-addr-comment', conf,
                                    f'{rel}:{ln}'))
                        break
                    t = code[k - 1].strip() if k - 1 < len(code) else ''
                    if t and not t.startswith('template') and not t.startswith('#'):
                        break
        # (c) #define FUNC_Class__Name 0xADDR
        for ln, line in enumerate(code, 1):
            m = re.match(r'\s*#\s*define\s+FUNC_(\w+?)__(\w+)\s+0x([0-9A-Fa-f]{6,7})\s*$', line)
            if m and int(m.group(3), 16) in entries:
                out.append((int(m.group(3), 16), f'{m.group(1)}::{m.group(2)}', 'src-define', 'high',
                            f'{rel}:{ln}'))
    return out


def norm_ctor(name):
    """`C::C` -> constructor, `C::~C` -> destructor (kept as written)."""
    return name


# ============================================================================ main
def main():
    cat = Path(sys.argv[1])
    exe = Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
    gr = Path(sys.argv[3]) if len(sys.argv) > 3 else ROOT / 're' / 'gta-reversed'
    psdk = Path(sys.argv[4]) if len(sys.argv) > 4 else ROOT / 're' / 'plugin-sdk'
    import callgraph as CG  # noqa: E402  (global_names, gr_bases)

    im = Image(exe)
    procs = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'procedures.csv'))}
    cls = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'classification.csv'))}
    rw = load_rw_names(cat)
    hooks = {}
    for h in csv.DictReader(open(cat / 'functions.csv')):
        hooks.setdefault(int(h['address'], 16), h)
    entries = set(procs)
    sorted_entries = sorted(entries)
    ext = {}
    for r in csv.DictReader(open(cat / 'extents.csv')):
        ext[int(r['address'], 16)] = [tuple(int(x, 16) for x in rg.split('-')) for rg in r['ranges'].split(';')
                                      if rg]

    def klass(a):
        p = procs[a]
        if p['state'] == 'excluded':
            return 'excluded'
        c = cls.get(a)
        return c['class'] if c else 'game'

    def base_name(a):
        p = procs[a]
        if p['gr_name']:
            return p['gr_name']
        r = rw.get(a)
        if r and r['confidence'] in ('high', 'medium'):
            return r['name']
        c = cls.get(a)
        return c['name'] if c else ''

    live = [a for a in sorted_entries if klass(a) != 'excluded']
    target = [a for a in live if klass(a) != 'RW' and a not in rw
              and (not base_name(a) or VFUNC_RE.search(base_name(a)))]
    tset = set(target)
    names_used = collections.defaultdict(set)
    for a in live:
        n = base_name(a)
        if n and not VFUNC_RE.search(n):
            names_used[n].add(a)
    rw_name_addr = {r['name']: a for a, r in rw.items() if r['confidence'] in ('high', 'medium')}

    def sub_of_name(name):
        o = name.split('::')[0] if '::' in name else ''
        if o:
            for sid, _c, rx in SUB.COMPILED:
                if rx.match(o):
                    return sid
        return ''

    def proc_sub(a):
        """Subsystem of a function from its gta-reversed hook (independent of CAT-6)."""
        h = hooks.get(a)
        if h:
            return SUB.classify(h['category'], h['owner'], h['name'])
        return ''

    cands = collections.defaultdict(list)   # addr -> [(name, source, conf, evidence, role, sub)]
    rejected = []

    # ------------------------------------------------------------------ 1. sources
    src = source_candidates(psdk, ['plugin_sa/game_sa'], 'plugin-sdk', entries) + \
        source_candidates(gr, ['source/game_sa', 'source/app'], 'gta-reversed', entries)
    by_addr = collections.defaultdict(list)
    for a, nm, kind, conf, loc in src:
        if a in tset:
            by_addr[a].append((nm, kind, conf, loc))
    order = {'src-wrapper': 0, 'src-converted': 1, 'src-define': 2, 'src-addr-comment': 3}
    for a in sorted(by_addr):
        lst = sorted(set(by_addr[a]), key=lambda x: (x[3].startswith('plugin-sdk'), order[x[1]],
                                                     x[2] != 'high', x[3]))
        good = []
        for nm, kind, conf, loc in lst:
            other = rw_name_addr.get(nm)
            if other is not None and other != a:
                rejected.append((a, nm, f'{kind} {loc}: name belongs to RenderWare function {other:#x} '
                                        f'(rw-names.csv)'))
                continue
            used = names_used.get(nm, set()) - {a}
            if used and kind == 'src-wrapper':
                # a hooked function of that name exists elsewhere: this body calls a helper
                rejected.append((a, nm, f'{kind} {loc}: {nm} is the name of {",".join(hx(u) for u in sorted(used))}'
                                        f' (the wrapper reimplements that function and calls this one)'))
                continue
            good.append((nm, kind, conf, loc, used))
        if not good:
            continue
        names = []
        for g in good:
            if g[0] not in names:
                names.append(g[0])
        nm, kind, conf, loc, used = good[0]
        locs = [g[3] for g in good if g[0] == nm]
        ev = f'{kind} {"; ".join(locs[:3])}' + (f' (+{len(locs) - 3})' if len(locs) > 3 else '')
        if len(names) > 1:
            conf = 'medium'
            ev += '; other source names: ' + ', '.join(
                f'{n} ({next(g[3] for g in good if g[0] == n)})' for n in names[1:4])
        if used:
            conf = 'medium'
            ev += f'; same name as {",".join(hx(u) for u in sorted(used))} (overload?)'
        if all(l.startswith('plugin-sdk') for l in locs) and conf == 'high':
            # plugin-sdk names that also exist in gta-reversed disagree in ~2 % of cases, a few of
            # them swapped pairs (CRestart, CPostEffects, CMessages): plugin-sdk alone is medium
            conf = 'medium'
            ev += '; plugin-sdk only'
        cands[a].append((nm, kind, conf, ev, '', sub_of_name(nm)))

    # ------------------------------------------------------------------ 2. static initialisers
    gnames = CG.global_names(gr, psdk, in_data)
    gsorted = sorted(gnames)
    data_rows = list(csv.DictReader(open(cat / 'data-code-addresses.csv')))
    inits = sorted(int(r['address'], 16) for r in data_rows if r['kind'].startswith('crt-init'))
    # functions that use each global (for the subsystem of an unnamed global)
    users = collections.defaultdict(set)
    for r in csv.DictReader(open(cat / 'rwsets.csv')):
        users[int(r['address'], 16)].add(int(r['function'], 16))

    def gname(addr):
        if addr in gnames:
            return gnames[addr][0], gnames[addr][1], ''
        i = bisect.bisect_right(gsorted, addr) - 1
        if i >= 0 and addr - gsorted[i] < 0x400:
            return None, gnames[gsorted[i]][1], f'{gnames[gsorted[i]][0]}+{addr - gsorted[i]:#x}'
        return None, '', ''

    def body_insns(a):
        out = []
        for lo, hi in ext.get(a, []):
            if GAME_CODE[0] <= lo < GAME_CODE[1]:
                out += im.insns(lo, hi)
        return out

    def init_facts(a):
        """(global, how, dtor callbacks, ctor called) of a static initialiser (or of a destructor
        callback: the object passed as `this` / first argument)."""
        ins = body_insns(a)
        regs = {}                     # register -> data address loaded as an immediate
        pushes = []
        glob_ = how = ctor = None
        dtors = []
        for i in ins:
            ops = i.operands
            if i.mnemonic == 'mov' and len(ops) == 2 and ops[0].type == X.X86_OP_REG and \
                    ops[1].type == X.X86_OP_IMM and in_data(ops[1].imm & 0xFFFFFFFF):
                regs[ops[0].reg] = ops[1].imm & 0xFFFFFFFF
                continue
            if i.mnemonic == 'lea' and ops[1].type == X.X86_OP_MEM and ops[1].mem.base == 0 \
                    and ops[1].mem.index == 0 and in_data(ops[1].mem.disp & 0xFFFFFFFF):
                regs[ops[0].reg] = ops[1].mem.disp & 0xFFFFFFFF
                continue
            if i.mnemonic == 'push' and ops and ops[0].type == X.X86_OP_IMM:
                pushes.append(ops[0].imm & 0xFFFFFFFF)
                continue
            if i.mnemonic in ('call', 'jmp') and ops and ops[0].type == X.X86_OP_IMM:
                t = ops[0].imm & 0xFFFFFFFF
                ecx = regs.get(X.X86_REG_ECX)
                if t == ATEXIT and pushes and GAME_CODE[0] <= pushes[-1] < GAME_CODE[1]:
                    dtors.append(pushes[-1])
                elif glob_ is None and t in procs and procs[t]['ghidra_name'].startswith(
                        ('`eh_vector_constructor', '`eh_vector_destructor', '`eh vector')):
                    dp = [v for v in pushes if in_data(v)]
                    if dp:
                        glob_, how, ctor = dp[-1], f'array passed to {procs[t]["ghidra_name"]} at {i.address:#x}', t
                elif glob_ is None and ecx is not None and (i.mnemonic == 'call' or GAME_CODE[0] <= t < GAME_CODE[1]):
                    glob_, how, ctor = ecx, f'this of the {i.mnemonic} {t:#x} at {i.address:#x}', t
                elif glob_ is None and pushes and in_data(pushes[-1]) and i.mnemonic == 'call':
                    glob_, how, ctor = pushes[-1], f'first argument of the call {t:#x} at {i.address:#x}', t
                pushes = []
                regs.pop(X.X86_REG_EAX, None), regs.pop(X.X86_REG_ECX, None), regs.pop(X.X86_REG_EDX, None)
                continue
            if glob_ is None and ops and ops[0].type == X.X86_OP_MEM and ops[0].mem.index == 0 and \
                    i.mnemonic not in ('cmp', 'test', 'push') and (len(ops) > 1 or i.mnemonic in FSTORE):
                m = ops[0].mem
                if m.base == 0 and in_data(m.disp & 0xFFFFFFFF):
                    glob_, how = m.disp & 0xFFFFFFFF, f'first store `{i.mnemonic} {i.op_str}` at {i.address:#x}'
                elif m.base in regs and in_data((regs[m.base] + m.disp) & 0xFFFFFFFF):
                    glob_, how = (regs[m.base] + m.disp) & 0xFFFFFFFF, \
                        f'first store `{i.mnemonic} {i.op_str}` at {i.address:#x} ({regs[m.base]:#x} in {i.reg_name(m.base)})'
            for r_ in i.regs_access()[1]:
                regs.pop(r_, None)
        return glob_, how, dtors, ctor

    def ident(s):
        return re.sub(r'[^\w]', '_', s.replace('::', '__')).strip('_')

    init_dtor, dtor_of = {}, {}

    def describe(g):
        nm, where, near = gname(g)
        if nm:
            label = ident(nm)
            gev = f'global {nm} ({where})'
            sub = sub_of_name(nm) or sub_of_name(Path(where.split(':', 1)[-1]).stem + '::x')
        else:
            label = f'g_{g:x}'
            gev = f'global {g:#x} (unnamed' + (f'; nearest named global below: {near} ({where}), extent unknown' if near else '') + ')'
            sub = ''   # the nearest named global is not proof of containment: no subsystem from it
        if not sub:
            us = [u for u in users.get(g, ()) if u not in initset and proc_sub(u)]
            ss = {proc_sub(u) for u in us}
            if len(ss) == 1:
                sub = ss.pop()
                gev += f'; subsystem from its {len(us)} gta-reversed user(s)'
        return label, gev, sub

    initset = set(inits)
    for a in inits:
        g, how, dtors, ctor = init_facts(a)
        for d in dtors:
            dtor_of.setdefault(d, a)
        if g is None and dtors:
            g, dh, _x, _c = init_facts(dtors[0])
            how = f'object of its atexit destructor callback {dtors[0]:#x} ({dh})'
        if g is None:
            if a in tset:
                cands[a].append((f'__init_{a:x}', 'init', 'high',
                                 'C++ dynamic initialiser in __xc_a..__xc_z (no global store found: '
                                 'calls only)', 'init', ''))
            continue
        label, gev, sub = describe(g)
        ev = f'C++ dynamic initialiser (__xc table) of {gev}; {how}'
        if a in tset:
            cands[a].append((f'__init_{label}', 'init', 'high', ev, 'init', sub))
        for d in dtors:
            init_dtor.setdefault(d, (label, a, sub, gev))
    for d, a in sorted(dtor_of.items()):
        if d in init_dtor or d not in tset:
            continue
        g, how, _x, _c = init_facts(d)
        if g is None:
            cands[d].append((f'__dtor_{a:x}', 'dtor', 'high', f'atexit destructor callback registered by the '
                             f'initialiser {a:#x} (no object found in its body)', 'dtor', ''))
            continue
        label, gev, sub = describe(g)
        init_dtor[d] = (label, a, sub, gev + f'; object from the callback itself: {how}')
    for d, (label, a, sub, gev) in sorted(init_dtor.items()):
        if d in tset:
            cands[d].append((f'__dtor_{label}', 'dtor', 'high',
                             f'atexit destructor callback registered by the initialiser {a:#x} of {gev}', 'dtor', sub))

    # ------------------------------------------------------------------ 3. unknown vtables
    vts = VTables(exe, gr)
    bases = CG.gr_bases(gr)
    hdr_vt = {}
    for f in sorted(gr.glob('source/**/*.h')):
        for m in re.finditer(r'RH_ScopedVirtualClass\(\s*([\w:<>, ]+?)\s*,\s*(0x[0-9A-Fa-f]+)\s*,\s*(\d+)\s*\)',
                             f.read_text(errors='replace')):
            if not m.group(1).startswith('cls'):
                hdr_vt[int(m.group(2), 16)] = (m.group(1), int(m.group(3)), str(f.relative_to(gr)))
    tables = [r for r in csv.DictReader(open(cat / 'data-tables.csv')) if r['kind'] == 'vtable']
    gr_vt = {v: c for c, (v, _n) in vts.vt.items()}
    # every vtable (gta-reversed + unknown) and its slots, to find slots unique to one table
    all_vt = {}
    for r in csv.DictReader(open(cat / 'data-tables.csv')):
        if r['kind'] == 'vtable' or r['kind'].startswith('vtable-gr'):
            s, e = int(r['start'], 16), int(r['end'], 16)
            all_vt[s] = [im.u32(x) for x in range(s, e, 4)]
    slot_tables = collections.defaultdict(set)
    for v, sl in all_vt.items():
        for x in sl:
            slot_tables[x].add(v)
    # vtable installs: `mov dword ptr [reg(+disp)], imm32` anywhere in live bodies
    vset = {int(r['start'], 16) for r in tables}
    installs = collections.defaultdict(list)    # vtable -> [(function, site, disp, index among installs)]
    fn_installs = collections.defaultdict(list)  # function -> [(site, vtable, disp)]
    text = im.rd(GAME_CODE[0], GAME_CODE[1] - GAME_CODE[0])
    starts_sorted = sorted(ext)
    allv = set(all_vt)
    for v in sorted(allv):
        pat = struct.pack('<I', v)
        pos = text.find(pat)
        while pos >= 0:
            va = GAME_CODE[0] + pos
            for k in (2, 3, 6, 7):
                for i in im.md.disasm(im.rd(va - k, 16), va - k, 1):
                    if i.mnemonic == 'mov' and i.size == k + 4 and len(i.operands) == 2 and \
                            i.operands[0].type == X.X86_OP_MEM and i.operands[0].mem.base and \
                            i.operands[1].type == X.X86_OP_IMM and (i.operands[1].imm & 0xFFFFFFFF) == v:
                        j = bisect.bisect_right(starts_sorted, i.address) - 1
                        if j >= 0:
                            f = starts_sorted[j]
                            if any(lo <= i.address < hi for lo, hi in ext[f]):
                                fn_installs[f].append((i.address, v, i.operands[0].mem.disp))
            pos = text.find(pat, pos + 1)
    for f in fn_installs:
        lst = sorted(set(fn_installs[f]))
        fn_installs[f] = lst
        prim = [x for x in lst if x[2] == 0]
        for n, (site, v, disp) in enumerate(lst):
            installs[v].append((f, site, disp, prim.index((site, v, disp)) if disp == 0 else -1, len(prim)))

    CTOR = re.compile(r'^(?:(.+)::)?(\w+)$')

    def role_of(name):
        """('ctor'|'dtor', class) for a constructor / destructor name."""
        m = CTOR.match(name)
        if not m:
            return None
        own, nm = m.group(1), m.group(2)
        if own is None:
            return ('ctor', nm) if re.match(r'^C[A-Z]\w+$', nm) else None
        last = own.split('::')[-1]
        if nm in ('Constructor', last) or re.match(r'Constructor\d*$', nm):
            return ('ctor', last)
        if nm in ('Destructor', '~' + last):
            return ('dtor', last)
        return None

    def known_name(a):
        n = base_name(a)
        if n and not VFUNC_RE.search(n):
            return n
        for c in cands.get(a, []):
            if c[2] == 'high' and c[1].startswith('src-'):
                return c[0]
        return ''

    vt_class = {}
    for r in sorted(tables, key=lambda r: int(r['start'], 16)):
        v = int(r['start'], 16)
        slots = all_vt[v]
        votes = []
        if v in hdr_vt:
            votes.append((hdr_vt[v][0], 'high', f'RH_ScopedVirtualClass in {hdr_vt[v][2]}'))
        for f, site, disp, idx, nprim in installs.get(v, []):
            if disp != 0:
                continue
            nm = known_name(f)
            rl = role_of(nm.replace('~', '~')) if nm else None
            if not rl and '::~' in nm:
                rl = ('dtor', nm.split('::~')[-1])
            if not rl:
                continue
            kind, c = rl
            # a constructor always stores its own vtable last; a destructor stores its own first,
            # but MSVC drops that store when the body calls no virtual, so a lone store proves nothing
            if (kind == 'ctor' and idx == nprim - 1) or (kind == 'dtor' and idx == 0 and nprim >= 2):
                votes.append((c, 'high', f'{"constructor" if kind == "ctor" else "destructor"} {nm} {f:#x} '
                                         f'installs it at {site:#x} ({"last" if kind == "ctor" else "first"} '
                                         f'of {nprim} vtable stores)'))
        s0 = slots[0] if slots else 0
        if s0 in ext:
            for i in body_insns(s0)[:24]:
                if i.mnemonic == 'call' and i.operands[0].type == X.X86_OP_IMM:
                    t = i.operands[0].imm & 0xFFFFFFFF
                    nm = known_name(t) if t in procs else ''
                    rl = role_of(nm) if nm else None
                    if not rl and '::~' in nm:
                        rl = ('dtor', nm.split('::~')[-1])
                    if rl and rl[0] == 'dtor' and not any(x[1] == v for x in fn_installs.get(s0, [])):
                        votes.append((rl[1], 'high', f'slot 0 {s0:#x} (scalar deleting destructor) calls '
                                                     f'{nm} {t:#x} and installs no vtable itself'))
                    break
        if not votes:
            owners = collections.Counter()
            for x in slots:
                if slot_tables[x] == {v} and x in procs:
                    nm = known_name(x)
                    if '::' in nm and not nm.startswith('__'):
                        owners[nm.rsplit('::', 1)[0]] += 1
            if len(owners) == 1:
                c, n = owners.most_common(1)[0]
                votes.append((c, 'medium', f'{n} named slot(s) unique to this vtable belong to {c}'))
        cs = sorted({c for c, _cf, _e in votes})
        if len(cs) == 1:
            conf = 'high' if any(cf == 'high' for _c, cf, _e in votes) else 'medium'
            vt_class[v] = (cs[0], conf, '; '.join(sorted({e for _c, _cf, e in votes})))
        elif len(cs) > 1:
            rejected.append((v, '/'.join(cs), 'vtable class votes disagree: ' +
                             '; '.join(f'{c}: {e}' for c, _cf, e in votes)))

    # base class of each unknown vtable: a gta-reversed vtable sharing >= 2 slots at the same index
    def gr_base(v):
        """(class A, len(layout(A)), evidence) for the deepest validated gta-reversed class A that the
        unknown vtable provably derives from: it shares (same index, same code) at least one slot
        that is specific to A (A's code differs from its parent's there, or A adds the slot).
        Sharing only inherited slots proves nothing about A (siblings share them too)."""
        slots = all_vt[v]
        best = None
        for gv, c in gr_vt.items():
            if c not in vts.layout:
                continue
            n = len(vts.layout[c])
            if n > len(slots):
                continue
            same = sum(1 for i in range(n) if im.u32(gv + 4 * i) == slots[i])
            if same >= 2 and (best is None or (n, same, -gv) > (best[1], best[2], -best[3])):
                best = (c, n, same, gv)
        if not best:
            return None
        a = best[0]
        while a:
            if a not in vts.layout or a not in vts.vt:
                return None
            av, an = vts.vt[a][0], len(vts.layout[a])
            par = vts.decl.get(a, (None, []))[0]
            pv = vts.vt[par][0] if par in vts.vt and par in vts.layout else None
            pn = len(vts.layout[par]) if pv else 0
            spec = [i for i in range(an) if i >= pn or im.u32(av + 4 * i) != im.u32(pv + 4 * i)]
            hit = [i for i in spec if im.u32(av + 4 * i) == slots[i]]
            if hit:
                return a, an, f'shares {len(hit)} slot(s) specific to {a} (e.g. slot {hit[0]})'
            if pv is None:
                return None
            a = par
        return None

    for r in sorted(tables, key=lambda r: int(r['start'], 16)):
        v = int(r['start'], 16)
        slots = all_vt[v]
        b = gr_base(v)
        c = vt_class.get(v)
        if c:
            cname, conf, cev = c
        elif b:
            cname, conf, cev = f'vt_{v:x}', 'medium', f'class unknown; derived from {b[0]}: {b[2]}'
        else:
            continue
        lay = None
        lev = ''
        short = cname.split('<')[0]
        if c and short in vts.decl:
            lr = vts._layout(short)
            if not isinstance(lr, str) and len(lr[0]) == len(slots):
                lay = lr[0]
                lev = f'layout of {short} from gta-reversed headers ({len(lay)} slots = table)'
        if lay is None and b:
            lay = vts.layout[b[0]] + [None] * (len(slots) - b[1])
            lev = f'first {b[1]} slots named after base {b[0]} (validated layout; {b[2]})'
        for i, x in enumerate(slots):
            if x not in tset:
                continue
            nm = lay[i] if lay and i < len(lay) and lay[i] else None
            if nm == '~':
                nm = '~' + short.split('::')[-1] if c else 'Destructor'
            if nm and nm != 'operator?':
                name = f'{cname}::{nm}'
            else:
                name = f'{cname}::vfunc_{i}'
            ev = f'slot {i} of vtable {v:#x}; class {cname}: {cev}' + (f'; {lev}' if nm else '')
            if len(slot_tables[x]) > 1:
                ev += f'; also in {len(slot_tables[x]) - 1} other vtable(s)'
            cands[x].append((name, 'vtable-class', conf if (c or nm) else 'medium', ev, '', sub_of_name(
                short + '::x') or (sub_of_name(b[0] + '::x') if b else '')))

    # ------------------------------------------------------------------ 5. low hints
    strings_of = collections.defaultdict(set)
    for r in csv.DictReader(open(cat / 'rwsets.csv')):
        if r['section'] == '.rdata' and r['access'] == 'A':
            fa = int(r['function'], 16)
            if fa in tset:
                s = im.cstr(int(r['address'], 16))
                if s and len(s) >= 4 and re.search(r'[A-Za-z]{3}', s):
                    strings_of[fa].add(s)
    callers = collections.defaultdict(set)
    for r in csv.DictReader(open(cat / 'callgraph.csv')):
        if r['kind'] in ('call', 'tailcall') and r['callee'].startswith('0x'):
            callers[int(r['callee'], 16)].add(int(r['caller'], 16))
    for a in target:
        if cands.get(a):
            continue
        if len(strings_of.get(a, ())) == 1:
            s = next(iter(strings_of[a]))
            cands[a].append((f'sub_{a:x}_{ident(s)[:40]}', 'hint-string', 'low',
                             f'references the one string "{s[:60]}"', '', ''))
        elif len(callers.get(a, ())) == 1:
            c = next(iter(callers[a]))
            n = base_name(c) or ''
            if n and not VFUNC_RE.search(n):
                cands[a].append((f'sub_{a:x}_from_{ident(n)[:48]}', 'hint-caller', 'low',
                                 f'only caller {c:#x} {n}', '', ''))

    # ------------------------------------------------------------------ resolve + write
    rank = {'init': 0, 'dtor': 0, 'src-wrapper': 1, 'src-converted': 2, 'src-define': 3,
            'src-addr-comment': 5, 'vtable-class': 6, 'hint-string': 8, 'hint-caller': 9}

    def rk(c):
        # a slot name from a validated class layout is checked against the bytes; it beats an
        # address comment (gta-reversed's TaskComplexClimb.h has Clone/GetTaskType comments swapped)
        r = rank[c[1]]
        if c[1] == 'vtable-class' and c[2] == 'high' and 'layout' in c[3] and '::vfunc_' not in c[0]:
            r = 4
        return (c[2] == 'low', c[2] != 'high', r, c[0])
    rows = []
    for a in sorted(cands):
        lst = sorted(cands[a], key=rk)
        nm, src_, conf, ev, role, sub = lst[0]
        others = [c for c in lst[1:] if c[0] != nm and c[2] != 'low']
        if others:
            ev += '; other candidates: ' + ', '.join(f'{c[0]} ({c[1]})' for c in others[:3])
            if lst[0][1] not in ('init', 'dtor') and conf == 'high' and not (
                    src_.startswith('src-') and all(c[1] == 'vtable-class' and c[0].split('::')[-1].startswith('vfunc_')
                                                    for c in others)):
                conf = 'medium'
        rows.append({'address': hx(a), 'name': nm, 'role': role, 'subsystem': sub, 'source': src_,
                     'confidence': conf, 'evidence': ev})
    buf = io.StringIO()
    w = csv.DictWriter(buf, COLS, lineterminator='\n')
    w.writeheader()
    w.writerows(rows)
    new = buf.getvalue()
    path = cat / 'names-extra.csv'
    old = path.read_text() if path.exists() else ''
    if new != old:
        path.write_text(new)
    rbuf = io.StringIO()
    rw_ = csv.writer(rbuf, lineterminator='\n')
    rw_.writerow(['address', 'name', 'reason'])
    for a, nm, why in sorted(set(rejected)):
        rw_.writerow([hx(a), nm, why])
    (cat / 'raw').mkdir(exist_ok=True)
    (cat / 'raw' / 'names-extra-rejected.csv').write_text(rbuf.getvalue())
    cnt = collections.Counter((r['source'], r['confidence']) for r in rows)
    for k, n in sorted(cnt.items()):
        print(f'{n:6d}  {k[0]} {k[1]}')
    print(f'names_extra: {len(target)} target functions, {len(rows)} rows '
          f'({sum(1 for r in rows if r["confidence"] != "low")} high/medium), {len(rejected)} rejected, '
          f'{len(vt_class)}/{len(tables)} unknown vtables with a class; '
          f'{"changed" if new != old else "unchanged"}')
    return 3 if new != old else 0


def refresh_rw_tags(cat):
    """One-off maintenance (not run by run_all.sh): rewrite the leading `[catalogue: SUBSYSTEM/state]`
    / `[not in catalogue]` tag of every catalog/rw-names.csv evidence from the current state.csv.
    Nothing else in the file changes."""
    st = {int(r['address'], 16): r for r in csv.DictReader(open(cat / 'state.csv'))}
    path = cat / 'rw-names.csv'
    rows = list(csv.DictReader(open(path)))
    cols = list(rows[0].keys())
    n = 0
    for r in rows:
        a = int(r['address'], 16)
        tag = f"[catalogue: {st[a]['subsystem']}/{st[a]['state']}]" if a in st else '[not in catalogue]'
        new = re.sub(r'^\[(?:catalogue: [^\]]*|not in catalogue)\]', tag, r['evidence'], count=1)
        if new != r['evidence']:
            r['evidence'] = new
            n += 1
    eol = '\r\n' if b'\r\n' in path.read_bytes()[:4096] else '\n'
    buf = io.StringIO()
    w = csv.DictWriter(buf, cols, lineterminator=eol)
    w.writeheader()
    w.writerows(rows)
    path.write_bytes(buf.getvalue().encode())
    print(f'refresh-rw-tags: {n} of {len(rows)} evidence tags updated')


if __name__ == '__main__':
    if len(sys.argv) > 1 and sys.argv[1] == 'refresh-rw-tags':
        refresh_rw_tags(Path(sys.argv[2]) if len(sys.argv) > 2 else ROOT / 'catalog')
        sys.exit(0)
    sys.exit(main())
