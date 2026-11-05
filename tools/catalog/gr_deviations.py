#!/usr/bin/env python3
"""CAT-14: gta-reversed fidelity audit.

Lists every place where gta-reversed (re/gta-reversed, pinned 20d8d2a) knowingly or intentionally
differs from the original gta_sa.exe 1.0 US, and maps each one to a catalogue function.

Sources scanned: source/game_sa and source/app (*.cpp, *.h, *.hpp). source/toolsmenu (debug UI),
source/extensions (helpers, attributed at their use sites), source/reversiblehooks and libs have
no original counterpart and are not scanned.

Markers (`kind`, one row per source line, first matching kind wins):
  rbf                 `notsa::bugfixes::X` runtime switch (source/reversiblebugfixes, enabled by
                      default in gta-reversed)
  fix-bugs-ifdef      `#ifdef / #if / #ifndef FIX_BUGS` block (FIX_BUGS is defined in
                      source/CMakeLists.txt, so the fixed branch is the one gta-reversed runs)
  fix-bugs-runtime    `notsa::IsFixBugs()` (constexpr true because FIX_BUGS is defined)
  fix-bugs-comment    `FIX_BUGS` named in a comment (fix applied without the macro)
  define-branch       `#if/#ifdef` on another gta-reversed build define (NOTSA_*, USE_D3D9,
                      SA_SKINNED_PEDS, EXTRA_DEBUG_FEATURES, USE_GTASA_ALLOCATOR,
                      APPLY_CD_STREAM_DEADLOCK_FIX)
  notsa-unreachable   `NOTSA_UNREACHABLE` / `NOTSA_UNREACHABLE_CASE` / `NOTSA_UNUSED_FUNCTION`
                      (debug: exit(3); release: __assume(false); the original does something else
                      on that path)
  bugfix-comment      comment saying a bug was fixed / a check was added (BUGFIX, "bug fix",
                      "fixed", "added check", "null check", "prevent crash", ...)
  original-comment    comment describing the original code (`Original:`, `originally`, `R*`,
                      `vanilla`, `in SA`, `SA:`)
  bug-note            comment noting a (possible) original bug without saying it was changed
  notsa               `NOTSA` in a comment or name (not covered above)
  todo                TODO/FIXME that mentions original / inline / vanilla / different / wrong
  assert              `assert(` (debug-only check the original does not have)
  notsa-helper        a `notsa::` helper used in place of original logic
  inlined             comment saying the original code was inlined here
Plus one row per script opcode whose gta-reversed status disagrees with the PC exe dispatch
(catalog/script-commands.csv):
  script-default      gta-reversed has a handler, the exe dispatches the opcode to the switch default
  script-unimpl-case  gta-reversed registers it as unimplemented, the exe has a case for it

Mapping (`mapping`): hook (definition matched to an RH_Scoped*Install registration), hook-overload
(several overloads, matched by parameter list), hook-ambiguous (overloads could not be told apart:
all candidates listed, first one used), addr-comment (no hook; mapped through a `// 0xADDR` comment on
or just above the definition, to the catalogue function containing that address), script-handler
(command handler function -> its opcode's ProcessCommandsXToY), script-opcode, gr-only (a gta-reversed
function with no original counterpart), none (not mapped).

Classification: keep-original (the port must reproduce the original, gta-reversed differs),
harmless (no observable difference), unclear (needs a measurement against the original).
`deviates`: yes (gta-reversed's behaviour differs), maybe, no (a note only, e.g. gta-reversed keeps
the original bug, or a debug-only check). Hand-checked rows come from catalog/gr-deviations-notes.csv
(key file + line + a substring of the line) and override the rule-based values.

Outputs: catalog/gr-deviations.csv, catalog/gr-deviations-summary.json (counts, used by the docs).
Usage: gr_deviations.py <gta-reversed> <catalog-dir> <gta_sa.exe>
"""
import bisect
import csv
import json
import re
import sys
from collections import Counter, defaultdict
from pathlib import Path

import capstone
import pefile

SCAN_DIRS = ('source/game_sa', 'source/app')
EXTS = ('.cpp', '.h', '.hpp')
BUILD_DEFINES = ('NOTSA_WINDOWED_MODE', 'USE_D3D9', 'SA_SKINNED_PEDS', 'EXTRA_DEBUG_FEATURES',
                 'USE_GTASA_ALLOCATOR', 'APPLY_CD_STREAM_DEADLOCK_FIX', 'NOTSA_USE_SDL3',
                 'NOTSA_DEBUG', 'NOTSA_UNITY_BUILD')

# --------------------------------------------------------------------------------------------
# Lexing: blank comments and string literals (keeping line structure), collect comment text.
# --------------------------------------------------------------------------------------------


def lex(text):
    """Return (code_lines, comment_by_line) with comments/strings replaced by spaces."""
    out = []
    comments = defaultdict(list)
    i, n, line = 0, len(text), 1
    while i < n:
        c = text[i]
        if c == '\n':
            out.append(c)
            line += 1
            i += 1
        elif text.startswith('//', i):
            j = text.find('\n', i)
            j = n if j < 0 else j
            # line continuation inside a // comment is rare; ignore
            comments[line].append(text[i + 2:j].strip())
            out.append(' ' * (j - i))
            i = j
        elif text.startswith('/*', i):
            j = text.find('*/', i + 2)
            j = n if j < 0 else j + 2
            chunk = text[i:j]
            for k, part in enumerate(chunk[2:-2].split('\n')):
                comments[line + k].append(part.strip(' *'))
            out.append(re.sub(r'[^\n]', ' ', chunk))
            line += chunk.count('\n')
            i = j
        elif c == 'R' and text.startswith('R"', i) and (i == 0 or not (text[i - 1].isalnum() or text[i - 1] == '_')):
            m = re.match(r'R"([^(\s]*)\(', text[i:])
            if not m:
                out.append(c)
                i += 1
                continue
            end = text.find(')' + m.group(1) + '"', i)
            j = n if end < 0 else end + len(m.group(1)) + 2
            chunk = text[i:j]
            out.append(re.sub(r'[^\n]', ' ', chunk))
            line += chunk.count('\n')
            i = j
        elif c in '"\'':
            # char literal vs digit separator (100'000)
            if c == "'" and i > 0 and text[i - 1].isalnum() and i + 1 < n and text[i + 1].isalnum():
                out.append(' ')
                i += 1
                continue
            j = i + 1
            while j < n and text[j] != c and text[j] != '\n':
                j += 2 if text[j] == '\\' else 1
            j = min(j + 1, n)
            out.append(c + ' ' * (j - i - 2) + c if j - i >= 2 else ' ')
            i = j
        else:
            out.append(c)
            i += 1
    return ''.join(out).split('\n'), comments


