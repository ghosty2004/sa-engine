#!/usr/bin/env python3
"""SAMP-1 (static): inventory of the gta_sa.exe 1.0 US functionality the SA:MP client depends on.

Inputs (read-only):
  ~/samp-rs/crates/samp-client/src/**/*.rs          addresses, field offsets and SCM opcodes samp-rs uses
  ~/samp-rs/docs/re/data/gta-patches-r4-annotated.csv  every memory patch samp.dll 0.3.7-R4 applies
  catalog/procedures.csv, extents.csv, callgraph.csv, data-tables.csv, script-commands.csv
  gta-reversed / plugin-sdk headers (global names via tools/catalog/callgraph.py, field offsets via
  plugin-sdk VALIDATE_OFFSET)
  re/gta/1.0-US/gta_sa.exe                          vtable slot targets, call targets at patched sites

Outputs:
  catalog/samp-usage.csv     one row per (address or field or opcode, source occurrence group)
  docs/samp/aggregates.json  numbers the README quotes (regenerated, deterministic)
  docs/samp/required-functions.csv  the direct set and the closure, one row per catalogue function

Run: .venv/bin/python -I docs/samp/samp_usage.py   (never writes outside catalog/ and docs/samp/)
Method and rules: docs/samp/README.md.
"""
import bisect
import collections
import csv
import json
import os
import re
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SAMP = Path(os.path.expanduser('~/samp-rs'))
SRC = SAMP / 'crates' / 'samp-client' / 'src'
PATCH_CSV = SAMP / 'docs' / 're' / 'data' / 'gta-patches-r4-annotated.csv'
CAT = ROOT / 'catalog'
OUT_CSV = CAT / 'samp-usage.csv'
OUT_DIR = ROOT / 'docs' / 'samp'

sys.path.insert(0, str(ROOT / 'tools' / 'catalog'))
import callgraph as cg  # noqa: E402  (global_names, section_of, DATA, Image)

GAME_LO, GAME_HI = 0x401000, 0x1577000


def hx(v):
    return f'{v:#x}'


def in_code(a):
    return 0x401000 <= a < 0x858000 or 0x1556000 <= a < 0x1577000


def in_game_image(a):
    return in_code(a) or cg.section_of(a) is not None


# ============================================================================ catalogue
class Catalogue:
    def __init__(self):
        self.proc = {}
        for r in csv.DictReader(open(CAT / 'procedures.csv')):
            self.proc[int(r['address'], 16)] = r
        self.ext = {}
        ivs = []
        for r in csv.DictReader(open(CAT / 'extents.csv')):
            fa = int(r['address'], 16)
            self.ext[fa] = r
            for rg in r['ranges'].split(';'):
                if not rg:
                    continue
                lo, hi = (int(x, 16) for x in rg.split('-'))
                ivs.append((lo, hi, fa))
        ivs.sort()
        self.ivs = ivs
        self.iv_lo = [x[0] for x in ivs]
        self.gnames = cg.global_names(cg.GR, cg.PSDK, lambda a: cg.section_of(a) is not None)
        self.gsorted = sorted(self.gnames)
        # vtables / data tables
        self.tables = []
        for r in csv.DictReader(open(CAT / 'data-tables.csv')):
            self.tables.append((int(r['start'], 16), int(r['end'], 16), r['kind'], r['name']))
        self.tables.sort()
        self.tab_lo = [t[0] for t in self.tables]
        self.edges = collections.defaultdict(list)
        for r in csv.DictReader(open(CAT / 'callgraph.csv')):
            try:
                a = int(r['caller'], 16)
                b = int(r['callee'], 16)
            except ValueError:
                continue
            self.edges[a].append((b, r['kind'], int(r['sites'] or 0)))
        self.img = cg.Image(cg.EXE)

    def fname(self, fa):
        r = self.proc.get(fa)
        if not r:
            return ''
        return r['name'] or r['gr_name'] or ''

    def subsystem(self, fa):
        r = self.proc.get(fa)
        return r['subsystem'] if r else ''

    def containing(self, a):
        """Catalogue functions whose body ranges contain a (several when tails are shared)."""
        out = []
        i = bisect.bisect_right(self.iv_lo, a) - 1
        # ranges can overlap (shared tails); scan back a little
        j = i
        while j >= 0 and j > i - 64:
            lo, hi, fa = self.ivs[j]
            if lo <= a < hi and fa not in out:
                out.append(fa)
            j -= 1
        return out

    def gname(self, a):
        if a in self.gnames:
            return self.gnames[a][0], ''
        i = bisect.bisect_right(self.gsorted, a) - 1
        if i >= 0 and a - self.gsorted[i] < 0x400:
            b = self.gsorted[i]
            return f'{self.gnames[b][0]}+{a - b:#x}', 'near'
        return '', ''

    def table(self, a):
        i = bisect.bisect_right(self.tab_lo, a) - 1
        if i >= 0:
            lo, hi, kind, name = self.tables[i]
            if lo <= a < hi:
                return lo, kind, name
        return None

    def u32(self, a):
        try:
            return self.img.u32(a)
        except Exception:
            return None

    def resolve(self, a):
        """-> dict(loc, func, func_name, subsystem, global_name)."""
        d = {'loc': '', 'func': '', 'func_name': '', 'subsystem': '', 'global': '', 'extra': ''}
        if in_code(a):
            if a in self.proc:
                d['loc'] = 'fn-entry'
                fs = [a]
            else:
                fs = self.containing(a)
                d['loc'] = 'fn-body' if fs else 'code-unmapped'
            if fs:
                d['func'] = ';'.join(hx(f) for f in fs)
                d['func_name'] = ';'.join(self.fname(f) for f in fs)
                d['subsystem'] = ';'.join(sorted({self.subsystem(f) for f in fs}))
            return d
        sec = cg.section_of(a)
        if sec is None:
            d['loc'] = 'outside-image'
            return d
        d['loc'] = 'data:' + sec
        t = self.table(a)
        if t:
            lo, kind, name = t
            if kind.startswith('vtable'):
                slot = (a - lo) // 4
                tgt = self.u32(a)
                cls = kind.split(':', 1)[1] if ':' in kind else name
                d['loc'] = 'vtable' if a == lo else 'vtable-slot'
                d['global'] = f'vtable {cls or hx(lo)}' + ('' if a == lo else f' slot {slot} (+{a - lo:#x})')
                if a != lo and tgt and tgt in self.proc:
                    d['func'] = hx(tgt)
                    d['func_name'] = self.fname(tgt)
                    d['subsystem'] = self.subsystem(tgt)
                return d
            d['extra'] = f'table {kind} {name} {hx(lo)}'
        g, how = self.gname(a)
        d['global'] = g
        return d


# ============================================================================ samp-rs source scan
HEX = re.compile(r'\b0x([0-9A-Fa-f_]+)\b')
CONST = re.compile(r'^\s*(?:pub(?:\([\w:]+\))?\s+)?(?:const|static)\s+(?:mut\s+)?([A-Z][A-Z0-9_]*)\s*:\s*([^=]+?)\s*=\s*(.*)$')


def strip_comment(line):
    """Remove // comments outside string literals."""
    out, i, ins = [], 0, False
    while i < len(line):
        c = line[i]
        if c == '"' and (i == 0 or line[i - 1] != '\\'):
            ins = not ins
        if not ins and line.startswith('//', i):
            break
        out.append(c)
        i += 1
    return ''.join(out)


ASM_STR = re.compile(r'^\s*(push|cmp|mov|jmp|call|lea|add|sub|test|and|or|xor|fld|fstp|movzx)\b', re.I)


