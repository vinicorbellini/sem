#!/usr/bin/env python3
"""ABAP agent benchmark: grep tools vs grep tools plus sem's MCP tools, on abapGit.

Paired agent runs on a pinned commit of abapGit: same model, same prompt, one arm
with grep, glob, read_file and bash ("grep"), the other with those plus sem_find,
sem_impact and sem_certify from `sem mcp` ("sem"). Three task classes: B1
where-used, B2 change-and-verify, B3 review. Every run gets a fresh checkout and
appends one row to results.csv.

Usage:
    export ANTHROPIC_API_KEY=...
    python3 bench/abap-agent/run.py --checkpoint baseline --arm both --class all --cap-usd 50
    python3 bench/abap-agent/run.py --dry-run --arm both --class all --reps 1

Dependencies: anthropic (not needed for --dry-run), git, node + npm, a release build of sem
"""

import argparse
import csv
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import scorers  # noqa: E402
from tools import RunStats, grep_arm_tools, sem_arm_tools  # noqa: E402

# ── Config ──────────────────────────────────────────────────────────────────

MODEL = "claude-sonnet-5-5"
MAX_TOKENS = 16_000        # per model request, not per run
EFFORT = "high"            # set explicitly: the default differs by model
MAX_TURNS = 80             # model requests per run (the tool runner's max_iterations)
REPS = 3
CHECKPOINTS = ("baseline", "gate1", "gate2")
ARMS = ("grep", "sem")
CLASSES = ("B1", "B2", "B3")

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
    "B2": BENCH_DIR / "tasks" / "b2_change.json",
    "B3": BENCH_DIR / "tasks" / "b3_review.json",
}

# Identical for both arms; the arms differ only in their tool lists.
SYSTEM_PROMPT = (
    "You are a software engineer working in an ABAP code base checked out on disk. "
    "Use the tools to inspect and, when the task asks for it, change the repository. "
    "Work until the task is complete, then give your final answer in the format the task asks for."
)

