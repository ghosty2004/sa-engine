"""Verifier helper: disassemble gta_sa.exe 1.0 US. Usage: sadis.py START [END] (hex)."""
import sys, pefile, capstone, csv
import os
EXE = os.environ.get('SA_GTA_EXE', os.path.expanduser('~/sa-engine/re/gta/1.0-US/gta_sa.exe'))
pe = pefile.PE(EXE, fast_load=True)
img = pe.get_memory_mapped_image()
BASE = pe.OPTIONAL_HEADER.ImageBase
md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
md.detail = True

def rd(a, n):
    return img[a - BASE:a - BASE + n]

def dis(a, n=200, stop_ret=True):
    out = []
    for i in md.disasm(rd(a, n * 8), a):
        out.append(i)
        if len(out) >= n:
            break
    return out

if __name__ == '__main__':
    a = int(sys.argv[1], 16)
    end = int(sys.argv[2], 16) if len(sys.argv) > 2 else None
    for i in dis(a, 400):
        if end and i.address >= end:
            break
        print(f"{i.address:08x}: {i.bytes.hex():<20} {i.mnemonic} {i.op_str}")
        if not end and i.mnemonic in ('ret', 'int3'):
            pass
