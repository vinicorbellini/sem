# ABAP agent benchmark

Paired agent runs on a pinned commit of [abapGit](https://github.com/abapGit/abapGit)
(`b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`). Each task runs twice per repetition with the
same model, system prompt and task prompt:

- **grep arm**: `grep`, `glob`, `read_file` and `bash` over the checkout.
- **sem arm**: the same four, plus `sem_find`, `sem_impact` and `sem_certify` from `sem mcp`
  over stdio, with the schemas and descriptions the server lists (`crates/sem-mcp/src/tools.rs`,
  `docs/shared-mcp.md`).

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
Until it does, B1 rows carry `error = no ground truth yet` and an empty score. Convert sapcli's
output to:

```json
{"abapgit_commit": "b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0",
 "targets": {"b1_01": [{"method": "<class>-><method>", "file": "src/...", "line": 12}]}}
```

`b3_rubric.json` is a **draft**. It was written from `git show` and grep at the pinned commit and
cross-checked with `sem diff --commit`. Review it before scoring a real run.

## Running

```sh
cargo build --release --manifest-path crates/Cargo.toml -p sem-cli   # sem with ABAP (default features)
pip install anthropic
# fill in prices.json first
export ANTHROPIC_API_KEY=...

python3 bench/abap-agent/run.py --dry-run --arm both --class all --reps 1        # no model calls
python3 bench/abap-agent/run.py --checkpoint baseline --arm both --class all --cap-usd 50
python3 bench/abap-agent/run.py --checkpoint gate1 --class B1 --task b1_03 --reps 1
```

| Flag | Meaning |
|---|---|
| `--checkpoint baseline\|gate1\|gate2` | Written to every row; required for a real run. |
| `--arm grep\|sem\|both` | Which arms to run (default both). |
| `--class B1\|B2\|B3\|all` | Which task classes (default all); `--task b2_04` (repeatable) narrows to single tasks. |
| `--model` | Model id (default `claude-sonnet-5-5`); it needs a complete row in `prices.json`. |
| `--reps` | Repetitions per task per arm (default 3). |
| `--dry-run` | Everything except the model call. It prepares checkouts, starts `sem mcp`, builds each first request and prints it (written in full to `<work-dir>/requests/`), calls each tool once to show it works, and scores the untouched checkout. B2 scoring runs the unit suite, so expect about 30 s per B2 run. |
| `--cap-usd` | Stop before a request that would take the cumulative cost past the cap (see below). |
| `--abapgit` | abapGit clone to copy from (default `/tmp/claude-0/abapGit`). It is cloned, never modified. |
| `--work-dir` | Checkouts, request dumps, sem logs (default `/tmp/sem-abap-agent`). |
| `--sem-binary` | sem to serve MCP. By default `$SEM_BINARY`, then `crates/target/release/sem` in this checkout, then in the main checkout when this is a linked worktree. |
| `--keep` | Keep each run's checkout. |

The first run prepares `<work-dir>/base`:

1. Clone `--abapgit`.
2. Fetch 300 commits of history from GitHub when the B3 commits are missing (the usual clone is shallow).
3. Check out the pinned commit.
4. `npm ci --ignore-scripts` with `abapgit-package-lock.json` (see Known issues).

Each run clones `base` and symlinks its `node_modules`.

Model settings, the same for both arms (the `run.py` config block):

- `max_tokens` 16000 per request.
- Adaptive thinking.
- Effort `high`, set explicitly because the default differs by model.
- Automatic prompt caching (top-level `cache_control`).
- At most 80 requests per run.

The agent loop is the Anthropic SDK's tool runner (`client.beta.messages.tool_runner` with
`beta_tool`-wrapped functions); a tool error goes back to the model as an `is_error` result.
Server-side refusal fallbacks are deliberately **not** enabled. A silent switch to another model
would break the pairing, so a refusal is recorded in `stop_reason` and `error` instead.

**Cost.** Cost is usage multiplied by `prices.json` (USD per million tokens). That file ships with
`claude-sonnet-5-5` and `claude-opus-4-8` and every value null. A real run refuses to start while any
value for its model is null; a dry run only warns.

`--cap-usd` checks the first request of each run before it is sent, using the free token-count
endpoint and input price only. It checks each later request using the previous request's cost as
the estimate. Context only grows, so that estimate is low, and a run can overshoot the cap by
about one request.

## Output

`results.csv` is append-only, one row per run. A file with different columns is an error, never
rewritten. `results.jsonl` holds the same rows plus the prompt, final answer, per-tool call counts,
files read and full score details. Dry runs write `results.dry-run.csv` / `.jsonl` (gitignored). The repo's `.gitignore` ignores
`bench/`, so commit results with `git add -f bench/abap-agent/results.csv bench/abap-agent/results.jsonl`.

