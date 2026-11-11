"""AUDIO-GEN (PORT-AUDIO-2): generate sa-audio ports from the 1.0 US exe bytes with AI-GEN's interpreter.

Usage: .venv/bin/python -I tools/gen/audio_patterns.py [--check]

Reuses `tools/gen/ai_patterns.py` unchanged (its exact symbolic x86 interpreter, emitter, hookability
rules and body-hash test); only the candidate set and the output location differ:

- candidates: catalogue subsystem AUDIO, plus the UNK rows `catalog/unk-classification-proposal.csv`
  assigns to AUDIO, minus every address already ported in `crates/*/src` (`#[hook(addr`), the
  PORT-AUDIO-1 functions ported without a hook (`PORTED_UNHOOKED`), the out-of-scope classes (sound
  output and decoding stay original in phase A: DirectSound channels, the stream thread, decoders,
  data streams, the smooth-fade thread; `EXCLUDE_CLASS`) and `EXCLUDE_ADDR` (determinism-mode
  hooks, patched OS call sites, `AudioLog10`);
- every function the interpreter executes completely is emitted, not only AI-GEN's named families: a
  body outside every family goes to `gen_bodies.rs` (family `body`);
- output: `crates/sa-audio/src/gen_*.rs`, reports in `target/audio-gen/{report,rest}.tsv`.

The output is a pure function of the exe, the catalogue and the ported-address set, so two runs give
byte-identical files; `--check` fails when a generated file differs.
"""
import csv
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import ai_patterns as A  # noqa: E402

ROOT = A.ROOT
A.OUT_DIR = os.path.join(ROOT, 'crates', 'sa-audio', 'src')
A.REPORT_DIR = os.path.join(ROOT, 'target', 'audio-gen')
A.FAMILY_FILE['body'] = 'gen_bodies'
A.FILE_DOC['gen_bodies'] = ('bodies outside the named families that the interpreter executed completely\n'
                            '//! (forwarders, small dispatchers, field updates with calls)')

# Sound output and decoding stay original in phase A (PORT-AUDIO scope).
EXCLUDE_CLASS = re.compile(r'^(CAEAudioChannel|CAEStaticChannel|CAEStreamingChannel|CAEStreamThread|'
                           r'CAE\w*Decoder|CAEDataStream\w*|CAESmoothFadeThread|CAEMFDecoder)$')
# CAEAudioHardware::Service (determinism hook), the thread Starts (patched ResumeThread sites),
# CAEAudioUtility::GetCurrentTimeInMS (patched QPC site), AudioLog10 (x87 fyl2x, waits for the X87 branch).
EXCLUDE_ADDR = {0x4D9870, 0x4F11F0, 0x4EEA10, 0x4D9E80, 0x4D9E50}
# PORT-AUDIO-1 ports reached through orig! only (docs/arch/audio-port.md "Not hooked").
PORTED_UNHOOKED = {0x4D9E10, 0x4E37B0, 0x4EF880, 0x4E44F0, 0x4E4600, 0x4F4FF0, 0x4F6000, 0x4F5F30,
                   0x4F5D00, 0x4F5C60, 0x4F60B0, 0x4F6150, 0x4F51F0, 0x4F53D0, 0x4FA0C0, 0x4F8DF0}

# Matched by the interpreter but not generated: an original caller keeps EDX across the call (sa-hook
# `callers` test; hookable with `preserve` once an L1 spec compares the register).
SKIP = {
    0x4E4F10: 'CAEPedSpeechAudioEntity::Constructor: caller 0x4E6072 reads EDX at 0x4E6077',
}

GEN_FILES =('gen_leaves', 'gen_virtuals', 'gen_ctors', 'gen_dtors', 'gen_bodies')


def already_ported():
    done = set()
    for dirpath, _, files in os.walk(os.path.join(ROOT, 'crates')):
        if '/target' in dirpath:
            continue
        in_audio = os.sep + 'sa-audio' + os.sep in dirpath + os.sep
        for f in sorted(files):
            if not f.endswith('.rs') or (in_audio and f.startswith('gen_')):
                continue
            s = open(os.path.join(dirpath, f), encoding='utf-8').read()
            for m in re.finditer(r'#\[hook\(addr = 0x([0-9A-Fa-f]+)', s):
                done.add(int(m.group(1), 16))
    return done


def unk_audio():
    out = set()
    for r in csv.DictReader(open(os.path.join(ROOT, 'catalog', 'unk-classification-proposal.csv'))):
        if r['proposed_subsystem'] == 'AUDIO':
            out.add(int(r['address'], 16))
    return out