def strip_strings(line):
    """Blank string literals unless they hold an x86 instruction (naked_asm! operands keep their addresses)."""
    return re.sub(r'"((?:[^"\\]|\\.)*)"', lambda m: m.group(0) if ASM_STR.match(m.group(1)) else '""', line)


class SrcFile:
    def __init__(self, path):
        self.path = path
        self.rel = os.path.relpath(path, SAMP)
        self.raw = open(path, errors='replace').read().split('\n')
        self.code = [strip_strings(strip_comment(x)) for x in self.raw]
        self.test = [False] * len(self.raw)
        i = 0
        while i < len(self.raw):
            if re.match(r'\s*#\[cfg\(test\)\]', self.raw[i]):
                depth, started, j = 0, False, i
                while j < len(self.raw):
                    self.test[j] = True
                    depth += self.code[j].count('{') - self.code[j].count('}')
                    if '{' in self.code[j]:
                        started = True
                    if started and depth <= 0:
                        break
                    if not started and self.code[j].rstrip().endswith(';') and j > i:
                        break
                    j += 1
                i = j + 1
            else:
                i += 1

    def doc_above(self, i):
        out = []
        j = i - 1
        while j >= 0 and self.raw[j].strip().startswith(('///', '#[', '//')):
            if self.raw[j].strip().startswith('///'):
                out.append(self.raw[j].strip()[3:].strip())
            j -= 1
        return ' '.join(reversed(out))


def parse_int(tok):
    tok = tok.replace('_', '')
    return int(tok, 16)


def eval_expr(expr, env):
    """Evaluate a simple const expression (ints, hex, names, + - * << |, parentheses, `as T`)."""
    e = expr.strip().rstrip(';').strip()
    e = re.sub(r'\bas\s+(?:u8|u16|u32|i32|usize|isize|u64|i64)\b', '', e)
    e = re.sub(r'(\d)_(?=\d)', r'\1', e)
    if re.search(r'[\[\]{}"&]|\.\.|::<', e):
        return None
    toks = re.findall(r'0x[0-9A-Fa-f_]+|\d+(?:u32|usize|u8|u16|i32)?|[A-Za-z_][\w:]*|[-+*()|]|<<', e)
    if ''.join(toks).replace(' ', '') != re.sub(r'\s+', '', e):
        return None
    py = []
    for t in toks:
        if t.startswith('0x'):
            py.append(str(parse_int(t[2:])))
        elif re.match(r'\d', t):
            py.append(re.sub(r'[a-z]\w*$', '', t))
        elif re.match(r'[A-Za-z_]', t):
            n = t.split('::')[-1]
            if n not in env or env[n] is None:
                return None
            py.append(str(env[n]))
        else:
            py.append(t)
    try:
        return eval(''.join(py), {'__builtins__': {}}, {})
    except Exception:
        return None


def scan_sources():
    files = [SrcFile(p) for p in sorted(SRC.rglob('*.rs'))]
    # const definitions (single-line value expressions + array consts spanning lines)
    defs = {}      # name -> list of (file, line_idx, expr_text, type)
    arrays = {}    # (file, line_idx of element) -> array const name
    for f in files:
        i = 0
        while i < len(f.code):
            m = CONST.match(f.code[i])
            if m:
                name, typ, rest = m.group(1), m.group(2), m.group(3)
                if rest.strip().endswith(';') or ';' in rest:
                    defs.setdefault(name, []).append((f, i, rest.split(';')[0], typ))
                    if '[' in rest:
                        arrays[(f.rel, i)] = name
                else:  # multi-line value (arrays, struct literals)
                    j = i
                    while j < len(f.code) and ';' not in f.code[j][len(f.code[j]) if j == i else 0:] and j - i < 4000:
                        if j > i and re.match(r'\s*(?:pub\s+)?(?:const|static|fn|impl|struct|enum|mod)\b', f.code[j]):
                            break
                        arrays[(f.rel, j)] = name
                        if j > i and ';' in f.code[j]:
                            break
                        j += 1
                    arrays[(f.rel, j)] = name
                    defs.setdefault(name, []).append((f, i, '', typ))
                    i = j
            i += 1
    env = {}
    for _ in range(6):
        for n, lst in defs.items():
            vals = {eval_expr(e, env) for (_f, _i, e, _t) in lst if e}
            vals.discard(None)
            if len(vals) == 1:
                env[n] = vals.pop()
    return files, defs, arrays, env


def usage_lines(files, name, skip):
    out = []
    pat = re.compile(r'\b' + re.escape(name) + r'\b')
    for f in files:
        for i, c in enumerate(f.code):
            if (f.rel, i) in skip:
                continue
            if pat.search(c):
                out.append((f, i))
    return out


# ============================================================================ occurrences + kind rules
HOOK_RE = re.compile(r'hook_call|hook_jmp|retarget_branch|encode_branch|OP_CALL|OP_JMP|swap_pointer|hook_jmp_indirect|'
                     r'\bdetour|install_indirect|\bredirect|trampoline', re.I)
NOP_RE = re.compile(r'\.nop\(|OP_NOP|Write::Nop|\b0x90\b')
RET_RE = re.compile(r'\[\s*0xC3\b|\[\s*0xc3\b|\[\s*0xC2\b|\[\s*0xc2\b')
WRITE_RE = re.compile(r'\.write\w*\(|\bwr(?:::<[^>]+>)?\(|write_volatile|\bpoke\(|\bwr_\w+\(|\bset_\w+\(|copy_nonoverlapping\([^,]+,\s*\w*\s*as')
READ_RE = re.compile(r'\.read\w*\(|\brd(?:::<[^>]+>)?\(|read_volatile|read_unaligned|\bpeek\(|\bderef\(|\*\s*\(\s*\w+\s+as\s+\*const')
CMP_RE = re.compile(r'==|!=|\.contains\(|matches!')


CALL_RE = re.compile(r'transmute|extern\s+"(?:thiscall|C|fastcall|stdcall|system|cdecl)"\s+fn\s*\(|'
                     r'\b(?:call\d*|thiscall\w*|cdecl\w*|stdcall\w*|fastcall\w*|call_\w+)\s*\(|Op::Call\(|\borig\s*=\s*const')
CONT_NAME = re.compile(r'(RESUME|CONTINUE|_CONT$|SKIP|BLOW|_BACK$|RETURN|UNIMPLEMENTED_TARGET|SEH_SCOPE)')
DATA_W = re.compile(r'Write::Data\(|Op::Zero\(|Op::Write\(')
DATA_R = re.compile(r'\b(?:rd_\w+|v3|read_\w+)\s*\(')


