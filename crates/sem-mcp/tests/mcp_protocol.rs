//! RED tests proving three MCP server gaps, run against the real `sem-mcp`
//! binary over its actual wire protocol (newline-delimited JSON-RPC over
//! stdio, `ResilientStdioTransport` — see `crates/sem-mcp/src/transport.rs`).
//! There was no MCP-level test harness before this file: every existing test
//! in `server.rs` calls tool methods in-process
//! (`server.sem_entities(Parameters(...))`), which cannot observe any of the
//! bugs this file proves:
//!
//!   1. `EntitiesParams::query`/`::limit` are parsed but never read by
//!      `sem_entities` — only reachable by actually driving the tool through
//!      its JSON schema over the wire.
//!   2. `find`/`grep` don't exist as MCP tools at all, and `entities`/
//!      `context` accept no `format` param — both are "is this in
//!      `tools/list`" / "does this param exist" questions that only the
//!      protocol surface can answer; `EntitiesParams` has
//!      `#[serde(deny_unknown_fields)]`, so passing `format` today is a
//!      hard parse-time rejection, not a quietly-ignored field.
//!
//! `McpClient` below is the minimal harness: spawn the built binary in a
//! fixture repo (cwd = repo root, matching `sem_mcp::run`'s cwd-derived
//! `discover_repo_root`), speak `initialize` → `notifications/initialized`
//! → `tools/list` / `tools/call`, one JSON object per line.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

// ── Minimal MCP stdio client ──

struct McpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl McpClient {
    fn spawn(repo: &Path) -> Self {
        Self::spawn_with_discovery(repo, false)
    }

    fn spawn_with_discovery(repo: &Path, discovery: bool) -> Self {
        Self::spawn_with_probe(repo, discovery.then(|| json!({
            "_meta": {
                "io.modelcontextprotocol/protocolVersion": "2026-07-28",
                "io.modelcontextprotocol/clientInfo": {"name": "copilot-cli", "version": "1.0.88"}
            }
        })))
    }

    fn spawn_with_probe(repo: &Path, probe: Option<Value>) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_sem-mcp"))
            .current_dir(repo)
            // These tests exercise one isolated stdio server each.
            .env("SEM_MCP_NO_SHARED", "1")
            // `sem_certify`, and the callers verdict, run the `sem` CLI found on PATH
            .env("PATH", path_with_sem())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn sem-mcp binary");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        let mut client = Self {
            child,
            stdin,
            stdout,
            next_id: 1,
        };
        if let Some(params) = probe {
            let response = client.request("server/discover", params);
            assert_eq!(response["error"]["code"], -32601, "{response}");
        }
        client.initialize();
        client
    }

    fn write_line(&mut self, value: &Value) {
        let line = serde_json::to_string(value).expect("serialize json-rpc message");
        self.stdin
            .write_all(line.as_bytes())
            .expect("write to sem-mcp stdin");
        self.stdin.write_all(b"\n").expect("write newline");
        self.stdin.flush().expect("flush sem-mcp stdin");
    }

    /// Send a request and block for the response with the matching id,
    /// skipping any notifications the server interleaves.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.write_line(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }));
        loop {
            let mut line = String::new();
            let n = self
                .stdout
                .read_line(&mut line)
                .expect("read sem-mcp stdout");
            assert!(n > 0, "sem-mcp closed stdout before answering {method}");
            if line.trim().is_empty() {
                continue;
            }
            let msg: Value = serde_json::from_str(&line)
                .unwrap_or_else(|e| panic!("non-JSON line from sem-mcp: {e}\n{line}"));
            if msg.get("id") == Some(&Value::from(id)) {
                return msg;
            }
            // Not our response (e.g. a server-initiated notification) — keep reading.
        }
    }

    fn notify(&mut self, method: &str, params: Value) {
        self.write_line(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        }));
    }

    fn initialize(&mut self) {
        let resp = self.request(
            "initialize",
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "mcp-protocol-test", "version": "0.0.0"},
            }),
        );
        assert!(
            resp.get("result").is_some(),
            "initialize handshake failed: {resp}"
        );
        self.notify("notifications/initialized", json!({}));
    }

    fn tools_list(&mut self) -> Vec<Value> {
        let resp = self.request("tools/list", json!({}));
        resp["result"]["tools"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| panic!("tools/list returned no tools array: {resp}"))
    }

    /// Call a tool and return the decoded response, panicking loudly (a RED
    /// failure) if the call errored at the JSON-RPC layer (e.g. an unknown
    /// tool name, or `deny_unknown_fields` rejecting an unrecognized param).
    fn call_tool(&mut self, name: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments,
            }),
        )
    }
}

