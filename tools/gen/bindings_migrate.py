"""DEDUP-1 pass 2: merge every remaining `orig!` declaration of an original address into its one binding
(`tools/gen/bindings.py --resolve`), with the binding's signature taken from the original.

For each address declared more than once outside the binding modules (or already bound), the declarations
that can share one binding (literal-address forms; types primitive, `GPtr<..>` of them, arrays, or the
workspace's own `CVector` / `X87` by full path; local aliases such as `type Ent = GPtr<u8>` resolved) are
merged into one signature:

- convention and stack arguments: the declarations' when they agree, else the original's (`tools/lib/abi.py`:
  the callee's `ret N`, whether it reads ECX, `add esp, N` at its call sites);
- each argument: a pointer (`GPtr<..>`) when any declaration passes one, else `f32` when any passes a float,
  else an integer as wide as the original's callee reads it (byte / word / dword), signed as most declarations
  have it;
- the result: as the original's callers use it (AL only: `u8`; EAX: 32 bits; ST0: the widest float a
  declaration reads), a pointer when any declaration has one;
- the `=>` target: the port when one exists and the binding crate reaches it.

Every merged declaration becomes `use <crate>::bindings::<name> as <local>;` and each call of it in its module
(and in the modules of its crate that import a `pub` declaration) converts at the call expression: the
arguments from the type the caller used to the binding's (`GPtr::from_addr(x as usize)`, `p.addr() as u32`,
`f32::from_bits(w)`, `x as u8`, ...), the result back to the caller's type. Port logic is not touched.

Addresses whose declarations cannot share one signature by these rules (C varargs: printf / sscanf / sprintf /
RwError, ...) stay declared per caller; they are listed in VARARGS with the reason.
"""
import collections
import os
import re
import sys

import bindings as B

sys.path.insert(0, os.path.join(B.ROOT, 'tools', 'lib'))

# C varargs (declared per caller): `bindings.VARARGS`
VARARGS = B.VARARGS
INTS = ('u8', 'i8', 'u16', 'i16', 'u32', 'i32', 'usize', 'isize', 'u64', 'i64')
WIDTH = {'u8': 1, 'i8': 1, 'bool': 1, 'u16': 2, 'i16': 2, 'u32': 4, 'i32': 4, 'usize': 4, 'isize': 4, 'f32': 4}
EXTRA_TYPES = {'CVector': 'sa_math::vector::CVector', 'X87': 'sa_math::x87::X87'}


# ---- module paths and target resolution ---------------------------------------------------------------
def module_of(rel):
    """`crates/sa-x/src/a/b.rs` -> `sa_x::a::b` (lib.rs / main.rs / mod.rs name their directory)."""
    parts = rel.split(os.sep)
    crate = B.crate_mod(parts[1])
    sub = parts[3:]
    if sub[-1] in ('lib.rs', 'main.rs', 'mod.rs'):
        sub = sub[:-1]
    else:
        sub[-1] = sub[-1][:-3]
    return '::'.join([crate] + sub)


def inline_mods(s, pos):
    """Names of the inline `mod x { .. }` blocks enclosing `pos`."""
    out = []
    for m in re.finditer(r'\bmod\s+(\w+)\s*\{', s[:pos]):
        b = m.end() - 1
        e = B.close_brace(s, b)
        if b < pos < e:
            out.append(m.group(1))
    return out


def use_map(s):
    """Local name -> absolute path of the file's `use` items (top level and nested groups, one level)."""
    out = {}
    for m in re.finditer(r'\buse\s+([^;]+);', s):
        body = ' '.join(m.group(1).split())
        if '{' in body:
            head, rest = body.split('{', 1)
            head = head.rstrip(':').rstrip(':').strip()
            rest = rest.rsplit('}', 1)[0]
            items = [x.strip() for x in B.split_args(rest)]
            for it in items:
                if '{' in it or it == 'self':
                    continue
                name, _, alias = it.partition(' as ')
                out[(alias or name.split('::')[-1]).strip()] = head.rstrip(':') + '::' + name.strip()
        else:
            name, _, alias = body.partition(' as ')
            out[(alias or name.split('::')[-1]).strip()] = name.strip()
    return out


def absolute(path, modpath, uses):
    """An item path as written in module `modpath` -> absolute (`sa_x::..`), or None."""
    parts = path.split('::')
    if parts[0] == 'crate':
        return modpath.split('::')[0] + '::' + '::'.join(parts[1:])
    if parts[0] == 'self':
        return modpath + '::' + '::'.join(parts[1:])
    if parts[0] == 'super':
        m = modpath.split('::')
        while parts and parts[0] == 'super':
            m = m[:-1]
            parts = parts[1:]
        return '::'.join(m + parts)
    if parts[0].startswith('sa_'):
        return path
    if parts[0] in uses:
        a = absolute(uses[parts[0]], modpath, {})
        return a and '::'.join([a] + parts[1:])
    return modpath + '::' + path


def resolve_target_abs(d, s, deps):
    if d.target == 'unported':
        return 'unported'
    mp = module_of(d.rel)
    mods = inline_mods(s, d.span[0])
    if mods:
        mp = mp + '::' + '::'.join(mods)
    t = absolute(d.target, mp, use_map(s))
    if t and t.split('::')[0].replace('_', '-') in deps:
        return t
    return None


# ---- types -----------------------------------------------------------------------------------------
def alias_map(s, uses, global_aliases):
    out = {}
    for m in re.finditer(r'^\s*(?:pub(?:\([^)]*\))?\s+)?type\s+(\w+)\s*=\s*((?:[^;\[]|\[[^\]]*\])+);', s, re.M):
        out[m.group(1)] = B.norm_type(m.group(2))
    for k, v in uses.items():
        last = v.split('::')[-1]
        if k not in out and last in global_aliases:
            out[k] = global_aliases[last]
    # names a glob import brings in: the workspace's unique `pub type` aliases of simple types
    for k, v in global_aliases.items():
        if k not in out and SIMPLE.match(v):
            out[k] = v
    return out


def global_aliases():
    seen = collections.defaultdict(set)
    for p in B.rust_files():
        for m in re.finditer(r'^\s*pub\s+type\s+(\w+)\s*=\s*((?:[^;\[]|\[[^\]]*\])+);', open(p, encoding='utf-8').read(), re.M):
            seen[m.group(1)].add(B.norm_type(m.group(2)))
    return {k: v.pop() for k, v in seen.items() if len(v) == 1}


