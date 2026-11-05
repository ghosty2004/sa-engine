#!/usr/bin/env python3
"""CAT-10: script command table.

Part 1 (gta-reversed): for every opcode in eScriptCommands.h, its name and gta-reversed status
(handler / unimplemented / nop / unsupported / unregistered) with the source location.

Part 2 (the original binary): CRunningScript::ProcessOneCommand (0x469EB0) calls
`s_OriginalCommandHandlerTable[opcode / 100](opcode)`; the table is read from the
`call [edx*4 + table]` in its code (0x8A6168, 27 ProcessCommandsXToY member functions, ended by
a non-code word). Each range function is decoded up to its switch: the opcode argument load
(`[esp + k]`, checked to be the first stack argument with esp tracking), the bias (`add`/`lea`),
the bound (`cmp r, n; ja default`), an optional byte index table (`movzx r, byte [r + idx]`) and
the jump table (`jmp [r*4 + jt]`). Per opcode this gives:
  orig_handler    case address in the original (empty when not dispatched to a case)
  orig_range_fn   the ProcessCommandsXToY function that owns the opcode's range
  orig_dispatch   case | default (switch default: the PC exe has no handler for it) |
                  no-range-function (opcode / 100 beyond the table)
  orig_case_shared  number of opcodes whose case address is the same
Usage: script_commands.py <gta-reversed> <catalog-dir> [gta_sa.exe]
"""
import csv
import re
import struct
import sys
from collections import Counter
from pathlib import Path

import capstone
import pefile
from capstone import x86_const as X

ROOT = Path(__file__).resolve().parents[2]
BASE = 0x400000
PROCESS_ONE_COMMAND = 0x469EB0
CODE = (0x401000, 0x858000)


class Exe:
    def __init__(self, path):
        self.img = pefile.PE(str(path), fast_load=True).get_memory_mapped_image()
        self.md = capstone.Cs(capstone.CS_ARCH_X86, capstone.CS_MODE_32)
        self.md.detail = True

    def u32(self, va):
        return struct.unpack_from('<I', self.img, va - BASE)[0]

    def u8(self, va):
        return self.img[va - BASE]

    def insns(self, va, n=0x400):
        return self.md.disasm(bytes(self.img[va - BASE:va - BASE + n]), va)


def handler_table(exe):
    """Address of the range-function table from ProcessOneCommand's `call reg` dispatch."""
    tbl = None
    for ins in exe.insns(PROCESS_ONE_COMMAND, 0x60):
        if ins.id == X.X86_INS_MOV and ins.operands[1].type == X.X86_OP_MEM:
            m = ins.operands[1].mem
            if m.base == 0 and m.index and m.scale == 4:
                tbl = m.disp & 0xFFFFFFFF
        if ins.id == X.X86_INS_CALL:
            break
    assert tbl, 'no table load in ProcessOneCommand'
    fns = []
    while CODE[0] <= exe.u32(tbl + 4 * len(fns)) < CODE[1]:
        fns.append(exe.u32(tbl + 4 * len(fns)))
    return tbl, fns


