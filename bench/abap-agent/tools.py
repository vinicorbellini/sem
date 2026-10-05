"""Tool sets and transcripts for the ABAP agent benchmark, on headless Claude Code.

grep arm: Claude Code's built-in Bash, Read, Grep and Glob (plus Edit and Write on B2).
sem arm:  the same, plus the tools `sem mcp` lists (sem_find, sem_impact, sem_certify and
          the rest of LISTED_TOOLS in crates/sem-mcp/src/server.rs), attached with
          --mcp-config as a standalone stdio server (SEM_MCP_NO_SHARED=1, docs/shared-mcp.md).

Claude Code runs the agent loop and the tools. This module builds the command line, the
MCP config and the environment for one run, and reads the run's stream-json transcript
back into a RunStats. SemMcp is only used by --dry-run, to show the server answers.
"""

import json
import os
import queue
import re
import shlex
import subprocess
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

# ── Config ──────────────────────────────────────────────────────────────────

CLAUDE = "claude"
READ_TOOLS = ["Bash", "Read", "Grep", "Glob"]   # both arms, every class
WRITE_TOOLS = ["Edit", "Write"]                 # both arms, B2 only
MCP_SERVER = "sem"                              # tools appear as mcp__sem__<name>
MCP_PREFIX = f"mcp__{MCP_SERVER}__"
MCP_TIMEOUT_S = 300
BASH_TIMEOUT_MS = 900_000        # `npm run unit` takes ~35 s warm; first build fetches libraries
MCP_PROTOCOL_VERSION = "2025-03-26"  # as in benchmarks/shared-mcp/run.py
SEM_ENV = {"SEM_MCP_NO_SHARED": "1", "SEM_NO_TELEMETRY": "1", "SEM_NO_UPDATE_CHECK": "1",
           "SEM_CLOUD": "0", "SEM_NO_NETWORK": "1"}

# Variables of the Claude Code session that launches the benchmark. A child `claude` that
# inherits them joins the parent's session id, extra directories (and their CLAUDE.md
# files), effort level and remote messaging channel. Auth and proxy variables stay.
PARENT_SESSION_ENV = [
    "CLAUDE_CODE_SESSION_ID", "CLAUDE_CODE_REMOTE_SESSION_ID", "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_ADDITIONAL_DIRECTORIES", "CLAUDE_CODE_ADDITIONAL_DIRECTORIES_CLAUDE_MD", "CLAUDE_EFFORT",
    "CLAUDE_CODE_MESSAGING_SOCKET", "CLAUDE_CODE_MESSAGING_TOKEN", "CLAUDE_CODE_TEE_SDK_STDOUT",
    "CLAUDE_CODE_REMOTE_TOOLS_FORWARD", "CLAUDE_CODE_SYNC_SKILLS", "CLAUDE_CODE_SYNC_SESSION_REFS",
    "CLAUDE_CODE_DEBUG", "CLAUDE_CODE_SESSION_ATTENDED", "CLAUDE_CODE_DIAGNOSTICS_FILE",
    "CLAUDE_AFTER_LAST_COMPACT", "CLAUDE_PID", "CLAUDECODE", "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_HOLD_UNANSWERED_PARKED_PERMISSION", "CLAUDE_CODE_ARTIFACT_ASSETS",
    "CLAUDE_AUTO_BACKGROUND_TASKS", "CLAUDE_CODE_BG_TASKS_REPORT_RUNNING", "CLAUDE_CODE_WORKER_EPOCH",
    "MCP_CONNECTION_NONBLOCKING",  # the sem tools must be there from the first request
    "SEM_MCP_REQUIRE_SHARED",
]

# One line per unit test method printed by abapGit's output/index.mjs, e.g.
# "ZCL_ABAPGIT_PATH: running ltcl_path->split_file_location" (", skipped ..." when not run)
TEST_LINE = re.compile(r"^(\w+): running (\w+)->(\w+)(, skipped.*)?$", re.M)


# ── Command line ────────────────────────────────────────────────────────────


def arm_tools(arm: str, writes: bool) -> tuple[list[str], list[str]]:
    """(--tools, --allowedTools) for an arm. --tools decides which built-in tools exist at all."""
    builtin = READ_TOOLS + (WRITE_TOOLS if writes else [])
    allowed = builtin + ([f"mcp__{MCP_SERVER}"] if arm == "sem" else [])
    return builtin, allowed


def mcp_config(sem_binary: str, workspace: Path, log_path: Path) -> dict:
    """`sem mcp` over stdio in the run's checkout, standalone, its stderr appended to log_path."""
    return {"mcpServers": {MCP_SERVER: {
        "type": "stdio",
        "command": "sh",
        "args": ["-c", 'cd "$2" && exec "$0" mcp 2>>"$1"', sem_binary, str(log_path), str(workspace)],
        "env": {"SEM_REPO": str(workspace), **SEM_ENV},
    }}}


def claude_command(prompt: str, model: str, effort: str, max_turns: int, arm: str, writes: bool,
                   session_id: str, mcp_config_path: Path | None, budget_usd: float | None) -> list[str]:
    builtin, allowed = arm_tools(arm, writes)
    cmd = [
        CLAUDE, "-p", prompt,
        "--output-format", "stream-json", "--verbose",   # stream-json is the only output with tool calls
        "--model", model, "--effort", effort, "--max-turns", str(max_turns),
        "--tools", ",".join(builtin),
        "--allowedTools", ",".join(allowed),
        "--permission-mode", "dontAsk",                  # anything not allowed is denied, never prompted
        "--setting-sources", "",                         # no user/project/local settings, hooks or CLAUDE.md
        "--strict-mcp-config",                           # no MCP server but the one passed here
        "--no-session-persistence", "--session-id", session_id,
    ]
    if mcp_config_path is not None:
        cmd += ["--mcp-config", str(mcp_config_path)]
    if budget_usd is not None:
        cmd += ["--max-budget-usd", f"{budget_usd:.2f}"]
    return cmd