def norm(t, aliases):
    t = B.norm_type(t)
    for _ in range(3):
        t = re.sub(r'\b(?:sa_core::)?mem::GPtr\b', 'GPtr', t)
        t = re.sub(r'\bsa_core::GPtr\b', 'GPtr', t)
        for k, v in aliases.items():
            t = re.sub(r'(?<![\w])(?:(?:super|self|crate|sa_\w+|\w+)::)*%s(?![\w])' % re.escape(k), v, t)
    t = re.sub(r'(?<![\w:])(?:sa_math::(?:vector::)?)?CVector\b', 'sa_math::vector::CVector', t)
    t = re.sub(r'(?<![\w:])(?:sa_math::)?(?:x87::)?X87\b', 'sa_math::x87::X87', t)
    return t


SIMPLE = re.compile(r'^(?:GPtr<sa_[a-z_0-9]+(?:::\w+)+>|%s|sa_math::vector::CVector|sa_math::x87::X87|GPtr<(?:%s|sa_math::vector::CVector|GPtr<%s>|\[%s;\d+\])>|\[%s;\d+\]|\(\))$'
                    % ((B.PRIM,) * 5))


def is_ptr(t):
    return t.startswith('GPtr<')


TYPED_PTR = re.compile(r'^GPtr<(?:[A-Za-z_][\w:]*)>$')


def simple(t):
    """A type a binding can carry (or, for a typed struct pointer, convert from with `.cast()`)."""
    return SIMPLE.match(t) is not None or TYPED_PTR.match(t) is not None


# ---- conversions at the call site --------------------------------------------------------------------
def atom(e):
    e = e.strip()
    if re.fullmatch(r'\w+', e) or re.fullmatch(r'[\w:.]+', e):
        return e
    if re.fullmatch(r'[\w:.]+(?:::<[^()]*>)?\(.*\)', e, re.S):
        # one call: the first `(` must close at the end
        i = e.index('(')
        if close_paren(e, i) == len(e) - 1:
            return e
    if e.startswith('(') and close_paren(e, 0) == len(e) - 1:
        return e
    return '(' + e + ')'


def close_paren(s, i):
    d = 0
    j = i
    while j < len(s):
        c = s[j]
        if c == '"':
            j += 1
            while j < len(s) and s[j] != '"':
                j += 2 if s[j] == '\\' else 1
        elif c in '([{':
            d += 1
        elif c in ')]}':
            d -= 1
            if d == 0:
                return j
        j += 1
    return -1


def bare(v):
    """`v` without one pair of outer parentheses."""
    v = v.strip()
    if v.startswith('(') and close_paren(v, 0) == len(v) - 1:
        return v[1:-1]
    return v


NON_GENERIC = set()  # readers the current file defines itself without type parameters
GENERIC_READERS = ('rd', 'g', 'gu', 'rdp', 'rv', 'ld')


def typed(e, t):
    """`e` as an operand of `as`: a call of a generic reader (`rd(p, 4)`) gets the type it returned under the
    old declaration (`rd::<i32>(p, 4)`), since `as` does not infer it."""
    e = e.strip()
    m = re.match(r'(\w+)\(', e)
    if m and m.group(1) in GENERIC_READERS and m.group(1) not in NON_GENERIC and close_paren(e, m.end() - 1) == len(e) - 1:
        return f'{m.group(1)}::<{t}>{e[len(m.group(1)):]}'
    return atom(e)


def conv(e, frm, to):
    """`e` (of type `frm`) as `to`, or None when the conversion is not a plain word conversion."""
    if frm == to:
        return e
    lit = re.fullmatch(r'-?(?:0x[0-9A-Fa-f_]+|\d[\d_]*)', e.strip())
    if is_ptr(to) and not is_ptr(frm) and frm in INTS:
        if lit and int(e.strip().replace('_', ''), 0) == 0:
            return 'GPtr::NULL'
        return f'GPtr::<{to[5:-1]}>::from_addr({atom(e)} as usize)'
    # a pointer expression whose pointee only the old parameter type decided names it (UFCS)
    plain = re.fullmatch(r'[\w.]+', e.strip()) is not None
    ufcs = frm.replace('GPtr<', 'GPtr::<', 1)
    if is_ptr(frm) and to in INTS:
        return f'({atom(e)}.addr() as {to})' if plain else f'({ufcs}::addr({e.strip()}) as {to})'
    if is_ptr(frm) and is_ptr(to):
        return f'{atom(e)}.cast::<{to[5:-1]}>()' if plain else f'{ufcs}::cast::<{to[5:-1]}>({e.strip()})'
    if frm in INTS and to in INTS:
        if lit and not e.strip().startswith('-'):
            return e.strip()
        if lit:
            return f'({e.strip()}{frm} as {to})'
        return f'({typed(e, frm)} as {to})'
    if frm == 'bool' and to in INTS:
        return f'{to}::from({e.strip()})'
    if frm in INTS and to == 'bool':
        return f'({atom(e)} != 0)'
    if frm == 'f32' and re.fullmatch(r'-?\d[\d_]*\.[\d_]*(?:e-?\d+)?', e.strip()):
        e = e.strip() + 'f32'
    if frm == 'f32' and to in ('u32', 'i32'):
        return f'{atom(e)}.to_bits()' if to == 'u32' else f'({atom(e)}.to_bits() as i32)'
    if frm in ('u32', 'i32') and to == 'f32':
        return f'f32::from_bits({e.strip() if frm == "u32" else atom(e) + " as u32"})'
    if frm == 'f32' and to == 'f64':
        return f'f64::from({e.strip()})'
    x87 = ('X87', 'sa_math::x87::X87')
    if frm in x87 and to in ('f64', 'f32'):
        return f'{atom(e)}.to_{to}()'
    if frm == 'f64' and to in x87:
        return f'sa_math::x87::X87::from_f64_exact({e.strip()})'
    if frm == 'f64' and to == 'f32':
        return f'({atom(e)} as f32)'
    return None


# ---- the signature from the declarations and the original ---------------------------------------------
_facts = {}


def facts(addr):
    if addr not in _facts:
        try:
            import abi
            _facts[addr] = abi.facts(addr)
        except Exception as ex:  # noqa: BLE001 -- a fact the bytes do not give is simply unknown
            _facts[addr] = {'conv': None, 'stack': None, 'ret': collections.Counter(), 'widths': [], 'err': str(ex)}
    return _facts[addr]


def pick_int(types, width):
    signed = sum(1 for t in types if t.startswith('i'))
    unsigned = sum(1 for t in types if t.startswith('u'))
    s = 'i' if signed > unsigned else 'u'
    return f'{s}{8 * width}'


CVEC = 'sa_math::vector::CVector'


def words(args):
    """Stack words of an argument list (a `CVector` by value is three)."""
    return sum(3 if t == CVEC else 1 for t in args)


