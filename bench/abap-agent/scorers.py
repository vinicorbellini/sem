"""Scorers for the ABAP agent benchmark.

B1 where-used:      precision/recall of the calling methods against ground_truth/whereused.json.
B2 change-and-verify: hidden tests + `npm run unit` in a scratch copy of the agent's checkout,
                    the new parameter's signature, and call sites that do not pass it.
B3 review:          changed entities and callers left behind against ground_truth/b3_rubric.json.

Every scorer returns a dict; "success_score" is the one number per run that goes in the CSV.
"""

import json
import re
import shutil
import subprocess
import time
from pathlib import Path

# ── Config ──────────────────────────────────────────────────────────────────

LINE_TOLERANCE = 2        # B1: a reported line within this many lines of the true call matches
NPM_UNIT_TIMEOUT_S = 1800
HIDDEN_TEST_CLASS = "ltcl_hidden_b2"

TEST_LINE = re.compile(r"^(\w+): running (\w+)->(\w+)(, skipped.*)?$", re.M)
SYNTAX_ERROR = re.compile(r"((?:check_syntax|parser_error), .+)$", re.M)


# ── Answer parsing ──────────────────────────────────────────────────────────


def parse_final_json(text: str, key: str):
    """The last JSON object in the answer that has `key`, or None."""
    if not text:
        return None
    decoder = json.JSONDecoder()
    found = None
    for start in [i for i, c in enumerate(text) if c == "{"]:
        try:
            data, _ = decoder.raw_decode(text[start:])
        except json.JSONDecodeError:
            continue
        if isinstance(data, dict) and key in data:
            found = data
    return found


def _norm_file(path: str) -> str:
    path = (path or "").strip().replace("\\", "/").lower()
    return path[2:] if path.startswith("./") else path


def _method_part(name: str) -> str:
    """'ZCL_FOO->ZIF_BAR~RUN' -> 'zif_bar~run'; 'zcl_foo=>bar' -> 'bar'; 'bar' -> 'bar'."""
    name = (name or "").strip().lower()
    for sep in ("=>", "->"):
        if sep in name:
            name = name.rsplit(sep, 1)[1]
    return name.strip("() ")


def set_scores(predicted: set, truth: set) -> dict:
    tp = len(predicted & truth)
    precision = tp / len(predicted) if predicted else 0.0
    recall = tp / len(truth) if truth else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0.0
    return {"precision": round(precision, 4), "recall": round(recall, 4), "f1": round(f1, 4),
            "tp": tp, "fp": len(predicted - truth), "fn": len(truth - predicted)}


# ── B1 ──────────────────────────────────────────────────────────────────────


def load_whereused(path: Path, task_id: str):
    """Ground truth for one task, or None when the file or the task is not there yet.

    Format (convert `sapcli whereused` output to it):
    {"abapgit_commit": "...", "targets": {"b1_01": [{"method": "<class>-><method>", "file": "src/...", "line": 12}]}}
    """
    if not path.exists():
        return None
    data = json.loads(path.read_text())
    return data.get("targets", {}).get(task_id)


def score_b1(answer: str, truth) -> dict:
    parsed = parse_final_json(answer, "callers")
    callers = parsed.get("callers", []) if parsed else []
    callers = [c for c in callers if isinstance(c, dict)]
    result = {"parsed": parsed is not None, "predicted_count": len(callers)}
    if truth is None:
        result["error"] = "no ground truth yet (ground_truth/whereused.json)"
        return result

    predicted = {(_norm_file(c.get("file")), _method_part(c.get("method"))) for c in callers}
    expected = {(_norm_file(t["file"]), _method_part(t["method"])) for t in truth}
    method_scores = set_scores(predicted, expected)

    # Line level: one-to-one, same file, within LINE_TOLERANCE.
    remaining = [(_norm_file(t["file"]), int(t["line"])) for t in truth]
    line_tp = 0
    for c in callers:
        try:
            key = (_norm_file(c.get("file")), int(c.get("line")))
        except (TypeError, ValueError):
            continue
        hit = next((t for t in remaining if t[0] == key[0] and abs(t[1] - key[1]) <= LINE_TOLERANCE), None)
        if hit:
            remaining.remove(hit)
            line_tp += 1

    result.update({
        "truth_count": len(expected),
        "precision": method_scores["precision"],
        "recall": method_scores["recall"],
        "f1": method_scores["f1"],
        "line_precision": round(line_tp / len(callers), 4) if callers else 0.0,
        "line_recall": round(line_tp / len(truth), 4) if truth else 0.0,
        "false_positives": sorted("::".join(p) for p in predicted - expected),
        "false_negatives": sorted("::".join(t) for t in expected - predicted),
        "success_score": method_scores["f1"],
    })
    return result


