#!/usr/bin/env python3
"""Commit message check (Conventional Commits, CLAUDE.md "Commit messages").

    commit_lint.py --file MSGFILE        check one message (the commit-msg hook: .githooks/commit-msg)
    commit_lint.py --range BASE..HEAD    check every non-merge commit of a range (the writer gate's `commits` step)

The format:

    <type>(<scope>): <summary>          type: feat fix refactor perf test docs build ci chore revert;
                                         scope: optional, lowercase (a crate, `tools`, `catalog`, `roadmap`, ...);
    <blank line>                         summary: imperative, no trailing period; the whole line <= 72 characters
    <body, wrapped at 100>
    <trailers: Refs: TASK-ID, Verified: ...>

Commits committed before CUTOFF (when the rule was introduced) are not checked by --range, so branches that were
already in progress keep passing; the history rewrite (tools/git/rewrite_history.py) handles those.
"""
import argparse
import re
import subprocess
import sys

TYPES = ("feat", "fix", "refactor", "perf", "test", "docs", "build", "ci", "chore", "revert")
SUBJECT = re.compile(r"^(?P<type>[a-z]+)(?:\((?P<scope>[a-z0-9][a-z0-9._/-]*)\))?(?P<bang>!)?: (?P<summary>\S.*)$")
MAX_SUBJECT = 72
MAX_BODY = 100
# 2026-11-15 08:12 +0200: the rule's introduction
CUTOFF = 1794723141


def problems(msg: str) -> list[str]:
    lines = [ln for ln in msg.splitlines() if not ln.startswith("#")]
    while lines and not lines[-1].strip():
        lines.pop()
    if not lines:
        return ["empty message"]
    out = []
    subject = lines[0]
    m = SUBJECT.match(subject)
    if not m:
        out.append(f"subject is not '<type>(<scope>): <summary>': {subject[:80]!r}")
    else:
        if m["type"] not in TYPES:
            out.append(f"type {m['type']!r} is not one of {', '.join(TYPES)}")
        s = m["summary"]
        if s.endswith("."):
            out.append("summary ends with a period")
        if s[0].isupper() and not re.match(r"^[A-Z][A-Za-z0-9]*[A-Z0-9_:]", s):
            out.append("summary starts with a capital letter (use the imperative, lowercase: 'port …', 'fix …')")
    if len(subject) > MAX_SUBJECT:
        out.append(f"subject is {len(subject)} characters (at most {MAX_SUBJECT})")
    if len(lines) > 1 and lines[1].strip():
        out.append("no blank line between the subject and the body")
    for i, ln in enumerate(lines[2:], 3):
        if len(ln) > MAX_BODY and not re.match(r"^\s*(https?://|\S+$)", ln):
            out.append(f"body line {i} is {len(ln)} characters (at most {MAX_BODY})")
            break
    return out


def git(*args: str) -> str:
    return subprocess.run(["git", *args], check=True, capture_output=True, text=True).stdout


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--file")
    g.add_argument("--range")
    a = ap.parse_args()
    if a.file:
        with open(a.file, encoding="utf-8") as f:
            p = problems(f.read())
        for x in p:
            print(f"commit message: {x}", file=sys.stderr)
        if p:
            print("format: <type>(<scope>): <summary>  (CLAUDE.md, Commit messages; tools/verify/commit_lint.py)",
                  file=sys.stderr)
        return 1 if p else 0
    shas = git("rev-list", "--no-merges", "--reverse", a.range).split()
    checked = bad = 0
    for sha in shas:
        if int(git("show", "-s", "--format=%ct", sha)) < CUTOFF:
            continue
        checked += 1
        p = problems(git("show", "-s", "--format=%B", sha))
        if p:
            bad += 1
            print(f"FAIL {sha[:8]}: {'; '.join(p)}")
    print(f"commits: {checked} checked in {a.range} ({len(shas) - checked} older than the rule), {bad} bad")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