def canonical(addr, sigs, notes, prefer=None):
    """sigs: list of (conv, spec, args, ret) of the mergeable declarations (and the current binding). Returns
    (conv, spec, args, ret) or None when they cannot share one."""
    f = None
    convs = collections.Counter((c, sp) for c, sp, _, _ in sigs)
    if len(convs) > 1:
        f = facts(addr)
        fc = f['conv']
        cands = [k for k in convs if k[0] == fc]
        if not cands:
            notes.append(f'convention: declarations {sorted(convs)}; original {fc}')
            return None
        conv_ = cands[0]
        notes.append(f'convention {sorted(k[0] + k[1] for k in convs)} -> {conv_[0]} (original)')
    else:
        conv_ = next(iter(convs))
    counts = collections.Counter(len(a) for _, _, a, _ in sigs)
    words = collections.Counter(sum(3 if t == 'sa_math::vector::CVector' else 1 for t in a) for _, _, a, _ in sigs)
    if len(counts) > 1 and len(words) == 1:
        # a `CVector` by value is three stack words: the same arguments, another spelling; the binding takes the
        # vector (callers that pass the words build it at the call)
        n = min(counts)
        notes.append(f'CVector by value vs its words: the binding takes {n} arguments (the vectors)')
    elif len(counts) > 1:
        f = f or facts(addr)
        n = f['stack']
        if n is not None and conv_[0] in ('thiscall', 'fastcall'):
            n += 1
        if n is None or n not in counts:
            notes.append(f'argument count: declarations {sorted(counts)}; original {n}')
            return None
        notes.append(f'argument count {sorted(counts)} -> {n} (original)')
    elif len(counts) == 1:
        n = next(iter(counts))
    # only the declarations of the chosen shape decide the types
    pool = [s for s in sigs if (s[0], s[1]) == conv_ and len(s[2]) == n]
    args = []
    widths = None
    for k in range(n):
        ts = [s[2][k] for s in pool]
        if len(set(ts)) == 1 and SIMPLE.match(ts[0]):
            args.append(ts[0])
            continue
        if any(is_ptr(t) for t in ts):
            ps = collections.Counter(t for t in ts if is_ptr(t))
            prim = [t for t in ps if SIMPLE.match(t) and 'CVector' not in t]
            args.append(max(prim, key=lambda t: (ps[t], t == 'GPtr<u8>')) if prim else 'GPtr<u8>')
        elif any(t == 'f32' for t in ts) and conv_[0] in ('cdecl', 'stdcall', 'thiscall') and all(t in INTS + ('f32',) for t in ts):
            args.append('f32')
        elif all(t in INTS + ('bool',) for t in ts):
            if widths is None:
                f = f or facts(addr)
                widths = f['widths']
            stack_k = k - (1 if conv_[0] in ('thiscall', 'fastcall') else 0)
            w = None
            if conv_[0] != 'reg' and 0 <= stack_k < len(widths) and widths[stack_k] and widths[stack_k][1] == 'int':
                w = widths[stack_k][0]
            if w not in (1, 2, 4):
                w = max(WIDTH.get(t, 4) for t in ts)
            args.append(pick_int([t for t in ts if WIDTH.get(t) == w] or ts, w))
        else:
            notes.append(f'argument {k}: {sorted(set(ts))}')
            return None
    rets = [s[3] for s in pool]
    if len(set(rets)) == 1 and (not rets[0] or SIMPLE.match(rets[0])):
        ret = rets[0]
    else:
        vals = [r for r in rets if r]
        f = f or facts(addr)
        use = f['ret']
        if any(is_ptr(r) for r in vals):
            ps = collections.Counter(r for r in vals if is_ptr(r) and SIMPLE.match(r))
            ret = max(ps, key=lambda t: (ps[t], t == 'GPtr<u8>')) if ps else 'GPtr<u8>'
        elif vals and all(r in ('f32', 'f64') + B.X87_TYPES for r in vals):
            # the widest read of ST0: all 80 bits, else a double, else a float
            ret = next(t for t in ('sa_math::x87::X87', 'f64', 'f32') if t in vals)
        elif vals and all(r in INTS + ('bool',) for r in vals):
            ws = {'byte': 1, 'word': 2, 'dword': 4}
            have = {WIDTH.get(r, 4) for r in vals}
            # the widest read among the call sites of the original, of the widths the declarations use
            seen = [ws[u.split('-')[0]] for u in use if u and u.split('-')[0] in ws and ws[u.split('-')[0]] in have]
            w = max(seen) if seen else max(have)
            ret = pick_int([r for r in vals if WIDTH.get(r) == w] or vals, w)
            if w < 4:
                # a narrow result: signed when the original's callers sign-extend it
                sx = sum(n for u, n in use.items() if u and u.endswith('-s'))
                zx = sum(n for u, n in use.items() if u and u.endswith('-u'))
                if sx or zx:
                    ret = ('i' if sx > zx else 'u') + ret[1:]
        elif not vals:
            ret = ''
        else:
            notes.append(f'result: {sorted(set(rets))}')
            return None
        if '' in rets and ret:
            notes.append(f'result: some declarations ignore it ({sorted(set(rets))}) -> {ret}')
    return conv_[0], conv_[1], tuple(args), ret


# ---- call-site rewriting -------------------------------------------------------------------------------
class Callee:
    def __init__(self, name, frm_args, frm_ret, to_args, to_ret):
        self.name, self.frm_args, self.frm_ret, self.to_args, self.to_ret = name, frm_args, frm_ret, to_args, to_ret


