"""Normalised form of a free function (tools/verify/dupes.py's text after the name), for matching copies
of a shared helper whatever their parameter, generic and local names, generic bounds, path prefixes,
integer spellings and turbofish annotations (the types are fixed by the signature)."""
import re

TOK = re.compile(r'::|->|=>|[A-Za-z_][A-Za-z0-9_]*|0[xX][0-9A-Fa-f_]+(?:[iu](?:8|16|32|64|size))?|[0-9][0-9_]*(?:\.[0-9_]+)?(?:[iuf](?:8|16|32|64|size))?|\S')
PREFIXES = [['sa_core', '::', 'mem', '::'], ['crate', '::', 'mem', '::'], ['sa_math', '::', 'x87', '::'],
            ['sa_core', '::', 'field', '::'], ['crate', '::', 'field', '::'], ['core', '::', 'mem', '::'],
            ['sa_core', '::', 'pools', '::'], ['crate', '::', 'pools', '::'], ['std', '::', 'ptr', '::'],
            ['core', '::', 'ptr', '::']]


def close_of(t, i, o, c):
    d = 0
    for j in range(i, len(t)):
        if t[j] == o:
            d += 1
        elif t[j] == c:
            d -= 1
            if d == 0:
                return j
    return len(t) - 1


def normalise(text):
    t = TOK.findall(text)
    # path prefixes
    out, i = [], 0
    while i < len(t):
        for p in PREFIXES:
            if t[i:i + len(p)] == p:
                i += len(p)
                break
        else:
            out.append(t[i])
            i += 1
    t = out
    ren = {}
    i = 0
    # generics: `< A : bound , B >` directly at the start
    if t and t[0] == '<':
        j = close_of(t, 0, '<', '>')
        gen, k, n = [], 1, 0
        while k < j:
            if re.match(r'[A-Za-z_]', t[k]) and t[k - 1] in ('<', ','):
                ren[t[k]] = f'T{n}'
                n += 1
                gen += [f'T{n - 1}', ',']
                # skip bound
                k += 1
                d = 0
                while k < j and not (t[k] == ',' and d == 0):
                    if t[k] == '<':
                        d += 1
                    elif t[k] == '>':
                        d -= 1
                    k += 1
                k += 1
            else:
                k += 1
        t = ['<'] + gen[:-1] + ['>'] + t[j + 1:]
    # parameters
    if '(' in t:
        a = t.index('(')
        b = close_of(t, a, '(', ')')
        n = 0
        d = 0
        for k in range(a + 1, b):
            if t[k] in '(<[':
                d += 1
            elif t[k] in ')>]':
                d -= 1
            if d == 0 and re.match(r'[a-z_][A-Za-z0-9_]*$', t[k]) and t[k + 1] == ':' and t[k - 1] in ('(', ','):
                ren[t[k]] = f'P{n}'
                n += 1
            elif d == 0 and t[k] == 'mut' and t[k - 1] in ('(', ','):
                pass
    # locals
    n = 0
    for k in range(len(t) - 1):
        if t[k] == 'let':
            v = t[k + 2] if t[k + 1] == 'mut' else t[k + 1]
            if re.match(r'[a-z_][A-Za-z0-9_]*$', v) and v not in ren:
                ren[v] = f'L{n}'
                n += 1
    t = [ren.get(x, x) if not (k > 0 and t[k - 1] in ('.', '::')) else x for k, x in enumerate(t)]
    # integers
    def num(x):
        m = re.match(r'^(0[xX][0-9A-Fa-f_]+|[0-9][0-9_]*)$', x)
        if m:
            return str(int(x.replace('_', ''), 0))
        return x
    t = [num(x) for x in t]
    # turbofish after cast / GPtr / from_addr is implied by the signature
    out, k = [], 0
    while k < len(t):
        if t[k] == '::' and k + 1 < len(t) and t[k + 1] == '<' and out and out[-1] in ('cast', 'GPtr'):
            k = close_of(t, k + 1, '<', '>') + 1
            continue
        out.append(t[k])
        k += 1
    s = ' '.join(out)
    s = s.replace('; }', ' }')
    return s
