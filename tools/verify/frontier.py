#!/usr/bin/env python3
"""The phase B porting frontier: unported functions that already-ported code can reach.

Usage: python3 -I tools/verify/frontier.py [--all] [--csv FILE] [--limit N] [--direct|--virtual|--imports|--partial]
                                           [--no-branches] [--stays-original]

Four lists (a native run "blocks" on any of them when it reaches it):

  direct   every address an `orig!` declaration in crates/ calls with `=> unported` in a literal address
           form (`= CONV 0xADDR`, `= reg("...") 0xADDR`, and `= CONV table CELL OFF`, whose slot is read
           from the exe) that nothing registers.
  virtual  every slot target of a catalogued vtable (catalog/data-tables.csv rows whose kind starts with
           `vtable`; slots read from re/gta/1.0-US/gta_sa.exe) whose catalogue state (catalog/state.csv)
           is unknown/specified and that nothing registers.
  import   (FRONTIER-2 (a)) every Win32 import an `orig!` declaration calls `=> unported` through its IAT
           cell (`= CONV cell CELL`, or `= CONV site SITE` naming a `call dword ptr [CELL]`), named
           `DLL!function` from the exe's import directory, that the phase B platform layer does not implement
           (no `"DLL!function" =>` entry in crates/sa-game/src/imports.rs's `natives!` table, the table sa-game
           registers at the import's trap). Address = the IAT cell; uses = the declarations naming it.
  partial  (FRONTIER-2 (d)) the ADR-002 partial ports (`#[hook(..., fallback)]`): each one's unported cases run
           `<fn>_original`, which has no target natively (a native run reports it as `<fn>_original (0x0)`).
           Address = the hook; uses = the `<fn>_original` call sites; name = the catalogue name plus, for a
           script switch, how many of its 100 opcodes are not in its `PORTED` list.

"Registered" = a `#[hook(addr = ...)]` / `#[native(addr = ...)]` attribute (any path prefix, e.g.
`#[sa_core::native(`) anywhere under crates/, a row of the table-driven natives (`lone_rets!` in
sa-core std7_natives.rs, `static_inits!` in sa-core static_inits.rs) or one of sa-game's address
entries (crt.rs, rw_standin.rs, d3d9_shader.rs, com.rs, vorbisfile.rs). Excluded: `site` / `callsite` /
`cell` / `dyn` / `vslot` forms (imports, runtime pointers), excluded catalogue rows (subsystem X-*)
and targets outside game `.text` 0x401000-0x857000 (the no-CD protection's code).

Functions that stay original by design (`catalog/stays-original.csv`, read through tools/lib/stays_original.py:
the audio decoders, data streams, channels and threads, ...) are not work: they are left out of the lists and
counted in the summary line (`stays original N`); `--stays-original` lists them instead, with their reason.

Addresses that another branch or agent worktree already registers (tools/gen/inflight.py: master,
every worktree-agent-* branch, uncommitted agent worktrees) are marked with that branch and left out
of the to-port list; `--all` lists them too. Never hand-port those: integrate the branch's port.

Columns: kind (direct/virtual/table/import/partial), address, name, subsystem, state, callers (distinct callers in
catalog/callgraph.csv), uses (orig! declarations naming it, direct only), elsewhere. Sorted by
callers, then uses, then address.
"""
import csv
import os
import re
import struct
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
MAIN = ROOT.split('/.claude/worktrees/')[0]
sys.path.insert(0, os.path.join(ROOT, 'tools', 'gen'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'lib'))
import stays_original  # noqa: E402

HEX = r'0x[0-9A-Fa-f_]+'
CONV = r'(?:cdecl|thiscall|stdcall|fastcall|reg\(\s*"[^"]*"\s*\))'
# one orig! declaration: `fn name(...) -> R = CONV ADDR => unported;` (may span lines)
DECL = re.compile(r'\bfn\s+\w+\s*\([^;{}]*?\)\s*(?:->[^=;{}]*?)?=\s*(' + CONV + r')\s+(' + HEX +
                  r'|table\s+' + HEX + r'\s+' + HEX + r')\s*=>\s*unported\s*;', re.S)
