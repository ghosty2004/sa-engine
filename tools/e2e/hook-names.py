#!/usr/bin/env python3
"""Lists sa-hook's hook table as `ADDR<TAB>GROUP<TAB>NAME` lines (for tools/e2e/port-l2.sh).

  hook-names.py [ROOT] [ports|all|LIST]

Sources: the local table in crates/sa-hook/src/table.rs (`addr:` / `name:` pairs, group `local`) and
every `#[hook(...)]` registration in crates/*/src (group `port`; sa-hook installs these with
SA_HOOK_PORTS). With `ports` only the port hooks are printed, with `all` every hook; LIST is a comma
list of names, addresses, `@FILE` (one name/address per line, `#` comments) or `state:STATE[:DOC]` (the
addresses of the docs/arch/*.md (or docs/arch/DOC.md) function-table rows with that state, e.g.
`state:implemented-needs-L2:core-port` = PORT-CORE's L2 batch) or `doc:DOC` (every row of docs/arch/DOC.md
that is a hook), resolved against the table; an unknown entry is
an error.
"""
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
sel = sys.argv[2] if len(sys.argv) > 2 else "ports"

hooks = []  # (addr, group, name)
table = (root / "crates/sa-hook/src/table.rs").read_text()
for m in re.finditer(r'addr:\s*(0x[0-9A-Fa-f]+),\s*name:\s*"([^"]+)"', table):
    hooks.append((int(m.group(1), 16), "local", m.group(2)))
for src in sorted(root.glob("crates/*/src/**/*.rs")):
    if src.parts[-3] in ("sa-macros", "sa-hook"):
        continue
    text = src.read_text()
    for m in re.finditer(r"^\s*#\[hook\((.*?)\)\]", text, re.S | re.M):
        body = m.group(1)
        a = re.search(r"addr\s*=\s*(0x[0-9A-Fa-f]+)", body)
        n = re.search(r'name\s*=\s*"([^"]+)"', body)
        if a and n:
            hooks.append((int(a.group(1), 16), "port", n.group(1)))

def keys_of(sel):
    """Comma list; `@FILE` = one name/address per line (`#` comments); `state:STATE` = the addresses of
    the rows of the docs/arch/*.md function tables whose state column is STATE."""
    for k in (x.strip() for x in sel.split(",")):
        if not k:
            continue
        if k.startswith("@"):
            for line in pathlib.Path(k[1:]).read_text().splitlines():
                line = line.split("#", 1)[0].strip()
                if line:
                    yield line
        elif k.startswith("doc:"):
            # every function-table row of docs/arch/DOC.md that is a hook (rows without a hook skipped)
            for line in (root / f"docs/arch/{k[len('doc:'):]}.md").read_text().splitlines():
                cells = [c.strip() for c in line.strip().strip("|").split("|")]
                if len(cells) > 2 and re.fullmatch(r"0x[0-9A-Fa-f]+", cells[0]):
                    if any(h[0] == int(cells[0], 16) for h in hooks):
                        yield cells[0]
        elif k.startswith("state:"):
            want, _, doc = k[len("state:"):].partition(":")
            for md in sorted((root / "docs/arch").glob(f"{doc or '*'}.md")):
                for line in md.read_text().splitlines():
                    cells = [c.strip() for c in line.strip().strip("|").split("|")]
                    if len(cells) > 2 and re.fullmatch(r"0x[0-9A-Fa-f]+", cells[0]) and any(
                        c.split(" ", 1)[0].split("(", 1)[0].rstrip(":") == want for c in cells[1:] if c
                    ):
                        yield cells[0]
        else:
            yield k


if sel == "all":
    out = hooks
elif sel == "ports":
    out = [h for h in hooks if h[1] == "port"]
else:
    out = []
    for k in keys_of(sel):
        hit = [h for h in hooks if h[2] == k or (k.lower().startswith("0x") and h[0] == int(k, 16))]
        if not hit:
            sys.exit(f"hook-names: unknown hook {k!r}")
        if hit[0] not in out:
            out.append(hit[0])
for a, g, n in out:
    print(f"0x{a:X}\t{g}\t{n}")
