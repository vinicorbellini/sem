"""Tool sets for the ABAP agent benchmark.

grep arm: grep, glob, read_file and bash over the run's abapGit checkout.
sem arm:  the same four, plus sem_find, sem_impact and sem_certify, served by
          `sem mcp` over newline-delimited JSON-RPC on stdio (docs/shared-mcp.md).

Tools here are plain Python: run.py wraps each one for the Anthropic SDK's tool
runner, so this module never imports the SDK and --dry-run works without it.
Every call is counted in a RunStats.
"""

import json
import os
import queue
import re
import subprocess
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Callable

# ── Config ──────────────────────────────────────────────────────────────────

MAX_TOOL_OUTPUT_BYTES = 60_000   # same cap for every tool, both arms
GREP_MAX_MATCHES = 500
GLOB_MAX_RESULTS = 1_000
READ_DEFAULT_LIMIT = 2_000
BASH_TIMEOUT_S = 900             # `npm run unit` takes ~35 s warm; first build fetches libraries
MCP_TIMEOUT_S = 300
MCP_PROTOCOL_VERSION = "2025-03-26"  # as in benchmarks/shared-mcp/run.py
SEM_TOOLS = ["sem_find", "sem_impact", "sem_certify"]
EXCLUDED_DIRS = {".git", "node_modules", "output"}

# One line per unit test method printed by abapGit's output/index.mjs, e.g.
# "ZCL_ABAPGIT_PATH: running ltcl_path->split_file_location" (", skipped ..." when not run)
TEST_LINE = re.compile(r"^(\w+): running (\w+)->(\w+)(, skipped.*)?$", re.M)


# ── Stats ───────────────────────────────────────────────────────────────────


@dataclass
class RunStats:
    tool_calls: int = 0
    calls_by_tool: dict[str, int] = field(default_factory=dict)
    files_read: set[str] = field(default_factory=set)
    bytes_read: int = 0  # bytes of tool output returned to the model, all tools
    test_classes: set[tuple[str, str]] = field(default_factory=set)  # (object, local class) that ran

    def record(self, name: str, output: str):
        self.tool_calls += 1
        self.calls_by_tool[name] = self.calls_by_tool.get(name, 0) + 1
        self.bytes_read += len(output.encode())


def test_classes_in(output: str) -> set[tuple[str, str]]:
    """(object, local class) pairs a unit-test run printed as running, not skipped."""
    return {(m.group(1), m.group(2)) for m in TEST_LINE.finditer(output) if not m.group(4)}


def truncate(text: str) -> str:
    raw = text.encode()
    if len(raw) <= MAX_TOOL_OUTPUT_BYTES:
        return text
    head = raw[:MAX_TOOL_OUTPUT_BYTES].decode(errors="ignore")
    return head + f"\n\n... [truncated at {MAX_TOOL_OUTPUT_BYTES // 1000}KB of {len(raw) // 1000}KB] ..."


# ── Tool ────────────────────────────────────────────────────────────────────


@dataclass
class Tool:
    name: str
    description: str
    input_schema: dict
    func: Callable[[dict], str]
    stats: RunStats

    def __call__(self, **kwargs) -> str:
        """Run the tool. Errors raise; the SDK tool runner returns them to the model with is_error."""
        try:
            output = truncate(self.func(kwargs))
        except Exception as e:
            self.stats.record(self.name, str(e))
            raise
        self.stats.record(self.name, output)
        return output

    def definition(self) -> dict:
        return {"name": self.name, "description": self.description, "input_schema": self.input_schema}


def _schema(properties: dict, required: list[str]) -> dict:
    return {"type": "object", "properties": properties, "required": required, "additionalProperties": False}


def _inside(workspace: Path, rel: str) -> Path:
    path = (workspace / rel).resolve()
    if path != workspace and workspace not in path.parents:
        raise ValueError(f"path outside the repository: {rel}")
    return path


def _glob_regex(pattern: str) -> re.Pattern:
    """Glob with ** (any directories), * and ? (within one path segment)."""
    out, i = "", 0
    while i < len(pattern):
        if pattern.startswith("**/", i):
            out, i = out + "(?:.*/)?", i + 3
        elif pattern.startswith("**", i):
            out, i = out + ".*", i + 2
        elif pattern[i] == "*":
            out, i = out + "[^/]*", i + 1
        elif pattern[i] == "?":
            out, i = out + "[^/]", i + 1
        else:
            out, i = out + re.escape(pattern[i]), i + 1
    return re.compile(out)


# ── grep arm ────────────────────────────────────────────────────────────────


