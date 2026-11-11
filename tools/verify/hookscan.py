#!/usr/bin/env python3
"""hookscan.py CRATE_SRC_DIR... : two checks for every `#[hook(addr = ...)]` found under the given dirs.

1. INF-20 protection data reads: every dword constant in the game .text (0x401000-0x857000) and .HOODLUM
   that lies within [addr - 3, addr + 5) of a hook entry (the bytes the installer patches), decoded from
   up to 7 bytes before it: MEM = a memory operand reading those bytes, imm = an immediate (a function
   pointer at +0 is skipped), undecoded = check by hand. A hit means the no-CD
   protection (or anything else) reads the patched bytes as data: do not register that hook.
2. Implicit ECX/EDX use after a call into the hook: for every direct `call addr` in game .text, the
   following instructions up to the first write of ECX (resp. EDX) are scanned; reported are explicit
   reads, and for ECX a following `call` reached before ECX is written (the callee may be a thiscall
   that receives the caller's ECX unchanged; check the callee by hand, PHYS ApplyMoveSpeed case).
   Paths end at ret/jmp/call; conditional branches are followed on the fall-through path only.
`push ecx` (stack reservation) is not counted as a read. Run: SA_ROOT=<checkout holding re/> .venv/bin/python -I tools/verify/hookscan.py crates/<crate>/src ... (PORT-WEAPON-PED)

Both checks now run as sa-hook tests over every registered hook (HARNESS-2): check 1 as
`patch_refs::tests::nothing_references_the_inside_of_a_patched_entry` (crates/sa-hook/src/patch_refs.rs:
immediates and displacements on decoded instructions of catalogued code ranges, all executable sections,
plus every direct branch into a patched range from anywhere) next to `data_reads.rs` (protection reads,
catalog/code-data-reads.csv); check 2 as `callers::tests::callers_reading_ecx_edx_after_a_hook_are_declared`
(path-following). This script stays as a quick pre-registration scan of a crate's not yet registered hooks.
"""
import re, sys, os, csv
import pefile, capstone
from capstone import x86

ROOT = os.environ.get('SA_ROOT') or os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
EXE = ROOT + '/re/gta/1.0-US/gta_sa.exe'
CAT = ROOT + '/catalog/procedures.csv'
pe = pefile.PE(EXE, fast_load=True)
base = pe.OPTIONAL_HEADER.ImageBase
img = pe.get_memory_mapped_image()
names = {}
for r in csv.DictReader(open(CAT)):
    names[int(r['address'], 16)] = r['name'] or r['gr_name'] or r['ghidra_name']

hooks = {}
for d in sys.argv[1:]:
    for root, _, files in os.walk(d):
        for f in files:
            if not f.endswith('.rs'):
                continue
            for m in re.finditer(r'#\[hook\(addr\s*=\s*(0x[0-9A-Fa-f]+)[^\]]*name\s*=\s*"([^"]*)"[^\]]*\]', open(os.path.join(root, f)).read()):
                hooks[int(m.group(1), 16)] = (m.group(2), os.path.join(root, f), 'preserve' in m.group(0))
if not hooks:
    print('no hooks'); sys.exit(0)

md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True
md.skipdata = True
secs = []
for s in pe.sections:
    if s.Characteristics & 0x20000000 and (base + s.VirtualAddress) in (0x401000, 0x1556000):  # game .text, .HOODLUM
        secs.append((base + s.VirtualAddress, max(s.Misc_VirtualSize, s.SizeOfRawData)))

ranges = sorted((a - 3, a + 5, a) for a in hooks)
def hit(disp):
    for lo, hi, a in ranges:
        if lo <= disp < hi:
            return a
    return None

