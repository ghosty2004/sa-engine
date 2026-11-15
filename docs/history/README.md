# History rewrite to Conventional Commits (GIT-1)

The user asked (2026-11-15) for every commit message of the history to follow the Conventional Commits format
of CLAUDE.md ("Commit messages"), **without losing anything**. Only the messages change; trees, authors,
committers and dates stay the same. The main session runs it when no agent is working; the user force-pushes
afterwards.

## Files

| File | What |
| --- | --- |
| `docs/history/commit-messages.tsv` | One row per non-merge commit of master, oldest first: `old sha <TAB> new subject <TAB> refs`. Subjects were written by hand from each original message and its diff stat; `refs` holds the ROADMAP task ids the original message names (comma-separated, empty when it names none). |
| `docs/history/hash-map.tsv` | Written by the real run: every rewritten commit (merges too), `old sha <TAB> new sha`. |
| `tools/git/rewrite_history.py` | The rewrite (`--check`, `--draft-missing`, `--dry-run DIR`, real run). |
| `tools/git/rebase_after_rewrite.py` | Moves a branch based on the old history onto the new one. |
| `tools/verify/commit_lint.py --all` | Checks commits older than the rule's CUTOFF too. |

## What a rewritten commit looks like

```
feat(sa-pop): port the police path of CCarAI::UpdateCarAI        <- new subject from the TSV

FRONTIER-2: police path (CCarAI::UpdateCarAI and ...)             <- the original message, verbatim
...                                                                   (not reflowed, not trimmed)

Refs: FRONTIER-2                                                  <- when the TSV row has refs
Original-commit: 11e8cd0f...                                      <- the old full sha, always
```

- A commit whose original subject already equals its TSV subject (the two commits made after the rule) keeps
  its message; only `Original-commit:` is added, into its trailer block.
- Merge commits keep their message and get `Original-commit:` (merges are not linted).
- `commit_lint.py` does not apply the 100-column body rule to a message with an `Original-commit:` trailer:
  the body is the original text, which may not be reflowed. Subject and blank line are still checked.

The script does not use `git filter-repo`: it rebuilds each commit object from its raw bytes
(`git cat-file` → replace the `parent` lines with the rewritten parents and the message → `git hash-object -w`).
Every other header (tree, author, committer, encoding) is copied byte for byte, so nothing else can change,
and it runs the same way on a clone or on the real repository. Signed commits would be refused (there are none).

## Decisions

- **Rewritten:** master. **Moved with it:** every other local branch and lightweight tag whose tip is a commit
  of master's history (fully merged); they point at the rewritten commit afterwards. A branch checked out in
  another worktree is skipped (reported). Annotated tags are reported, not moved (there are none today).
- **Left alone:** branches with commits of their own (not merged into master). They stay on the old history
  until moved with `rebase_after_rewrite.py` (below), or deleted when no longer needed.
- **Cited hashes:** after the rewrite, every token in a tracked text file that is lowercase hex, 7–40 characters,
  not part of a longer word, not preceded by `0x`, and is the prefix of exactly one old commit, is replaced by the
  new commit's id at the same length (longer only if that prefix would be ambiguous). Addresses such as
  `0x6D7BC0`, `FUN_006070f0`, sha-256 values and samp-rs commit ids (not commits of this repository) do not
  resolve, so they are not touched. A prefix of several old commits is left alone and listed as AMBIGUOUS.
  Ids made of decimal digits only (`14221138`, `3933363`) are real citations in the verification queue and the
  worklog; they are replaced and listed as DECIMAL-ONLY for a glance. `docs/history/` keeps the old ids.
  Commit messages are not edited: the original text cites old ids, which `hash-map.tsv` and the archive resolve.
- The hash map, the reference updates and any reviewed draft TSV rows go into one commit on master:
  `docs: update commit references after the history rewrite`.

## Running it (main session)