# --------------------------------------------------------------------------------------------
# Preprocessor: which lines take part in brace counting (first branch of every #if, except #if 0)
# --------------------------------------------------------------------------------------------

PP_RE = re.compile(r'^\s*#\s*(\w+)\s*(.*)$')


def preprocess(code):
    """Return (active[line], directives) ; directives = list of (line, kw, arg) (1-based)."""
    active = [True] * (len(code) + 1)
    directives = []
    stack = []  # (taking_branch, any_taken)
    cont = False
    for idx, ln in enumerate(code, 1):
        if cont:
            active[idx] = False
            cont = ln.rstrip().endswith('\\')
            continue
        m = PP_RE.match(ln)
        on = all(s[0] for s in stack)
        if m:
            kw, arg = m.group(1), m.group(2).strip()
            directives.append((idx, kw, arg))
            active[idx] = False
            cont = ln.rstrip().endswith('\\')
            if kw in ('if', 'ifdef', 'ifndef'):
                take = not (kw == 'if' and re.fullmatch(r'0\b.*', arg))
                stack.append([take, take])
            elif kw in ('elif', 'else') and stack:
                take = not stack[-1][1]
                stack[-1] = [take, stack[-1][1] or take]
            elif kw == 'endif' and stack:
                stack.pop()
            continue
        active[idx] = on
    return active, directives


def if_blocks(directives):
    """For each #if-family directive: (line, kw, arg, else_line, endif_line)."""
    out, stack = [], []
    for ln, kw, arg in directives:
        if kw in ('if', 'ifdef', 'ifndef'):
            stack.append([ln, kw, arg, None, None])
        elif kw in ('else', 'elif') and stack:
            if stack[-1][3] is None:
                stack[-1][3] = ln
        elif kw == 'endif' and stack:
            b = stack.pop()
            b[4] = ln
            out.append(tuple(b))
    return out


# --------------------------------------------------------------------------------------------
# Scope parser: function definitions with line extents.
# --------------------------------------------------------------------------------------------

NAME_BEFORE_PAREN = re.compile(
    r'((?:[A-Za-z_]\w*(?:<[^()]*?>)?\s*::\s*)*(?:operator\s*(?:\(\)|[^\s(]+)|~?[A-Za-z_]\w*))\s*(?:<[^()]*>)?\s*$')
CLASS_HDR = re.compile(r'\b(class|struct|union)\s+(?:alignas\s*\([^)]*\)\s*)?(?:[A-Z_]+\s+)?([A-Za-z_][\w:]*)?[^(]*$')
KEYWORDS = {'if', 'for', 'while', 'switch', 'return', 'sizeof', 'decltype', 'alignof', 'catch',
            'static_assert', 'requires', 'noexcept', 'alignas', '__declspec', 'defined', 'template'}


def first_call_paren(h):
    """Index of the '(' that opens the parameter list of a function header, or -1."""
    depth = 0
    for i, ch in enumerate(h):
        if ch == '<':
            # template brackets: only count when it looks like a template (identifier before)
            pass
        if ch == '(' and depth == 0:
            m = NAME_BEFORE_PAREN.search(h[:i])
            if m:
                nm = m.group(1).split('::')[-1].strip()
                if nm not in KEYWORDS and not nm.startswith('__'):
                    return i
            depth += 1
        elif ch == '(':
            depth += 1
        elif ch == ')':
            depth = max(0, depth - 1)
    return -1


def match_paren(h, i):
    depth = 0
    for j in range(i, len(h)):
        if h[j] == '(':
            depth += 1
        elif h[j] == ')':
            depth -= 1
            if depth == 0:
                return j
    return len(h)


def strip_template(h):
    """Drop leading `template<...>` clauses (balanced angle brackets) from a header."""
    while True:
        m = re.match(r'\s*template\s*<', h)
        if not m:
            return h
        depth, i = 1, m.end()
        while i < len(h) and depth:
            depth += {'<': 1, '>': -1}.get(h[i], 0)
            i += 1
        h = h[i:]


