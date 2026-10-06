#!/usr/bin/env python3
"""Text where-used for ABAP methods: the fallback ground truth of story 2.7.

Until `sapcli whereused` has been run against a SAP system, the precision
study scores sem's resolved callers against this. It is a different method
from the one under test on purpose: it never calls sem's stripper or its
graph. The only thing it takes from sem is entity line ranges
(`sem find --in <files> --json`), to say which method or form a call site sits
in.

How a site is found, on the source with comments and literals blanked by the
stripper below (`*` in column 1, `"` to the end of the line, '...', `...`,
and the text of |...| templates; a template's { ... } parts stay code):

- `->m(`, `=>m(`, `~m(` (functional calls; `~` only after the declaring
  interface, `zif_x~m(`), case-folded;
- `CALL METHOD <receiver>m` with or without a parameter list;
- a bare `m(` or `CALL METHOD m` inside the declaring class or a subclass of it
  (a call to its own or an inherited method);
- an alias: `ALIASES a FOR zif_x~m` in a class or interface makes `->a(`,
  `CALL METHOD ...a` and a bare `a(` in that object calls of `zif_x~m`
  (marked adjudicated);
- `PERFORM f` for a form.

Then each site is adjudicated by its receiver, without sem: a `cls=>m(` whose
`cls` is neither the declaring class nor a subclass is dropped; a `lo->m(`
whose `lo` is declared in the enclosing method or class as `TYPE REF TO` a
type that is not in the repository (`cl_...`, `if_...`) is dropped, and the
rows decided this way carry `"adjudicated": true`. Declarations (`METHODS`,
`CLASS-METHODS`, `METHOD m.`, `ALIASES`) have no `(` after the name and are
not sites. A computed call cannot be attributed by text; a literal naming the
method (`'GET_DISPLAY_NAME'`, as in `CALL METHOD lo->(lv)` setups) is listed
under `dynamic` per target and never counted.

The targets are methods whose name is declared once in the repository (one
`METHODS`/`CLASS-METHODS` in one class or interface, a `REDEFINITION` not
counted), as in the B.1 selection rule, so a name match can be adjudicated by
its receiver alone.

Usage:
  abap-whereused-grep.py truth --repo R --sem SEM --targets T.json --out F.json
  abap-whereused-grep.py sample --repo R --sem SEM --out S.json

`truth` writes the schema of crates/sem-core/tests/fixtures/abap/inside-sap/
README.md: an object keyed by `CLASS=>METHOD` (`INTF~METHOD`, `PROG/FORM` for
a form), each value the array of `{object, type, include, line}`, `include`
being the calling method (the converter's job for a sapcli export), plus
`file`, `caller` (local class and method, as written), `shape` and
`adjudicated`. One `_meta` key holds how it was made, the grep numbers and the
dynamic mentions; readers skip keys that start with `_`.

`sample` applies the selection rule of docs/abap/census-gate2.md and writes
the candidates per stratum and the 30 picks. It reads no caller output of sem.
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

PINNED_COMMIT = "b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0"

# ── Stripping ──────────────────────────────────────────────────────────────


def strip_line(line: str) -> str:
    """Blank comments and literal text in one line, keeping its length."""
    if line.startswith("*"):
        return " " * len(line)
    out = []
    i, n = 0, len(line)
    while i < n:
        c = line[i]
        if c == '"':
            out.append(" " * (n - i))
            break
        if c in "'`":
            j = i + 1
            while j < n:
                if line[j] == c:
                    if j + 1 < n and line[j + 1] == c:  # doubled quote is an escape
                        j += 2
                        continue
                    break
                j += 1
            out.append(" " * (min(j, n - 1) - i + 1))
            i = j + 1
            continue
        if c == "|":
            # A template: blank its text, keep the code in { ... }.
            out.append(" ")
            i += 1
            depth = 0
            while i < n:
                d = line[i]
                if depth == 0:
                    if d == "\\" and i + 1 < n:
                        out.append("  ")
                        i += 2
                        continue
                    if d == "|":
                        out.append(" ")
                        i += 1
                        break
                    if d == "{":
                        depth = 1
                        out.append(" ")
                    else:
                        out.append(" ")
                else:
                    if d == "{":
                        depth += 1
                    elif d == "}":
                        depth -= 1
                        if depth == 0:
                            out.append(" ")
                            i += 1
                            continue
                    out.append(d)
                i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def literals(line: str):
    """The text of each '...' and `...` literal in a line (for dynamic mentions)."""
    if line.startswith("*"):
        return []
    found = []
    code = line.split('"', 1)[0] if "'" not in line and "`" not in line else line
    for m in re.finditer(r"'((?:[^']|'')*)'|`((?:[^`]|``)*)`", code):
        found.append((m.group(1) or m.group(2) or ""))
    return found


class Source:
    def __init__(self, path: Path, rel: str):
        self.rel = rel
        self.lines = path.read_text(encoding="utf-8", errors="replace").split("\n")
        self.code_lines = [strip_line(l).lower() for l in self.lines]
        self.code = "\n".join(self.code_lines)
        self.raw_lower = "\n".join(self.lines).lower()
        starts, at = [], 0
        for l in self.code_lines:
            starts.append(at)
            at += len(l) + 1
        self.line_starts = starts

    def line_of(self, offset: int) -> int:
        lo, hi = 0, len(self.line_starts) - 1
        while lo < hi:
            mid = (lo + hi + 1) // 2
            if self.line_starts[mid] <= offset:
                lo = mid
            else:
                hi = mid - 1
        return lo + 1

    def statements(self):
        """(start offset, text) of each statement, cut at each period."""
        start = 0
        for m in re.finditer(r"\.", self.code):
            yield start, self.code[start:m.start()]
            start = m.end()


def object_of(rel: str) -> str:
    return Path(rel).name.split(".")[0].lower()


def tadir_type(rel: str) -> str:
    parts = Path(rel).name.split(".")
    return parts[1].upper() if len(parts) > 2 else "PROG"


# ── Declarations ───────────────────────────────────────────────────────────

NAME = r"[a-z_/][\w/]*"


class Decl:
    def __init__(self, **kw):
        self.__dict__.update(kw)


def chain_parts(body: str):
    """Split the text after a chained keyword's `:` at top-level commas."""
    parts, depth, cur = [], 0, []
    for ch in body:
        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
        if ch == "," and depth == 0:
            parts.append("".join(cur))
            cur = []
        else:
            cur.append(ch)
    parts.append("".join(cur))
    return [p.strip() for p in parts if p.strip()]