def bool_methods():
    """C++ `bool` members (Can*/Is*/Has*/Does*/Will*/Are*): only AL is the result. A path that returns a
    callee's EAX returns AL only (port-l2-a2gen-air-water: CanPedHoldConversation returned the upper EAX
    bytes CanPedSayGlobalContext leaves from its caller, 0x850000 vs 0x4E5700)."""
    out = set()
    for r in A.PROCS.values():
        m = A.method_of(r['name'] or '')
        if re.match(r'(Can|Is|Has|Does|Will|Are)[A-Z]', m):
            out.add(m)
    return out


def main():
    check = '--check' in sys.argv
    A.BOOL_METHODS = set(A.BOOL_METHODS) | bool_methods()
    done = already_ported() | PORTED_UNHOOKED | EXCLUDE_ADDR
    extra = unk_audio()
    cand = []
    for a in sorted(A.PROCS):
        r = A.PROCS[a]
        if r['subsystem'] != 'AUDIO' and a not in extra:
            continue
        if a in done or r['state'] == 'excluded':
            continue
        name = A.catname(a) or 'FUN_%08x' % a
        if '(' in name or ' ' in name:
            # placeholder catalogue names ("(missed by ghidra: ...)"): sa-hook's table test wants the
            # catalogue name as the hook name, which no hook list can carry; left original
            continue
        if a in SKIP:
            continue
        if EXCLUDE_CLASS.match(A.cls_of(name)):
            continue
        cand.append((a, name))
    gens, rest, seen = {}, [], set()
    for a, name in cand:
        try:
            fn = A.analyse(a, name)
            fam = A.family(name, fn) or 'body'
            if fam in ('subtask', 'make-abortable', 'clone'):
                fam = 'body'
            if fam == 'dtor' and fn.ret == 'u32':
                rv = [x[1] for x in A.flatten(fn.out) if x[0] == 'ret']
                calls = {x[1] for x in A.flatten(fn.out) if x[0] == 'call'}
                if all(v is not None and v[0] == 'v' and v[1] in calls for v in rv):
                    fn.ret = 'void'
            em = A.Emit(fn)
            body = em.stmts(fn.out, 1)
            text = '\n'.join(body)
            if A.nm_static(name) and fn.conv == 'thiscall' and not fn.uses_this and fn.nargs == 0:
                fn.conv = 'cdecl'
            if re.search(r'\bthis\b', text) and fn.conv != 'thiscall':
                raise A.Reject('reads ECX in a caller-cleaned function')
            fn.uses_this = bool(re.search(r'\bthis\b', text))
            for m in sorted(set(re.findall(r'let (v\d+) =', text))):
                if len(re.findall(r'\b%s\b' % m, text)) == 1:
                    body = [re.sub(r'let %s =' % m, 'let _%s =' % m, l) for l in body]
        except A.Reject as ex:
            rest.append((a, name, str(ex)))
            continue
        except RecursionError:
            rest.append((a, name, 'recursion'))
            continue
        fid = A.ident(name, a)
        while fid in seen:
            fid += '_b'
        seen.add(fid)
        gens.setdefault(A.FAMILY_FILE[fam], []).append((a, name, fid, fn, fam, A.strip_returns(body)))
    files, report = {}, []
    for f in GEN_FILES:
        text = A.render(f, gens.get(f, []), report)
        text = text.replace('`tools/gen/ai_patterns.py` (AI-GEN)', '`tools/gen/audio_patterns.py` (AUDIO-GEN, AI-GEN\'s interpreter)')
        text = text.replace('`docs/arch/ai-gen.md`', '`docs/arch/ai-gen.md` and `docs/arch/audio-port.md`')
        files[f] = text
    os.makedirs(A.REPORT_DIR, exist_ok=True)
    bad = []
    for f, text in sorted(files.items()):
        path = os.path.join(A.OUT_DIR, f + '.rs')
        if check:
            old = open(path).read() if os.path.exists(path) else None
            if old != text:
                bad.append(f)
        else:
            with open(path, 'w') as fh:
                fh.write(text)
    with open(os.path.join(A.REPORT_DIR, 'report.tsv'), 'w') as fh:
        for row in sorted(report):
            print(*row, sep='\t', file=fh)
    with open(os.path.join(A.REPORT_DIR, 'rest.tsv'), 'w') as fh:
        for a, n, why in rest:
            print('0x%X' % a, n, why, sep='\t', file=fh)
    counts = {}
    for row in report:
        k = (row[3], row[4] == 'hook')
        counts[k] = counts.get(k, 0) + 1
    for k in sorted(counts):
        print('%-16s %-10s %d' % (k[0], 'hook' if k[1] else 'unhookable', counts[k]))
    print('candidates', len(cand), 'generated', len(report), 'hooks', sum(1 for r in report if r[4] == 'hook'),
          'rest', len(rest))
    if bad:
        print('differs:', ', '.join(bad))
        sys.exit(1)


if __name__ == '__main__':
    sys.setrecursionlimit(10000)
    main()
