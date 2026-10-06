#!/usr/bin/env python3
"""ABAP agent benchmark: Claude Code with its built-in tools vs the same plus sem's MCP tools, on abapGit.

Paired agent runs on a pinned commit of abapGit: same model, same prompt, both through
headless Claude Code (`claude -p`). One arm gets the built-in Bash, Read, Grep and Glob
("grep"), the other those plus the tools of `sem mcp` attached with --mcp-config ("sem"). A third arm, "cli"
(Gate 2d), has the grep arm's tools and the sem binary on PATH, to run `sem find` through Bash.
Task classes: B1 where-used, B1A where-used on ambiguous names, B2 change-and-verify, B3 review, and C1 where-used
on a Rust repository (the control; a task file's `repo` names the repository). Every run gets a fresh checkout and
appends one row to results.csv.

Usage:
    python3 bench/abap-agent/run.py --checkpoint baseline --arm both --class B3 --reps 1
    python3 bench/abap-agent/run.py --dry-run --arm both --class all --reps 1

Dependencies: Claude Code (`claude` on PATH, logged in), git, node + npm, a release build of sem
"""

import argparse
import csv
import json
import os
import shutil
import signal
import subprocess
import sys
import time
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import scorers  # noqa: E402
import tools  # noqa: E402

# ── Config ──────────────────────────────────────────────────────────────────

MODEL = "claude-sonnet-5-5"
EFFORT = "high"            # set explicitly: the default differs by model
MAX_TURNS = 40             # claude --max-turns: a safety bound, not a target
RUN_TIMEOUT_S = {"B1": 1800, "B1A": 1800, "B2": 3600, "B3": 1800, "C1": 1800}   # wall clock per run; the process group is killed
REPS = 3
CHECKPOINTS = ("baseline", "gate1", "gate2", "gate2b", "gate2c", "control", "gate2d")
ARMS = ("grep", "sem")     # --arm both; `--arm cli` runs the cli arm alone
CLASSES = ("B1", "B1A", "B2", "B3", "C1")
SCORED_AS_B1 = ("B1", "B1A", "C1")   # where-used: score_b1 against the task file's ground_truth

BENCH_DIR = Path(__file__).resolve().parent
REPO_DIR = BENCH_DIR.parent.parent
SEM_BINARY = REPO_DIR / "crates" / "target" / "release" / "sem"
ABAPGIT_SOURCE = "/tmp/claude-0/abapGit"
ABAPGIT_REMOTE = "https://github.com/abapGit/abapGit.git"
WORK_DIR = Path("/tmp") / "sem-abap-agent"
PRICES_PATH = BENCH_DIR / "prices.json"
LOCKFILE = BENCH_DIR / "abapgit-package-lock.json"
RESULTS_CSV = BENCH_DIR / "results.csv"
RESULTS_JSONL = BENCH_DIR / "results.jsonl"
DRY_RUN_CSV = BENCH_DIR / "results.dry-run.csv"
DRY_RUN_JSONL = BENCH_DIR / "results.dry-run.jsonl"

TASK_FILES = {
    "B1": BENCH_DIR / "tasks" / "b1_whereused.json",
    "B1A": BENCH_DIR / "tasks" / "b1a_whereused.json",
    "B2": BENCH_DIR / "tasks" / "b2_change.json",
    "B3": BENCH_DIR / "tasks" / "b3_review.json",
    "C1": BENCH_DIR / "tasks" / "c1_whereused.json",
}

# Identical for both arms, and sent as the head of the prompt: `claude -p` takes one prompt,
# and Claude Code's own system prompt stays in place. The arms differ only in their tools.
INSTRUCTIONS = (
    "You are a software engineer working in {code_base} checked out on disk. "
    "Use the tools to inspect and, when the task asks for it, change the repository. "
    "Work until the task is complete, then give your final answer in the format the task asks for."
)

# --brief: one paragraph added to the sem arm's prompt only (the grep arm's prompt never changes).
# Without it the baseline's sem arm never called a sem tool and still paid for the tool schemas.
BRIEF = (
    "Besides the usual tools you have three sem tools for this code base: sem_find, sem_impact and "
    "sem_certify. Prefer them over grep for questions of where something is defined, who calls it, "
    "and what a change to it affects: sem_find looks entities up by name and lists their callers, "
    "sem_impact lists what depends on an entity and which tests to run, and sem_certify summarises "
    "what a commit or range changed."
)

