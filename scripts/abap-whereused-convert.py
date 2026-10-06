#!/usr/bin/env python3
"""Convert ABAP where-used ground truth between its two shapes.

Source shape (crates/sem-core/tests/fixtures/abap/inside-sap/README.md): an
object keyed by `CLASS=>METHOD` (`INTF~METHOD`), each value the array of
`{object, type, include, line}` rows `sapcli whereused` returns. The grep
fallback (`whereused.grep.json`) writes the same shape, with `include` already
the calling method and the file in `file`; keys starting with `_` are
metadata and skipped.

Harness shape (bench/abap-agent/README.md, read by `scorers.load_whereused`):
  {"abapgit_commit": "...",
   "targets": {"b1_01": [{"method": "<class>-><method>", "file": "src/...", "line": 12}]}}
keyed by the task ids of bench/abap-agent/tasks/b1_whereused.json.

The match unit of both is the caller entity, (object, calling method), case-
folded. A sapcli `include` is resolved to a method once, here:

- a row that carries `file` (the grep fallback) keeps it, and its `include`
  is the method already;
- otherwise the file comes from abapGit naming: a class's `...CCAU` include is
  `<class>.clas.testclasses.abap`, `...CCIMP` `.clas.locals_imp.abap`,
  `...CCDEF` `.clas.locals_def.abap`, `...CCMAC` `.clas.macros.abap`, any
  other class include (`...CM001`, the method includes) `<class>.clas.abap`;
  an interface `<intf>.intf.abap`; a program `<prog>.prog.abap`; a function
  group include `<group>.fugr.<include>.abap`;
- a method include's name says nothing about the method (SAP numbers them in
  TMDIR), so `--method-map` takes a JSON object `{include: method}` exported
  with the where-used; an include not in it is kept as the method name, and
  will not match. sapcli's `line` is a line of that include, not of the
  abapGit file: for a method include the harness's line score is off by the
  method's start line, its method-level score is not.

Usage:
  abap-whereused-convert.py to-harness   SRC.json --tasks TASKS.json --out H.json [--method-map M.json]
  abap-whereused-convert.py from-harness H.json   --tasks TASKS.json --out SRC.json
"""

import argparse
import json
import re
import sys
from pathlib import Path

PINNED_COMMIT = "b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0"


def key_of_target(target: str) -> str:
    """'zcl_x=>m' / 'zcl_x->m' -> 'ZCL_X=>M'; 'zif_x~m' -> 'ZIF_X~M'."""
    t = target.strip()
    if "~" in t and "=>" not in t and "->" not in t:
        owner, method = t.split("~", 1)
        return f"{owner.upper()}~{method.upper()}"
    owner, method = re.split(r"=>|->", t, maxsplit=1)
    return f"{owner.upper()}=>{method.upper()}"


def file_of(row: dict, folders: dict) -> str:
    if row.get("file"):
        return row["file"]
    obj = row["object"].lower()
    kind = row.get("type", "").upper()
    include = (row.get("include") or "").upper()
    if kind == "CLAS":
        suffix = ".clas.abap"
        for tail, part in (("CCAU", "testclasses"), ("CCIMP", "locals_imp"),
                           ("CCDEF", "locals_def"), ("CCMAC", "macros")):
            if include.endswith(tail):
                suffix = f".clas.{part}.abap"
        name = obj + suffix
    elif kind == "INTF":
        name = obj + ".intf.abap"
    elif kind == "FUGR":
        name = f"{obj}.fugr.{include.lower()}.abap"
    else:
        name = obj + ".prog.abap"
    folder = folders.get(name)
    return f"{folder}/{name}" if folder else f"src/{name}"


def method_of(row: dict, method_map: dict) -> str:
    include = row.get("include") or ""
    return method_map.get(include, method_map.get(include.upper(), include)).lower()


def to_harness(src: dict, tasks: dict, method_map: dict, folders: dict) -> dict:
    targets = {}
    for task in tasks["tasks"]:
        rows = src.get(key_of_target(task["target"]))
        if rows is None:
            continue
        out = []
        for row in rows:
            method = method_of(row, method_map)
            owner = (row.get("caller") or "").rsplit("->", 1)[0] if "->" in (row.get("caller") or "") else row["object"].lower()
            out.append({"method": f"{owner}->{method}", "file": file_of(row, folders), "line": int(row["line"])})
        targets[task["id"]] = out
    commit = src.get("_meta", {}).get("abapgit_commit", tasks.get("abapgit_commit", PINNED_COMMIT))
    return {"abapgit_commit": commit, "targets": targets}


def from_harness(harness: dict, tasks: dict) -> dict:
    out = {"_meta": {"abapgit_commit": harness.get("abapgit_commit"),
                     "source": "converted from the bench/abap-agent harness shape"}}
    by_id = {t["id"]: t for t in tasks["tasks"]}
    for task_id, rows in harness.get("targets", {}).items():
        key = key_of_target(by_id[task_id]["target"])
        converted = []
        for r in rows:
            name = Path(r["file"]).name
            parts = name.split(".")
            caller = r["method"]
            converted.append({
                "object": parts[0].upper(),
                "type": parts[1].upper() if len(parts) > 2 else "PROG",
                "include": re.split(r"=>|->", caller)[-1].lower(),
                "line": int(r["line"]),
                "file": r["file"],
                "caller": caller,
            })
        out[key] = converted
    return out


def repo_folders(repo: Path) -> dict:
    """abapGit file name -> its folder, for rows that carry no file."""
    if not repo or not (repo / "src").is_dir():
        return {}
    return {p.name: str(p.parent.relative_to(repo)) for p in (repo / "src").rglob("*.abap")}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("direction", choices=["to-harness", "from-harness"])
    ap.add_argument("input")
    ap.add_argument("--tasks", required=True, help="bench/abap-agent/tasks/b1_whereused.json")
    ap.add_argument("--out", required=True)
    ap.add_argument("--method-map", help="JSON {SAP include: method} for sapcli rows")
    ap.add_argument("--repo", default="/tmp/claude-0/abapGit", help="abapGit checkout, to place files in their folders")
    args = ap.parse_args()

    data = json.loads(Path(args.input).read_text())
    tasks = json.loads(Path(args.tasks).read_text())
    if args.direction == "to-harness":
        method_map = json.loads(Path(args.method_map).read_text()) if args.method_map else {}
        out = to_harness(data, tasks, method_map, repo_folders(Path(args.repo)))
    else:
        out = from_harness(data, tasks)
    Path(args.out).write_text(json.dumps(out, indent=1) + "\n")
    print(f"{args.direction}: {len(out.get('targets', out)) - (0 if 'targets' in out else 1)} targets -> {args.out}")


if __name__ == "__main__":
    sys.exit(main())
