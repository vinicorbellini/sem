# ABAP agent benchmark

Paired agent runs on a pinned commit of [abapGit](https://github.com/abapGit/abapGit)
(`b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`). Each run is one headless Claude Code invocation
(`claude -p`), started as a subprocess in the run's own checkout. Each task runs twice per
repetition. Both runs use the same model, effort, turn bound and prompt:

- **grep arm**: Claude Code's built-in `Bash`, `Read`, `Grep` and `Glob`. On B2, `Edit` and `Write` as well.
- **sem arm**: the same, plus every tool `sem mcp` lists (`sem_find`, `sem_grep`, `sem_impact`,
  `sem_check`, `sem_certify`, `sem_diff`, `sem_graph`, `sem_history`; `LISTED_TOOLS` in
  `crates/sem-mcp/src/server.rs`), attached with `--mcp-config` as a standalone stdio server.

Every run gets a fresh clone of the pinned checkout, so an edit in one run never reaches another.
The arm order alternates by repetition. One class, C1, is a control on a Rust repository (sem itself at
upstream's merge-base `dc2cc4d7173d9f3eab87d491b351927fbe2f1f63`); see "C1 targets" and "Control: Rust where-used
(reporting only)".

## Tasks

| Class | Tasks | Asks | Scored by |
|---|---|---|---|
| B1 where-used | 10 (`tasks/b1_whereused.json`) | list every method that calls X, with file and line | precision/recall of calling methods against `ground_truth/whereused.json` |
| B1A where-used, ambiguous | 12 (`tasks/b1a_whereused.json`) | the same, for a method whose name other classes also declare | as B1, against `ground_truth/whereused-ambiguous.json` |
| B2 change-and-verify | 6 (`tasks/b2_change.json`) | add mandatory parameter p to method X, update every caller, make the tests pass | hidden tests and `npm run unit` pass, test classes the agent ran, callers missed |
| B3 review | 4 (`tasks/b3_review.json`) | summarise what merged PR #n changed and what it could break | changed entities and callers left behind against `ground_truth/b3_rubric.json` |
| C1 where-used, Rust control | 12 (`tasks/c1_whereused.json`) | B1A's question on sem's own Rust source | as B1, against `ground_truth/whereused-rust.json` |
| R1 read-method | 8 (`tasks/r1_readmethod.json`) | list every method one method in a large class calls, with the line of each call | precision/recall of called method names against `ground_truth/readmethod.json` (`score_r1`) |

How the targets were picked is in each task file's `selection` field. In short:

- **B1**: methods declared in exactly one class or interface, with 3 to 30 calling methods found by grep, spread over 3 or more files. The grep counts are stored as a baseline, not as ground truth.
- **B1A**: methods whose name is declared in 3 or more classes or interfaces, so a text search for the name is ambiguous. See "B1A targets".
- **B2**: methods whose own class has a test class that runs under `npm run unit`. Each hidden test in `hidden_tests/` was checked twice. It passes against a reference implementation (the parameter added with a DEFAULT, callers untouched). It fails to build on the unchanged checkout.
- **B3**: four squash-merged PRs from the 300 commits before the pinned one.
- **C1**: B1A's rule carried over to Rust: functions and methods whose name is declared 3 or more times. See "C1 targets".
- **R1**: methods of 30 to 150 lines past line 500 of a class file of 1,500 lines or more, two per file. See "R1 targets".

**Ground truth status.** `whereused.json` does not exist yet; it will come from `sapcli whereused`.
Until it does, B1 is scored against `ground_truth/whereused.grep.json`, the hand-checked text
where-used of story 2.7 (`docs/abap/census-gate2.md`), which `tasks/b1_whereused.json` points to. It is
`crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json` converted with
`python3 scripts/abap-whereused-convert.py to-harness ... --tasks bench/abap-agent/tasks/b1_whereused.json`.
B1 rows from before it carry `error = no ground truth yet` and an empty score. The answers are kept
in `results.jsonl` (`final_text`), so they can be scored later without rerunning. Convert sapcli's
output to:

```json
{"abapgit_commit": "b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0",
 "targets": {"b1_01": [{"method": "<class>-><method>", "file": "src/...", "line": 12}]}}
```

`b3_rubric.json` is a **draft**. It was written from `git show` and grep at the pinned commit and
cross-checked with `sem diff --commit`. Review it before reading much into B3 scores.

## B1A targets

Gate 2 found grep at its ceiling on B1: each B1 target's name is declared once, so a text search for it
finds exactly the callers. B1A ("where-used, ambiguous") keeps B1's prompt shape, answer format, scorer and
ground-truth schema, and picks targets whose name other classes and interfaces declare too. The rule was
written before any candidate's callers were read (`selection` in `tasks/b1a_whereused.json`):

| Dimension | Threshold |
|---|---|
| (a) name declared in several objects | `METHODS`/`CLASS-METHODS` of that name (chains split, `REDEFINITION` not counted) in **3 or more** classes or interfaces, global or local. Every target meets it. |
| (b) interface method, several implementations | declared in a global interface that **3 or more** global classes implement. The question is **callers of the interface method**: every call bound to `zif_x~m`, whatever class implements it. A same-named method of an implementing class (one inherited from a superclass, say) is a different method. |
| (c) name also used as something else | whole-word grep hits of the name **at least twice** its call-shaped hits (the name is also a field, parameter, variable, constant component or literal). |
| (d) short name | **under 8** characters. |

Candidates are instance methods of a global class and methods of a global interface under `src/`: no test
methods, event handlers, constructors or B1 targets. `CLASS-METHODS` are left out, because `zcl_x=>m(` names
the class at every call from outside it, so the class disambiguates. Each candidate needs **6 to 80**
call-shaped grep hits (`->name(`, `=>name(`, `~name(`, `CALL METHOD ...name`, any receiver) in 3 or more files.
Strata, assigned in this order: `intf` (b), `short` (d), `polluted` (c), `multi` (the rest). Each takes 3 picks at
index 0, k and 2k of its candidates in (file, line) order, with k = n div 3. A used index or a name already
picked moves to the next index. **Post-check**, the only use of callers: a pick is kept only with **3 to 40**
true calling methods and true call sites at most **75%** of its call-shaped hits. Otherwise the next index is tried.

`polluted` ran out after one pick. The rule had not said what happens then, so one sentence was added at that
point, before any further callers were read. A stratum that runs out hands its remaining picks, round robin, to
`intf`, `short` and `multi` in that order. `intf` and `multi` had run out too, so both went to `short`. The
walk, with every rejected candidate and why, is at the top of `ground_truth/whereused-ambiguous.review.md`. Of
the 58 candidates tried, 38 were rejected, almost all for having 1 or 2 callers (private helpers called from
their own class), and 8 skipped for a name already picked. One was rejected by the 75% check:
`zcl_abapgit_stage->get_all`, where 12 of 15 call-shaped hits are true.

| Task | Target | Stratum | Dimensions | Declared in | grep hits | grep files | Call-shaped hits | True call sites | True callers |
|---|---|---|---|---:|---:|---:|---:|---:|---:|
| b1a_01 | `zif_abapgit_object~is_active` | intf | a, b, c | 5 | 278 | 141 | 15 | 8 | 7 |
| b1a_02 | `zif_abapgit_object~get_metadata` | intf | a, b, c | 3 | 287 | 145 | 13 | 3 | 3 |
| b1a_03 | `zif_abapgit_gui_event_handler~on_event` | intf | a, b, c | 6 | 71 | 52 | 8 | 4 | 4 |
| b1a_04 | `zcl_abapgit_object_pinf->load` | short | a, c, d | 7 | 70 | 29 | 28 | 3 | 3 |
| b1a_05 | `zcl_abapgit_persistence_db->list` | short | a, c, d | 5 | 116 | 70 | 19 | 3 | 3 |
| b1a_06 | `zcl_abapgit_gui_page_flow->refresh` | short | a, c, d | 7 | 117 | 32 | 29 | 5 | 3 |
| b1a_07 | `zcl_abapgit_string_map->clear` | short | a, c, d | 9 | 1033 | 284 | 27 | 4 | 4 |
| b1a_08 | `zcl_abapgit_string_map->to_abap` | short | a, d | 6 | 88 | 19 | 64 | 10 | 7 |
| b1a_09 | `zcl_abapgit_syntax_highlighter->parse_line` | polluted | a, c | 3 | 30 | 9 | 14 | 8 | 7 |
| b1a_10 | `zcl_abapgit_html_form_utils->normalize` | multi | a | 6 | 49 | 27 | 36 | 21 | 19 |
| b1a_11 | `zcl_abapgit_html_form_utils->validate` | multi | a | 7 | 70 | 30 | 38 | 27 | 19 |
| b1a_12 | `zcl_abapgit_html_form_utils->is_empty` | multi | a | 7 | 55 | 28 | 39 | 8 | 3 |
| total | | | | | 2264 | | 330 | 104 | 82 |

"grep hits" and "grep files" are lines and files of `src/` with the name as a whole word, case-insensitive (the
task file's `grep_hits`, `grep_files`): what a plain text search returns. "Call-shaped hits" narrows that to
lines that look like a call of some method of that name (`grep_call_sites`). "True callers" are distinct
(file, calling method) pairs, the scoring unit. Across the twelve, 104 of 330 call-shaped hits are calls of the
target (32%). On B1 the call-shaped counts (144 sites) and the truth (145) differ by five sites
(`docs/abap/census-gate2.md`). The set leans on two classes: three targets are in
`zcl_abapgit_html_form_utils` and two in `zcl_abapgit_string_map`. Both came out of the index rule; nothing was
swapped by hand.

**The prompt.** The template is B1's, plus two sentences: calls to other classes' same-named methods are not
calls of the target, and dynamic calls do not count. A per-task `count_rule` field (filled in by
`build_prompt`, which formats the template with the task's fields) says what counts. For a class target, a
receiver statically typed as the class or a subclass, a call that returns that type, `me->`, `super->` or a bare
call inside the class or a subclass. For an interface target, every call bound to `zif_x~m` (see (b) above), named
explicitly: an interface reference, an alias, `->zif_x~m(` on a class reference, inside an implementer, and
`super->zif_x~m(` in a redefinition.

**The ground truth.** `ground_truth/whereused-ambiguous.json`, in the schema of `whereused.grep.json`, `src/`
only, like the first truth. It is not the output of `scripts/abap-whereused-grep.py`, whose rows assume a name
declared once. Each of the 2264 whole-word hits was decided by hand against the receiver's declaration, and
every decision is a line in `ground_truth/whereused-ambiguous.review.md` (`file:line | decision | reason`). 2160
are not calls of the target: CLEAR statements (937, all on `clear`), comments and literals, declarations and
`METHOD` headers, and calls whose receiver is typed as another class or interface. sem was neither built nor
run for any of it.

Judgement calls a second reader should check:

- `b1a_09`: `lo_syntax->parse_line( )` in the test classes of `zcl_abapgit_syntax_abap` and `_xml` has a subclass
  receiver, so at run time it goes to the subclass's redefinition. Under the subclass rule it counts, and so do
  the five `super->parse_line( )` calls in the four redefinitions. A reading of "callers of this implementation only"
  would drop the two test callers.
- `b1a_03`: `super->zif_abapgit_gui_event_handler~on_event( )` in `zcl_abapgit_gui_page_patch` counts, as the
  prompt says.
- `b1a_01`, `b1a_02`: about 250 bare `is_active( )` / `get_metadata( )` calls in the object classes call the
  protected methods inherited from `zcl_abapgit_objects_super`, not the interface method; every one of those
  classes was checked to inherit from it. This is the trap grep falls into on these two targets.
- `b1a_02`: two real callers sit in abapGit's `test/src/` (`zcl_abapgit_test_doma`, `zcl_abapgit_test_dtel`,
  `lo_doma` / `lo_dtel TYPE REF TO zif_abapgit_object`). They are outside the truth's scope, so the scorer counts them
  neither as hits nor as misses (`TRUTH_SCOPE`).
- Dynamic calls are not counted. The only one naming a target's name with an unknown receiver is
  `CALL METHOD lo_odso->('IS_ACTIVE')` in `zcl_abapgit_object_odso`, on an SAP object (`TYPE REF TO object`).

Check: `python3` loads the truth and the task file, confirms every task id has a non-empty caller list,
and confirms every truth file path and every `defined_in` exists at the pinned commit (`git cat-file -e`).

`run.py` does not run B1A yet: `CLASSES`, `--class` and `RUN_TIMEOUT_S` know B1 to B3. B1A needs the B1 path
(scorer `score_b1`, ground truth from the task file's `ground_truth`) under the new class name.

## R1 targets

Every class before R1 asks a where-used question, and an agent answers those with little source read (about 13k bytes
of tool output per run in Gate 2c). R1 ("read a method in a large class") asks for what one method's body calls, the
kind of question for which an agent reads a large class to understand one method. The rule was written into
`tasks/r1_readmethod.json` (`selection`) before any method body was read:

| Step | Rule |
|---|---|
| file | a global class's main include, `src/**/*.clas.abap` (not `.locals_imp`, not `.testclasses`), of **1,500 lines or more** |
| method | a `METHOD ... ENDMETHOD` block of that class's `IMPLEMENTATION`, starting **after line 500**, body (the lines strictly between `METHOD` and `ENDMETHOD`) **30 to 150** lines; no constructor, `class_constructor` or test method |
| callees | **5 to 30** distinct names by a call-shaped grep over the body (below) |
| picks | the kept candidates in (file, line) order, n of them, k = n div 8, picks at index 0, k, ..., 7k; an index already used or whose file already has **2** picks moves to the next |

The call-shaped grep removes comments and `'...'` / `` `...` `` literals (string templates stay, since `{ }` in them can
hold calls), then collects the names of `->name(` and `=>name(` (with `zif_x~name`), `CALL METHOD`, a bare `name(`
followed by a blank, `)` or the line end that is not an ABAP built-in function or constructor operator (the task
file's `grep_builtins`), and `NEW x(` / `CREATE OBJECT` as `constructor`, each reduced to the part after its last `->`,
`=>` or `~`. A script applied the rule and printed line numbers and counts only.

At the pinned commit four files have 1,500 lines or more, so the cap of 2 per file makes it 2 picks per file, and
n = 14 gives k = 1: in each file the first two candidates. The script's first version only allowed a blank or `)`
after a bare call's `(`, so a call whose `(` ends the line (`parse_error(` with its parameters below) was missed,
`zcl_abapgit_object_tabl_ddl` had no candidate and the rule could not fill eight picks under the per-file cap. The regex
was fixed before any body was read, and n went from 11 to 14. Nothing else was changed and no pick was swapped by hand.

| Task | Method | File | File lines | Lines | Body lines | grep hits | grep files | Call-shaped names | True callees | True call sites |
|---|---|---|---:|---|---:|---:|---:|---:|---:|---:|
| r1_01 | `zcl_abapgit_object_tabl_ddl->set_builtin_type` | `zcl_abapgit_object_tabl_ddl.clas.abap` | 2,670 | 1758-1842 | 83 | 3 | 1 | 6 | 6 | 13 |
| r1_02 | `zcl_abapgit_object_tabl_ddl->serialize` | `zcl_abapgit_object_tabl_ddl.clas.abap` | 2,670 | 2030-2120 | 89 | 337 | 194 | 8 | 8 | 11 |
| r1_03 | `zcl_abapgit_object_fugr->zif_abapgit_object~deserialize` | `zcl_abapgit_object_fugr.clas.abap` | 1,529 | 1292-1353 | 60 | 311 | 187 | 12 | 12 | 15 |
| r1_04 | `zcl_abapgit_object_fugr->zif_abapgit_object~serialize` | `zcl_abapgit_object_fugr.clas.abap` | 1,529 | 1474-1528 | 53 | 337 | 194 | 14 | 14 | 17 |
| r1_05 | `zcl_abapgit_objects=>deserialize_lxe` | `zcl_abapgit_objects.clas.abap` | 1,521 | 820-856 | 35 | 3 | 1 | 6 | 6 | 7 |
| r1_06 | `zcl_abapgit_objects=>deserialize_step` | `zcl_abapgit_objects.clas.abap` | 1,521 | 859-942 | 82 | 3 | 1 | 13 | 13 | 19 |
| r1_07 | `zcl_abapgit_objects_program->deserialize_program` | `zcl_abapgit_objects_program.clas.abap` | 1,597 | 668-717 | 48 | 4 | 3 | 10 | 10 | 10 |
| r1_08 | `zcl_abapgit_objects_program->serialize_program` | `zcl_abapgit_objects_program.clas.abap` | 1,597 | 1311-1418 | 106 | 4 | 3 | 16 | 16 | 23 |
| total | | | | | | | | | 85 | 115 |

"grep hits" and "grep files" are lines and files under `src/` with the method's bare name (the part after `~`) as a
whole word, case-insensitive: how hard the implementation would be to find by text alone. They do not come into play
here, because the prompt names the file. `r1_03` and `r1_04` are `zif_abapgit_object~deserialize` / `~serialize`
implementations, whose bare names have 311 and 337 whole-word hits in 187 and 194 files; `r1_02` is the class's own
`serialize`, with the same 337 hits; the other five names occur in one to three files. "Call-shaped names" is the selection grep, "True callees" and "True call sites" the
ground truth; on all eight the grep's name count equals the truth's.

**The prompt.** The method as `zcl_x->m` (`zcl_x=>m` for a static method), its file, the counting rules, and the
answer format `{"calls": [{"callee": "...", "line": 123}]}`. Method calls are `receiver->name( )`, `class=>name( )`,
`zif_x~name( )`, a bare `name( )` of the class's own or inherited methods, `CALL METHOD` with a static name, and object
creation with `NEW` or `CREATE OBJECT`, a call of `constructor` (as is `super->constructor( )`); a chained call
`a( )->b( )` is two calls. Function modules (`CALL FUNCTION`), `PERFORM`, dynamic calls with a literal or variable
name, built-in functions, constructor operators, `RAISE EXCEPTION TYPE`, statements such as `CLEAR`, comments and calls
in the class's local test classes do not count. The method's line range is not in the prompt.

**The ground truth.** `ground_truth/readmethod.json`, `targets` -> task id -> `[{"callee", "line"}]`, one item per call
site, the callee as written in the source, the line where the called method's name appears. Every body was read in full
at the pinned commit, and every call-shaped hit in it (any `name(` with its receiver chain, and every `CALL METHOD`,
`CALL FUNCTION`, `CREATE OBJECT`, `NEW` and `PERFORM`, comments included) is a line in
`ground_truth/readmethod.review.md` (`line | decision | reason`): 127 hits, 115 calls, 12 not (the
built-in functions `to_lower`, `strlen`, `repeat` and `lines`, and one `CALL FUNCTION 'RPY_PROGRAM_READ'`). sem was
neither built nor run for any of it. Judgement calls a second reader should check: `CREATE OBJECT li_xml TYPE
zcl_abapgit_xml_output` in `r1_08` counts as `constructor` (the prompt says so); `zcl_abapgit_objects_activation=>clear( )`
in `r1_06` is a method call, not the `CLEAR` statement; a chained factory call such as
`zcl_abapgit_factory=>get_sap_report( )->read_progdir( )` is two calls on one line.

**The scorer.** `score_r1` in `scorers.py`, in the style of `score_b1`. Both sides are reduced to the called method's
name (`_callee_name`): lower case, a leading `CALL METHOD` dropped, `NEW ...` and `CREATE OBJECT ...` read as
`constructor`, argument lists removed innermost first, then everything up to the last `=>`, `->` and `~` stripped. So
`lo_repo->get_name`, `me->get_name`, `zif_abapgit_repo~get_name` and `get_name` all score as `get_name`, and the
receiver's spelling never matters. Precision, recall and F1 are over the set of names (`success_score` is F1); two
receivers calling the same name are one item. `line_precision` (and `line_recall`, in `results.jsonl`) match answer
items one to one to true call sites of the same name within `LINE_TOLERANCE` (2) lines, as B1 does.

Check: every task has callees, every file exists at the pinned commit with the recorded length, every truth line lies
inside its method, and the truth scored against itself gives 1.0 on all eight. The scorer on three hand-written
answers for `r1_08`: the truth itself, the truth with `li_report->` written as `zif_abapgit_sap_report~` and `li_xml->`
as `me->mo_xml->`, and the truth plus the function module:

```sh
python3 -c "
import json, sys; sys.path.insert(0, 'bench/abap-agent'); import scorers
truth = json.load(open('bench/abap-agent/ground_truth/readmethod.json'))['targets']['r1_08']
exact = {'calls': truth}
receiver = {'calls': [dict(c, callee=c['callee'].replace('li_report->', 'zif_abapgit_sap_report~').replace('li_xml->', 'me->mo_xml->')) for c in truth]}
fm = {'calls': truth + [{'callee': 'RPY_PROGRAM_READ', 'line': 1332}]}
for label, answer in (('exact', exact), ('receiver', receiver), ('function module', fm)):
    r = scorers.score_r1(json.dumps(answer), truth)
    print(f\"{label:<16} f1 {r['f1']} precision {r['precision']} recall {r['recall']} line_precision {r['line_precision']} fp {r['false_positives']}\")
"
```

```
exact            f1 1.0 precision 1.0 recall 1.0 line_precision 1.0 fp []
receiver         f1 1.0 precision 1.0 recall 1.0 line_precision 1.0 fp []
function module  f1 0.9697 precision 0.9412 recall 1.0 line_precision 0.9583 fp ['rpy_program_read']
```

## C1 targets

Gate 2b could not say whether its result is about sem on short agent tasks or about the ABAP port: every gate ran on
abapGit. C1 ("where-used, Rust control") asks B1A's question on a language upstream sem was built for, with the same
harness, arms, briefing and model. The repository is sem itself at upstream's merge-base with this branch,
`dc2cc4d7173d9f3eab87d491b351927fbe2f1f63`: 250 `.rs` files, 163k lines, all under `crates/`. The task file names
it (`repo`), so C1 runs need no abapGit. The rule was written into `tasks/c1_whereused.json` (`selection`) before any
candidate's callers were read:

| Dimension | Threshold |
|---|---|
| (a) name declared several times | `fn name`, any kind (free function, inherent method, trait method, impl method), **3 or more** times in the repository's `.rs` files, tests and examples included. Every target meets it. |
| (b) trait method, several impls | declared in a trait defined in the repository that **3 or more** `impl` blocks implement. The question is **callers of the trait method**: every call that resolves to `Trait::method`, whatever type implements it, through `dyn Trait`, a generic or `impl Trait` bound, a concrete type whose method comes from its impl of the trait, `Trait::method(x)`, or `self.method( )` inside an impl. An inherent method or free function of the same name is a different function. |
| (c) name also used as something else | whole-word grep hits (`\bname\b`, case-sensitive) **at least twice** its call-shaped hits (a field, variable, string, module or other item). |
| (d) short name | **under 8** characters. |

Candidates are under `crates/*/src/`, outside `#[cfg(test)]` modules: methods with a `self` receiver of a repository
trait (target `Trait::method`) or an inherent impl (`Type::method`), and free functions (`module::function`, not
`main`). Associated functions without `self` are left out, because `Type::f(` names the type at every call from
outside the impl, as `CLASS-METHODS` do in B1A; so are methods of impls of std and crate traits (`fmt`, `from`,
`default`), whose calls are operators and macros. Each candidate needs **6 to 80** call-shaped grep hits (lines
matching `\bname\s*\(`, which covers `.name(`, `::name(` and a bare `name(`, minus the `fn name(` declaration) in 3 or
more files. Strata, assigned in this order: `trait` (b), `short` (d), `polluted` (c), `multi` (the rest); 3 picks
each at index 0, k and 2k, k = n div 3, a used index or a picked name moving to the next index, the 75% and
3-to-40-callers post-check and the round-robin hand-off exactly as B1A's. Callers anywhere under `crates/` count,
`#[cfg(test)]` modules, `tests/`, `benches/` and `examples/` included, like B1A's local test classes. A call is a
call expression in the source, including one inside a macro's arguments; a function named as a value
(`.map(Type::method)`) and a call made only by a macro's expansion do not count. A caller is the innermost named
`fn` (a closure belongs to the function it is written in).

The walk is at the top of `ground_truth/whereused-rust.review.md`. Of the 26 candidates tried, 13 were rejected
(12 for having 1 or 2 callers, as in B1A mostly private helpers, one for the 75% check: `graph::is_test_entity`,
13 of 17 call-shaped hits true) and one skipped for a name already picked. No stratum ran out.

| Task | Target | Stratum | Dimensions | Declared | grep hits | grep files | Call-shaped hits | True call sites | True callers |
|---|---|---|---|---:|---:|---:|---:|---:|---:|
| c1_01 | `Lang::lower` | trait | a, b, c, d | 8 | 74 | 26 | 8 | 3 | 3 |
| c1_02 | `Lang::layout` | trait | a, b, c, d | 7 | 90 | 17 | 11 | 7 | 7 |
| c1_03 | `SemanticParserPlugin::extract_entities_with_tree` | trait | a, b | 3 | 40 | 11 | 25 | 6 | 6 |
| c1_04 | `arch_view::names` | short | a, c, d | 3 | 1070 | 92 | 42 | 9 | 3 |
| c1_05 | `QueryIndex::lookup` | short | a, c, d | 4 | 189 | 48 | 52 | 26 | 22 |
| c1_06 | `Lower::expr` | short | a, d | 4 | 127 | 20 | 69 | 19 | 4 |
| c1_07 | `util::fingerprint` | polluted | a, c | 5 | 99 | 29 | 29 | 5 | 5 |
| c1_08 | `QueryIndex::file_count` | polluted | a, c | 5 | 89 | 31 | 20 | 9 | 9 |
| c1_09 | `FastExtractorSet::identity` | polluted | a, c | 7 | 106 | 34 | 7 | 3 | 3 |
| c1_10 | `CloudClient::auth_header` | multi | a | 3 | 30 | 3 | 27 | 14 | 14 |
| c1_11 | `telemetry::now_secs` | multi | a | 3 | 11 | 3 | 8 | 4 | 3 |
| c1_12 | `SvelteLowerer::make_entity` | multi | a | 3 | 40 | 3 | 37 | 4 | 4 |
| total | | | | | 1965 | | 335 | 109 | 83 |

"grep hits" and "grep files" are lines and files of `crates/**/*.rs` with the name as a whole word, case-sensitive;
"Call-shaped hits" narrows that to lines that look like a call of something of that name, comments and strings
included, as a plain grep returns them. Across the twelve, 109 of 335 call-shaped hits are calls of the target (33%;
B1A: 32%). Where B1A leaned on two classes, C1 leans on `QueryIndex` (two targets) and on `parser/calls/` (three),
again straight from the index rule.

**The prompt.** B1A's template with the ABAP wording replaced: "a checkout of the sem repository (Rust sources in a
Cargo workspace under crates/)", "every function that calls", the answer as `Type::method` or `module::function`.
Its clauses are B1A's: calls to other same-named functions are not calls of the target, a per-task `count_rule` says
what counts (for a trait method, every call that resolves to it as in (b); for an inherent method, a receiver
statically typed as that type; for a free function, a bare call where it is in scope or a path to it), and dynamic
calls do not count, which in Rust means a function named as a value and a call made only by a macro's expansion.
Calls in test modules, `tests/`, `benches/` and `examples/` count. The shared head of the prompt (`INSTRUCTIONS`)
names the code base from the task file's `code_base` ("a Rust code base"; the ABAP classes keep "an ABAP code
base", so their prompts are unchanged byte for byte). The sem-first briefing is unchanged, including its ABAP
example, as the control requires.

**The ground truth.** `ground_truth/whereused-rust.json`, B1A's schema (`targets` -> task id -> `[{method, file,
line}]`), `crates/` only, which is every `.rs` file. Each of the 1965 whole-word hits was decided by hand against the
receiver's declaration, and every decision is a line in `ground_truth/whereused-rust.review.md`. 1856 are not calls
of the target: comments and strings (fixture source code in raw strings), declarations, `use` lines, variables and
fields of the same name, and calls that resolve to another function of the name. sem was neither run nor read for
any of it.

Judgement calls a second reader should check:

- `c1_01`, `c1_02`: in each of the three `impl Lang for ...`, the trait method's body is a bare `lower(tree, src)` /
  `layout(...)`. A method is never in scope by its bare name in Rust, so these call the module's own free function
  (`go::lower`, `rust::layout`), not the trait method. They are the trap a text search falls into here.
- `c1_03`: a call on a concrete `CodeParserPlugin` counts, because that type's `extract_entities_with_tree` exists
  only as its impl of the trait. 19 of the 25 call-shaped hits are the inherent
  `ParserRegistry::extract_entities_with_tree`, a different method with the same name.
- `c1_09`: `FastExtractorSet::identity` and the trait method `FastExtractor::identity` are declared 41 lines apart in
  the same file; `e.identity()` in `FastExtractorSet::new` is on a `Box<dyn FastExtractor>` and calls the trait's.
  The call in `identity_salt` is on an `Arc<FastExtractorSet>` and counts.
- `c1_06`, `c1_10`: two files define a type of the same name. `parser/calls/go.rs`, `python.rs` and `rust.rs` each
  have a private `struct Lower` with an `expr` method (and `FileFacts::expr` is a fourth); `sem-cli` and
  `sem-cloud-client` each have a `CloudClient` with `auth_header`. Only `rust.rs`'s and `sem-cli`'s count.
- `c1_04`, `c1_11`: `certify::certificate` binds a closure named `names`, and `json.rs` and `oxc_extractor.rs`
  have test helpers named `names`; `cloud.rs`, `update.rs` and `telemetry.rs` each have a private `now_secs`.
- The `.rs` fixtures under `crates/sem-core/tests/fixtures/` are in scope like any other file. None of their hits is
  a call of a target.

Check: `python3` loads the truth and the task file, confirms every task id has a non-empty caller list, that every
truth file path and every `defined_in` exists at the commit (`git cat-file -e`), that the task file's `true_*`
counts match the truth, and that scoring the truth against itself gives 1.0 with `scope` `crates/`. There is no
test file for `scorers.py`, so the Rust normalisation is checked with:

```sh
cd bench/abap-agent && python3 -c "
import json, scorers
t = [{'method': 'QueryIndex::lookup', 'file': 'crates/a/src/Foo.rs', 'line': 3}]
f = lambda m, p: scorers.score_b1(json.dumps({'callers': [{'method': m, 'file': p, 'line': 3}]}), t, scope='crates/')['success_score']
assert f('QueryIndex::lookup', 'crates/a/src/Foo.rs') == f('lookup', './crates/a/src/Foo.rs') == f('<QueryIndex as X>::lookup', 'crates/a/src/Foo.rs') == 1.0
assert f('QueryIndex::Lookup', 'crates/a/src/Foo.rs') == f('QueryIndex::lookup', 'crates/a/src/foo.rs') == 0.0
a = [{'method': 'zcl_a->run', 'file': 'src/zcl_a.clas.abap', 'line': 1}]
assert scorers.score_b1(json.dumps({'callers': [{'method': 'ZCL_A->RUN', 'file': 'SRC/ZCL_A.CLAS.ABAP', 'line': 1}]}), a)['success_score'] == 1.0
print('ok')"
```

`score_b1` keys a Rust item on (file, last `::` segment) with case kept, so `Type::method`, `Lower<'a>::expr` and
`<X as Trait>::m` all match; ABAP items and paths are folded to lower case as before. The 132 stored Gate 2b B1 and
B1A answers rescore to their stored scores with the changed scorer.

Runs use the **owner's Claude account**: whichever account `claude` on PATH is logged in with
(OAuth, `claude auth`). No `ANTHROPIC_API_KEY` is needed or read. Under a subscription the USD
figure is Claude Code's **nominal estimate** at list prices (`total_cost_usd`), not a bill. It still
drives `--cap-usd`, because it is the one cost number every run reports the same way.

```sh
cp -r <main checkout>/crates/target crates/target       # optional: reuse a warm build
(cd crates && cargo build --release -p sem-cli)          # sem with ABAP (default features)
claude --version                                         # Claude Code on PATH, logged in

python3 bench/abap-agent/run.py --dry-run --arm both --class all --reps 1     # never calls claude
python3 bench/abap-agent/run.py --checkpoint baseline --arm both --class B3 --reps 1
python3 bench/abap-agent/run.py --checkpoint gate1 --class B1 --task b1_03 --reps 1
```

| Flag | Meaning |
|---|---|
| `--checkpoint baseline\|gate1\|gate2\|gate2b\|gate2c\|control` | Written to every row; required for a real run. |
| `--arm grep\|sem\|cli\|both` | Which arms to run (default both, grep and sem). `cli` (Gate 2d) is the grep arm's tools with `sem` on PATH, run alone with `--brief cli`; see "Gate 2d, CLI arm". |
| `--class B1\|B1A\|B2\|B3\|C1\|R1\|all` | Which task classes (default all); `--task b2_04` (repeatable) narrows to single tasks. A class whose task file is absent is skipped with one line. |
| `--model` | Passed to `claude --model` (default `claude-sonnet-5-5`). |
| `--reps` | Repetitions per task per arm (default 3). |
| `--brief [sem-first\|sem-find-only]` | Add one paragraph to the **sem arm's** prompt naming `sem_find`, `sem_impact` and `sem_certify`. Plain `--brief` tells the agent to prefer them over grep (`BRIEF` in `run.py`); `--brief sem-first` tells it to ask `sem_find` first and grep only to check what it names (`BRIEF_SEM_FIRST`; see "Briefing"); `--brief sem-find-only` is the same for a server that lists `sem_find` alone, and needs `--mcp-tools sem_find`. The grep arm's prompt is unchanged. Default off. Recorded in the `brief` column. |
| `--mcp-tools NAMES` | Comma-separated tools the sem arm's server lists, passed to `sem mcp` as `SEM_MCP_TOOLS` (e.g. `sem_find`); the others stay callable by name but are not in the agent's context. Default: all eight. Recorded in the `mcp_tools` column. |
| `--dry-run` | Everything except calling `claude`. It prepares the checkouts, writes each sem run's MCP config, and prints each run's working directory, exact command line and prompt. For the sem arm it starts `sem mcp` once, lists its tools and calls `sem_find` (B1, B2) or `sem_certify` (B3). It scores the untouched checkout. B2 scoring runs the unit suite, so expect about 30 s per B2 run. |
| `--cap-usd` | Stop once the cumulative `total_cost_usd` reaches the cap (default 30; see below). |
| `--abapgit` | abapGit clone to copy from (default `/tmp/claude-0/abapGit`). It is cloned, never modified. A task file with a `repo` (C1) names its own source instead. |
| `--work-dir` | Checkouts, transcripts, MCP configs, sem logs (default `/tmp/sem-abap-agent`). |
| `--sem-binary` | sem to serve MCP. By default `$SEM_BINARY`, then `crates/target/release/sem` in this checkout, then in the main checkout when this is a linked worktree. |
| `--keep` | Keep each run's checkout. |

The first run prepares `<work-dir>/base`:

1. Clone `--abapgit`.
2. Fetch 300 commits of history from GitHub when the B3 commits are missing (a shallow clone).
3. Check out the pinned commit.
4. With B2 in the run only: `npm ci --ignore-scripts` with `abapgit-package-lock.json` (see Known issues).
5. With B2 in the run only: clone the transpiler's libraries at the commits in `abapgit-transpile-libs.json` into `<work-dir>/libs/<name>` (see Known issues).

Each run clones `base`. A B2 run also symlinks its `node_modules` and points its `test/abap_transpile.json` at `<work-dir>/libs`. That file is marked `skip-worktree`, so it is not in the agent's `git status` or in the patch the scorer replays. Until the C1 control these two steps ran for every class; B1, B1A and B3 never use them, so their checkouts now lack the `node_modules` symlink and keep the file as abapGit has it.

A task file may name its repository instead: `"repo": {"source": "<path>", "commit": "<sha>"}` (C1). The harness then prepares `<work-dir>/base-<commit, 12 characters>` as a shallow single-branch clone of the source's HEAD, which must be that commit (check it out detached in a worktree), and fails if it is not or if the checkout holds a `target/` directory. The clone has that one commit and no other refs, so an agent's `git log --all` cannot reach the benchmark's own branches. None of the abapGit steps run for it. A `repo` task file also sets `scope` (the ground truth's directory, default `src/`) and `code_base` (the shared instructions' "an ABAP code base").

### The command per run

The grep arm, from the run's checkout (`cwd`), with the prompt as the one positional argument:

```sh
claude -p "$PROMPT" --output-format stream-json --verbose \
  --model claude-sonnet-5-5 --effort high --max-turns 40 \
  --tools Bash,Read,Grep,Glob --allowedTools Bash,Read,Grep,Glob \
  --permission-mode dontAsk --setting-sources '' --strict-mcp-config \
  --no-session-persistence --session-id <uuid> --max-budget-usd <rest of the cap>
```

The sem arm is the same, with `mcp__sem` added to `--allowedTools` and
`--mcp-config <work-dir>/mcp/<run>.json`. That file starts `sem mcp` in the checkout with
`SEM_REPO` set to it, `SEM_MCP_NO_SHARED=1` (a standalone stdio process, not the shared daemon, so
the server ends with the run), and telemetry, update checks, cloud and network off. The server's
stderr goes to `<work-dir>/<run>.sem-mcp.log`. On B2 both arms add `Edit,Write` to `--tools` and
`--allowedTools`.

Why each flag:

- `--tools` decides which built-in tools exist at all. `--allowedTools` only pre-approves them.
- `--permission-mode dontAsk` denies anything not pre-approved instead of prompting. Nobody is there to answer a prompt.
- `--setting-sources ''` keeps user, project and local settings out of the run, with their hooks and CLAUDE.md files. `--strict-mcp-config` does the same for MCP servers configured elsewhere.
- `--max-turns 40` is a safety bound. Every baseline run finished in 11 turns or fewer.
- `--effort high` and `--model` are explicit, because the defaults depend on the account and environment. A probe with no `--model` ran on a different model depending on the inherited environment.

The harness removes the launching Claude Code session's own variables from the child's
environment (`PARENT_SESSION_ENV` in `tools.py`). If they are inherited, the child joins the
parent's session id and picks up its extra directories and their CLAUDE.md files. Auth and proxy
variables stay. Each run also has a wall-clock timeout (`RUN_TIMEOUT_S`: 30 min for B1, B1A, B3 and C1,
60 min for B2). On timeout the whole process group is killed (claude, sem mcp, npm).

**Prompt.** `claude -p` takes one prompt, and Claude Code keeps its own system prompt. The text the
SDK harness sent as its system prompt (`INSTRUCTIONS` in `run.py`) is therefore the head of the
prompt, followed by the task text and its output contract. Both arms get identical text. The one
inherent difference: Claude Code adds the sem server's MCP instructions and tool schemas to the sem
arm's context. That is part of what the sem arm costs.

**Transcript.** `--output-format json` gives the totals but no tool calls, so runs use
`--output-format stream-json --verbose`. The harness saves each transcript to
`<work-dir>/transcripts/<run>.jsonl`. Tool calls, files read, bytes read and test classes are
parsed from its `tool_use` and `tool_result` events. Tokens, cost, turns and API time come from
its final `result` event, the same object `--output-format json` returns.

**Cost cap.** `--cap-usd` is fed by `total_cost_usd`. No run starts once the cumulative cost has
reached the cap. Each run gets the rest of the cap as `--max-budget-usd`, so the cap can be passed
by about one request. `prices.json` is now only a cross-check (`list_price_usd` in
`results.jsonl`). It runs about 20% under Claude Code's figure, because Claude Code writes 1-hour
cache entries (2x the input price) and `prices.json` holds the 5-minute write rate.

## Output

`results.csv` is append-only, one row per run. A file with different columns is an error, never
rewritten. `results.jsonl` holds the same rows plus:

- the Claude Code version, session id and exact command;
- the prompt and final answer;
- the tools and MCP server status Claude Code reported at start;
- per-model usage, permission denials and the price-table cross-check;
- per-tool call counts, files read and full score details;
- the transcript path.

Dry runs write `results.dry-run.csv` / `.jsonl`. The repo's `.gitignore` ignores `bench/`, so
commit results with `git add -f bench/abap-agent/results.csv bench/abap-agent/results.jsonl`.

| Column | Meaning |
|---|---|
| `timestamp` | UTC, when the run finished. |
| `checkpoint` | `baseline`, `gate1`, `gate2`, `gate2b`, `gate2c`, `control` or `gate2d` (`dry-run` for dry runs). |
| `build` | sem commit measured: `git rev-parse HEAD` of this repo, `-dirty` if `crates/` has uncommitted changes. Build the binary from that commit. |
| `sem_version` | `sem --version` of the binary that served MCP. |
| `abapgit_commit` | Pinned abapGit commit. On C1 rows, the commit of the task file's `repo` (`dc2cc4d7173d`, sem at upstream's merge-base); the column keeps its name. |
| `model`, `arm`, `task_class`, `task_id`, `rep` | Which run. |
| `brief` | Which sem briefing the run's prompt carried (sem arm only): `1` for `--brief`, `sem-first` for `--brief sem-first`, `sem-find-only` for `--brief sem-find-only`, `cli` for the cli arm's `--brief cli`, else `0`. Baseline rows and every grep row are 0. Compare only rows with the same `brief`. |
| `dry_run` | 1 for `--dry-run` rows. |
| `input_tokens` | Uncached input tokens, summed over the run's requests (`usage.input_tokens`). |
| `output_tokens` | Output tokens, thinking included (`usage.output_tokens`). |
| `cache_read_tokens` | Input tokens served from the prompt cache (`usage.cache_read_input_tokens`). |
| `cache_write_tokens` | Input tokens written to the cache (`usage.cache_creation_input_tokens`). |
| `cost_usd` | Claude Code's `total_cost_usd`: a nominal list-price estimate, not a bill under a subscription. |
| `wall_time_s` | Wall time of the `claude` subprocess: model requests, tool execution, Claude Code and `sem mcp` startup. Excludes checkout preparation and scoring. |
| `api_time_s` | Claude Code's `duration_api_ms`: time spent waiting on the model. |
| `turns` | Claude Code's `num_turns`. |
| `tool_calls` | Tool calls the model made, all tools (`tool_use` blocks in the transcript). |
| `files_read` | Distinct files opened with the `Read` tool. Files printed through `Bash` (cat, sed) or matched by `Grep` are not counted. |
| `bytes_read` | Bytes of tool output returned to the model, all tools; what the model had to read. |
| `test_classes_executed` | Distinct (object, local test class) pairs that ran, not skipped, in the output of the agent's `Bash` calls. |
| `success_score` | B1: F1 of calling methods. B2: 1 if the mandatory parameter is in the signature and the hidden tests and full suite pass, else 0. B3: mean of entity F1 and caller recall (entity F1 alone when the rubric lists no callers). |
| `precision`, `recall` | B1: of calling methods (file + method). B3: of changed entities. Empty for B2. |
| `tests_passed` | B2: `npm run unit` exit 0 with every hidden test method run. |
| `callers_missed` | B2: call statements under `src/` that do not pass the new parameter by name. |
| `stop_reason` | The final result's stop reason. |
| `error` | Cap reached, max turns, timeout, sem mcp not connected, API error, refusal, missing ground truth, patch not applying. |
| `mcp_tools` | `--mcp-tools` of a sem run (`sem_find`); empty for the grep arm and for a server listing all its tools. Added after Gate 2b: the header gained it at the end, and the rows written before have no field for it, which reads as empty. |

B1 line numbers are also scored, within ±2 lines (`line_precision`, `line_recall` in `results.jsonl`).
B1 answer items outside `src/` (abapGit's `test/src/`) are neither hits nor misses, because the
ground truth reads `src/` only (`TRUTH_SCOPE` in `scorers.py`; listed as `out_of_scope` in the score). C1's task file sets
`scope` to `crates/`.

B3 matching (`scorers._matches`): an answer item matches a rubric entry when one of the entry's
aliases equals a whole identifier in the item. The identifier may carry a `zcl_abapgit_` /
`zif_abapgit_` / `zcx_abapgit_` prefix. Items naming a local test class (`ltcl_...->...`, or
`<class>->ltcl_...` since Gate 2) are neither hits nor misses. `caller_precision` is reported in `results.jsonl` but not scored, because
the rubric is a draft.

## Briefing

The baseline's sem arm never called a sem tool and still paid about 4,200 extra context tokens per
request for the tool schemas. `--brief` tests whether telling the agent about the tools changes
that. It adds this paragraph after the shared instructions, to the sem arm only:

> Besides the usual tools you have three sem tools for this code base: sem_find, sem_impact and sem_certify. Prefer them over grep for questions of where something is defined, who calls it, and what a change to it affects: sem_find looks entities up by name and lists their callers, sem_impact lists what depends on an entity and which tests to run, and sem_certify summarises what a commit or range changed.

This is the same lever sem's own benchmark calls a briefing. Briefed and unbriefed sem runs answer
different questions (does sem help when the agent is told to use it, vs. when it is left to find
the tools), so each gets its own rows (`brief` column) and its own run id; apply the adoption rule
to one `brief` value at a time, against the grep arm of the same checkpoint.

`--brief sem-first` (`BRIEF_SEM_FIRST`, Gate 2b) is a second briefing, because Gate 2's briefed sem
arm ran grep next to `sem_find` instead of in place of it. It adds this paragraph to the sem arm only:

> Besides the usual tools you have three sem tools for this code base: sem_find, sem_impact and sem_certify. For where-used questions (who calls a method) call sem_find with mode "callers" first and use its answer as the caller list, passing file with the defining file the task names and the bare method name (an interface method zif_x~m is the entity m in the interface's file). Use Grep only if sem_find returns an error or says INCOMPLETE, and then only to check the possible callers it names, not to search the code base again. When sem_find reports the line of each call, use those lines; otherwise read the calling method's range for the line. For other questions, sem_impact lists what depends on an entity and which tests to run, and sem_certify summarises what a commit or range changed.

Its rows carry `brief` = `sem-first` and never mix with `1` rows.

`--brief sem-find-only` (`BRIEF_SEM_FIND_ONLY`, Gate 2c) is `sem-first` for a server that lists
`sem_find` alone (`--mcp-tools sem_find`): its first sentence names that one tool, and the last
sentence, on `sem_impact` and `sem_certify`, is left out. The rest is word for word the same. Its
rows carry `brief` = `sem-find-only`.

`summarize.py` prints a checkpoint's headline table, the sem arm's deltas against the grep arm and the
adoption-rule verdict as Markdown: `python3 bench/abap-agent/summarize.py --checkpoint gate2b --brief sem-first`.
It reads the scores stored in `results.csv`, not rescored ones, and leaves dry-run rows out.

## Adoption rule

Adopt sem for ABAP agents at a checkpoint if, over all repetitions:

- **B1 precision** (mean over tasks) of the sem arm beats the grep arm by **20 points or more**, **or**
- **B2 wall time** (mean per task) of the sem arm is **30% or more lower** at **equal pass rate**
  (same `tests_passed` count), **and in either case**
- the sem arm's **input tokens** are **no more than 15% higher** than the grep arm's.

Measure input tokens as `input_tokens + cache_read_tokens + cache_write_tokens`, the tokens the model read. Cached tokens are cheaper, but they are still context.

## Baseline, 2026-10-05

Checkpoint `baseline`, one repetition, `claude-sonnet-5-5` at effort high through Claude Code 2.1.289.
sem 0.27.0 was built from this branch (`build` `ca85e72845b0`; `crates/` as at `9cc2bab`).
B3: 8 runs. B1: 20 runs. B2: not run (see Known issues). Cumulative cost: **$2.06** by Claude Code's figure.
Per-run means, except cost (summed):

| Class | Arm | Runs | Success | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|
| B1 | grep | 10 | no ground truth | 31,115 | 2,370 | 0.49 | 19.1 | 3.0 | - |
| B1 | sem | 10 | no ground truth | 46,117 (+48%) | 2,459 | 0.55 | 18.7 | 3.1 | 0 |
| B3 | grep | 4 | 1.00 | 118,153 | 3,917 | 0.55 | 34.6 | 6.8 | - |
| B3 | sem | 4 | 1.00 | 109,642 (-7%) | 3,785 | 0.47 | 39.6 | 5.2 | 0 |

"Tokens read" is `input_tokens + cache_read_tokens + cache_write_tokens`, as in the adoption rule.
No run hit an error, the turn bound or the timeout. The sem server was `connected`, with all eight
tools listed, in every sem run. `files_read` is 0 everywhere: both arms read code through `Bash`
(`git show`, `grep -n`, `sed -n`) and `Grep`, never the `Read` tool. Every B1 answer parsed. Its
number of reported calls was within one of the task file's grep call-site count for 9 of 10 targets
in both arms. Both arms reported 2 calls for `b1_10`, where the grep count is 4. The B3 rows were
scored before the B3 matching fix described under Output. Rescoring their stored answers with the fixed scorer gives
the same 1.0 for all eight.

**Reading.** Neither class separates the arms yet. B3 is at its ceiling: both arms find every
rubric entity and every rubric caller with `git show` and grep in five to eleven turns, so this
rubric cannot show a sem advantage. B1 has no ground truth to score against. The sem arm did worse
than "fall back to grep": in all 14 sem runs it never called a single sem tool, not even once to
find out that `sem_find` callers cannot resolve across files yet. It went straight to `Grep` and
`Bash` and still paid for the sem tool schemas and MCP instructions, about 4,200 extra tokens on
every request (first request 10,140 vs 5,913 tokens on `b1_05`). That puts its B1 tokens read 48%
above the grep arm, against the adoption rule's 15% ceiling, before sem contributes anything.

## Gate 2, 2026-10-06

Checkpoint `gate2`, one repetition, `--brief` (the sem arm's prompt carries the briefing; the grep
arm's does not), `claude-sonnet-5-5` at effort high through Claude Code 2.1.289. sem was built from
this branch at `f61bf50` (`build` `f61bf50a7f9c`, clean `crates/`; the binary still reports
`sem 0.27.0`). B1: 20 runs, B3: 8 runs, B2: 12 runs, all 40 planned. Cumulative cost: **$3.13** by
Claude Code's figure. No run hit an error, the turn bound or the timeout, and the sem server was
`connected` in every sem run. Per-run means, except cost (summed):

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| B1 | grep | 10 | 1.000 | 1.000 | 27,127 | 2,289 | 0.49 | 19.1 | 2.5 | - |
| B1 | sem (brief) | 10 | 1.000 | 1.000 | 40,369 (+49%) | 2,164 | 0.59 | 22.7 | 3.2 | 1.0 |
| B1 | baseline grep | 10 | 0.992 | 0.992 | 31,115 | 2,370 | 0.49 | 19.1 | 3.0 | - |
| B1 | baseline sem | 10 | 1.000 | 1.000 | 46,117 (+48%) | 2,459 | 0.55 | 18.7 | 3.1 | 0 |
| B2 | grep | 6 | 6/6 pass | - | 59,542 | 2,118 | 0.45 | 53.3 | 6.5 | - |
| B2 | sem (brief) | 6 | 6/6 pass | - | 91,419 (+54%) | 2,180 | 0.54 | 57.6 (+8%) | 7.0 | 1.5 |
| B2 | baseline | - | not run | | | | | | | |
| B3 | grep | 4 | 0.875 | 1.000 | 108,572 | 4,240 | 0.54 | 39.7 | 6.5 | - |
| B3 | sem (brief) | 4 | 1.000 | 1.000 | 123,847 (+14%) | 3,828 | 0.52 | 37.3 | 7.0 | 1.25 |
| B3 | baseline grep | 4 | 1.000 | 1.000 | 118,153 | 3,917 | 0.55 | 34.6 | 6.8 | - |
| B3 | baseline sem | 4 | 1.000 | 1.000 | 109,642 (-7%) | 3,785 | 0.47 | 39.6 | 5.2 | 0 |

Success and precision are from the scorers as fixed during this run (below), applied to the stored
answers of both checkpoints; the baseline's B1 rows carry no score in `results.csv`, these are its
answers scored now against `whereused.grep.json`. "sem calls" counts `sem_*` entries in
`calls_by_tool` (`results.jsonl`); `results.csv` has no column for it. The only B1 miss at either
checkpoint is the baseline grep arm's `b1_01`, which attributed a call in
`zcl_abapgit_repo_online` to `set_objects` instead of `check_for_valid_branch`. The only B3 miss at
Gate 2 is the grep arm's `b3_01`, which did not name `zcl_abapgit_transport_2_branch` among the
callers left behind (caller recall 0). B2 timings per task, grep vs sem (s): 56.7/64.4, 50.5/52.2,
52.4/61.6, 55.4/50.0, 57.4/64.5, 47.3/52.9; sem was faster on one task of six.

**Scorer fixes.** Two scorers were wrong on real answers; both are fixed in `scorers.py`, and the
stored rows in `results.csv` / `results.jsonl` keep the scores they were written with (the files
are append-only):

- B3: an item naming a local test class under its global class, `zcl_abapgit_repo_online->ltcl_create_branch`,
  counted as a wrong changed entity, because the test-class rule only matched items starting with
  `ltc`. Both arms listed it on `b3_03` (stored 0.8333, entity precision 0.5). Rescored: 1.0 in both arms.
- B1: the real call in `test/src/zcl_abapgit_sap_package_test.clas.testclasses.abap`
  (`check_list_subpackages`, `b1_04`) counted as a false positive, though the ground truth reads
  `src/` only and `docs/abap/census-gate2.md` lists that call as real and out of the truth's scope.
  Every arm at both checkpoints listed it (stored 0.9697, precision 0.9412). Answer items outside
  `src/` are now neither hits nor misses. Rescored: 1.0 in every arm.

Neither fix moves one arm against the other. Stored means were B1 0.997 in both arms and B3 0.833
(grep) vs 0.958 (sem).

**What the briefed sem arm did.** The briefing worked as an instruction: every sem run called a sem
tool, 25 calls in 20 runs, against none in the baseline. It was `sem_find` with `mode: "callers"`
in 22 calls, `sem_grep` 4 times on B2 (`sem_grep` is not named in the briefing), `sem_certify` once
(`b3_04`) and `sem_impact` never, not even on B2, where the briefing names it for "what a change
affects and which tests to run". On B1 and B2 the first turn always sent `sem_find` together with
`Grep` (or `sem_grep`) in the same message, then built the answer from the text search: call lines
come from grep, since `sem_find` reports each caller at its method's line, not the call's.
`sem_find` answered with callers in 8 of 10 B1 runs, and in all 8 its list covered every caller in
the ground truth. In 6 of them it said `complete`; on `b1_04` and `b1_08` it said `INCOMPLETE` and
added possible callers (same-named implementations, the name used as a value) that are not calls.
Six of those eight final answers mention `sem_find`, two of them to say it agreed with grep. In 7 of 22 calls it
answered "matches N definitions; pass file" (B1 `b1_06`, `b1_10`; B2 `b2_02`, `b2_03`, `b2_04`,
where the test class's own same-named test method is the second definition; B3 `b3_02`, `b3_03`),
and the agent retried with `file` once (`b3_02`) and otherwise went on with grep. On B3 `sem_find`
came in the second turn, after `git show`, to look up callers of a changed method; `sem_certify`
was called once, in parallel with `git show`, and the answer was written from both.

**Adoption rule** (`brief` = 1 against the grep arm of `gate2`):

- B1 precision: sem 1.000 vs grep 1.000, **+0 points** (needs +20). Not met.
- B2 wall time at equal pass rate (6/6 each): sem 57.6 s vs grep 53.3 s, **+8%** (needs -30%). Not met.
- Input tokens (tokens read): B1 **+49%**, B2 **+54%**, B3 +14% (ceiling +15%). Over on both
  classes the rule decides on.

**Verdict: drop** for this checkpoint. Neither criterion holds, so the rule's "in between,
per task" case does not arise; no single task shows sem ahead on both B1 precision and tokens, or
on B2 wall time by 30%.

**Reading.** Briefed, the agent did use sem: one `sem_find` per B1 run, one or two sem calls per B2
run, and every answer it gave was as correct as the grep arm's, so sem cost nothing in quality, and
where `sem_find` answered, its callers covered the ground truth. It did not use sem in place of
grep, though: it ran both side by side in the first turn and took its file and line answers from
the text search, so the sem calls added a cold index, the tool schemas and their output to the
context (+49% tokens on B1, +54% on B2) without removing a single grep. The benchmark also cannot
show a precision gain: grep scores 1.000 on B1 at this checkpoint (0.992 at baseline), because
abapGit's method names are distinctive enough that a case-insensitive grep plus reading the
enclosing method is already exact, so the 20-point B1 criterion is out of reach on these ten
targets. On B2 the edit-and-test loop dominates (both arms about 50 to 60 s, all twelve passing,
no caller missed) and caller lookup is a small part of it, so sem made the runs slightly slower,
not faster, and `sem_impact` (the tool meant for "which tests to run") was never called. The one
concrete sem defect the transcripts show is the "matches N definitions" answer on 7 of 22 lookups,
5 of them caused by a local test method that shares the target's name, which sent the agent back to
grep; a B1/B2 set where grep is ambiguous (common names, dynamic calls, interface dispatch) is what
it would take for this benchmark to show what sem resolves that grep cannot.

## Gate 2b, pre-registered

Written before any `gate2b` row exists. Gate 2's verdict was drop, but the benchmark could not show
a difference: grep scored 1.000 on every B1 target, and the briefed sem arm ran grep next to
`sem_find` instead of in place of it. Gate 2b changes two things: a where-used class on targets that
are ambiguous for a text search (B1A), and the `sem-first` briefing. The sem fixes it tests
(`sem_find` callers no longer refusing on a same-named test method, and call lines where available)
are in the build under test; `build` and `sem_version` in each row say which.

| Setting | Value |
|---|---|
| Checkpoint | `gate2b` |
| Sem arm | `--brief sem-first` |
| Repetitions | 3 per task |
| Classes | B1A, B1, B2, B3 |
| Model, effort | `claude-sonnet-5-5`, high, as Gate 2 |
| Cap | 15 USD (`--cap-usd 15`) |

```sh
python3 bench/abap-agent/run.py --checkpoint gate2b --brief sem-first --class all --reps 3 --cap-usd 15
python3 bench/abap-agent/summarize.py --checkpoint gate2b --brief sem-first
```

**Rule.** Adopt sem for the ABAP agents if either holds, both as means over the 3 repetitions:

| Criterion | Sem arm against grep arm |
|---|---|
| B1A | success at least the grep arm's, **and** tokens read at most 85% of the grep arm's (a saving of 15% or more) |
| B2 | wall time 30% or more lower at equal pass rate (same `tests_passed` count), **and** tokens read at most 15% higher (unchanged from Gate 2) |

Otherwise drop the port. There is no third re-run. B1 and B3 are reported, not decided on.
Tokens read is `input_tokens + cache_read_tokens + cache_write_tokens`, as before, and `summarize.py`
computes the verdict from exactly this rule.

**One-sided risk.** The B1A targets were chosen to be hard for grep. A pass says sem helps where grep
is ambiguous, not everywhere: B1, B2 and B3 are where an ordinary agent task sits, and Gate 2 showed
no gain there. A fail on B1A would be the stronger result, since it is the class built to favour sem.

## Gate 2b, 2026-10-06

Checkpoint `gate2b`, `--brief sem-first`, 3 repetitions, `claude-sonnet-5-5` at effort high through
Claude Code 2.1.291, build `15ad2dd5224b` and later on this branch (the `sem_find` fixes of 40acad8
are in it; the binary still reports `sem 0.27.0`). 187 of the 192 planned runs, **13.21 USD** by
Claude Code's figure against the 15 USD cap. The run was interrupted once: the session's container
restarted after the first 10 B1A rows (b1a_01 complete, b1a_02 two repetitions per arm), and the
remainder was run from `--task` lists, so b1a_02's third repetition carries `rep` 0 again. All rows
are kept. B3 ran under a per-class cap of 2 USD and reached it on the last grep run (`b3_04`, rep 2,
`error` set, no score), so B3 has 10 grep and 9 sem runs. Per-run means, except cost (summed);
deltas are the sem arm against the grep arm (`summarize.py --checkpoint gate2b --brief sem-first`):

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| B1A | grep | 36 | 1.000 | 1.000 | 55,601 | 2,500 | 2.71 | 22.9 | 5.1 | - |
| B1A | sem (sem-first) | 36 | 1.000 | 1.000 | 75,095 (+35%) | 2,225 (-11%) | 2.80 (+3%) | 25.1 (+10%) | 4.5 (-10%) | 1.0 |
| B2 | grep | 18 | 18/18 pass | - | 66,749 | 2,204 | 1.41 | 59.4 | 6.9 | - |
| B2 | sem (sem-first) | 18 | 18/18 pass | - | 100,819 (+51%) | 2,350 (+7%) | 1.70 (+20%) | 65.7 (+11%) | 7.3 (+5%) | 1.0 |
| B1 | grep | 30 | 1.000 | 1.000 | 29,147 | 2,332 | 1.44 | 18.5 | 2.8 | - |
| B1 | sem (sem-first) | 30 | 0.971 | 0.971 | 31,684 (+9%) | 1,621 (-30%) | 1.14 (-21%) | 16.9 (-9%) | 1.8 (-36%) | 1.0 |
| B3 | grep | 10 | 0.900 | 0.900 | 80,306 | 3,522 | 1.10 | 34.5 | 5.4 | - |
| B3 | sem (sem-first) | 9 | 1.000 | 1.000 | 94,220 (+17%) | 3,627 (+3%) | 0.92 (-17%) | 37.0 (+7%) | 6.2 (+15%) | 2.78 |

**Against the pre-registered rule:**

- B1A: success 1.000 against 1.000 (met), tokens read 135% of the grep arm's (needs 85% or less). Not met.
- B2: 18/18 passes in both arms (met), wall time 111% of the grep arm's (needs 70% or less), tokens
  read 151% (needs 115% or less). Not met.

**Verdict: drop.** There is no third re-run.

**Where the tokens go.** The gap is structural, not a wrong answer. The first API call of every sem
run carries the schemas of the eight `sem mcp` tools: 10.5k to 11.3k tokens of context against 6.0k
to 6.8k in the grep arm, the same 4.5k on every class. Claude Code re-reads that context on every
turn, so over the 5 to 6 turns of a B1A or B2 run it alone is some 25k tokens, which is the whole
B1A gap (19.5k) with the sem arm's fewer turns taken off. `sem_find` itself was cheap: one call per
run, about 4.8k characters (1.2k tokens) per answer. To break even the sem arm would have to finish
in 40% fewer turns than grep, and grep needs six.

**What `sem_find` did.** Every callers answer on B1A said INCOMPLETE (36 of 36: on names declared in
several classes the resolver reports the untyped `->` calls it did not bind, which is honest), so the
briefing sent the agent back to Grep for the possible callers it named, and the sem arm still made
1.8 Grep and 1.5 Bash calls per run. The "matches N definitions" refusals of Gate 2 are gone. On B1
the sem arm needed 1.8 tool calls against 2.8 and read 30% fewer output tokens, and cost 21% less,
the one class where it is cheaper; its 0.971 is one run (`b1_05`, rep 1) that named the callers as
`class->deserialize` instead of `class->zif_abapgit_object~deserialize`, with every file and line
right (`line_precision` 1.0). On B3 the sem arm scored 1.000 against 0.900 and cost 17% less, the
same direction as Gate 2; the rule does not decide on B3.

**What grep did.** Sonnet with Grep, Read and Bash scored 1.000 on all twelve B1A targets, where only
32% of the call-shaped grep hits are real calls: it read the receiver declarations itself in two or
three extra calls. The premise of B1A, that a text-search agent would get ambiguous names wrong, did
not hold for this model.

**Reading.** The resolver is right (story 2.7: 100% precision, 94% recall; B1A: no miss in 36 runs
where `sem_find` was used), and on review (B3) sem is ahead both times it was measured. But a static
index behind an eight-tool MCP server costs more context per turn than it saves on a single-file
question, and grep on this model is not wrong often enough on ABAP to pay for it. A slimmer server
(one tool, a short description) would remove most of the per-turn cost; that is the one lever this
run identifies, and under the rule it is for the owner to decide whether it is worth a run of its own.

## Gate 2c, slim server (reporting only)

Gate 2b put the sem arm's extra tokens on the schemas of the eight `sem mcp` tools, re-read on
every turn. Gate 2c asks whether a server that lists `sem_find` alone closes the B1A gap. It is a
reporting experiment, not a re-run of the adoption rule: Gate 2b's verdict, drop, stands whatever
this shows, and there is still no third re-run.

`sem mcp` now reads `SEM_MCP_TOOLS` (comma-separated tool names) and lists only those; the others
stay callable by name, and no description changed. The harness passes it with `--mcp-tools`. The
briefing is `sem-first` with the sentences on `sem_impact` and `sem_certify` taken out (`--brief
sem-find-only`, see "Briefing"), since the run does not list them. The grep arm was re-run the same
day, so both arms are measured under the same conditions.

| Setting | Value |
|---|---|
| Checkpoint | `gate2c` |
| Sem arm | `--brief sem-find-only --mcp-tools sem_find` |
| Repetitions | 3 per task |
| Classes | B1A |
| Model, effort | `claude-sonnet-5-5`, high, Claude Code 2.1.291 |
| Build | `abe1d86e5c0b` (`sem 0.27.0`) |
| Cap | 6 USD |

```sh
python3 bench/abap-agent/run.py --checkpoint gate2c --brief sem-find-only --mcp-tools sem_find --class B1A --reps 3 --cap-usd 6
python3 bench/abap-agent/summarize.py --checkpoint gate2c --brief sem-find-only
```

The run went in seven invocations of that command, one or two `--task` each, so no foreground call
outlived its timeout; each got the rest of the 6 USD as `--cap-usd`. The plan's order and `rep`
numbers are the same as one invocation's.

**The listing.** `tools/list` from the release binary over stdio, on the pinned abapGit checkout,
as compact JSON:

| `SEM_MCP_TOOLS` | Tools | Bytes | About tokens (bytes / 4) |
|---|---|---|---|
| unset | 8 | 9,876 | 2,469 |
| `sem_find` | 1 | 2,837 | 709 |

`sem_find` is the largest tool: 2,835 bytes of JSON, most of it its parameter schema, and a
description of 570 characters, well under the 1,500 at which it would have been worth shortening.
The server instructions (1,003 characters) are sent in both cases and still name all eight tools.

**Results.** 72 of 72 planned runs, **5.30 USD** by Claude Code's figure. No run hit an error, the
turn bound or the timeout, and the sem server was `connected` and listed `sem_find` alone in every
sem run. Per-run means, except cost (summed); deltas are the sem arm against the grep arm
(`summarize.py --checkpoint gate2c --brief sem-find-only`):

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| B1A | grep | 36 | 1.000 | 1.000 | 53,192 | 2,495 | 2.67 | 22.9 | 5.0 | - |
| B1A | sem (sem-find-only) | 36 | 1.000 | 1.000 | 63,054 (+19%) | 2,162 (-13%) | 2.63 (-2%) | 26.2 (+14%) | 4.8 (-4%) | 1.03 |
| B1A | Gate 2b grep | 36 | 1.000 | 1.000 | 55,601 | 2,500 | 2.71 | 22.9 | 5.1 | - |
| B1A | Gate 2b sem (sem-first) | 36 | 1.000 | 1.000 | 75,095 (+35%) | 2,225 (-11%) | 2.80 (+3%) | 25.1 (+10%) | 4.5 (-10%) | 1.0 |

**First-call context** (the first assistant message's `cache_creation + cache_read + input` tokens,
mean over the 36 runs of each arm, from the transcripts):

| Arm | Gate 2c | Gate 2b |
|---|---|---|
| grep | 6,253 | 6,250 |
| sem | 7,868 | 10,741 |
| sem minus grep | 1,615 | 4,491 |

The slim server takes 2,873 tokens off every request of the sem arm, 64% of its per-request overhead. What is left,
1.6k, is the one schema, the server instructions and the briefing paragraph (615 characters), at
the same ratio of tokens to bytes as the full listing showed in Gate 2b.

**What `sem_find` did.** It was called in every sem run, 37 times in 36 runs (on `b1a_08` rep 2 a
second time with `limit: 200`), and never returned an error. All 37 answers said INCOMPLETE, as in
Gate 2b, at about 4.9k characters each, so the briefing sent the agent to Grep for the possible
callers: the sem arm made 2.2 Grep and 1.3 Bash calls per run, against 2.5 and 2.3 in the grep arm
(Gate 2b's sem arm: 1.8 and 1.5). 30 of 36 final answers mention `sem_find`. Turns per run: 5.8 sem,
6.0 grep.

**Against Gate 2b's B1A criterion**, for reference only: success 1.000 against 1.000, tokens read
119% of the grep arm's (needs 85% or less). Not met. **The adoption verdict is unchanged: drop.**

**Reading.** The slim server does what Gate 2b predicted for the fixed cost: the sem arm's first
request falls from 10.7k to 7.9k tokens, and its B1A tokens read from +35% to +19% over grep, at
the same success, with cost now level with grep (-2%). It does not close the gap. The remaining
1.6k per request, over 5.8 turns, is 9.3k of the 9.9k difference by itself; `sem_find`'s answer
(1.2k tokens, in the context of every later turn) and the text-search output it saves roughly
cancel (inferred from the totals, not measured per turn). The turn count did not move:
with every answer INCOMPLETE on these targets, the agent still checks the possible callers with
Grep and Bash, so `sem_find` replaces about one text search, not the read-the-receiver loop that
makes grep exact here. To reach the 85% Gate 2b asked for, the sem arm would need fewer turns than
grep, which on B1A needs an answer the agent can take without checking, a complete verdict, more
than a smaller listing.

## Control: Rust where-used (reporting only)

The task file's `repo.source` is a detached checkout of the pinned commit, which the harness clones from; it is not
committed. Recreate it before a run or a dry run with
`git -C <this clone> worktree add --detach /tmp/claude-0/sem-control-src dc2cc4d7173d9f3eab87d491b351927fbe2f1f63`.

Checkpoint `control`, class C1, `--brief sem-first`, 3 repetitions, `claude-sonnet-5-5` at effort high through
Claude Code 2.1.291, the same harness, arms, briefing and turn bound as Gate 2b. sem served MCP from
`/home/user/sem/crates/target/release/sem`, built from this branch at `767f67e` (the merge of the `sem_find` callers
fixes of 40acad8, as in Gate 2b; the binary reports `sem 0.27.0`). The rows' `build` column says `1d00678b37eb`,
because it records the checkout the harness ran from (the commit that added C1), not the binary's. `abapgit_commit`
is `dc2cc4d7173d`, sem at upstream's merge-base. 72 of 72 planned runs, **4.23 USD** by Claude Code's figure against
a 6 USD cap. No run hit an error, the turn bound or the timeout, and the sem server was `connected` in every sem run.

```sh
python3 bench/abap-agent/run.py --checkpoint control --brief sem-first --class C1 --reps 3 --cap-usd 6 \
  --sem-binary /home/user/sem/crates/target/release/sem --work-dir /tmp/sem-bench-control
python3 bench/abap-agent/summarize.py --checkpoint control --brief sem-first
```

**What the control isolates.** Gate 2b found the sem arm reading 35% more tokens than grep on B1A, with grep at
1.000. That can mean two things: sem does not pay for itself on a short where-used task with this model, or the ABAP
port is what costs. C1 keeps everything but the language and the repository: the same question shape (a name
declared 3 or more times, strata as B1A's), the same prompt clauses, the same briefing, on a Rust repository whose
resolver is upstream's. If the gap were the port's, it should close on Rust. The targets are under "C1 targets".

Per-run means, except cost (summed); deltas are the sem arm against the grep arm (`summarize.py --checkpoint
control --brief sem-first`), with Gate 2b's B1A rows for comparison:

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| C1 | grep | 36 | 1.000 | 1.000 | 45,458 | 1,664 | 2.23 | 17.1 | 4.0 | - |
| C1 | sem (sem-first) | 36 | 1.000 | 1.000 | 53,655 (+18%) | 1,457 (-12%) | 2.00 (-10%) | 17.9 (+5%) | 3.6 (-9%) | 1.25 |
| B1A (Gate 2b) | grep | 36 | 1.000 | 1.000 | 55,601 | 2,500 | 2.71 | 22.9 | 5.1 | - |
| B1A (Gate 2b) | sem (sem-first) | 36 | 1.000 | 1.000 | 75,095 (+35%) | 2,225 (-11%) | 2.80 (+3%) | 25.1 (+10%) | 4.5 (-10%) | 1.0 |

Tokens read per task, grep vs sem (mean of 3): c1_01 35,545 / 55,001 (+55%), c1_02 52,939 / 73,805 (+39%), c1_03
61,766 / 73,261 (+19%), c1_04 37,843 / 46,456 (+23%), c1_05 82,269 / 48,895 (-41%), c1_06 39,959 / 61,308 (+53%),
c1_07 41,479 / 49,873 (+20%), c1_08 73,961 / 49,974 (-32%), c1_09 35,070 / 71,564 (+104%), c1_10 23,791 / 44,014
(+85%), c1_11 23,947 / 34,313 (+43%), c1_12 36,926 / 35,394 (-4%). Turns: 5.0 grep, 4.6 sem.

**First-call context.** The first assistant message's `input_tokens + cache_read_input_tokens +
cache_creation_input_tokens`, from the transcripts, mean over runs: grep **6,331** (6,296 to 6,465), sem **10,821**
(10,786 to 10,955). The difference, 4,490 tokens, is the eight `sem mcp` tool schemas and the server's instructions,
the same 4.5k as on every ABAP class; it does not depend on the language. Over the sem arm's 4.6 turns it is some
21k tokens read, more than the whole C1 gap (8.2k), so on Rust the sem arm's tool traffic was lighter than grep's and
the schemas alone put it over.

**What `sem_find` did.** Every sem run called it first, as briefed: 42 calls in 36 runs. 27 answered with a caller
list, and all 27 said INCOMPLETE (on names declared several times the resolver reports the possible callers it did
not bind, as on ABAP). 10 were refusals, in 9 runs on three targets: "no entity named 'lower'" / "'layout'" on
`c1_01` and `c1_02` (a trait method declared without a body is not an entity, so the trait targets' question cannot be
asked), and "'identity' matches 2 definitions; pass file" on `c1_09` (4 calls in 3 runs), where both definitions are in
the file the task names (`FastExtractorSet::identity` and a test stub's impl). The last 5 listed `lang.rs` with `in`
after the `c1_01` / `c1_02` refusal, before the agent went to Grep. The sem arm made 1.47 Grep and 0.53 Bash calls per run (grep arm 1.94 and 1.50; B1A's sem
arm 1.8 and 1.5). Where the answer was usable the arm was cheaper on the targets with the most call sites
(`QueryIndex::lookup` -41%, `QueryIndex::file_count` -32%), and most expensive where it refused (`c1_09` +104%,
`c1_01` +55%) or where grep is trivially exact (`c1_10`, fourteen `self.auth_header()` calls in one file, +85%).

**What grep did.** 1.000 on all twelve targets, all 36 runs, where 33% of the call-shaped hits are calls of the
target, as on B1A: the model read the receivers itself, including the three traps the targets were picked for (the
bare `lower(` inside each `impl Lang`, `ParserRegistry`'s same-named method, two `CloudClient` types).

**Reading.** The control reproduces Gate 2b's shape on a language upstream sem was built for: equal answers, the sem
arm reading more context (+18% against +35%), and the gap explained by the same 4.5k-token per-turn schema cost.
Under Gate 2b's B1A criterion (tokens read at most 85% of grep's) C1 would fail too, at 118%. So the result is mostly
"sem does not pay for itself on short where-used tasks with this model", not "the ABAP port is the problem". What
the port adds is the other half of the B1A gap; that it comes from the extra Bash and Grep the ABAP sem arm ran after
its INCOMPLETE answers is inferred from the call counts above, not measured. Two findings are about sem on Rust, not about the port: a trait method without a default body has no entity,
and `sem_find` cannot pick between two same-named methods of different types in one file by `file` alone. Neither
moved a score. **The adoption verdict is unchanged: drop**, as Gate 2b decided; this control is reporting only and
decides nothing.

## Gate 2d, CLI arm (reporting only)

Gate 2b, Gate 2c and the control put the sem arm's extra tokens on the MCP tool schemas and server instructions,
re-read on every turn: 4.5k tokens per request with eight tools, 1.6k with `sem_find` alone. The command line costs
nothing per turn, because the agent already has Bash. Gate 2d asks whether a `cli` arm, the grep arm's tools plus
`sem` on PATH and a one-paragraph briefing, reads fewer tokens than grep at equal success on B1A. It is a reporting
experiment: Gate 2b's verdict, drop, stands whatever this shows, and there is still no third re-run.

`--arm cli` gets exactly the grep arm's `--tools` and `--allowedTools` and no `--mcp-config`. Its `claude` runs
through `env` with the sem binary's directory first on PATH and `SEM_ENV` set (telemetry, update checks, cloud and
network off, as for `sem mcp`). `--brief cli` (`BRIEF_CLI`) is the only prompt change, to the cli arm only:

> Besides the usual tools you have the sem command line on PATH for this code base. For where-used questions (who calls a method) run `sem find NAME --callers --file FILE` in Bash first, passing the bare method name as NAME and the defining file the task names as FILE (an interface method zif_x~m is the entity m in the interface's file), and use its resolved callers and their call lines as the caller list. Use Grep only if sem fails or says INCOMPLETE, and then only to check the possible callers it names, not to search the code base again.

The grep arm was not re-run: the comparison is against Gate 2c's grep rows, the same 12 targets, 36 runs, the same
day, model and Claude Code (`summarize.py --grep-from gate2c`).
| Arm | `--arm cli --brief cli` |
| Grep side | Gate 2c's grep rows (36), not re-run |
| Repetitions | 3 per task |
| Classes | B1A |
| Model, effort | `claude-sonnet-5-5`, high, Claude Code 2.1.291 |
| sem | `/home/user/sem/crates/target/release/sem`, built from this branch at `767f67e` (the `sem_find` callers fixes, as in Gate 2b and the control; `sem 0.27.0`) |
| Cap | 3.5 USD |

```sh
python3 bench/abap-agent/run.py --checkpoint gate2d --brief cli --arm cli --class B1A --reps 3 --cap-usd 3.5 \
  --sem-binary /home/user/sem/crates/target/release/sem --work-dir /tmp/sem-bench-gate2d
python3 bench/abap-agent/summarize.py --checkpoint gate2d --brief cli --grep-from gate2c
```

The run went in four invocations of that command, by `--task`, each with the rest of the 3.5 USD as `--cap-usd`.
The rows' `build` column says `93ea20e89789`, the harness commit that added the arm, not the binary's.

**The incantation.** On the pinned abapGit checkout, with the release binary, `sem find NAME --callers --file FILE`
with the bare name and the defining file (the interface file for `b1a_01`). `--file` is not in `sem find --help`,
which names `--in`; the two printed byte-identical output on all three targets. Output in bytes:

| Target | Default (text) | `--json` | `--limit 10` | `--limit 0` | `--limit 500` |
|---|---|---|---|---|---|
| `b1a_01` `is_active`, interface | 4,974 | 33,966 | 2,841 | 1,097 | 21,633 |
| `b1a_05` `list`, short name | 4,522 | 7,164 | 2,494 | 917 | 4,522 |
| `b1a_10` `normalize`, multi | 4,854 | 8,664 | 3,228 | 595 | 4,854 |

`--json` is 1.6 to 7 times larger. `--limit` applies to the resolved callers and to the possible callers alike:
`--limit 0` keeps only the INCOMPLETE summary, and `--limit 10` drops 9 of `b1a_10`'s 19 resolved callers, the
answer itself. The default (25 per list) is the smallest form that keeps the answer, so the briefing names no flag.
All three answers say INCOMPLETE, as `sem_find` does over MCP, and they are the same text at the same size (Gate 2c:
about 4.9k characters).

**The index.** `sem find` on an unindexed checkout prints nothing extra: it builds the index and answers, 2.2 to
3.3 s on a fresh clone of the pinned commit (5.0 s on the first try), then 0.2 s warm. It writes
nothing into the checkout; the index goes to `~/.cache/sem/repos/<hash of the checkout's path>` (`cache.db` and
`index.sem`, about 32 MB), so every run's checkout, at its own path, gets its own index, as the MCP arm's server
does. Well under 20 s, so nothing is pre-built: the cold index is in the cli arm's wall time, as it is in the sem arm's.

**Results.** 36 of 36 planned runs, **2.38 USD** by Claude Code's figure. No run hit an error, the turn bound or the
timeout. Per-run means, except cost (summed); deltas are the cli arm against Gate 2c's grep arm
(`summarize.py --checkpoint gate2d --brief cli --grep-from gate2c`), with Gate 2c's sem arm for comparison:

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| B1A | grep (Gate 2c) | 36 | 1.000 | 1.000 | 53,192 | 2,495 | 2.67 | 22.9 | 5.0 | - |
| B1A | cli | 36 | 1.000 | 1.000 | 51,460 (-3%) | 2,117 (-15%) | 2.38 (-11%) | 23.6 (+3%) | 4.0 (-21%) | 1.14 |
| B1A | Gate 2c sem (sem-find-only) | 36 | 1.000 | 1.000 | 63,054 (+19%) | 2,162 (-13%) | 2.63 (-2%) | 26.2 (+14%) | 4.8 (-4%) | 1.03 |

sem calls for the cli arm are the `Bash` calls whose command starts with `sem ` (`sem_cli_calls` in `results.jsonl`).

Tokens read per task, grep vs cli (mean of 3): b1a_01 59,753 / 75,698 (+27%), b1a_02 62,428 / 69,561 (+11%), b1a_03
41,446 / 53,068 (+28%), b1a_04 24,455 / 24,311 (-1%), b1a_05 62,920 / 49,128 (-22%), b1a_06 39,339 / 36,337 (-8%),
b1a_07 49,942 / 53,222 (+7%), b1a_08 67,459 / 53,049 (-21%), b1a_09 32,431 / 27,262 (-16%), b1a_10 83,049 / 42,885
(-48%), b1a_11 70,939 / 41,939 (-41%), b1a_12 44,145 / 91,061 (+106%). Turns: 4.97 cli, 6.00 grep, 5.78 Gate 2c sem.

**First-call context** (the first assistant message's `cache_creation + cache_read + input` tokens, mean over the 36
runs of each arm, from the transcripts):

| Arm | First call | Minus grep |
|---|---|---|
| grep (Gate 2c) | 6,253 (6,208 to 6,372) | - |
| cli | 6,425 (6,380 to 6,544) | 172 |
| Gate 2c sem, `sem_find` alone | 7,868 | 1,615 |
| Gate 2b sem, eight tools | 10,741 | 4,491 |

The cli arm's 172 tokens are the briefing paragraph (543 characters). The schema and server-instruction cost is gone.

**What the agent did.** Every cli run ran the briefed command first, word for word (`sem find NAME --callers --file
FILE`, no flag), and all 36 answers said INCOMPLETE, as over MCP in Gate 2b and 2c; the first answer averaged 4,738
bytes (1,849 for `parse_line` to 5,740 for `to_abap`). Four runs (`b1a_08` three times, `b1a_12` once) ran it a
second or third time with `--limit 100` piped through `grep -v` or `tail`, 41 sem calls in all; three of those
pipes cut the INCOMPLETE line, so no answer was complete. The agent then checked with `grep` inside Bash, not with
the Grep tool: 2.75 other Bash calls, 0.08 Grep and no Read per run, against 2.28 Bash, 2.53 Grep and 0.19 Read in
the grep arm. That check was not limited to the possible callers sem named: 33 of 36 runs ran a recursive grep over
`src/` (about 1.9 per run, by a pattern match on the commands; the grep arm 2.3, Gate 2c's sem arm 2.0). On
`b1a_04` (every run) and `b1a_09` (two runs of three) the answer plus one Bash check was enough, in 3 turns. 33 of 36 final answers mention sem.

**Against Gate 2b's B1A criterion**, for reference only: success 1.000 against 1.000, tokens read 97% of the grep
arm's (needs 85% or less). Not met. **The adoption verdict is unchanged: drop.**

**Reading.** The CLI removes the per-turn cost that Gate 2b and 2c measured: the cli arm's first request is 172
tokens over grep's, against 1.6k for the one-tool server and 4.5k for the full one. With it gone, sem is no longer
behind: at the same success the cli arm read 3% fewer tokens than grep, cost 11% less, and finished in one turn
fewer (5.0 against 6.0), the first sem configuration on B1A to read fewer tokens than grep. It is not the 15% saving
Gate 2b asked for, and the per-task spread (-48% to +106%) is wider than the mean difference. What is left is the
answer itself: every answer is INCOMPLETE on these targets, so the agent still checks, mostly by a recursive grep it
was told not to run, and the 4.7k-byte answer stays in the context of every later turn. On the targets where the
resolved callers are most of the answer (`b1a_10`, `b1a_11`, 19 callers each) the cli arm read 41 to 48% less; on
the interface and `is_empty` targets, where sem lists many possible callers, it read more. The lever this run
identifies is the same as Gate 2c's last one, a complete verdict on more targets, not a cheaper transport.
## Gate 2d, R1 read-method (reporting only)

**The question.** Every class before R1 asks where-used questions, which read little source. Real agent work on ABAP
reads large classes to understand one method, and that is where context goes. Does a tool that returns one method by
name save input tokens against an agent that has to find and read it with Grep and Read, at equal correctness? This is
reporting only: it has no adoption criterion, and **the adoption verdict is unchanged: drop**, as Gate 2b decided.

| Setting | Value |
|---|---|
| Checkpoint | `gate2d` |
| Class | R1 (see "R1 targets": the selection rule, the eight targets, the scorer's normalisation) |
| Arms | grep, and the cli arm (`--arm cli --brief cli`, merged from `bench/gate2d-cli`): the grep arm's tools exactly, with the sem binary's directory first on `PATH`, no MCP server |
| Repetitions | 3 per task per arm, 48 runs |
| Model, effort | `claude-sonnet-5-5`, high, Claude Code 2.1.291 |
| sem | `/home/user/sem/crates/target/release/sem`, the release binary built at `767f67e` on this branch (`sem 0.27.0`); not rebuilt. The rows' `build` column says `72a6c38334ac`, the harness commit the run started from, not the binary's |
| Cap | 3.5 USD |

```sh
python3 bench/abap-agent/run.py --checkpoint gate2d --arm grep --class R1 --task r1_01 --reps 3 --cap-usd 3.5 \
  --sem-binary /home/user/sem/crates/target/release/sem --work-dir /tmp/sem-bench-r1
python3 bench/abap-agent/run.py --checkpoint gate2d --arm cli --brief cli --class R1 --task r1_01 --reps 3 --cap-usd 3.5 \
  --sem-binary /home/user/sem/crates/target/release/sem --work-dir /tmp/sem-bench-r1
python3 bench/abap-agent/summarize.py --checkpoint gate2d --brief cli --class R1
```

`--arm cli` runs alone, so each task took two invocations, one per arm, each with the rest of the 3.5 USD as
`--cap-usd`. The arm that went first alternated by task (cli first on `r1_01`, `r1_03`, `r1_05` and `r1_07`), in place
of the per-repetition alternation of a `--arm both` run.

**The CLI route.** What the release binary offers for reading one method by name, probed on a checkout of the pinned
commit:

| Command | Prints | Bytes, `r1_01` / `r1_08` |
|---|---|---:|
| `sem find NAME --file F` | `method NAME F:START`: the start line only | 84 / 80 |
| `sem find NAME --file F --json` | id, name, type, file, `start_line`, `end_line` | 260 / 252 |
| `sem find NAME --file F --context` | the body, then its callees, callers and transitive neighbours, packed into an 8,000-token budget | 6,331 / 14,759 |
| `sem find NAME --file F --context --budget 900` | the same, packed tighter | 5,626 / 5,206 |
| `sem find NAME --file F --refs` | what it uses: methods, classes, types and constants, each with its definition's line, not the call's | 594 / 1,973 |
| the method's own lines (`sed -n START,ENDp`) | the body | 3,148 / 3,406 |
| the whole file | | 89,896 / 48,792 |

No subcommand prints just an entity's body (`sem --help` and `sem find --help` list no `body`, `show` or `read`).
`--context` is the one that prints code by name, but always with the neighbours: 2 to 4 times the body, and with a
budget too small for them it leaves out the target as well (`--budget 1`: "target omitted"). `--file` is not in
`sem find --help` but is accepted. An interface method a class implements is found only by its full name:
`sem find deserialize --file src/objects/zcl_abapgit_object_fugr.clas.abap --json` returns `[]`, with
`zif_abapgit_object~deserialize` it returns the range. So the CLI route is `sem find NAME --file F --json` for the
range, then `Read` with offset and limit.

**The briefing.** The cli arm's paragraph (`BRIEF_CLI`, the where-used advice) plus, on R1 only, one sentence
(`BRIEF_CLASS_SUFFIX[("cli", "R1")]` in `run.py`; the `brief` column stays `cli`):

> To read one method, run `sem find NAME --file FILE --json` in Bash, with NAME the method's name after the class as the task writes it (zif_x~m for an interface method a class implements), which prints its start_line and end_line, and Read only that range of the file (offset and limit), not the whole file.

The dry run's tool check runs that command (2.3 to 2.7 s on a cold index, 252 to 266 bytes). The first two cli
transcripts were read before going on: each made the `sem find ... --json` call, then one `Read` of exactly the range.

**Results.** 48 of 48 planned runs, **1.34 USD** by Claude Code's figure. No run hit an error, the turn bound or the
timeout. Per-run means, except cost (summed); deltas are the cli arm against the grep arm (`summarize.py --checkpoint
gate2d --brief cli --class R1`):

| Class | Arm | Runs | Success | Precision | Tokens read | Output tokens | Cost (USD) | Wall time (s) | Tool calls | sem calls |
|---|---|---|---|---|---|---|---|---|---|---|
| R1 | grep | 24 | 1.000 | 1.000 | 24,245 | 985 | 0.71 | 11.2 | 2.2 | - |
| R1 | cli | 24 | 1.000 | 1.000 | 22,531 (-7%) | 838 (-15%) | 0.64 (-10%) | 13.1 (+17%) | 2.0 (-8%) | 1.0 |

From `results.jsonl` and the transcripts, per run:

| Arm | First-call context | bytes_read | files_read | Read calls | Grep calls | Bash calls | Turns | line_precision |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| grep | 6,428 | 3,550 | 1.0 | 1.0 | 1.08 | 0.13 | 3.21 | 1.000 |
| cli | 6,694 | 2,958 | 1.0 | 1.0 | 0 | 1.04 | 3.04 | 1.000 |

First-call context is the first assistant message's `input_tokens + cache_read_input_tokens +
cache_creation_input_tokens` (grep 6,420 to 6,438, cli 6,686 to 6,704); the 266 tokens between them are the cli
briefing. Of `bytes_read`, `Read` returned 2,913 bytes per call in the grep arm and 2,710 in the cli arm, the
`Grep` calls 584 bytes each, and `sem find --json` 238 bytes each (one `echo done` included). Every `Read` in both
arms had an offset and a limit; no run read a whole file.

Tokens read per task, grep / cli (mean of 3): r1_01 22,734 / 22,714 (-0%), r1_02 30,444 / 22,923 (-25%), r1_03
22,122 / 21,897 (-1%), r1_04 21,653 / 21,772 (+1%), r1_05 24,282 / 21,469 (-12%), r1_06 25,490 / 25,289 (-1%), r1_07
21,435 / 21,471 (+0%), r1_08 25,805 / 22,712 (-12%).

**What grep did.** The premise of R1, that a text-search agent reads the large class, did not hold. In all 24 runs
the grep arm ran one `Grep` in the file the prompt names, in 18 of them for `METHOD name|ENDMETHOD` (the method's first
line and every `ENDMETHOD` line of the file), in the rest for the name alone or with a second name, and then one
`Read` of the range, overshooting the method by 16 lines on average (0 to 88). Two of the three `r1_02` runs added a `Grep` for
`escape_name` to check that it is a method; that is the -25%. The bare name's repo-wide grep hits (up to 337 in 194
files) never mattered: the prompt names the file.

**What the CLI did.** Every cli run made exactly one `sem find ... --json` call (24 of 24, `sem_cli_calls`), with the
`zif_abapgit_object~` name on `r1_03` and `r1_04`, and read exactly the range. It replaced the 584-byte `Grep` with a
238-byte answer and the overshoot with an exact limit, and finished in fewer turns (3.04 against 3.21). Wall time
rose 17%, about the 2.5 s sem takes to index the cold checkout.

**Reading.** At equal correctness (1.000 in both arms, line precision 1.000) the cli arm read 7% fewer tokens and cost
10% less, after paying 266 tokens per request for its briefing. But the saving is in locating the method, not in
reading it: the grep agent already reads only the method, because Sonnet greps for the `METHOD` line and reads a
window, so the "large class" is never in context in either arm. On this question, with the file named, a by-name
read tool can save little more than this: the grep arm's `Grep` output (584 bytes, about 150 tokens) and its read
overshoot are the whole gap, and the per-request briefing costs about as much again. Where the gap is large (`r1_02`,
`r1_05`, `r1_08`, -12% to -25%) it is one extra grep turn or a wide window in the grep arm, not a whole file read. The
CLI cannot return the body alone (`--context` returns 2 to 4 times it). A command that did would save a turn, and a turn
here re-reads the whole 6.5k-token context, so up to a third of a run's tokens; but the grep arm could save the same
turn with one `sed -n '/METHOD name\./,/ENDMETHOD/p'` in Bash, which no run tried (inferred from the per-turn context,
not measured). A harder variant would not name the file, or would ask about a method reached through
an interface. **Reporting only: the adoption verdict is unchanged, drop.**

## Known issues

- **abapGit's libraries are pinned here, not by abapGit.** `abap_transpile` clones the libraries named in `test/abap_transpile.json` (`open-abap-core`, `open-abap-gui`, `open-abap-seo`, `express-icf-shim`, `abapGit-web-classic`) from their default branches on every build. On 2026-10-05 `open-abap-gui` changed (#188 to #195) and `npm run unit` started failing before any test ran (`Error: Void type: DISVARIANT` in `cl_alv_variant`), which made B2 unscorable. `abapgit-transpile-libs.json` now maps each library to its repository and its last commit before the pinned abapGit commit (2026-10-04T17:55Z). The harness clones each at that commit into `<work-dir>/libs/<name>` and rewrites `libs[]` in each checkout's `test/abap_transpile.json` from `url` to `folder`. The transpiler resolves `folder` as `path.join(cwd, folder)`, so an absolute path does not work; the harness writes it relative to the checkout. The libraries sit outside every checkout so agents' `grep` and sem's index never see them. The harness fails if the config names a library the pin file lacks, or the reverse. Re-pin on purpose only, and say so in the commit.
  With the pinned libraries `npm run unit` exits 0 on the pinned commit, 959 test methods in 169 local test classes, as on the morning of 2026-10-05.
- **abapGit has no `package-lock.json`.** abapGit gitignores it, so `npm ci` on the checkout as is fails with `EUSAGE`, and `npm run unit` then fails with `sh: 1: abap_transpile: not found` (exit 127). Fix used:
  1. `npm install --ignore-scripts` once in a scratch copy (12 s, 210 packages, every one resolved from registry.npmjs.org).
  2. That lockfile is pinned as `abapgit-package-lock.json`.
  3. The harness runs `npm ci --ignore-scripts` against it.

  Regenerate the lockfile only on purpose, and read its diff.
- **Earlier on 2026-10-05, `npm run unit` passed** at the pinned commit with that lockfile: exit 0, 35 s cold and 33 s warm, on Node v22.22.0 / npm 10.9.4. 959 test methods ran, in 169 local test classes of 111 objects; 74 were skipped by configuration or CRITICAL risk level. With a correct b2_01 solution plus its hidden tests, the suite ran 961 methods in 170 classes in 22 s and scored 1.0.
- **`npm run unit` no longer needs network once the checkout is prepared.** The libraries are cloned once, when `<work-dir>/base` is prepared. B2's prompt used to say the build needs network access; it now says how long the suite takes (B2 had not run, so no row is affected).
- **The transpiler only checks files in its `input_filter`.** That is 1038 of 1526 files. A caller in an excluded file that misses the new parameter does not break the build, so `callers_missed` is counted by grep over all of `src/` separately.
- **The usual abapGit clone is shallow** (1 commit). B3 needs history, so the base checkout fetches 300 commits from GitHub once when they are missing.
- **sem's ABAP support answers few B1 questions** in sem 0.27.0. This is a property of the build under test, not of the harness; it is what a baseline run measures. The dry run's tool check shows it. Probed through `sem mcp`:
  - `sem_find` with `mode: "callers"` lists only the defining class for every B1 target it knows, never the cross-file callers grep finds.
  - 4 of the 10 targets are not extracted at all: the interface methods `zif_abapgit_sap_package~list_subpackages` and `zif_abapgit_repo~get_files_local`, and the class methods `get_display_name` and `textarea`. For example, `sem_find` with `in: src/git/zcl_abapgit_git_branch_utils.clas.abap` lists 1 of that class's 3 methods.
- **`Bash` is not sandboxed.** Claude Code's Bash runs model-written commands as the current user in the run's checkout, with network and with the auth and proxy variables Claude Code itself needs. `--permission-mode dontAsk` and `--setting-sources ''` limit what is pre-approved, not what an allowed `Bash` call can do. Run real benchmarks in a container or VM.
- **The sem arm's index is cold in every run.** Each run is a fresh clone, so the first sem call pays for indexing; abapGit indexes in about 7 s. That cost is part of the sem arm's wall time, on purpose. Claude Code also starts `sem mcp` before the first request in every sem run, whether or not a sem tool is called. That startup is in `wall_time_s`.
- **`sem mcp` runs standalone** (`SEM_MCP_NO_SHARED=1`), with telemetry, update checks, cloud and network off. Each run has its own checkout, and the process ends with the run.
- **`files_read`** only sees the `Read` tool. An agent that reads with `Bash` or `Grep` looks like it read nothing, which is what both arms did in the baseline. `bytes_read` covers every tool.
- **`test_classes_executed`** parses `Bash` output. Claude Code truncates long Bash output, and the harness also reads the untruncated `tool_use_result` when the transcript carries it. An agent that pipes `npm run unit` through `tail` still hides most test classes, as before.
- **Scratch output is shared across runs.** Several Gate 2 B2 agents wrote scratch output to a literal `/tmp/out.txt`. The harness sets each run's working directory but not `TMPDIR`, and a hard-coded path ignores `TMPDIR` anyway, so a per-run scratch directory is more than a one-line change; it is left as is. Runs are sequential, so the file is overwritten rather than raced, but one run could read the previous run's output.