def rewrite_calls(text, callees, problems, where, qual=None, ctx=None):
    """`text` with every call `name(..)` of a callee converted (arguments to the binding's types, the result
    back to the caller's). `qual`: absolute path -> Callee for calls written with a module path
    (`super::entity::name(..)`), resolved in `ctx` = (module path, use map)."""
    qual = qual or {}
    names = set(callees) | {k.rsplit('::', 1)[1] for k in qual}
    if not names:
        return text
    s = B.strip_comments(text)
    pat = re.compile(r'(?<![\w.$])((?:\w+::)*)(%s)\s*(\(|::<)' % '|'.join(map(re.escape, sorted(names))))
    local_mods = set(re.findall(r'\bmod\s+(\w+)\s*\{', s))
    out = []
    i = 0
    while True:
        m = pat.search(s, i)
        if not m:
            break
        st = m.start()
        # `fn name(` (a definition); a path through another module or crate names another function
        before = s[:st].rstrip()
        prefix = [x for x in m.group(1).split('::') if x]
        c = None
        if not before.endswith('fn') and m.group(3) != '::<':
            if all(x in ('self', 'super', 'crate') or x in local_mods for x in prefix):
                c = callees.get(m.group(2))
            if c is None and prefix and ctx:
                c = qual.get(absolute('::'.join(prefix + [m.group(2)]), ctx[0], ctx[1]))
        if c is None:
            out.append(text[i:m.end()])
            i = m.end()
            continue
        po = m.end() - 1
        pc = close_paren(s, po)
        if pc < 0:
            break
        inner = text[po + 1:pc]
        args = split_exprs(inner) if inner.strip() else []
        if len(args) != len(c.frm_args) or words(c.frm_args) != words(c.to_args):
            problems.append(f'{where}: {m.group(2)}: {len(args)} arguments at a call, declared {len(c.frm_args)}')
            out.append(text[i:pc + 1])
            i = pc + 1
            continue
        new_args = []
        ok = True
        k = 0
        for tt in c.to_args:
            ft = c.frm_args[k] if k < len(c.frm_args) else None
            if tt == CVEC and ft != CVEC:
                # three words the caller passes -> the vector the binding takes
                parts = []
                for j in range(3):
                    a2 = rewrite_calls(args[k + j], callees, problems, where, qual, ctx)
                    v = conv(a2, c.frm_args[k + j], 'f32')
                    if v is None:
                        ok = False
                        problems.append(f'{where}: {m.group(2)}: no conversion {c.frm_args[k + j]} -> f32')
                        v = a2
                    parts.append(bare(v))
                new_args.append(f'{CVEC}::new({", ".join(parts)})')
                k += 3
                continue
            a2 = rewrite_calls(args[k], callees, problems, where, qual, ctx)
            v = conv(a2, ft, tt)
            if v is not None:
                v = bare(v)
            if v is None:
                ok = False
                problems.append(f'{where}: {m.group(2)}: no conversion {ft} -> {tt}')
                v = a2
            new_args.append(v)
            k += 1
        call = f'{m.group(1)}{m.group(2)}({", ".join(new_args)})' if ok else text[st:pc + 1]
        if ok and len(call) > 100 and '\n' in inner and len(args) == len(new_args):
            call = text[st:po + 1] + rebuild_multiline(inner, args, new_args) + ')'
        after = s[pc + 1:].lstrip()
        if ok and not c.frm_ret and c.to_ret:
            # the caller used the call as a `()` expression: a block's tail or a match arm
            if after.startswith('}'):
                call += ';'
            elif before.endswith('=>') and after.startswith(','):
                call = '{ ' + call + '; }'
        if c.frm_ret != c.to_ret and c.frm_ret:
            stmt = after.startswith(';') and (before.endswith((';', '{', '}')) or before == '')
            if not stmt:
                if not c.to_ret:
                    problems.append(f'{where}: {m.group(2)}: the caller uses a result the binding does not return')
                else:
                    v = conv(call, c.to_ret, c.frm_ret)
                    if v is None:
                        problems.append(f'{where}: {m.group(2)}: no conversion of the result {c.to_ret} -> {c.frm_ret}')
                    else:
                        # an operand of a unary operator, a method call, `?`, an index or a `<` keeps them
                        operand = before.endswith(('!', '-', '*', '&')) and not before.endswith(('&&', '->')) \
                            or re.match(r'(\.|\?|\[|<|as\b)', after) is not None
                        call = v if operand else bare(v)
        out.append(text[i:st])
        out.append(call)
        i = pc + 1
    out.append(text[i:])
    return ''.join(out)


def split_exprs(a):
    """Call arguments split at top-level commas (`<` / `>` are operators here, except in a turbofish `::<..>`;
    `|a, b|` closure heads are skipped)."""
    out, d, ang, cur, i = [], 0, 0, '', 0
    while i < len(a):
        c = a[i]
        if c == '"':
            j = i + 1
            while j < len(a) and a[j] != '"':
                j += 2 if a[j] == '\\' else 1
            cur += a[i:j + 1]
            i = j + 1
            continue
        if a.startswith('::<', i):
            ang += 1
            cur += '::<'
            i += 3
            continue
        if c == '>' and ang:
            ang -= 1
        elif c == '|' and d == 0 and not ang and cur.strip() == '' or (c == '|' and cur.rstrip().endswith(('move', '(', ','))):
            j = a.find('|', i + 1)
            if j > 0:
                cur += a[i:j + 1]
                i = j + 1
                continue
        elif c in '([{':
            d += 1
        elif c in ')]}':
            d -= 1
        if c == ',' and d == 0 and not ang:
            out.append(cur)
            cur = ''
        else:
            cur += c
        i += 1
    if cur.strip():
        out.append(cur)
    return [x.strip() for x in out if x.strip()]


def rebuild_multiline(inner, args, new_args):
    """Keep a multi-line argument list's layout: replace each argument in place."""
    res = inner
    pos = 0
    for a, n in zip(args, new_args):
        k = res.find(a, pos)
        if k < 0:
            continue
        res = res[:k] + n + res[k + len(a):]
        pos = k + len(n)
    return res


# ---- the driver ----------------------------------------------------------------------------------------
def registered():
    """Registered addresses (`#[hook]` / `#[native]`) -> the absolute path of the port function."""
    out = {}
    for p in B.rust_files():
        src = open(p, encoding='utf-8').read()
        if 'addr' not in src:
            continue
        s = B.strip_comments(src)
        rel = os.path.relpath(p, B.ROOT)
        for m in re.finditer(r'#\[(?:\w+::)*(?:hook|native)\(\s*addr\s*=\s*(0x[0-9A-Fa-f_]+)', s):
            f = re.compile(r'\bfn\s+(\w+)').search(s, m.end())
            mp = module_of(rel)
            mods = inline_mods(s, m.start())
            if mods:
                mp += '::' + '::'.join(mods)
            out.setdefault(int(m.group(1).replace('_', ''), 16), mp + '::' + f.group(1) if f else None)
    return out


def binding_importers(rows_by_name):
    """`use <crate>::bindings::<name> [as <local>];` items outside the binding modules: (path, crate, local,
    binding key, span)."""
    out = []
    for p in B.rust_files():
        if p.endswith(os.sep + 'bindings.rs'):
            continue
        src = open(p, encoding='utf-8').read()
        if 'bindings::' not in src:
            continue
        rel = os.path.relpath(p, B.ROOT)
        crate = rel.split(os.sep)[1]
        for m in re.finditer(r'\buse\s+(crate|sa_\w+)::bindings::(\w+)(?:\s+as\s+(\w+))?\s*;', src):
            bc = crate if m.group(1) == 'crate' else m.group(1).replace('_', '-')
            key = (bc, m.group(2))
            if key in rows_by_name:
                vis = src[max(0, m.start() - 12):m.start()]
                out.append((p, crate, m.group(3) or m.group(2), key, m.span(), 'pub ' if re.search(r'pub(\([^)]*\))?\s*$', vis) else ''))
    return out