CSV_COLUMNS = [
    "timestamp", "checkpoint", "build", "sem_version", "abapgit_commit", "model", "arm",
    "task_class", "task_id", "rep", "dry_run",
    "input_tokens", "output_tokens", "cache_read_tokens", "cache_write_tokens", "cost_usd",
    "wall_time_s", "turns", "tool_calls", "files_read", "bytes_read", "test_classes_executed",
    "success_score", "precision", "recall", "tests_passed", "callers_missed",
    "stop_reason", "error",
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
            return Path(c)
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


# ── Prices ───────────────────────────────────────────────────────────────────


def load_prices(model: str, required: bool) -> dict | None:
    """USD per million tokens for `model`. A real run stops here if any value is missing."""
    table = json.loads(PRICES_PATH.read_text()).get("models", {})
    prices = table.get(model)
    missing = [k for k in ("input", "output", "cache_read", "cache_write") if not prices or prices.get(k) is None]
    if missing:
        message = (f"prices.json has no {', '.join(missing)} price for {model}. "
                   f"Fill in {PRICES_PATH} (USD per million tokens) before a real run.")
        if required:
            print(f"ERROR: {message}")
            sys.exit(1)
        print(f"WARNING: {message} A real run would stop here.")
        return None
    return prices


def cost_usd(usage: dict, prices: dict | None) -> float | None:
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


def prepare_base(work_dir: Path, source: str, commit: str, needed: list[str], depth: int) -> Path:
    """One pristine checkout at `commit`, with history back to every B3 commit and node_modules from the pinned lockfile."""
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
    if not (base / "node_modules").exists():
        print("  Installing abapGit's npm dependencies from the pinned lockfile")
        shutil.copy(LOCKFILE, base / "package-lock.json")
        run(["npm", "ci", "--ignore-scripts", "--no-audit", "--no-fund"], cwd=base)
    return base


def make_workspace(base: Path, work_dir: Path, run_id: str, commit: str) -> Path:
    workspace = work_dir / "runs" / run_id
    if workspace.exists():
        shutil.rmtree(workspace)
    workspace.parent.mkdir(parents=True, exist_ok=True)
    run(["git", "clone", "-q", "--no-hardlinks", str(base), str(workspace)])
    run(["git", "-C", str(workspace), "checkout", "-q", "--detach", commit])
    (workspace / "node_modules").symlink_to(base / "node_modules")
    # abapGit's .gitignore has "node_modules/", which does not match a symlink.
    with open(workspace / ".git" / "info" / "exclude", "a") as f:
        f.write("node_modules\n")
    return workspace


# ── Prompts ──────────────────────────────────────────────────────────────────


def build_prompt(spec: dict, task: dict) -> str:
    fields = dict(task)
    if spec["class"] == "B3":
        fields["head"] = spec["abapgit_commit"][:12]
    return spec["prompt_template"].format(**fields)


def request_summary(model: str, tools: list, prompt: str) -> dict:
    """What a real run sends as its first request (the tool runner adds turns after it)."""
    return {
        "model": model,
        "max_tokens": MAX_TOKENS,
        "thinking": {"type": "adaptive"},
        "output_config": {"effort": EFFORT},
        "cache_control": {"type": "ephemeral"},
        "max_iterations": MAX_TURNS,
        "system": SYSTEM_PROMPT,
        "tools": [t.definition() for t in tools],
        "messages": [{"role": "user", "content": prompt}],
    }


# ── Agent run ────────────────────────────────────────────────────────────────


class Budget:
    """--cap-usd: stop before a request whose cost would take the cumulative total past the cap.

    A request's cost is only known after it returns, so the next request is estimated at
    the cost of the previous one; context only grows, so this tends to under-estimate.
    """

    def __init__(self, cap: float | None):
        self.cap = cap
        self.spent = 0.0
        self.exhausted = False

    def allows(self, next_estimate: float) -> bool:
        if self.cap is None:
            return True
        if self.spent + next_estimate > self.cap:
            self.exhausted = True
            return False
        return True


def _bind(tool):
    def call(**kwargs) -> str:
        return tool(**kwargs)
    return call


def run_agent(client, model: str, tools: list, prompt: str, prices: dict, budget: Budget) -> dict:
    """One agent run through the SDK's tool runner (client.beta.messages.tool_runner)."""
    import anthropic
    from anthropic import beta_tool

    usage = {"input_tokens": 0, "output_tokens": 0, "cache_read_tokens": 0, "cache_write_tokens": 0}
    out = {"usage": usage, "turns": 0, "stop_reason": None, "final_text": "", "error": None}

    # First request: count its input tokens (free) so the cap is checked before anything is spent.
    definitions = [t.definition() for t in tools]
    counted = client.messages.count_tokens(model=model, system=SYSTEM_PROMPT, tools=definitions,
                                           messages=[{"role": "user", "content": prompt}])
    if not budget.allows(counted.input_tokens * prices["input"] / 1_000_000):
        out["error"] = f"cap: first request would pass --cap-usd {budget.cap}"
        return out

    runnable = [beta_tool(_bind(t), name=t.name, description=t.description, input_schema=t.input_schema)
                for t in tools]
    runner = client.beta.messages.tool_runner(
        model=model,
        max_tokens=MAX_TOKENS,
        system=SYSTEM_PROMPT,
        tools=runnable,
        messages=[{"role": "user", "content": prompt}],
        thinking={"type": "adaptive"},
        output_config={"effort": EFFORT},
        cache_control={"type": "ephemeral"},
        max_iterations=MAX_TURNS,
    )
    last = None
    try:
        for message in runner:
            last = message
            out["turns"] += 1
            before = cost_usd(usage, prices)
            u = message.usage
            usage["input_tokens"] += u.input_tokens or 0
            usage["output_tokens"] += u.output_tokens or 0
            usage["cache_read_tokens"] += u.cache_read_input_tokens or 0
            usage["cache_write_tokens"] += u.cache_creation_input_tokens or 0
            turn_cost = cost_usd(usage, prices) - before
            budget.spent += turn_cost
            if message.stop_reason == "tool_use" and not budget.allows(turn_cost):
                out["error"] = f"cap: next request would pass --cap-usd {budget.cap}"
                break
    except anthropic.APIStatusError as e:
        out["error"] = f"api {e.status_code}: {str(e)[:300]}"
    except anthropic.APIConnectionError as e:
        out["error"] = f"connection: {str(e)[:300]}"

    if last is not None:
        out["stop_reason"] = last.stop_reason
        out["final_text"] = "\n".join(b.text for b in last.content if b.type == "text")
        if last.stop_reason == "tool_use" and out["error"] is None:
            out["error"] = f"max turns ({MAX_TURNS})"
        elif last.stop_reason in ("max_tokens", "refusal") and out["error"] is None:
            out["error"] = last.stop_reason
    return out


# ── Scoring ──────────────────────────────────────────────────────────────────


def score(spec: dict, task: dict, answer: str, workspace: Path, base: Path, scratch: Path) -> dict:
    if spec["class"] == "B1":
        truth = scorers.load_whereused(BENCH_DIR / spec["ground_truth"], task["id"])
        return scorers.score_b1(answer, truth)
    if spec["class"] == "B2":
        return scorers.score_b2(task, workspace, base, BENCH_DIR / spec["hidden_tests_dir"], scratch)
    rubric = scorers.load_rubric(BENCH_DIR / spec["rubric"], task["id"])
    return scorers.score_b3(answer, rubric)


# ── Output ───────────────────────────────────────────────────────────────────


def append_row(path: Path, row: dict):
    """Append-only. A file whose header differs from CSV_COLUMNS is an error, never rewritten."""
    if path.exists() and path.stat().st_size > 0:
        with open(path, newline="") as f:
            header = next(csv.reader(f), [])
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


def smoke_tools(tools: list, spec: dict, task: dict) -> list[str]:
    """--dry-run: call each tool once on the run's checkout (not counted) to show it works."""
    target = task.get("method") or task.get("target")
    method = target.replace("=>", "->").split("->")[-1].split("~")[-1] if target else None
    calls = {
        "grep": {"pattern": method, "glob": "*.abap"},
        "glob": {"pattern": "src/git/*.clas.abap"},
        "read_file": {"path": task.get("defined_in", "package.json"), "limit": 5},
        "bash": {"command": "git log --oneline -1"},
        "sem_find": {"query": method, "mode": "callers"},
        "sem_impact": {"file_path": task.get("defined_in", ""), "entity_name": method, "mode": "dependents"},
        "sem_certify": {"range": f"{task.get('commit', 'HEAD')}~1..{task.get('commit', 'HEAD')}"},
    }
    lines = []
    for tool in tools:
        if tool.name in ("grep", "sem_find", "sem_impact") and method is None:
            continue
        if tool.name == "sem_impact" and "defined_in" not in task:
            continue
        if tool.name == "sem_certify" and spec["class"] != "B3":
            continue
        start = time.time()
        try:
            output = tool.func(calls[tool.name])
            lines.append(f"{tool.name}: {len(output.encode())} bytes, {time.time() - start:.1f}s")
        except Exception as e:
            lines.append(f"{tool.name}: ERROR {str(e)[:200]}")
    return lines


# ── Main ─────────────────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--checkpoint", choices=CHECKPOINTS, help="Which checkpoint these runs measure.")
    parser.add_argument("--arm", choices=("grep", "sem", "both"), default="both")
    parser.add_argument("--class", dest="task_class", choices=("B1", "B2", "B3", "all"), default="all")
    parser.add_argument("--model", default=MODEL)
    parser.add_argument("--reps", type=int, default=REPS)
    parser.add_argument("--task", action="append", help="Only this task id (repeatable), e.g. b1_03.")
    parser.add_argument("--dry-run", action="store_true",
                        help="Do everything except the model call; print what would be sent.")
    parser.add_argument("--cap-usd", type=float, help="Stop when the cumulative cost would exceed this.")
    parser.add_argument("--abapgit", default=ABAPGIT_SOURCE, help="abapGit clone to copy checkouts from.")
    parser.add_argument("--work-dir", type=Path, default=WORK_DIR)
    parser.add_argument("--sem-binary")
    parser.add_argument("--keep", action="store_true", help="Keep each run's checkout.")
    args = parser.parse_args()

    if not args.dry_run and not args.checkpoint:
        parser.error("--checkpoint is required for a real run")
    checkpoint = args.checkpoint or "dry-run"
    arms = ARMS if args.arm == "both" else (args.arm,)
    classes = CLASSES if args.task_class == "all" else (args.task_class,)

    prices = load_prices(args.model, required=not args.dry_run)
    client = None
    if not args.dry_run:
        try:
            import anthropic
        except ImportError:
            print("Install the Anthropic SDK: pip install anthropic")
            sys.exit(1)
        client = anthropic.Anthropic()

    sem_binary = resolve_sem_binary(args.sem_binary)
    build, sem_version = sem_build(sem_binary)
    specs = {c: load_tasks(c) for c in classes}
    commit = next(iter(specs.values()))["abapgit_commit"]
    assert all(s["abapgit_commit"] == commit for s in specs.values()), "task files pin different abapGit commits"
    b3 = load_tasks("B3")
    base = prepare_base(args.work_dir, args.abapgit, commit,
                        [t["commit"] for t in b3["tasks"]] if "B3" in classes else [], b3["history_depth"])

    plan = [(c, t, rep) for c in classes for t in specs[c]["tasks"]
            if not args.task or t["id"] in args.task for rep in range(args.reps)]
    csv_path, jsonl_path = (DRY_RUN_CSV, DRY_RUN_JSONL) if args.dry_run else (RESULTS_CSV, RESULTS_JSONL)

    print(f"ABAP agent benchmark: grep vs sem on abapGit {commit[:12]}")
    print(f"Model: {args.model} | Checkpoint: {checkpoint} | Build: {build} ({sem_version}) | sem: {sem_binary}")
    print(f"Classes: {', '.join(classes)} | Tasks: {len({(c, t['id']) for c, t, _ in plan})} | "
          f"Reps: {args.reps} | Arms: {', '.join(arms)} | Runs: {len(plan) * len(arms)}")
    if args.dry_run:
        print("DRY RUN: no model calls; scoring runs on the untouched checkouts")
    print()

    budget = Budget(args.cap_usd)
    shown = set()
    summary: dict[tuple[str, str], list] = {}
    request_dir = args.work_dir / "requests"
    request_dir.mkdir(parents=True, exist_ok=True)

    for task_class, task, rep in plan:
        spec = specs[task_class]
        # Alternate the arm order by repetition so drift (cache warmth, rate limits) is not one-sided.
        order = arms if rep % 2 == 0 else tuple(reversed(arms))
        for arm in order:
            if budget.exhausted:
                break
            run_id = f"{checkpoint}-{args.model}-{arm}-{task['id']}-r{rep}"
            print(f"── {task['id']} [{arm}] rep {rep} ──")
            workspace = make_workspace(base, args.work_dir, run_id, commit)
            stats = RunStats()
            mcp = None
            if arm == "sem":
                tools, mcp = sem_arm_tools(workspace, stats, str(sem_binary), args.work_dir / f"{run_id}.sem-mcp.log")
            else:
                tools = grep_arm_tools(workspace, stats)
            prompt = build_prompt(spec, task)
            request = request_summary(args.model, tools, prompt)
            (request_dir / f"{run_id}.json").write_text(json.dumps(request, indent=2))

            start = time.time()
            if args.dry_run:
                if (task_class, arm) not in shown:
                    shown.add((task_class, arm))
                    print(f"  would send (first request, {len(json.dumps(request))} bytes):")
                    print("    " + json.dumps({k: v for k, v in request.items() if k != "tools"}, indent=2)
                          .replace("\n", "\n    "))
                    print(f"    tools: {[t['name'] for t in request['tools']]}")
                else:
                    print(f"  would send: {len(json.dumps(request))} bytes, tools {[t.name for t in tools]}")
                for line in smoke_tools(tools, spec, task):
                    print(f"  tool check: {line}")
                agent = {"usage": {"input_tokens": 0, "output_tokens": 0, "cache_read_tokens": 0,
                                   "cache_write_tokens": 0},
                         "turns": 0, "stop_reason": None, "final_text": "", "error": None}
            else:
                agent = run_agent(client, args.model, tools, prompt, prices, budget)
            wall = round(time.time() - start, 1)
            if mcp:
                mcp.close()

            result = score(spec, task, agent["final_text"], workspace,
                           base, args.work_dir / "score" / run_id)
            row = {
                "timestamp": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
                "checkpoint": checkpoint, "build": build, "sem_version": sem_version,
                "abapgit_commit": commit[:12], "model": args.model, "arm": arm,
                "task_class": task_class, "task_id": task["id"], "rep": rep, "dry_run": int(args.dry_run),
                **agent["usage"],
                "cost_usd": cost_usd(agent["usage"], prices),
                "wall_time_s": wall, "turns": agent["turns"], "tool_calls": stats.tool_calls,
                "files_read": len(stats.files_read), "bytes_read": stats.bytes_read,
                "test_classes_executed": len(stats.test_classes),
                "success_score": result.get("success_score"),
                "precision": result.get("precision"), "recall": result.get("recall"),
                "tests_passed": result.get("tests_passed"), "callers_missed": result.get("callers_missed"),
                "stop_reason": agent["stop_reason"],
                "error": agent["error"] or result.get("error"),
            }
            append_row(csv_path, row)
            append_jsonl(jsonl_path, {**row, "prompt": prompt, "final_text": agent["final_text"],
                                      "calls_by_tool": stats.calls_by_tool,
                                      "files_read_list": sorted(stats.files_read),
                                      "test_classes_list": sorted(stats.test_classes), "score": result})
            summary.setdefault((task_class, arm), []).append(row)
            print(f"  score {row['success_score']} | tests_passed {row['tests_passed']} | "
                  f"callers_missed {row['callers_missed']} | tool calls {row['tool_calls']} | "
                  f"cost {row['cost_usd']} | {wall}s" + (f" | {row['error']}" if row["error"] else ""))
            if not args.keep:
                shutil.rmtree(workspace, ignore_errors=True)
                shutil.rmtree(args.work_dir / "score" / run_id, ignore_errors=True)
        if budget.exhausted:
            print(f"\nStopped: cumulative cost ${budget.spent:.2f} would pass --cap-usd {budget.cap}")
            break

    # ── Summary ──────────────────────────────────────────────────────────────

    print()
    print("=" * 72)
    print(f"{'Class':<6} {'Arm':<5} {'Runs':>5} {'Score':>7} {'In tok':>10} {'Out tok':>9} {'Cost':>9} {'Wall s':>8}")
    print("-" * 72)
    for (task_class, arm), rows in sorted(summary.items()):
        scores = [float(r["success_score"]) for r in rows if r["success_score"] not in (None, "")]
        mean = f"{sum(scores) / len(scores):.3f}" if scores else "-"
        costs = [r["cost_usd"] for r in rows if r["cost_usd"] is not None]
        cost = f"{sum(costs):.2f}" if costs else "-"
        print(f"{task_class:<6} {arm:<5} {len(rows):>5} {mean:>7} {sum(r['input_tokens'] for r in rows):>10} "
              f"{sum(r['output_tokens'] for r in rows):>9} {cost:>9} {sum(r['wall_time_s'] for r in rows):>8.0f}")
    print()
    print(f"Rows appended to {csv_path}")
    print(f"Details appended to {jsonl_path}")


if __name__ == "__main__":
    main()