# ── B2 ──────────────────────────────────────────────────────────────────────


def _git(args: list[str], cwd: Path, **kw) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(cwd), *args], capture_output=True, **kw)


def workspace_patch(workspace: Path) -> bytes:
    """The agent's changes, untracked files included, as a binary patch against HEAD.

    node_modules (excluded in .git/info/exclude) and output/ (abapGit's .gitignore) stay out.
    """
    _git(["add", "-A"], workspace, check=True)
    return _git(["diff", "--cached", "--binary", "HEAD"], workspace, check=True).stdout


def statement_at(text: str, start: int) -> str:
    """The ABAP statement text from `start` to its closing period (outside literals, brackets, comments)."""
    depth, i, n = 0, start, len(text)
    quote = None
    while i < n:
        c = text[i]
        if quote:
            if c == quote:
                quote = None
        elif c in "'`|":
            quote = c
        elif c == '"':
            j = text.find("\n", i)
            i = n if j < 0 else j
            continue
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
        elif c == "." and depth <= 0:
            return text[start:i + 1]
        i += 1
    return text[start:]


def call_sites_missing(root: Path, method: str, param: str) -> list[str]:
    """Calls of `method` under src/ whose statement does not name `param`."""
    call = re.compile(r"(?:->|=>)\s*" + re.escape(method) + r"\s*\(|CALL\s+METHOD\s+\S*?(?:->|=>)"
                      + re.escape(method) + r"\b", re.I)
    named = re.compile(r"\b" + re.escape(param) + r"\s*=", re.I)
    missing = []
    for path in sorted(root.glob("src/**/*.abap")):
        text = path.read_text(errors="replace")
        for m in call.finditer(text):
            line_start = text.rfind("\n", 0, m.start()) + 1
            if text[line_start:m.start()].lstrip().startswith(("*", '"')):
                continue
            if not named.search(statement_at(text, m.start())):
                line = text.count("\n", 0, m.start()) + 1
                missing.append(f"{path.relative_to(root)}:{line}")
    return missing


def signature_check(root: Path, task: dict) -> dict:
    """Is `param` in the method's definition, mandatory (no DEFAULT, no OPTIONAL)?"""
    text = (root / task["defined_in"]).read_text(errors="replace")
    m = re.search(r"^\s*(?:CLASS-)?METHODS:?\s+" + re.escape(task["method"]) + r"\b", text, re.I | re.M)
    if not m:
        return {"signature_ok": False, "signature_note": "method definition not found"}
    definition = statement_at(text, m.start())
    p = re.search(r"!?\b" + re.escape(task["param"]) + r"\b[^\n,.]*", definition, re.I)
    if not p:
        return {"signature_ok": False, "signature_note": "parameter not in the definition"}
    optional = re.search(r"\b(DEFAULT|OPTIONAL)\b", p.group(0), re.I)
    return {"signature_ok": not optional,
            "signature_note": "parameter is optional" if optional else "mandatory parameter present"}


def run_unit(root: Path, timeout_s: int = NPM_UNIT_TIMEOUT_S) -> dict:
    start = time.time()
    try:
        r = subprocess.run(["npm", "run", "unit"], cwd=root, capture_output=True, text=True,
                           errors="replace", timeout=timeout_s)
        output, code = r.stdout + r.stderr, r.returncode
    except subprocess.TimeoutExpired as e:
        output, code = (e.stdout or "") if isinstance(e.stdout, str) else "", -1
    ran = [m for m in TEST_LINE.finditer(output) if not m.group(4)]
    return {
        "exit_code": code,
        "seconds": round(time.time() - start, 1),
        "syntax_errors": SYNTAX_ERROR.findall(output)[:20],
        "test_methods_run": len(ran),
        "test_classes_run": len({(m.group(1), m.group(2)) for m in ran}),
        "hidden_methods_run": sum(1 for m in ran if m.group(2).lower() == HIDDEN_TEST_CLASS),
        "last_test": ran[-1].group(0) if ran else None,
        "output_tail": "\n".join(output.splitlines()[-15:]),
    }