def resolve(dry, report):
    deps = B.direct_deps()
    trans = B.closure(deps)
    cat = B.catalogue()
    hosts = B.host_names()
    regs = registered()
    galias = global_aliases()
    rows = {int(r['address'], 16): r for r in B.load_rows()}
    texts, decls = {}, []
    for p in B.rust_files():
        if p.endswith(os.sep + 'bindings.rs') or B.generated(p):
            continue
        src, ds = B.scan_file(p)
        texts[p] = src
        s = B.strip_comments(src)
        uses = use_map(s)
        aliases = alias_map(s, uses, galias)
        for d in ds:
            if not d.ok:
                continue
            d.args = [norm(t, aliases) for t in d.args]
            d.ret = norm(d.ret, aliases)
            d.abs_target = resolve_target_abs(d, s, deps)
            if d.conv == 'reg' and d.ret in B.X87_TYPES:
                for conv in ('cdecl', 'stdcall', 'thiscall'):
                    if B.x87_spec({'conv': conv, 'args': ';'.join('a:' + t for t in d.args), 'ret': d.ret}) == d.spec:
                        d.conv, d.spec = conv, ''
                        break
            decls.append(d)
    by_addr = collections.defaultdict(list)
    for d in decls:
        by_addr[d.addr].append(d)
    stats = collections.Counter()
    notes_out = []
    plan = []   # (addr, row, new row, merged decls)
    for addr in sorted(set(by_addr) | set(rows)):
        ds = by_addr.get(addr, [])
        if addr not in rows and len(ds) < 2:
            continue
        if addr in VARARGS:
            for d in ds:
                notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tC varargs ({VARARGS[addr]})')
            stats['kept: C varargs'] += len(ds)
            continue
        good = []
        for d in ds:
            why = None
            if (d.attrs and not re.fullmatch(r'(?:#\s*\[\s*allow\([^\]]*\)\s*\]\s*)+', d.attrs)) or d.block_attrs:
                why = 'attributes'
            elif not all(simple(t) for t in d.args + ([d.ret] if d.ret else [])):
                why = 'types: ' + ', '.join(t for t in d.args + [d.ret] if t and not simple(t))
            elif d.abs_target is None:
                why = 'target not resolvable: ' + d.target
            if why:
                stats['kept: ' + why.split(':')[0]] += 1
                notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\t{why}')
            else:
                good.append(d)
        row = rows.get(addr)
        if not good or (not row and len(good) < 2):
            continue
        sigs = [(d.conv, d.spec, tuple(d.args), d.ret) for d in good]
        if row:
            sigs.append((row['conv'], row['spec'], tuple(sig_of_row(row)[0]), row['ret']))
        notes = []
        sig = canonical(addr, sigs, notes, sig_of_row(row)[0] if row else None)
        if sig is None:
            stats['kept: no common signature'] += len(good)
            for d in good:
                notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tno common signature: {"; ".join(notes)}')
            continue
        members = [d for d in good if (d.conv, d.spec) == sig[:2] and words(d.args) == words(sig[2])]
        for d in good:
            if d not in members:
                stats['kept: other shape'] += 1
                notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tshape {d.conv} {len(d.args)} args vs {sig[0]} {len(sig[2])}')
        if not members:
            continue
        users = {d.crate for d in members}
        if row:
            users |= {row['crate']}
        # the target: a port when one exists
        tg = {d.abs_target for d in members if d.abs_target != 'unported'}
        if row and row['target'] != 'unported':
            tg.add(row['target'])
        if len(tg) > 1:
            # the registered port wins: the other targets are per-crate phase B adapters of the same function
            # (an ST0 result rounded to f64 for the plain forms); the binding reaches the port itself
            if regs.get(addr):
                target = regs[addr]
                for d in members:
                    if d.abs_target not in ('unported', target):
                        stats['adapters replaced by the registered port'] += 1
                        notes_out.append(f'0x{addr:X}\tadapter\t{d.rel}\t{d.name}\t{d.abs_target} -> {target}')
            else:
                stats['kept: several ports'] += len(members)
                for d in members:
                    notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tseveral targets {sorted(tg)}')
                continue
        else:
            target = tg.pop() if tg else 'unported'
        r = cat.get(addr)

        def reach(c, us):
            return all(c == u or c in trans.get(u, ()) for u in us)
        owner = row['crate'] if row else (B.SUBSYS_CRATE.get(r['subsystem']) if r else None)
        if not (owner and reach(owner, users)):
            cands = [c for c in deps if reach(c, users)]
            if not cands:
                stats['kept: no common crate'] += len(members)
                continue
            owner = max(sorted(cands), key=lambda c: len(trans.get(c, ())))
        if target != 'unported':
            tc = target.split('::', 1)[0].replace('_', '-')
            if tc != owner and tc not in trans.get(owner, ()):
                if reach(tc, users):
                    owner = tc
                elif addr in regs:
                    notes.append(f'target {target} not reachable from {owner}: unported (the registry dispatches to the port)')
                    target = 'unported'
                else:
                    stats['kept: port not reachable'] += len(members)
                    for d in members:
                        notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tport {target} not reachable from {owner}')
                    continue
        if target != 'unported':
            # phases B/C call the port directly: the binding has the port's own signature (the declarations
            # that reached it had to match it), and the other callers convert
            via = [(d.conv, d.spec, tuple(d.args), d.ret) for d in members if d.abs_target == target]
            if row and row['target'] == target:
                via.append((row['conv'], row['spec'], tuple(sig_of_row(row)[0]), row['ret']))
            ps = port_signature(target, galias) if target == regs.get(addr) else None
            psig = (sig[0], sig[1], tuple(ps[0]), ps[1]) if ps and len(ps[0]) == len(sig[2]) else (via[0] if via else sig)
            if not all(SIMPLE.match(t) for t in list(psig[2]) + ([psig[3]] if psig[3] else [])):
                stats['kept: port types not nameable in the binding crate'] += len(members)
                for d in members:
                    notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tport {target} types {psig[2]}')
                continue
            if psig != sig:
                if psig[:2] != sig[:2] or len(psig[2]) != len(sig[2]):
                    stats['kept: port shape differs'] += len(members)
                    for d in members:
                        notes_out.append(f'0x{addr:X}\tkept\t{d.rel}\t{d.name}\tport {target} has {psig}, the original {sig}')
                    continue
                notes_out.append(f'0x{addr:X}\tport-sig\t\t\tport {target} types {psig[2]} -> {psig[3]!r}; from the original {sig[2]} -> {sig[3]!r}')
                sig = psig
        name = row['name'] if row else sorted(collections.Counter(d.name for d in members).items(), key=lambda kv: (-kv[1], kv[0]))[0][0]
        argnames = [a.split(':', 1)[0] for a in B.row_args(row['args']) if a] if row else None
        if not argnames or len(argnames) != len(sig[2]):
            src = next((d for d in members if len(d.argnames) == len(sig[2])), None)
            argnames = src.argnames if src else [f'a{i}' for i in range(len(sig[2]))]
        new = {'address': f'0x{addr:X}', 'crate': owner, 'name': name, 'conv': sig[0], 'spec': sig[1],
               'args': ';'.join(f'{n}:{t}' for n, t in zip(argnames, sig[2])), 'ret': sig[3], 'target': target,
               'catalogue_name': row['catalogue_name'] if row else ((r.get('name') or r.get('gr_name') or r.get('ghidra_name') or '') if r else '')}
        # a ported twin stays while the binding itself stays below its port
        new['port'] = (row.get('port') or '') if row and target == 'unported' else ''
        plan.append((addr, row, new, members))
        # a caller that read a wider result than the original returns (AL / AX as EAX) saw undefined high bits
        for d in members + ([] if not row else []):
            if d.ret and sig[3] and d.ret != sig[3] and WIDTH.get(d.ret, 4) > WIDTH.get(sig[3], 4) and not is_ptr(d.ret):
                notes_out.append(f'0x{addr:X}\tmismatch\t{d.rel}\t{d.name}\tresult read as {d.ret}, the original returns {sig[3]} (high bits undefined)')
                stats['mismatch: result wider than the original'] += 1
            if d.ret and sig[3] and d.ret in ('f32', 'f64') and sig[3] in ('f32', 'f64') and d.ret != sig[3]:
                notes_out.append(f'0x{addr:X}\tfloat\t{d.rel}\t{d.name}\tST0 read as {d.ret}, binding {sig[3]}')
        for n in notes:
            notes_out.append(f'0x{addr:X}\tsignature\t\t{name}\t{n}')
        stats['bindings'] += 1
        stats['declarations merged'] += len(members)
        if row and (row['conv'], row['spec'], tuple(sig_of_row(row)[0]), row['ret']) != sig:
            stats['bindings whose signature changed'] += 1
        if row and row['target'] != target:
            stats['bindings whose target changed'] += 1
    # unique names per crate (rows that move or are new)
    taken = collections.defaultdict(set)
    for addr, row, new, _ in plan:
        if row and row['crate'] == new['crate']:
            taken[new['crate']].add(new['name'])
    for a, r in rows.items():
        taken[r['crate']].add(r['name'])
    for addr, row, new, _ in plan:
        if row and row['crate'] == new['crate']:
            continue
        if new['name'] in taken[new['crate']]:
            new['name'] = f"{new['name']}_{addr:x}"
        taken[new['crate']].add(new['name'])
    if report:
        open(report, 'w').write('address\tkind\tfile\tname\tdetail\n' + '\n'.join(notes_out) + '\n')
    if dry:
        return stats
    apply(plan, rows, texts, deps)
    return stats


