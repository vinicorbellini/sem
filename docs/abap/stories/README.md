# Tier 1 stories: review-grade ABAP support

Nine stories, one per spec row. Each file has Intent, Acceptance criteria,
Files, Approach, Verification, Out of scope, Estimate and Depends on.

| Story | Title | Days | Depends on |
|-------|-------|------|------------|
| [1.1](1-1-case-folding.md) | Case-insensitive names | 3-4 | none |
| [1.2](1-2-comment-string-stripping.md) | ABAP comment and string stripping | 1-2 | none |
| [1.3](1-3-entity-set.md) | Full entity set | 3-4 | none (file-name parser from 1.5 for local classes) |
| [1.4](1-4-one-entity-per-class.md) | One entity per class | 1-2 | 1.3 |
| [1.5](1-5-abapgit-layout.md) | abapGit layout | 1-2 | none |
| [1.6](1-6-parse-error-tolerance.md) | Parse-error tolerance | 1-2 | 1.3 |
| [1.7](1-7-test-detection.md) | Test detection | 0.5 | none |
| [1.8](1-8-diff-quality.md) | Diff quality on abapGit PRs | 1-2 | 1.1 to 1.7, 1.9 |
| [1.9](1-9-tests-and-fixture.md) | Rust tests and fixture wiring | 1-2 | alongside all |

## Order

1. Start 1.1, 1.2, 1.5 and 1.7 first. They are independent. 1.1 and 1.2 both
   touch `graph.rs` and `languages.rs`, so rebase one on the other instead of
   editing in parallel.
2. Then 1.3, then 1.4, then 1.6. Each needs the entity shapes of the one before.
3. Run 1.9 alongside everything. Each story adds its Rust tests as it lands and
   flips its fixture tests from ignored to passing.
4. Run 1.8 last. It judges the combined result on real abapGit history.

Serial total: 12.5 to 20.5 days.

## Conventions

- Follow `CONTRIBUTING.md`. Language support lives in `languages.rs` and
  `entity_extractor.rs`. Tests go in
  `crates/sem-core/src/parser/plugins/code/mod.rs`. Every upstreamable code
  change adds a `CHANGELOG.md` entry under `## [Unreleased]`.
- Run cargo from `crates/`. The default features include `grammar-all`, which
  includes `lang-abap`.
- Fixture tests live under `crates/sem-core/tests/fixtures/abap/` and are built by someone else.
  `crates/sem-core/tests/fixtures/abap/README.md` maps each fixture file to the
  tests that read it and the story each belongs to. All of them are in
  `crates/sem-core/src/parser/plugins/code/mod.rs`, none is `#[ignore]`d, and
  `cd crates && cargo test -p sem-core abap` runs them all. Names in use:
  `test_abap_fixture_<object>` for the per-file entity lists (`intf`, `clas`,
  `clas_locals_def`, `clas_locals_imp`, `clas_testclasses`,
  `clas_testclasses_detection`, `clas_sub`, `prog`, `prog_include`,
  `fugr_function_module`, `fugr_main_program`, `fugr_top_include`,
  `fugr_form_include`, `fugr_pbo_include`, `layout`);
  `abap_fixture_<story>_<topic>` for the behaviour tests (`1_1_find_any_case`,
  `1_1_refs_any_case`, `1_2_comment_not_reference`, `1_2_string_not_reference`,
  `1_3_report`, `_form`, `_function`, `_module`, `_method`, `_class`,
  `_interface`, `_types_data`, `_macro`, `_local_classes_attach`,
  `1_6_errors_still_yield_entities`, `1_6_error_count_reported`); and
  `test_abap_entity_extraction` and `test_abap_class_two_ranges` for inline
  source. A story that adds a test names it in the same scheme and file.

## Gate 1

Tier 1 is done when all of these hold:

1. All nine stories are done and merged on the `abap` branch.
2. Every fixture test under `crates/sem-core/tests/fixtures/abap/` is un-ignored and passing.
   `cd crates && cargo test -p sem-core abap_fixture` shows zero ignored and
   zero failed.
3. The full suite passes: `cd crates && cargo test --workspace`.
4. The parse-error census on abapGit is recorded in `docs/abap/census-gate1.md`.
   It lists files parsed, files with error nodes, total error nodes, the
   entities-per-file distribution, and the ten worst files by error count.

# Tier 2 stories: call resolution

Nine stories, 2.0 to 2.8. They turn ABAP from "entities and name matches inside one
file" into a call graph a reviewer can trust: callers across files, receivers bound
to classes, dispatch through interfaces and base classes, includes, dynamic calls
reported and not dropped, tests reached through calls, and a measured precision.
Same format as Tier 1. The spec rows come from the plan; where the code differs
from them the story says so.