impl Drop for McpClient {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Extract the text of a successful, non-tool-error `tools/call` response.
/// Panics (RED) on a JSON-RPC error, a tool-level `isError`, or a
/// non-text content block — every one of those is a legitimate failure mode
/// for the bugs this file proves.
fn tool_text(resp: &Value) -> String {
    if let Some(err) = resp.get("error") {
        panic!("tools/call returned a JSON-RPC error: {err}");
    }
    let result = resp
        .get("result")
        .unwrap_or_else(|| panic!("tools/call response has neither result nor error: {resp}"));
    if result.get("isError") == Some(&Value::Bool(true)) {
        panic!("tool reported isError: {result}");
    }
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tool result content[0] is not text: {result}"))
        .to_string()
}

// ── Fixture repo ──

#[test]
fn agy_empty_discovery_probe_keeps_session_usable() {
    let repo = fixture_repo();
    // Issue #502: request id 1, server/discover, params {}.
    let mut client = McpClient::spawn_with_probe(repo.path(), Some(json!({})));
    assert!(!client.tools_list().is_empty());
    let response = client.call_tool("sem_entities", json!({"path": "src/needle.py"}));
    assert!(tool_text(&response).contains("needle_target_fn"));
}

#[test]
fn copilot_discovery_falls_back_to_initialize_and_tools_work() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn_with_discovery(repo.path(), true);
    let tools = client.tools_list();
    assert!(tools.iter().any(|tool| tool["name"] == "sem_find"));
    // sem_entities is no longer listed (sem_find covers it), but still answers.
    assert!(!tools.iter().any(|tool| tool["name"] == "sem_entities"));
    let response = client.call_tool("sem_entities", json!({"path": "src/needle.py"}));
    assert!(tool_text(&response).contains("needle_target_fn"));
    // Unknown methods must also leave an initialized session usable.
    let response = client.request("future/extension", json!({}));
    assert_eq!(response["error"]["code"], -32601);
    assert!(!client.tools_list().is_empty());
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// A small git repo with:
///   - one exact-name target (`needle_target_fn`) plus six name-prefixed
///     decoys (`needle_target_fn_helper_a`..`_f`) in the same file, so a
///     substring query has >5 candidate matches and exact-name relevance is
///     the only thing that can put the target first — proving the
///     "unranked full listing" bug can't pass by accident.
///   - a second file with a literal grep marker not derivable from any
///     entity *name*, so `sem grep`/`sem_grep` is exercised on text content
///     rather than the entity index.
fn fixture_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@t.com"]);
    git(root, &["config", "user.name", "test"]);

    std::fs::create_dir_all(root.join("src")).unwrap();
    let mut needle_py = String::new();
    needle_py.push_str("def needle_target_fn():\n    return 0\n\n\n");
    for letter in ["a", "b", "c", "d", "e", "f"] {
        needle_py.push_str(&format!(
            "def needle_target_fn_helper_{letter}():\n    return 0\n\n\n"
        ));
    }
    needle_py.push_str("def unrelated_other():\n    return 0\n");
    std::fs::write(root.join("src/needle.py"), needle_py).unwrap();

    std::fs::write(
        root.join("src/marker.py"),
        "# a line with NEEDLE_GREP_MARKER_XYZ inside it\ndef marker_holder():\n    return 1\n",
    )
    .unwrap();

    git(root, &["add", "-A"]);
    let status = Command::new("git")
        .current_dir(root)
        .args(["commit", "-q", "-m", "fixture"])
        .status()
        .unwrap();
    assert!(status.success());

