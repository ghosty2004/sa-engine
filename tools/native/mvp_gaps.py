#!/usr/bin/env python3
"""Platform MVP gap list: the functions on the native boot -> main menu -> in-world walk path that
are not yet in Rust, in call order.

Inputs (paths relative to the repository root; the gitignored ones fall back to the main checkout
when this runs in a worktree, see `find_input`):
  re/gta/1.0-US/gta_sa.exe                 the 1.0 US exe (disassembled with capstone)
  catalog/raw/coverage-all.csv             runtime coverage, merge of all recordings
                                           (address,hits,blocks_hit,...; hits is 0/1): the default
                                           path recording (--path NAME selects coverage-NAME.csv)
  catalog/raw/coverage-grove-walk.csv      boot -> menus -> loading recording; partial, see
                                           DEFAULT_PATH; only for the `in_grove_walk` column
  catalog/state.csv                        per-function state (what is in Rust)
  catalog/extents.csv                      function bodies (`ranges`, incl. .HOODLUM bodies of stubs)
  catalog/callgraph.csv                    static edges (caller,callee,kind,...)
  catalog/indirect-calls.csv               resolved indirect call sites
  catalog/data-tables.csv                  vtables / function-pointer tables in data

Output:
  docs/arch/platform-mvp-gaps.csv          one row per hit function that is not in Rust:
      order,address,name,subsystem,state,kind,phase,depth,first_caller,first_caller_name,
      reached,hits,in_grove_walk,in_all
  stdout                                   summary counts

Definitions:
  on the path   hits > 0 in the path recording (default coverage-all.csv, see DEFAULT_PATH).
  in Rust       state in {implemented, L1, L2, L3}, or a `#[hook(addr = …)]` in crates/ (ports
                written but not yet recorded in state.csv); such rows are not written (use --all
                to write them too, with kind=rust).
  kind          `gap` (state specified/unknown), `native` (an excluded function sa-game replaces
                at its address: crates/sa-game/src/{crt,com,dx,rw_standin}.rs; `#[native]` ports count as
                Rust) or `excluded` (CRT,
                protection, third-party libs, EH funclets: phase B replaces them with std /
                platform code, not ports).
  --next N      also print the first N gaps of the static tier in call order.
  order         global index over all hit functions (Rust ones included), so gaps keep their
                position relative to already-ported functions.

Order (four tiers; `reached` column):
  static   Pre-order DFS from Win::NOTSA_WinMain (0x748710) over the strong static edges
           (callgraph kinds call, tailcall, indirect-ghidra, virtual, virtual-typed, plus direct
           call/jmp targets found by disassembly). A function's callees are visited in the order
           of their first call site inside the caller (capstone over the caller's extent ranges;
           .HOODLUM bodies are part of the stub's ranges, and a call to a .HOODLUM body is
           attributed to its stub address). Only hit functions are entered.
  ref      Hit functions reachable only through weak edges: function addresses used as operands
           (callbacks: RW pipeline/plugin callbacks, atexit, script handler tables), operands
           pointing into a data table of catalog/data-tables.csv (vtable / function-pointer table:
           every slot becomes an edge), and the callgraph kinds fnref, virtual-override,
           virtual-typed-override. Walked after the static tier: for each static-tier function in
           order, its weak edges in site order, each followed by a DFS over all edges.
  crt-init Static initialisers (`__init_*`) that the CRT's _initterm runs before WinMain, in
           table order (__xi, then __xc slots of catalog/data-tables.csv), each followed by a DFS
           over all edges; phase boot.
  dynamic  The rest (no static or weak edge from a reached function; e.g. CTask virtuals behind
           a "wide" virtual call, callbacks stored through computed pointers). Roots are
           taken in (subsystem, address) order, each followed by a DFS over all edges.

Phase heuristic (coarse): a function inherits the phase of the caller through which it was first
reached, unless its own name is a phase marker (PHASE_MARKERS below, matched by exact name or
`Class::` prefix), which overrides it for its whole subtree. WinMain itself is `boot`: the RW / OS
/ window setup (RsInitialize, RsRwInitialize, CGame::InitialiseRenderWare, ...AfterRW) stays boot;
Game::FrontendIdle, Game::RenderMenus and CMenuManager::* are `frontend-menu`; Game::InitialiseGame,
CGame::Initialise / Init1..3 / ...WhenRestarting are `loading`; Game::Idle, CGame::Process,
Game::RenderScene / RenderEffects / Render2dStuff, CWorld::Process and CRenderer::* are `in-world`.
Because App::AppEventHandler's switch reaches FrontendIdle and Idle at its first call from WinMain,
the static DFS defers a marker of a later phase (rank boot < frontend-menu < loading < in-world)
until every subtree of the current phase is done, so the static tier reads boot, main menu, loading,
in-world. ref rows inherit the phase of the function referencing the table / callback (so a vtable
first referenced by a constructor during loading tags its virtuals `loading`); crt-init rows are
boot; a dynamic row takes the phase of its earliest-ordered hit caller of any edge kind, else
`unknown`. One DFS discovers a shared helper once, so a helper used both while loading and in-world
carries the phase that comes first.

Deterministic (no timestamps, sorted inputs). Runs in well under a minute.
Run: .venv/bin/python -I tools/native/mvp_gaps.py [--all] [--path all|grove-walk|pause-restart]
"""
import bisect
import collections
import csv
import re
import struct
import sys
from pathlib import Path

