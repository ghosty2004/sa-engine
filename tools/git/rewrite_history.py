#!/usr/bin/env python3
"""Rewrite the commit messages of master's history to Conventional Commits (ROADMAP GIT-1).

Only messages change. Every commit object is rebuilt from its raw bytes: the tree, author, committer (names,
e-mails, dates, time zones) and every other header stay byte-identical; parents are mapped to their rewritten
commits. The procedure, the decisions and the recovery steps are in docs/history/README.md.

    rewrite_history.py                      the real run, in this checkout (refuses while agents work)
    rewrite_history.py --dry-run DIR        clone this repository into DIR (git clone --no-local) and run there
    rewrite_history.py --draft-missing      append draft rows for commits that have no row in the TSV
    rewrite_history.py --check              only report missing rows and subject problems

New message of a non-merge commit with a TSV row:

    <new subject>
    <blank line>
    <the original message, verbatim>
    <blank line>
    Refs: <ids>                (when the row has any)
    Original-commit: <old sha>

When the row's subject equals the original subject (commits already in the new format), the original message is
kept and only `Original-commit:` is added (to its trailer block when it ends with one). Merge commits keep their
message and get the `Original-commit:` trailer the same way.
"""
import argparse
import datetime
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "..", "verify"))
import commit_lint  # noqa: E402

TSV_DEFAULT = "docs/history/commit-messages.tsv"
HASH_MAP = "docs/history/hash-map.tsv"
ARCHIVE = "refs/archive/pre-rewrite/"
REF_COMMIT_SUBJECT = "docs: update commit references after the history rewrite"
TRAILER_LINE = re.compile(r"^[A-Za-z][A-Za-z0-9-]*: \S")
# a task id as the ROADMAP writes them (FRONTIER-2, DEDUP-1, STD-12, CORE-L2b ...)
TASK_ID = re.compile(r"(?<![A-Za-z0-9_.\-/])([A-Z][A-Z0-9]*(?:-[A-Z0-9]+)+[a-z]?)(?![A-Za-z0-9_\-])")
# a commit id candidate in text: lowercase hex, 7..40 characters, not part of a longer word or hex run,
# not preceded by 0x (addresses such as 0x6d7bc0)
HEX_TOKEN = re.compile(r"(?<![0-9A-Za-z_])(?<!0x)(?<!0X)([0-9a-f]{7,40})(?![0-9A-Za-z_])")
# files never rewritten by the reference update: they must keep the old ids
KEEP_OLD_IDS = ("docs/history/",)


class Git:
    def __init__(self, repo):
        self.repo = repo

    def run(self, *args, input=None, check=True):
        r = subprocess.run(["git", "-C", self.repo, *args], input=input, capture_output=True, check=False)
        if check and r.returncode != 0:
            raise SystemExit(f"git {' '.join(args)} failed:\n{r.stderr.decode(errors='replace')}")
        return r

    def out(self, *args):
        return self.run(*args).stdout.decode()

    def cat_commits(self, shas):
        """Raw commit objects by sha, read through one cat-file --batch process."""
        r = subprocess.run(["git", "-C", self.repo, "cat-file", "--batch"], input="".join(s + "\n" for s in shas).encode(),
                           capture_output=True, check=True)
        data, pos, res = r.stdout, 0, {}
        for s in shas:
            nl = data.index(b"\n", pos)
            sha, typ, size = data[pos:nl].decode().split()
            assert typ == "commit" and sha == s, (sha, typ)
            res[s] = data[nl + 1: nl + 1 + int(size)]
            pos = nl + 1 + int(size) + 1
        return res


def split_commit(raw):
    """(header lines, message bytes) of a raw commit object."""
    head, sep, msg = raw.partition(b"\n\n")
    assert sep, "commit without a message separator"
    return head.split(b"\n"), msg


def last_paragraph_is_trailers(msg):
    paras = [p for p in msg.strip("\n").split("\n\n") if p.strip()]
    if len(paras) < 2:
        return False
    return all(TRAILER_LINE.match(ln) or ln.startswith(" ") for ln in paras[-1].split("\n"))


def append_trailers(msg, trailers):
    """msg with trailer lines added: to its trailer block if it ends with one, else as a new paragraph.
    Nothing of msg is changed or removed; only trailing newlines are normalised to one."""
    body = msg.rstrip("\n")
    sep = "\n" if last_paragraph_is_trailers(msg) else "\n\n"
    return body + sep + "\n".join(trailers) + "\n"