def classify_src(addr, res, ctx_lines, doc, name, line):
    """Kind for a samp-rs address from its code context. Returns (kind, also, note)."""
    ctx = ' '.join(ctx_lines)
    loc = res['loc']
    nm = name or ''
    lit = hx(addr)
    is_code = loc in ('fn-entry', 'fn-body', 'code-unmapped')
    # naked-asm operands
    if re.search(r'naked_asm!|"\s*(?:push|cmp|mov)\b', line):
        if re.search(r'cmp\s+dword ptr \[esp\],\s*' + lit, line, re.I):
            return 'hook', '', 'return-address test in a hook stub (identifies the caller)'
        if is_code:
            return 'hook', '', 'code address used inside a hook stub (push/continuation)'
        return 'global-access', '', 'operand of a hook stub instruction (read/write not separated)'
    if re.search(r'\b(?:resume|cont|back|skip|blow|refused|seh)\s*=\s*const\b', ctx) and CONT_NAME.search(nm):
        return 'hook', '', 'continuation address of a hook stub (re-enters the original code)'
    if CONT_NAME.search(nm) and is_code:
        return 'hook', '', 'continuation address of a hook (re-enters the original code)'
    if re.search(r'\b(?:f\d|flag|var)\s*=\s*const\b', ctx) and not is_code:
        return 'global-access', '', 'global used by a hook stub (read/write not separated)'
    if nm.endswith('_SLOT') and loc in ('code-unmapped', 'fn-body'):
        return 'hook', '', 'indirect-jmp slot in game code (holds the handler address)'
    has_call = bool(CALL_RE.search(ctx)) or bool(re.search(r'let\s+f\s*=.*\b' + re.escape(nm) + r'\b', ctx)) if nm else bool(CALL_RE.search(ctx))
    has_hook = bool(HOOK_RE.search(ctx)) or bool(re.search(r'\bpatcher\.\w+\(', ctx))
    has_nop = bool(NOP_RE.search(ctx))
    has_ret = bool(RET_RE.search(ctx))
    has_w = bool(WRITE_RE.search(ctx)) or bool(DATA_W.search(ctx))
    has_r = bool(READ_RE.search(ctx)) or bool(DATA_R.search(ctx))
    has_cmp = bool(CMP_RE.search(ctx)) or bool(re.search(lit + r'\s*=>', line, re.I)) or ('=>' in line and is_code)
    kinds = []
    if is_code:
        if has_hook:
            kinds.append('hook')
        if has_ret and loc == 'fn-entry':
            kinds.append('patch-disable')
        elif has_nop:
            kinds.append('patch-disable')
        if has_call and loc == 'fn-entry':
            kinds.append('call')
        if not kinds and has_w and loc == 'fn-body':
            kinds.append('patch-modify')
        if not kinds and has_cmp:
            kinds.append('code-compare')
        if not kinds and has_r:
            kinds.append('code-read')
        if not kinds and loc == 'fn-entry' and re.search(r'\((?:thiscall|cdecl|stdcall|fastcall)', doc):
            kinds.append('call')
    elif loc == 'vtable-slot':
        if has_hook or has_w:
            kinds.append('hook')
        elif has_call:
            kinds.append('call')
        elif has_r:
            kinds.append('global-read')
    elif loc == 'vtable':
        kinds.append('global-read')
    elif loc.startswith('data'):
        if has_hook:
            kinds.append('hook')
        if has_w:
            kinds.append('global-write')
        if has_r or has_cmp:
            kinds.append('global-read')
        if not kinds and has_call:
            kinds.append('global-read')
            return 'global-read', '', 'address passed to a game function (object / this argument)'
    if not kinds:
        return 'unclear', '', 'no recognised access pattern in the code context'
    k0 = kinds[0]
    also = ';'.join(dict.fromkeys(k for k in kinds[1:] if k != k0))
    return k0, also, ''


def is_flag_like(v, line):
    return (v & (v - 1)) == 0 and re.search(r'[&|]|bits?|flag|mask', line, re.I) is not None


def collect_src_occurrences(files, defs, arrays, env):
    occ = []   # dicts: addr, file, line, name, test, def_line
    def_lines = {(f.rel, i) for lst in defs.values() for (f, i, _e, _t) in lst}
    seen_const_addr = set()
    for n, lst in defs.items():
        for (f, i, e, typ) in lst:
            v = eval_expr(e, env) if e else None
            if v is None or not isinstance(v, int) or not (GAME_LO <= v < GAME_HI):
                continue
            if re.search(r'\b(u8|u16|i16|i8|f32|f64)\b', typ):
                continue
            occ.append({'addr': v, 'file': f, 'line': i, 'name': n, 'test': f.test[i], 'via': 'const'})
            seen_const_addr.add((f.rel, i))
    for f in files:
        for i, c in enumerate(f.code):
            if (f.rel, i) in seen_const_addr:
                continue
            for m in HEX.finditer(c):
                try:
                    v = parse_int(m.group(1))
                except ValueError:
                    continue
                if not (GAME_LO <= v < GAME_HI):
                    continue
                if is_flag_like(v, c):
                    continue
                # offset added to a const base on the same line: `BASE + 0x...` keeps base only
                name = arrays.get((f.rel, i))
                occ.append({'addr': v, 'file': f, 'line': i, 'name': name, 'test': f.test[i],
                            'via': 'array' if name else 'inline'})
    return occ


def statement(f, i):
    """Code of the statement containing line i (back to the previous line ending in ';', '{' or '}', max 8 lines)."""
    lo = i
    while lo > 0 and i - lo < 8:
        p = f.code[lo - 1].rstrip()
        if not p.strip() or p.endswith((';', '{', '}')):
            break
        lo -= 1
    hi = i
    while hi < len(f.code) - 1 and hi - i < 6 and not f.code[hi].rstrip().endswith((';', '{', '}')):
        hi += 1
    return ' '.join(x.strip() for x in f.code[lo:hi + 1])


def src_context(files, o, cache):
    f, i, n = o['file'], o['line'], o['name']
    parts = [statement(f, i)]
    refs = []
    if n:
        if n not in cache:
            cache[n] = usage_lines(files, n, set())
        for (g, j) in cache[n]:
            if (g.rel, j) == (f.rel, i):
                continue
            if g.test[j]:
                continue
            # a use inside another const definition: follow that const one level
            parts.append(statement(g, j))
            refs.append(f'{g.rel}:{j + 1}')
    return parts, refs