def sig_of_row(r):
    return [a.split(':', 1)[1] for a in B.row_args(r['args']) if a], r['ret']


def apply(plan, rows, texts, deps):
    problems = []
    edits = collections.defaultdict(list)       # path -> [(start, end, block, text)]
    callees = collections.defaultdict(dict)      # path -> {local: Callee}
    pub_callees = {}                             # (crate, module path::name) -> Callee
    need_dep = collections.defaultdict(set)
    new_rows = {a: n for a, _, n, _ in plan}
    rows_by_name = {(r['crate'], r['name']): a for a, r in rows.items()}
    for addr, row, new, members in plan:
        to_args, to_ret = sig_of_row(new)
        for d in members:
            if new['crate'] != d.crate and new['crate'] not in deps[d.crate]:
                need_dep[d.crate].add(new['crate'])
            path = ('crate' if new['crate'] == d.crate else B.crate_mod(new['crate'])) + '::bindings::' + new['name']
            vis = (d.vis + ' ') if d.vis else ''
            use = f'{vis}use {path}{"" if new["name"] == d.name else " as " + d.name};'
            edits[d.path].append((d.span[0], d.span[1], d.block, use))
            if list(d.args) != to_args or d.ret != to_ret:
                c = Callee(d.name, list(d.args), d.ret, to_args, to_ret)
                callees[d.path][d.name] = c
                if d.vis:
                    pub_callees[(d.crate, module_of(d.rel) + '::' + d.name)] = c
    # importers of existing bindings whose signature or place changed
    for p, crate, local, key, span, vis in binding_importers(rows_by_name):
        addr = rows_by_name[key]
        if addr not in new_rows:
            continue
        old, new = rows[addr], new_rows[addr]
        fa, fr = sig_of_row(old)
        ta, tr = sig_of_row(new)
        if (old['crate'], old['name']) != (new['crate'], new['name']):
            path = ('crate' if new['crate'] == crate else B.crate_mod(new['crate'])) + '::bindings::' + new['name']
            edits[p].append((span[0], span[1], None, f'use {path}{"" if new["name"] == local else " as " + local};'))
            if new['crate'] != crate and new['crate'] not in deps[crate]:
                need_dep[crate].add(new['crate'])
        if fa != ta or fr != tr:
            c = Callee(local, fa, fr, ta, tr)
            callees[p][local] = c
            if vis:
                pub_callees[(crate, module_of(os.path.relpath(p, B.ROOT)) + '::' + local)] = c
    # importers of `pub` declarations / re-exports that changed signature
    if pub_callees:
        for p in B.rust_files():
            if p not in texts:
                continue
            rel = os.path.relpath(p, B.ROOT)
            crate = rel.split(os.sep)[1]
            s = B.strip_comments(texts[p])
            for local, path in use_map(s).items():
                a = absolute(path, module_of(rel), {})
                if a and (crate, a) in pub_callees and local not in callees[p]:
                    c = pub_callees[(crate, a)]
                    callees[p][local] = Callee(local, c.frm_args, c.frm_ret, c.to_args, c.to_ret)
    if pub_callees:
        tails = {k[1].rsplit('::', 1)[1] for k in pub_callees}
        for p in texts:
            if p not in callees and any(t + '(' in texts[p] for t in tails):
                callees[p] = {}
    for p in sorted(set(edits) | set(callees)):
        src = texts.get(p) or open(p, encoding='utf-8').read()
        src2 = apply_decl_edits(src, edits.get(p, []))
        if p in callees:
            rel = os.path.relpath(p, B.ROOT)
            crate = rel.split(os.sep)[1]
            q = {k[1]: c for k, c in pub_callees.items() if k[0] == crate}
            NON_GENERIC.clear()
            NON_GENERIC.update(re.findall(r'\bfn\s+(\w+)\s*\(', src2))
            src2 = rewrite_calls(src2, callees[p], problems, rel, q, (module_of(rel), use_map(B.strip_comments(src2))))
            if 'GPtr::' in src2 and not has_gptr(src2):
                src2 = add_use(src2, 'use sa_core::mem::GPtr;')
        if src2 != src:
            open(p, 'w', encoding='utf-8').write(src2)
    for crate, add in need_dep.items():
        for a in sorted(add):
            add_dependency(crate, a)
    keys = {}
    for addr, row, new, members in plan:
        for d in members:
            mp = module_of(d.rel)
            mods = inline_mods(B.strip_comments(texts[d.path]), d.span[0])
            if mods:
                mp += '::' + '::'.join(mods)
            keys[mp + '::' + d.name] = B.crate_mod(new['crate']) + '::bindings::' + new['name']
    for p in B.rust_files():
        t = open(p, encoding='utf-8').read()
        t2 = re.sub(r'"(sa_[a-z_0-9]+(?:::[A-Za-z0-9_]+)+)"', lambda m: '"%s"' % keys.get(m.group(1), m.group(1)), t)
        if t2 != t:
            open(p, 'w', encoding='utf-8').write(t2)
    out = dict(rows)
    out.update(new_rows)
    rs = sorted(out.values(), key=lambda r: (r['crate'], int(r['address'], 16)))
    import csv
    with open(B.CSV, 'w', newline='') as fh:
        w = csv.DictWriter(fh, B.FIELDS, lineterminator='\n')
        w.writeheader()
        w.writerows(rs)
    for pr in problems:
        print('problem:', pr)