    dir
}

/// The `sem` CLI binary, resolved relative to `sem-mcp`'s own test binary
/// rather than via `CARGO_BIN_EXE_sem`: `sem-cli` has no `[lib]` target
/// (bin-only crate), so Cargo won't add it as a dependency edge and can't
/// set that env var for us. Both binaries land in the same profile
/// directory under one `CARGO_TARGET_DIR` (see the MCP handoff/task setup),
/// so the sibling path is reliable as long as `sem-cli` has been built —
/// `cargo build -p sem-cli --bin sem` (or `-p sem-mcp -p sem-cli --bins`)
/// before running this test file.
fn sem_cli_bin() -> PathBuf {
    let mcp_bin = PathBuf::from(env!("CARGO_BIN_EXE_sem-mcp"));
    let dir = mcp_bin.parent().expect("sem-mcp bin has a parent dir");
    let candidate = dir.join(if cfg!(windows) { "sem.exe" } else { "sem" });
    assert!(
        candidate.exists(),
        "sem CLI binary not found at {candidate:?} — build it first: \
         `cargo build -p sem-cli --bin sem` with the same CARGO_TARGET_DIR \
         used for sem-mcp's tests"
    );
    candidate
}

/// `PATH` with the built `sem` binary's directory first, so the server's shell-outs find it.
fn path_with_sem() -> std::ffi::OsString {
    let dir = sem_cli_bin().parent().unwrap().to_path_buf();
    let mut paths = vec![dir];
    paths.extend(std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()));
    std::env::join_paths(paths).expect("join PATH")
}

fn cli_json(repo: &Path, args: &[&str]) -> Value {
    let output = Command::new(sem_cli_bin())
        .current_dir(repo)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run sem {args:?}: {e}"));
    assert!(
        output.status.success(),
        "sem {args:?} failed: stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "sem {args:?} did not print JSON: {e}\nstdout was: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

// ── (a) entities query/limit ──

#[test]
fn entities_query_mode_ranks_named_entity_first_and_respects_limit() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_entities",
        json!({"query": "needle_target_fn", "limit": 5}),
    );
    let text = tool_text(&resp);

    // "needle_target_fn" substring-matches 7 entities (the exact name plus
    // 6 helper_* siblings) in a repo of 8 total. Today's bug ignores query
    // and limit entirely and dumps every entity in "." (>=8 lines, in file
    // order, unrelated_other included) — so both assertions below are RED
    // against current behavior and are the actual spec for the fix: rank by
    // name relevance (exact match first) and cap at `limit`.
    let hit_lines: Vec<&str> = text
        .lines()
        .filter(|l| l.contains("needle_target_fn") || l.contains("unrelated_other"))
        .collect();

    assert!(
        hit_lines.len() <= 5,
        "query mode must respect limit=5, got {} matching lines:\n{text}",
        hit_lines.len()
    );
    assert!(
        !text.contains("unrelated_other"),
        "query \"needle_target_fn\" must not surface an unrelated entity ahead of \
         the 7 actual matches under a limit of 5:\n{text}"
    );
    let first_hit = hit_lines
        .first()
        .unwrap_or_else(|| panic!("no hits at all for query mode:\n{text}"));
    assert!(
        first_hit.contains("needle_target_fn") && !first_hit.contains("needle_target_fn_helper"),
        "exact name match must rank first, got: {first_hit:?}\nfull output:\n{text}"
    );
}

// ── (b) find + grep as MCP tools ──