| Story | Title | Days | Depends on |
|-------|-------|------|------------|
| [2.0](2-0-cross-file-scope.md) | Cross-file candidate scope | 2-3 | Tier 1 |
| [2.1](2-1-static-calls.md) | Static call resolution | 8-10 | 2.0 |
| [2.2](2-2-type-binding.md) | Type binding (design decision) | 5-7 | 2.1 |
| [2.3](2-3-inheritance-interfaces.md) | Inheritance and interfaces (design decision, includes T2-B) | 4-5 | 2.1, 2.2 for call-edge tests |
| [2.4](2-4-includes-function-groups.md) | Includes and function groups | 3-4 | 2.0, skeleton of 2.1 |
| [2.5](2-5-unresolved-visible.md) | Unresolved calls stay visible | 1-2 | 2.0, reasons from 2.1 |
| [2.6](2-6-tests-reached.md) | Tests reached | 2-3 | 2.2, 2.3, 1.7 |
| [2.7](2-7-precision-study.md) | Precision check (includes T2-C) | 3-4 | 2.1 to 2.5 |
| [2.8](2-8-mcp.md) | MCP | 1 | 2.5, 2.6, 1.1 |

Serial total: 29 to 39 days. Tier 1's stories ran far faster than their estimates
when done as parallel agents, so read every figure above as a ceiling.

## Tier 2 order

1. Story 2.0 first and alone. Everything depends on it: the other stories assume
   a name can reach another file.
2. Then 2.1, 2.5 and 2.4 in parallel. They all start from the `calls/abap.rs`
   skeleton and the `Layout` shape that 2.1 settles in its first-day spike.
   So 2.4 and 2.5 begin with fixtures, tests and the parts that do not touch it,
   such as the `completeness.rs` half of 2.5. They rebase onto 2.1 before
   merging. 2.1 and 2.4 both edit `abap.rs` and `Layout`, so rebase one on the other.
3. Then 2.2 and 2.3. Two decisions need a human look before code is written:
   how receivers bind to classes (2.2) and how interface and inheritance dispatch
   become edges (2.3). Both stories state their options and a recommendation.
   2.3's entity half can start with 2.2, and its call-edge tests wait for it.
4. Then 2.6, 2.7 and 2.8. 2.7 is the measurement and goes last of the three in
   practice, since it judges the combined result on real code.

Fixtures. Each story adds its own files under `crates/sem-core/tests/fixtures/abap/`
and a row for each in that folder's `README.md`: 2.0 `zcl_fx_user` and
`zcl_fx_other`, 2.1 `zcl_fx_calls`, 2.2 `zcl_fx_types`, 2.3 `zif_fx_audit`,
`zcl_fx_order_alt` and `zcl_fx_dispatch`, 2.4 `zfx_report2` and `zfx_other`, 2.5
`zfx_dynamic`, 2.6 two `testclasses` files, 2.7 `zcl_fx_chain` and the
`precision/` folder. Do not edit the Tier 1 fixture files: the `abap_fixture_1_*`
tests assert their line ranges. `test_abap_fixture_layout` applies to every new
file (an `.xml` envelope for each `clas`, `intf` and `prog`, under 60 lines).
New tests are named `abap_fixture_2_<n>_<topic>` in
`crates/sem-core/src/parser/plugins/code/mod.rs`, with the `abap_graph` and
`abap_fixture_dependencies` helpers.

## Conventions that differ from Tier 1

- The call pipeline is not configured on `LanguageConfig`. ABAP is added to
  `LANGUAGES` in `crates/sem-core/src/parser/calls/mod.rs`, with a `Lang` in the new
  `calls/abap.rs`, and keeps `scope_resolve: None` like Python, Go and Rust.
- Until story 2.2 flips `replaces_bow()`, the bag-of-words resolver still runs for
  ABAP beside the pipeline (stories 2.0 and 2.1).
- Every story keeps non-ABAP `sem graph --json` byte-identical to the base commit.
- Run cargo from `crates/`, as in Tier 1.

## Gate 2

Tier 2 is done when all of these hold:

1. All nine stories are done and merged on the `abap` branch.
2. The precision check of spec 2.7 passes: pooled over the 30 methods, resolved
   callers have at least 90% precision and 80% recall against the where-used truth.
   The result is in `docs/abap/census-gate2.md`.
3. Every Tier 2 fixture test is green and none is ignored:
   `cd crates && cargo test -p sem-core abap_fixture_2` shows zero ignored and
   zero failed, and `cargo test -p sem-core abap` still passes with Tier 1's.
4. The full suite passes: `cd crates && cargo test --workspace`.
5. The baseline benchmark B.1 is re-run. That is the B1 where-used class of
   `bench/abap-agent`, run with `--checkpoint gate2` against the same pinned
   abapGit commit, with the grep and sem arms side by side
   (`python3 bench/abap-agent/run.py --checkpoint gate2 --class B1`). Compare it with
   the `baseline` and `gate1` rows. `bench/` is git-ignored, so commit the results
   with `git add -f bench/abap-agent/results.csv bench/abap-agent/results.jsonl`.