import capstone
from capstone import x86_const as X

ROOT = Path(__file__).resolve().parents[2]
CAT = ROOT / 'catalog'
OUT = ROOT / 'docs' / 'arch' / 'platform-mvp-gaps.csv'

WINMAIN = 0x748710
RUST_STATES = {'implemented', 'L1', 'L2', 'L3'}
STRONG_KINDS = {'call', 'tailcall', 'indirect-ghidra', 'virtual', 'virtual-typed'}
WEAK_KINDS = {'fnref', 'virtual-override', 'virtual-typed-override'}
HOODLUM = (0x1556000, 0x1577000)
CODE = [(0x401000, 0x858000), (0xCB1000, 0x1301000), HOODLUM]
NO_SITE = 1 << 40
# Recording that defines "on the path". coverage-grove-walk.csv (2026-11-05) is a partial run: it
# stops in the loading-screen fade loop (WORKLOG INF-10), so CGame::Init2/Init3, Game::Idle,
# CGame::Process and Game::RenderScene have 0 hits in it. coverage-all.csv (2026-11-10; today equal
# to coverage-pause-restart.csv) boots, goes through the menus, loads, walks in the world (every
# corpus program starts like grove-walk), opens the pause menu and restarts: it is the default.
DEFAULT_PATH = 'all'  # sort key for edges whose site was not found (after all real sites)

# name -> phase; an entry ending in '::' matches every member of that class / namespace
PHASE_MARKERS = [
    ('Win::NOTSA_WinMain', 'boot'),
    ('Rs::RsInitialize', 'boot'),
    ('Rs::RsRwInitialize', 'boot'),
    ('CGame::InitialiseOnceBeforeRW', 'boot'),
    ('CGame::InitialiseRenderWare', 'boot'),
    ('CGame::InitialiseEssentialsAfterRW', 'boot'),
    ('CGame::InitialiseCoreDataAfterRW', 'boot'),
    ('Game::InitialiseGame', 'loading'),
    ('CGame::Initialise', 'loading'),
    ('CGame::Init1', 'loading'),
    ('CGame::Init2', 'loading'),
    ('CGame::Init3', 'loading'),
    ('CGame::InitialiseWhenRestarting', 'loading'),
    ('CGame::ReInitGameObjectVariables', 'loading'),
    ('CGame::ShutDownForRestart', 'loading'),
    ('Game::FrontendIdle', 'frontend-menu'),
    ('Game::RenderMenus', 'frontend-menu'),
    ('CMenuManager::', 'frontend-menu'),
    ('Game::Idle', 'in-world'),
    ('CGame::Process', 'in-world'),
    ('Game::RenderScene', 'in-world'),
    ('Game::RenderEffects', 'in-world'),
    ('Game::Render2dStuff', 'in-world'),
    ('CWorld::Process', 'in-world'),
    ('CRenderer::', 'in-world'),
]


# boot -> main menu -> loading -> in-world: the static DFS finishes every subtree of an earlier
# phase before it enters a marker of a later one (see the module docstring)
PHASE_RANK = {'boot': 0, 'frontend-menu': 1, 'loading': 2, 'in-world': 3}


def find_input(rel):
    """Repository file, falling back to the main checkout for gitignored inputs in a worktree."""
    p = ROOT / rel
    if p.exists():
        return p
    parts = ROOT.parts
    if '.claude' in parts and 'worktrees' in parts:
        main = Path(*parts[:parts.index('.claude')])
        if (main / rel).exists():
            return main / rel
    sys.exit(f'missing input: {rel}')