#[test]
fn find_and_grep_tools_are_registered_and_match_the_cli() {
    let repo = fixture_repo();

    let cli_find: Value = cli_json(repo.path(), &["find", "needle_target_fn", "--json"]);
    let cli_find_row = cli_find
        .as_array()
        .and_then(|rows| rows.first())
        .unwrap_or_else(|| panic!("sem find returned no rows: {cli_find}"));
    let cli_file = cli_find_row["file"].as_str().unwrap().to_string();
    let cli_line = cli_find_row["start_line"].as_u64().unwrap();

    let cli_grep: Value = cli_json(repo.path(), &["grep", "NEEDLE_GREP_MARKER_XYZ", "--json"]);
    let cli_grep_hit = cli_grep["hits"]
        .as_array()
        .and_then(|rows| rows.first())
        .unwrap_or_else(|| panic!("sem grep returned no hits: {cli_grep}"));
    let grep_file = cli_grep_hit["file"].as_str().unwrap().to_string();
    let grep_line = cli_grep_hit["line"].as_u64().unwrap();

    let mut client = McpClient::spawn(repo.path());
    let tools = client.tools_list();
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["name"].as_str().map(String::from))
        .collect();

    assert!(
        names.iter().any(|n| n == "sem_find"),
        "sem_find missing from tools/list: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "sem_grep"),
        "sem_grep missing from tools/list: {names:?}"
    );

    let find_resp = client.call_tool("sem_find", json!({"query": "needle_target_fn"}));
    let find_text = tool_text(&find_resp);
    assert!(
        find_text.contains(&cli_file) && find_text.contains(&cli_line.to_string()),
        "sem_find over MCP must match `sem find` CLI: expected {cli_file}:{cli_line} in:\n{find_text}"
    );

    let grep_resp = client.call_tool("sem_grep", json!({"pattern": "NEEDLE_GREP_MARKER_XYZ"}));
    let grep_text = tool_text(&grep_resp);
    assert!(
        grep_text.contains(&grep_file) && grep_text.contains(&grep_line.to_string()),
        "sem_grep over MCP must match `sem grep` CLI: expected {grep_file}:{grep_line} in:\n{grep_text}"
    );
}

// ── (c) format:"json" for entities + context ──

#[test]
fn entities_and_context_support_format_json_with_cli_fields() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let entities_resp = client.call_tool(
        "sem_entities",
        json!({"path": "src/needle.py", "format": "json"}),
    );
    let entities_text = tool_text(&entities_resp);
    let rows: Value = serde_json::from_str(&entities_text).unwrap_or_else(|e| {
        panic!("sem_entities format=json did not return JSON: {e}\ngot:\n{entities_text}")
    });
    let target = rows
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["name"] == "needle_target_fn"))
        .unwrap_or_else(|| panic!("needle_target_fn missing from JSON rows: {rows}"));

    // Field names match the CLI's `EntityJsonRow` (crates/sem-cli/src/commands/entities.rs):
    // name, type, start_line, end_line, start_byte, end_byte, parent_id.
    assert_eq!(target["type"], "function", "row: {target}");
    assert!(target["start_line"].is_u64(), "row: {target}");
    assert!(target["end_line"].is_u64(), "row: {target}");
    assert!(
        target.get("start_byte").is_some_and(Value::is_u64),
        "missing/wrong-typed start_byte: {target}"
    );
    assert!(
        target.get("end_byte").is_some_and(Value::is_u64),
        "missing/wrong-typed end_byte: {target}"
    );
    assert!(
        target.get("parent_id").is_some(),
        "missing parent_id key (should be null or a string): {target}"
    );

    let context_resp = client.call_tool(
        "sem_context",
        json!({"entity_name": "needle_target_fn", "format": "json"}),
    );
    let context_text = tool_text(&context_resp);
    let context: Value = serde_json::from_str(&context_text).unwrap_or_else(|e| {
        panic!("sem_context format=json did not return JSON: {e}\ngot:\n{context_text}")
    });
    let entries = context["entries"]
        .as_array()
        .unwrap_or_else(|| panic!("context json missing entries array: {context}"));
    assert!(!entries.is_empty(), "context entries empty: {context}");
    let entry = &entries[0];
    // Field names match the CLI's context --format json entry shape
    // (crates/sem-cli/src/commands/context.rs render_context): entityId, name,
    // type, file, role, tokens, content.
    assert!(entry.get("entityId").is_some(), "entry: {entry}");
    assert!(entry.get("name").is_some(), "entry: {entry}");
    assert!(entry.get("content").is_some(), "entry: {entry}");
}

// ── Batch (multi-query) forms ──