def parse_scopes(code, active):
    """Return (functions, classes). functions: dicts with name/owner/params/start/open/end."""
    funcs, classes = [], []
    stack = []  # entries: dict(kind, name, start)
    hdr, hdr_start = [], None

    def scope_names():
        return [s['name'] for s in stack if s['kind'] in ('ns', 'class') and s['name']]

    for ln, text in enumerate(code, 1):
        if not active[ln]:
            continue
        for ch in text:
            top = stack[-1]['kind'] if stack else None
            if top in ('func', 'block'):
                if ch == '{':
                    stack.append({'kind': top, 'name': '', 'start': ln, 'nested': True})
                elif ch == '}':
                    s = stack.pop()
                    if not s.get('nested'):
                        if s['kind'] == 'func':
                            s['end'] = ln
                            funcs.append(s)
                        hdr, hdr_start = ([], None) if s['kind'] == 'func' else (s['hdr_keep'], s['hdr_start'])
                continue
            if ch == '{':
                h = strip_template(' '.join(''.join(hdr).split()))
                cls_m = CLASS_HDR.search(h)
                if re.search(r'\bnamespace\b', h) and '(' not in h:
                    nm = re.search(r'\bnamespace\s+([\w:]+)', h)
                    stack.append({'kind': 'ns', 'name': nm.group(1) if nm else '', 'start': ln})
                    hdr, hdr_start = [], None
                elif re.search(r'\benum\b', h) and '(' not in h:
                    stack.append({'kind': 'block', 'name': '', 'start': ln, 'hdr_keep': [], 'hdr_start': None})
                    hdr, hdr_start = [], None
                elif cls_m and '=' not in h and ')' not in h.split(':')[0]:
                    nm = (cls_m.group(2) or '').split('::')[-1]
                    stack.append({'kind': 'class', 'name': nm, 'start': ln})
                    hdr, hdr_start = [], None
                elif 'extern "C"' in h or h.startswith('extern'):
                    stack.append({'kind': 'ns', 'name': '', 'start': ln})
                    hdr, hdr_start = [], None
                else:
                    p = first_call_paren(h)
                    eq = h.find('=')
                    is_init_brace = False
                    if p >= 0:
                        close = match_paren(h, p)
                        tail = h[close + 1:]
                        # constructor member initialiser `m_x{...}` / `Base{...}`
                        if re.match(r'\s*(?:const\s*)?(?:noexcept\s*)?:', tail) and not tail.lstrip().startswith('::'):
                            if re.search(r'[\w>]\s*$', h) and not re.search(
                                    r'\b(?:const|override|final|noexcept|mutable)\s*$', h) and not h.endswith(')'):
                                is_init_brace = True
                    if p >= 0 and (eq < 0 or eq > p) and not is_init_brace:
                        nm = NAME_BEFORE_PAREN.search(h[:p]).group(1)
                        nm = re.sub(r'\s+', '', nm)
                        nm = re.sub(r'<[^<>]*(?:<[^<>]*>[^<>]*)*>', '', nm)
                        parts = nm.split('::')
                        close = match_paren(h, p)
                        stack.append({'kind': 'func', 'name': parts[-1], 'qual': parts[:-1],
                                      'scope': scope_names(), 'params': h[p + 1:close],
                                      'header': h, 'start': hdr_start or ln, 'open': ln})
                    else:
                        # data initialiser, member init brace or lambda outside a function
                        stack.append({'kind': 'block', 'name': '', 'start': ln,
                                      'hdr_keep': hdr + ['{...}'] if is_init_brace else [],
                                      'hdr_start': hdr_start if is_init_brace else None})
                        if not is_init_brace:
                            hdr, hdr_start = [], None
                        continue
                continue
            if ch == '}':
                if stack:
                    s = stack.pop()
                    if s['kind'] == 'class':
                        s['end'] = ln
                        classes.append(s)
                hdr, hdr_start = [], None
                continue
            if ch == ';':
                hdr, hdr_start = [], None
                continue
            if hdr_start is None and not ch.isspace():
                hdr_start = ln
            hdr.append(ch)
        hdr.append(' ')
    return funcs, classes


def param_types(params):
    """Normalised list of parameter types (names and default values dropped)."""
    params = params.strip()
    if not params or params == 'void':
        return []
    out, depth, cur = [], 0, ''
    for ch in params:
        if ch in '<([{':
            depth += 1
        elif ch in '>)]}':
            depth -= 1
        if ch == ',' and depth == 0:
            out.append(cur)
            cur = ''
        else:
            cur += ch
    out.append(cur)
    res = []
    for p in out:
        p = p.split('=')[0].strip()
        p = re.sub(r'\b(const|volatile|struct|class|enum)\b', '', p)
        toks = re.findall(r'[A-Za-z_][\w:]*(?:<[^>]*>)?|[*&]+|\.\.\.', p)
        if len(toks) >= 2 and re.match(r'[A-Za-z_]\w*$', toks[-1]) and toks[-2] not in ('*', '&', '&&', '**'):
            toks = toks[:-1]  # parameter name after a plain type
        elif len(toks) >= 2 and re.match(r'[A-Za-z_]\w*$', toks[-1]) and toks[-2] in ('*', '&', '&&', '**'):
            toks = toks[:-1]
        res.append(''.join(toks).replace('::', ''))
    return res


# --------------------------------------------------------------------------------------------
# Hook registrations (with the overload signature, which functions.csv drops)
# --------------------------------------------------------------------------------------------

HOOK_RE = re.compile(r'\b(RH_Scoped(?:\w*Install\w*|VmtInstall))\s*\((.*)$')
CLASS_RE = re.compile(r'\bRH_Scoped(?:Virtual|Named)?Class\s*\(\s*([\w:<>]+)')
NS_RE = re.compile(r'\bRH_ScopedNamespace(?:Name)?\s*\(\s*"?([\w:]+)')
ADDR_RE = re.compile(r'0x([0-9A-Fa-f]{6,8})\b')
ADDR_TABLE_RE = re.compile(r'\{\s*0x([0-9A-Fa-f]{6})\s*,\s*&?([A-Za-z_][\w:]*)\s*[,}]')
CMD_HANDLER_RE = re.compile(r'REGISTER_COMMAND_(?:HANDLER|HANDLER_BEGIN|NOP|UNIMPLEMENTED)\s*\(\s*(COMMAND_\w+)\s*(?:,\s*([\w:]+))?')


def split_args(s):
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


def scan_hooks(files):
    """{(owner, base_name): [hook]} and {base_name: [hook]}; hook = dict(addr, owner, name, sig, file)."""
    by_owner, by_name = defaultdict(list), defaultdict(list)
    cmd_handlers = defaultdict(set)  # handler function name -> opcode names
    for rel, lines in files:
        cls = ns = ''
        depth, scopes, prev = 0, [], ''  # RH_ScopedClass is block-scoped: restore the outer class after `}`
        for ln, line in enumerate(lines, 1):
            code = prev.split('//')[0]  # braces of the previous line
            depth += code.count('{') - code.count('}')
            prev = line
            while scopes and depth < scopes[-1][0]:
                _, cls, ns = scopes.pop()
            if (m := CLASS_RE.search(line)):
                if not scopes or scopes[-1][0] < depth:
                    scopes.append((depth, cls, ns))  # restore point for the end of this block
                cls, ns = m.group(1).split('<')[0].split('::')[-1], ''
            if (m := NS_RE.search(line)):
                ns = m.group(1).split('::')[-1]
            for m in CMD_HANDLER_RE.finditer(line):
                if m.group(2):
                    cmd_handlers[m.group(2).split('::')[-1]].add(m.group(1))
            # address tables such as CCheat's `{ 0x439B20, CCheat::MayhemCheat, ... }`
            for m in ADDR_TABLE_RE.finditer(line):
                parts = m.group(2).split('::')
                h = {'addr': int(m.group(1), 16), 'owner': parts[-2] if len(parts) > 1 else '',
                     'name': parts[-1], 'sig': '', 'overloaded': False, 'file': rel}
                by_owner[(h['owner'], h['name'])].append(h)
                by_name[h['name']].append(h)
            m = HOOK_RE.search(line)
            if not m or line.lstrip().startswith('#define'):
                continue
            macro, rest = m.group(1), m.group(2)
            j = ln
            while rest.count('(') + 1 > rest.count(')') and j < len(lines) and j - ln < 6:
                rest += ' ' + lines[j]
                j += 1
            args = split_args(rest)
            addr = next((int(a.group(1), 16) for a in (ADDR_RE.search(x) for x in args) if a), None)
            if addr is None:
                continue
            fn = args[0]
            if 'Named' in macro and len(args) > 1:
                fn = args[1].strip('"')
            fn = re.sub(r'<.*', '', fn).split('::')[-1].strip()
            sig = ''
            if 'Overloaded' in macro and len(args) > 3:
                s = args[3]
                # last top-level parenthesised group = parameter list
                groups, depth, start = [], 0, None
                for k, ch in enumerate(s):
                    if ch == '(':
                        if depth == 0:
                            start = k
                        depth += 1
                    elif ch == ')':
                        depth -= 1
                        if depth == 0 and start is not None:
                            groups.append(s[start + 1:k])
                sig = groups[-1] if groups else ''
            h = {'addr': addr, 'owner': cls or ns, 'name': fn, 'sig': sig,
                 'overloaded': 'Overloaded' in macro, 'file': rel}
            by_owner[(h['owner'], fn)].append(h)
            by_name[fn].append(h)
    return by_owner, by_name, cmd_handlers