def read_declarations(sources):
    """Every METHODS/CLASS-METHODS/FORM/ALIASES/INHERITING fact of the repo."""
    methods, aliases, parents, implements = [], [], {}, defaultdict(set)
    redefinitions = defaultdict(set)  # method name -> classes that redefine it
    forms = []
    test_classes = set()
    for src in sources:
        container = None  # (kind, name, testing)
        section = None
        for start, stmt in src.statements():
            words = stmt.split()
            if not words:
                continue
            kw = words[0].rstrip(":")
            line = src.line_of(start + len(stmt) - len(stmt.lstrip()))
            if kw == "class" and len(words) > 2 and words[2] == "definition":
                if any(w in ("deferred", "load") for w in words[3:]) or "local" in words[3:4]:
                    continue
                testing = "testing" in words and "for" in words
                container = ("class", words[1], testing)
                if testing:
                    test_classes.add((src.rel, words[1]))
                if "inheriting" in words:
                    i = words.index("inheriting")
                    if i + 2 < len(words):
                        parents[words[1]] = words[i + 2]
                section = "public"
            elif kw == "interface" and len(words) > 1 and not any(w in ("deferred", "load") for w in words[2:]):
                container = ("interface", words[1], False)
                section = "public"
            elif kw in ("endclass", "endinterface"):
                container = None
            elif kw in ("public", "protected", "private") and len(words) > 1 and words[1].startswith("section"):
                section = kw
            elif container and kw in ("methods", "class-methods"):
                body = stmt.split(None, 1)[1] if len(words) > 1 else ""
                body = body.lstrip()
                if body.startswith(":"):
                    body = body[1:]
                for part in chain_parts(body):
                    pw = part.split()
                    if not pw:
                        continue
                    name = pw[0]
                    flags = set(pw[1:])
                    if "redefinition" in flags:
                        redefinitions[name].add(container[1])
                        continue
                    methods.append(Decl(
                        file=src.rel, object=object_of(src.rel), container_kind=container[0],
                        owner=container[1], name=name, static=(kw == "class-methods"),
                        section=section, event=("event" in flags and "for" in flags),
                        testing=("testing" in flags and "for" in flags) or container[2],
                        abstract=("abstract" in flags), line=line,
                    ))
            elif container and kw == "interfaces" and len(words) > 1:
                body = stmt.split(None, 1)[1].lstrip().lstrip(":")
                for part in chain_parts(body):
                    if part.split():
                        implements[container[1]].add(part.split()[0])
            elif container and kw == "aliases":
                body = stmt.split(None, 1)[1].lstrip().lstrip(":") if len(words) > 1 else ""
                for part in chain_parts(body):
                    m = re.match(rf"({NAME})\s+for\s+({NAME})~({NAME})", part)
                    if m:
                        aliases.append(Decl(file=src.rel, object=object_of(src.rel), owner=container[1],
                                            alias=m.group(1), interface=m.group(2), method=m.group(3)))
            elif kw == "form" and len(words) > 1:
                forms.append(Decl(file=src.rel, object=object_of(src.rel), name=words[1], line=line))
    return methods, aliases, parents, implements, redefinitions, forms, test_classes


