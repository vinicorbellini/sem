# 1.8 Diff quality on abapGit pull requests

## Intent

All the structure work is only worth it if `sem diff` names the methods a
reviewer would name. Test that on real history: 20 merged pull requests of the
abapGit repository, ten of them checked by hand against what a reviewer would
call changed. This story is the human-judged part of Gate 1, and it runs last.

## Acceptance criteria

- The abapGit clone has enough history. It is shallow at one commit today. After
  `git fetch --unshallow`, `git log --oneline | grep -c '(#[0-9]*)$'` is at least
  200.
- A list of 20 PR numbers is chosen and recorded in
  `docs/abap/census-gate1.md` before any diff is run. Selection rule: take
  squash-merge commits on the main branch whose subject ends in `(#NNNN)`,
  keep those that change at least one `.abap` file under `src/`, sort by PR
  number, and take every k-th so the 20 spread over the range. Do not pick by
  looking at the diffs.
- `sem diff <sha>^ <sha>` runs for each of the 20 without error and without a
  crash on any file.
- For 10 of the 20 (every second one in the sorted list), the story records in
  `census-gate1.md` a table row: PR number, the changed methods a reviewer
  would name (taken from the PR title and the `git diff -U0` hunks), the
  entities `sem diff` named, and a verdict of match, partial or miss.
- Pass threshold: at least 8 of the 10 are match or partial, and no miss is a
  case where sem named nothing for a file with a changed method. Each miss and
  partial has a one-line cause, for example "form in report, no FORM entity".
- The parse-error counts from story 1.6 are recorded for the changed files.
- Every miss is turned into either a fixed bug (new test in `mod.rs`) or a
  line in `docs/abap/stories/` as a Tier 2 candidate.
- No fixture test is tied to this story; its evidence is the census document.

## Files

- `docs/abap/census-gate1.md`: the PR list, the 10-row hand-check table, the
  pass/fail result.
- `/tmp/claude-0/abapGit`: read-only except for the unshallow fetch.
- No source changes are planned. Fixes found here land as small commits under
  the story that owns the cause.

## Approach

1. Deepen the clone: `git -C /tmp/claude-0/abapGit fetch --unshallow`. The
   history uses squash merges, so `git log --merges` is empty. Find PRs through
   the `(#NNNN)` suffix in the subject.
2. List candidates:
   `git -C /tmp/claude-0/abapGit log --format='%h %s' -- 'src/**/*.abap'` and
   filter on the suffix. Apply the selection rule and write the 20 numbers down.
3. For each, run `sem diff <sha>^ <sha> --json` from the clone with the built
   binary. The binary comes from the `abap` branch with stories 1.1 to 1.7
   merged.
4. For the 10 hand-checked PRs, read `git show --stat` and `git show -U0` for
   the `.abap` files. Write down the methods you would name in a review. Do
   this before reading the sem output, so the check is not anchored.
5. Compare and score. Treat "partial" as: sem named the class but not the method,
   or named a method plus unrelated noise. Treat "miss" as: sem named no entity
   in a file whose method changed.

Pattern to copy: `crates/sem-core/examples/diff_oracle.rs` replays real commits
through the diff pipeline and compares two modes; its commit-walking code is a
starting point for the batch run, though it requires the `git` feature and does
not do the hand check.

## Verification

```bash
git -C /tmp/claude-0/abapGit fetch --unshallow
git -C /tmp/claude-0/abapGit log --format='%h %s' -- 'src/**/*.abap' | grep -E '\(#[0-9]+\)$' | wc -l
cd crates && cargo build --release -p sem-cli
cd /tmp/claude-0/abapGit && /path/to/crates/target/release/sem diff <sha>^ <sha> --json
cd crates && cargo test --workspace
```

Replace `/path/to` with the worktree path. There are no fixture tests to flip for
this story. Gate 1 evidence is the filled-in `docs/abap/census-gate1.md`.

## Out of scope

- Rating diff quality on non-method changes (DATA, TYPES, macros).
- Automating the hand check. The point is human judgement.
- Comparing against other tools.
- Fixing grammar errors found along the way.

## Estimate

1-2 days.

## Depends on

Stories 1.1 to 1.7 and 1.9. Run it last.