# --------------------------------------------------------------------------------------------
# Marker detection
# --------------------------------------------------------------------------------------------

RBF_RE = re.compile(r'\b(?:notsa::)?bugfixes::(\w+)')
FIXBUGS_RT_RE = re.compile(r'\bIsFixBugs\s*\(')
UNREACH_RE = re.compile(r'\bNOTSA_(?:UNREACHABLE(?:_CASE)?|UNUSED_FUNCTION)\b')
ASSERT_RE = re.compile(r'(?<![\w.])assert\s*\(')
NOTSA_HELPER_RE = re.compile(r'\bnotsa::((?:\w+::)*\w+)')
BUGFIX_CMT = re.compile(r'\bBUG\s*FIX|\bbug[- ]?fix|\bfix(?:ed|es)? (?:a |the |for |this |that )?(?:bug|crash|oob|ub|issue|problem)|'
                        r'^fix(?:ed)?\b|\bFIX\b(?! ?ME)|\badded (?:a |an )?(?:null |nullptr |bounds |sanity |range |extra )?check|'
                        r'\bnull(?:ptr)?[- ]check|\bprevent(?:s|ing)? (?:a |the )?(?:crash|oob|bug|ub|overflow|underflow)|\bsanity check|'
                        r'\bto avoid (?:a |the )?(?:crash|oob|ub|overflow)|\bsafety check|\bcrash fix', re.I)
ORIG_CMT = re.compile(r'\boriginal(?:ly)?\b|\bR\*|\bvanilla\b|\bin SA\b|^SA:|\bthe game (?:did|does)\b|'
                      r'\bRockstar\b|\bPS2\b|\bmobile\b', re.I)
BUG_CMT = re.compile(r'\bbug(?:gy|s)?\b|\bwrong\b|\bincorrect\b|\btypo\b', re.I)
NOTSA_WORD = re.compile(r'\bNOTSA\b')
TODO_CMT = re.compile(r'\b(?:TODO|FIXME|todo|Todo)\b')
TODO_TOPIC = re.compile(r'original|inline|inlined|vanilla|different|differs|wrong|incorrect|\bSA\b|R\*|\bbug', re.I)
POOL_ITER_RE = re.compile(r'\bGetAllValid(?:WithIndex)?\s*(?:<[^()]*>)?\s*\(')
RNG_RE = re.compile(r'(?:\bCGeneral::|(?<![:\w]))GetRandomNumberInRange\s*(<\s*\w+\s*>)?\s*\(')
INLINED_CMT = re.compile(r'\binlined?\b', re.I)

# notsa:: helpers that are pure utilities / tooling (same result as the inline original code, or no
# original counterpart at all: UI, logging, debug modules, platform layer, casts, script plumbing).
HELPER_HARMLESS = {
    'contains', 'dyn_cast_if_present', 'cast', 'isa', 'dyn_cast', 'cast_if_present', 'isa_and_present',
    'isa_and_nonnull', 'mdarray', 'make_mapping', 'ci_string_view', 'ci_string', 'WEnumS8', 'WEnumS16',
    'WEnumS32', 'WEnumU16', 'WEnumU32', 'find_value', 'find_value_or', 'accumulate', 'indexof',
    'array_indexof', 'ScopeGuard', 'PoolOf', 'PooledType', 'NotIsNull', 'IsNull', 'Not', 'coalesce',
    'range_of', 'is_standard_integer', 'is_specialization_of', 'is_any_of_type_v', 'Singleton',
    'IterateFunction', 'log_ptr', 'GetSourceCodeBasePath', 'unreachable', 'ReadAs',
}
HELPER_TOOLING_NS = ('ui', 'debugmodules', 'debug', 'Logging', 'SDLWrapper', 'WindowedMode', 'script',
                     'detail', 'details', 'InjectWindowedModeHooks', 'GetDLLHandle', 'bugfixes')


def classify_comment_kind(cmt):
    if 'FIX_BUGS' in cmt:
        return 'fix-bugs-comment'
    if BUGFIX_CMT.search(cmt) and not re.search(r'\bprefix|\bsuffix|\bfixed[- ]?(?:point|size|length|array|step|timestep|rate)', cmt, re.I):
        return 'bugfix-comment'
    if ORIG_CMT.search(cmt) and not re.search(r'originally (?:part of|from) the LLVM', cmt):
        if INLINED_CMT.search(cmt) and not re.search(r'\bbut\b|\bchang|\bdiffer|\bfix', cmt, re.I):
            return 'inlined'
        return 'original-comment'
    if BUG_CMT.search(cmt):
        return 'bug-note'
    if NOTSA_WORD.search(cmt):
        return 'notsa'
    if TODO_CMT.search(cmt) and TODO_TOPIC.search(cmt):
        return 'todo'
    if INLINED_CMT.search(cmt) and re.search(r'\binlined\b|\binline[ds]? (?:in|into|here|function|code)|\bwas inlined', cmt, re.I):
        return 'inlined'
    return None