# ── Entities (ranges only) ─────────────────────────────────────────────────


def sem_entities(sem: str, repo: Path, cache: str):
    env = dict(os.environ, SEM_CACHE_DIR=cache)
    out = subprocess.run(
        [sem, "find", "--in", "src", "--json", "--file-exts", ".abap", "--no-default-excludes"],
        cwd=repo, env=env, capture_output=True, text=True, check=True,
    ).stdout
    return json.loads(out)


class Ranges:
    """The innermost method, form, function or module around a line, per file."""

    CALLABLE = ("method", "form", "function", "module")

    def __init__(self, entities):
        self.by_file = defaultdict(list)
        for e in entities:
            self.by_file[e["file"]].append(e)

    def enclosing(self, rel: str, line: int):
        inside = [e for e in self.by_file.get(rel, []) if e["start_line"] <= line <= e["end_line"]]
        callables = [e for e in inside if e["type"] in self.CALLABLE]
        pool = callables or inside
        if not pool:
            return None
        return min(pool, key=lambda e: e["end_line"] - e["start_line"])

    def parent_name(self, e):
        pid = e.get("parent_id") or ""
        return pid.split("::")[-1] if pid else None


# ── Where-used ─────────────────────────────────────────────────────────────


class Repo:
    def __init__(self, repo: Path, sem: str, cache: str):
        self.root = repo
        files = sorted(str(p.relative_to(repo)) for p in (repo / "src").rglob("*.abap"))
        self.sources = [Source(repo / f, f) for f in files]
        self.by_rel = {s.rel: s for s in self.sources}
        (self.methods, self.aliases, self.parents, self.implements,
         self.redefinitions, self.forms, self.test_classes) = read_declarations(self.sources)
        self.decl_count = defaultdict(int)
        for m in self.methods:
            self.decl_count[m.name] += 1
        self.entities = sem_entities(sem, repo, cache)
        self.ranges = Ranges(self.entities)
        self.global_files = defaultdict(list)  # object -> its files
        for s in self.sources:
            self.global_files[object_of(s.rel)].append(s.rel)

    def subclasses(self, cls: str):
        out, frontier = {cls}, [cls]
        while frontier:
            c = frontier.pop()
            for sub, par in self.parents.items():
                if par == c and sub not in out:
                    out.add(sub)
                    frontier.append(sub)
        return out

    def implementers(self, intf: str):
        direct = {c for c, ints in self.implements.items() if intf in ints}
        # an interface that includes it, and its implementers
        for other, ints in list(self.implements.items()):
            if intf in ints and other.startswith(("zif", "lif")):
                direct |= self.implementers(other)
        out = set()
        for c in direct:
            out |= self.subclasses(c)
        return out

    def supertypes_ok(self, owner: str, kind: str):
        """Types a receiver may be declared with and still reach the target:
        for an interface method, the interfaces that include the interface."""
        if kind != "interface":
            return set()
        return {i for i, ints in self.implements.items() if owner in ints and i.startswith(("zif", "lif"))}

    def receiver_type(self, src: Source, line: int, var: str):
        """The `TYPE REF TO x` a variable is declared with, near the site, if any."""
        var = re.escape(var)
        pat = re.compile(rf"(?:data|class-data|importing|exporting|changing|returning|value\()[\s:]*[^.]*?\b{var}\)?\s+type\s+ref\s+to\s+({NAME})")
        enclosing = self.ranges.enclosing(src.rel, line)
        lo = enclosing["start_line"] - 1 if enclosing else 0
        text = "\n".join(src.code_lines[lo:line])
        m = None
        for m in pat.finditer(text):
            pass
        if m:
            return m.group(1)
        simple = re.compile(rf"\b{var}\s+type\s+ref\s+to\s+({NAME})")
        hits = [m.group(1) for m in simple.finditer(src.code)]
        return hits[0] if len(set(hits)) == 1 else None

    def known_type(self, name: str) -> bool:
        return name.startswith(("z", "y", "lcl", "lif", "ltcl", "lth", "lt_", "ltd"))

    def whereused(self, target):
        """Call sites of one target: a dict with `rows` (attributed) and the rest."""
        kind = target["kind"]
        name = target["method"].lower()
        owner = target["owner"].lower()
        rows, rejected, dynamic = [], [], []

        if kind == "form":
            pat = re.compile(rf"\bperform\s+{re.escape(name)}\b(?!\()")
            for src in self.sources:
                for m in pat.finditer(src.code):
                    rows.append(self._row(src, m.start(), "perform", False))
            return {"rows": rows, "rejected": rejected, "dynamic": dynamic}

        if kind == "interface":
            in_scope = self.implementers(owner) | {owner}
        else:
            in_scope = self.subclasses(owner)
        # objects whose files may hold a bare call: the declaring class's and its subclasses'
        bare_classes = set() if kind == "interface" else in_scope

        names = [(name, False)]
        for a in self.aliases:
            if a.interface == owner and a.method == name:
                names.append((a.alias, a.owner))
        seen = set()
        for src in self.sources:
            if not any(call_name in src.code for call_name, _ in names) and name not in src.raw_lower:
                continue
            for call_name, alias_owner in names:
                if call_name not in src.code:
                    continue
                n = re.escape(call_name)
                patterns = [
                    ("arrow", re.compile(rf"([\w/]+|\))\s*->\s*{n}\(")),
                    ("static", re.compile(rf"([\w/]+)\s*=>\s*{n}\(")),
                    ("tilde", re.compile(rf"([\w/]+)~{n}\(")),
                    ("call_method", re.compile(rf"\bcall\s+method\s+((?:[\w/]+(?:->|=>))*)(?:([\w/]+)~)?{n}\b(?!\s*->|\s*=>)")),
                    ("bare", re.compile(rf"(?<![\w/~>\-=]){n}\(")),
                ]
                for shape, pat in patterns:
                    for m in pat.finditer(src.code):
                        key = (src.rel, src.line_of(m.start()), m.start())
                        line = key[1]
                        cls_here = self._class_at(src, line)
                        adjudicated = bool(alias_owner)
                        if shape == "bare":
                            # `call method m` is matched above; a bare `m(` is a call
                            # only inside the owner or a subclass (or the alias's class).
                            prefix = src.code[max(0, m.start() - 12):m.start()]
                            if re.search(r"call\s+method\s+$", prefix):
                                continue
                            scope = {alias_owner} if alias_owner else bare_classes
                            if cls_here not in scope:
                                continue
                        elif shape == "static":
                            recv = m.group(1)
                            if alias_owner:
                                continue
                            if recv not in in_scope:
                                rejected.append(self._reject(src, m.start(), shape, f"static receiver {recv}"))
                                continue
                        elif shape == "tilde":
                            if alias_owner or m.group(1) != owner:
                                continue
                        elif shape == "call_method":
                            chain, intf = m.group(1), m.group(2)
                            if intf and intf != owner:
                                continue
                            if alias_owner and intf:
                                continue
                            if not chain and not intf:
                                scope = {alias_owner} if alias_owner else bare_classes
                                if cls_here not in scope and kind != "interface":
                                    continue
                                if kind == "interface" and not alias_owner:
                                    continue
                            if chain.endswith("=>"):
                                recv = chain[:-2].split("->")[-1].split("=>")[-1]
                                if recv not in in_scope and not alias_owner:
                                    rejected.append(self._reject(src, m.start(), shape, f"static receiver {recv}"))
                                    continue
                        elif shape == "arrow":
                            recv = m.group(1)
                            if alias_owner and recv not in ("me",):
                                adjudicated = True
                            if recv not in ("me", "super", ")"):
                                t = self.receiver_type(src, line, recv)
                                if t and not self.known_type(t):
                                    rejected.append(self._reject(src, m.start(), shape, f"receiver {recv} is a {t}"))
                                    continue
                                # Typed as a class or interface of the repository
                                # that is not the target's: another method of
                                # the same name (B.1's get_proxy_url).
                                if t and t not in in_scope and t not in self.supertypes_ok(owner, kind):
                                    rejected.append(self._reject(src, m.start(), shape, f"receiver {recv} is a {t}"))
                                    continue
                                if t:
                                    adjudicated = adjudicated or self.decl_count[name] > 1
                        if key in seen:
                            continue
                        seen.add(key)
                        row = self._row(src, m.start(), shape, adjudicated)
                        if alias_owner:
                            row["alias"] = call_name
                        rows.append(row)
            # Literals naming the method: possible computed calls, never counted.
            if name not in src.raw_lower:
                continue
            for i, l in enumerate(src.lines, 1):
                for lit in literals(l):
                    if lit.strip().lower() == name:
                        dynamic.append({"file": src.rel, "line": i, "text": l.strip()[:120]})
        return {"rows": rows, "rejected": rejected, "dynamic": dynamic}

    def _class_at(self, src, line):
        e = self.ranges.enclosing(src.rel, line)
        if not e:
            return None
        if e["type"] == "class":
            return e["name"].lower()
        pid = e.get("parent_id") or ""
        parts = pid.split("::")
        return parts[-1].lower() if len(parts) >= 3 else None

    def _row(self, src, offset, shape, adjudicated):
        line = src.line_of(offset)
        e = self.ranges.enclosing(src.rel, line)
        if e and e["type"] in Ranges.CALLABLE:
            caller = e["name"].lower()
            local = self.ranges.parent_name(e)
            caller_text = f"{local}->{e['name']}" if local and e["type"] == "method" else e["name"]
        else:
            caller = object_of(src.rel)
            caller_text = e["name"] if e else object_of(src.rel)
        return {
            "object": object_of(src.rel).upper(),
            "type": tadir_type(src.rel),
            "include": caller,
            "line": line,
            "file": src.rel,
            "caller": caller_text,
            "shape": shape,
            "adjudicated": adjudicated,
        }

    def _reject(self, src, offset, shape, why):
        r = self._row(src, offset, shape, True)
        r["why"] = why
        return r