print('== 1. data reads of patched entry bytes (raw dword search in code sections, decoded around each hit)')
nd = 0
for va, size in secs:
    code = bytes(img[va - base: va - base + size])
    for a in sorted(hooks):
        for k in range(-3, 5):
            pat = ((a + k) & 0xFFFFFFFF).to_bytes(4, 'little')
            start = 0
            while True:
                i = code.find(pat, start)
                if i < 0:
                    break
                start = i + 1
                kind = 'undecoded'
                imm = None
                for back in range(1, 8):
                    if i - back < 0:
                        continue
                    ins = next(md.disasm(code[i - back:i - back + 15], va + i - back), None)
                    if ins is None or ins.size < back + 4:
                        continue
                    try:
                        ops = ins.operands
                    except Exception:
                        continue
                    if any(o.type == x86.X86_OP_MEM and (o.mem.disp & 0xFFFFFFFF) == a + k for o in ops):
                        kind = f'MEM {ins.address:#x}: {ins.mnemonic} {ins.op_str}'
                        break
                    if imm is None and any(o.type == x86.X86_OP_IMM and (o.imm & 0xFFFFFFFF) == a + k for o in ops):
                        imm = f'imm {ins.address:#x}: {ins.mnemonic} {ins.op_str}'
                if kind == 'undecoded' and imm:
                    kind = imm
                if kind.startswith('imm') and k == 0:
                    continue  # a function pointer pushed/stored as an immediate: not a read of the bytes
                nd += 1
                print(f'  {va + i:#x} (+{k}): {kind}  -> entry of {a:#x} {hooks[a][0]}')
print(f'  {nd} hit(s) (MEM = a data read of the patched bytes: do not register; undecoded = check by hand)')

print('== 2. ECX/EDX use after calls into hooks')
text = img[0x401000 - base: 0x857000 - base]
md2 = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md2.detail = True
ECX = {x86.X86_REG_ECX, x86.X86_REG_CX, x86.X86_REG_CL, x86.X86_REG_CH}
EDX = {x86.X86_REG_EDX, x86.X86_REG_DX, x86.X86_REG_DL, x86.X86_REG_DH}

def uses_ecx(t, depth=0):
    """True if the callee reads ECX before writing it (likely thiscall/fastcall); follows E9 stubs."""
    code = img[t - base: t - base + 120]
    for ins in md2.disasm(code, t):
        if ins.mnemonic == 'jmp' and ins.operands[0].type == x86.X86_OP_IMM and depth < 2:
            return uses_ecx(ins.operands[0].imm, depth + 1)
        try:
            rr, ww = ins.regs_access()
        except Exception:
            return True
        if set(rr) & ECX and not (ins.mnemonic == 'xor' and ins.op_str == 'ecx, ecx') and not (ins.mnemonic == 'push' and ins.op_str == 'ecx'):
            return True
        if set(ww) & ECX or ins.mnemonic in ('ret', 'call', 'jmp'):
            return False
    return False

def scan_after(site_end):
    out = []
    code = img[site_end - base: site_end - base + 200]
    live = {'ecx': True, 'edx': True}
    for ins in md2.disasm(code, site_end):
        try:
            rr, ww = ins.regs_access()
        except Exception:
            break
        rr, ww = set(rr), set(ww)
        if ins.mnemonic == 'call':
            t = ins.operands[0].imm if ins.operands[0].type == x86.X86_OP_IMM else None
            if live['ecx'] and (t is None or uses_ecx(t)):
                out.append(f'ecx-into-call {ins.address:#x} -> {t and hex(t)} {names.get(t, "") if t else ins.op_str}')
            break
        ops = [o.strip() for o in ins.op_str.split(',')]
        selfop = ins.mnemonic in ('xor', 'sub', 'sbb') and len(ops) == 2 and ops[0] == ops[1]
        for reg, S in (('ecx', ECX), ('edx', EDX)):
            if selfop and (rr & S):
                # `xor ecx, ecx` / `xor cl, cl`: a write, not a read (a partial self-xor is treated as
                # killing the register too: the upper bits are rarely read afterwards)
                live[reg] = False
                continue
            if selfop:
                continue
            if live[reg] and (rr & S) and not (ins.mnemonic == 'push' and ins.op_str == reg):
                out.append(f'{reg}-read {ins.address:#x}: {ins.mnemonic} {ins.op_str}')
                live[reg] = False
            if live[reg] and (ww & S) and not (rr & S):
                live[reg] = False
        if ins.mnemonic in ('ret', 'jmp') or not (live['ecx'] or live['edx']):
            break
    return out

for i in range(len(text) - 5):
    if text[i] != 0xE8:
        continue
    site = 0x401000 + i
    tgt = (site + 5 + int.from_bytes(text[i + 1:i + 5], 'little', signed=True)) & 0xFFFFFFFF
    if tgt in hooks:
        res = scan_after(site + 5)
        if res:
            print(f'  {tgt:#x} {hooks[tgt][0]}{" (preserve declared)" if hooks[tgt][2] else ""}: call at {site:#x}: ' + '; '.join(res))