def decode_switch(exe, fn):
    """(bias, bound, byte_table or None, jump_table, default) of the switch at the top of fn."""
    esp = 0                      # esp relative to entry (entry esp + 4 = first stack argument)
    val = {}                     # reg -> offset relative to the opcode argument
    bound = default = btab = None
    for ins in exe.insns(fn):
        mn, ops = ins.mnemonic, ins.operands
        if mn == 'push':
            esp -= 4
            continue
        if mn == 'pop':
            esp += 4
            continue
        if mn in ('sub', 'add') and ops[0].type == X.X86_OP_REG and ops[0].reg == X.X86_REG_ESP:
            esp += (-1 if mn == 'sub' else 1) * ops[1].imm
            continue
        if mn == 'mov' and ops[0].type == X.X86_OP_REG and ops[1].type == X.X86_OP_MEM \
                and ops[1].mem.base == X.X86_REG_ESP and ops[1].mem.index == 0 and ops[1].mem.segment == 0:
            if esp + ops[1].mem.disp == 4:
                val[ops[0].reg] = 0
            else:
                val.pop(ops[0].reg, None)
            continue
        if mn == 'add' and ops[0].type == X.X86_OP_REG and ops[0].reg in val and ops[1].type == X.X86_OP_IMM:
            val[ops[0].reg] += struct.unpack('<i', struct.pack('<I', ops[1].imm & 0xFFFFFFFF))[0]
            continue
        if mn == 'lea' and ops[1].mem.base in val and ops[1].mem.index == 0:
            val[ops[0].reg] = val[ops[1].mem.base] + ops[1].mem.disp
            continue
        if mn == 'cmp' and ops[0].type == X.X86_OP_REG and ops[0].reg in val and ops[1].type == X.X86_OP_IMM:
            bound = (ops[0].reg, ops[1].imm)
            continue
        if mn == 'ja' and bound:
            default = ops[0].imm
            continue
        if mn == 'movzx' and ops[1].type == X.X86_OP_MEM and ops[1].size == 1 and ops[1].mem.base in val \
                and default:
            btab = ops[1].mem.disp
            assert ops[1].mem.base == bound[0], hex(ins.address)
            val[ops[0].reg] = ('byte', ops[1].mem.base, val[ops[1].mem.base])
            continue
        if mn == 'jmp' and ops[0].type == X.X86_OP_MEM:
            r = ops[0].mem.index
            assert ops[0].mem.scale == 4 and default and bound, hex(ins.address)
            v = val[r]
            if isinstance(v, tuple):
                src, off = v[1], v[2]
            else:
                src, off = r, v
            assert src == bound[0] and (btab is None) == (not isinstance(v, tuple)), hex(ins.address)
            return -off, bound[1], btab, ops[0].mem.disp, default
        if ops and ops[0].type == X.X86_OP_REG and ops[0].reg in val and mn not in ('cmp', 'test'):
            val.pop(ops[0].reg)  # any other write kills the tracked value
        if mn in ('ret', 'call') or (mn == 'jmp'):
            break
    raise AssertionError(f'no switch decoded in {fn:#x}')


def original_handlers(exe_path):
    """{opcode: (case address or None, range fn, dispatch)} and the table description."""
    exe = Exe(exe_path)
    tbl, fns = handler_table(exe)
    out, info = {}, []
    for k, fn in enumerate(fns):
        bias, bound, btab, jt, default = decode_switch(exe, fn)
        info.append((k, fn, bias, bound, btab, jt, default))
        for op in range(100 * k, 100 * k + 100):
            i = op - bias
            if 0 <= i <= bound:
                case = exe.u32(jt + 4 * (exe.u8(btab + i) if btab else i))
                out[op] = (case, fn, 'default' if case == default else 'case')
            else:
                out[op] = (None, fn, 'default')
    return tbl, fns, out, info

ENUM = re.compile(r'^\s*(COMMAND_\w+)\s*=\s*(0x[0-9A-Fa-f]+)\s*,')
REG = re.compile(r'\b(REGISTER_COMMAND_HANDLER|REGISTER_COMMAND_UNIMPLEMENTED|REGISTER_COMMAND_NOP|'
                 r'REGISTER_UNSUPPORTED_COMMAND_HANDLER)\s*\(\s*(COMMAND_\w+)')
STATUS = {'REGISTER_COMMAND_HANDLER': 'handler', 'REGISTER_COMMAND_UNIMPLEMENTED': 'unimplemented',
          'REGISTER_COMMAND_NOP': 'nop', 'REGISTER_UNSUPPORTED_COMMAND_HANDLER': 'unsupported'}


GR_NOTE = re.compile(r'//\s*(?:COMMAND_)?([A-Z][A-Z0-9_]+)\s*(?:\((\d+)\))?\s*-\s*(0x[0-9A-Fa-f]{6})\b')


