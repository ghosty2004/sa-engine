"""The functions that stay original by design (ADR-003 §3.2: one list, read by every tool that needs it).

`catalog/stays-original.csv` (hand-maintained, columns `key,reason`) names them: `key` is either an address
(`0x4F11E0`) or `class:NAME` (every function whose catalogue name is `NAME::...`; the class's registered ports
are unaffected, only its unported functions count as staying original). Readers:

- `tools/verify/frontier.py` leaves them out of the to-port lists and reports their count in its summary line
  (`--stays-original` lists them);
- `tools/gen/audio_patterns.py` does not generate ports for them.

Plain Python (no capstone), so tools run with the system python3 can import it.
"""
import csv
import os

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), '..', '..'))
PATH = os.path.join(ROOT, 'catalog', 'stays-original.csv')


def load(path=PATH):
    """(addresses: {addr: reason}, classes: {class: reason})."""
    addrs, classes = {}, {}
    if not os.path.exists(path):
        return addrs, classes
    for r in csv.DictReader(open(path, encoding='utf-8')):
        key = r['key'].strip()
        if key.startswith('class:'):
            classes[key[len('class:'):]] = r['reason']
        else:
            addrs[int(key, 16)] = r['reason']
    return addrs, classes


def class_of(name):
    return name.split('::')[0] if name and '::' in name else ''


def reason(addr, name, table=None):
    """Why `addr` (catalogue name `name`) stays original, or None."""
    addrs, classes = table if table is not None else load()
    if addr in addrs:
        return addrs[addr]
    return classes.get(class_of(name))