#[test]
fn find_batch_queries_resolve_independently() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_find",
        json!({"queries": ["needle_target_fn", "does_not_exist_zzz"], "format": "json"}),
    );
    let rows: Value = serde_json::from_str(&tool_text(&resp)).expect("batch find json");
    let rows = rows.as_array().expect("array");
    assert_eq!(rows.len(), 2, "one entry per query, in order: {rows:?}");
    assert_eq!(rows[0]["query"], "needle_target_fn");
    assert_eq!(rows[0]["matches"][0]["file"], "src/needle.py");
    assert_eq!(rows[1]["query"], "does_not_exist_zzz");
    assert_eq!(
        rows[1]["matches"].as_array().unwrap().len(),
        0,
        "a miss is an empty entry, never an error for the batch"
    );
}

#[test]
fn grep_batch_patterns_report_separately() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_grep",
        json!({"patterns": ["NEEDLE_GREP_MARKER_XYZ", "no_such_text_zzz"], "format": "json"}),
    );
    let results: Value = serde_json::from_str(&tool_text(&resp)).expect("batch grep json");
    let results = results.as_array().expect("array");
    assert_eq!(results.len(), 2, "one entry per pattern: {results:?}");
    assert_eq!(results[0]["pattern"], "NEEDLE_GREP_MARKER_XYZ");
    assert_eq!(results[0]["hits"][0]["file"], "src/marker.py");
    assert_eq!(results[1]["pattern"], "no_such_text_zzz");
    assert_eq!(results[1]["hits"].as_array().unwrap().len(), 0);
}

#[test]
fn context_batch_entities_pack_one_block_each() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_context",
        json!({"entities": ["needle_target_fn", "unrelated_other"], "format": "json"}),
    );
    let text = tool_text(&resp);
    let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
    assert_eq!(
        lines.len(),
        2,
        "json batch is newline-delimited, one object per entity:\n{text}"
    );
    let first: Value = serde_json::from_str(lines[0]).expect("first entity json");
    let second: Value = serde_json::from_str(lines[1]).expect("second entity json");
    assert_eq!(first["entity"], "needle_target_fn");
    assert_eq!(second["entity"], "unrelated_other");
}

// ── sem_callers ──

/// Extract the text of a tool-level *error* response — the inverse of
/// `tool_text`: panics unless the tool reported `isError`.
fn tool_error_text(resp: &Value) -> String {
    if let Some(err) = resp.get("error") {
        panic!("tools/call returned a JSON-RPC error: {err}");
    }
    let result = &resp["result"];
    assert_eq!(
        result.get("isError"),
        Some(&Value::Bool(true)),
        "expected a tool-level error: {result}"
    );
    result["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("tool result content[0] is not text: {result}"))
        .to_string()
}

/// A repo where `target_fn` has exactly two same-file callers and
/// `dup_name` is defined in two different files — the two shapes
/// `sem_callers` has to get right (limit, and the ambiguity refusal).
fn callers_fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@t.com"]);
    git(root, &["config", "user.name", "test"]);

    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(
        root.join("src/app.py"),
        "def target_fn():\n    return 0\n\n\ndef caller_a():\n    return target_fn()\n\n\ndef caller_b():\n    return target_fn() + caller_a()\n",
    )
    .unwrap();
    std::fs::write(root.join("src/dup.py"), "def dup_name():\n    return 1\n").unwrap();
    std::fs::write(root.join("src/dup2.py"), "def dup_name():\n    return 2\n").unwrap();

    git(root, &["add", "-A"]);
    let status = Command::new("git")
        .current_dir(root)
        .args(["commit", "-q", "-m", "fixture"])
        .status()
        .unwrap();
    assert!(status.success());

    dir
}

#[test]
fn callers_lists_direct_callers_and_honors_limit() {
    let repo = callers_fixture();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_callers",
        json!({"query": "target_fn", "format": "json"}),
    );
    let out: Value = serde_json::from_str(&tool_text(&resp)).expect("callers json");
    assert_eq!(out["entity"]["name"], "target_fn");
    assert_eq!(
        out["callers"].as_array().unwrap().len(),
        2,
        "both direct callers listed: {out}"
    );

    let resp = client.call_tool(
        "sem_callers",
        json!({"query": "target_fn", "format": "json", "limit": 1}),
    );
    let out: Value = serde_json::from_str(&tool_text(&resp)).expect("capped callers json");
    assert_eq!(
        out["callers"].as_array().unwrap().len(),
        1,
        "limit caps the caller rows: {out}"
    );
}