# --brief sem-first: the same for the sem arm, but where-used questions go to sem_find first and grep only
# checks what it names. Gate 2's briefed sem arm ran grep next to sem_find, so sem's calls only added context.
BRIEF_SEM_FIRST = (
    "Besides the usual tools you have three sem tools for this code base: sem_find, sem_impact and "
    "sem_certify. For where-used questions (who calls a method) call sem_find with mode \"callers\" first "
    "and use its answer as the caller list, passing file with the defining file the task names and the bare method name (an interface method zif_x~m is the entity m in the interface's file). Use Grep only if sem_find returns an error or says INCOMPLETE, "
    "and then only to check the possible callers it names, not to search the code base again. When sem_find "
    "reports the line of each call, use those lines; otherwise read the calling method's range for the line. "
    "For other questions, sem_impact lists what depends on an entity and which tests to run, and "
    "sem_certify summarises what a commit or range changed."
)

# --brief sem-find-only: BRIEF_SEM_FIRST for a server that lists sem_find alone (--mcp-tools sem_find, Gate 2c),
# so it names neither sem_impact nor sem_certify, which that run does not have.
BRIEF_SEM_FIND_ONLY = (
    "Besides the usual tools you have a sem tool for this code base: sem_find. "
    "For where-used questions (who calls a method) call sem_find with mode \"callers\" first "
    "and use its answer as the caller list, passing file with the defining file the task names and the bare method name (an interface method zif_x~m is the entity m in the interface's file). Use Grep only if sem_find returns an error or says INCOMPLETE, "
    "and then only to check the possible callers it names, not to search the code base again. When sem_find "
    "reports the line of each call, use those lines; otherwise read the calling method's range for the line."
)
# --brief cli: for the cli arm (--arm cli, Gate 2d), which has no sem tool but `sem` on PATH; BRIEF_SEM_FIRST's
# where-used advice, as a command run through Bash.
BRIEF_CLI = (
    "Besides the usual tools you have the sem command line on PATH for this code base. "
    "For where-used questions (who calls a method) run `sem find NAME --callers --file FILE` in Bash first, "
    "passing the bare method name as NAME and the defining file the task names as FILE (an interface method zif_x~m "
    "is the entity m in the interface's file), and use its resolved callers and their call lines as the caller list. "
    "Use Grep only if sem fails or says INCOMPLETE, "
    "and then only to check the possible callers it names, not to search the code base again."
)
BRIEFS = {"1": BRIEF, "sem-first": BRIEF_SEM_FIRST, "sem-find-only": BRIEF_SEM_FIND_ONLY,
          "cli": BRIEF_CLI}   # the `brief` column's values; "0" is no briefing

CSV_COLUMNS = [
    "timestamp", "checkpoint", "build", "sem_version", "abapgit_commit", "model", "arm", "brief",
    "task_class", "task_id", "rep", "dry_run",
    "input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens", "cost_usd",
    "wall_time_s", "api_time_s", "turns", "tool_calls", "files_read", "bytes_read", "test_classes_executed",
    "success_score", "precision", "recall", "tests_passed", "callers_missed",
    "stop_reason", "error", "mcp_tools",
]

# ── Helpers ──────────────────────────────────────────────────────────────────


def run(cmd: list[str], cwd: Path | str | None = None, check: bool = True) -> str:
    result = subprocess.run(cmd, capture_output=True, text=True, cwd=cwd)
    if check and result.returncode != 0:
        print(f"  Command failed: {' '.join(cmd)}", file=sys.stderr)
        print(f"  stderr: {result.stderr[:500]}", file=sys.stderr)
        sys.exit(1)
    return result.stdout


def load_tasks(task_class: str) -> dict:
    return json.loads(TASK_FILES[task_class].read_text())


def resolve_sem_binary(flag: str | None) -> Path:
    """--sem-binary, $SEM_BINARY, this checkout's release build, then the main checkout's (linked worktrees)."""
    candidates = [flag, os.environ.get("SEM_BINARY"), SEM_BINARY]
    common = run(["git", "-C", str(REPO_DIR), "rev-parse", "--path-format=absolute", "--git-common-dir"], check=False).strip()
    if common:
        candidates.append(Path(common).parent / "crates" / "target" / "release" / "sem")
    for c in candidates:
        if c and Path(c).is_file():
            return Path(c).resolve()
    print(f"sem binary not found (tried {[str(c) for c in candidates if c]}).\n"
          f"  Fix: cargo build --release --manifest-path {REPO_DIR / 'crates' / 'Cargo.toml'} -p sem-cli")
    sys.exit(1)