GR_KEEPS = re.compile(r'\b(?:too|as well|kept|keep(?:ing)? it|as[- ]is|left as|like (?:the )?original|'
                      r'same as (?:the )?original|bug is kept|kept the bug|leave it)\b', re.I)
HARMLESS_WORDS = re.compile(r'readab|cleaner|nicer|simpler|simplif|refactor|not that it matters|same (?:thing|result|behaviou?r)|'
                            r'identical|equivalent|achieves the same|for convenience|\bhelpers?\b|\blog(?:ging)?\b|debug|imgui|'
                            r'\bsignature\b|\bnamed?\b|naming|\bunused\b|typedef|for consistency|\bRVO|lookup table|'
                            r'abstract class|virtual functions?\b|\bpadding\b|\bwrapper\b|\bmobile\b|\bPS2\b|\bXbox\b', re.I)
KEEP_WORDS = re.compile(r'\binstead\b|\bwe (?:use|do|just|now|try|check|don\'t|won\'t|skip|ignore|handle|return|clamp)|'
                        r'\bwe\'ll\b|\bchanged?\b|\bno longer\b|\bremoved\b|\badded\b|\bfix(?:ed|es)?\b|\bprevent|'
                        r'original (?:code )?(?:didn\'t|doesn\'t|did not|does not|never)\b|'
                        r'\boriginally\b.*\b(?:was|were|used|did|had|returned|checked|tried|wrote|called|squared|looped|continued)\b|'
                        r'\bmissing\b|\buninit|\bleak|division by zero|use-after|\bOOB\b|\bcrash', re.I)


def classify(kind, text, helper=None):
    """Rule-based (deviates, classification) for a marker."""
    if kind == 'rbf':
        return 'yes', 'keep-original'
    if kind in ('fix-bugs-comment', 'bugfix-comment') and re.search(
            r'\btodo\b|\bmaybe\b|\?|\bshould\b|\bcould\b|\bneeds?\b|\bcan\'t\b|\bcannot\b', text, re.I):
        return 'no', 'keep-original'  # a fix gta-reversed only proposes; it runs the original code
    if kind in ('fix-bugs-ifdef', 'fix-bugs-runtime', 'fix-bugs-comment', 'bugfix-comment'):
        if re.search(r'^#\s*if\s+0\b', text.strip()) or re.search(r'\bdisabled\b|leave it as-is', text, re.I):
            return 'no', 'harmless'
        return 'yes', 'keep-original'
    if kind == 'define-branch':
        return 'maybe', 'unclear'
    if kind == 'notsa-unreachable':
        if re.search(r'\boriginal(?:ly)?\b|\bR\*|\bvanilla\b', text, re.I):
            return 'yes', 'keep-original'  # the comment documents what the original does on that path
        return 'maybe', 'unclear'
    if kind == 'rng-helper':
        return 'yes', 'keep-original'
    if kind == 'pool-iteration':
        return 'maybe', 'unclear'
    if kind == 'assert':
        return 'no', 'harmless'
    if kind == 'inlined':
        return 'no', 'harmless'
    if kind == 'notsa-helper':
        top = helper.split('::')[0]
        if helper in HELPER_HARMLESS or top in HELPER_TOOLING_NS:
            return 'no', 'harmless'
        return 'maybe', 'unclear'
    if kind in ('notsa-function', 'notsa-decl') and not re.search(r'0x[0-9A-Fa-f]{6}', text):
        return 'no', 'harmless'  # gta-reversed-only definition; its use sites carry the deviation
    if GR_KEEPS.search(text) and not re.search(r'\binstead\b|\bfix', text, re.I):
        return 'no', 'keep-original'  # gta-reversed says it keeps the original behaviour here
    if HARMLESS_WORDS.search(text) and not re.search(r'\bfix|\bcrash|\bprevent|\bcheck', text, re.I):
        return 'no', 'harmless'
    if KEEP_WORDS.search(text):
        return 'yes', 'keep-original'
    if kind == 'bug-note':
        return 'no', 'keep-original'  # a (suspected) original bug that gta-reversed reproduces
    return 'maybe', 'unclear'


# --------------------------------------------------------------------------------------------
# Catalogue helpers
# --------------------------------------------------------------------------------------------


class Catalogue:
    def __init__(self, cat):
        sys.path.insert(0, str(Path(__file__).resolve().parent))
        import progress  # noqa: E402  (same name/subsystem derivation as state.csv)
        derived = progress.derive(cat)
        self.names = {a: r['name'] for a, r in derived.items()}
        self.subsystem = {a: r['subsystem'] for a, r in derived.items()}
        self.ranges = []  # (lo, hi, fn)
        for r in csv.DictReader(open(cat / 'extents.csv')):
            fn = int(r['address'], 16)
            for rg in r['ranges'].split(';'):
                if rg:
                    lo, hi = (int(x, 16) for x in rg.split('-'))
                    self.ranges.append((lo, hi, fn))
        self.ranges.sort()
        self.los = [r[0] for r in self.ranges]

    def containing(self, va):
        if va in self.names:
            return va
        i = bisect.bisect_right(self.los, va) - 1
        while i >= 0 and i >= bisect.bisect_right(self.los, va) - 8:
            lo, hi, fn = self.ranges[i]
            if lo <= va < hi:
                return fn
            i -= 1
        return None


def short(s, n=170):
    s = ' '.join(s.split())
    return s if len(s) <= n else s[:n - 3] + '...'


# --------------------------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------------------------


