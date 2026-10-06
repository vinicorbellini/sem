# ABAP agent benchmark

Paired agent runs on a pinned commit of [abapGit](https://github.com/abapGit/abapGit)
(`b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`). Each run is one headless Claude Code invocation
(`claude -p`), started as a subprocess in the run's own checkout. Each task runs twice per
repetition. Both runs use the same model, effort, turn bound and prompt:

- **grep arm**: Claude Code's built-in `Bash`, `Read`, `Grep` and `Glob`. On B2, `Edit` and `Write` as well.
- **sem arm**: the same, plus every tool `sem mcp` lists (`sem_find`, `sem_grep`, `sem_impact`,
  `sem_check`, `sem_certify`, `sem_diff`, `sem_graph`, `sem_history`; `LISTED_TOOLS` in
  `crates/sem-mcp/src/server.rs`), attached with `--mcp-config` as a standalone stdio server.

Every run gets a fresh clone of the pinned checkout, so an edit in one run never reaches another.
The arm order alternates by repetition.

## Tasks

| Class | Tasks | Asks | Scored by |
|---|---|---|---|
| B1 where-used | 10 (`tasks/b1_whereused.json`) | list every method that calls X, with file and line | precision/recall of calling methods against `ground_truth/whereused.json` |
| B2 change-and-verify | 6 (`tasks/b2_change.json`) | add mandatory parameter p to method X, update every caller, make the tests pass | hidden tests and `npm run unit` pass, test classes the agent ran, callers missed |
| B3 review | 4 (`tasks/b3_review.json`) | summarise what merged PR #n changed and what it could break | changed entities and callers left behind against `ground_truth/b3_rubric.json` |

How the targets were picked is in each task file's `selection` field. In short:

- **B1**: methods declared in exactly one class or interface, with 3 to 30 calling methods found by grep, spread over 3 or more files. The grep counts are stored as a baseline, not as ground truth.
- **B2**: methods whose own class has a test class that runs under `npm run unit`. Each hidden test in `hidden_tests/` was checked twice. It passes against a reference implementation (the parameter added with a DEFAULT, callers untouched). It fails to build on the unchanged checkout.
- **B3**: four squash-merged PRs from the 300 commits before the pinned one.

**Ground truth status.** `whereused.json` does not exist yet; it will come from `sapcli whereused`.
Until it does, B1 is scored against `ground_truth/whereused.grep.json`, the hand-checked text
where-used of story 2.7 (`docs/abap/census-gate2.md`), which `tasks/b1_whereused.json` points to. It is
`crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json` converted with
`python3 scripts/abap-whereused-convert.py to-harness ... --tasks bench/abap-agent/tasks/b1_whereused.json`.
B1 rows from before it carry `error = no ground truth yet` and an empty score. The answers are kept
in `results.jsonl` (`final_text`), so they can be scored later without rerunning. Convert sapcli's
output to:

```json
{"abapgit_commit": "b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0",
 "targets": {"b1_01": [{"method": "<class>-><method>", "file": "src/...", "line": 12}]}}
```

`b3_rubric.json` is a **draft**. It was written from `git show` and grep at the pinned commit and
cross-checked with `sem diff --commit`. Review it before reading much into B3 scores.

## Running

Runs use the **owner's Claude account**: whichever account `claude` on PATH is logged in with
(OAuth, `claude auth`). No `ANTHROPIC_API_KEY` is needed or read. Under a subscription the USD
figure is Claude Code's **nominal estimate** at list prices (`total_cost_usd`), not a bill. It still
drives `--cap-usd`, because it is the one cost number every run reports the same way.

```sh
cp -r <main checkout>/crates/target crates/target       # optional: reuse a warm build
(cd crates && cargo build --release -p sem-cli)          # sem with ABAP (default features)
claude --version                                         # Claude Code on PATH, logged in

python3 bench/abap-agent/run.py --dry-run --arm both --class all --reps 1     # never calls claude
python3 bench/abap-agent/run.py --checkpoint baseline --arm both --class B3 --reps 1
python3 bench/abap-agent/run.py --checkpoint gate1 --class B1 --task b1_03 --reps 1
```

| Flag | Meaning |
|---|---|
| `--checkpoint baseline\|gate1\|gate2` | Written to every row; required for a real run. |
| `--arm grep\|sem\|both` | Which arms to run (default both). |
| `--class B1\|B2\|B3\|all` | Which task classes (default all); `--task b2_04` (repeatable) narrows to single tasks. |
| `--model` | Passed to `claude --model` (default `claude-sonnet-5-5`). |
| `--reps` | Repetitions per task per arm (default 3). |
| `--brief` | Add one paragraph to the **sem arm's** prompt naming `sem_find`, `sem_impact` and `sem_certify` and telling the agent to prefer them over grep (`BRIEF` in `run.py`; see "Briefing"). The grep arm's prompt is unchanged. Default off. Recorded in the `brief` column. |
| `--dry-run` | Everything except calling `claude`. It prepares the checkouts, writes each sem run's MCP config, and prints each run's working directory, exact command line and prompt. For the sem arm it starts `sem mcp` once, lists its tools and calls `sem_find` (B1, B2) or `sem_certify` (B3). It scores the untouched checkout. B2 scoring runs the unit suite, so expect about 30 s per B2 run. |
| `--cap-usd` | Stop once the cumulative `total_cost_usd` reaches the cap (default 30; see below). |
| `--abapgit` | abapGit clone to copy from (default `/tmp/claude-0/abapGit`). It is cloned, never modified. |
| `--work-dir` | Checkouts, transcripts, MCP configs, sem logs (default `/tmp/sem-abap-agent`). |
| `--sem-binary` | sem to serve MCP. By default `$SEM_BINARY`, then `crates/target/release/sem` in this checkout, then in the main checkout when this is a linked worktree. |
| `--keep` | Keep each run's checkout. |

The first run prepares `<work-dir>/base`:

1. Clone `--abapgit`.
2. Fetch 300 commits of history from GitHub when the B3 commits are missing (a shallow clone).
3. Check out the pinned commit.
4. `npm ci --ignore-scripts` with `abapgit-package-lock.json` (see Known issues).
5. Clone the transpiler's libraries at the commits in `abapgit-transpile-libs.json` into `<work-dir>/libs/<name>` (see Known issues).

Each run clones `base`, symlinks its `node_modules` and points its `test/abap_transpile.json` at `<work-dir>/libs`. That file is marked `skip-worktree`, so it is not in the agent's `git status` or in the patch the scorer replays.

### The command per run

The grep arm, from the run's checkout (`cwd`), with the prompt as the one positional argument:

```sh
claude -p "$PROMPT" --output-format stream-json --verbose \
  --model claude-sonnet-5-5 --effort high --max-turns 40 \
  --tools Bash,Read,Grep,Glob --allowedTools Bash,Read,Grep,Glob \
  --permission-mode dontAsk --setting-sources '' --strict-mcp-config \
  --no-session-persistence --session-id <uuid> --max-budget-usd <rest of the cap>
```

The sem arm is the same, with `mcp__sem` added to `--allowedTools` and
`--mcp-config <work-dir>/mcp/<run>.json`. That file starts `sem mcp` in the checkout with
`SEM_REPO` set to it, `SEM_MCP_NO_SHARED=1` (a standalone stdio process, not the shared daemon, so
the server ends with the run), and telemetry, update checks, cloud and network off. The server's
stderr goes to `<work-dir>/<run>.sem-mcp.log`. On B2 both arms add `Edit,Write` to `--tools` and
`--allowedTools`.

Why each flag:

- `--tools` decides which built-in tools exist at all. `--allowedTools` only pre-approves them.
- `--permission-mode dontAsk` denies anything not pre-approved instead of prompting. Nobody is there to answer a prompt.
- `--setting-sources ''` keeps user, project and local settings out of the run, with their hooks and CLAUDE.md files. `--strict-mcp-config` does the same for MCP servers configured elsewhere.
- `--max-turns 40` is a safety bound. Every baseline run finished in 11 turns or fewer.
- `--effort high` and `--model` are explicit, because the defaults depend on the account and environment. A probe with no `--model` ran on a different model depending on the inherited environment.

The harness removes the launching Claude Code session's own variables from the child's
environment (`PARENT_SESSION_ENV` in `tools.py`). If they are inherited, the child joins the
parent's session id and picks up its extra directories and their CLAUDE.md files. Auth and proxy
variables stay. Each run also has a wall-clock timeout (`RUN_TIMEOUT_S`: 30 min for B1 and B3,
60 min for B2). On timeout the whole process group is killed (claude, sem mcp, npm).

**Prompt.** `claude -p` takes one prompt, and Claude Code keeps its own system prompt. The text the
SDK harness sent as its system prompt (`INSTRUCTIONS` in `run.py`) is therefore the head of the
prompt, followed by the task text and its output contract. Both arms get identical text. The one
inherent difference: Claude Code adds the sem server's MCP instructions and tool schemas to the sem
arm's context. That is part of what the sem arm costs.

**Transcript.** `--output-format json` gives the totals but no tool calls, so runs use
`--output-format stream-json --verbose`. The harness saves each transcript to
`<work-dir>/transcripts/<run>.jsonl`. Tool calls, files read, bytes read and test classes are
parsed from its `tool_use` and `tool_result` events. Tokens, cost, turns and API time come from
its final `result` event, the same object `--output-format json` returns.

**Cost cap.** `--cap-usd` is fed by `total_cost_usd`. No run starts once the cumulative cost has
reached the cap. Each run gets the rest of the cap as `--max-budget-usd`, so the cap can be passed
by about one request. `prices.json` is now only a cross-check (`list_price_usd` in
`results.jsonl`). It runs about 20% under Claude Code's figure, because Claude Code writes 1-hour
cache entries (2x the input price) and `prices.json` holds the 5-minute write rate.

## Output

`results.csv` is append-only, one row per run. A file with different columns is an error, never
rewritten. `results.jsonl` holds the same rows plus:

- the Claude Code version, session id and exact command;
- the prompt and final answer;
- the tools and MCP server status Claude Code reported at start;
- per-model usage, permission denials and the price-table cross-check;
- per-tool call counts, files read and full score details;
- the transcript path.

Dry runs write `results.dry-run.csv` / `.jsonl`. The repo's `.gitignore` ignores `bench/`, so
commit results with `git add -f bench/abap-agent/results.csv bench/abap-agent/results.jsonl`.

| Column | Meaning |
|---|---|
| `timestamp` | UTC, when the run finished. |
| `checkpoint` | `baseline`, `gate1` or `gate2` (`dry-run` for dry runs). |
| `build` | sem commit measured: `git rev-parse HEAD` of this repo, `-dirty` if `crates/` has uncommitted changes. Build the binary from that commit. |
| `sem_version` | `sem --version` of the binary that served MCP. |
| `abapgit_commit` | Pinned abapGit commit. |
| `model`, `arm`, `task_class`, `task_id`, `rep` | Which run. |
| `brief` | 1 when the run's prompt carried the sem briefing (`--brief`, sem arm only), else 0. Baseline rows are 0. Compare only rows with the same `brief`. |
| `dry_run` | 1 for `--dry-run` rows. |
| `input_tokens` | Uncached input tokens, summed over the run's requests (`usage.input_tokens`). |
| `output_tokens` | Output tokens, thinking included (`usage.output_tokens`). |
| `cache_read_tokens` | Input tokens served from the prompt cache (`usage.cache_read_input_tokens`). |
| `cache_write_tokens` | Input tokens written to the cache (`usage.cache_creation_input_tokens`). |
| `cost_usd` | Claude Code's `total_cost_usd`: a nominal list-price estimate, not a bill under a subscription. |
| `wall_time_s` | Wall time of the `claude` subprocess: model requests, tool execution, Claude Code and `sem mcp` startup. Excludes checkout preparation and scoring. |
| `api_time_s` | Claude Code's `duration_api_ms`: time spent waiting on the model. |
| `turns` | Claude Code's `num_turns`. |
| `tool_calls` | Tool calls the model made, all tools (`tool_use` blocks in the transcript). |
| `files_read` | Distinct files opened with the `Read` tool. Files printed through `Bash` (cat, sed) or matched by `Grep` are not counted. |
| `bytes_read` | Bytes of tool output returned to the model, all tools; what the model had to read. |
| `test_classes_executed` | Distinct (object, local test class) pairs that ran, not skipped, in the output of the agent's `Bash` calls. |
| `success_score` | B1: F1 of calling methods. B2: 1 if the mandatory parameter is in the signature and the hidden tests and full suite pass, else 0. B3: mean of entity F1 and caller recall (entity F1 alone when the rubric lists no callers). |
| `precision`, `recall` | B1: of calling methods (file + method). B3: of changed entities. Empty for B2. |
| `tests_passed` | B2: `npm run unit` exit 0 with every hidden test method run. |
| `callers_missed` | B2: call statements under `src/` that do not pass the new parameter by name. |
| `stop_reason` | The final result's stop reason. |
| `error` | Cap reached, max turns, timeout, sem mcp not connected, API error, refusal, missing ground truth, patch not applying. |

B1 line numbers are also scored, within ±2 lines (`line_precision`, `line_recall` in `results.jsonl`).
B1 answer items outside `src/` (abapGit's `test/src/`) are neither hits nor misses, because the
ground truth reads `src/` only (`TRUTH_SCOPE` in `scorers.py`; listed as `out_of_scope` in the score).

B3 matching (`scorers._matches`): an answer item matches a rubric entry when one of the entry's
aliases equals a whole identifier in the item. The identifier may carry a `zcl_abapgit_` /
`zif_abapgit_` / `zcx_abapgit_` prefix. Items naming a local test class (`ltcl_...->...`, or
`<class>->ltcl_...` since Gate 2) are neither hits nor misses. `caller_precision` is reported in `results.jsonl` but not scored, because
the rubric is a draft.

## Briefing

The baseline's sem arm never called a sem tool and still paid about 4,200 extra context tokens per
request for the tool schemas. `--brief` tests whether telling the agent about the tools changes
that. It adds this paragraph after the shared instructions, to the sem arm only:

> Besides the usual tools you have three sem tools for this code base: sem_find, sem_impact and sem_certify. Prefer them over grep for questions of where something is defined, who calls it, and what a change to it affects: sem_find looks entities up by name and lists their callers, sem_impact lists what depends on an entity and which tests to run, and sem_certify summarises what a commit or range changed.

This is the same lever sem's own benchmark calls a briefing. Briefed and unbriefed sem runs answer
different questions (does sem help when the agent is told to use it, vs. when it is left to find
the tools), so each gets its own rows (`brief` column) and its own run id; apply the adoption rule
to one `brief` value at a time, against the grep arm of the same checkpoint.

## Adoption rule

Adopt sem for ABAP agents at a checkpoint if, over all repetitions:

- **B1 precision** (mean over tasks) of the sem arm beats the grep arm by **20 points or more**, **or**
- **B2 wall time** (mean per task) of the sem arm is **30% or more lower** at **equal pass rate**
  (same `tests_passed` count), **and in either case**
- the sem arm's **input tokens** are **no more than 15% higher** than the grep arm's.

Measure input tokens as `input_tokens + cache_read_tokens + cache_write_tokens`, the tokens the model read. Cached tokens are cheaper, but they are still context.

## Baseline, 2026-10-05

Checkpoint `baseline`, one repetition, `claude-sonnet-5-5` at effort high through Claude Code 2.1.289.
sem 0.27.0 was built from this branch (`build` `ca85e72845b0`; `crates/` as at `9cc2bab`).
B3: 8 runs. B1: 20 runs. B2: not run (see Known issues). Cumulative cost: **$2.06** by Claude Code's figure.
Per-run means, except cost (summed):

| Class | Arm | Runs | Success | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|
| B1 | grep | 10 | no ground truth | 31,115 | 2,370 | 0.49 | 19.1 | 3.0 | - |
| B1 | sem | 10 | no ground truth | 46,117 (+48%) | 2,459 | 0.55 | 18.7 | 3.1 | 0 |
| B3 | grep | 4 | 1.00 | 118,153 | 3,917 | 0.55 | 34.6 | 6.8 | - |
| B3 | sem | 4 | 1.00 | 109,642 (-7%) | 3,785 | 0.47 | 39.6 | 5.2 | 0 |

"Tokens read" is `input_tokens + cache_read_tokens + cache_write_tokens`, as in the adoption rule.
No run hit an error, the turn bound or the timeout. The sem server was `connected`, with all eight
tools listed, in every sem run. `files_read` is 0 everywhere: both arms read code through `Bash`
(`git show`, `grep -n`, `sed -n`) and `Grep`, never the `Read` tool. Every B1 answer parsed. Its
number of reported calls was within one of the task file's grep call-site count for 9 of 10 targets
in both arms. Both arms reported 2 calls for `b1_10`, where the grep count is 4. The B3 rows were
scored before the B3 matching fix described under Output. Rescoring their stored answers with the fixed scorer gives
the same 1.0 for all eight.

**Reading.** Neither class separates the arms yet. B3 is at its ceiling: both arms find every
rubric entity and every rubric caller with `git show` and grep in five to eleven turns, so this
rubric cannot show a sem advantage. B1 has no ground truth to score against. The sem arm did worse
than "fall back to grep": in all 14 sem runs it never called a single sem tool, not even once to
find out that `sem_find` callers cannot resolve across files yet. It went straight to `Grep` and
`Bash` and still paid for the sem tool schemas and MCP instructions, about 4,200 extra tokens on
every request (first request 10,140 vs 5,913 tokens on `b1_05`). That puts its B1 tokens read 48%
above the grep arm, against the adoption rule's 15% ceiling, before sem contributes anything.

## Gate 2, 2026-10-06

Checkpoint `gate2`, one repetition, `--brief` (the sem arm's prompt carries the briefing; the grep
arm's does not), `claude-sonnet-5-5` at effort high through Claude Code 2.1.289. sem was built from
this branch at `f61bf50` (`build` `f61bf50a7f9c`, clean `crates/`; the binary still reports
`sem 0.27.0`). B1: 20 runs, B3: 8 runs, B2: 12 runs, all 40 planned. Cumulative cost: **$3.13** by
Claude Code's figure. No run hit an error, the turn bound or the timeout, and the sem server was
`connected` in every sem run. Per-run means, except cost (summed):

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| B1 | grep | 10 | 1.000 | 1.000 | 27,127 | 2,289 | 0.49 | 19.1 | 2.5 | - |
| B1 | sem (brief) | 10 | 1.000 | 1.000 | 40,369 (+49%) | 2,164 | 0.59 | 22.7 | 3.2 | 1.0 |
| B1 | baseline grep | 10 | 0.992 | 0.992 | 31,115 | 2,370 | 0.49 | 19.1 | 3.0 | - |
| B1 | baseline sem | 10 | 1.000 | 1.000 | 46,117 (+48%) | 2,459 | 0.55 | 18.7 | 3.1 | 0 |
| B2 | grep | 6 | 6/6 pass | - | 59,542 | 2,118 | 0.45 | 53.3 | 6.5 | - |
| B2 | sem (brief) | 6 | 6/6 pass | - | 91,419 (+54%) | 2,180 | 0.54 | 57.6 (+8%) | 7.0 | 1.5 |
| B2 | baseline | - | not run | | | | | | | |
| B3 | grep | 4 | 0.875 | 1.000 | 108,572 | 4,240 | 0.54 | 39.7 | 6.5 | - |
| B3 | sem (brief) | 4 | 1.000 | 1.000 | 123,847 (+14%) | 3,828 | 0.52 | 37.3 | 7.0 | 1.25 |
| B3 | baseline grep | 4 | 1.000 | 1.000 | 118,153 | 3,917 | 0.55 | 34.6 | 6.8 | - |
| B3 | baseline sem | 4 | 1.000 | 1.000 | 109,642 (-7%) | 3,785 | 0.47 | 39.6 | 5.2 | 0 |

Success and precision are from the scorers as fixed during this run (below), applied to the stored
answers of both checkpoints; the baseline's B1 rows carry no score in `results.csv`, these are its
answers scored now against `whereused.grep.json`. "sem calls" counts `sem_*` entries in
`calls_by_tool` (`results.jsonl`); `results.csv` has no column for it. The only B1 miss at either
checkpoint is the baseline grep arm's `b1_01`, which attributed a call in
`zcl_abapgit_repo_online` to `set_objects` instead of `check_for_valid_branch`. The only B3 miss at
Gate 2 is the grep arm's `b3_01`, which did not name `zcl_abapgit_transport_2_branch` among the
callers left behind (caller recall 0). B2 timings per task, grep vs sem (s): 56.7/64.4, 50.5/52.2,
52.4/61.6, 55.4/50.0, 57.4/64.5, 47.3/52.9; sem was faster on one task of six.

**Scorer fixes.** Two scorers were wrong on real answers; both are fixed in `scorers.py`, and the
stored rows in `results.csv` / `results.jsonl` keep the scores they were written with (the files
are append-only):

- B3: an item naming a local test class under its global class, `zcl_abapgit_repo_online->ltcl_create_branch`,
  counted as a wrong changed entity, because the test-class rule only matched items starting with
  `ltc`. Both arms listed it on `b3_03` (stored 0.8333, entity precision 0.5). Rescored: 1.0 in both arms.
- B1: the real call in `test/src/zcl_abapgit_sap_package_test.clas.testclasses.abap`
  (`check_list_subpackages`, `b1_04`) counted as a false positive, though the ground truth reads
  `src/` only and `docs/abap/census-gate2.md` lists that call as real and out of the truth's scope.
  Every arm at both checkpoints listed it (stored 0.9697, precision 0.9412). Answer items outside
  `src/` are now neither hits nor misses. Rescored: 1.0 in every arm.

Neither fix moves one arm against the other. Stored means were B1 0.997 in both arms and B3 0.833
(grep) vs 0.958 (sem).

**What the briefed sem arm did.** The briefing worked as an instruction: every sem run called a sem
tool, 25 calls in 20 runs, against none in the baseline. It was `sem_find` with `mode: "callers"`
in 22 calls, `sem_grep` 4 times on B2 (`sem_grep` is not named in the briefing), `sem_certify` once
(`b3_04`) and `sem_impact` never, not even on B2, where the briefing names it for "what a change
affects and which tests to run". On B1 and B2 the first turn always sent `sem_find` together with
`Grep` (or `sem_grep`) in the same message, then built the answer from the text search: call lines
come from grep, since `sem_find` reports each caller at its method's line, not the call's.
`sem_find` answered with callers in 8 of 10 B1 runs, and in all 8 its list covered every caller in
the ground truth. In 6 of them it said `complete`; on `b1_04` and `b1_08` it said `INCOMPLETE` and
added possible callers (same-named implementations, the name used as a value) that are not calls.
Six of those eight final answers mention `sem_find`, two of them to say it agreed with grep. In 7 of 22 calls it
answered "matches N definitions; pass file" (B1 `b1_06`, `b1_10`; B2 `b2_02`, `b2_03`, `b2_04`,
where the test class's own same-named test method is the second definition; B3 `b3_02`, `b3_03`),
and the agent retried with `file` once (`b3_02`) and otherwise went on with grep. On B3 `sem_find`
came in the second turn, after `git show`, to look up callers of a changed method; `sem_certify`
was called once, in parallel with `git show`, and the answer was written from both.

**Adoption rule** (`brief` = 1 against the grep arm of `gate2`):

- B1 precision: sem 1.000 vs grep 1.000, **+0 points** (needs +20). Not met.
- B2 wall time at equal pass rate (6/6 each): sem 57.6 s vs grep 53.3 s, **+8%** (needs -30%). Not met.
- Input tokens (tokens read): B1 **+49%**, B2 **+54%**, B3 +14% (ceiling +15%). Over on both
  classes the rule decides on.

**Verdict: drop** for this checkpoint. Neither criterion holds, so the rule's "in between,
per task" case does not arise; no single task shows sem ahead on both B1 precision and tokens, or
on B2 wall time by 30%.

**Reading.** Briefed, the agent did use sem: one `sem_find` per B1 run, one or two sem calls per B2
run, and every answer it gave was as correct as the grep arm's, so sem cost nothing in quality, and
where `sem_find` answered, its callers covered the ground truth. It did not use sem in place of
grep, though: it ran both side by side in the first turn and took its file and line answers from
the text search, so the sem calls added a cold index, the tool schemas and their output to the
context (+49% tokens on B1, +54% on B2) without removing a single grep. The benchmark also cannot
show a precision gain: grep scores 1.000 on B1 at this checkpoint (0.992 at baseline), because
abapGit's method names are distinctive enough that a case-insensitive grep plus reading the
enclosing method is already exact, so the 20-point B1 criterion is out of reach on these ten
targets. On B2 the edit-and-test loop dominates (both arms about 50 to 60 s, all twelve passing,
no caller missed) and caller lookup is a small part of it, so sem made the runs slightly slower,
not faster, and `sem_impact` (the tool meant for "which tests to run") was never called. The one
concrete sem defect the transcripts show is the "matches N definitions" answer on 7 of 22 lookups,
5 of them caused by a local test method that shares the target's name, which sent the agent back to
grep; a B1/B2 set where grep is ambiguous (common names, dynamic calls, interface dispatch) is what
it would take for this benchmark to show what sem resolves that grep cannot.

## Known issues

- **abapGit's libraries are pinned here, not by abapGit.** `abap_transpile` clones the libraries named in `test/abap_transpile.json` (`open-abap-core`, `open-abap-gui`, `open-abap-seo`, `express-icf-shim`, `abapGit-web-classic`) from their default branches on every build. On 2026-10-05 `open-abap-gui` changed (#188 to #195) and `npm run unit` started failing before any test ran (`Error: Void type: DISVARIANT` in `cl_alv_variant`), which made B2 unscorable. `abapgit-transpile-libs.json` now maps each library to its repository and its last commit before the pinned abapGit commit (2026-10-04T17:55Z). The harness clones each at that commit into `<work-dir>/libs/<name>` and rewrites `libs[]` in each checkout's `test/abap_transpile.json` from `url` to `folder`. The transpiler resolves `folder` as `path.join(cwd, folder)`, so an absolute path does not work; the harness writes it relative to the checkout. The libraries sit outside every checkout so agents' `grep` and sem's index never see them. The harness fails if the config names a library the pin file lacks, or the reverse. Re-pin on purpose only, and say so in the commit.
  With the pinned libraries `npm run unit` exits 0 on the pinned commit, 959 test methods in 169 local test classes, as on the morning of 2026-10-05.
- **abapGit has no `package-lock.json`.** abapGit gitignores it, so `npm ci` on the checkout as is fails with `EUSAGE`, and `npm run unit` then fails with `sh: 1: abap_transpile: not found` (exit 127). Fix used:
  1. `npm install --ignore-scripts` once in a scratch copy (12 s, 210 packages, every one resolved from registry.npmjs.org).
  2. That lockfile is pinned as `abapgit-package-lock.json`.
  3. The harness runs `npm ci --ignore-scripts` against it.

  Regenerate the lockfile only on purpose, and read its diff.
- **Earlier on 2026-10-05, `npm run unit` passed** at the pinned commit with that lockfile: exit 0, 35 s cold and 33 s warm, on Node v22.22.0 / npm 10.9.4. 959 test methods ran, in 169 local test classes of 111 objects; 74 were skipped by configuration or CRITICAL risk level. With a correct b2_01 solution plus its hidden tests, the suite ran 961 methods in 170 classes in 22 s and scored 1.0.
- **`npm run unit` no longer needs network once the checkout is prepared.** The libraries are cloned once, when `<work-dir>/base` is prepared. B2's prompt used to say the build needs network access; it now says how long the suite takes (B2 had not run, so no row is affected).
- **The transpiler only checks files in its `input_filter`.** That is 1038 of 1526 files. A caller in an excluded file that misses the new parameter does not break the build, so `callers_missed` is counted by grep over all of `src/` separately.
- **The usual abapGit clone is shallow** (1 commit). B3 needs history, so the base checkout fetches 300 commits from GitHub once when they are missing.
- **sem's ABAP support answers few B1 questions** in sem 0.27.0. This is a property of the build under test, not of the harness; it is what a baseline run measures. The dry run's tool check shows it. Probed through `sem mcp`:
  - `sem_find` with `mode: "callers"` lists only the defining class for every B1 target it knows, never the cross-file callers grep finds.
  - 4 of the 10 targets are not extracted at all: the interface methods `zif_abapgit_sap_package~list_subpackages` and `zif_abapgit_repo~get_files_local`, and the class methods `get_display_name` and `textarea`. For example, `sem_find` with `in: src/git/zcl_abapgit_git_branch_utils.clas.abap` lists 1 of that class's 3 methods.
- **`Bash` is not sandboxed.** Claude Code's Bash runs model-written commands as the current user in the run's checkout, with network and with the auth and proxy variables Claude Code itself needs. `--permission-mode dontAsk` and `--setting-sources ''` limit what is pre-approved, not what an allowed `Bash` call can do. Run real benchmarks in a container or VM.
- **The sem arm's index is cold in every run.** Each run is a fresh clone, so the first sem call pays for indexing; abapGit indexes in about 7 s. That cost is part of the sem arm's wall time, on purpose. Claude Code also starts `sem mcp` before the first request in every sem run, whether or not a sem tool is called. That startup is in `wall_time_s`.
- **`sem mcp` runs standalone** (`SEM_MCP_NO_SHARED=1`), with telemetry, update checks, cloud and network off. Each run has its own checkout, and the process ends with the run.
- **`files_read`** only sees the `Read` tool. An agent that reads with `Bash` or `Grep` looks like it read nothing, which is what both arms did in the baseline. `bytes_read` covers every tool.
- **`test_classes_executed`** parses `Bash` output. Claude Code truncates long Bash output, and the harness also reads the untruncated `tool_use_result` when the transcript carries it. An agent that pipes `npm run unit` through `tail` still hides most test classes, as before.