def score_b2(task: dict, workspace: Path, base: Path, hidden_dir: Path, scratch: Path) -> dict:
    """Apply the agent's patch to a fresh copy of the pinned checkout, add the hidden tests, run the suite."""
    if scratch.exists():
        shutil.rmtree(scratch)
    patch = workspace_patch(workspace)
    _git(["clone", "-q", "--no-hardlinks", str(base), str(scratch)], base.parent, check=True)
    (scratch / "node_modules").symlink_to(base / "node_modules")
    if patch:
        applied = subprocess.run(["git", "-C", str(scratch), "apply", "--binary", "-"], input=patch, capture_output=True)
        if applied.returncode != 0:
            return {"success_score": 0.0, "tests_passed": False, "patch_bytes": len(patch),
                    "error": "patch did not apply: " + applied.stderr.decode(errors="replace")[:300]}

    result = {"patch_bytes": len(patch)}
    result.update(signature_check(scratch, task))
    missing = call_sites_missing(scratch, task["method"], task["param"])
    result["callers_missed"] = len(missing)
    result["callers_missed_at"] = missing

    hidden = (hidden_dir / f"{task['id']}.testclasses.abap").read_text()
    expected_hidden = len(re.findall(r"\bFOR TESTING\b", hidden)) - 1  # minus the class's own FOR TESTING
    with open(scratch / task["testclasses"], "a") as f:
        f.write(hidden)

    unit = run_unit(scratch)
    result["unit"] = unit
    result["tests_passed"] = unit["exit_code"] == 0 and unit["hidden_methods_run"] == expected_hidden
    result["success_score"] = 1.0 if (result["tests_passed"] and result["signature_ok"]) else 0.0
    return result


# ── B3 ──────────────────────────────────────────────────────────────────────


def load_rubric(path: Path, task_id: str):
    if not path.exists():
        return None
    return json.loads(path.read_text()).get("tasks", {}).get(task_id)


IDENTIFIER = re.compile(r"[a-z0-9_]+")
ABAPGIT_PREFIX = re.compile(r"^z(?:cl|if|cx)_abapgit_")
TEST_CLASS_ITEM = re.compile(r"^\s*ltc\w*\s*(?:->|=>)", re.I)


def _identifiers(item: str) -> set[str]:
    """Whole identifiers in an answer item, each also without its zcl_abapgit_ / zif_abapgit_ prefix."""
    tokens = set(IDENTIFIER.findall(item.lower()))
    return tokens | {ABAPGIT_PREFIX.sub("", t) for t in tokens}


def _matches(item: str, rubric_entry: dict) -> bool:
    """An alias matches a whole identifier of the item, not any substring of it.

    Substring matching let 'warning_overwrite_files' count for the 'overwrite_files' alias of
    another entity, 'ty_deserialize_checks' for 'deserialize_checks', 'receive_pack_push' for
    'receive_pack', and the test method 'ltcl_git_utils->pkt_string_utf8' for 'pkt_string'.
    """
    words = _identifiers(item)
    return any(alias.lower() in words for alias in rubric_entry["aliases"])


def _is_test_class_item(item: str) -> bool:
    """'ltcl_foo->bar': the rubric does not list test classes, so such an item is neither hit nor miss."""
    return bool(TEST_CLASS_ITEM.match(item))


def score_b3(answer: str, rubric) -> dict:
    parsed = parse_final_json(answer, "changed_entities")
    result = {"parsed": parsed is not None}
    if rubric is None:
        result["error"] = "no rubric (ground_truth/b3_rubric.json)"
        return result
    listed = [str(e) for e in (parsed or {}).get("changed_entities", [])]
    entities = [e for e in listed if not _is_test_class_item(e)]
    callers = [c for c in (parsed or {}).get("callers_left_behind", []) if isinstance(c, dict)]
    caller_text = [f"{c.get('caller', '')} {c.get('file', '')}" for c in callers]

    want = rubric["changed_entities"]
    found = [e for e in want if any(_matches(item, e) for item in entities)]
    relevant = [item for item in entities if any(_matches(item, e) for e in want)]
    precision = len(relevant) / len(entities) if entities else 0.0
    recall = len(found) / len(want) if want else 0.0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0.0

    left = rubric["callers_left_behind"]
    callers_found = [c for c in left if any(_matches(item, c) for item in caller_text)]
    caller_recall = len(callers_found) / len(left) if left else None
    # Reported, not scored: the rubric is a draft, and a caller it does not list may still be a fair find.
    callers_relevant = [item for item in caller_text if any(_matches(item, c) for c in left)]

    result.update({
        "test_class_items_ignored": [e for e in listed if e not in entities],
        "caller_precision": round(len(callers_relevant) / len(caller_text), 4) if caller_text else None,
        "entity_precision": round(precision, 4), "entity_recall": round(recall, 4), "entity_f1": round(f1, 4),
        "entities_missed": [e["id"] for e in want if e not in found],
        "caller_recall": None if caller_recall is None else round(caller_recall, 4),
        "rubric_callers_missed": [c["id"] for c in left if c not in callers_found],
        "precision": round(precision, 4), "recall": round(recall, 4),
        "success_score": round(f1 if caller_recall is None else (f1 + caller_recall) / 2, 4),
    })
    return result
