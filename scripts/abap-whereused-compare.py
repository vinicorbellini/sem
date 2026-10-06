#!/usr/bin/env python3
"""Score sem's resolved ABAP callers against a where-used ground truth.

The study of story 2.7 (`docs/abap/census-gate2.md`). For each target method
of the truth file it runs `sem find <method> --callers --json` in the
repository, takes the result whose entity is the target, and compares its
`related` callers (the sources of `calls` edges into the method: resolved
callers only; `possible_callers` do not count) with the truth's, at the
caller entity: the pair (object, calling method or form), case-folded. The
object is the abapGit object of the caller's file (`zcl_foo` for
`zcl_foo.clas.testclasses.abap`), so a local class's method counts under its
object, as SAP's where-used reports it.

Per method it gives truth size, resolved size, hits, precision and recall;
pooled, the sums. A second column adds the callers of every entity linked to
the target by a `dispatch` edge, either way (an interface or base
declaration the target implements or redefines, or an implementation or
redefinition of the target), from `sem graph --json`: what story 2.3's
dispatch adds to a where-used.

Every false positive and false negative is listed with a reason, decided
from sem's own answer, the truth row and the graph:

  FN: possible caller (sem saw the site and did not bind it: <site kind>),
      dispatch (the caller is bound to an implementation or redefinition of
      the target, or to the declaration it implements), other target (bound
      to another entity of the same name), no caller entity (sem has no
      entity around the site), missing edge (none of these);
  FP: outside src/ (a caller in a folder the truth does not read, such as
      abapGit's test/), truth rejected (the grep dropped that site for its
      receiver),
      not in the truth (the caller mentions the name, the grep did not count
      it), wrong edge (the caller's text does not mention the name).

The truth file defaults to inside-sap/whereused.json when the sapcli export
exists and to whereused.grep.json otherwise.

Usage:
  abap-whereused-compare.py [--truth F] [--stratum S ...] [--json OUT] [--markdown]
"""

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INSIDE_SAP = ROOT / "crates/sem-core/tests/fixtures/abap/inside-sap"
PRECISION_MIN, RECALL_MIN = 0.90, 0.80


def object_of(path: str) -> str:
    return Path(path).name.split(".")[0].lower()


def split_key(key: str):
    if "~" in key:
        owner, method = key.split("~", 1)
        return "interface", owner.lower(), method.lower()
    owner, method = key.split("=>", 1)
    return "class", owner.lower(), method.lower()


def entity_key(e) -> tuple:
    return (object_of(e["file"]), e["name"].lower())


def run_sem(sem, repo, cache, args):
    env = dict(os.environ, SEM_CACHE_DIR=cache)
    out = subprocess.run([sem, *args], cwd=repo, env=env, capture_output=True, text=True)
    if out.returncode != 0:
        raise SystemExit(f"sem {' '.join(args)} failed: {out.stderr.strip()}")
    return json.loads(out.stdout)


def find_target(results, owner, method):
    """The `sem find --callers` result whose entity is owner's method."""
    want = f"::{owner}::{method}"
    for r in results:
        eid = r["entity"]["id"].lower()
        if eid.endswith(want):
            return r
    return None


class Graph:
    def __init__(self, data):
        self.entities = {e["id"]: e for e in data["entities"]}
        self.callers = defaultdict(set)
        self.calls_from = defaultdict(set)
        self.dispatch = defaultdict(set)
        for e in data["edges"]:
            if e["refType"] == "calls":
                self.callers[e["toEntity"]].add(e["fromEntity"])
                self.calls_from[e["fromEntity"]].add(e["toEntity"])
            elif e["refType"] == "dispatch":
                self.dispatch[e["fromEntity"]].add(e["toEntity"])
                self.dispatch[e["toEntity"]].add(e["fromEntity"])

    def key(self, eid):
        e = self.entities.get(eid)
        return (object_of(e["filePath"]), e["name"].lower()) if e else None

    def entity_at(self, file, line):
        inside = [e for e in self.entities.values()
                  if e["filePath"] == file and e["startLine"] <= line <= e["endLine"]
                  and e["entityType"] in ("method", "form", "function", "module")]
        return min(inside, key=lambda e: e["endLine"] - e["startLine"]) if inside else None