def has_gptr(src):
    for u in re.findall(r'\buse\s+[^;]*;', src):
        if re.search(r'\bGPtr\b', u) or re.search(r'mem::\*', u):
            return True
    return re.search(r'\b(?:struct|type)\s+GPtr\b', src) is not None


def add_use(src, use):
    """Insert a `use` before the module's first `use` item (else after its inner attributes and docs)."""
    m = re.search(r'^use [^;]+;\n', src, re.M)
    if m:
        return src[:m.start()] + use + '\n' + src[m.start():]
    m = re.search(r'^(?!//!|#!\[|\s*$)', src, re.M)
    i = m.start() if m else 0
    return src[:i] + use + '\n' + src[i:]


def apply_decl_edits(src, eds):
    """Remove the merged entries from their `orig!` blocks (a block left empty goes entirely) and put the
    `use` items before the block; replace the `use` items of moved bindings."""
    if not eds:
        return src
    s = B.strip_comments(src)
    blocks = collections.defaultdict(list)
    plain = []
    for a, b, blk, use in eds:
        if blk is None:
            plain.append((a, b, use))
        else:
            blocks[blk].append((a, b, use))
    ops = []
    for (bs, be), items in blocks.items():
        rest = s[bs:be]
        for a, b, _ in sorted(items):
            rest = rest[:a - bs] + ' ' * (b - a) + rest[b - bs:]
        body = rest[rest.index('{') + 1:rest.rindex('}')]
        ls = src.rfind('\n', 0, bs) + 1
        indent = re.match(r'[ \t]*', src[ls:bs]).group(0)
        ins = ''.join(f'{indent}{u}\n' for u in sorted({u for _, _, u in items}))
        if not body.strip():
            end = be
            if src[end:end + 1] == ';':
                end += 1
            if src[end:end + 1] == '\n':
                end += 1
            ops.append((ls, end, ins))
        else:
            for a, b, _ in items:
                # the span runs from after the previous entry's `;` (or the block's `{`): doc comments included
                ops.append((a, b, ''))
            ops.append((ls, ls, ins))
    for a, b, use in plain:
        ops.append((a, b, use))
    for a, b, new in sorted(ops, key=lambda o: (o[0], o[1]), reverse=True):
        src = src[:a] + new + src[b:]
    src = re.sub(r'\{\s*\n(\s*\n)+', '{\n', src)
    src = re.sub(r'\n[ \t]*\n([ \t]*\n)+', '\n\n', src)
    return src


# ---- bindings back onto their ports --------------------------------------------------------------------
def port_signature(path, galias):
    """(args, ret) of the Rust function at absolute `path` (`sa_x::a::b::f`), normalised, or None."""
    parts = path.split('::')
    crate = parts[0].replace('_', '-')
    mods, name = parts[1:-1], parts[-1]
    cands = [os.path.join(B.CRATES, crate, 'src', *mods) + '.rs',
             os.path.join(B.CRATES, crate, 'src', *mods, 'mod.rs')]
    if not mods:
        cands = [os.path.join(B.CRATES, crate, 'src', 'lib.rs')]
    for cand in cands:
        if not os.path.exists(cand):
            continue
        src = open(cand, encoding='utf-8').read()
        s = B.strip_comments(src)
        m = re.search(r'\bfn\s+%s\s*\(' % re.escape(name), s)
        if not m:
            continue
        e = close_paren(s, m.end() - 1)
        al = alias_map(s, use_map(s), galias)
        args = [norm(a.split(':', 1)[1], al) for a in split_exprs(s[m.end():e]) if ':' in a]
        r = re.match(r'\s*->\s*([^{]+?)\s*(?:where\b[^{]*)?\{', s[e + 1:])
        ret = norm(r.group(1), al) if r else ''
        return [absolute_struct(t, crate) for t in args], absolute_struct(ret, crate)
    return None


_structs = {}


def absolute_struct(t, crate):
    """`GPtr<Name>` with `Name` a struct of `crate` -> `GPtr<sa_crate::module::Name>` (a binding names it so)."""
    m = re.fullmatch(r'GPtr<([A-Z]\w*)>', t)
    if not m:
        return t
    if crate not in _structs:
        found = {}
        for d, _, fs in os.walk(os.path.join(B.CRATES, crate, 'src')):
            for f in sorted(fs):
                if f.endswith('.rs'):
                    p = os.path.join(d, f)
                    for n in re.findall(r'^pub struct (\w+)', open(p, encoding='utf-8').read(), re.M):
                        found.setdefault(n, []).append(module_of(os.path.relpath(p, B.ROOT)))
        _structs[crate] = {n: ms[0] for n, ms in found.items() if len(ms) == 1}
    mp = _structs[crate].get(m.group(1))
    return f'GPtr<{mp}::{m.group(1)}>' if mp else t