def target_key(t) -> str:
    if t["kind"] == "form":
        return f"{t['object'].upper()}/{t['method'].upper()}"
    sep = "~" if t["kind"] == "interface" else "=>"
    return f"{t['owner'].upper()}{sep}{t['method'].upper()}"


def caller_set(rows):
    return sorted({(r["object"].lower(), r["include"].lower()) for r in rows})


# ── Sample ────────────────────────────────────────────────────────────────

QUOTAS = [("static", 7), ("instance", 7), ("interface", 7), ("redefined", 4), ("me", 3), ("form", 2)]
# A stratum whose pool is smaller than its quota hands the rest out one pick at
# a time, in this order, round robin.
REFILL = ["static", "instance", "interface", "me"]
MIN_SITES, MAX_SITES = 2, 40


def stratum(repo: Repo, d) -> str:
    if d.container_kind == "interface":
        return "interface"
    if d.static:
        return "static"
    if repo.redefinitions.get(d.name):
        return "redefined"
    if d.section in ("private", "protected"):
        return "me"
    return "instance"


def sample(repo: Repo):
    """The selection rule of docs/abap/census-gate2.md, applied."""
    order = {}
    for i, e in enumerate(repo.entities):
        if e["type"] == "method":
            order.setdefault((e["file"], e["name"].lower()), i)
    pools = defaultdict(list)
    excluded = defaultdict(int)
    for d in repo.methods:
        why = None
        if not re.search(r"\.(clas|intf)\.abap$", d.file):
            why = "not a global class or interface file"
        elif d.testing or "testclasses" in d.file:
            why = "test"
        elif d.event:
            why = "event handler"
        elif d.name in ("constructor", "class_constructor"):
            why = "constructor"
        elif repo.decl_count[d.name] != 1:
            why = "name declared more than once"
        elif (d.file, d.name) not in order:
            why = "no method entity"
        if why:
            excluded[why] += 1
            continue
        target = {"kind": "interface" if d.container_kind == "interface" else "class",
                  "owner": d.owner, "method": d.name, "object": d.object, "file": d.file}
        result = repo.whereused(target)
        sites = len(result["rows"])
        if not (MIN_SITES <= sites <= MAX_SITES):
            excluded[f"call sites outside {MIN_SITES}..{MAX_SITES}"] += 1
            continue
        target.update(stratum=stratum(repo, d), grep_call_sites=sites,
                      grep_caller_methods=len(caller_set(result["rows"])),
                      grep_files=len({r["file"] for r in result["rows"]}),
                      order=order[(d.file, d.name)])
        pools[target["stratum"]].append(target)
    for f in repo.forms:
        target = {"kind": "form", "owner": f.object, "method": f.name, "object": f.object, "file": f.file}
        result = repo.whereused(target)
        sites = len(result["rows"])
        if MIN_SITES <= sites <= MAX_SITES:
            order_i = next((i for i, e in enumerate(repo.entities)
                            if e["type"] == "form" and e["file"] == f.file and e["name"].lower() == f.name), None)
            if order_i is None:
                excluded["no form entity"] += 1
                continue
            target.update(stratum="form", grep_call_sites=sites,
                          grep_caller_methods=len(caller_set(result["rows"])),
                          grep_files=len({r["file"] for r in result["rows"]}), order=order_i)
            pools["form"].append(target)
        else:
            excluded[f"call sites outside {MIN_SITES}..{MAX_SITES}"] += 1

    quotas = {name: min(quota, len(pools[name])) for name, quota in QUOTAS}
    short = sum(q for _, q in QUOTAS) - sum(quotas.values())
    i = 0
    while short > 0:
        name = REFILL[i % len(REFILL)]
        if quotas[name] < len(pools[name]):
            quotas[name] += 1
            short -= 1
        i += 1
    picks = []
    for name, _ in QUOTAS:
        pool = sorted(pools[name], key=lambda t: t["order"])
        take = quotas[name]
        if take:
            k = len(pool) // take
            picks += [pool[i * k] for i in range(take)]
    return {"quotas": quotas,"pools": {k: len(v) for k, v in pools.items()}, "excluded": dict(excluded),
            "picks": picks, "pool_members": {k: [target_key(t) for t in sorted(v, key=lambda t: t["order"])] for k, v in pools.items()}}