def new_message(old_sha, old_msg, row, is_merge):
    if row is None:
        assert is_merge
        return append_trailers(old_msg, [f"Original-commit: {old_sha}"])
    subject, refs = row
    old_subject = old_msg.split("\n", 1)[0]
    if subject == old_subject:
        trailers = []
        if refs and not re.search(r"^Refs: ", old_msg, re.M):
            trailers.append(f"Refs: {refs.replace(',', ', ')}")
        trailers.append(f"Original-commit: {old_sha}")
        return append_trailers(old_msg, trailers)
    trailers = ([f"Refs: {refs.replace(',', ', ')}"] if refs else []) + [f"Original-commit: {old_sha}"]
    # the original message verbatim; only a missing final newline is added before the trailer paragraph
    body = old_msg if old_msg.endswith("\n") else old_msg + "\n"
    return f"{subject}\n\n{body}\n" + "\n".join(trailers) + "\n"


def load_tsv(path):
    rows = {}
    with open(path, encoding="utf-8") as f:
        for n, ln in enumerate(f, 1):
            ln = ln.rstrip("\n")
            if not ln or ln.startswith("#"):
                continue
            p = ln.split("\t")
            if len(p) not in (2, 3) or not re.fullmatch(r"[0-9a-f]{40}", p[0]):
                raise SystemExit(f"{path}:{n}: expected 'old_sha<TAB>subject[<TAB>refs]'")
            if p[0] in rows:
                raise SystemExit(f"{path}:{n}: second row for {p[0]}")
            rows[p[0]] = (p[1], p[2] if len(p) == 3 else "")
    return rows


def check_rows(g, rows, branch):
    """(missing non-merge shas oldest first, subject problems)."""
    shas = g.out("rev-list", "--no-merges", "--reverse", branch).split()
    missing = [s for s in shas if s not in rows]
    bad = []
    for s in shas:
        if s in rows:
            p = commit_lint.problems(rows[s][0] + "\n")
            if p:
                bad.append((s, rows[s][0], p))
    extra = set(rows) - set(shas)
    return missing, bad, extra


def draft_subject(msg):
    """A mechanical draft subject for review: the original subject when it already passes, else chore: + it."""
    first = msg.split("\n", 1)[0].strip()
    if not commit_lint.problems(first + "\n"):
        return first
    s = first.rstrip(".")
    s = s[0].lower() + s[1:] if s and s[0].isupper() and not re.match(r"^[A-Z][A-Za-z0-9]*[A-Z0-9_:]", s) else s
    s = "chore: " + s
    if len(s) > commit_lint.MAX_SUBJECT:
        s = s[: commit_lint.MAX_SUBJECT].rsplit(" ", 1)[0].rstrip(",;:")
    return s


def draft_missing(g, tsv, branch):
    rows = load_tsv(tsv)
    missing, _, _ = check_rows(g, rows, branch)
    if not missing:
        print("no missing rows")
        return 0
    with open(tsv, "a", encoding="utf-8") as f:
        f.write(f"# draft rows appended {datetime.date.today()} by --draft-missing: review every subject\n")
        for s in missing:
            msg = g.out("show", "-s", "--format=%B", s)
            ids = []
            for t in TASK_ID.findall(msg.split("\n", 1)[0]) + re.findall(r"^Refs:\s*(.+)$", msg, re.M):
                for x in re.split(r"[,\s]+", t):
                    if x and x not in ids:
                        ids.append(x)
            subj = draft_subject(msg)
            f.write(f"{s}\t{subj}\t{','.join(ids)}\n")
            print(f"DRAFT {s[:10]}  {subj}")
    print(f"{len(missing)} draft rows appended to {tsv}; review them, then run again")
    return 0


def agent_worktrees(g):
    """Linked worktrees under .claude/worktrees (the first entry is the main worktree itself)."""
    out = g.out("worktree", "list", "--porcelain")
    paths = [ln[9:] for ln in out.splitlines() if ln.startswith("worktree ")]
    return [p for p in paths[1:] if "/.claude/worktrees/" in p]


