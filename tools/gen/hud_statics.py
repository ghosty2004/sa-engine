"""Generate the header-constant part of crates/sa-hud/src/statics2.rs (PORT-HUD-2) from the exe bytes into
target/scratch/statics2_gen.rs (pasted above the hand-written part). Usage: hud_statics.py REPO_ROOT.
Each body must be exactly `fld [A]; (fmul|fdiv) [B]; [fdivr [C];] fstp [G]; ret`."""
import csv, os, struct, sys
import capstone, pefile

ROOT = sys.argv[1]
EXE = '/home/ghosty2004/sa-engine/re/gta/1.0-US/gta_sa.exe'
pe = pefile.PE(EXE, fast_load=True)
IMG = pe.get_memory_mapped_image()
BASE = pe.OPTIONAL_HEADER.ImageBase
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True


def rd(a, n):
    return IMG[a - BASE:a - BASE + n]


def f32bits(a):
    return struct.unpack('<I', rd(a, 4))[0]


rows = [r for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/unk-classification-proposal.csv')))
        if r['proposed_subsystem'] == 'HUD' and '(header constant)' in r['proposed_name']]
names = {r['address']: r['name'] for r in csv.DictReader(open(os.path.join(ROOT, 'catalog/procedures.csv')))}
out = []
regs = []
for r in sorted(rows, key=lambda r: int(r['address'], 16)):
    a = int(r['address'], 16)
    ins = []
    for i in md.disasm(rd(a, 64), a):
        ins.append(i)
        if i.mnemonic == 'ret':
            break
    ops = [(i.mnemonic, i.operands[0].mem.disp if i.operands else None) for i in ins]
    ok_mem = all(i.operands[0].mem.base == 0 and i.operands[0].mem.index == 0 and i.operands[0].size == 4
                 for i in ins[:-1])
    assert ok_mem, hex(a)
    assert ops[0][0] == 'fld' and ops[-1][0] == 'ret' and ops[-2][0] == 'fstp', (hex(a), ops)
    mid = ops[1:-2]
    dst = ops[-2][1] & 0xFFFFFFFF
    name = names['0x%x' % a]
    assert name == r['proposed_name'].split(' ')[0], (name, r['proposed_name'])

    def operand(m):
        m &= 0xFFFFFFFF
        if m >= 0xB00000:  # writable .data/.bss: read at run time (set by an earlier initialiser)
            return f'g::<f32>(0x{m:X}).read()', f'[0x{m:X}]'
        b = f32bits(m)
        v = struct.unpack('<f', struct.pack('<I', b))[0]
        for prec in range(1, 10):
            t = '%.*g' % (prec, v)
            if struct.unpack('<f', struct.pack('<f', float(t)))[0] == v:
                break
        if '.' not in t and 'e' not in t:
            t += '.0'
        return f'f32::from_bits(0x{b:08X})', repr(float(t))

    s0, d0 = operand(ops[0][1])
    expr, desc = f'x({s0})', d0
    for mn, m in mid:
        s, d = operand(m)
        if mn == 'fmul':
            expr += f'.fmul_m({s})'
            desc = f'{desc} * {d}'
        elif mn == 'fdiv':
            expr += f'.fdiv_m({s})'
            desc = f'{desc} / {d}'
        elif mn == 'fdivr':
            expr += f'.fdivr_m({s})'
            desc = f'{d} / ({desc})'
        else:
            raise SystemExit(f'{a:#x}: unexpected {mn}')
    exp = ' '.join('%02X' % b for b in rd(a, ins[0].size))
    fn = name.lstrip('_')
    out.append(f'/// `{name}` (0x{a:X}): `[0x{dst:X}] = {desc}`.\n'
               f'#[hook(addr = 0x{a:X}, conv = cdecl, name = "{name}", expect = "{exp}")]\n'
               f'pub fn {fn}() {{\n    g::<f32>(0x{dst:X}).write({expr}.to_f32());\n}}\n')
    regs.append('HOOK_' + fn.upper())

hdr = '''//! HUD static initialisers, second pass (PORT-HUD-2): the header-constant `__xc` initialisers of the HUD
//! translation units (RE-SPEC-UNK, `docs/re/specs/HUD/_static-init-unk.md`), `__init_TheText`, the two
//! initialisers that only register an empty destructor, and the `atexit` destructors of the HUD globals.
//! Header constants are generated from the 1.0 US bytes by `tools/gen/hud_statics.py` (each body is
//! exactly `fld; fmul|fdiv [; fdivr]; fstp; ret`); x87 order, stored as f32. Operands in `.rdata` are
//! their exe bits; operands in writable data (set by an earlier initialiser) are read at run time.

use sa_core::abi::HookReg;
use sa_core::mem::{GPtr, GlobalDesc};
use sa_core::{hook, orig};

use crate::common::{g, x};

/// No globals of its own.
pub const GLOBALS: &[GlobalDesc] = &[];

// ---- header constants (generated) ----

'''
open(os.path.join(ROOT, 'target/scratch/statics2_gen.rs'), 'w').write(hdr + '\n'.join(out))
open(os.path.join(ROOT, 'target/scratch/statics2_regs.txt'), 'w').write('\n'.join(regs) + '\n')
print(len(out))