def read_csv(path):
    with open(path, newline='') as f:
        return list(csv.DictReader(f))


def h(s):
    return int(s, 16)


class Image:
    """Minimal PE32 loader: maps section raw data at their virtual addresses."""

    def __init__(self, path):
        self.data = Path(path).read_bytes()
        d = self.data
        pe = struct.unpack_from('<I', d, 0x3C)[0]
        nsec = struct.unpack_from('<H', d, pe + 6)[0]
        optsz = struct.unpack_from('<H', d, pe + 20)[0]
        self.base = struct.unpack_from('<I', d, pe + 24 + 28)[0]
        self.secs = []
        off = pe + 24 + optsz
        for i in range(nsec):
            vsize, va, rsize, raw = struct.unpack_from('<IIII', d, off + i * 40 + 8)
            self.secs.append((self.base + va, self.base + va + max(vsize, rsize), raw, rsize))

    def read(self, va, n):
        for lo, hi, raw, rsize in self.secs:
            if lo <= va < hi:
                o = va - lo
                b = self.data[raw + o: raw + min(o + n, rsize)]
                return b + b'\0' * (n - len(b))
        return None


def phase_marker(name):
    for key, ph in PHASE_MARKERS:
        if name == key or (key.endswith('::') and name.startswith(key)):
            return ph
    return None


