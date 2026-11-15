#!/usr/bin/env python3
"""Evaluate a no-CD protection VM call site (0xCB73E0) of gta_sa.exe 1.0 US on the exe image with unicorn.

Usage: .venv/bin/python -I tools/re/vm_eval.py START END [--reg R=V ...] [--stack N] [--call] [--trace]

The no-CD build hides some constants and call targets behind the protection's VM: a call site pushes a few
constants, `push esp`, a descriptor (0x155Fxxx) and calls 0xCB73E0, then folds the VM's results into a value that it
leaves on the stack (or in a register) before it continues. The image is static (no relocations, the protection's
tables are initialised data), so emulating the site from the image gives the value the original computes at run
time, unless the site also reads run-time state.

The tool maps every section of the exe at its virtual address (the module's own image, never copied into the repo:
`tools/lib/exe.py` finds it; `SA_GTA_EXE` overrides), a 1 MiB stack and a zeroed TEB for `fs:`, sets the registers
(`--reg eax=0x...`, the others 0; ESP points into the stack), and runs from START until EIP reaches END.

At END it prints the registers, the top `--stack` dwords (default 8) and every read of memory outside the stack
whose address lies in the image (address, size, value: these are the run-time globals the site depends on, e.g. a
pointer decoded through `~[0x1560530] ^ [0x72D9F4]`). With `--call`, emulation continues past END until EIP leaves
the image's protection sections or reaches a catalogued function, and prints that target (the computed call of the
`call $+5; ...; xchg [esp], edi; ret` idiom) with its stack arguments.

Instructions the emulator cannot run (the protection's `pop ss` / single-step probes, privileged instructions) are
skipped one at a time and reported. `--trace` prints every executed address.

unicorn: from the project `.venv` (`.venv/bin/python -m pip install unicorn`); without it in the venv, install it
into a scratch directory and point `SA_UNICORN_PATH` at it (`.venv/bin/python -m pip install --target
target/pyunicorn unicorn`, the default path the tool also looks in).
"""
import argparse
import os
import struct
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, '..', '..'))
sys.path.insert(0, os.path.join(ROOT, 'tools', 'lib'))


def import_unicorn():
    try:
        import unicorn  # noqa: F401
        return
    except ImportError:
        pass
    for p in (os.environ.get('SA_UNICORN_PATH'), os.path.join(ROOT, 'target', 'pyunicorn')):
        if p and os.path.isdir(p):
            sys.path.insert(0, p)
            try:
                import unicorn  # noqa: F401
                return
            except ImportError:
                sys.path.pop(0)
    sys.exit('vm_eval: unicorn not found; see the module docs (pip install --target target/pyunicorn unicorn)')


import_unicorn()
import unicorn as uc  # noqa: E402
from unicorn import x86_const as R  # noqa: E402

import exe  # noqa: E402

STACK_TOP = 0x0FF00000
STACK_SIZE = 0x00100000
TEB = 0x7FFD0000
REGS = {'eax': R.UC_X86_REG_EAX, 'ebx': R.UC_X86_REG_EBX, 'ecx': R.UC_X86_REG_ECX, 'edx': R.UC_X86_REG_EDX,
        'esi': R.UC_X86_REG_ESI, 'edi': R.UC_X86_REG_EDI, 'ebp': R.UC_X86_REG_EBP, 'esp': R.UC_X86_REG_ESP}


def page(a):
    return a & ~0xFFF


def map_image(mu):
    pe = exe.pe
    base = exe.BASE
    end = base + pe.OPTIONAL_HEADER.SizeOfImage
    mu.mem_map(base, (end - base + 0xFFF) & ~0xFFF)
    mu.mem_write(base, bytes(exe.IMG))
    return base, end


def setup(regs):
    mu = uc.Uc(uc.UC_ARCH_X86, uc.UC_MODE_32)
    base, end = map_image(mu)
    mu.mem_map(STACK_TOP - STACK_SIZE, STACK_SIZE)
    mu.mem_map(TEB, 0x2000)
    mu.mem_write(TEB + 0x18, struct.pack('<I', TEB))
    # fs: through a GDT entry
    gdt = 0x7FFC0000
    mu.mem_map(gdt, 0x1000)

    def desc(b, lim, access, flags):
        return struct.pack('<Q', (lim & 0xFFFF) | ((b & 0xFFFFFF) << 16) | (access << 40) | (((lim >> 16) & 0xF) << 48)
                           | (flags << 52) | (((b >> 24) & 0xFF) << 56))
    mu.mem_write(gdt + 8 * 1, desc(0, 0xFFFFF, 0x9B, 0xC))   # code
    mu.mem_write(gdt + 8 * 2, desc(0, 0xFFFFF, 0x93, 0xC))   # data
    mu.mem_write(gdt + 8 * 3, desc(TEB, 0xFFF, 0x93, 0x4))   # fs
    mu.reg_write(R.UC_X86_REG_GDTR, (0, gdt, 0x1000 - 1, 0))
    mu.reg_write(R.UC_X86_REG_SS, 2 << 3)
    mu.reg_write(R.UC_X86_REG_DS, 2 << 3)
    mu.reg_write(R.UC_X86_REG_ES, 2 << 3)
    mu.reg_write(R.UC_X86_REG_FS, 3 << 3)
    for k, v in regs.items():
        mu.reg_write(REGS[k], v)
    if 'esp' not in regs:
        mu.reg_write(R.UC_X86_REG_ESP, STACK_TOP - 0x10000)
    return mu, base, end