def claude_env() -> dict:
    env = {k: v for k, v in os.environ.items() if k not in PARENT_SESSION_ENV}
    env.update({"CI": "1", "MCP_TOOL_TIMEOUT": str(MCP_TIMEOUT_S * 1000),
                "BASH_DEFAULT_TIMEOUT_MS": str(BASH_TIMEOUT_MS), "BASH_MAX_TIMEOUT_MS": str(BASH_TIMEOUT_MS)})
    return env


def show_command(cmd: list[str]) -> str:
    return shlex.join(cmd)


# ── Transcript ──────────────────────────────────────────────────────────────


@dataclass
class RunStats:
    tool_calls: int = 0
    calls_by_tool: dict[str, int] = field(default_factory=dict)
    files_read: set[str] = field(default_factory=set)
    bytes_read: int = 0  # bytes of tool output returned to the model, all tools
    test_classes: set[tuple[str, str]] = field(default_factory=set)  # (object, local class) that ran
    init: dict = field(default_factory=dict)     # the system/init event: tools, mcp_servers, model
    result: dict = field(default_factory=dict)   # the final result event
    usage_by_message: dict[str, dict] = field(default_factory=dict)  # fallback when there is no result


def test_classes_in(output: str) -> set[tuple[str, str]]:
    """(object, local class) pairs a unit-test run printed as running, not skipped."""
    return {(m.group(1), m.group(2)) for m in TEST_LINE.finditer(output) if not m.group(4)}


def _result_text(content) -> str:
    if isinstance(content, str):
        return content
    if isinstance(content, list):
        return "\n".join(c.get("text", "") for c in content if isinstance(c, dict))
    return ""


def tool_name(name: str) -> str:
    return name[len(MCP_PREFIX):] if name.startswith(MCP_PREFIX) else name


def read_transcript(lines, workspace: Path) -> RunStats:
    """Fold Claude Code's stream-json events into one run's stats."""
    stats = RunStats()
    workspace = workspace.resolve()
    pending: dict[str, str] = {}  # tool_use id -> tool name
    for line in lines:
        try:
            event = json.loads(line)
        except (json.JSONDecodeError, TypeError):
            continue
        kind = event.get("type")
        if kind == "system" and event.get("subtype") == "init":
            stats.init = event
        elif kind == "result":
            stats.result = event
        elif kind == "assistant" and not event.get("parent_tool_use_id"):
            message = event.get("message", {})
            if message.get("id") and message.get("usage"):
                stats.usage_by_message[message["id"]] = message["usage"]
            for block in message.get("content", []):
                if block.get("type") != "tool_use":
                    continue
                name = tool_name(block.get("name", ""))
                pending[block.get("id")] = name
                stats.tool_calls += 1
                stats.calls_by_tool[name] = stats.calls_by_tool.get(name, 0) + 1
                if name == "Read" and block.get("input", {}).get("file_path"):
                    path = Path(block["input"]["file_path"])
                    path = path if path.is_absolute() else workspace / path
                    try:
                        stats.files_read.add(str(path.resolve().relative_to(workspace)))
                    except ValueError:
                        stats.files_read.add(str(path))
        elif kind == "user":
            for block in event.get("message", {}).get("content", []) or []:
                if not isinstance(block, dict) or block.get("type") != "tool_result":
                    continue
                text = _result_text(block.get("content"))
                stats.bytes_read += len(text.encode())
                if pending.get(block.get("tool_use_id")) == "Bash":
                    stats.test_classes |= test_classes_in(text)
                    full = event.get("tool_use_result")
                    if isinstance(full, dict):  # the untruncated output, when Claude Code includes it
                        stats.test_classes |= test_classes_in(f"{full.get('stdout', '')}\n{full.get('stderr', '')}")
    return stats


def usage_of(stats: RunStats) -> dict:
    """Token totals: the result event's, else the sum over the transcript's requests (a killed run)."""
    u = stats.result.get("usage")
    if not u:
        u = {}
        for m in stats.usage_by_message.values():
            for k in ("input_tokens", "output_tokens", "cache_read_input_tokens", "cache_creation_input_tokens"):
                u[k] = u.get(k, 0) + (m.get(k) or 0)
    return {"input_tokens": u.get("input_tokens", 0) or 0,
            "output_tokens": u.get("output_tokens", 0) or 0,
            "cache_read_tokens": u.get("cache_read_input_tokens", 0) or 0,
            "cache_write_tokens": u.get("cache_creation_input_tokens", 0) or 0}


# ── sem mcp, for --dry-run ───────────────────────────────────────────────────


class SemMcp:
    """A `sem mcp` process for one checkout, spoken to over stdio, the way Claude Code starts it."""

    def __init__(self, sem_binary: str, workspace: Path, log_path: Path):
        env = {k: v for k, v in os.environ.items() if k != "SEM_MCP_REQUIRE_SHARED"}
        env.update({"SEM_REPO": str(workspace), **SEM_ENV})
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
