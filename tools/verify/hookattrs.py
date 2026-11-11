"""Reads the `#[hook(...)]` registrations of the engine crates from their sources (no build needed).

Shared by tools/e2e/hook-names.py, tools/verify/queue.py and tools/verify/writer-gate.sh. A hook is a
dict: addr (int), name, fn, file (path relative to the root), line, and the attribute's other keys as
strings (`unverified` = batch id, `route`, `hazards`, `recs`, `note`, `conv`, `fallback`, ...).
`SA_UNVERIFIED` selection (the same grammar sa-hook's `table::parse_unverified` reads) is in
`unverified_selection`.
"""
import os
import pathlib
import re

SKIP_CRATES = ("sa-macros", "sa-hook")


def _attr_body(text, start):
    """The text between `#[hook(` at `start` and its closing `)`, honouring string literals."""
    i = start
    depth = 1
    in_str = False
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return text[start:i], i
        i += 1
    return None, len(text)


_KV = re.compile(r'\s*([A-Za-z_][A-Za-z0-9_]*)\s*(?:=\s*("(?:[^"\\]|\\.)*"|[^,]+))?\s*(?:,|$)', re.S)


def parse_attr(body):
    out = {}
    pos = 0
    body = body.strip()
    while pos < len(body):
        m = _KV.match(body, pos)
        if not m or m.end() == pos:
            break
        k, v = m.group(1), m.group(2)
        if v is None:
            v = "true"
        else:
            v = v.strip()
            if v.startswith('"'):
                v = re.sub(r"\\(.)", r"\1", v[1:-1])
        out[k] = v
        pos = m.end()
    return out


def parse_source(text, rel):
    hooks = []
    # attributes only (a line starting with `#[hook(`), never `#[hook(` quoted in a doc comment
    for m in re.finditer(r"^[ \t]*#\[hook\(", text, re.M):
        body, end = _attr_body(text, m.end())
        if body is None:
            continue
        a = parse_attr(body)
        if "addr" not in a or "name" not in a:
            continue
        f = re.search(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", text[end:end + 2000])
        h = dict(a)
        h["addr"] = int(a["addr"].replace("_", ""), 0)
        h["fn"] = f.group(1) if f else ""
        h["file"] = rel
        h["line"] = text.count("\n", 0, m.start()) + 1
        hooks.append(h)
    return hooks


def source_files(root):
    root = pathlib.Path(root)
    for src in sorted(root.glob("crates/*/src/**/*.rs")):
        rel = src.relative_to(root)
        if rel.parts[1] in SKIP_CRATES:
            continue
        yield src, str(rel)


def hooks(root):
    """Every `#[hook]` registration under ROOT/crates/*/src (sa-macros and sa-hook excluded)."""
    out = []
    for src, rel in source_files(root):
        text = src.read_text()
        if "#[hook(" in text:
            out.extend(parse_source(text, rel))
    return out


def list_items(value):
    """Entries of a list value: comma/newline separated, `#` comments, `@FILE` = that file's contents."""
    value = value.strip()
    if value.startswith("@"):
        value = pathlib.Path(value[1:].strip()).read_text()
    for line in value.splitlines():
        line = line.split("#", 1)[0]
        for k in line.split(","):
            k = k.strip()
            if k:
                yield k


def unverified_selection(hs, value=None):
    """Addresses of the unverified hooks `SA_UNVERIFIED` (or VALUE) selects: `all`, batch ids, hook
    names or 0x addresses, `@FILE`. Raises ValueError on an unknown entry."""
    if value is None:
        value = os.environ.get("SA_UNVERIFIED", "")
    sel = set()
    unv = [h for h in hs if h.get("unverified")]
    batches = {h["unverified"] for h in unv}
    for k in list_items(value) if value.strip() else []:
        if k == "all":
            sel.update(h["addr"] for h in unv)
        elif k in batches:
            sel.update(h["addr"] for h in unv if h["unverified"] == k)
        else:
            hit = [h for h in hs if h["name"] == k or (k.lower().startswith("0x") and h["addr"] == int(k, 16))]
            if not hit:
                raise ValueError(f"SA_UNVERIFIED: {k!r} is neither `all`, a batch id nor a hook")
            sel.update(h["addr"] for h in hit if h.get("unverified"))
    return sel


def data_root(root):
    """The checkout holding the gitignored data (catalog/raw, target/e2e, wine): SA_ROOT, else the
    main checkout of a git worktree, else ROOT."""
    if os.environ.get("SA_ROOT"):
        return pathlib.Path(os.environ["SA_ROOT"])
    root = pathlib.Path(root).resolve()
    git = root / ".git"
    if git.is_file():
        m = re.match(r"gitdir:\s*(.+)", git.read_text().strip())
        if m:
            gd = pathlib.Path(m.group(1))
            if not gd.is_absolute():
                gd = (root / gd).resolve()
            # <main>/.git/worktrees/<name>
            if gd.parent.name == "worktrees":
                return gd.parent.parent.parent
    return root