# ── Main ──────────────────────────────────────────────────────────────────


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("mode", choices=["truth", "sample"])
    ap.add_argument("--repo", default="/tmp/claude-0/abapGit")
    ap.add_argument("--sem", default=str(Path(__file__).resolve().parents[1] / "crates/target/release/sem"))
    ap.add_argument("--cache", default=None, help="SEM_CACHE_DIR for the entity listing (default: a scratch dir)")
    ap.add_argument("--targets", help="truth: JSON list of {kind, owner, method[, object]}")
    ap.add_argument("--out", required=True)
    args = ap.parse_args()

    repo_path = Path(args.repo)
    head = subprocess.run(["git", "-C", str(repo_path), "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
    cache = args.cache or str(Path(tempfile.gettempdir()) / "sem-abap-whereused-cache")
    repo = Repo(repo_path, str(Path(args.sem).resolve()), cache)

    if args.mode == "sample":
        out = sample(repo)
        out["abapgit_commit"] = head
        Path(args.out).write_text(json.dumps(out, indent=1) + "\n")
        for t in out["picks"]:
            print(f"{t['stratum']:10} {target_key(t):60} sites={t['grep_call_sites']}")
        return

    targets = json.loads(Path(args.targets).read_text())
    result = {"_meta": {
        "abapgit_commit": head,
        "source": "grep fallback (scripts/abap-whereused-grep.py), not sapcli",
        "include": "the calling method or form, case-folded (a sapcli export gives the SAP include; the converter maps it)",
        "pinned_commit_matches": head == PINNED_COMMIT,
        "targets": {},
    }}
    for t in targets:
        key = target_key(t)
        r = repo.whereused(t)
        rows = sorted(r["rows"], key=lambda x: (x["file"], x["line"]))
        result[key] = rows
        result["_meta"]["targets"][key] = {
            "kind": t["kind"], "stratum": t.get("stratum"), "file": t.get("file"),
            "grep_call_sites": len(rows),
            "grep_caller_methods": len(caller_set(rows)),
            "grep_files": len({x["file"] for x in rows}),
            "adjudicated_rows": sum(1 for x in rows if x["adjudicated"]),
            "rejected": r["rejected"],
            "dynamic": r["dynamic"],
        }
    Path(args.out).write_text(json.dumps(result, indent=1) + "\n")
    print(f"{len(targets)} targets -> {args.out}")


if __name__ == "__main__":
    sys.exit(main())
