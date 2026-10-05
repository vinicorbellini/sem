# ABAP Gate 1 census: diff quality on abapGit pull requests

Story 1.8, the hand-judged part of Gate 1: does `sem diff` name the methods a
reviewer would name, on real abapGit history? The answer on this run is
**no: Gate 1 fails**, 4 of 10 hand-checked PRs match or partial against a
threshold of 8, and six of them have a file where a method changed and sem
named no entity. The cause is one and the same in almost every case, and it is
below the diff: about a third of abapGit's `METHOD` blocks never become a
method entity.

| | |
|---|---|
| Date | 2026-10-05 |
| sem | `abap` branch at `861b66d` (stories 1.1 to 1.7 merged), `cargo build --release -p sem-cli` |
| Grammar | `tree-sitter-abap-sqry` 32.0.1 (mkoval1/tree-sitter-abap) |
| Corpus | abapGit, `main` at `b2b4e25` (2026-10-04, PR #7928) |
| Story 1.9 | running in parallel, not merged; 1.8 was meant to run after it |

## The clone

`git -C /tmp/claude-0/abapGit fetch --unshallow` was accepted, so the history
is complete: 6721 commits on `main` (the fetch also brought abapGit's tags).
The clone was not otherwise changed; every file version below was read with
`git show <sha>:<path>`, never checked out.

| Measure | Count |
|---|---:|
| Commits on `main` | 6721 |
| Subjects ending in `(#NNNN)` (`git log --oneline \| grep -c '(#[0-9]*)$'`) | 3538 |
| ... that touch `.abap` under `src/`, the story's pathspec `'src/**/*.abap'` | 2734 |
| ... the same with `':(glob)src/**/*.abap'` | 2968 |
| ... and first-parent, not a merge commit (the candidate set) | 2966 |

Two adaptations of the story's commands, both decided before any diff was run:

- The story's pathspec `'src/**/*.abap'` is a plain git pathspec, where `**`
  is `*` and the pattern needs a `/` after `src/`. It drops the `.abap` files
  directly under `src/` (`zcl_abapgit_factory.clas.abap`, `zabapgit.prog.abap`
  and others). The rule says "under `src/`", so the candidate list uses the
  `:(glob)` form, which matches zero or more directories.
- The story says `git log --merges` is empty. It is not: the deep history has
  642 merge commits (abapGit merged PRs with merge commits before it switched
  to squash merges). The rule asks for squash-merge commits, so the candidate
  list is `git log --first-parent --no-merges`. Against the 2968 that drops
  two commits reachable only through merged side branches (`caf45545` #1032,
  `e73b7973` #42), and `--no-merges` keeps out one merge commit with a
  `(#NNNN)` subject that `--first-parent` alone would list.

## The 20 PRs

Chosen before any diff was read. The 2966 candidates have 2966 distinct PR
numbers (#273 to #7929). Sorted by PR number, `k = 2966 div 20 = 148`, and the
pick is index `0, k, 2k, ..., 19k` (0-based). The hand-checked ten are every
second one: list positions 2, 4, ..., 20.

| # | PR | Commit | Hand-checked | Date | `.abap` files | sem changes |
|---:|---|---|---|---|---:|---:|
| 1 | #273 | `7618cada582d` | | 2016-07-02 | 2 | 468 |
| 2 | #1928 | `529bfc478e0d` | yes | 2018-09-21 | 2 | 6 |
| 3 | #2749 | `a69e313090a3` | | 2019-06-21 | 1 | 1 |
| 4 | #3185 | `0c3cdf639e12` | yes | 2020-01-30 | 1 | 6 |
| 5 | #3561 | `051c9c6569c0` | | 2020-06-29 | 6 | 50 |
| 6 | #3891 | `6f8147036e36` | yes | 2020-09-16 | 9 | 34 |
| 7 | #4172 | `16e8f3d02d36` | | 2020-11-14 | 1 | 29 |
| 8 | #4432 | `978b7a4a57b7` | yes | 2021-01-21 | 8 | 18 |
| 9 | #4751 | `50bd3941e0f6` | | 2021-05-12 | 3 | 71 |
| 10 | #5072 | `a3e598ac1f06` | yes | 2021-10-29 | 1 | 1 |
| 11 | #5430 | `014d40db1cd6` | | 2022-04-01 | 5 | 9 |
| 12 | #5711 | `284f056f606e` | yes | 2022-08-05 | 2 | 2 |
| 13 | #5977 | `5a0a0f352ac9` | | 2023-01-09 | 2 | 3 |
| 14 | #6217 | `2b16dce9a30f` | yes | 2023-04-15 | 1 | 1 |
| 15 | #6469 | `c2254f9f8fb1` | | 2023-09-04 | 1 | 1 |
| 16 | #6669 | `ef9f640fea24` | yes | 2023-11-28 | 2 | 2 |
| 17 | #6935 | `11a7e58237e9` | | 2024-05-23 | 1 | 2 |
| 18 | #7180 | `a0e7a47f7af2` | yes | 2025-03-31 | 8 | 21 |
| 19 | #7416 | `b0c92a7768fa` | | 2025-10-09 | 3 | 7 |
| 20 | #7644 | `10ccc60e5baf` | yes | 2026-03-27 | 1 | 1 |

"sem changes" is `summary.total` of the JSON output.

## Running sem

```bash
cd /tmp/claude-0/abapGit
SEM_CACHE_DIR=<scratch> <worktree>/crates/target/release/sem diff <sha>^ <sha> --json
```

All 20 exit 0 with nothing on stderr, no crash on any file, including #273
(the abapmerge split, 468 changes). The acceptance criterion on running holds.

## Hand-check protocol

For each of the ten, before opening sem's output: `git show --stat` and
`git show -U0` of the `.abap` files, read with a hunk header that names the
nearest `METHOD`, `ENDMETHOD`, `CLASS` or section line (a diff driver set by
`-c core.attributesFile` and `-c diff.abap.xfuncname`, nothing written to the
clone), and every hunk whose header could be stale checked against the file.
The reviewer's methods were written to a notes file first, then sem was run.

What a reviewer names: every method whose body or signature changed, added or
deleted. A method that only moved, unchanged, is not a change (#3185's
`apply_order_by`). Non-method changes (DATA, TYPES, CONSTANTS, the
`CLASS X IMPLEMENTATION` name changing case) are out of scope for the rating,
as the story says, and listed only where they explain sem's output.

Scoring, per file, then per PR:

- **match**: every changed method in the file is named, and no unchanged
  method is named as changed.
- **partial** (the story's definition): sem named the class but not the
  method, or named the method plus unrelated noise. A file where some changed
  methods are named and others are not is a partial.
- **miss** (the story's definition): sem named no entity in a file whose
  method changed. A `module-level` orphan is not an entity.
- A **PR's verdict is its worst file's.** A PR is a match only if every file
  is; one missed file makes the PR a miss. The alternative roll-up is given
  under the result.

## The ten

"sem" lists entities as `changeType entity`, `orphan` for `module-level`
orphans. Methods sem got right are not repeated when a file is a match.

| PR | Reviewer's methods | sem's entities | Verdict | Cause |
|---|---|---|---|---|
| #1928 add handling of deleted branches in branchoverview | `zcl_abapgit_gui_page_boverview`: `body`. `zcl_abapgit_branch_overview`: `constructor`, `determine_merges`, `_reverse_sort_order` (new) | boverview: orphan only. branch_overview: modified class, `constructor`, `determine_merges`; added `_reverse_sort_order`; orphan | **miss** | boverview yields no method entity at all (3 entities, 81 error nodes): the grammar's recovery loses the whole IMPLEMENTATION, so `body`'s edit is an orphan. branch_overview is a match. |
| #3185 remove DEFINED from zcl_abapgit_gui_page_repo_over | `render_table_header` (macro calls replaced), `_add_col` (new) | modified class; added variable `mt_col_spec`, deleted variable `mv_time_zone`; added method `apply_order_by`; modified method `parse_filter`; deleted macro `_add_col` | **partial** | Class named, neither method named, and two wrong method claims. `parse_filter`'s entity runs from its `METHOD` to the line before `ENDCLASS` (L210-468) and absorbs every method after it, so the edits to `render_table_header` and `_add_col` are reported as `parse_filter`. In the old file it also absorbed `apply_order_by`, so that unchanged move reads as an addition. The chained `DATA:` is one variable entity named after a member, so adding a member renamed it (delete + add). This is a partial by the letter of the definition only; as review output it is wrong. |
| #3891 XML Refactoring: Various Objects | 30 methods over 9 files (`io_xml` renamed `ii_xml`): clas 11, doma 4, dsys 1 (`deserialize_dsys`), dtel 1, intf 3, msag 4, prog 3, tabl 1, objects_super 2 | clas, intf, prog, objects_super: class + every method (19). msag: class + 3 of 4, orphan. doma: class, orphan. tabl: class. dsys: class, `constructor`. dtel: orphan only | **miss** | 23 of 30 named. dtel has no method entity (3 entities); doma and tabl implementations yield none (class and types only); msag's method entities stop before `zif_abapgit_object~serialize`. dsys's `constructor` entity (L44-99) runs over `deserialize_dsys` (L68), so that edit is named `constructor`. Files: 4 match, 4 partial, 1 miss. |
| #4432 Terminology: Inclusive Language - Part 4 | performance_test: `constructor`, `run_measurement`. addofflin, addonline: `get_form_schema`. sett_repo: `get_form_schema`, `read_settings`. popups: `zif_abapgit_popups~popup_perf_test_parameters`. services_basis: `run_performance_test`. `zif_abapgit_popups`: `popup_perf_test_parameters` (signature). language: none | performance_test, addofflin, addonline, services_basis: class + the methods (plus variable `mt_result` for an `mv_` rename in a chained DATA). sett_repo: class, `get_form_schema`, 2 orphans. popups: orphan only. `zif_abapgit_popups`: modified interface **`NO_TEXT`**. language: class, orphan | **miss** | 6 of 9 named. `zcl_abapgit_popups` has 0 entities (165 error nodes), so its edit is an orphan. sett_repo's method entities stop before `read_settings`. The interface is named `NO_TEXT`: a pragma after its first statement put the real name in an ERROR and the grammar's `name` field on `##NO_TEXT` (a bug, fixed, see below); and interface method declarations are not entities, so the signature change can only be the interface. |
| #5072 FUGR/PROG: Keep field text in case of masking | `zcl_abapgit_objects_program`: `serialize_dynpros` | orphan only | **miss** | Method entities stop at `serialize_cua` (L708-737); `serialize_dynpros` and every method after it are lost to error recovery (137 error nodes). |
| #5711 downport assert_true() | intf testclasses: `ltcl_serialize` `serialize_default`, `serialize_non_default`. utils testclasses: `ltcl_is_binary` `then_is_not_binary`, `then_is_binary` | orphan only, in both files | **miss** | Neither file yields these methods: intf testclasses has no `ltcl_serialize` entity or its methods; utils testclasses lists `ltcl_is_binary`'s methods only up to `given_cds_metadata`. |
| #6217 One more SHI3 language deserialization fix | `zcl_abapgit_object_shi3`: `zif_abapgit_object~deserialize` | modified method `zif_abapgit_object~deserialize` | **match** | |
| #6669 Downport: Fix 702 syntax errors | ssfo: `sort_texts`. ueno: `serialize_docu_xxxx` | ssfo: orphan only. ueno: modified `serialize_docu_xxxx` | **miss** | ssfo's method entities stop after `handle_attrib_leading_spaces`; `sort_texts` is lost. ueno is a match. |
| #7180 Refactor: Decouple code inspection from factory | code_inspector: `get_code_inspector`, `set_code_inspector` (new). factory: `get_code_inspector` (deleted). injector: `set_code_inspector` (deleted). repo_online: `zif_abapgit_repo_online~push`. gui_page_code_insp: `run_code_inspector`. gui_page_syntax: `run_syntax_check`. gui_page_sett_locl: `validate_form`. popup_code_insp: `fetch_list` | all nine, with the right change types; plus the moved `ty_code_inspector_pack(s)` types and `gt_code_inspector` as added in code_inspector and deleted in factory, and the classes whose definitions changed | **match** | |
| #7644 TRAN: Handle empty transaction attributes | `zcl_abapgit_object_tran`: `transaction_read`, `zif_abapgit_object~serialize` | modified method `transaction_read` | **partial** | `transaction_read`'s entity runs L608-1006, to the line before `ENDCLASS`, over every method after it, so `serialize`'s edit is folded into it and only one method is named. |

Method-level, over the ten: a reviewer names 64 methods; sem names 44 of them
(69%), and names a method that did not change three times (`parse_filter`,
`apply_order_by`, `constructor` in dsys).

## Result

| Verdict | PRs |
|---|---|
| match | 2 (#6217, #7180) |
| partial | 2 (#3185, #7644) |
| miss | 6 (#1928, #3891, #4432, #5072, #5711, #6669) |

**Gate 1 diff quality: FAIL.** 4 of 10 are match or partial; the threshold is
8. All six misses are of the excluded kind, a file with a changed method where
sem named nothing.

The roll-up does not decide it. Rated per PR on its best file instead of its
worst, #1928, #3891, #4432 and #6669 become partials and the count is 8 of 10,
but #5072 and #5711 still have sem naming nothing for a changed method, and so
do files in the other four, so the gate fails under either reading. Grading
#3185 as a miss instead of a partial (it names no right method) only lowers the
count.

### Why: methods the parse loses

Every miss and both partials come from one gap.
mkoval1/tree-sitter-abap's error recovery, on a statement it cannot parse,
either drops the rest of a `CLASS ... IMPLEMENTATION` (no method entities from
that point on), or lets a `method_implementation` run past its own `ENDMETHOD`
to the end of the class (one entity covering many methods, named after the
first). Over all of abapGit at `b2b4e25`, with the same binary:

| Measure | Count |
|---|---:|
| Files with `METHOD` blocks | 624 |
| `METHOD x.` blocks in them (text count) | 7577 |
| ... with a method entity of that name | 5161 (68%) |
| Files missing at least one | 317 |
| Method entities that span another `METHOD` line | 94 |

The story 1.6 fallback pass recovers FORM, MODULE, DEFINE, PROGRAM and TYPES
from the token stream, but not METHOD, because the grammar has a node for it.
The node is what goes missing. Until methods are recovered the same way, a
diff on roughly half of abapGit's files with methods can miss a method a reviewer
would name, and this check cannot pass; nothing in the differ is at fault.

## Parse-error counts (story 1.6)

`sem find --in <files> --parse-report --json --file-exts .abap` on each
hand-checked PR's changed `.abap` files, before (`<sha>^`) and after (`<sha>`),
each version written to a scratch tree with its repository path. Columns:
entities / from the grammar / from the fallback / error nodes.

| PR | File | Before | After |
|---|---|---|---|
| #1928 | `src/ui/zcl_abapgit_gui_page_boverview.clas.abap` | 3 / 2 / 1 / 63 | 3 / 2 / 1 / 81 |
| #1928 | `src/zcl_abapgit_branch_overview.clas.abap` | 12 / 9 / 3 / 116 | 13 / 10 / 3 / 126 |
| #3185 | `src/ui/zcl_abapgit_gui_page_repo_over.clas.abap` | 9 / 6 / 3 / 58 | 9 / 7 / 2 / 64 |
| #3891 | `src/objects/zcl_abapgit_object_clas.clas.abap` | 27 / 27 / 0 / 188 | 27 / 27 / 0 / 188 |
| #3891 | `src/objects/zcl_abapgit_object_doma.clas.abap` | 5 / 1 / 4 / 15 | 5 / 1 / 4 / 15 |
| #3891 | `src/objects/zcl_abapgit_object_dsys.clas.abap` | 15 / 14 / 1 / 50 | 15 / 14 / 1 / 50 |
| #3891 | `src/objects/zcl_abapgit_object_dtel.clas.abap` | 3 / 1 / 2 / 10 | 3 / 1 / 2 / 10 |
| #3891 | `src/objects/zcl_abapgit_object_intf.clas.abap` | 18 / 18 / 0 / 95 | 18 / 18 / 0 / 95 |
| #3891 | `src/objects/zcl_abapgit_object_msag.clas.abap` | 10 / 7 / 3 / 95 | 10 / 7 / 3 / 95 |
| #3891 | `src/objects/zcl_abapgit_object_prog.clas.abap` | 17 / 15 / 2 / 61 | 17 / 15 / 2 / 61 |
| #3891 | `src/objects/zcl_abapgit_object_tabl.clas.abap` | 8 / 1 / 7 / 52 | 8 / 1 / 7 / 52 |
| #3891 | `src/objects/zcl_abapgit_objects_super.clas.abap` | 17 / 17 / 0 / 64 | 17 / 17 / 0 / 64 |
| #4432 | `src/test/zcl_abapgit_performance_test.clas.abap` | 12 / 10 / 2 / 21 | 12 / 10 / 2 / 21 |
| #4432 | `src/ui/zcl_abapgit_gui_page_addofflin.clas.abap` | 11 / 11 / 0 / 72 | 11 / 11 / 0 / 72 |
| #4432 | `src/ui/zcl_abapgit_gui_page_addonline.clas.abap` | 9 / 9 / 0 / 87 | 9 / 9 / 0 / 87 |
| #4432 | `src/ui/zcl_abapgit_gui_page_sett_repo.clas.abap` | 10 / 10 / 0 / 51 | 10 / 10 / 0 / 51 |
| #4432 | `src/ui/zcl_abapgit_popups.clas.abap` | 0 / 0 / 0 / 165 | 0 / 0 / 0 / 165 |
| #4432 | `src/ui/zcl_abapgit_services_basis.clas.abap` | 6 / 6 / 0 / 51 | 6 / 6 / 0 / 51 |
| #4432 | `src/ui/zif_abapgit_popups.intf.abap` | 1 / 1 / 0 / 29 | 1 / 1 / 0 / 29 |
| #4432 | `src/utils/zcl_abapgit_language.clas.abap` | 5 / 5 / 0 / 1 | 5 / 5 / 0 / 1 |
| #5072 | `src/objects/zcl_abapgit_objects_program.clas.abap` | 21 / 16 / 5 / 137 | 21 / 16 / 5 / 137 |
| #5711 | `src/objects/zcl_abapgit_object_intf.clas.testclasses.abap` | 29 / 29 / 0 / 55 | 29 / 29 / 0 / 55 |
| #5711 | `src/utils/zcl_abapgit_utils.clas.testclasses.abap` | 19 / 19 / 0 / 103 | 19 / 19 / 0 / 103 |
| #6217 | `src/objects/zcl_abapgit_object_shi3.clas.abap` | 21 / 21 / 0 / 83 | 21 / 21 / 0 / 87 |
| #6669 | `src/objects/zcl_abapgit_object_ssfo.clas.abap` | 7 / 6 / 1 / 87 | 7 / 6 / 1 / 87 |
| #6669 | `src/objects/zcl_abapgit_object_ueno.clas.abap` | 34 / 32 / 2 / 118 | 34 / 32 / 2 / 118 |
| #7180 | `src/inspect/zcl_abapgit_code_inspector.clas.abap` | 20 / 19 / 1 / 86 | 25 / 22 / 3 / 95 |
| #7180 | `src/repo/zcl_abapgit_repo_online.clas.abap` | 19 / 19 / 0 / 91 | 19 / 19 / 0 / 91 |
| #7180 | `src/ui/pages/codi/zcl_abapgit_gui_page_code_insp.clas.abap` | 16 / 16 / 0 / 69 | 16 / 16 / 0 / 69 |
| #7180 | `src/ui/pages/codi/zcl_abapgit_gui_page_syntax.clas.abap` | 9 / 9 / 0 / 25 | 9 / 9 / 0 / 25 |
| #7180 | `src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap` | 21 / 21 / 0 / 185 | 21 / 21 / 0 / 185 |
| #7180 | `src/ui/popups/zcl_abapgit_popup_code_insp.clas.abap` | 5 / 5 / 0 / 13 | 5 / 5 / 0 / 13 |
| #7180 | `src/zcl_abapgit_factory.clas.abap` | 29 / 25 / 4 / 37 | 25 / 23 / 2 / 32 |
| #7180 | `src/zcl_abapgit_injector.clas.abap` | 13 / 13 / 0 / 10 | 12 / 12 / 0 / 6 |
| #7644 | `src/objects/zcl_abapgit_object_tran.clas.abap` | 16 / 14 / 2 / 92 | 16 / 14 / 2 / 92 |

Every changed file has error nodes. The two PRs that match are not the ones
with the fewest errors (#7180's sett_locl has 185); what decides it is whether
the recovery happens to drop or stretch the method that changed.

### Whole-corpus census at `b2b4e25`

Gate 1 item 4 asks for the census here too. Same command and corpus as
`census-baseline.md`, on the `861b66d` binary:

| Measure | Baseline (`ad282ca` + 1.6) | Gate 1 (`861b66d`) |
|---|---:|---:|
| Files parsed | 752 | 752 |
| Files with error nodes | 731 | 731 |
| Files with no error nodes | 21 | 21 |
| Error nodes | 39314 | 39314 |
| Entities | 8389 | 7776 |
| Entities from the grammar | 7999 | 7386 |
| Entities from the fallback pass | 390 | 390 |
| Files with at least one fallback entity | 167 | 167 |
| Files with no entities | 5 | 5 |

The error counts are unchanged (no story touched the grammar). Entities drop
by 613 because story 1.4 folds each `CLASS x IMPLEMENTATION` into its
definition, one entity where there were two. Per file, the median is 7
entities and 31 error nodes; the largest is 149 entities and 1764 error nodes.

| Entities | Files |
|---|---:|
| 0 | 5 |
| 1-5 | 320 |
| 6-20 | 325 |
| 21-100 | 101 |
| over 100 | 1 |

| Error nodes | Files |
|---|---:|
| 0 | 21 |
| 1-10 | 171 |
| 11-50 | 301 |
| 51-100 | 158 |
| over 100 | 101 |

| Error nodes | Entities | Grammar | Fallback | File |
|---:|---:|---:|---:|---|
| 1764 | 87 | 74 | 13 | `src/json/zcl_abapgit_ajson.clas.testclasses.abap` |
| 627 | 11 | 11 | 0 | `src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap` |
| 510 | 18 | 15 | 3 | `src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap` |
| 485 | 21 | 14 | 7 | `src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap` |
| 399 | 14 | 12 | 2 | `src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap` |
| 380 | 32 | 32 | 0 | `src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap` |
| 325 | 22 | 22 | 0 | `src/ui/routing/zcl_abapgit_services_repo.clas.abap` |
| 323 | 16 | 16 | 0 | `src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap` |
| 320 | 23 | 21 | 2 | `src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap` |
| 309 | 24 | 24 | 0 | `src/ui/routing/zcl_abapgit_gui_router.clas.abap` |

Files with no entities: `zif_abapgit_aff_prog_v1.intf.abap` (1 error node),
`zabapgit_parallel.fugr.lzabapgit_paralleltop.abap` (2),
`zabapgit_parallel.fugr.saplzabapgit_parallel.abap` (0),
`zcl_abapgit_object_fdt0.clas.abap` (5), `zcl_abapgit_persistence_db.clas.abap` (3).

## Every miss and partial, and where it went

| PR | File | What sem got wrong | Goes to |
|---|---|---|---|
| #1928 | gui_page_boverview | no method entities in the file | Tier 2 candidate T2-A |
| #3185 | gui_page_repo_over | `parse_filter` spans the methods after it; move read as add | T2-A |
| #3185 | gui_page_repo_over | chained `DATA:` renamed by adding a member | T2-C (non-method, not rated) |
| #3891 | dtel, doma, tabl, msag | methods lost to recovery | T2-A |
| #3891 | dsys | `constructor` spans `deserialize_dsys` | T2-A |
| #4432 | popups | 0 entities in the file | T2-A |
| #4432 | sett_repo | `read_settings` lost | T2-A |
| #4432 | `zif_abapgit_popups` | interface named `NO_TEXT` | **bug, fixed** (second commit) |
| #4432 | `zif_abapgit_popups` | interface method declarations are not entities | T2-B |
| #5072 | objects_program | methods lost after `serialize_cua` | T2-A |
| #5711 | both testclasses | test methods lost | T2-A |
| #6669 | ssfo | `sort_texts` lost | T2-A |
| #7644 | object_tran | `transaction_read` spans the methods after it | T2-A |

The Tier 2 candidates are written up in `docs/abap/stories/tier-2-candidates.md`.

**The bug.** `INTERFACE zif_x PUBLIC.` followed by a statement the grammar
cannot parse (here a `##NO_TEXT` pragma) puts the interface's own name inside
an ERROR node and gives the grammar's `name` field to a later token. 48 of the
113 global interfaces in abapGit were named after a later token (`NO_TEXT`,
`c_transport_status`, `ty_get`, `abap_bool` ...), which in a diff turns an edit
into a delete and an add whenever that token changes. Fixed by taking the first
`name` token of the `interface_declaration`; regression test
`abap_1_8_interface_name_is_not_a_later_token` in
`crates/sem-core/src/parser/plugins/code/mod.rs`. After it all 113 interfaces
carry their own name. It changes no verdict above: #4432 is still a miss
through `zcl_abapgit_popups`, and `zif_abapgit_popups` still a partial because
its methods are not entities. Global classes were checked the same way and
are all named correctly (462 of 462).

The census table and verdicts above are from `861b66d`, without the fix.

## After story 1.10

| | |
|---|---|
| Date | 2026-10-05 |
| sem | `story/1-10` at `39622cb` (on `abap` at `43d022c`), `cargo build --release -p sem-cli` |
| Grammar, corpus | unchanged: `tree-sitter-abap-sqry` 32.0.1, abapGit `main` at `b2b4e25` |

Story 1.10 is candidate T2-A, promoted: `METHOD x.` ... `ENDMETHOD.` blocks are
read off the fallback pass's statements and reconciled with the grammar's
methods (`docs/abap/README.md`, fact 10). Same commands as above, with a
scratch `SEM_CACHE_DIR`; the clone was only read.

### Whole-corpus METHOD census

How the method counts above are taken, and were taken again here: every
`src/**/*.abap` file at `b2b4e25`, the lines matching
`^\s*METHOD\s+([\w~/]+)\s*\.` (case-insensitive) are the `METHOD x.` blocks.
The entities are `sem find --in src --json --file-exts .abap
--no-default-excludes`. A block counts as found when its file has a `method`
entity of that name (case-folded); an entity spans another method when a later
`METHOD x.` line falls inside its line range. "Before" is the `abap` branch at
`43d022c`, which gives exactly the numbers of the `861b66d` table above.

| Measure | Before (`43d022c`) | After (`39622cb`) |
|---|---:|---:|
| Files with `METHOD` blocks | 624 | 624 |
| `METHOD x.` blocks in them (text count) | 7577 | 7577 |
| ... with a method entity of that name | 5161 (68%) | 7577 (100%) |
| Files missing at least one | 317 | 0 |
| Method entities that span another `METHOD` line | 94 | 0 |
| Method entities in all | 5705 | 7582 |

The five method entities over 7577 are real: `METHOD constructor
##ADT_SUPPRESS_GENERATION.` in five exception classes, which the text count's
pattern does not match. Forms, types, variables, interfaces, function modules
and the report are the same entities, with the same ranges, as before.

Why the leaves were not enough: the grammar does not end a `'...'` literal at
its line, so after a quote it misreads (`''`, a template like `|{ a }*|`) one
`character_literal` leaf runs over many lines. 943 `METHOD` lines and 942
`ENDMETHOD` lines in abapGit sit inside such a leaf, out of reach of a
token stream cut from the leaves. The statements are now cut from the source
with comments and literals blanked by the reference scan's stripper.

How the 7582 came about: 5105 are the grammar's own (5705 less 600 dropped)
and 2477 the fallback's. The 600 dropped are grammar methods that matched no
block, all of them misnamed: 581 after the last part of an interface method's
name (`get_deserialize_order` for `zif_abapgit_object~get_deserialize_order`),
19 after a pragma (`ADT_SUPPRESS_GENERATION`) or text running on past the
period. The block's own entity, with the right name, replaces each. The 5105
the grammar named right are all kept.
The grammar had lost 50 classes whole (local test classes mostly, and
`zcl_abapgit_object_fdt0`, `zcl_abapgit_persistence_db`, `zcl_abapgit_popups`
in its #4432 version); each is now one class from its definition and its
implementation.

### Parse-error census

`sem find --in src --parse-report --json --file-exts .abap`:

| Measure | Gate 1 (`861b66d`) | After 1.10 (`39622cb`) |
|---|---:|---:|
| Files parsed | 752 | 752 |
| Files with error nodes | 731 | 731 |
| Error nodes | 39314 | 39314 |
| Entities | 7776 | 9703 |
| Entities from the grammar | 7386 | 6786 |
| Entities from the fallback pass | 390 | 2917 |
| Files with at least one fallback entity | 167 | 372 |
| Files with no entities | 5 | 3 |
| Entities per file, median / largest | 7 / 149 | 9 / 194 |

The three files with no entities are the ones with nothing to find:
`zif_abapgit_aff_prog_v1.intf.abap` and the two `zabapgit_parallel` function
group includes.

### The eight PRs again

`sem diff <sha>^ <sha> --json` on the eight PRs rated miss or partial, scored
against the reviewer's methods recorded in "The ten" above (not re-derived).
#6217 and #7180 were run too and are unchanged, both still a match.

| PR | Reviewer's methods | sem's entities now | Was | Now |
|---|---|---|---|---|
| #1928 | boverview: `body`. branch_overview: `constructor`, `determine_merges`, `_reverse_sort_order` (new) | boverview: modified `body`. branch_overview: modified class, `constructor`, `determine_merges`; added `_reverse_sort_order` | miss | **match** |
| #3185 | `render_table_header`, `_add_col` (new) | modified class, `render_table_header`; added method `_add_col`; deleted macro `_add_col`; reordered `apply_order_by`; added variable `mt_col_spec`, deleted `mv_time_zone` | partial | **match** |
| #3891 | 30 methods over 9 files | all 30, each modified, and the class in the 7 files whose definition changed; nothing else | miss | **match** |
| #4432 | 9 methods over 7 files, one of them the interface `zif_abapgit_popups`'s `popup_perf_test_parameters` signature | the 8 class methods (popups: modified class and `zif_abapgit_popups~popup_perf_test_parameters`); `zif_abapgit_popups`: modified interface; variable `mt_result`; language: class, orphan | miss | **partial** |
| #5072 | `serialize_dynpros` | modified `serialize_dynpros` | miss | **match** |
| #5711 | `serialize_default`, `serialize_non_default`, `then_is_not_binary`, `then_is_binary` | the four, modified | miss | **match** |
| #6669 | ssfo: `sort_texts`. ueno: `serialize_docu_xxxx` | modified `sort_texts`, `serialize_docu_xxxx` | miss | **match** |
| #7644 | `transaction_read`, `zif_abapgit_object~serialize` | modified `transaction_read`, `zif_abapgit_object~serialize` | partial | **match** |

Notes on the calls:

- #3185: `apply_order_by` moved unchanged and is reported as *reordered*, not
  modified, which the protocol accepts ("a method that only moved, unchanged,
  is not a change"; reporting the move is not naming it as changed). The
  variable pair is the chained-`DATA:` gap T2-C, and the deleted macro is the
  `DEFINE _add_col` the PR removed, both non-method and not rated.
- #4432: the one file short of a match is `zif_abapgit_popups.intf.abap`,
  where sem names the interface and not the method, because interface method
  declarations are not entities (T2-B): a partial by the story's definition.
  Every other file matches. `zcl_abapgit_popups` also renamed its
  `CLASS ... IMPLEMENTATION` from upper to lower case; with the class's
  definition recovered too, its id comes from the definition and the method
  is matched across the two versions. (Recovering the implementation alone
  named the class after the implementation's spelling, and the case change
  then hid the method's edit.)
- Method-level, over the ten: of the 64 methods a reviewer names, sem names
  63 (was 44); the one left is #4432's interface method. It names no method
  that did not change (was three).

### Result after story 1.10

| Verdict | PRs |
|---|---|
| match | 9 (#1928, #3185, #3891, #5072, #5711, #6217, #6669, #7180, #7644) |
| partial | 1 (#4432) |
| miss | 0 |

**Gate 1 diff quality: PASS.** 10 of 10 are match or partial against a
threshold of 8, and no PR has a file with a changed method where sem named
nothing. Rolled up on each PR's best file instead of its worst, all ten are
matches; the verdict is the same under either reading.

What still falls short, none of it a miss:

- Interface method declarations are not entities (T2-B), so a changed
  signature in an `INTERFACE` reads as the interface changing (#4432).
- A chained `DATA:` is one variable entity named after one member (T2-C),
  noise in #3185 and #4432.
- The 50 classes the grammar lost whole are recovered as a class and their
  methods, but the `TYPES` and `DATA` in their definitions are still not
  entities: the fallback reads a class's `TYPES` only under a class the grammar
  found.
