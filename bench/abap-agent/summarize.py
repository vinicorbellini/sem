#!/usr/bin/env python3
"""Headline table of one checkpoint: per class and arm means over runs, sem vs grep deltas, adoption-rule verdict.

Reads results.csv (dry-run rows excluded) and results.jsonl (sem calls per run, from `calls_by_tool`).
Prints Markdown, so it can be pasted into the README. Scores are the stored ones, not rescored.

Usage:
    python3 bench/abap-agent/summarize.py --checkpoint gate2b --brief sem-first
    python3 bench/abap-agent/summarize.py --checkpoint gate2
"""

import argparse
import csv
import json
import sys
from pathlib import Path

BENCH_DIR = Path(__file__).resolve().parent
RESULTS_CSV = BENCH_DIR / "results.csv"
RESULTS_JSONL = BENCH_DIR / "results.jsonl"
CLASSES = ("B1", "B1A", "B2", "B3")


def load_rows(checkpoint: str) -> list[dict]:
    """Rows of the checkpoint, dry runs out, with `sem_calls` (sem_* entries of calls_by_tool) from results.jsonl."""
    sem_calls = {}
    with open(RESULTS_JSONL) as f:
        for line in f:
            r = json.loads(line)
            sem_calls[(r["timestamp"], r["arm"], r["task_id"], str(r["rep"]))] = sum(
                n for tool, n in (r.get("calls_by_tool") or {}).items() if tool.startswith("mcp__sem__") or tool.startswith("sem_"))
    with open(RESULTS_CSV, newline="") as f:
        rows = [r for r in csv.DictReader(f) if r["checkpoint"] == checkpoint and r["dry_run"] != "1"]
    for r in rows:
        r["sem_calls"] = sem_calls.get((r["timestamp"], r["arm"], r["task_id"], r["rep"]))
    return rows


def num(r: dict, col: str) -> float | None:
    return float(r[col]) if r[col] not in ("", None) else None


def mean(values: list) -> float | None:
    values = [v for v in values if v is not None]
    return sum(values) / len(values) if values else None


def stats(rows: list[dict]) -> dict:
    """One class and arm: means per run, cost summed. Tokens read = input + cache read + cache write."""
    n = len(rows)
    costs = [num(r, "cost_usd") for r in rows if num(r, "cost_usd") is not None]
    return {
        "runs": n,
        "success": mean([num(r, "success_score") for r in rows]),
        "passed": sum(r["tests_passed"] in ("1", "1.0", "True") for r in rows),
        "precision": mean([num(r, "precision") for r in rows]),
        "read": mean([num(r, "input_tokens") + num(r, "cache_read_tokens") + num(r, "cache_write_tokens") for r in rows]),
        "output": mean([num(r, "output_tokens") for r in rows]),
        "cost": sum(costs) if costs else None,
        "wall": mean([num(r, "wall_time_s") for r in rows]),
        "tools": mean([num(r, "tool_calls") for r in rows]),
        "sem_calls": mean([r["sem_calls"] for r in rows]),
    }


def pct(sem: float | None, grep: float | None) -> str:
    if sem is None or not grep:
        return ""
    d = (sem / grep - 1) * 100
    return f" ({d:+.0f}%)"


def fmt(v: float | None, spec: str, delta: str = "") -> str:
    return "-" if v is None else f"{v:{spec}}{delta}"


def calls(v: float | None) -> str:
    """One decimal, two when the mean needs them (1.25)."""
    return "-" if v is None else (f"{v:.2f}" if round(v, 1) != round(v, 2) else f"{v:.1f}")


def table_row(task_class: str, label: str, s: dict, grep: dict | None) -> str:
    d = (lambda k: pct(s[k], grep[k])) if grep else (lambda k: "")
    success = f"{s['passed']}/{s['runs']} pass" if task_class == "B2" else fmt(s["success"], ".3f")
    return (f"| {task_class} | {label} | {s['runs']} | {success} | {fmt(s['precision'], '.3f')} | "
            f"{fmt(s['read'], ',.0f', d('read'))} | {fmt(s['output'], ',.0f', d('output'))} | "
            f"{fmt(s['cost'], '.2f', d('cost'))} | {fmt(s['wall'], '.1f', d('wall'))} | "
            f"{fmt(s['tools'], '.1f', d('tools'))} | {calls(s['sem_calls']) if label != 'grep' else '-'} |")