def rewrite_objects(g, rows, branch):
    """Write the rewritten commit objects; return {old: new} for every commit reachable from branch."""
    order = g.out("rev-list", "--reverse", "--topo-order", branch).split()
    raws = g.cat_commits(order)
    mapping = {}
    for old in order:
        head, msg = split_commit(raws[old])
        parents = [ln[7:].decode() for ln in head if ln.startswith(b"parent ")]
        new_head = []
        for ln in head:
            if ln.startswith(b"parent "):
                ln = b"parent " + mapping[ln[7:].decode()].encode()
            elif ln.startswith((b"gpgsig", b"mergetag")):
                raise SystemExit(f"{old}: signed commits are not handled")
            new_head.append(ln)
        is_merge = len(parents) > 1
        row = rows.get(old)
        if row is None and not is_merge:
            raise SystemExit(f"{old}: no TSV row")
        text = new_message(old, msg.decode("utf-8"), row, is_merge)
        raw = b"\n".join(new_head) + b"\n\n" + text.encode("utf-8")
        new = g.run("hash-object", "-t", "commit", "-w", "--stdin", input=raw).stdout.decode().strip()
        mapping[old] = new
    return order, mapping


def verify(g, order, mapping, rows):
    """Byte-level checks of every old -> new pair; returns a list of failures."""
    olds = g.cat_commits(order)
    news = g.cat_commits([mapping[o] for o in order])
    fails = []
    for o in order:
        oh, om = split_commit(olds[o])
        nh, nm = split_commit(news[mapping[o]])
        ohp = [ln for ln in oh if not ln.startswith(b"parent ")]
        nhp = [ln for ln in nh if not ln.startswith(b"parent ")]
        if ohp != nhp:
            fails.append(f"{o}: headers differ (tree/author/committer)")
        op = [ln[7:].decode() for ln in oh if ln.startswith(b"parent ")]
        np_ = [ln[7:].decode() for ln in nh if ln.startswith(b"parent ")]
        if [mapping[p] for p in op] != np_:
            fails.append(f"{o}: parents not mapped")
        nm_s, om_s = nm.decode(), om.decode()
        if om_s.rstrip("\n") not in nm_s:
            fails.append(f"{o}: original message not kept verbatim")
        if f"Original-commit: {o}" not in nm_s:
            fails.append(f"{o}: no Original-commit trailer")
        if len(op) <= 1:
            p = commit_lint.problems(nm_s)
            if p:
                fails.append(f"{o}: lint: {'; '.join(p)}")
    return fails


def tracked_text_files(g):
    files = g.out("ls-files", "-z").split("\0")
    out = []
    for f in files:
        if not f or f.startswith(KEEP_OLD_IDS):
            continue
        path = os.path.join(g.repo, f)
        if not os.path.isfile(path) or os.path.islink(path):
            continue
        with open(path, "rb") as fh:
            data = fh.read()
        if b"\0" in data:
            continue
        try:
            data.decode("utf-8")
        except UnicodeDecodeError:
            continue
        out.append(f)
    return out


def update_references(g, mapping):
    """Rewrite cited commit ids in tracked text files.
    Returns (replacements, per-file counts, ambiguous tokens left alone, decimal-only tokens replaced)."""
    old_ids = sorted(mapping)
    all_ids = set(mapping) | set(mapping.values())

    def resolve(tok):
        # binary search over the sorted old ids for every id starting with tok
        import bisect
        i = bisect.bisect_left(old_ids, tok)
        hits = []
        while i < len(old_ids) and old_ids[i].startswith(tok):
            hits.append(old_ids[i])
            i += 1
        return hits

    def unique_prefix(new, n):
        while n < 40 and sum(1 for x in all_ids if x.startswith(new[:n])) > 1:
            n += 1
        return new[:n]

    total, per_file, ambiguous, decimal = 0, {}, [], []
    for f in tracked_text_files(g):
        path = os.path.join(g.repo, f)
        with open(path, encoding="utf-8", newline="") as fh:
            text = fh.read()
        count = 0

        def sub(m):
            nonlocal count
            tok = m.group(1)
            hits = resolve(tok)
            if not hits:
                return tok
            if len(hits) > 1:
                ambiguous.append((f, tok, f"prefix of {len(hits)} old commits, left as is"))
                return tok
            count += 1
            if tok.isdigit():
                # short ids made of decimal digits only (14221138, 3933363) are real citations in the queue
                # and the worklog; listed so the main session can glance over them
                decimal.append((f, tok))
            return unique_prefix(mapping[hits[0]], len(tok))

        new_text = HEX_TOKEN.sub(sub, text)
        if new_text != text:
            with open(path, "w", encoding="utf-8", newline="") as fh:
                fh.write(new_text)
            per_file[f] = count
            total += count
    return total, per_file, ambiguous, decimal