# ============================================================================ samp.dll patch table
# Hand classification of every non-hook patch row (line number in gta-patches-r4-annotated.csv, header = 1).
# Value: (kind, feature). kind patch-disable = a game feature is switched off; patch-modify = behaviour/limit
# changed; global-write = a game variable is set. feature is filled where something is switched off.
# Every entry was read against the row's `effect` column (see README §2.2).
D, M, W = 'patch-disable', 'patch-modify', 'global-write'
PATCH_MANUAL = {
    4: (D, 'weapon lock-on (auto-aim)'), 5: (D, 'weapon lock-on (auto-aim)'),
    **{n: (M, 'pool / limit sizes') for n in (107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 131, 156)},
    120: (W, 'startup logos / intro movies'), 121: (D, 'startup logos / intro movies'),
    122: (M, 'loading-screen splash texture'), 123: (M, 'loading-screen splash texture'),
    124: (D, 'video-mode aspect-ratio filter'), 125: (D, 'video-mode aspect-ratio filter'),
    126: (M, 'timer source (QueryPerformanceCounter path off)'), 127: (M, 'streaming memory budget'),
    128: (D, 'game image-list registration (samp/archive registers IMGs)'),
    129: (D, 'loading-screen splash rectangle'), 130: (M, 'frame limiter 14 ms -> 2 ms'),
    **{n: (M, 'game-controlled vehicle lights (manual engine/lights mode only)') for n in range(140, 148)},
    155: (D, 'ambient gunfire audio'), 157: (D, 'zone-name display (CPlaceName::Process)'),
    158: (D, 'plant manager (CPlantMgr::Update)'), 159: (D, 'sun / moon coronas'),
    160: (D, 're-init on window focus regain (CGame::InitAfterLostFocus)'), 161: (D, 'HUD vehicle name'),
    162: (D, 'player idle animations (playidles block)'), 163: (D, 'replay (CReplay::Update)'),
    164: (D, 'interior ambient peds (CEntryExit::GenerateAmbientPeds)'),
    165: (D, 'real-time shadows (CRealTimeShadowManager::Update)'), 166: (M, 'stencil shadow alpha 50 -> 80'),
    167: (D, 'player anim-block release / move-anim switch in ProcessAnimGroups'),
    168: (M, 'remote CPlayerPed support (FindPlayerVehicle remote branch)'),
    169: (M, 'remote CPlayerPed support (CBike::ProcessAI Players[0] test)'),
    170: (D, 'event facial expressions'), 171: (D, "Pay'n'Spray sprayable check"),
    172: (D, 'IPL pickups (CFileLoader::LoadPickup)'),
    173: (M, 'remote CPlayerPed support (player-info lookups)'), 174: (M, 'remote CPlayerPed support (player-info lookups)'),
    175: (M, 'remote CPlayerPed support (player-info lookups)'), 176: (M, 'remote CPlayerPed support (player-info lookups)'),
    177: (D, 'speed blur (CPostEffects::SpeedFX)'), 178: (D, 'streaming mass RW-object deletion (DeleteAllRwObjects)'),
    179: (D, 'RW-object deletion after death (DeleteRwObjectsAfterDeath)'), 180: (D, 'single-player help boxes'),
    181: (D, 'car generators (CTheCarGenerators::Process)'), 182: (D, 'ambient planes (CPlane::DoPlaneGenerationAndRemoval)'),
    183: (M, 'CPlayerPed constructor ped-group setup condition'), 184: (D, 'real-time shadows (ReturnRealTimeShadow)'),
    185: (D, 'ambient population peds (CPopulation::AddPed)'), 186: (D, 'ambient population: seating peds created in cars'),
    187: (M, 'ProcessAnimGroups move-anim store (replaced by a hook)'),
    188: (D, 'CTrain::ProcessControl passenger block'), 189: (D, 'random traffic (CCarCtrl::GenerateRandomCars)'),
    190: (M, 'CPad::GetHandBrake ignores DisablePlayerControls'), 191: (D, "'Wasted' big message"),
    192: (D, 'ClearPlayerWeaponMode in ClearWeaponTarget'),
    193: (M, 'remote CPlayerPed support (player slot / remote vehicle test)'),
    **{n: (M, 'vehicle audio Service moved into the ProcessControl hook') for n in (194, 195, 196, 197)},
    198: (M, 'POINT_CAMERA_AT_CHAR always ped mode'), 199: (D, 'BMX fire player ignition'),
    200: (D, "sniper 'fire failed' sound"), 201: (D, "rocket 'fire failed' sound"),
    202: (M, 'HUD wanted level from samp'), 203: (D, 'menu map DrawRect'),
    204: (M, 'enter-car task does not register the vehicle reference'), 205: (D, 'HUD vital stats'),
    206: (M, 'CPed::UpdatePosition ignores standing-on entity'), 207: (D, 'flying-car physics (ProcessFlyingCarStuff)'),
    208: (D, 'interior pickups (Interior_c::AddPickups)'), 209: (M, 'DirectInput mouse non-exclusive'),
    212: (D, 'Interior_c::Exit in InteriorGroup_c::Exit'),
    213: (M, 'aiming-rotation stores (toggled per ped)'), 214: (M, 'aiming-rotation stores (toggled per ped)'),
    215: (D, 'CPlayerInfo::Process task block (task not identified)'), 217: (D, 'CPlayerInfo::Process task block (task not identified)'),
    216: (M, 'weapon rendered while holding an object'), 218: (D, 'CVehicle::SetDriver freebies'),
    **{n: (M, 'peds keep their weapon when entering a car') for n in (219, 220, 221)},
    222: (M, 'CPed::AttachPedToEntity returns early'),
    223: (D, 'cell-phone model'), 224: (D, 'cell-phone model'),
    **{n: (M, 'CFont::ProcessCurrentString larger stack buffer') for n in range(225, 240)},
    244: (D, 'idle camera (CIdleCam::Process)'), 245: (M, 'bird path LOS also tests objects'),
    246: (D, 'cutscenes (CCutsceneMgr::Update)'), 247: (D, 'peds reacting to guns aimed at them'),
    347: (W, 'world sector grid relocated'),
    349: (D, 'game keyboard input while samp UI owns input (runtime toggle)'),
    350: (D, 'game mouse input while samp UI owns input (runtime toggle)'),
    351: (D, 'game mouse input while samp UI owns input (runtime toggle)'),
    353: (D, 'RsMouseSetPos while samp shows the cursor (runtime toggle)'),
    354: (D, 'game clock (CClock::Update) while the HUD clock is off (runtime toggle)'),
    355: (W, 'HUD clock format string (runtime toggle)'),
    356: (D, 'stat changes (CStats::SetStatValue)'), 357: (W, ''), 358: (W, ''), 359: (W, ''),
    360: (M, 'HUD wanted level from samp'), 361: (M, 'police scanner swim/jet-pack checks'),
    362: (M, 'COMMAND_CREATE_OBJECT accepts negative model ids (temporary)'),
    363: (D, 'CJ clothes rebuild on skin change'),
    364: (D, 'ClumpCollisionStreamRead during samp model load (temporary)'),
    365: (D, 'camera fade at spawn'), 366: (M, 'CPed::UpdatePosition call toggled per ped'),
    367: (M, 'aiming-rotation stores (toggled per ped)'), 368: (M, 'screen-scale constants for wide aspect'),
    369: (M, 'HUD constants for wide aspect'), 370: (M, 'temporary float constant around a hooked call'),
    371: (M, 'temporary escalator float constants'), 372: (M, 'CStreaming::AddEntity call suppressed once'),
    373: (M, 'escalator switch-off suppressed once'), 374: (M, 'visibility tests suppressed once'),
    375: (D, 'CVehicle::SetupRender (made RET by a hook body, never restored)'),
    376: ('code-read', ''), 377: (W, 'main menu skipped'), 378: (W, 'main menu skipped'),
    382: ('dead', ''), 383: ('dead', ''), 384: ('dead', ''), 385: (W, ''),
}
# init-time entry RET / return-0 patches: the function body never runs -> cut from the closure
# (runtime and temporary RET patches are not cut; see README §3.1)
CUT_ENTRY_LINES = (4, 5, 128, 172, 178, 180, 185, 207)


def load_patches(cat):
    rows = []
    for ln, r in enumerate(csv.DictReader(open(PATCH_CSV)), 2):
        rows.append((ln, r))
    out = []
    for ln, r in rows:
        k = r['kind'].strip()
        kb = k.split('(')[0].strip()
        addrs = [int(t, 16) for t in re.findall(r'0x[0-9a-fA-F]+', r['gta_addr'])]
        if ln == 382:   # dead code row lists one site plus a .data table
            addrs = addrs[:1]
        if not addrs and '707 sites' in r['gta_addr']:
            # CModelInfo::ms_modelInfoPtrs relocation: the 707 code sites are only in samp.dll .data
            addrs = [0xA9B0C8]
        size = r['size'].strip()
        slot = re.search(r'GTA slot (0x[0-9a-f]+)', r['purpose'] + ' ' + r['effect'])
        tgt = re.search(r'\(A:(0x[0-9a-f]+)', k)
        orig_call = re.search(r'orig: (?:push [^;]+; )?(?:mov ecx, [^;]+; )?call (0x[0-9a-f]+)', r['effect'])
        if ln in PATCH_MANUAL:
            kind, feature = PATCH_MANUAL[ln]
        elif kb.startswith('hook') or kb == 'call-retarget':
            kind, feature = 'hook', ''
        elif kb.startswith('pointer-redirect'):
            t = int(tgt.group(1), 16) if tgt else 0
            if t and t < 0x10100000:
                kind, feature = 'hook', ''
            else:
                kind, feature = 'patch-modify', 'table / constant relocated into samp memory'
        else:
            kind, feature = 'unclassified', ''
        when = r['applied_when'].strip()
        for a in addrs:
            out.append({'line': ln, 'addr': a, 'kind': kind, 'feature': feature, 'raw_kind': k, 'size': size,
                        'when': when, 'slot': int(slot.group(1), 16) if slot else None,
                        'orig_call': int(orig_call.group(1), 16) if orig_call else None,
                        'gta_function': r['gta_function'], 'effect': r['effect']})
    return out