def run(start, end, regs, nstack, call, trace):
    mu, base, img_end = setup(regs)
    reads = []
    skipped = []

    def on_read(mu_, access, addr, size, value, data):
        if base <= addr < img_end:
            v = int.from_bytes(mu_.mem_read(addr, size), 'little')
            reads.append((mu_.reg_read(R.UC_X86_REG_EIP), addr, size, v))

    def on_code(mu_, addr, size, data):
        if trace:
            print(f'  {addr:08x}')
    mu.hook_add(uc.UC_HOOK_MEM_READ, on_read)
    if trace:
        mu.hook_add(uc.UC_HOOK_CODE, on_code)
    pc = start
    while True:
        try:
            mu.emu_start(pc, end, count=2_000_000)
            break
        except uc.UcError as e:
            pc = mu.reg_read(R.UC_X86_REG_EIP)
            i = exe.insn_at(pc)
            if i is None or len(skipped) > 64:
                sys.exit(f'vm_eval: stopped at {pc:08x}: {e}')
            skipped.append((pc, f'{i.mnemonic} {i.op_str}', str(e)))
            pc += i.size
    esp = mu.reg_read(R.UC_X86_REG_ESP)
    print(f'at {end:08x}:')
    print('  ' + ' '.join(f'{k}={mu.reg_read(v):08x}' for k, v in REGS.items()))
    for k in range(nstack):
        v = struct.unpack('<I', mu.mem_read(esp + 4 * k, 4))[0]
        print(f'  [esp+{4 * k:02x}] = {v:08x}')
    if skipped:
        print('skipped instructions:')
        for a, t, e in skipped:
            print(f'  {a:08x}: {t} ({e})')
    print('image reads outside the stack (eip, address, size, value):')
    seen = set()
    for eip, a, s, v in reads:
        if (a, s) in seen:
            continue
        seen.add((a, s))
        name = exe.catname(a) if hasattr(exe, 'catname') else ''
        print(f'  {eip:08x}: [{a:08x}]:{s} = {v:0{2 * s}x} {name or ""}')
    if call:
        stop = {'pc': None}

        def on_block(mu_, addr, size, data):
            if not (0xCB1000 <= addr < 0xCC0000 or 0x1556000 <= addr < 0x1600000) or exe.catname(addr):
                stop['pc'] = addr
                mu_.emu_stop()
        mu.hook_add(uc.UC_HOOK_BLOCK, on_block)
        try:
            mu.emu_start(end, 0xFFFFFFFF, count=100_000)
        except uc.UcError as e:
            print(f'call: emulation error {e}')
        if stop['pc'] is not None:
            esp = mu.reg_read(R.UC_X86_REG_ESP)
            st = [struct.unpack('<I', mu.mem_read(esp + 4 * k, 4))[0] for k in range(4)]
            print(f'call target {stop["pc"]:08x} {exe.catname(stop["pc"]) or ""}: ecx={mu.reg_read(R.UC_X86_REG_ECX):08x}'
                  f' return={st[0]:08x} args={" ".join(f"{v:08x}" for v in st[1:])}')


def main():
    ap = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    ap.add_argument('start', type=lambda s: int(s, 16))
    ap.add_argument('end', type=lambda s: int(s, 16))
    ap.add_argument('--reg', action='append', default=[], help='R=V (hex), e.g. eax=0x9691C0')
    ap.add_argument('--stack', type=int, default=8)
    ap.add_argument('--call', action='store_true')
    ap.add_argument('--trace', action='store_true')
    a = ap.parse_args()
    regs = {}
    for r in a.reg:
        k, v = r.split('=')
        regs[k.lower()] = int(v, 0)
    run(a.start, a.end, regs, a.stack, a.call, a.trace)


if __name__ == '__main__':
    main()