def grep_arm_tools(workspace: Path, stats: RunStats) -> list[Tool]:
    workspace = workspace.resolve()

    def grep(args: dict) -> str:
        path = args.get("path") or "."
        _inside(workspace, path)
        cmd = ["grep", "-rnE", "-I"] + [f"--exclude-dir={d}" for d in sorted(EXCLUDED_DIRS)]
        if args.get("ignore_case"):
            cmd.append("-i")
        if args.get("glob"):
            cmd.append(f"--include={args['glob']}")
        cmd += ["-e", args["pattern"], "--", path]
        result = subprocess.run(cmd, cwd=workspace, capture_output=True, text=True, errors="replace")
        if result.returncode > 1:
            raise RuntimeError(result.stderr.strip() or f"grep exited {result.returncode}")
        lines = [l[2:] if l.startswith("./") else l for l in result.stdout.splitlines()]
        if not lines:
            return "No matches."
        out = "\n".join(lines[:GREP_MAX_MATCHES])
        if len(lines) > GREP_MAX_MATCHES:
            out += f"\n... [{len(lines) - GREP_MAX_MATCHES} more matches, narrow the pattern or path]"
        return out

    def glob(args: dict) -> str:
        # os.walk, not Path.glob: it prunes .git and node_modules instead of walking them.
        pattern = _glob_regex(args["pattern"].lstrip("./"))
        hits = []
        for root, dirs, files in os.walk(workspace):
            dirs[:] = sorted(d for d in dirs if d not in EXCLUDED_DIRS)
            rel_root = Path(root).relative_to(workspace)
            hits += [str(rel_root / f) if str(rel_root) != "." else f for f in files
                     if pattern.fullmatch(str(rel_root / f) if str(rel_root) != "." else f)]
        hits.sort()
        if not hits:
            return "No files."
        out = "\n".join(hits[:GLOB_MAX_RESULTS])
        if len(hits) > GLOB_MAX_RESULTS:
            out += f"\n... [{len(hits) - GLOB_MAX_RESULTS} more files]"
        return out

    def read_file(args: dict) -> str:
        path = _inside(workspace, args["path"])
        offset = max(int(args.get("offset") or 1), 1)
        limit = int(args.get("limit") or READ_DEFAULT_LIMIT)
        lines = path.read_text(errors="replace").splitlines()
        stats.files_read.add(str(path.relative_to(workspace)))
        chunk = lines[offset - 1: offset - 1 + limit]
        out = "\n".join(f"{offset + i:>6}\t{line}" for i, line in enumerate(chunk))
        if offset - 1 + limit < len(lines):
            out += f"\n... [{len(lines)} lines in total; continue with offset={offset + limit}]"
        return out or "(empty)"

    def bash(args: dict) -> str:
        try:
            result = subprocess.run(
                ["bash", "-c", args["command"]], cwd=workspace, capture_output=True, text=True,
                errors="replace", timeout=BASH_TIMEOUT_S, env=_bash_env(),
            )
        except subprocess.TimeoutExpired:
            raise RuntimeError(f"command timed out after {BASH_TIMEOUT_S} s")
        output = result.stdout + result.stderr
        stats.test_classes |= test_classes_in(output)
        return f"{output}\n[exit code {result.returncode}]"

    return [
        Tool("grep", "Search file contents with an extended regular expression (grep -rnE). Returns file:line:text "
             "hits, relative to the repository root.", _schema({
                 "pattern": {"type": "string", "description": "Extended regular expression."},
                 "path": {"type": "string", "description": "File or directory to search, relative to the repository root. Default '.'."},
                 "glob": {"type": "string", "description": "Only files whose name matches this glob, e.g. '*.abap'."},
                 "ignore_case": {"type": "boolean", "description": "Case-insensitive match."},
             }, ["pattern"]), grep, stats),
        Tool("glob", "List files matching a glob pattern relative to the repository root, e.g. 'src/**/*.clas.abap'.",
             _schema({"pattern": {"type": "string", "description": "Glob pattern; ** matches directories recursively."}},
                     ["pattern"]), glob, stats),
        Tool("read_file", "Read a text file with line numbers. Use offset and limit for long files.", _schema({
            "path": {"type": "string", "description": "Path relative to the repository root."},
            "offset": {"type": "integer", "description": "First line to return, 1-based. Default 1."},
            "limit": {"type": "integer", "description": f"Number of lines. Default {READ_DEFAULT_LIMIT}."},
        }, ["path"]), read_file, stats),
        Tool("bash", "Run a bash command in the repository root and return its output and exit code. Use it to edit "
             "files (sed, heredocs, python) and to run `npm run unit`.",
             _schema({"command": {"type": "string", "description": "The command to run."}}, ["command"]), bash, stats),
    ]