NEEDS = {'call': 'implement-1:1', 'hook': 'implement-1:1', 'patch-modify': 'implement-1:1',
         'struct-field': 'implement-1:1', 'scm-opcode': 'implement-1:1', 'global-read': 'implement-1:1',
         'global-write': 'implement-1:1', 'global-access': 'implement-1:1', 'code-read': 'n/a',
         'code-compare': 'implement-1:1', 'patch-operand': 'implement-1:1', 'patch-disable': 'option-off',
         'dead': 'n/a', 'unclear': 'unknown', 'unclassified': 'unknown'}


# ============================================================================ struct fields
PSDK_GAME = cg.PSDK / 'plugin_sa' / 'game_sa'


def psdk_layout():
    offs = collections.defaultdict(dict)   # class -> {offset: member}
    by_member = {}
    bases = {}
    for p in sorted(PSDK_GAME.rglob('*.h')):
        t = open(p, errors='replace').read()
        for m in re.finditer(r'VALIDATE_OFFSET\(\s*(\w+)\s*,\s*([\w.\[\]]+)\s*,\s*(0x[0-9A-Fa-f]+|\d+)\s*\)', t):
            o = int(m.group(3), 0)
            offs[m.group(1)].setdefault(o, m.group(2))
            by_member[(m.group(1), m.group(2))] = o
        for m in re.finditer(r'^(?:class|struct)\s+(?:PLUGIN_API\s+)?(\w+)\s*:\s*public\s+(\w+)', t, re.M):
            bases.setdefault(m.group(1), m.group(2))
    return offs, by_member, bases


FIELD_DOC = re.compile(r'`([A-Z]\w+)::(m_\w+|\w+)`|`([A-Z]\w+)`')
VAR_CLASS = [(r'^(ped|remote_ped|local_ped|p)$', 'CPed'), (r'^(veh|vehicle|car|v)$', 'CVehicle'),
             (r'^(intelligence|intel)$', 'CPedIntelligence'), (r'^(mat|matrix|m)$', 'CMatrix'),
             (r'^(object|obj)$', 'CObject'), (r'^(ent|entity|e)$', 'CEntity'), (r'^(player)$', 'CPlayerPed'),
             (r'^(cam|camera)$', 'CCamera'), (r'^(pad)$', 'CPad'), (r'^(col)$', 'CColModel')]


def class_chain(c, bases):
    out = []
    while c and c not in out:
        out.append(c)
        c = bases.get(c)
    return out


def collect_fields(files, defs, env, layout):
    """Struct-field accesses: (a) offset consts documented with a `Class::member`, (b) inline `var + 0xNN`
    in a read/write context. Returns list of dict rows."""
    offs, by_member, bases = layout
    rows = []
    for n, lst in defs.items():
        v = env.get(n)
        if not isinstance(v, int) or v >= 0x10000 or v < 0:
            continue
        for (f, i, e, typ) in lst:
            if f.test[i] or not re.search(r'\b(u32|usize|i32|isize)\b', typ):
                continue
            doc = f.doc_above(i)
            m = re.search(r'`([A-Z]\w+)::(\w+)`', doc)
            if not m:
                m2 = re.search(r'`([A-Z][A-Za-z]+)`[^`]*(?:\+0x|offset|field|slot)', doc)
                if not (m2 and re.search(r'(?:OFF|FIELD|_OFFSET|^VT_|SLOT)', n)):
                    continue
                cls, mem = m2.group(1), ''
            else:
                cls, mem = m.group(1), m.group(2)
            # is the name used as an offset somewhere (x + NAME / NAME + x)?
            used = usage_lines(files, n, {(f.rel, i)})
            addp = re.compile(r'\+\s*(?:\w+::)*' + re.escape(n) + r'\b|\b' + re.escape(n) + r'\s*\+')
            used = [(g, j) for (g, j) in used if addp.search(g.code[j])]
            if not used:
                continue
            status, psdk_member = verify_field(cls, mem, v, offs, by_member, bases)
            rows.append({'cls': cls, 'member': mem, 'off': v, 'name': n, 'file': f, 'line': i, 'how': 'doc',
                         'status': status, 'psdk': psdk_member,
                         'refs': [f'{g.rel}:{j + 1}' for (g, j) in used if not g.test[j]]})
    pat = re.compile(r'\b([a-z_][a-z0-9_]*)\s*\+\s*(0x[0-9A-Fa-f]{1,4})\b')
    for f in files:
        for i, c in enumerate(f.code):
            if f.test[i] or 'naked_asm' in c or '"' in c and re.search(r'"[^"]*\+\s*0x', c):
                continue
            st = statement(f, i)
            if not (READ_RE.search(st) or WRITE_RE.search(st) or re.search(r'\brd\w*\(|\bwr\w*\(|v3\(', st)):
                continue
            for m in pat.finditer(c):
                var, o = m.group(1), int(m.group(2), 16)
                cls = next((k for (rx, k) in VAR_CLASS if re.match(rx, var)), '')
                status, psdk_member = verify_field(cls, '', o, offs, by_member, bases) if cls else ('class-unknown', '')
                rows.append({'cls': cls, 'member': '', 'off': o, 'name': var, 'file': f, 'line': i,
                             'how': 'inline(var-name class guess)' if cls else 'inline', 'status': status,
                             'psdk': psdk_member, 'refs': []})
    return rows


def verify_field(cls, mem, off, offs, by_member, bases):
    chain = class_chain(cls, bases)
    if mem:
        for c in chain:
            if (c, mem) in by_member:
                return ('psdk-match' if by_member[(c, mem)] == off else 'psdk-offset-differs'), f'{c}::{mem}@{by_member[(c, mem)]:#x}'
    for c in chain:
        if off in offs.get(c, {}):
            return 'psdk-offset-match', f'{c}::{offs[c][off]}'
    return ('not-in-psdk' if chain and any(c in offs for c in chain) else 'class-not-in-psdk'), ''


# ============================================================================ SCM opcodes
def collect_opcodes(files):
    """Opcodes samp-rs executes through CRunningScript::ProcessOneCommand (script.rs / worldents.rs `op`
    modules) plus the two custom opcodes it installs. Only constants that are used outside tests count."""
    out = {}
    for f in files:
        in_op = False
        depth = 0
        for i, c in enumerate(f.code):
            if re.match(r'\s*pub mod op\s*\{', c):
                in_op, depth = True, 0
            if in_op:
                depth += c.count('{') - c.count('}')
                m = re.match(r'\s*pub const ([A-Z0-9_]+):\s*u16\s*=\s*0x([0-9A-Fa-f]{4});', c)
                if m:
                    out[m.group(1)] = (int(m.group(2), 16), f, i)
                if depth <= 0 and '}' in c:
                    in_op = False
    rows = []
    for n, (op, f, i) in sorted(out.items(), key=lambda x: x[1][0]):
        used = [(g, j) for (g, j) in usage_lines(files, n, {(f.rel, i)}) if not g.test[j]]
        rows.append({'op': op, 'name': n, 'file': f, 'line': i, 'used': used})
    return rows


