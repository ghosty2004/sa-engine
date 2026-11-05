#!/usr/bin/env python3
"""CAT-10 (part 1): script command table from gta-reversed.

For every opcode in eScriptCommands.h: name and gta-reversed status
(handler / unimplemented / nop / unsupported / unregistered) with the source
location. The original's handler address per opcode (ProcessCommands* switch
ranges) is added in part 2 from the binary.
Usage: script_commands.py <gta-reversed> <catalog-dir>
"""
import csv
import re
import sys
from pathlib import Path

ENUM = re.compile(r'^\s*(COMMAND_\w+)\s*=\s*(0x[0-9A-Fa-f]+)\s*,')
REG = re.compile(r'\b(REGISTER_COMMAND_HANDLER|REGISTER_COMMAND_UNIMPLEMENTED|REGISTER_COMMAND_NOP|'
                 r'REGISTER_UNSUPPORTED_COMMAND_HANDLER)\s*\(\s*(COMMAND_\w+)')
STATUS = {'REGISTER_COMMAND_HANDLER': 'handler', 'REGISTER_COMMAND_UNIMPLEMENTED': 'unimplemented',
          'REGISTER_COMMAND_NOP': 'nop', 'REGISTER_UNSUPPORTED_COMMAND_HANDLER': 'unsupported'}


def main():
    root, out = Path(sys.argv[1]), Path(sys.argv[2])
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
        w = csv.writer(f)
        w.writerow(['opcode', 'name', 'gr_status', 'gr_source', 'orig_handler'])
        for name, op in sorted(ops.items(), key=lambda kv: kv[1]):
            st, src = reg.get(name, ('unregistered', ''))
            w.writerow([f'{op:04X}', name, st, src, ''])
    from collections import Counter
    c = Counter(reg.get(n, ('unregistered',))[0] for n in ops)
    print(len(ops), dict(c), 'registered-but-not-in-enum:', len(set(reg) - set(ops)))


if __name__ == '__main__':
    main()