ATTR = re.compile(r'#\[(?:[A-Za-z_]\w*::)*(?:hook|native)\(\s*addr\s*=\s*(' + HEX + r')')
ROW = re.compile(r'^\s*\(\s*(' + HEX + r')\s*,\s*"', re.M)
GAME_TEXT = (0x401000, 0x857000)
TABLE_FILES = ['crates/sa-core/src/std7_natives.rs', 'crates/sa-core/src/static_inits.rs']
GAME_FILES = ['crates/sa-game/src/' + f for f in
              ('crt.rs', 'rw_standin.rs', 'd3d9_shader.rs', 'com.rs', 'vorbisfile.rs')]


def h(s):
    return int(s.replace('_', ''), 16)


def find_input(rel):
    p = os.path.join(ROOT, rel)
    return p if os.path.exists(p) else os.path.join(MAIN, rel)


class Image:
    def __init__(self, path):
        self.data = open(path, 'rb').read()
        pe = struct.unpack_from('<I', self.data, 0x3C)[0]
        nsec = struct.unpack_from('<H', self.data, pe + 6)[0]
        opt = struct.unpack_from('<H', self.data, pe + 20)[0]
        self.base = struct.unpack_from('<I', self.data, pe + 24 + 28)[0]
        self.secs = []
        off = pe + 24 + opt
        for i in range(nsec):
            vsize, va, rsize, raw = struct.unpack_from('<IIII', self.data, off + 40 * i + 8)
            self.secs.append((self.base + va, max(vsize, rsize), raw, rsize))

    def u32(self, va):
        for start, size, raw, rsize in self.secs:
            if start <= va < start + size:
                o = va - start
                if o + 4 > rsize:
                    return 0
                return struct.unpack_from('<I', self.data, raw + o)[0]
        return None


def rust_files():
    for dirpath, dirs, files in os.walk(os.path.join(ROOT, 'crates')):
        dirs[:] = sorted(d for d in dirs if d != 'target')
        for f in sorted(files):
            if f.endswith('.rs'):
                p = os.path.join(dirpath, f)
                yield os.path.relpath(p, ROOT), open(p, encoding='utf-8', errors='replace').read()


def scan(img):
    registered, uses, kinds = set(), {}, {}
    for rel, text in rust_files():
        registered |= {h(a) for a in ATTR.findall(text)}
        if rel in TABLE_FILES or rel in GAME_FILES:
            registered |= {h(a) for a in ROW.findall(text)}
        for conv, tgt in DECL.findall(text):
            if tgt.startswith('table'):
                _, cell, off = tgt.split()
                a = img.u32(h(cell) + h(off))
                if not a:
                    continue
                kind = 'table'
            else:
                a, kind = h(tgt), 'direct'
            uses[a] = uses.get(a, 0) + 1
            kinds.setdefault(a, kind)
    return registered, uses, kinds


def import_cells(img):
    """IAT cell -> `DLL!function` (`#N` for an ordinal), from the exe's import directory (as sa-game's arena)."""
    d = img.data
    pe = struct.unpack_from('<I', d, 0x3C)[0]
    rva = struct.unpack_from('<I', d, pe + 24 + 104)[0]

    def off(r):
        va = img.base + r
        for start, size, raw, rsize in img.secs:
            if start <= va < start + size:
                return raw + va - start
        return None

    def cstr(o):
        return d[o:d.index(b'\0', o)].decode('latin-1')

    cells = {}
    o = off(rva)
    while o is not None:
        oft, _, _, name, ft = struct.unpack_from('<IIIII', d, o)
        if name == 0:
            break
        dll = cstr(off(name))
        names = off(oft if oft else ft)
        i = 0
        while True:
            t = struct.unpack_from('<I', d, names + 4 * i)[0]
            if t == 0:
                break
            f = '#%d' % (t & 0xFFFF) if t & 0x8000_0000 else cstr(off(t) + 2)
            cells[img.base + ft + 4 * i] = '%s!%s' % (dll, f)
            i += 1
        o += 20
    return cells