# ============================================================================ main: rows
NOT_ADDRESS = {   # values in the image range that the code uses as something else (checked by hand)
    'STREAM_FLAGS': 'BASS stream flags 0x940000',
}
# hand decisions for occurrences the rules cannot classify (address -> (kind, note)); see README §2.3
SRC_MANUAL = {
    0xB6BC90: ('global-read', 'AudioEngine object passed as `this` to game audio functions'),
    0x8AC15C: ('global-read', 'AEAmbienceTrackManager object passed as `this` to game functions'),
    0xC15448: ('global-read', 'g_ikChainMan object passed as `this` to a game function'),
    0xBA6748: ('global-write', 'FrontEndMenuManager fields +0xE9/+0x5C/+0x33 written at state 8 (patch rows 377/378)'),
    0xB4E480: ('patch-operand', 'original colour-table address expected at the redirected operands'),
    0x9788C0: ('global-read', 'CPickups::aPickUps base used to turn a pickup pointer into an index'),
    0x586650: ('hook', 'original callee at the two hooked call sites (rows 41/42)'),
    0xC73FA0: ('global-write', 'COcclusion::Occluders records read and zeroed (RemoveBuildingForPlayer)'),
    0xB744A0: ('global-read', 'CPools::ms_pDummyPool read'),
}


def build_rows():
    cat = Catalogue()
    patches = load_patches(cat)
    files, defs, arrays, env = scan_sources()
    occ = collect_src_occurrences(files, defs, arrays, env)
    pidx = collections.defaultdict(list)
    ranges = []
    for p in patches:
        pidx[p['addr']].append(p)
        if p['slot']:
            pidx[p['slot']].append(p)
        try:
            n = int(p['size'])
        except ValueError:
            n = 0
        if n:
            ranges.append((p['addr'], p['addr'] + n, p))
    def plink(a):
        if a in pidx:
            return pidx[a], 'site'
        hit = [p for (lo, hi, p) in ranges if lo < a <= hi]
        return hit, 'inside/after site' if hit else ''

    dropped = collections.Counter()
    cache = {}
    groups = collections.OrderedDict()
    for o in occ:
        a = o['addr']
        if not in_game_image(a):
            dropped['outside game sections (colours/constants)'] += 1
            continue
        if o['name'] in NOT_ADDRESS:
            dropped[NOT_ADDRESS[o['name']]] += 1
            continue
        line = o['file'].code[o['line']]
        if re.search(r'\.\.=?\s*0x[0-9a-f_]+\)\.contains|0x[0-9A-Fa-f_]+\.\.=?\s*0x', line):
            dropped['range bound in a plausibility check'] += 1
            continue
        res = cat.resolve(a)
        if o['via'] == 'const' and o['name']:
            if o['name'] not in cache:
                cache[o['name']] = usage_lines(files, o['name'], set())
            uses = [(g, j) for (g, j) in cache[o['name']] if (g.rel, j) != (o['file'].rel, o['line']) and not g.test[j]]
            if uses and all(CONST.match(g.code[j]) for (g, j) in uses) and a not in SRC_MANUAL:
                dropped['base of derived constants only (the derived addresses are listed)'] += 1
                continue
        parts, refs = src_context(files, o, cache)
        doc = o['file'].doc_above(o['line'])
        kind, also, note = classify_src(a, res, parts, doc, o['name'], line)
        prow, how = plink(a)
        if not prow:
            st = parts[0]
            linked = []
            for m in HEX.finditer(st):
                try:
                    b = parse_int(m.group(1))
                except ValueError:
                    continue
                if b != a and b in pidx:
                    linked += pidx[b]
            if linked:
                prow, how = linked, 'operand of a patch statement'
        if how == 'site' and kind != 'call' and a not in SRC_MANUAL:
            pk = {p['kind'] for p in prow if p['kind'] != 'dead'}
            if len(pk) == 1:
                kind, also = pk.pop(), ''
                note = 'implements the samp.dll patch row(s); kind taken from the row'
        if a in SRC_MANUAL:
            kind, note = SRC_MANUAL[a]
            also = ''
        elif kind == 'unclear' and prow:
            kind = 'patch-operand'
            note = f'value in the statement of samp patch row(s) ({how}): original target / relocated object'
        key = (a, kind)
        g = groups.setdefault(key, {'addr': a, 'kind': kind, 'also': set(), 'res': res, 'src': [], 'test_src': [],
                                    'names': set(), 'notes': set(), 'patch_lines': set()})
        if also:
            g['also'].update(also.split(';'))
        if note:
            g['notes'].add(note)
        if o['name']:
            g['names'].add(o['name'])
        loc = f"{o['file'].rel}:{o['line'] + 1}"
        (g['test_src'] if o['test'] else g['src']).append(loc)
        for p in prow:
            g['patch_lines'].add(p['line'])
    rows = []
    for (a, kind), g in groups.items():
        if not g['src']:
            continue   # test-only occurrences are reported in aggregates, not as rows
        res = g['res']
        rows.append({
            'source': 'samp-rs', 'address': hx(a), 'kind': kind, 'also': ';'.join(sorted(g['also'] - {kind})),
            'loc': res['loc'], 'function': res['func'], 'function_name': res['func_name'], 'subsystem': res['subsystem'],
            'global': res['global'], 'feature': '', 'needs': NEEDS.get(kind, 'unknown'),
            'patch_rows': ';'.join(str(x) for x in sorted(g['patch_lines'])), 'applied_when': '',
            'samp_rs_name': ';'.join(sorted(g['names'])), 'src': ';'.join(g['src']),
            'note': '; '.join(sorted(g['notes']))})
    test_only = sorted({a for (a, k), g in groups.items() if not g['src']} - {a for (a, k), g in groups.items() if g['src']})
    # samp.dll patch rows
    src_by_addr = collections.defaultdict(set)
    for (a, k), g in groups.items():
        for s in g['src']:
            src_by_addr[a].add(s)
    for p in patches:
        a = p['addr']
        res = cat.resolve(a)
        note = []
        if p['orig_call']:
            note.append(f"replaced instruction calls {hx(p['orig_call'])} {cat.fname(p['orig_call'])}")
        if res['loc'] == 'vtable-slot' and res['func']:
            note.append(f"slot held {res['func']} {res['func_name']}")
        if p['slot']:
            note.append(f"indirect-jmp slot {hx(p['slot'])}")
        needs = NEEDS.get(p['kind'], 'unknown')
        if p['when'].startswith('never'):
            needs = 'n/a'
            note.append('dead code in samp.dll (never applied)')
        rs = sorted(src_by_addr.get(a, set()) | (src_by_addr.get(p['slot'], set()) if p['slot'] else set()))
        rows.append({
            'source': 'samp.dll-patch', 'address': hx(a), 'kind': p['kind'], 'also': '', 'loc': res['loc'],
            'function': res['func'], 'function_name': res['func_name'], 'subsystem': res['subsystem'],
            'global': res['global'], 'feature': p['feature'], 'needs': needs, 'patch_rows': str(p['line']),
            'applied_when': p['when'], 'samp_rs_name': '',
            'src': ';'.join([f"samp-rs/docs/re/data/gta-patches-r4-annotated.csv:{p['line']}"] + [f'impl {x}' for x in rs]),
            'note': '; '.join(note), '_patch': p})
    return cat, patches, files, defs, env, rows, dropped, test_only