def sem_build(sem_binary: Path) -> tuple[str, str]:
    """The sem commit this run measures (REPO_DIR HEAD, -dirty if crates/ has changes) and the binary's version."""
    sha = run(["git", "-C", str(REPO_DIR), "rev-parse", "--short=12", "HEAD"]).strip()
    if run(["git", "-C", str(REPO_DIR), "status", "--porcelain", "--", "crates"]).strip():
        sha += "-dirty"
    version = run([str(sem_binary), "--version"]).strip()
    return sha, version


# ── Prices (cross-check only) ────────────────────────────────────────────────


def load_prices(model: str) -> dict | None:
    """USD per million tokens for `model`, to cross-check Claude Code's total_cost_usd. None when incomplete."""
    prices = json.loads(PRICES_PATH.read_text()).get("models", {}).get(model)
    if not prices or any(prices.get(k) is None for k in ("input", "output", "cache_read", "cache_write")):
        print(f"NOTE: prices.json has no complete row for {model}; no price cross-check.")
        return None
    return prices


def list_price_usd(usage: dict, prices: dict | None) -> float | None:
    if prices is None:
        return None
    return round((usage["input_tokens"] * prices["input"]
                  + usage["output_tokens"] * prices["output"]
                  + usage["cache_read_tokens"] * prices["cache_read"]
                  + usage["cache_write_tokens"] * prices["cache_write"]) / 1_000_000, 6)


# ── abapGit checkouts ────────────────────────────────────────────────────────


def has_commit(repo: Path, sha: str) -> bool:
    return subprocess.run(["git", "-C", str(repo), "cat-file", "-e", f"{sha}^{{commit}}"],
                          capture_output=True).returncode == 0


def prepare_base(work_dir: Path, source: str, commit: str, needed: list[str], depth: int, b2: bool) -> Path:
    """One pristine checkout at `commit`, with history back to every B3 commit; for B2, node_modules from the pinned lockfile."""
    base = work_dir / "base"
    if not base.exists():
        print(f"Preparing abapGit base checkout in {base} (from {source})")
        work_dir.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", "-q", source, str(base)])
    missing = [c for c in [commit, *needed] if not has_commit(base, c)]
    if missing:
        # The usual abapGit clone is shallow; B3 needs the reviewed commits in history.
        print(f"  Fetching {depth} commits of history from {ABAPGIT_REMOTE} (missing {len(missing)} commits)")
        run(["git", "-C", str(base), "fetch", "-q", f"--deepen={depth}", ABAPGIT_REMOTE, "main"])
        still = [c for c in missing if not has_commit(base, c)]
        if still:
            print(f"ERROR: abapGit commits not found after fetching: {still}")
            sys.exit(1)
    run(["git", "-C", str(base), "checkout", "-q", "--detach", commit])
    if not b2:
        return base
    tools.clone_pinned_libs(work_dir / tools.LIBS_SUBDIR)
    if not (base / "node_modules").exists():
        print("  Installing abapGit's npm dependencies from the pinned lockfile")
        shutil.copy(LOCKFILE, base / "package-lock.json")
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], cwd=base)
    return base


def prepare_repo_base(work_dir: Path, repo: dict) -> Path:
    """A task file's `repo` ({"source", "commit"}): one checkout of that commit alone, no other refs or history.

    A shallow single-branch clone of the source's HEAD, which must be the commit (a detached worktree of it), so
    an agent's `git log --all` cannot reach the benchmark's own branches and ground truth. Only tracked files
    are copied, so a build's target/ never is.
    """
    base = work_dir / f"base-{repo['commit'][:12]}"
    if not base.exists():
        print(f"Preparing base checkout in {base} (from {repo['source']})")
        work_dir.mkdir(parents=True, exist_ok=True)
        run(["git", "clone", "-q", "--depth", "1", "--single-branch", "--no-tags",
             f"file://{Path(repo['source']).resolve()}", str(base)])
    head = run(["git", "-C", str(base), "rev-parse", "HEAD"]).strip()
    if head != repo["commit"]:
        print(f"ERROR: {base} is at {head}, the task file pins {repo['commit']}; check out the commit in {repo['source']}")
        sys.exit(1)
    if (base / "target").exists() or any(base.glob("crates/target")):
        print(f"ERROR: {base} contains a target/ directory")
        sys.exit(1)
    return base