| Column | Meaning |
|---|---|
| `timestamp` | UTC, when the run finished. |
| `checkpoint` | `baseline`, `gate1` or `gate2` (`dry-run` for dry runs). |
| `build` | sem commit measured: `git rev-parse HEAD` of this repo, `-dirty` if `crates/` has uncommitted changes. Build the binary from that commit. |
| `sem_version` | `sem --version` of the binary that served MCP. |
| `abapgit_commit` | Pinned abapGit commit. |
| `model`, `arm`, `task_class`, `task_id`, `rep` | Which run. |
| `dry_run` | 1 for `--dry-run` rows. |
| `input_tokens` | Uncached input tokens, summed over the run's requests. |
| `output_tokens` | Output tokens (thinking included), summed. |
| `cache_read_tokens` | Input tokens served from the prompt cache, summed. |
| `cache_write_tokens` | Input tokens written to the cache, summed (priced as `cache_write`). |
| `cost_usd` | Token counts multiplied by `prices.json`; empty when prices are missing (dry run). |
| `wall_time_s` | Agent loop wall time: model requests plus tool execution. Excludes checkout preparation and scoring. |
| `turns` | Model requests in the run. |
| `tool_calls` | Tool calls the model made, all tools. |
| `files_read` | Distinct files opened with `read_file`. Files printed through `bash` (cat, sed) are not counted. |
| `bytes_read` | Bytes of tool output returned to the model, all tools; what the model had to read. |
| `test_classes_executed` | Distinct (object, local test class) pairs that ran, not skipped, in the output of the agent's `bash` calls. |
| `success_score` | B1: F1 of calling methods. B2: 1 if the mandatory parameter is in the signature and the hidden tests and full suite pass, else 0. B3: mean of entity F1 and caller recall (entity F1 alone when the rubric lists no callers). |
| `precision`, `recall` | B1: of calling methods (file + method). B3: of changed entities. Empty for B2. |
| `tests_passed` | B2: `npm run unit` exit 0 with every hidden test method run. |
| `callers_missed` | B2: call statements under `src/` that do not pass the new parameter by name. |
| `stop_reason` | Last response's stop reason. |
| `error` | Cap reached, max turns, API error, refusal, missing ground truth, patch not applying. |

B1 line numbers are also scored, within ±2 lines (`line_precision`, `line_recall` in `results.jsonl`).

## Adoption rule

Adopt sem for ABAP agents at a checkpoint if, over all repetitions:

- **B1 precision** (mean over tasks) of the sem arm beats the grep arm by **20 points or more**, **or**
- **B2 wall time** (mean per task) of the sem arm is **30% or more lower** at **equal pass rate**
  (same `tests_passed` count), **and in either case**
- the sem arm's **input tokens** are **no more than 15% higher** than the grep arm's.

Measure input tokens as `input_tokens + cache_read_tokens + cache_write_tokens`, the tokens the model read. Cached tokens are cheaper, but they are still context.

## Known issues

- **`npm ci` fails on abapGit as checked out.** abapGit gitignores `package-lock.json`. Measured here on 2026-10-05:

  ```
  npm error code EUSAGE
  npm error The `npm ci` command can only install with an existing package-lock.json or
  npm error npm-shrinkwrap.json with lockfileVersion >= 1. Run an install with npm@5 or
  npm error later to generate a package-lock.json file, then try again.
  ```

  `npm run unit` then fails with `sh: 1: abap_transpile: not found` (exit 127). Fix used:
  1. `npm install --ignore-scripts` once in a scratch copy (12 s, 210 packages, every one resolved from registry.npmjs.org).
  2. That lockfile is pinned as `abapgit-package-lock.json`.
  3. The harness runs `npm ci --ignore-scripts` against it.

  Regenerate the lockfile only on purpose, and read its diff.
- **`npm run unit` passes** at the pinned commit with that lockfile: exit 0, 35 s cold and 33 s warm, on Node v22.22.0 / npm 10.9.4. 959 test methods ran, in 169 local test classes of 111 objects; 74 were skipped by configuration or CRITICAL risk level. With a correct b2_01 solution plus its hidden tests, the suite runs 961 methods in 170 classes in 22 s and scores 1.0.
- **`npm run unit` needs network on every build.** `abap_transpile` clones `open-abap-core`, `open-abap-gui`, `open-abap-seo`, `express-icf-shim` and `abapGit-web-classic` from GitHub (`test/abap_transpile.json`). Without GitHub access, B2 scoring and the agents' own test runs fail.
- **The transpiler only checks files in its `input_filter`.** That is 1038 of 1526 files. A caller in an excluded file that misses the new parameter does not break the build, so `callers_missed` is counted by grep over all of `src/` separately.
- **The usual abapGit clone is shallow** (1 commit). B3 needs history, so the base checkout fetches 300 commits from GitHub once.
- **sem's ABAP support answers few B1 questions** in the binary used for the first dry run (sem 0.27.0, the main checkout's release build, with lang-abap; its timestamp predates commit `2618ff1` by minutes, so it was built from that change's working tree). This is a property of the build under test, not of the harness; it is what a baseline run measures. The dry run's tool check shows it (`sem_find` / `sem_impact` errors). Probed through `sem mcp`:
  - `sem_find` with `mode: "callers"` lists only the defining class for every B1 target it knows, never the cross-file callers grep finds.
  - 4 of the 10 targets are not extracted at all: the interface methods `zif_abapgit_sap_package~list_subpackages` and `zif_abapgit_repo~get_files_local`, and the class methods `get_display_name` and `textarea`. For example, `sem_find` with `in: src/git/zcl_abapgit_git_branch_utils.clas.abap` lists 1 of that class's 3 methods.
- **`bash` is not sandboxed.** It runs model-written commands as the current user in the run's checkout, with network. `read_file`, `grep` and `glob` refuse paths outside the checkout; `bash` cannot. Run real benchmarks in a container or VM. `ANTHROPIC_*` variables are removed from its environment.
- **The sem arm's index is cold in every run.** Each run is a fresh clone, so the first sem call pays for indexing; abapGit indexes in about 7 s. That cost is part of the sem arm's wall time, on purpose.
- **`sem mcp` runs standalone** (`SEM_MCP_NO_SHARED=1`), with telemetry, update checks, cloud and network off. Each run has its own checkout, and the process ends with the run.
- **`files_read`** only sees `read_file`; an agent that reads with `bash` looks like it read nothing. `bytes_read` covers every tool.