#[test]
fn callers_refuses_ambiguous_names_with_the_candidate_list() {
    let repo = callers_fixture();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool("sem_callers", json!({"query": "dup_name"}));
    let text = tool_error_text(&resp);
    assert!(
        text.contains("src/dup.py") && text.contains("src/dup2.py"),
        "refusal lists every candidate definition: {text}"
    );

    // The file param picks one and the same query then answers.
    let resp = client.call_tool(
        "sem_callers",
        json!({"query": "dup_name", "file": "src/dup.py", "format": "json"}),
    );
    let out: Value = serde_json::from_str(&tool_text(&resp)).expect("disambiguated json");
    assert_eq!(out["entity"]["file"], "src/dup.py");
}

// ── format:"json" on the entities text-search and query-ranking shapes ──

#[test]
fn entities_text_search_supports_format_json() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_entities",
        json!({"text": "needle_target_fn_helper_a():", "format": "json"}),
    );
    let rows: Value = serde_json::from_str(&tool_text(&resp)).expect("text-search json");
    let rows = rows.as_array().expect("array");
    assert_eq!(rows.len(), 1, "one hit for a unique needle: {rows:?}");
    assert_eq!(rows[0]["file"], "src/needle.py");
    assert_eq!(rows[0]["entity"], "needle_target_fn_helper_a");
    assert_eq!(rows[0]["type"], "function");
    assert!(
        rows[0]["text"]
            .as_str()
            .unwrap()
            .contains("needle_target_fn_helper_a"),
        "the full matched line, not a clipped preview: {rows:?}"
    );
    assert!(rows[0]["line"].is_u64());
}

#[test]
fn entities_query_mode_supports_format_json() {
    let repo = fixture_repo();
    let mut client = McpClient::spawn(repo.path());

    let resp = client.call_tool(
        "sem_entities",
        json!({"query": "needle_target_fn", "format": "json", "limit": 3}),
    );
    let rows: Value = serde_json::from_str(&tool_text(&resp)).expect("query-mode json");
    let rows = rows.as_array().expect("array");
    assert_eq!(rows.len(), 3, "limit honored: {rows:?}");
    assert_eq!(
        rows[0]["name"], "needle_target_fn",
        "exact-name relevance still ranks first in json: {rows:?}"
    );
    for row in rows {
        assert!(row["dependents"].is_u64(), "dependent count carried: {row}");
        assert!(row["file"].is_string() && row["start_line"].is_u64());
    }
}

// ── ABAP over MCP (story 2.8) ──

/// The ABAP fixture files in an abapGit `src/` layout (not under a directory named `fixtures`,
/// which the default excludes skip), committed twice: the second commit edits
/// `zcl_fx_order.describe`.
fn abap_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    let root = dir.path();
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "t@t.com"]);
    git(root, &["config", "user.name", "test"]);
    std::fs::create_dir_all(root.join("src")).unwrap();
    let from = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sem-core/tests/fixtures/abap");
    for stem in ["zcl_fx_order", "zcl_fx_order_sub", "zcl_fx_user", "zcl_fx_other", "zif_fx_order", "zfx_dynamic", "zfx_report"] {
        for entry in std::fs::read_dir(&from).unwrap() {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with(&format!("{stem}.")) {
                std::fs::copy(entry.path(), root.join("src").join(&name)).unwrap();
            }
        }
    }
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "abap fixtures"]);
    let order = root.join("src/zcl_fx_order.clas.abap");
    let edited = std::fs::read_to_string(&order).unwrap().replace("rv_text = |order", "rv_text = |ORDER");
    std::fs::write(&order, edited).unwrap();
    git(root, &["commit", "-q", "-am", "describe says ORDER"]);
    dir
}