def make_workspace(base: Path, work_dir: Path, run_id: str, commit: str, task_class: str) -> Path:
    workspace = work_dir / "runs" / run_id
    if workspace.exists():
        shutil.rmtree(workspace)
    workspace.parent.mkdir(parents=True, exist_ok=True)
    run(["git", "clone", "-q", "--no-hardlinks", str(base), str(workspace)])
    run(["git", "-C", str(workspace), "checkout", "-q", "--detach", commit])
    if task_class == "B2":   # abapGit's build: npm dependencies and the pinned transpiler libraries
        (workspace / "node_modules").symlink_to(base / "node_modules")
        tools.pin_transpile_libs(workspace, work_dir / tools.LIBS_SUBDIR)
        # abapGit's .gitignore has "node_modules/", which does not match a symlink.
        with open(workspace / ".git" / "info" / "exclude", "a") as f:
            f.write("node_modules\n")
    return workspace


# ── Prompts ──────────────────────────────────────────────────────────────────


def build_prompt(spec: dict, task: dict, brief: str | None = None) -> str:
    """Both arms get the same text; `brief` (the sem arm only, a key of BRIEFS) adds that sem briefing paragraph."""
    fields = dict(task)
    if spec["class"] == "B3":
        fields["head"] = spec["abapgit_commit"][:12]
    head = INSTRUCTIONS.format(code_base=spec.get("code_base", "an ABAP code base")) + "\n\n" + (BRIEFS[brief] + "\n\n" if brief else "")
    return head + spec["prompt_template"].format(**fields)


# ── Agent run ────────────────────────────────────────────────────────────────


class Budget:
    """--cap-usd, fed by Claude Code's total_cost_usd.

    No run starts once the cumulative cost has reached the cap, and each run gets the rest
    of the cap as `claude --max-budget-usd`, so one run can pass the cap by at most one
    request.
    """

    def __init__(self, cap: float | None):
        self.cap = cap
        self.spent = 0.0
        self.exhausted = False

    def remaining(self) -> float | None:
        return None if self.cap is None else max(self.cap - self.spent, 0.0)

    def allows_run(self) -> bool:
        if self.cap is not None and self.spent >= self.cap:
            self.exhausted = True
        return not self.exhausted


def run_agent(cmd: list[str], workspace: Path, transcript: Path, timeout_s: int) -> dict:
    """One `claude -p` run in the checkout. The stream-json transcript is saved as it arrives."""
    out = {"stop_reason": None, "final_text": "", "error": None, "timed_out": False}
    with open(transcript, "w") as f:
        proc = subprocess.Popen(cmd, cwd=workspace, env=tools.claude_env(), stdin=subprocess.DEVNULL,
                                stdout=f, stderr=subprocess.PIPE, text=True, start_new_session=True)
        try:
            _, stderr = proc.communicate(timeout=timeout_s)
        except subprocess.TimeoutExpired:
            os.killpg(proc.pid, signal.SIGKILL)  # claude and everything it started (sem mcp, npm)
            _, stderr = proc.communicate()
            out["timed_out"] = True
    out["exit_code"] = proc.returncode
    out["stderr_tail"] = (stderr or "")[-1000:]
    return out


def agent_outcome(out: dict, stats: tools.RunStats, timeout_s: int) -> dict:
    result = stats.result
    out["final_text"] = result.get("result") or ""
    out["stop_reason"] = result.get("stop_reason") or result.get("subtype")
    subtype = result.get("subtype")
    if out["timed_out"]:
        out["error"] = f"timeout ({timeout_s} s)"
    elif not result:
        out["error"] = f"no result event (claude exit {out['exit_code']}): {out['stderr_tail'][-300:]}"
    elif subtype == "error_max_turns":
        out["error"] = f"max turns ({MAX_TURNS})"
    elif subtype == "error_max_budget_usd":
        out["error"] = "cap: --max-budget-usd reached"
    elif result.get("is_error") or subtype != "success":
        out["error"] = f"{subtype}: {str(result.get('api_error_status') or out['final_text'])[:300]}"
    elif stats.result.get("stop_reason") in ("max_tokens", "refusal"):
        out["error"] = stats.result["stop_reason"]
    return out


# ── Scoring ──────────────────────────────────────────────────────────────────


