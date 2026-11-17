#!/usr/bin/env python3
"""Every `orig!` declaration of a registered address passes its arguments where the registered port reads them.

Usage: python3 -I tools/verify/convcheck.py [--all]

Phase B on wasm32 calls a registry entry through one type-erased signature (`sa_core::wabi`): the caller's
declaration writes its arguments as words in declaration order (`this` / register arguments first, as their
Rust parameters), and the port's entry reads its own parameters in order from the same words. The original
calling convention is not looked at there, so a declaration whose convention differs from the port's
registration shifts the words: `fn f(this, a) = thiscall 0x..` against a port registered `stdcall` (`fn
f(a)`) hands the port ECX as `a` (the MoveLimb trap of FRONTIER-2 turn 10, a web-only bug: on i686 both are
the same stack). On i686 a cdecl declaration of a callee-popped port (or the reverse) unbalances the stack.

The check models every argument as the original locations of its words (`ecx`, `stack+0`, ...: the
convention's register prefix, `reg("args = ...")` locations, stack slots rounded up to words) and compares
the declaration's word list with the port's (`#[hook]` / `#[native]`) word list. FAIL when

  * shift: some word the port reads comes from another original location than the one the declaration
    passes there (or the declaration passes fewer words than the port reads), or
  * cleanup: the two disagree on who pops the stack arguments (only when there are stack arguments).

A declaration that passes extra trailing words the port never reads is fine (both builds ignore them).
`--all` also lists the declarations whose convention name differs but whose words line up (informational).
Exit 1 on any FAIL. Part of the writer gate (`conv` step).
"""
import os
import re
import sys

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'gen'))
import bindings  # noqa: E402  (orig! parser shared with the bindings generator)

SIZES = {
    'u8': 1, 'i8': 1, 'bool': 1, 'GBool': 1, 'u16': 2, 'i16': 2, 'u32': 4, 'i32': 4, 'f32': 4, 'usize': 4,
    'isize': 4, 'u64': 8, 'i64': 8, 'f64': 8, 'X87': 16, 'CVector': 12, 'RwV3d': 12, 'CVector2D': 8,
    'RwV2d': 8, 'CRGBA': 4, 'RwRGBA': 4,
}
REG8 = {'al': 'eax', 'cl': 'ecx', 'dl': 'edx', 'bl': 'ebx', 'ax': 'eax', 'cx': 'ecx', 'dx': 'edx',
        'bx': 'ebx', 'si': 'esi', 'di': 'edi'}
ATTR = re.compile(r'^[ \t]*#\[(?:[\w:]+::)?(hook|native)\(', re.M)
CONV = re.compile(r'\bconv\s*=\s*(cdecl|stdcall|thiscall|fastcall|reg\(\s*"([^"]*)"\s*\))')
ADDR = re.compile(r'\baddr\s*=\s*(0x[0-9A-Fa-f_]+)')
FNSIG = re.compile(r'\bfn\s+(\w+)\s*(?:<[^>]*>)?\s*\(')


def size(t):
    """Bytes of one argument type (pointers, references and unknown structs: one word)."""
    t = re.sub(r'\s+', '', t)
    t = re.sub(r'^(?:[\w]+::)+', '', t)
    m = re.match(r'^\[(.+);(\d+)\]$', t)
    if m:
        return size(m.group(1)) * int(m.group(2))
    return SIZES.get(t, 4)


def reg_locs(spec):
    """The `args = ...` locations of a reg() spec (None: every argument on the stack); `pop`."""
    locs, pop = None, False
    for clause in spec.split(';'):
        clause = clause.strip()
        if clause == 'pop':
            pop = True
        elif clause.startswith('args'):
            locs = [x for x in re.split(r'[\s,]+', clause.split('=', 1)[1].strip()) if x]
    return locs, pop