# ============================================================================ closure
FOLLOW = ('call', 'tailcall', 'virtual', 'virtual-override', 'indirect-ghidra', 'eh-unwind', 'fnref')
GAME_ROOTS = {0x748710: 'Win::NOTSA_WinMain (game entry after CRT start-up)',
              0x747eb0: 'Win::__MainWndProc (window procedure, called by the OS)'}


def live(cat, f):
    r = cat.proc.get(f)
    return r is not None and r['state'] != 'excluded' and not r['subsystem'].startswith('X-')


def table_refs(cat):
    """{table start: set(referencing functions)} from rwsets.csv, and {table start: [slot functions]}."""
    refs = collections.defaultdict(set)
    for r in csv.DictReader(open(CAT / 'rwsets.csv')):
        a = int(r['address'], 16)
        t = cat.table(a)
        if t:
            refs[t[0]].add(int(r['function'], 16))
    members = {}
    for (lo, hi, kind, name) in cat.tables:
        ms = []
        for x in range(lo, hi, 4):
            v = cat.u32(x)
            if v in cat.proc:
                ms.append(v)
        members[lo] = ms
    return refs, members


def fallthrough_edges(cat):
    """Edges f -> g where a body range of f ends exactly at the entry of catalogue function g and the last
    instruction of that range does not end the flow (ret / jmp / int3): f falls through into g.
    callgraph.csv has no edge for this (e.g. Win::NOTSA_WinMain falls into the entry gta-reversed names
    Input::Initialise at 0x7487cf, which holds the rest of WinMain)."""
    out = set()
    for (lo, hi, fa) in cat.ivs:
        if hi == fa or hi not in cat.proc:
            continue
        last = None
        for ins in cat.img.insns(lo, hi):
            last = ins
        if last is not None and last.mnemonic not in ('ret', 'retn', 'jmp', 'int3', 'ljmp'):
            out.add((fa, hi))
    return out


def closure(cat, roots, cut, removed_edges, tables=None, extra=None):
    seen = set()
    stack = [r for r in roots if r not in cut]
    inst = set()
    by_ref = collections.defaultdict(list)
    if tables:
        refs, members = tables
        for t, fs in refs.items():
            for f in fs:
                by_ref[f].append(t)
    while stack:
        f = stack.pop()
        if f in seen or f in cut or not live(cat, f):
            continue
        seen.add(f)
        for (b, kind, sites) in cat.edges.get(f, ()):
            if kind not in FOLLOW or (f, b) in removed_edges:
                continue
            if b not in seen:
                stack.append(b)
        if extra:
            stack.extend(b for b in extra.get(f, ()) if b not in seen)
        if tables:
            for t in by_ref.get(f, ()):
                if t in inst:
                    continue
                inst.add(t)
                stack.extend(m for m in tables[1].get(t, ()) if m not in seen)
    return seen


# ============================================================================ outputs
COLS = ['id', 'source', 'address', 'kind', 'also', 'loc', 'function', 'function_name', 'subsystem', 'global',
        'feature', 'needs', 'patch_rows', 'applied_when', 'samp_rs_name', 'src', 'note']


def main():
    cat, patches, files, defs, env, rows, dropped, test_only = build_rows()
    layout = psdk_layout()
    fields = collect_fields(files, defs, env, layout)
    ops = collect_opcodes(files)
    sc = {int(r['opcode'], 16): r for r in csv.DictReader(open(CAT / 'script-commands.csv'))}
    # struct-field rows (grouped by class/member/offset)
    fg = collections.OrderedDict()
    for fr in fields:
        key = (fr['cls'], fr['member'], fr['off'], fr['how'].split('(')[0])
        g = fg.setdefault(key, {'fr': fr, 'src': [], 'names': set()})
        g['src'].append(f"{fr['file'].rel}:{fr['line'] + 1}")
        g['names'].add(fr['name'])
    for (cls, mem, off, how), g in fg.items():
        fr = g['fr']
        rows.append({'source': 'samp-rs', 'address': f'+{off:#x}', 'kind': 'struct-field', 'also': '',
                     'loc': 'field', 'function': '', 'function_name': '', 'subsystem': '',
                     'global': f"{cls or '?'}::{mem or '?'}", 'feature': '', 'needs': 'implement-1:1',
                     'patch_rows': '', 'applied_when': '', 'samp_rs_name': ';'.join(sorted(g['names']))[:120],
                     'src': ';'.join(g['src']),
                     'note': f"{fr['how']}; plugin-sdk check: {fr['status']}" + (f" ({fr['psdk']})" if fr['psdk'] else '')})
    # SCM opcode rows
    for o in ops:
        s = sc.get(o['op'], {})
        used = [f'{g.rel}:{j + 1}' for (g, j) in o['used']]
        rows.append({'source': 'samp-rs', 'address': f"op:{o['op']:04X}", 'kind': 'scm-opcode', 'also': '',
                     'loc': 'scm', 'function': s.get('orig_handler', ''), 'function_name': s.get('name', ''),
                     'subsystem': 'SCRIPT', 'global': '', 'feature': '',
                     'needs': 'implement-1:1' if used else 'n/a',
                     'patch_rows': '', 'applied_when': '', 'samp_rs_name': o['name'],
                     'src': ';'.join([f"{o['file'].rel}:{o['line'] + 1}"] + used),
                     'note': ('' if used else 'declared, not used outside tests; ') +
                             f"gr_status={s.get('gr_status', '?')} range_fn={s.get('orig_range_fn', '')}"})
    for p in patches:
        if p['line'] in (2, 3):
            pass
    # ids + write
    order = {'samp.dll-patch': 0, 'samp-rs': 1}
    rows.sort(key=lambda r: (order[r['source']], r['kind'], r['address']))
    for i, r in enumerate(rows, 1):
        r['id'] = i
    with open(OUT_CSV, 'w', newline='') as fh:
        w = csv.DictWriter(fh, fieldnames=COLS, extrasaction='ignore', lineterminator='\n')
        w.writeheader()
        for r in rows:
            w.writerow(r)
    aggregate(cat, patches, rows, dropped, test_only, fields, ops)


def fn_list(s):
    return [int(x, 16) for x in s.split(';') if x.startswith('0x')] if s else []