def gr_case_notes(root):
    """gta-reversed's own annotations `// NAME (opcode) - 0xADDR` above command handlers: an
    address inside the original's case block for that command."""
    out = []
    for p in sorted((root / 'source/game_sa/Scripts/Commands').rglob('*.cpp')):
        for ln, line in enumerate(p.read_text(errors='replace').splitlines(), 1):
            for m in GR_NOTE.finditer(line):
                out.append(('COMMAND_' + m.group(1), int(m.group(3), 16), f'{p.relative_to(root)}:{ln}'))
    return out


def verify_notes(ops, orig, notes):
    """Each annotated address must lie in [case, next label of the same switch)."""
    labels = {}
    for case, fn, disp in orig.values():
        if case:
            labels.setdefault(fn, set()).add(case)
    res = []
    for name, addr, src in notes:
        if name not in ops:
            res.append((name, addr, src, 'name not in enum'))
            continue
        case, fn, disp = orig.get(ops[name], (None, None, 'no-range-function'))
        if disp != 'case':
            res.append((name, addr, src, f'opcode dispatches to {disp}'))
            continue
        later = sorted(x for x in labels[fn] if x > case)
        end = later[0] if later else case + 0x1000
        res.append((name, addr, src, 'ok' if case <= addr < end else f'outside case block {case:#x}-{end:#x}'))
    return res


def main():
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
    exe = sys.argv[3] if len(sys.argv) > 3 else ROOT / 're' / 'gta' / '1.0-US' / 'gta_sa.exe'
    tbl, fns, orig, info = original_handlers(exe)
    shared = Counter(c for c, _f, d in orig.values() if d == 'case')
    ops = {}
    for line in (root / 'source/game_sa/Enums/eScriptCommands.h').read_text().splitlines():
        if (m := ENUM.match(line)):
            ops[m.group(1)] = int(m.group(2), 16)
    reg = {}
    for p in sorted((root / 'source/game_sa/Scripts').rglob('*.cpp')):
        for ln, line in enumerate(p.read_text(errors='replace').splitlines(), 1):
            for m in REG.finditer(line):
                reg.setdefault(m.group(2), (STATUS[m.group(1)], f'{p.relative_to(root)}:{ln}'))
    with open(out / 'script-commands.csv', 'w', newline='') as f:
        w = csv.writer(f, lineterminator='\n')
        w.writerow(['opcode', 'name', 'gr_status', 'gr_source', 'orig_handler', 'orig_range_fn', 'orig_dispatch',
                    'orig_case_shared'])
        for name, op in sorted(ops.items(), key=lambda kv: kv[1]):
            st, src = reg.get(name, ('unregistered', ''))
            case, fn, disp = orig.get(op, (None, None, 'no-range-function'))
            w.writerow([f'{op:04X}', name, st, src, f'{case:#x}' if case and disp == 'case' else '',
                        f'{fn:#x}' if fn else '', disp, shared[case] if disp == 'case' else ''])
    c = Counter(reg.get(n, ('unregistered',))[0] for n in ops)
    print(len(ops), dict(c), 'registered-but-not-in-enum:', len(set(reg) - set(ops)))
    print(f'handler table {tbl:#x}: {len(fns)} range functions')
    for k, fn, bias, bound, btab, jt, default in info:
        print(f'  {100 * k:4d}-{100 * k + 99:4d} {fn:#x}: cases {bias}-{bias + bound}, '
              f'jt {jt:#x}' + (f', byte index {btab:#x}' if btab else '') + f', default {default:#x}')
    d = Counter((reg.get(n, ('unregistered',))[0], orig.get(op, (0, 0, 'no-range-function'))[2])
                for n, op in ops.items())
    print('gta-reversed status x original dispatch:', dict(sorted(d.items())))
    res = verify_notes(ops, orig, gr_case_notes(root))
    print(f'gta-reversed case annotations: {sum(r[3] == "ok" for r in res)}/{len(res)} inside the decoded case block')
    for name, addr, src, why in res:
        if why != 'ok':
            print(f'  {name} {addr:#x} ({src}): {why}')


if __name__ == '__main__':
    main()