def verdict(checkpoint: str, by: dict) -> list[str]:
    """The adoption rule of the checkpoint (README: "Adoption rule" for gate2 and before, "Gate 2b, pre-registered")."""
    def pair(c):
        g, s = by.get((c, "grep")), by.get((c, "sem"))
        return (g, s) if g and s else (None, None)

    lines = []
    if checkpoint == "gate2b":
        g, s = pair("B1A")
        b1a = None
        if g:
            ratio = s["read"] / g["read"]
            b1a = s["success"] >= g["success"] and ratio <= 0.85
            lines.append(f"- B1A: success sem {s['success']:.3f} vs grep {g['success']:.3f} (needs at least equal); "
                         f"tokens read {ratio * 100:.0f}% of grep (needs 85% or less): {'met' if b1a else 'not met'}")
        else:
            lines.append("- B1A: no rows for both arms: not met")
        g, s = pair("B2")
        b2 = None
        if g:
            wall = s["wall"] / g["wall"]
            tokens = s["read"] / g["read"]
            b2 = wall <= 0.70 and s["passed"] == g["passed"] and tokens <= 1.15
            lines.append(f"- B2: wall time sem {wall * 100:.0f}% of grep (needs 70% or less); passes sem {s['passed']} vs grep {g['passed']} "
                         f"(needs equal); tokens read {tokens * 100:.0f}% of grep (needs 115% or less): {'met' if b2 else 'not met'}")
        else:
            lines.append("- B2: no rows for both arms: not met")
        lines.append(f"\n**Verdict: {'adopt' if b1a or b2 else 'drop'}** (B1 and B3 are reported, not decided on).")
        return lines
    # Gate 2 and earlier: B1 precision +20 points or B2 wall -30% at equal pass rate, and tokens read at most +15%.
    met = []
    for c, label in (("B1", "B1 precision"), ("B2", "B2 wall time")):
        g, s = pair(c)
        if not g:
            lines.append(f"- {label}: no rows for both arms: not met")
            continue
        tokens = s["read"] / g["read"]
        if c == "B1":
            ok_main = None not in (s["precision"], g["precision"]) and (s["precision"] - g["precision"]) * 100 >= 20
            main = f"precision sem {fmt(s['precision'], '.3f')} vs grep {fmt(g['precision'], '.3f')} (needs +20 points)"
        else:
            ok_main = s["wall"] / g["wall"] <= 0.70 and s["passed"] == g["passed"]
            main = f"wall sem {s['wall']:.1f} s vs grep {g['wall']:.1f} s{pct(s['wall'], g['wall'])} at passes {s['passed']} vs {g['passed']} (needs -30% at equal passes)"
        ok = ok_main and tokens <= 1.15
        met.append(ok)
        lines.append(f"- {label}: {main}; tokens read {tokens * 100 - 100:+.0f}% (ceiling +15%): {'met' if ok else 'not met'}")
    lines.append(f"\n**Verdict: {'adopt' if any(met) else 'drop'}**.")
    return lines


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--checkpoint", required=True, help="Which checkpoint to summarise, e.g. gate2b.")
    parser.add_argument("--brief", help="Which `brief` value the sem arm's rows carry (0, 1 or sem-first). "
                                        "Default: the one the checkpoint's sem rows have; an error when it has several.")
    args = parser.parse_args()

    rows = load_rows(args.checkpoint)
    if not rows:
        print(f"No rows for checkpoint {args.checkpoint} in {RESULTS_CSV.name}")
        sys.exit(1)
    briefs = sorted({r["brief"] for r in rows if r["arm"] == "sem"})
    if args.brief is None:
        if len(briefs) > 1:
            print(f"Checkpoint {args.checkpoint} has sem rows with brief {briefs}: pass --brief, never mix them.")
            sys.exit(1)
        args.brief = briefs[0] if briefs else "0"
    # The grep arm's prompt never changes, so its rows carry brief 0 whatever the sem arm got.
    rows = [r for r in rows if r["arm"] == "grep" or r["brief"] == args.brief]
    label = {"0": "sem", "1": "sem (brief)"}.get(args.brief, f"sem ({args.brief})")

    by = {}
    for c in CLASSES:
        for arm in ("grep", "sem"):
            sub = [r for r in rows if r["task_class"] == c and r["arm"] == arm]
            if sub:
                by[(c, arm)] = stats(sub)

    print(f"Checkpoint `{args.checkpoint}`, sem arm `brief` = {args.brief}. Per-run means, except cost (summed); "
          "deltas are the sem arm against the grep arm.\n")
    print("| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |")
    print("|---|---|---|---|---|---|---|---|---|---|---|")
    for c in CLASSES:
        if (c, "grep") in by:
            print(table_row(c, "grep", by[(c, "grep")], None))
        if (c, "sem") in by:
            print(table_row(c, label, by[(c, "sem")], by.get((c, "grep"))))
    print("\nTokens read = input + cache read + cache write. Success is the score stored in results.csv "
          "(B2: runs with tests_passed). The README's Gate 2 table uses rescored B1 `b1_04` and B3 `b3_03` rows; "
          "these stored scores differ for them (see \"Scorer fixes\").\n")
    print("Adoption rule:\n")
    print("\n".join(verdict(args.checkpoint, {(c, "sem" if a == "sem" else "grep"): v for (c, a), v in by.items()})))


if __name__ == "__main__":
    main()