def score(spec: dict, task: dict, answer: str, workspace: Path, base: Path, scratch: Path) -> dict:
    if spec["class"] in SCORED_AS_B1:   # same scorer and ground-truth schema
        truth = scorers.load_whereused(BENCH_DIR / spec["ground_truth"], task["id"])
        return scorers.score_b1(answer, truth, scope=spec.get("scope", scorers.TRUTH_SCOPE))
    if spec["class"] == "B2":
        return scorers.score_b2(task, workspace, base, BENCH_DIR / spec["hidden_tests_dir"], scratch)
    rubric = scorers.load_rubric(BENCH_DIR / spec["rubric"], task["id"])
    return scorers.score_b3(answer, rubric)


# ── Output ───────────────────────────────────────────────────────────────────


def append_row(path: Path, row: dict):
    """Append-only. A file whose header differs from CSV_COLUMNS is an error, never rewritten.

    Columns are only ever added at the end: a header that is a prefix of CSV_COLUMNS gets the new
    names on its first line, and the rows already there are left as they are (csv reads the
    missing fields as empty).
    """
    if path.exists() and path.stat().st_size > 0:
        with open(path, newline="") as f:
            header = next(csv.reader(f), [])
        if header != CSV_COLUMNS and header == CSV_COLUMNS[:len(header)]:
            with open(path, newline="") as f:
                f.readline()
                rest = f.read()
            with open(path, "w", newline="") as f:
                csv.writer(f).writerow(CSV_COLUMNS)   # the line ending csv wrote the rows with
                f.write(rest)
            header = CSV_COLUMNS
        if header != CSV_COLUMNS:
            print(f"ERROR: {path} has different columns than this harness; start a new file.")
            sys.exit(1)
        new = False
    else:
        new = True
    with open(path, "a", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=CSV_COLUMNS)
        if new:
            writer.writeheader()
        writer.writerow({k: row.get(k, "") for k in CSV_COLUMNS})


def append_jsonl(path: Path, record: dict):
    with open(path, "a") as f:
        f.write(json.dumps(record, default=str) + "\n")


def check_sem_cli(sem_binary: Path, workspace: Path, task: dict) -> list[str]:
    """--dry-run, cli arm: the command BRIEF_CLI names, run once in the checkout (a cold index)."""
    target = task.get("method") or task.get("target")
    method = target.replace("=>", "->").split("->")[-1].split("~")[-1].split("::")[-1]
    cmd = [str(sem_binary), "find", method, "--callers", "--file", task["defined_in"]]
    start = time.time()
    r = subprocess.run(cmd, cwd=workspace, capture_output=True, text=True, env={**os.environ, **tools.SEM_ENV})
    return [f"{' '.join(cmd[1:])}: exit {r.returncode}, {len(r.stdout.encode())} bytes, {time.time() - start:.1f}s"]


def check_sem_mcp(sem_binary: Path, workspace: Path, log_path: Path, spec: dict, task: dict,
                  mcp_tools: str | None = None) -> list[str]:
    """--dry-run: start `sem mcp` as the MCP config does, list its tools and call sem_find once."""
    target = task.get("method") or task.get("target")
    method = target.replace("=>", "->").split("->")[-1].split("~")[-1].split("::")[-1] if target else None
    lines = []
    try:
        mcp = tools.SemMcp(str(sem_binary), workspace, log_path, mcp_tools)
    except Exception as e:
        return [f"sem mcp: ERROR {str(e)[:200]}"]
    try:
        lines.append(f"sem mcp lists: {[t['name'] for t in mcp.list_tools()]}")
        start = time.time()
        if method:
            query = {"query": method, "mode": "callers"}
            if spec.get("repo") and task.get("defined_in"):   # C1: as the sem-first briefing says, with the defining file
                query["file"] = task["defined_in"]
            output = mcp.call("sem_find", query)
            lines.append(f"sem_find callers {method}: {len(output.encode())} bytes, {time.time() - start:.1f}s")
        if spec["class"] == "B3":
            output = mcp.call("sem_certify", {"range": f"{task['commit']}~1..{task['commit']}"})
            lines.append(f"sem_certify: {len(output.encode())} bytes")
    except RuntimeError as e:
        # tools.SemMcp.call raises RuntimeError for a tool result with isError: the server answered,
        # e.g. "matches 2 definitions" or "no entity named ..." (what an agent would see too).
        lines.append(f"sem_find/sem_certify answered with a tool error: {str(e).splitlines()[0][:160]}")
    except Exception as e:
        lines.append(f"sem mcp: ERROR {str(e)[:200]}")
    finally:
        mcp.close()
    return lines