def main():
    gr, cat, exe = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    catalogue = Catalogue(cat)

    files = []
    for d in SCAN_DIRS:
        for p in sorted(Path(gr, d).rglob('*')):
            if p.suffix in EXTS and p.is_file():
                files.append((str(p.relative_to(gr)), p.read_text(errors='replace').splitlines()))
    files.sort()
    by_owner, by_name, cmd_handlers = scan_hooks(files)

    sc_rows = list(csv.DictReader(open(cat / 'script-commands.csv')))
    sc_by_name = {r['name']: r for r in sc_rows}

    rows = []
    stats = Counter()

    def resolve(fn, rel):
        """Map a parsed function definition to catalogue addresses: (addrs, mapping)."""
        name = fn['name']
        owners = [q for q in fn['qual']] + list(reversed(fn['scope']))
        owners = [o for o in owners if o] or ['']
        cands = []
        names = [name]
        if owners and name == owners[0]:
            names = ['Constructor']
        elif name.startswith('~'):
            names = ['Destructor']
        for o in owners:
            for nm in names:
                cands = by_owner.get((o, nm), [])
                if cands:
                    break
            if cands:
                break
        if not cands:
            same_file = [h for nm in names for h in by_name.get(nm, []) if h['file'] == rel and not h['owner']]
            cands = same_file or ([h for h in by_name.get(name, []) if h['file'] == rel and owners == ['']])
        if not cands:
            return [], None
        if len(cands) == 1:
            return [cands[0]['addr']], 'hook'
        want = param_types(fn['params'])
        scored = []
        for h in cands:
            have = param_types(h['sig']) if h['overloaded'] else None
            if have is None:
                score = 0
            elif len(have) != len(want):
                score = -100
            else:
                score = 10 + sum(1 for a, b in zip(have, want) if a == b)
            scored.append((score, h['addr']))
        best = max(s for s, _ in scored)
        top = sorted({a for s, a in scored if s == best})
        if len(top) == 1:
            return top, 'hook-overload'
        return top, 'hook-ambiguous'

    def addr_comment(lines, comments, fn):
        """`// 0xADDR` on the definition line or up to 3 lines above it."""
        for ln in range(fn['start'], max(0, fn['start'] - 4), -1):
            for c in comments.get(ln, []):
                m = re.match(r'^\s*(?:@addr(?:ess)?\s*)?0x([0-9A-Fa-f]{6,7})\b', c)
                if m:
                    a = catalogue.containing(int(m.group(1), 16))
                    if a is not None:
                        return a
        return None

    for rel, raw in files:
        text = '\n'.join(raw)
        code, comments = lex(text)
        active, directives = preprocess(code)
        funcs, classes = parse_scopes(code, active)
        blocks = if_blocks(directives)
        fmap = {}

        def fn_at(ln):
            best = None
            for f in funcs:
                if f['start'] <= ln <= f['end'] and (best is None or f['end'] - f['start'] < best['end'] - best['start']):
                    best = f
            return best

        def fn_starting_near(ln):
            for f in funcs:
                if ln <= f['start'] <= ln + 3 and not any(
                        code[k - 1].strip() and not comments.get(k) for k in range(ln + 1, f['start'])
                        if k - 1 < len(code) and not code[k - 1].strip().startswith('#')
                        and code[k - 1].strip() not in ('template', '') and 'template' not in code[k - 1]):
                    return f
            return None

        def class_at(ln):
            best = None
            for c in classes:
                if c['start'] <= ln <= c['end'] and (best is None or c['start'] > best['start']):
                    best = c
            return best

        def mapping_for(fn):
            key = id(fn)
            if key in fmap:
                return fmap[key]
            addrs, how = resolve(fn, rel)
            if not addrs:
                a = addr_comment(raw, comments, fn)
                if a is not None:
                    addrs, how = [a], 'addr-comment'
            if not addrs and fn['name'] in cmd_handlers:
                ops = sorted(cmd_handlers[fn['name']])
                fns = sorted({sc_by_name[o]['orig_range_fn'] for o in ops if o in sc_by_name and sc_by_name[o]['orig_range_fn']})
                if fns:
                    addrs, how = [int(x, 16) for x in fns], 'script-handler'
                    fn['opcodes'] = ops
            if not addrs:
                how = 'gr-only'
            fmap[key] = (addrs, how)
            return addrs, how

        def lambda_command(fn, ln):
            """Opcode of a REGISTER_COMMAND_HANDLER(COMMAND_X, [](...){...}) lambda around line ln."""
            for k in range(ln, fn['open'] - 1, -1):
                m = re.search(r'REGISTER_COMMAND_HANDLER(?:_BEGIN)?\s*\(\s*(COMMAND_\w+)\s*,\s*\[', raw[k - 1])
                if m:
                    seg = '\n'.join(code[k - 1:ln])
                    if seg.count('{') > seg.count('}') or k == ln:
                        return m.group(1)
                    return None
            return None

        def emit(ln, kind, text, fn_override=None, extra=''):
            fn = fn_override or fn_at(ln)
            gr_fn, addrs, how = '', [], 'none'
            if fn is not None:
                gr_fn = '::'.join([*(fn['qual'] or fn['scope'][-1:]), fn['name']])
                addrs, how = mapping_for(fn)
                op = lambda_command(fn, ln) if fn['name'] in ('InjectHooks', 'RegisterHandlers') else None
                if op and op in sc_by_name and sc_by_name[op]['orig_range_fn']:
                    addrs, how = [int(sc_by_name[op]['orig_range_fn'], 16)], 'script-handler'
                    gr_fn = f'{gr_fn}/{op}'
            else:
                c = class_at(ln)
                decl = re.search(r'([~A-Za-z_]\w*)\s*\(', code[ln - 1]) if ln - 1 < len(code) else None
                if c is not None and decl and decl.group(1) not in KEYWORDS:
                    pseudo = {'name': decl.group(1), 'qual': [c['name']], 'scope': [], 'params':
                              code[ln - 1][decl.end():].split(')')[0], 'start': ln}
                    gr_fn = f"{c['name']}::{decl.group(1)} (declaration)"
                    a, h = resolve(pseudo, rel)
                    if a:
                        addrs, how = a, h
                    if kind == 'notsa':
                        kind = 'notsa-decl'
            # comment-referenced address for gr-only / unmapped rows
            if not addrs and kind not in ('rng-helper', 'pool-iteration'):
                m = re.search(r'0x([0-9A-Fa-f]{6,7})\b', ' '.join(comments.get(ln, [])))
                if m:
                    a = catalogue.containing(int(m.group(1), 16))
                    if a is not None:
                        addrs, how = [a], 'addr-comment'
            dev, cls = classify(kind, text, extra or None)
            desc = short(text)
            if fn is not None and fn.get('opcodes'):
                desc = short(f"[{','.join(fn['opcodes'])}] {text}")
            rows.append({
                'address': f'{addrs[0]:#08x}' if addrs else '',
                'name': catalogue.names.get(addrs[0], '') if addrs else '',
                'mapping': how + (f" ({' '.join(f'{a:#08x}' for a in addrs)})" if len(addrs) > 1 else '')
                + (' not-in-catalogue' if addrs and addrs[0] not in catalogue.names else ''),
                'gr_function': gr_fn,
                'file': rel, 'line': ln, 'kind': kind, 'deviates': dev, 'classification': cls,
                'description': desc, 'evidence': '',
            })

        # 1. preprocessor blocks
        handled = set()
        for (ln, kw, arg, else_ln, end_ln) in blocks:
            if re.search(r'\bFIX_BUGS\b', arg) or (kw == 'if' and re.match(r'0\b', arg) and any('FIX_BUGS' in c for c in comments.get(ln, []))):
                neg = kw == 'ifndef' or arg.replace(' ', '').startswith(('!FIX_BUGS', '!defined'))
                if neg:
                    orig = 'this branch (gr compiles the #else part or nothing)'
                else:
                    orig = 'the #else branch' if else_ln else 'absence of the block'
                cmt = ' '.join(c for k in range(ln, min(ln + 3, end_ln)) for c in comments.get(k, []))
                txt = f"#{kw} {arg} block, lines {ln}-{end_ln}; original = {orig}. {cmt}"
                if kw == 'if' and re.match(r'0\b', arg):
                    txt = f"#if 0 FIX_BUGS block (fix disabled, gr keeps the original), lines {ln}-{end_ln}. {cmt}"
                fo = fn_at(ln) or fn_starting_near(ln)
                emit(ln, 'fix-bugs-ifdef', txt, fo)
                handled.add(ln)
            elif any(re.search(rf'\b{d}\b', arg) for d in BUILD_DEFINES):
                cmt = ' '.join(comments.get(ln, []))
                d = next(d for d in BUILD_DEFINES if re.search(rf'\b{d}\b', arg))
                emit(ln, 'define-branch', f"#{kw} {arg} (gta-reversed build define {d}), lines {ln}-{end_ln}. {cmt}")
                handled.add(ln)

        # 2. line markers
        for ln in range(1, len(raw) + 1):
            if ln in handled:
                continue
            cline = code[ln - 1] if ln - 1 < len(code) else ''
            cmts = comments.get(ln, [])
            cmt = ' '.join(cmts)
            kind, extra = None, ''
            if (m := RBF_RE.search(cline + ' ' + raw[ln - 1] if 'bugfixes' in raw[ln - 1] else cline)) and 'include' not in raw[ln - 1]:
                kind, extra = 'rbf', m.group(1)
                if extra in ('ReversibleBugFix',):
                    kind = None
            if kind is None and FIXBUGS_RT_RE.search(cline):
                kind = 'fix-bugs-runtime'
            if kind is None and cline.lstrip().startswith('#'):
                continue
            if kind is None and UNREACH_RE.search(cline):
                kind = 'notsa-unreachable'
            if kind is None and cmt:
                kind = classify_comment_kind(cmt)
            if kind is None and NOTSA_WORD.search(cline):
                kind = 'notsa'
            if kind is None and ASSERT_RE.search(cline) and 'static_assert' not in cline:
                kind = 'assert'
            if kind is None and (m := NOTSA_HELPER_RE.search(cline)):
                helpers = [h for h in NOTSA_HELPER_RE.findall(cline)]
                # the most specific non-harmless helper decides
                risky = [h for h in helpers if classify('notsa-helper', '', h)[1] != 'harmless']
                kind, extra = 'notsa-helper', (risky or helpers)[0]
            if POOL_ITER_RE.search(cline) and fn_at(ln) is not None:
                emit(ln, 'pool-iteration', 'pool iterated with notsa GetAllValid (ascending slot order, '
                     'skipping free slots): ' + raw[ln - 1].strip())
            if (m := RNG_RE.search(cline)) and fn_at(ln) is not None and (
                    'CGeneral::' in m.group(0) or ('/Audio/' not in rel and not rel.endswith('/General.h'))):
                emit(ln, 'rng-helper', 'gta-reversed CGeneral::GetRandomNumberInRange (formula differs from '
                     '0x407180 / 0x41BD90, see docs/re/cat14-gr-deviations.md): ' + raw[ln - 1].strip())
            if kind is None:
                continue
            fn = fn_at(ln)
            if fn is None and kind in ('notsa', 'original-comment', 'bugfix-comment', 'bug-note', 'todo', 'inlined'):
                # comment right before a definition describes that function
                f2 = fn_starting_near(ln)
                if f2 is not None:
                    if kind == 'notsa':
                        kind = 'notsa-function'
                    emit(ln, kind, (cmt or raw[ln - 1].strip()), f2, extra)
                    continue
            text = raw[ln - 1].strip() if kind in ('rbf', 'fix-bugs-runtime', 'assert', 'notsa-helper', 'notsa-unreachable') else (cmt or raw[ln - 1].strip())
            if kind in ('rbf', 'notsa-unreachable', 'fix-bugs-runtime') and cmt and cmt not in text:
                text = f'{text} // {cmt}'
            if kind == 'rbf':
                text = f'bugfixes::{extra}: {text}'
            emit(ln, kind, text, None, extra)

    # 3. script opcodes whose gr status disagrees with the exe dispatch
    md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
    pe = pefile.PE(str(exe), fast_load=True)
    img = pe.get_memory_mapped_image()
    base = pe.OPTIONAL_HEADER.ImageBase

    def case_body(va):
        out = []
        for ins in md.disasm(img[va - base: va - base + 64], va):
            out.append(f'{ins.mnemonic} {ins.op_str}'.strip())
            if ins.mnemonic in ('ret', 'jmp') or len(out) >= 4:
                break
        return '; '.join(out)

    for r in sc_rows:
        if r['gr_status'] == 'handler' and r['orig_dispatch'] == 'default':
            a = int(r['orig_range_fn'], 16)
            f, ln = r['gr_source'].rsplit(':', 1)
            rows.append({
                'address': f'{a:#08x}', 'name': catalogue.names.get(a, ''),
                'mapping': 'script-opcode' + ('' if a in catalogue.names else ' not-in-catalogue'),
                'gr_function': f"{r['opcode']} {r['name']}", 'file': f, 'line': int(ln), 'kind': 'script-default',
                'deviates': 'yes', 'classification': 'keep-original',
                'description': f"gta-reversed implements opcode {r['opcode']}; the PC exe's {r['orig_range_fn']} sends it to the switch default (no handler)",
                'evidence': f"script-commands.csv: orig_dispatch=default in {r['orig_range_fn']} (jump-table decode, CAT-10)",
            })
        elif r['gr_status'] == 'unimplemented' and r['orig_dispatch'] == 'case':
            a = int(r['orig_range_fn'], 16)
            f, ln = r['gr_source'].rsplit(':', 1)
            h = int(r['orig_handler'], 16)
            rows.append({
                'address': f'{a:#08x}', 'name': catalogue.names.get(a, ''),
                'mapping': 'script-opcode' + ('' if a in catalogue.names else ' not-in-catalogue'),
                'gr_function': f"{r['opcode']} {r['name']}", 'file': f, 'line': int(ln), 'kind': 'script-unimpl-case',
                'deviates': 'yes', 'classification': 'keep-original',
                'description': f"gta-reversed registers opcode {r['opcode']} as unimplemented (logs, returns OR_INTERRUPT); the exe has a case at {r['orig_handler']} (shared by {r['orig_case_shared']} opcodes)",
                'evidence': f"case {r['orig_handler']} starts: {case_body(h)}",
            })

    # 4. hand-checked notes override the rules
    notes_path = cat / 'gr-deviations-notes.csv'
    notes_used = 0
    if notes_path.exists():
        idx = defaultdict(list)
        for i, r in enumerate(rows):
            idx[(r['file'], r['line'])].append(i)
        src = {rel: raw for rel, raw in files}
        for n in csv.DictReader(open(notes_path)):
            key = (n['file'], int(n['line']))
            line_text = src.get(n['file'], [''] * (key[1]))[key[1] - 1] if n['file'] in src else ''
            if n['contains'] and n['contains'] not in line_text and not any(n['contains'] in rows[i]['description'] for i in idx.get(key, [])):
                print(f"gr-deviations-notes: {n['file']}:{n['line']} does not contain {n['contains']!r}; skipped")
                continue
            hits = [i for i in idx.get(key, []) if not n.get('kind') or rows[i]['kind'] == n['kind']]
            if not hits:
                if not n.get('kind') or not n.get('deviates'):
                    print(f"gr-deviations-notes: no row at {n['file']}:{n['line']}; skipped")
                    continue
                # a deviation the markers do not show: added by hand
                a = int(n['address'], 16) if n.get('address') else None
                rows.append({'address': f'{a:#08x}' if a is not None else '',
                             'name': catalogue.names.get(a, '') if a is not None else '', 'mapping': 'hand',
                             'gr_function': n.get('gr_function', ''), 'file': n['file'], 'line': key[1],
                             'kind': n['kind'], 'deviates': n['deviates'], 'classification': n['classification'],
                             'description': n['description'], 'evidence': n['evidence']})
                notes_used += 1
                continue
            for i in hits:
                for k in ('deviates', 'classification', 'description', 'evidence', 'address', 'gr_function'):
                    if n.get(k):
                        if k == 'address':
                            a = int(n[k], 16)
                            if f'{a:#08x}' != rows[i]['address']:
                                rows[i]['address'] = f'{a:#08x}'
                                rows[i]['name'] = catalogue.names.get(a, '')
                                rows[i]['mapping'] = 'hand'
                        else:
                            rows[i][k] = n[k]
                notes_used += 1

    rows.sort(key=lambda r: (r['file'], r['line'], r['kind']))
    cols = ['address', 'name', 'mapping', 'gr_function', 'file', 'line', 'kind', 'deviates',
            'classification', 'description', 'evidence']
    with open(cat / 'gr-deviations.csv', 'w', newline='') as f:
        w = csv.DictWriter(f, cols, lineterminator='\n')
        w.writeheader()
        w.writerows(rows)

    # summary for the docs
    sub = {f'{a:#08x}': v for a, v in catalogue.subsystem.items()}
    summ = {
        'rows': len(rows),
        'notes_applied': notes_used,
        'by_kind': dict(sorted(Counter(r['kind'] for r in rows).items())),
        'by_class': dict(sorted(Counter(r['classification'] for r in rows).items())),
        'by_deviates': dict(sorted(Counter(r['deviates'] for r in rows).items())),
        'by_mapping': dict(sorted(Counter(r['mapping'].split(' ')[0] for r in rows).items())),
        'by_class_deviates': dict(sorted(Counter(f"{r['classification']}/{r['deviates']}" for r in rows).items())),
        'by_kind_class': {k: dict(sorted(Counter(r['classification'] for r in rows if r['kind'] == k).items()))
                          for k in sorted({r['kind'] for r in rows})},
        'mapped_rows': sum(1 for r in rows if r['address']),
        'mapped_not_in_catalogue': sum(1 for r in rows if r['address'] and int(r['address'], 16) not in catalogue.names),
        'not_in_catalogue_addresses': sorted({r['address'] for r in rows
                                              if r['address'] and int(r['address'], 16) not in catalogue.names}),
        'functions_with_rows': len({r['address'] for r in rows if r['address']}),
        'functions_counted': len({r['address'] for r in rows if r['address'] and r['deviates'] != 'no'}),
        'by_subsystem_counted': dict(sorted(Counter(sub.get(r['address'], '?') for r in rows
                                                    if r['address'] and r['deviates'] != 'no').items(),
                                            key=lambda kv: (-kv[1], kv[0]))),
        'keep_original_by_subsystem': dict(sorted(Counter(sub.get(r['address'], '?') for r in rows
                                                          if r['address'] and r['classification'] == 'keep-original'
                                                          and r['deviates'] == 'yes').items(),
                                                  key=lambda kv: (-kv[1], kv[0]))),
    }
    with open(cat / 'gr-deviations-summary.json', 'w') as f:
        json.dump(summ, f, indent=1, sort_keys=False)
        f.write('\n')
    print(f"gr-deviations: {len(rows)} rows, {summ['mapped_rows']} mapped to {summ['functions_with_rows']} functions; "
          f"by kind {summ['by_kind']}; by class {summ['by_class']}; by mapping {summ['by_mapping']}; "
          f"notes applied {notes_used}")


if __name__ == '__main__':
    main()