def main():
    args = sys.argv[1:]
    write_all = '--all' in args
    path_rec = args[args.index('--path') + 1] if '--path' in args else DEFAULT_PATH
    state_rows = read_csv(CAT / 'state.csv')
    info = {h(r['address']): r for r in state_rows}

    def load_cov(rec_name):
        return {h(r['address']): int(r['hits'])
                for r in read_csv(find_input(f'catalog/raw/coverage-{rec_name}.csv'))}

    cov = load_cov(path_rec)
    cov_grove = load_cov('grove-walk')
    cov_all = load_cov('all')
    hit = {a for a, n in cov.items() if n > 0 and a in info}
    missing_from_state = sorted(a for a, n in cov.items() if n > 0 and a not in info)

    ext = {}
    body2stub = {}
    ext_insns = {}
    for r in read_csv(CAT / 'extents.csv'):
        a = h(r['address'])
        ext_insns[a] = r['n_insns']
        ext[a] = [tuple(h(x) for x in rg.split('-')) for rg in r['ranges'].split(';') if rg]
        if r['hoodlum_body']:
            body2stub[h(r['hoodlum_body'])] = a

    def canon(t):
        if t in info:
            return t
        return body2stub.get(t)

    # data tables: [start, end) -> slot targets (catalogue functions only)
    img = Image(find_input('re/gta/1.0-US/gta_sa.exe'))
    tables = []
    crt_init = []  # (_initterm pass: __xi before __xc, table, slots in table order)
    for r in read_csv(CAT / 'data-tables.csv'):
        lo, hi = h(r['start']), h(r['end'])
        raw = img.read(lo, hi - lo)
        if raw is None:
            continue
        slots = []
        for i in range(0, len(raw) - 3, 4):
            t = canon(struct.unpack_from('<I', raw, i)[0])
            if t is not None and t not in slots:
                slots.append(t)
        if slots:
            tables.append((lo, hi, slots))
            if r['kind'].startswith('crt-init:'):
                crt_init.append((0 if r['kind'] == 'crt-init:__xi' else 1, lo, slots))
    tables.sort()
    tstarts = [t[0] for t in tables]

    def table_at(v):
        i = bisect.bisect_right(tstarts, v) - 1
        if i >= 0 and tables[i][0] <= v < tables[i][1]:
            return tables[i][2]
        return None

    indirect = collections.defaultdict(list)  # site -> targets
    for r in read_csv(CAT / 'indirect-calls.csv'):
        if r['resolution'] == 'unresolved':
            continue
        for m in re.findall(r'(?:->\s*|^)(0x[0-9a-fA-F]+)', r['detail']):
            t = canon(h(m))
            if t is not None:
                indirect[h(r['site'])].append(t)

    cg = collections.defaultdict(list)  # caller -> [(callee, kind)] in file order
    rev = collections.defaultdict(set)
    for r in read_csv(CAT / 'callgraph.csv'):
        if not r['callee'].startswith('0x'):
            continue
        a, b = h(r['caller']), h(r['callee'])
        b2 = canon(b)
        if a in info and b2 is not None:
            cg[a].append((b2, r['kind']))
            rev[b2].add(a)

    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    md.detail = True
    md.skipdata = True

    def is_code(v):
        return any(lo <= v < hi for lo, hi in CODE)

    def scan(f):
        """(strong_sites, weak_sites): callee -> first site address in f, by call-site address.

        Instructions come from a linear sweep of f's extent ranges plus a recursive descent from its
        entry that follows jumps (a stub's jmp into .HOODLUM, X-mis-split pieces) and switch jump
        tables (`jmp [reg*4 + table]`): extents.csv misses switch case blocks, e.g. App::AppEventHandler
        whose cases call Game::Idle / Game::FrontendIdle / CGame::InitialiseRenderWare. The descent
        stops at ret, at an unconditional jump to another catalogue function (a tail call) and when
        flow reaches another catalogue function's entry.
        """
        strong, weak = {}, {}
        rngs = ext.get(f, [])
        seen = set()

        def own(t):
            # a jump target that is still part of f
            c = canon(t)
            return c is None or c == f or info[c]['subsystem'] == 'X-mis-split'

        def add(d, t, site):
            if t is not None and t != f and (t not in d or site < d[t]):
                d[t] = site

        def handle(ins):
            """Record edges of one instruction; return successor addresses for the descent."""
            site, ops, iid = ins.address, ins.operands, ins.id
            succ = []
            if iid in (X.X86_INS_CALL, X.X86_INS_JMP) or ins.group(capstone.CS_GRP_JUMP):
                if len(ops) == 1 and ops[0].type == X.X86_OP_IMM:
                    t = ops[0].imm & 0xFFFFFFFF
                    if iid == X.X86_INS_CALL:
                        add(strong, canon(t), site)
                    elif own(t):
                        succ.append(t)
                    else:
                        add(strong, canon(t), site)
                    if iid != X.X86_INS_JMP:
                        succ.append(site + ins.size)
                    return succ
                if iid == X.X86_INS_JMP and len(ops) == 1 and ops[0].type == X.X86_OP_MEM \
                        and ops[0].mem.index != 0 and ops[0].mem.scale == 4 and ops[0].mem.base == 0:
                    tab = ops[0].mem.disp & 0xFFFFFFFF
                    for i in range(512):
                        raw = img.read(tab + 4 * i, 4)
                        t = struct.unpack('<I', raw)[0] if raw else 0
                        if not is_code(t) or abs(t - site) > 0x10000 or (t in info and not own(t)):
                            break
                        succ.append(t)
            for t in indirect.get(site, ()):
                add(strong, t, site)
            for op in ops:
                if op.type == X.X86_OP_IMM:
                    v = op.imm & 0xFFFFFFFF
                elif op.type == X.X86_OP_MEM and (op.mem.base == 0 or op.mem.disp > 0x400000):
                    v = op.mem.disp & 0xFFFFFFFF
                else:
                    continue
                if v < 0x401000:
                    continue
                if v in info and v != f and iid not in (X.X86_INS_CALL, X.X86_INS_JMP):
                    add(weak, v, site)
                slots = table_at(v)
                if slots:
                    for t in slots:
                        add(weak, t, site)
            if iid in (X.X86_INS_RET, X.X86_INS_JMP, X.X86_INS_INT3, X.X86_INS_HLT):
                return succ
            return succ + [site + ins.size]

        work = [f]
        for lo, hi in rngs:
            code = img.read(lo, hi - lo)
            if code is None:
                continue
            for ins in md.disasm(code, lo):
                if ins.id and ins.address not in seen:
                    seen.add(ins.address)
                    work.extend(t for t in handle(ins) if not lo <= t < hi)
        while work:
            a = work.pop()
            while a not in seen:
                if a != f and a in info and not own(a):
                    break  # flowed into another function
                code = img.read(a, 16)
                if code is None:
                    break
                ins = next(md.disasm(code, a, 1), None)
                if ins is None or ins.id == 0:
                    break
                seen.add(a)
                succ = handle(ins)
                nxt = a + ins.size
                if nxt in succ:
                    succ.remove(nxt)
                    work.extend(succ)
                    a = nxt
                else:
                    work.extend(succ)
                    break
        return strong, weak

    edges_cache = {}

    def edges(f):
        """Hit callees of f: (strong list, weak list), each in site order."""
        if f in edges_cache:
            return edges_cache[f]
        s_sites, w_sites = scan(f)
        strong, weak = {}, {}
        for i, (b, kind) in enumerate(cg.get(f, ())):
            if kind in STRONG_KINDS:
                strong.setdefault(b, s_sites.get(b, w_sites.get(b, NO_SITE + i)))
            elif kind in WEAK_KINDS:
                weak.setdefault(b, w_sites.get(b, s_sites.get(b, NO_SITE + i)))
        for b, s in s_sites.items():
            strong.setdefault(b, s)
        for b, s in w_sites.items():
            if b not in strong:
                weak.setdefault(b, s)
        for b in strong:
            weak.pop(b, None)
        res = ([b for b, _ in sorted(strong.items(), key=lambda x: (x[1], x[0])) if b in hit and b != f],
               [b for b, _ in sorted(weak.items(), key=lambda x: (x[1], x[0])) if b in hit and b != f])
        edges_cache[f] = res
        return res

    rec = {}  # address -> dict(order, depth, parent, reached, phase)
    order = []

    def visit(f, depth, parent, reached, phase):
        rec[f] = dict(order=len(order), depth=depth, parent=parent, reached=reached,
                      phase=phase_marker(info[f]['name']) or phase)
        order.append(f)

    def dfs(root, depth, parent, reached, phase, strong_only, defer=None, rank=None):
        """Pre-order DFS. With `defer` (a list), a child that is a phase marker of a later phase
        than `rank` is not entered but queued as (rank, discovery, child, parent)."""
        visit(root, depth, parent, reached, phase)
        stack = [(root, iter(edges(root)[0] if strong_only else merged(root)))]
        while stack:
            f, it = stack[-1]
            nxt = None
            for c in it:
                if c in rec:
                    continue
                if defer is not None:
                    pr = PHASE_RANK.get(phase_marker(info[c]['name']), -1)
                    if pr > rank:
                        if c not in deferred_seen:
                            deferred_seen.add(c)
                            defer.append((pr, len(deferred_seen), c, f))
                        continue
                nxt = c
                break
            if nxt is None:
                stack.pop()
                continue
            visit(nxt, rec[f]['depth'] + 1, f, reached, rec[f]['phase'])
            stack.append((nxt, iter(edges(nxt)[0] if strong_only else merged(nxt))))

    def merged(f):
        s, w = edges(f)
        return s + w

    # tier 1: static
    if WINMAIN not in hit:
        sys.exit('WinMain is not hit in the coverage file')
    deferred_seen = set()
    defer = []
    dfs(WINMAIN, 0, None, 'static', 'boot', True, defer, 0)
    while defer:
        defer.sort()
        rank, _, c, f = defer.pop(0)
        if c not in rec:
            dfs(c, rec[f]['depth'] + 1, f, 'static', rec[f]['phase'], True, defer, rank)
    n_static = len(order)
    # tier 2: weak references from the static tier, in static order
    for f in list(order[:n_static]):
        for c in edges(f)[1]:
            if c not in rec:
                dfs(c, rec[f]['depth'] + 1, f, 'ref', rec[f]['phase'], False)
    n_ref = len(order) - n_static
    # tier 3: static initialisers the CRT runs from the __xi / __xc tables before WinMain
    for _, _, slots in sorted(crt_init):
        for c in slots:
            if c in hit and c not in rec:
                dfs(c, 0, None, 'crt-init', 'boot', False)
    n_crt = len(order) - n_static - n_ref
    # tier 4: dynamic
    rest = sorted((a for a in hit if a not in rec), key=lambda a: (info[a]['subsystem'], a))
    for a in rest:
        if a in rec:
            continue
        callers = sorted((rec[c]['order'], c) for c in rev.get(a, ()) if c in rec)
        phase = rec[callers[0][1]]['phase'] if callers else 'unknown'
        dfs(a, 0, None, 'dynamic', phase, False)
    assert len(order) == len(hit)

    # What phase B runs natively, from the source tree (the catalogue's state lags behind ports that
    # are written but not yet verified): every `#[hook(addr = …)]`, and sa-game's address-keyed
    # native replacements (CRT, DirectX thunks, the temporary RW driver stand-ins); `#[native]` ports count as Rust.
    hooked, native = set(), set()
    for p in (ROOT / 'crates').rglob('*.rs'):
        t = p.read_text(errors='replace')
        hooked.update(int(m, 16) for m in re.findall(r'#\[(?:[a-z_]+::)?(?:hook|native)\(addr = (0x[0-9A-Fa-f]+)', t))
    for f, into in (('crt.rs', native), ('com.rs', native), ('dx.rs', native), ('rw_standin.rs', native)):
        p = ROOT / 'crates' / 'sa-game' / 'src' / f
        if p.exists():
            into.update(int(m.replace('_', ''), 16)
                        for m in re.findall(r'(?:\(|^)\s*(0x[0-9A-Fa-f_]+),\s*"', p.read_text(), re.M))

    def kind_of(a):
        st = info[a]['state']
        if st in RUST_STATES or a in hooked:
            return 'rust'
        if a in native:
            return 'native'
        if st == 'excluded':
            return 'excluded'
        return 'gap'

    OUT.parent.mkdir(parents=True, exist_ok=True)
    cols = ['order', 'address', 'name', 'subsystem', 'state', 'kind', 'phase', 'depth', 'first_caller',
            'first_caller_name', 'reached', 'hits', 'in_grove_walk', 'in_all']
    with open(OUT, 'w', newline='') as fh:
        w = csv.writer(fh, lineterminator='\n')
        w.writerow(cols)
        for a in order:
            k = kind_of(a)
            if k == 'rust' and not write_all:
                continue
            r = rec[a]
            p = r['parent']
            w.writerow([r['order'], f'0x{a:x}', info[a]['name'], info[a]['subsystem'], info[a]['state'], k,
                        r['phase'], r['depth'], f'0x{p:x}' if p is not None else '',
                        info[p]['name'] if p is not None else '', r['reached'], cov[a],
                        int(cov_grove.get(a, 0) > 0), int(cov_all.get(a, 0) > 0)])

    # summary
    kinds = collections.Counter(kind_of(a) for a in order)
    print(f'path recording: coverage-{path_rec}.csv')
    print(f'hit functions on the path: {len(order)}  (static {n_static}, ref {n_ref}, '
          f'crt-init {n_crt}, dynamic {len(order) - n_static - n_ref - n_crt})')
    if missing_from_state:
        print(f'hit addresses missing from state.csv (ignored): {len(missing_from_state)}')
    print(f'in Rust: {kinds["rust"]}  gaps: {kinds["gap"]}  excluded: {kinds["excluded"]}  '
          f'native replacements: {kinds["native"]}')
    if '--next' in args:
        n = int(args[args.index('--next') + 1])
        print(f'next {n} gaps in call order (static tier, initialisers excluded):')
        k = 0
        for a in order:
            if kind_of(a) != 'gap' or rec[a]['reached'] != 'static':
                continue
            ins = ext_insns.get(a, '')
            print(f'  {k + 1:3d}. 0x{a:x} {info[a]["name"] or "(unnamed)"} [{info[a]["subsystem"]}, '
                  f'{ins} insns, {rec[a]["phase"]}]')
            k += 1
            if k == n:
                break
    for label, key in (('gaps by reached', 'reached'), ('gaps by phase', 'phase')):
        c = collections.Counter(rec[a][key] for a in order if kind_of(a) == 'gap')
        print(f'{label}: ' + ', '.join(f'{k} {n}' for k, n in sorted(c.items(), key=lambda x: (-x[1], x[0]))))
    c = collections.Counter(info[a]['state'] for a in order if kind_of(a) == 'gap')
    print('gaps by state: ' + ', '.join(f'{k} {n}' for k, n in sorted(c.items())))
    sub = collections.Counter(info[a]['subsystem'] for a in order if kind_of(a) == 'gap')
    rs = collections.Counter(info[a]['subsystem'] for a in order if kind_of(a) == 'rust')
    print('gaps per subsystem (top 20; gap / in Rust on the path):')
    for s, n in sorted(sub.items(), key=lambda x: (-x[1], x[0]))[:20]:
        print(f'  {s:<12} {n:5d} / {rs[s]}')
    ex = collections.Counter(info[a]['subsystem'] for a in order if kind_of(a) == 'excluded')
    print('excluded per subsystem: ' + ', '.join(f'{k} {n}' for k, n in sorted(ex.items(), key=lambda x: (-x[1], x[0]))))
    print(f'wrote {OUT.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