def _bash_env() -> dict:
    env = {k: v for k, v in os.environ.items() if not k.startswith("ANTHROPIC_")}
    env["CI"] = "1"
    return env


# ── sem arm ─────────────────────────────────────────────────────────────────


class SemMcp:
    """A `sem mcp` process for one checkout, spoken to over stdio.

    Standalone stdio (SEM_MCP_NO_SHARED=1): each run has its own checkout, and the
    process ends with the run instead of leaving a daemon behind.
    """

    def __init__(self, sem_binary: str, workspace: Path, log_path: Path):
        env = dict(os.environ)
        env.update({
            "SEM_REPO": str(workspace), "SEM_MCP_NO_SHARED": "1", "SEM_NO_TELEMETRY": "1",
            "SEM_NO_UPDATE_CHECK": "1", "SEM_CLOUD": "0", "SEM_NO_NETWORK": "1",
        })
        env.pop("SEM_MCP_REQUIRE_SHARED", None)
        self._log = open(log_path, "ab")
        self._next_id = 0
        self.process = subprocess.Popen(
            [sem_binary, "mcp"], cwd=workspace, env=env,
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=self._log,
        )
        # A reader thread, so a reply that arrives in the same read as a notification is never stuck
        # in a buffer that select() cannot see.
        self._lines: queue.Queue = queue.Queue()
        threading.Thread(target=self._read, daemon=True).start()
        self.request("initialize", {
            "protocolVersion": MCP_PROTOCOL_VERSION, "capabilities": {},
            "clientInfo": {"name": "abap-agent-bench", "version": "1"},
        })
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized", "params": {}})

    def _send(self, message: dict):
        self.process.stdin.write((json.dumps(message) + "\n").encode())
        self.process.stdin.flush()

    def _read(self):
        for line in self.process.stdout:
            self._lines.put(line)
        self._lines.put(b"")  # EOF

    def request(self, method: str, params: dict) -> dict:
        self._next_id += 1
        ident = self._next_id
        self._send({"jsonrpc": "2.0", "id": ident, "method": method, "params": params})
        deadline = time.monotonic() + MCP_TIMEOUT_S
        while time.monotonic() < deadline:
            try:
                line = self._lines.get(timeout=max(0.0, deadline - time.monotonic()))
            except queue.Empty:
                break
            if not line:
                raise RuntimeError(f"sem mcp exited during {method}")
            reply = json.loads(line)
            if reply.get("id") != ident:
                continue  # notifications and stale replies
            if "error" in reply:
                raise RuntimeError(f"sem mcp {method}: {reply['error']}")
            return reply["result"]
        raise TimeoutError(f"sem mcp {method} timed out after {MCP_TIMEOUT_S} s")

    def list_tools(self) -> list[dict]:
        return self.request("tools/list", {})["tools"]

    def call(self, name: str, arguments: dict) -> str:
        result = self.request("tools/call", {"name": name, "arguments": arguments})
        text = "\n".join(item.get("text", "") for item in result.get("content", []) if item.get("type") == "text")
        if result.get("isError"):
            raise RuntimeError(text or f"{name} failed")
        return text

    def close(self):
        try:
            self.process.stdin.close()
            self.process.wait(timeout=5)
        except (OSError, subprocess.TimeoutExpired):
            self.process.kill()
            self.process.wait()
        self.process.stdout.close()
        self._log.close()


def sem_arm_tools(workspace: Path, stats: RunStats, sem_binary: str, log_path: Path) -> tuple[list[Tool], SemMcp]:
    """The grep arm's tools plus SEM_TOOLS, with the schemas and descriptions `sem mcp` lists."""
    workspace = workspace.resolve()
    mcp = SemMcp(sem_binary, workspace, log_path)
    listed = {t["name"]: t for t in mcp.list_tools()}
    missing = [n for n in SEM_TOOLS if n not in listed]
    if missing:
        mcp.close()
        raise RuntimeError(f"sem mcp does not list {missing}; listed: {sorted(listed)}")
    tools = grep_arm_tools(workspace, stats)
    for name in SEM_TOOLS:
        schema = {k: v for k, v in listed[name]["inputSchema"].items() if k not in ("$schema", "title")}
        tools.append(Tool(name, listed[name].get("description", ""), schema,
                          lambda args, n=name: mcp.call(n, args), stats))
    return tools, mcp