IMPORT_DECL = re.compile(r'\bfn\s+(\w+)\s*\([^;{}]*?\)\s*(?:->[^=;{}]*?)?=\s*' + CONV + r'\s+(cell|site)\s+(' + HEX +
                         r')\s*=>\s*unported\s*;', re.S)
NATIVE_IMPORT = re.compile(r'^\s*"([^"!]+![^"]+)"\s*=>', re.M)
HOST_KEY = re.compile(r'"((?:sa_\w+)(?:::\w+)+)"')


def import_rows(img):
    """(cell, name, uses) of every unported import declaration the platform layer does not implement."""
    cells = import_cells(img)
    impl = set(NATIVE_IMPORT.findall(open(os.path.join(ROOT, 'crates/sa-game/src/imports.rs')).read()))
    # declarations sa-game binds by path (`host::set("crate::module::fn", ...)` in bind.rs and friends)
    host = set()
    gdir = os.path.join(ROOT, 'crates', 'sa-game', 'src')
    for f in os.listdir(gdir):
        if f.endswith('.rs'):
            host |= set(HOST_KEY.findall(open(os.path.join(gdir, f)).read()))
    uses = {}
    for rel, text in rust_files():
        crate = rel.split(os.sep)[1].replace('-', '_')
        for fn, form, a in IMPORT_DECL.findall(text):
            if any(k.startswith(crate + '::') and k.endswith('::' + fn) for k in host):
                continue
            a = h(a)
            if form == 'site':
                if img.u32(a) is None:
                    continue
                cell = img.u32(a + 2)
                first = img.u32(a) & 0xFFFF
                if first != 0x15FF:
                    continue
            else:
                cell = a
            uses[cell] = uses.get(cell, 0) + 1
    out = []
    for cell, n in sorted(uses.items()):
        name = cells.get(cell)
        if name is not None and name not in impl:  # other `cell`s are runtime function pointers
            out.append((cell, name, n))
    return out


HOOK_FALLBACK = re.compile(r'#\[(?:\w+::)*hook\(((?:[^()"]|"[^"]*"|\([^()]*\))*)\)\]\s*(?:#\[[^\]]*\]\s*)*pub\s+fn\s+(\w+)',
                           re.S)


def partial_rows():
    """(address, name, original-call sites, note) of every `#[hook(..., fallback)]` port."""
    out = []
    for rel, text in rust_files():
        for m in HOOK_FALLBACK.finditer(text):
            attr, fn = m.group(1), m.group(2)
            if not re.search(r'(^|,)\s*fallback\s*(,|$)', attr):
                continue
            a = re.search(r'addr\s*=\s*(' + HEX + ')', attr)
            nm = re.search(r'name\s*=\s*"([^"]*)"', attr)
            if not a:
                continue
            calls = len(re.findall(r'\b' + fn + r'_original\s*[,()]', text)) - (1 if 'fn ' + fn + '_original' in text else 0)
            note = ''
            pm = re.search(r'pub const PORTED: &\[u16\] = &\[(.*?)\];', text, re.S)
            if pm and 'ProcessCommands' in (nm.group(1) if nm else ''):
                ported = {h(x) for x in re.findall(HEX, pm.group(1))}
                note = ' (%d of 100 opcodes unported)' % (100 - len(ported))
            out.append((h(a.group(1)), (nm.group(1) if nm else fn) + note, max(calls, 1), rel))
    return sorted(out)