def repoint(addr_file, dry):
    """For the addresses listed in `addr_file` (first column): when the address has a registered port and every
    crate that imports the binding may depend on the port's crate (tools/lib/layers.py), the binding moves to
    that crate and targets the port, with the port's signature (phases B/C and host tests call it directly, as
    the merged declarations did); importers convert at their calls."""
    import layers
    deps = B.direct_deps()
    galias = global_aliases()
    regs = registered()
    rows = {int(r['address'], 16): r for r in B.load_rows()}
    rows_by_name = {(r['crate'], r['name']): a for a, r in rows.items()}
    users = collections.defaultdict(set)
    for p, crate, local, key, span, vis in binding_importers(rows_by_name):
        users[rows_by_name[key]].add(crate)
    want = [int(line.split('\t')[0], 16) for line in open(addr_file) if line.strip()]
    plan, stats = [], collections.Counter()
    taken = collections.defaultdict(set)
    for r in rows.values():
        taken[r['crate']].add(r['name'])
    for a in want:
        row = rows.get(a)
        port = regs.get(a)
        if not row or not port or row['target'] != 'unported':
            stats['skipped: no registered port'] += 1
            continue
        pc = port.split('::')[0].replace('_', '-')
        if not all(u == pc or layers.may_depend(u, pc) for u in users[a]):
            stats['kept on the registry: a caller below the port'] += 1
            continue
        sig = port_signature(port, galias)
        if sig is None or not all(SIMPLE.match(t) for t in sig[0] + ([sig[1]] if sig[1] else [])):
            stats['kept on the registry: port signature not nameable'] += 1
            continue
        old_args, old_ret = sig_of_row(row)
        if len(sig[0]) != len(old_args) or (old_ret and not sig[1]):
            stats['kept on the registry: port arity differs'] += 1
            continue
        new = dict(row)
        new['crate'] = pc
        new['target'] = port
        names = [n.split(':', 1)[0] for n in B.row_args(row['args'])]
        new['args'] = ';'.join(f'{n}:{t}' for n, t in zip(names, sig[0]))
        new['ret'] = sig[1]
        if pc != row['crate']:
            if new['name'] in taken[pc]:
                new['name'] = f"{new['name']}_{a:x}"
            taken[pc].add(new['name'])
        plan.append((a, row, new, []))
        stats['bindings moved onto their port'] += 1
        if (list(sig[0]), sig[1]) != (old_args, old_ret):
            stats['  of which with another signature'] += 1
    if not dry:
        texts = {p: open(p, encoding='utf-8').read() for p in B.rust_files()}
        apply(plan, rows, texts, deps)
    return stats


def twin(addr_file, dry):
    """For the addresses listed in `addr_file` whose binding stays below its registered port (a caller in a
    lower layer reaches the port through the phase B registry): the binding takes the port's signature and the
    CSV names the port (`port` column); the generator then also writes the binding into the port's crate with
    the port as target, and the callers that may depend on that crate import it from there (phases B/C and host
    tests call the port directly, as their declarations did before the migration)."""
    import layers
    deps = B.direct_deps()
    galias = global_aliases()
    regs = registered()
    rows = {int(r['address'], 16): r for r in B.load_rows()}
    want = [int(line.split('\t')[0], 16) for line in open(addr_file) if line.strip()]
    plan, stats, twins = [], collections.Counter(), {}
    for a in want:
        row = rows.get(a)
        port = regs.get(a)
        if not row or not port or row['target'] != 'unported' or row.get('port'):
            continue
        pc = port.split('::')[0].replace('_', '-')
        if pc == row['crate'] or not layers.may_depend(pc, row['crate']):
            continue
        sig = port_signature(port, galias)
        reach = B.closure(deps).get(row["crate"], set()) | {row["crate"]}
        if sig is None or not all(SIMPLE.match(t) and all(c.replace("_", "-") in reach for c in re.findall(r"\b(sa_[a-z_0-9]+)::", t)) for t in sig[0] + ([sig[1]] if sig[1] else [])):
            stats['no twin: port signature not nameable'] += 1
            continue
        old_args, old_ret = sig_of_row(row)
        if len(sig[0]) != len(old_args) or (old_ret and not sig[1]):
            stats['no twin: port arity differs'] += 1
            continue
        twins[a] = port
        if (list(sig[0]), sig[1]) != (old_args, old_ret):
            new = dict(row)
            names = [n.split(':', 1)[0] for n in B.row_args(row['args'])]
            new['args'] = ';'.join(f'{n}:{t}' for n, t in zip(names, sig[0]))
            new['ret'] = sig[1]
            plan.append((a, row, new, []))
            stats['  of which with the port signature now'] += 1
        stats['bindings with a ported twin'] += 1
    if dry:
        return stats
    texts = {p: open(p, encoding='utf-8').read() for p in B.rust_files()}
    if plan:
        apply(plan, rows, texts, deps)
    rows = {int(r['address'], 16): r for r in B.load_rows()}
    for a, port in twins.items():
        rows[a]['port'] = port
    write_rows(rows.values())
    # the importers that may depend on the port's crate take the twin
    rows_by_name = {(r['crate'], r['name']): a for a, r in rows.items()}
    tw = B.twin_rows(list(rows.values()))
    tname = {int(t['address'], 16): t for t in tw}
    edits = collections.defaultdict(list)
    need = collections.defaultdict(set)
    for p, crate, local, key, span, vis in binding_importers(rows_by_name):
        a = rows_by_name[key]
        t = tname.get(a)
        if not t:
            continue
        pc = t['crate']
        if crate != pc and not layers.may_depend(crate, pc):
            continue
        path = ('crate' if pc == crate else B.crate_mod(pc)) + '::bindings::' + t['name']
        edits[p].append((span, f'use {path}{"" if t["name"] == local else " as " + local};'))
        if crate != pc and pc not in deps[crate]:
            need[crate].add(pc)
        stats['callers moved onto a twin'] += 1
    for p, es in edits.items():
        src = open(p, encoding='utf-8').read()
        for (a, b), new in sorted(es, reverse=True):
            src = src[:a] + new + src[b:]
        open(p, 'w', encoding='utf-8').write(src)
    for crate, add in need.items():
        for a in sorted(add):
            add_dependency(crate, a)
    return stats


def write_rows(rows):
    import csv
    rs = sorted(rows, key=lambda r: (r['crate'], int(r['address'], 16)))
    with open(B.CSV, 'w', newline='') as fh:
        w = csv.DictWriter(fh, B.FIELDS, lineterminator='\n')
        w.writeheader()
        for r in rs:
            w.writerow({k: r.get(k) or '' for k in B.FIELDS})


def add_dependency(crate, dep):
    """`dep` into `crate`'s `[dependencies]` (a `[dev-dependencies]` entry does not count)."""
    p = os.path.join(B.CRATES, crate, 'Cargo.toml')
    t = open(p).read()
    m = re.search(r'^\[dependencies\]\n(.*?)(?=^\[|\Z)', t, re.M | re.S)
    if m and re.search(r'^%s\s*=' % re.escape(dep), m.group(1), re.M):
        return
    t = re.sub(r'(\[dependencies\]\n)', r'\1%s = { path = "../%s" }\n' % (dep, dep), t, count=1)
    open(p, 'w').write(t)