def run(repo, tsv, branch, force, dry):
    g = Git(repo)
    # the subjects table may carry uncommitted rows (--draft-missing, reviewed): they go into the final commit
    tsv_rel = os.path.relpath(tsv, repo)
    dirty = [ln for ln in g.out("status", "--porcelain", "--untracked-files=no").splitlines() if ln[3:] != tsv_rel]
    if dirty:
        raise SystemExit("refusing: uncommitted changes in tracked files:\n  " + "\n  ".join(dirty))
    agents = agent_worktrees(g)
    if agents and not force:
        raise SystemExit("refusing: agent worktrees are registered (agents may be working):\n  "
                         + "\n  ".join(agents) + "\nfinish or remove them first, or pass --force")
    if g.run("rev-parse", "--verify", "-q", ARCHIVE + branch, check=False).returncode == 0 and not force:
        raise SystemExit(f"refusing: {ARCHIVE + branch} exists (already rewritten?); pass --force to run again")
    rows = load_tsv(tsv)
    missing, bad, extra = check_rows(g, rows, branch)
    if bad:
        for s, subj, p in bad:
            print(f"BAD SUBJECT {s[:10]} {subj!r}: {'; '.join(p)}")
        raise SystemExit("fix the subjects above in the TSV")
    if missing:
        for s in missing:
            print(f"NO ROW {s} {g.out('show', '-s', '--format=%s', s)[:90].strip()}")
        raise SystemExit(f"{len(missing)} commits have no TSV row; run --draft-missing, review, run again")
    if extra:
        print(f"note: {len(extra)} TSV rows name commits that are not on {branch} (ignored)")

    old_tip = g.out("rev-parse", branch).strip()
    stamp = datetime.datetime.now().strftime("%Y%m%d-%H%M%S")
    # 1. archive refs and a bundle of every ref
    branches = {}
    heads = g.out("for-each-ref", "--format=%(refname) %(objectname) %(objecttype)", "refs/heads", "refs/tags")
    order, mapping = rewrite_objects(g, rows, branch)
    fails = verify(g, order, mapping, rows)
    if fails:
        print("\n".join(fails[:50]))
        raise SystemExit(f"verification failed for {len(fails)} commits; no ref was changed")
    checked_out = set()
    for ln in g.out("worktree", "list", "--porcelain").splitlines():
        if ln.startswith("branch "):
            checked_out.add(ln[7:])
    main_wt_branch = g.out("symbolic-ref", "-q", "HEAD").strip() if g.run(
        "symbolic-ref", "-q", "HEAD", check=False).returncode == 0 else ""
    for ln in heads.splitlines():
        ref, sha, typ = ln.split()
        if ref == "refs/heads/" + branch:
            branches[ref] = sha
        elif typ == "commit" and sha in mapping:
            if ref in checked_out and ref != main_wt_branch:
                print(f"skip {ref}: checked out in another worktree")
                continue
            branches[ref] = sha
        elif ref.startswith("refs/tags/") and typ == "tag":
            print(f"skip {ref}: annotated tag (re-create it by hand on the mapped commit)")
    os.makedirs(os.path.join(repo, "target", "history"), exist_ok=True)
    bundle = os.path.join(repo, "target", "history", f"pre-rewrite-{stamp}.bundle")
    for ref, sha in branches.items():
        short = ref.split("/", 2)[2]
        g.run("update-ref", ARCHIVE + short, sha)
    g.run("bundle", "create", bundle, "--all")
    g.run("bundle", "verify", bundle)
    print(f"archive: {len(branches)} refs under {ARCHIVE}, bundle {bundle}")

    # 2. move the refs (old value checked, so a concurrent change fails instead of being overwritten)
    for ref, sha in branches.items():
        g.run("update-ref", "-m", "GIT-1 history rewrite", ref, mapping[sha], sha)
    print(f"moved {len(branches)} refs: " + ", ".join(r.split('/', 2)[2] for r in branches))
    if main_wt_branch == "refs/heads/" + branch:
        # same tree, so only the index needs to see the new HEAD; refresh it
        g.run("update-index", "-q", "--refresh", check=False)

    # 3. the hash map and the reference update, as one commit on the branch
    hm = os.path.join(repo, HASH_MAP)
    os.makedirs(os.path.dirname(hm), exist_ok=True)
    with open(hm, "w", encoding="utf-8") as f:
        f.write("# old_sha\tnew_sha  (GIT-1 history rewrite; old commits kept under refs/archive/pre-rewrite)\n")
        for o in order:
            f.write(f"{o}\t{mapping[o]}\n")
    total, per_file, ambiguous, decimal = update_references(g, mapping)
    for f, n in sorted(per_file.items()):
        print(f"  {n:5d}  {f}")
    for f, tok, why in ambiguous:
        print(f"  AMBIGUOUS {f}: {tok} ({why})")
    for f, tok in sorted(set(decimal)):
        print(f"  DECIMAL-ONLY {f}: {tok} (replaced: resolves to one old commit)")
    if main_wt_branch != "refs/heads/" + branch:
        raise SystemExit(f"HEAD is not {branch}; commit the reference update by hand")
    g.run("add", "-u")
    g.run("add", HASH_MAP)
    if not tsv_rel.startswith(".."):
        g.run("add", tsv_rel)
    body = (f"Rewrite the commit ids cited in tracked text files to the rewritten history ({total} references "
            f"in {len(per_file)} files;\n{len(ambiguous)} ambiguous tokens left as they were). "
            f"docs/history/hash-map.tsv maps every old commit\nto its new one; the old history stays under "
            f"refs/archive/pre-rewrite and in the bundle\n{os.path.relpath(bundle, repo)}.\n\nRefs: GIT-1\n")
    g.run("commit", "-q", "-F", "-", input=f"{REF_COMMIT_SUBJECT}\n\n{body}".encode())
    new_tip = g.out("rev-parse", branch).strip()
    print(f"{branch}: {old_tip[:12]} -> {mapping[old_tip][:12]} (+ reference commit {new_tip[:12]})")
    print(f"commits rewritten: {len(order)}; references updated: {total}; ambiguous left: {len(ambiguous)}")
    if not dry:
        print("next: check `git log`, then the user force-pushes (docs/history/README.md)")
    return 0


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", default=None, help="repository (default: this checkout)")
    ap.add_argument("--tsv", default=None, help=f"subjects table (default: <repo>/{TSV_DEFAULT})")
    ap.add_argument("--branch", default="master")
    ap.add_argument("--force", action="store_true", help="run although agent worktrees are registered")
    ap.add_argument("--dry-run", metavar="DIR", help="clone the repository into DIR (must not exist) and run there")
    ap.add_argument("--draft-missing", action="store_true", help="append draft TSV rows for commits without one")
    ap.add_argument("--check", action="store_true", help="only report missing rows and subject problems")
    a = ap.parse_args()
    repo = a.repo or subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True,
                                    check=True).stdout.strip()
    tsv = os.path.abspath(a.tsv or os.path.join(repo, TSV_DEFAULT))
    g = Git(repo)
    if a.draft_missing:
        return draft_missing(g, tsv, a.branch)
    if a.check:
        missing, bad, extra = check_rows(g, load_tsv(tsv), a.branch)
        for s in missing:
            print(f"NO ROW {s}")
        for s, subj, p in bad:
            print(f"BAD SUBJECT {s[:10]} {subj!r}: {'; '.join(p)}")
        print(f"missing {len(missing)}, bad subjects {len(bad)}, rows not on {a.branch}: {len(extra)}")
        return 1 if missing or bad else 0
    if a.dry_run:
        d = os.path.abspath(a.dry_run)
        if os.path.exists(d):
            raise SystemExit(f"{d} exists; remove it first")
        subprocess.run(["git", "clone", "-q", "--no-local", repo, d], check=True)
        cur = Git(d)
        if cur.out("symbolic-ref", "--short", "HEAD").strip() != a.branch:
            cur.run("checkout", "-q", a.branch)
        return run(d, tsv, a.branch, True, True)
    return run(repo, tsv, a.branch, a.force, False)


if __name__ == "__main__":
    sys.exit(main())