def aggregate(cat, patches, rows, dropped, test_only, fields, ops):
    agg = {}
    srows = [r for r in rows if r['source'] == 'samp-rs']
    prow = [r for r in rows if r['source'] == 'samp.dll-patch']
    addr_rows = [r for r in srows if r['kind'] not in ('struct-field', 'scm-opcode')]
    agg['samp_rs_distinct_addresses'] = len({r['address'] for r in addr_rows})
    agg['samp_rs_rows_by_kind'] = dict(collections.Counter(r['kind'] for r in srows))
    agg['samp_rs_test_only_addresses'] = len(test_only)
    agg['samp_rs_dropped_values'] = dict(dropped)
    agg['patch_rows'] = len({r['patch_rows'] for r in prow})
    agg['patch_entries_by_kind'] = dict(collections.Counter(r['kind'] for r in prow))
    agg['patch_rows_implemented_in_samp_rs'] = len({r['patch_rows'] for r in prow if 'impl ' in r['src']})
    # ---- direct set
    direct = collections.defaultdict(set)   # function -> reasons
    cut, removed, cut_why = set(), set(), {}
    for r in rows:
        if r['needs'] in ('n/a',) or r['kind'] in ('dead', 'code-read'):
            continue
        fs = fn_list(r['function'])
        if r['kind'] == 'scm-opcode':
            continue   # handled below (handler functions)
        p = r.get('_patch')
        if p and p['line'] in CUT_ENTRY_LINES and r['loc'] == 'fn-entry':
            for f in fs:
                cut.add(f)
                cut_why[f] = f"row {p['line']}: {p['feature']}"
            continue
        for f in fs:
            if live(cat, f):
                direct[f].add(r['kind'])
        if p and p['orig_call']:
            if r['kind'] == 'patch-disable' and not p['when'].startswith('runtime'):
                for f in fs:
                    sites = [s for (b, k, s) in cat.edges.get(f, ()) if b == p['orig_call'] and k == 'call']
                    if sites and sum(sites) == 1:
                        removed.add((f, p['orig_call']))
            elif r['kind'] == 'hook' and live(cat, p['orig_call']):
                direct[p['orig_call']].add('hooked-callee')
    # opcode handlers: the range function of every opcode samp-rs executes, plus ProcessOneCommand
    sc = {int(r['opcode'], 16): r for r in csv.DictReader(open(CAT / 'script-commands.csv'))}
    for o in ops:
        if not o['used']:
            continue
        rf = sc.get(o['op'], {}).get('orig_range_fn', '')
        if rf.startswith('0x') and live(cat, int(rf, 16)):
            direct[int(rf, 16)].add('scm-opcode-range')
    agg['cut_functions'] = {hx(f): f'{cat.fname(f)} ({w})' for f, w in sorted(cut_why.items())}
    agg['removed_call_edges'] = [f'{hx(a)} {cat.fname(a)} -> {hx(b)} {cat.fname(b)}' for (a, b) in sorted(removed)]
    tables = table_refs(cat)
    ft = fallthrough_edges(cat)
    extra = collections.defaultdict(list)
    for (a, b) in sorted(ft):
        extra[a].append(b)
    agg['fallthrough_edges'] = [f'{hx(a)} {cat.fname(a)} -> {hx(b)} {cat.fname(b)}' for (a, b) in sorted(ft)]
    # a function that is direct only because it contains a samp disable site is a root only if the rest of
    # the closure reaches it (otherwise the patch is moot: nothing calls the container)
    core = {f for f, k in direct.items() if k - {'patch-disable'}}
    moot = set()
    base_e = closure(cat, core | set(GAME_ROOTS), cut, removed, tables, extra)
    for f, k in direct.items():
        if not (k - {'patch-disable'}) and f not in base_e and f not in cut:
            moot.add(f)
    agg['direct_functions'] = len([f for f in direct if f not in cut])
    agg['moot_disable_containers'] = [f'{hx(f)} {cat.fname(f)}' for f in sorted(moot)]
    for f in cut:
        direct.pop(f, None)
    droots = set(direct) - moot
    allf = [f for f in cat.proc if live(cat, f)]
    sub_total = collections.Counter(cat.subsystem(f) for f in allf)
    res = {}
    variants = {
        'A_direct': set(direct),
        'B_samp_roots_resolved_edges': closure(cat, droots, cut, removed, None, extra),
        'C_samp_roots_plus_instantiated_vtables': closure(cat, droots, cut, removed, tables, extra),
        'D_game_and_samp_roots_resolved_edges': closure(cat, droots | set(GAME_ROOTS), cut, removed, None, extra),
        'E_game_and_samp_roots_plus_instantiated_vtables': closure(cat, droots | set(GAME_ROOTS), cut, removed, tables, extra),
        'F_E_without_samp_cuts': closure(cat, set(direct) | set(GAME_ROOTS), set(), set(), tables, extra),
        'G_game_roots_only_plus_instantiated_vtables': closure(cat, set(GAME_ROOTS), set(), set(), tables, extra),
    }
    for k, s in variants.items():
        c = collections.Counter(cat.subsystem(f) for f in s)
        res[k] = {'total': len(s), 'by_subsystem': {sub: [c.get(sub, 0), sub_total[sub]] for sub in sorted(sub_total)}}
    res['catalogue_live_total'] = len(allf)
    res['only_reachable_without_cuts'] = len(variants['F_E_without_samp_cuts'] - variants['E_game_and_samp_roots_plus_instantiated_vtables'])
    unl = variants['F_E_without_samp_cuts'] - variants['E_game_and_samp_roots_plus_instantiated_vtables']
    res['only_reachable_without_cuts_by_subsystem'] = dict(collections.Counter(cat.subsystem(f) for f in unl))
    agg['closure'] = res
    # per-function output
    with open(OUT_DIR / 'required-functions.csv', 'w', newline='') as fh:
        w = csv.writer(fh, lineterminator='\n')
        w.writerow(['address', 'name', 'subsystem', 'state', 'direct', 'direct_kinds', 'in_B', 'in_C', 'in_D', 'in_E',
                    'cut_by_samp', 'unreachable_after_samp_cuts'])
        E = variants['E_game_and_samp_roots_plus_instantiated_vtables']
        for f in sorted(set(allf)):
            inE = f in E
            row = [hx(f), cat.fname(f), cat.subsystem(f), cat.proc[f]['state'], int(f in direct),
                   ';'.join(sorted(direct.get(f, ()))), int(f in variants['B_samp_roots_resolved_edges']),
                   int(f in variants['C_samp_roots_plus_instantiated_vtables']),
                   int(f in variants['D_game_and_samp_roots_resolved_edges']), int(inE), int(f in cut), int(f in unl)]
            w.writerow(row)
    # disabled features
    feats = collections.OrderedDict()
    for p in patches:
        if p['kind'] == 'patch-disable':
            feats.setdefault(p['feature'], []).append(f"{p['line']}@{hx(p['addr'])}" + (' (runtime)' if p['when'].startswith('runtime') else ''))
    agg['disabled_features'] = feats
    # opcodes
    agg['scm_opcodes_used'] = [f"{o['op']:04X} {o['name']}" for o in ops if o['used']]
    agg['scm_opcodes_declared_unused'] = [f"{o['op']:04X} {o['name']}" for o in ops if not o['used']]
    # unresolved
    unres = []
    for r in rows:
        if r['kind'] in ('unclear', 'unclassified'):
            unres.append(f"{r['address']} {r['kind']} {r['src'][:80]}")
        elif r['loc'] in ('code-unmapped',) and r['kind'] not in ('hook',):
            unres.append(f"{r['address']} code outside every catalogue body ({r['kind']}) {r['src'][:80]}")
        elif r['loc'] in ('fn-body', 'fn-entry') and r['function'] and not r['function_name']:
            unres.append(f"{r['address']} in unnamed catalogue function {r['function']} ({r['kind']})")
        elif r['loc'].startswith('data') and not r['global'] and r['kind'] != 'struct-field':
            unres.append(f"{r['address']} data without a catalogue/global name ({r['kind']}, {r['loc']}) {r['src'][:60]}")
    agg['unresolved'] = sorted(set(unres))
    agg['code_unmapped_hook_slots'] = sorted({r['address'] for r in rows if r['loc'] == 'code-unmapped'})
    fc = collections.Counter(fr['status'] for fr in fields)
    agg['struct_fields'] = {'occurrences': len(fields), 'by_status': dict(fc),
                            'distinct': len({(fr['cls'], fr['member'], fr['off']) for fr in fields})}
    with open(OUT_DIR / 'aggregates.json', 'w') as fh:
        json.dump(agg, fh, indent=1, sort_keys=True)
        fh.write('\n')


if __name__ == '__main__':
    main()