def score(tp, predicted, truth):
    p = tp / predicted if predicted else None
    r = tp / truth if truth else None
    return p, r


def fmt(x):
    return "n/a" if x is None else f"{100 * x:.0f}%"


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--repo", default="/tmp/claude-0/abapGit")
    ap.add_argument("--sem", default=str(ROOT / "crates/target/release/sem"))
    ap.add_argument("--cache", default=None, help="SEM_CACHE_DIR (default: one under the system temp dir)")
    ap.add_argument("--truth", default=None)
    ap.add_argument("--stratum", action="append", help="only targets of this stratum (from the truth's _meta); repeatable")
    ap.add_argument("--exclude-stratum", action="append", default=[])
    ap.add_argument("--json", default=None, help="write the full result here")
    args = ap.parse_args()

    truth_path = Path(args.truth) if args.truth else (
        INSIDE_SAP / "whereused.json" if (INSIDE_SAP / "whereused.json").exists() else INSIDE_SAP / "whereused.grep.json")
    truth = json.loads(truth_path.read_text())
    meta = truth.get("_meta", {}).get("targets", {})
    keys = [k for k in truth if not k.startswith("_")]
    if args.stratum:
        keys = [k for k in keys if meta.get(k, {}).get("stratum") in args.stratum]
    keys = [k for k in keys if meta.get(k, {}).get("stratum") not in args.exclude_stratum]

    repo = Path(args.repo)
    args.sem = str(Path(args.sem).resolve())
    cache = args.cache or str(Path(tempfile.gettempdir()) / "sem-abap-whereused-cache")
    head = subprocess.run(["git", "-C", str(repo), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    sem_version = subprocess.run([args.sem, "--version"], capture_output=True, text=True).stdout.strip()
    graph = Graph(run_sem(args.sem, repo, cache, ["graph", "--json", "--file-exts", ".abap"]))

    rows, misses = [], []
    totals = defaultdict(int)
    for key in keys:
        kind, owner, method = split_key(key)
        truth_rows = truth[key]
        truth_set = {(r["object"].lower(), r["include"].lower()) for r in truth_rows}
        # `owner.method` picks the one definition when the name has several.
        result = None
        for query in (f"{owner}.{method}", method):
            try:
                results = run_sem(args.sem, repo, cache, ["find", query, "--callers", "--json"])
            except SystemExit:
                continue
            result = find_target(results if isinstance(results, list) else [], owner, method)
            if result:
                break
        if result is None:
            resolved, possible, target_id = set(), {}, None
        else:
            target_id = result["entity"]["id"]
            resolved = {entity_key(e) for e in result["related"]}
            possible = {}
            for p in result.get("possible_callers", []):
                parts = p["entity"].lower().split(".")
                possible[(object_of(p["file"]), parts[-1])] = ",".join(sorted({s.get("kind", "?") for s in p.get("sites", [])}))
        # second column: callers of entities dispatch-linked to the target
        linked = graph.dispatch.get(target_id, set()) if target_id else set()
        with_dispatch = set(resolved)
        for x in linked:
            with_dispatch |= {graph.key(c) for c in graph.callers.get(x, set()) if graph.key(c)}

        tp = len(resolved & truth_set)
        tp2 = len(with_dispatch & truth_set)
        p, r = score(tp, len(resolved), len(truth_set))
        p2, r2 = score(tp2, len(with_dispatch), len(truth_set))
        rows.append({"target": key, "stratum": meta.get(key, {}).get("stratum"),
                     "truth": len(truth_set), "resolved": len(resolved), "hits": tp,
                     "precision": p, "recall": r, "found": target_id is not None,
                     "with_dispatch": len(with_dispatch), "hits_with_dispatch": tp2,
                     "precision_with_dispatch": p2, "recall_with_dispatch": r2,
                     "complete": result.get("complete") if result else None})
        totals["truth"] += len(truth_set)
        totals["resolved"] += len(resolved)
        totals["hits"] += tp
        totals["with_dispatch"] += len(with_dispatch)
        totals["hits_with_dispatch"] += tp2

        rejected = {(r["object"].lower(), r["include"].lower()): r.get("why")
                    for r in meta.get(key, {}).get("rejected", [])}
        by_caller = defaultdict(list)
        for tr in truth_rows:
            by_caller[(tr["object"].lower(), tr["include"].lower())].append(tr)
        for fn in sorted(truth_set - resolved):
            trs = by_caller[fn]
            shapes = sorted({t.get("shape", "?") for t in trs})
            reason = "missing edge"
            ent = graph.entity_at(trs[0].get("file", ""), int(trs[0]["line"])) if trs[0].get("file") else None
            ent_id = next((i for i, e in graph.entities.items() if e is ent), None) if ent else None
            if trs[0].get("file") and ent is None:
                reason = "no caller entity"
            elif ent_id and graph.calls_from.get(ent_id, set()) & linked:
                bound = sorted(graph.calls_from[ent_id] & linked)[0]
                reason = f"dispatch (bound to {bound.split('::', 2)[-1]})"
            elif fn in possible:
                reason = f"possible caller ({possible[fn]})"
            elif ent_id:
                same_name = [t for t in graph.calls_from.get(ent_id, set())
                             if graph.entities.get(t, {}).get("name", "").lower() == method]
                if same_name:
                    reason = f"other target ({same_name[0]})"
            misses.append({"target": key, "kind": "FN", "caller": "->".join(fn), "reason": reason,
                           "shapes": shapes, "sites": [f"{t.get('file')}:{t['line']}" for t in trs]})
        for fp in sorted(resolved - truth_set):
            e = next(e for e in result["related"] if entity_key(e) == fp)
            text = "\n".join((repo / e["file"]).read_text(errors="replace").split("\n")[e["start_line"] - 1:e["end_line"]]).lower()
            if not e["file"].startswith("src/"):
                reason = "outside src/ (the truth reads src/ only, as SAP holds only abapGit's packages)"
            elif fp in rejected:
                reason = f"truth rejected ({rejected[fp]})"
            elif re.search(rf"\b{re.escape(method)}\b", text):
                reason = "not in the truth (the caller mentions the name)"
            else:
                reason = "wrong edge (no mention of the name)"
            misses.append({"target": key, "kind": "FP", "caller": "->".join(fp), "reason": reason,
                           "sites": [f"{e['file']}:{e['start_line']}-{e['end_line']}"]})

    P, R = score(totals["hits"], totals["resolved"], totals["truth"])
    P2, R2 = score(totals["hits_with_dispatch"], totals["with_dispatch"], totals["truth"])
    verdict = {"precision": P, "recall": R,
               "pass": (P or 0) >= PRECISION_MIN and (R or 0) >= RECALL_MIN,
               "precision_with_dispatch": P2, "recall_with_dispatch": R2}

    print(f"truth: {truth_path.relative_to(ROOT) if truth_path.is_relative_to(ROOT) else truth_path}")
    print(f"sem: {sem_version}; abapGit {head}")
    print()
    print("| Target | Stratum | Truth | Resolved | Hits | Precision | Recall | + dispatch: resolved, hits | Precision | Recall |")
    print("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|")
    for r in rows:
        print(f"| `{r['target']}` | {r['stratum']} | {r['truth']} | {r['resolved']} | {r['hits']} | {fmt(r['precision'])} | {fmt(r['recall'])} | {r['with_dispatch']}, {r['hits_with_dispatch']} | {fmt(r['precision_with_dispatch'])} | {fmt(r['recall_with_dispatch'])} |")
    print(f"| **pooled** | | {totals['truth']} | {totals['resolved']} | {totals['hits']} | **{fmt(P)}** | **{fmt(R)}** | {totals['with_dispatch']}, {totals['hits_with_dispatch']} | {fmt(P2)} | {fmt(R2)} |")
    print()
    print(f"Precision {fmt(P)} (threshold {PRECISION_MIN:.0%}), recall {fmt(R)} (threshold {RECALL_MIN:.0%}): "
          f"{'PASS' if verdict['pass'] else 'FAIL'}")
    print()
    for m in misses:
        print(f"{m['kind']} {m['target']} {m['caller']}: {m['reason']} [{', '.join(m['sites'][:3])}]")

    if args.json:
        Path(args.json).write_text(json.dumps({
            "truth": str(truth_path), "sem_version": sem_version, "abapgit_commit": head,
            "rows": rows, "totals": dict(totals), "verdict": verdict, "misses": misses,
        }, indent=1) + "\n")
    return 0


if __name__ == "__main__":
    sys.exit(main())