def main():
    args = sys.argv[1:]
    show_all = '--all' in args
    only = ('direct' if '--direct' in args else 'virtual' if '--virtual' in args else 'import' if '--imports' in args
            else 'partial' if '--partial' in args else None)
    limit = int(args[args.index('--limit') + 1]) if '--limit' in args else None
    out_csv = args[args.index('--csv') + 1] if '--csv' in args else None

    img = Image(find_input('re/gta/1.0-US/gta_sa.exe'))
    proc = {h(r['address']): r for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/procedures.csv')))}
    state = {h(r['address']): r for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/state.csv')))}
    callers = {}
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/callgraph.csv'))):
        if r['callee'].startswith('0x'):
            callers.setdefault(h(r['callee']), set()).add(r['caller'])

    registered, uses, kinds = scan(img)

    rows = {}

    def add(a, kind):
        if a in registered or a in rows:
            return
        st = state.get(a)
        if (st and st['subsystem'].startswith('X-')) or not GAME_TEXT[0] <= a < GAME_TEXT[1]:
            return  # excluded rows; the no-CD `.text` / `.HOODLUM` (protection code, not ports)
        rows[a] = kind

    for a, kind in kinds.items():
        if only in (None, 'direct'):
            add(a, kind)
    if only in (None, 'virtual'):
        for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/data-tables.csv'))):
            if not r['kind'].startswith('vtable'):
                continue
            start, n = h(r['start']), int(r['slots'])
            for i in range(n):
                a = img.u32(start + 4 * i)
                st = state.get(a)
                if not a or not st or st['state'] not in ('unknown', 'specified'):
                    continue
                add(a, 'virtual')

    if '--no-branches' in args:
        where = {}
    else:
        import inflight
        where = inflight.elsewhere()

    keep = stays_original.load()
    table, original = [], []
    for a, kind in rows.items():
        st, pr = state.get(a, {}), proc.get(a, {})
        name = (st.get('name') or pr.get('name') or pr.get('gr_name') or pr.get('ghidra_name')
                or ('?not-in-catalogue' if a not in proc else ''))
        why = stays_original.reason(a, name, keep)
        if why:
            original.append(dict(kind=kind, address='0x%X' % a, name=name, subsystem=st.get('subsystem', ''),
                                 state=st.get('state', ''), callers=len(callers.get(a, ())), uses=uses.get(a, 0),
                                 elsewhere=why))
            continue
        table.append(dict(kind=kind, address='0x%X' % a, name=name, subsystem=st.get('subsystem', ''),
                          state=st.get('state', ''), callers=len(callers.get(a, ())), uses=uses.get(a, 0),
                          elsewhere=where.get(a, '')))
    table.sort(key=lambda r: (-r['callers'], -r['uses'], h(r['address'])))

    if only in (None, 'import'):
        for cell, name, n in import_rows(img):
            table.append(dict(kind='import', address='0x%X' % cell, name=name, subsystem='PLATFORM', state='',
                              callers=0, uses=n, elsewhere=''))
    if only in (None, 'partial'):
        for a, name, n, rel in partial_rows():
            st = state.get(a, {})
            table.append(dict(kind='partial', address='0x%X' % a, name=name, subsystem=st.get('subsystem', ''),
                              state=st.get('state', ''), callers=len(callers.get(a, ())), uses=n, elsewhere=''))
    todo = [r for r in table if not r['elsewhere']]
    shown = table if show_all else todo
    if '--stays-original' in args:
        shown = sorted(original, key=lambda r: h(r['address']))
    counts = {}
    for r in table:
        k = (r['kind'] if r['kind'] in ('virtual', 'import', 'partial') else 'direct') + (
            '-elsewhere' if r['elsewhere'] else '')
        counts[k] = counts.get(k, 0) + 1
    print('# frontier: direct %d (+%d registered on other branches), virtual %d (+%d), imports %d, partial %d; '
          'to port %d; stays original %d (catalog/stays-original.csv, not listed)' % (
              counts.get('direct', 0), counts.get('direct-elsewhere', 0), counts.get('virtual', 0),
              counts.get('virtual-elsewhere', 0), counts.get('import', 0), counts.get('partial', 0), len(todo),
              len(original)))
    cols = ['kind', 'address', 'name', 'subsystem', 'state', 'callers', 'uses', 'elsewhere']
    if out_csv:
        with open(out_csv, 'w', newline='') as f:
            w = csv.DictWriter(f, fieldnames=cols)
            w.writeheader()
            w.writerows(shown)
    for r in shown[:limit]:
        print('%-7s %-9s %4d %3d  %-12s %-10s %s%s' % (
            r['kind'], r['address'], r['callers'], r['uses'], r['subsystem'], r['state'], r['name'],
            ('  [' + ('stays original: ' if '--stays-original' in args else 'on ') + r['elsewhere'] + ']')
            if r['elsewhere'] else ''))


if __name__ == '__main__':
    main()