def words(conv, spec, types, names=None):
    """(word locations, callee pops, stack bytes) of a call with argument TYPES under CONV; a word of a
    parameter in NAMES that starts with `_` (a port parameter it never reads) is None."""
    if conv == 'reg':
        locs, pop = reg_locs(spec)
    else:
        locs = {'thiscall': ['ecx'], 'fastcall': ['ecx', 'edx']}.get(conv, [])
        pop = conv in ('stdcall', 'thiscall', 'fastcall')
    out, sp = [], 0
    for k, t in enumerate(types):
        loc = locs[k] if locs is not None and k < len(locs) else 'stack'
        n = (size(t) + 3) // 4
        unused = names is not None and names[k].startswith('_')
        if loc == 'stack':
            for _ in range(n):
                out.append(None if unused else 'stack+%d' % sp)
                sp += 4
        else:
            out.append(None if unused else REG8.get(loc, loc))
            out.extend([None if unused else '%s.hi%d' % (loc, j) for j in range(1, n)])
    return out, pop and sp > 0, sp


def split_params(s):
    """(names, types) of a Rust `fn` signature's parameter list."""
    names, types = [], []
    for a in bindings.split_args(s):
        if ':' in a:
            n, t = a.split(':', 1)
            names.append(re.sub(r'^mut\s+', '', n.strip()))
            types.append(bindings.norm_type(t))
    return names, types


def registrations():
    """addr -> (conv, spec, param types, file:line, fn) of every #[hook] / #[native] in crates/."""
    regs = {}
    for path in bindings.rust_files():
        rel = os.path.relpath(path, ROOT)
        if rel.split(os.sep)[1] in ('sa-macros', 'sa-hook'):
            continue
        src = open(path, encoding='utf-8').read()
        if '#[hook(' not in src and 'native(' not in src:
            continue
        s = bindings.strip_comments(src)
        for m in ATTR.finditer(s):
            # the attribute's body up to its closing parenthesis
            d, j = 1, m.end()
            while j < len(s) and d:
                c = s[j]
                if c == '"':
                    j += 1
                    while j < len(s) and s[j] != '"':
                        j += 2 if s[j] == '\\' else 1
                elif c == '(':
                    d += 1
                elif c == ')':
                    d -= 1
                j += 1
            body = s[m.end():j - 1]
            a, c = ADDR.search(body), CONV.search(body)
            if not a or not c:
                continue
            f = FNSIG.search(s, j)
            if not f:
                continue
            k = f.end()
            d = 1
            while k < len(s) and d:
                d += {'(': 1, ')': -1}.get(s[k], 0)
                k += 1
            conv = 'reg' if c.group(1).startswith('reg') else c.group(1)
            addr = int(a.group(1).replace('_', ''), 16)
            names, types = split_params(s[f.end():k - 1])
            regs[addr] = (conv, c.group(2) or '', names, types,
                          '%s:%d' % (rel, s.count('\n', 0, m.start()) + 1), f.group(1))
    return regs


def main():
    show_all = '--all' in sys.argv[1:]
    regs = registrations()
    fails, same_words, n = [], [], 0
    for path in bindings.rust_files():
        _, decls = bindings.scan_file(path)
        for d in decls:
            # a declaration with a Rust target calls it directly in phases B/C (the compiler checks it)
            if not d.ok or d.addr not in regs or d.target != 'unported':
                continue
            n += 1
            conv, spec, pnames, ptypes, where, fname = regs[d.addr]
            dw, dpop, dsp = words(d.conv, d.spec.strip('"'), d.args)
            pw, ppop, psp = words(conv, spec, ptypes, pnames)
            loc = '%s:%d' % (d.rel, open(path, encoding='utf-8').read().count('\n', 0, d.span[0]) + 1)
            what = '0x%X\t%s\t%s %s (%d words)\tport %s %s (%d words) %s' % (
                d.addr, loc, d.conv, d.spec, len(dw), conv, spec, len(pw), where)
            if len(dw) < len(pw) and any(pw[len(dw):]) or any(w is not None and w != v for w, v in zip(pw, dw)):
                fails.append('FAIL shift\t' + what)
            elif dsp and psp and dpop != ppop:
                fails.append('FAIL cleanup\t' + what)
            elif (d.conv, d.spec.strip('"')) != (conv, spec):
                same_words.append('same-words\t' + what)
    for line in fails:
        print(line)
    if show_all:
        for line in same_words:
            print(line)
    print('convcheck: %d declarations of %d registered addresses, %d FAIL, %d with another convention name '
          'but the same words' % (n, len(regs), len(fails), len(same_words)))
    return 1 if fails else 0


if __name__ == '__main__':
    sys.exit(main())