# ── Main ─────────────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--checkpoint", choices=CHECKPOINTS, help="Which checkpoint these runs measure.")
    parser.add_argument("--arm", choices=("grep", "sem", "cli", "both"), default="both")
    parser.add_argument("--class", dest="task_class", choices=(*CLASSES, "all"), default="all")
    parser.add_argument("--model", default=MODEL)
    parser.add_argument("--reps", type=int, default=REPS)
    parser.add_argument("--task", action="append", help="Only this task id (repeatable), e.g. b1_03.")
    parser.add_argument("--brief", nargs="?", const="1", choices=tuple(BRIEFS),
                        help="Add a sem briefing paragraph to the sem arm's prompt; the grep arm is unchanged. "
                             "Plain --brief is BRIEF, `sem-first` is BRIEF_SEM_FIRST, `sem-find-only` is "
                             "BRIEF_SEM_FIND_ONLY (with --mcp-tools sem_find), `cli` is BRIEF_CLI (with --arm cli). "
                             "Recorded in the `brief` column: never mix briefings or briefed and unbriefed rows.")
    parser.add_argument("--mcp-tools", metavar="NAMES",
                        help="Comma-separated tools the sem arm's server lists (SEM_MCP_TOOLS), e.g. sem_find; "
                             "default all. Recorded in the `mcp_tools` column.")
    parser.add_argument("--dry-run", action="store_true",
                        help="Do everything except calling claude; print each run's command line and prompt.")
    parser.add_argument("--cap-usd", type=float, default=30.0,
                        help="Stop when Claude Code's cumulative total_cost_usd reaches this (default 30, the agreed benchmark cap).")
    parser.add_argument("--abapgit", default=ABAPGIT_SOURCE, help="abapGit clone to copy checkouts from.")
    parser.add_argument("--work-dir", type=Path, default=WORK_DIR)
    parser.add_argument("--sem-binary")
    parser.add_argument("--keep", action="store_true", help="Keep each run's checkout.")
    args = parser.parse_args()

    if not args.dry_run and not args.checkpoint:
        parser.error("--checkpoint is required for a real run")
    if args.brief == "sem-find-only" and args.mcp_tools != "sem_find":
        parser.error("--brief sem-find-only names sem_find alone: pass --mcp-tools sem_find")
    if (args.brief == "cli") != (args.arm == "cli"):
        parser.error("--arm cli runs with --brief cli, and --brief cli is for --arm cli alone")
    if not args.dry_run and not shutil.which(tools.CLAUDE):
        print("Claude Code not found: `claude` must be on PATH and logged in.")
        sys.exit(1)
    checkpoint = args.checkpoint or "dry-run"
    arms = ARMS if args.arm == "both" else (args.arm,)
    classes = CLASSES if args.task_class == "all" else (args.task_class,)
    for c in classes:
        if not TASK_FILES[c].is_file():
            print(f"Class {c} skipped: {TASK_FILES[c].relative_to(BENCH_DIR)} does not exist")
    classes = tuple(c for c in classes if TASK_FILES[c].is_file())
    if not classes:
        return
    work_dir = args.work_dir.resolve()

    prices = load_prices(args.model)
    sem_binary = resolve_sem_binary(args.sem_binary)
    build, sem_version = sem_build(sem_binary)
    claude_version = run([tools.CLAUDE, "--version"], check=False).strip() if shutil.which(tools.CLAUDE) else "-"
    specs = {c: load_tasks(c) for c in classes}
    # A task file with `repo` names its own repository and commit; the others run on the pinned abapGit commit.
    bases, commits = {}, {}
    abap = [c for c in classes if "repo" not in specs[c]]
    if abap:
        commit = specs[abap[0]]["abapgit_commit"]
        assert all(specs[c]["abapgit_commit"] == commit for c in abap), "task files pin different abapGit commits"
        b3 = load_tasks("B3")
        base = prepare_base(work_dir, args.abapgit, commit,
                            [t["commit"] for t in b3["tasks"]] if "B3" in classes else [], b3["history_depth"],
                            b2="B2" in classes)
        bases.update({c: base for c in abap})
        commits.update({c: commit for c in abap})
    for c in classes:
        if "repo" in specs[c]:
            bases[c] = prepare_repo_base(work_dir, specs[c]["repo"])
            commits[c] = specs[c]["repo"]["commit"]

    plan = [(c, t, rep) for c in classes for t in specs[c]["tasks"]
            if not args.task or t["id"] in args.task for rep in range(args.reps)]
    csv_path, jsonl_path = (DRY_RUN_CSV, DRY_RUN_JSONL) if args.dry_run else (RESULTS_CSV, RESULTS_JSONL)

    print(f"ABAP agent benchmark: grep vs sem on {', '.join(sorted({f'{commits[c][:12]} ({c})' for c in classes}))}, "
          f"through Claude Code ({claude_version})")
    print(f"Model: {args.model} | Checkpoint: {checkpoint} | Build: {build} ({sem_version}) | sem: {sem_binary}")
    print(f"Classes: {', '.join(classes)} | Tasks: {len({(c, t['id']) for c, t, _ in plan})} | "
          f"Reps: {args.reps} | Arms: {', '.join(arms)} | Runs: {len(plan) * len(arms)} | Cap: ${args.cap_usd}")
    if args.dry_run:
        print("DRY RUN: claude is not called; scoring runs on the untouched checkouts")
    print()

    budget = Budget(args.cap_usd)
    summary: dict[tuple[str, str], list] = {}
    for sub in ("transcripts", "mcp"):
        (work_dir / sub).mkdir(parents=True, exist_ok=True)

    for task_class, task, rep in plan:
        spec = specs[task_class]
        # Alternate the arm order by repetition so drift (cache warmth, rate limits) is not one-sided.
        order = arms if rep % 2 == 0 else tuple(reversed(arms))
        for arm in order:
            if not args.dry_run and not budget.allows_run():
                break
            brief = args.brief if arm in ("sem", "cli") else None
            mcp_tools = args.mcp_tools if arm == "sem" else None
            run_id = f"{checkpoint}-{args.model}-{arm}{'-brief' if brief == '1' else f'-{brief}' if brief else ''}-{task['id']}-r{rep}"
            print(f"── {task['id']} [{arm}] rep {rep} ──")
            base, commit = bases[task_class], commits[task_class]
            workspace = make_workspace(base, work_dir, run_id, commit, task_class)
            mcp_path = None
            sem_log = work_dir / f"{run_id}.sem-mcp.log"
            if arm == "sem":
                mcp_path = work_dir / "mcp" / f"{run_id}.json"
                mcp_path.write_text(json.dumps(tools.mcp_config(str(sem_binary), workspace, sem_log, mcp_tools), indent=2))
            prompt = build_prompt(spec, task, brief=brief)
            session_id = str(uuid.uuid4())
            cmd = tools.claude_command(prompt, args.model, EFFORT, MAX_TURNS, arm, writes=(task_class == "B2"),
                                       session_id=session_id, mcp_config_path=mcp_path,
                                       budget_usd=None if args.dry_run else budget.remaining(),
                                       sem_binary=str(sem_binary))
            transcript = work_dir / "transcripts" / f"{run_id}.jsonl"
            timeout_s = RUN_TIMEOUT_S[task_class]

            start = time.time()
            if args.dry_run:
                print(f"  cwd: {workspace}")
                print(f"  command: {tools.show_command(cmd)}")
                if mcp_path:
                    print(f"  mcp config ({mcp_path}): {mcp_path.read_text()}")
                print("  prompt:\n    " + prompt.replace("\n", "\n    "))
                if arm == "sem":
                    for line in check_sem_mcp(sem_binary, workspace, sem_log, spec, task, mcp_tools):
                        print(f"  tool check: {line}")
                if arm == "cli":
                    for line in check_sem_cli(sem_binary, workspace, task):
                        print(f"  tool check: {line}")
                agent = {"stop_reason": None, "final_text": "", "error": None}
                stats = tools.RunStats()
            else:
                agent = run_agent(cmd, workspace, transcript, timeout_s)
                with open(transcript) as f:
                    stats = tools.read_transcript(f, workspace)
                agent = agent_outcome(agent, stats, timeout_s)
            wall = round(time.time() - start, 1)

            usage = tools.usage_of(stats)
            cost = stats.result.get("total_cost_usd")
            if cost is not None:
                budget.spent += cost
            if agent["error"] and agent["error"].startswith("cap:"):
                budget.exhausted = True
            check = list_price_usd(usage, prices)
            if cost and check is not None and abs(check - cost) > 0.05 * cost:
                print(f"  NOTE: total_cost_usd {cost:.4f} vs prices.json {check:.4f}")
            mcp_status = [s for s in stats.init.get("mcp_servers", [])]
            if arm == "sem" and not args.dry_run and not any(s.get("status") == "connected" for s in mcp_status):
                agent["error"] = agent["error"] or f"sem mcp not connected: {mcp_status}"

            result = score(spec, task, agent["final_text"], workspace, base, work_dir / "score" / run_id)
            row = {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "checkpoint": checkpoint, "build": build, "sem_version": sem_version,
                "abapgit_commit": commit[:12], "model": args.model, "arm": arm,
                "brief": brief or "0",
                "task_class": task_class, "task_id": task["id"], "rep": rep, "dry_run": int(args.dry_run),
                **usage,
                "cost_usd": cost,
                "wall_time_s": wall,
                "api_time_s": round(stats.result["duration_api_ms"] / 1000, 1) if stats.result.get("duration_api_ms") else None,
                "turns": stats.result.get("num_turns"), "tool_calls": stats.tool_calls,
                "files_read": len(stats.files_read), "bytes_read": stats.bytes_read,
                "test_classes_executed": len(stats.test_classes),
                "success_score": result.get("success_score"),
                "precision": result.get("precision"), "recall": result.get("recall"),
                "tests_passed": result.get("tests_passed"), "callers_missed": result.get("callers_missed"),
                "stop_reason": agent["stop_reason"],
                "error": agent["error"] or result.get("error"),
                "mcp_tools": mcp_tools or "",
            }
            append_row(csv_path, row)
            append_jsonl(jsonl_path, {
                **row, "claude_version": claude_version, "session_id": session_id, "command": cmd,
                "prompt": prompt, "final_text": agent["final_text"],
                "init_tools": stats.init.get("tools"), "init_model": stats.init.get("model"),
                "mcp_servers": mcp_status, "model_usage": stats.result.get("modelUsage"),
                "permission_denials": stats.result.get("permission_denials"),
                "list_price_usd": check, "transcript": str(transcript),
                "calls_by_tool": stats.calls_by_tool, "sem_cli_calls": stats.sem_cli_calls, "files_read_list": sorted(stats.files_read),
                "test_classes_list": sorted(stats.test_classes), "score": result})
            summary.setdefault((task_class, arm), []).append(row)
            print(f"  score {row['success_score']} | tests_passed {row['tests_passed']} | "
                  f"callers_missed {row['callers_missed']} | tool calls {row['tool_calls']} {stats.calls_by_tool} | "
                  f"cost {row['cost_usd']} (total {budget.spent:.2f}) | {wall}s"
                  + (f" | {row['error']}" if row["error"] else ""))
            if not args.keep:
                shutil.rmtree(workspace, ignore_errors=True)
                shutil.rmtree(work_dir / "score" / run_id, ignore_errors=True)
        if budget.exhausted:
            print(f"\nStopped: cumulative cost ${budget.spent:.2f} reached --cap-usd {budget.cap}")
            break

    # ── Summary ──────────────────────────────────────────────────────────────

    print()
    print("=" * 84)
    print(f"{'Class':<6} {'Arm':<5} {'Runs':>5} {'Score':>7} {'In tok':>10} {'Out tok':>9} {'Cost':>9} {'Wall s':>8} {'Tools':>6}")
    print("-" * 84)
    for (task_class, arm), rows in sorted(summary.items()):
        scores = [float(r["success_score"]) for r in rows if r["success_score"] not in (None, "")]
        mean = f"{sum(scores) / len(scores):.3f}" if scores else "-"
        costs = [r["cost_usd"] for r in rows if r["cost_usd"] is not None]
        cost = f"{sum(costs):.2f}" if costs else "-"
        read = sum(r["input_tokens"] + r["cache_read_tokens"] + r["cache_write_tokens"] for r in rows)
        print(f"{task_class:<6} {arm:<5} {len(rows):>5} {mean:>7} {read:>10} "
              f"{sum(r['output_tokens'] for r in rows):>9} {cost:>9} {sum(r['wall_time_s'] for r in rows):>8.0f} "
              f"{sum(r['tool_calls'] for r in rows):>6}")
    print(f"\nCumulative cost (Claude Code total_cost_usd): ${budget.spent:.2f}")
    print(f"Rows appended to {csv_path}")
    print(f"Details appended to {jsonl_path}")


if __name__ == "__main__":
    main()