fn json_text(client: &mut McpClient, tool: &str, args: Value) -> Value {
    let resp = client.call_tool(tool, args);
    serde_json::from_str(&tool_text(&resp)).unwrap_or_else(|e| panic!("{tool} did not return JSON: {e}\n{resp}"))
}

fn ids(rows: &Value) -> Vec<String> {
    let mut v: Vec<String> = rows.as_array().unwrap().iter().map(|r| r["id"].as_str().unwrap().to_string()).collect();
    v.sort();
    v
}

#[test]
fn mcp_abap_serves_find_impact_certify() {
    let repo = abap_repo();
    let order_file = "src/zcl_fx_order.clas.abap";
    let mut client = McpClient::spawn(repo.path());

    // sem_find: the name in any case is the same entity, as the CLI answers it
    let upper = json_text(&mut client, "sem_find", json!({"query": "ZCL_FX_ORDER", "format": "json"}));
    let lower = json_text(&mut client, "sem_find", json!({"query": "zcl_fx_order", "format": "json"}));
    assert_eq!(upper, lower);
    assert_eq!(upper.as_array().unwrap().len(), 1, "{upper}");
    assert_eq!(upper[0]["name"], "zcl_fx_order");
    let cli = cli_json(repo.path(), &["find", "ZCL_FX_ORDER", "--json"]);
    assert_eq!(ids(&upper), ids(&cli));

    // sem_find callers on `create`: the caller from another file, and the CLI's verdict
    let mcp = json_text(&mut client, "sem_find", json!({"query": "CREATE", "mode": "callers", "file": order_file, "format": "json"}));
    let cli = cli_json(repo.path(), &["find", "create", "--callers", "--file", order_file, "--json"]);
    let cli = &cli[0];
    let callers = ids(&mcp["callers"]);
    assert!(callers.iter().any(|id| id.starts_with("src/zcl_fx_user.clas.abap::")), "{callers:?}");
    for id in ids(&cli["related"]) {
        assert!(callers.contains(&id), "CLI caller {id} missing over MCP: {callers:?}");
    }
    for key in ["complete", "incomplete_because", "checked", "possible_callers", "possible_caller_sites"] {
        assert_eq!(mcp[key], cli[key], "verdict field {key}");
    }
    assert_eq!(mcp["complete"], false, "a dynamic call reaches every method of the class");

    // the constructor is called where an instance is made (`NEW zcl_fx_order( 1 )`, `NEW #( )`
    // in `create`, story 2.6), and the computed call still makes the set incomplete
    let text = tool_text(&client.call_tool("sem_find", json!({"query": "constructor", "mode": "callers", "file": order_file})));
    assert!(!text.contains("(callers: none)\n"), "{text}");
    assert!(text.contains("method create") && text.contains("INCOMPLETE"), "{text}");
    assert!(text.contains("zfx_dynamic.prog.abap") && text.contains("dynamic call"), "{text}");
    let ctor = json_text(&mut client, "sem_callers", json!({"query": "constructor", "file": order_file, "format": "json"}));
    assert_eq!(ctor["complete"], false);
    assert!(!ctor["possible_callers"].as_array().unwrap().is_empty(), "{ctor}");

    // sem_impact: `Class.method` in any case, tests mode, the tests the CLI lists
    let resp = client.call_tool("sem_impact", json!({"file_path": order_file, "entity_name": "zcl_fx_order.create", "mode": "tests"}));
    let impact = tool_text(&resp);
    let cli = cli_json(repo.path(), &["impact", "zcl_fx_order.create", "--tests", "--json"]);
    let cli_tests: Vec<&str> = cli["tests"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert!(cli_tests.contains(&"setup"), "{cli}");
    for name in cli_tests {
        assert!(impact.contains(name), "CLI test {name} missing over MCP:\n{impact}");
    }
    assert!(impact.contains("zcl_fx_order.clas.testclasses.abap"), "{impact}");
    let folded = tool_text(&client.call_tool("sem_impact", json!({"file_path": order_file, "entity_name": "ZCL_FX_ORDER.CREATE", "mode": "tests"})));
    assert!(folded.contains("setup") && folded.contains("zcl_fx_order.clas.testclasses.abap"), "{folded}");

    // sem_certify over the two-commit range: the edited method and its callers, text and JSON
    let cert = json_text(&mut client, "sem_certify", json!({"range": "HEAD~1..HEAD", "format": "json"}));
    let cli = cli_json(repo.path(), &["certify", "HEAD~1..HEAD", "--json"]);
    assert_eq!(cert["entities"], cli["entities"]);
    assert_eq!(cert["entities"][0]["name"], "describe");
    assert_eq!(cert["entities"][0]["file"], order_file);
    let incomplete = cert["callerSetsIncomplete"].as_array().unwrap();
    assert_eq!(incomplete.len(), 1, "{cert}");
    assert_eq!(incomplete[0]["entity"], "describe");
    assert!(
        incomplete[0]["possibleCallersNotModified"].as_array().unwrap().iter().any(|c| c["file"] == "src/zfx_dynamic.prog.abap"),
        "{}",
        incomplete[0]
    );
    assert_eq!(cert["callerSetsIncomplete"], cli["callerSetsIncomplete"]);
    let text = tool_text(&client.call_tool("sem_certify", json!({"range": "HEAD~1..HEAD"})));
    assert!(text.contains("describe") && text.contains("zfx_dynamic.prog.abap"), "{text}");
}

#[test]
fn mcp_callers_skip_a_same_named_test_and_carry_call_lines() {
    // A test class whose test method is named like the global `create` it calls: callers of
    // `create` answer for the global one and name the test, as the CLI does, with each
    // caller's call lines
    let repo = abap_repo();
    std::fs::write(
        repo.path().join("src/zcl_fx_spy.clas.testclasses.abap"),
        "CLASS ltc_spy DEFINITION FINAL FOR TESTING\n  DURATION SHORT RISK LEVEL HARMLESS.\n  PRIVATE SECTION.\n    METHODS create FOR TESTING.\nENDCLASS.\n\nCLASS ltc_spy IMPLEMENTATION.\n  METHOD create.\n    cl_abap_unit_assert=>assert_bound( zcl_fx_order=>create( 3 ) ).\n  ENDMETHOD.\nENDCLASS.\n",
    )
    .unwrap();
    git(repo.path(), &["add", "-A"]);
    git(repo.path(), &["commit", "-q", "-m", "a test named create"]);
    let mut client = McpClient::spawn(repo.path());

    let mcp = json_text(
        &mut client,
        "sem_find",
        json!({"query": "create", "mode": "callers", "format": "json"}),
    );
    assert_eq!(mcp["entity"]["file"], "src/zcl_fx_order.clas.abap", "{mcp}");
    assert_eq!(
        ids(&mcp["skipped"]),
        ["src/zcl_fx_spy.clas.testclasses.abap::class::ltc_spy::create"],
        "{mcp}"
    );
    let run = mcp["callers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "run")
        .expect("run calls create");
    assert_eq!(run["call_lines"], json!([13]), "{run}");
    let cli = cli_json(repo.path(), &["find", "create", "--callers", "--json"]);
    assert_eq!(ids(&mcp["skipped"]), ids(&cli[0]["skipped"]));
    for caller in cli[0]["related"].as_array().unwrap() {
        let over_mcp = mcp["callers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == caller["id"])
            .expect("same callers");
        assert_eq!(over_mcp["call_lines"], caller["call_lines"], "{caller}");
    }

    let text = tool_text(&client.call_tool("sem_callers", json!({"query": "create"})));
    assert!(
        text.contains("\n  skipped 1 test definition of the same name: method create src/zcl_fx_spy.clas.testclasses.abap:8\n"),
        "{text}"
    );
    assert!(
        text.contains(
            "  method run src/zcl_fx_user.clas.abap:12 (call at src/zcl_fx_user.clas.abap:13)\n"
        ),
        "{text}"
    );

    // two definitions that are not tests are still refused
    let text = tool_error_text(&client.call_tool("sem_callers", json!({"query": "describe"})));
    assert!(text.contains("matches 3 definitions"), "{text}");
}
