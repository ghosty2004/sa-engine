#!/usr/bin/env python3
"""Move a branch that was based on the old (pre-rewrite) history onto the rewritten one (ROADMAP GIT-1).

    rebase_after_rewrite.py --list                 branches still based on the old history, with their commands
    rebase_after_rewrite.py BRANCH                 print the git rebase --onto command for BRANCH
    rebase_after_rewrite.py BRANCH --apply         run it (in the worktree that has BRANCH checked out, if any)

The branch's fork point is its merge base with the archived old master (refs/archive/pre-rewrite/master);
docs/history/hash-map.tsv gives the rewritten commit for it, and only the branch's own commits are replayed:

    git rebase --onto <new fork point> <old fork point> BRANCH

The trees of old and new history are identical, so the replay applies cleanly whenever the branch did before.
"""
import argparse
import os
import subprocess
import sys

ARCHIVE = "refs/archive/pre-rewrite/"


def git(repo, *args, check=True):
    r = subprocess.run(["git", "-C", repo, *args], capture_output=True, text=True)
    if check and r.returncode != 0:
        raise SystemExit(f"git {' '.join(args)}: {r.stderr.strip()}")
    return r


def load_map(path):
    m = {}
    with open(path, encoding="utf-8") as f:
        for ln in f:
            if ln.startswith("#") or not ln.strip():
                continue
            old, new = ln.split()[:2]
            m[old] = new
    return m


def worktree_of(repo, branch):
    cur = None
    for ln in git(repo, "worktree", "list", "--porcelain").stdout.splitlines():
        if ln.startswith("worktree "):
            cur = ln[9:]
        elif ln == f"branch refs/heads/{branch}":
            return cur
    return None


def plan(repo, branch, hmap, base_branch):
    old_master = ARCHIVE + base_branch
    tip = git(repo, "rev-parse", f"refs/heads/{branch}").stdout.strip()
    if tip in hmap.values():
        return None, "already on the rewritten history"
    if tip in hmap:
        return None, f"fully merged: point it at {hmap[tip][:12]} (git branch -f {branch} {hmap[tip][:12]})"
    mb = git(repo, "merge-base", tip, old_master, check=False).stdout.strip()
    if not mb:
        return None, "no common history with the old master"
    if mb not in hmap:
        return None, f"merge base {mb[:12]} is not in the hash map"
    return ["rebase", "--onto", hmap[mb], mb, branch], f"{len(git(repo, 'rev-list', f'{mb}..{tip}').stdout.split())} commits"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("branch", nargs="?")
    ap.add_argument("--list", action="store_true")
    ap.add_argument("--apply", action="store_true")
    ap.add_argument("--base", default="master", help="the rewritten branch (default master)")
    ap.add_argument("--repo", default=None)
    ap.add_argument("--map", default=None, help="hash map (default <repo>/docs/history/hash-map.tsv)")
    a = ap.parse_args()
    repo = a.repo or git(".", "rev-parse", "--show-toplevel").stdout.strip()
    hmap = load_map(a.map or os.path.join(repo, "docs/history/hash-map.tsv"))
    if git(repo, "rev-parse", "--verify", "-q", ARCHIVE + a.base, check=False).returncode != 0:
        raise SystemExit(f"{ARCHIVE + a.base} does not exist: the history has not been rewritten here")
    if a.list:
        heads = git(repo, "for-each-ref", "--format=%(refname:short)", "refs/heads").stdout.split()
        for b in heads:
            if b == a.base:
                continue
            cmd, note = plan(repo, b, hmap, a.base)
            wt = worktree_of(repo, b)
            where = f"  (checked out in {wt})" if wt else ""
            print(f"{b}: {note}{where}" + (f"\n    git {' '.join(cmd)}" if cmd else ""))
        return 0
    if not a.branch:
        ap.error("give a BRANCH or --list")
    cmd, note = plan(repo, a.branch, hmap, a.base)
    print(f"{a.branch}: {note}")
    if not cmd:
        return 0
    wt = worktree_of(repo, a.branch)
    if wt:
        print(f"git -C {wt} {' '.join(cmd)}")
        if a.apply:
            return subprocess.run(["git", "-C", wt, *cmd]).returncode
        return 0
    # not checked out anywhere: rebase in a temporary detached worktree, so the caller's checkout
    # (usually master in the main worktree) never switches branches
    tmp = os.path.join(repo, "target", "history", f"rebase-{a.branch.replace('/', '-')}")
    print(f"git worktree add --detach {tmp} {cmd[2]} && git -C {tmp} {' '.join(cmd)} && git worktree remove {tmp}")
    if not a.apply:
        return 0
    os.makedirs(os.path.dirname(tmp), exist_ok=True)
    git(repo, "worktree", "add", "-q", "--detach", tmp, cmd[2])
    rc = subprocess.run(["git", "-C", tmp, *cmd]).returncode
    if rc == 0:
        git(repo, "worktree", "remove", tmp)
    else:
        print(f"rebase stopped; resolve it in {tmp}, then `git worktree remove {tmp}`")
    return rc


if __name__ == "__main__":
    sys.exit(main())