Before: every agent finished and its work integrated or abandoned; `git worktree list` shows no
`.claude/worktrees/*` entry (the script refuses otherwise, unless `--force`); `git status` clean in tracked files.

```sh
cd ~/sa-engine
git switch master
# 1. rows for commits made after the TSV was written (anything after 03319d70 and this branch)
python3 -I tools/git/rewrite_history.py --check
python3 -I tools/git/rewrite_history.py --draft-missing     # appends draft rows; edit them by hand
python3 -I tools/git/rewrite_history.py --check            # must print: missing 0, bad subjects 0
# 2. a dry run on a fresh clone (target/ is gitignored), then look at it
python3 -I tools/git/rewrite_history.py --dry-run target/history-dryrun/clone
git -C target/history-dryrun/clone log --oneline | head -50
python3 -I tools/verify/commit_lint.py --range master --all   # run inside the clone: 0 bad
rm -rf target/history-dryrun
# 3. the real run
python3 -I tools/git/rewrite_history.py
git log --oneline | head -20
python3 -I tools/verify/commit_lint.py --range master --all
python3 -I tools/git/rebase_after_rewrite.py --list
```

The uncommitted draft rows in the TSV are allowed by the clean-tree check and are committed with the reference
update. The real run prints the archive refs, the bundle path, the moved refs, the per-file reference counts,
the AMBIGUOUS and DECIMAL-ONLY lists, and the new tip.

What the real run does, in order: refuse if needed → check the TSV covers every commit and every subject
passes the lint → write the new commit objects → verify every pair (headers other than `parent` identical,
parents mapped, original message contained verbatim, `Original-commit` present, lint clean) → only then
create `refs/archive/pre-rewrite/<branch>` for every ref it will move and the bundle
`target/history/pre-rewrite-<date>-<time>.bundle` (`--all`) → move the refs with `git update-ref <ref> <new> <old>`
(fails rather than overwrites if a ref moved meanwhile) → write `hash-map.tsv` → update the cited ids →
commit. If verification fails nothing has been changed.

## After the run: the user force-pushes

```sh
git push --force-with-lease=master:<old master sha> origin master
```

The old sha is `git rev-parse refs/archive/pre-rewrite/master`. `--force-with-lease` refuses if the remote moved
since. The archive refs are local only (not pushed); keep them and the bundle until the user is satisfied.
Other clones must re-clone or `git fetch && git reset --hard origin/master` (after saving local work).

## Agent branches based on the old history

```sh
python3 -I tools/git/rebase_after_rewrite.py --list            # every branch, with the command for it
python3 -I tools/git/rebase_after_rewrite.py BRANCH            # print: git rebase --onto <new> <old> BRANCH
python3 -I tools/git/rebase_after_rewrite.py BRANCH --apply    # run it (in the branch's worktree if it has one)
```

The fork point is the branch's merge base with `refs/archive/pre-rewrite/master`; `hash-map.tsv` gives its
new id; only the branch's own commits are replayed. Fully merged branches are reported with the
`git branch -f` command instead.

## Recovery

- Undo the whole rewrite locally: `git update-ref refs/heads/master refs/archive/pre-rewrite/master` (and the
  same for every other `refs/archive/pre-rewrite/*` ref), then `git reset --hard` in the main checkout.
- From the bundle (e.g. on another machine or after the archive refs were deleted):
  `git bundle verify target/history/pre-rewrite-*.bundle` then
  `git fetch target/history/pre-rewrite-<stamp>.bundle 'refs/heads/*:refs/restored/*'`.
- An old id cited somewhere (a message, a chat log): look it up in `docs/history/hash-map.tsv`, or search
  `git log --grep 'Original-commit: <old sha>'`.

## Dry run of 2026-11-15 (this branch)

Fresh clone, master fast-forwarded to this branch (so the drafted rows path is exercised), the 182 other
branches created as local branches. Results are in the ROADMAP / WORKLOG entry for GIT-1.
