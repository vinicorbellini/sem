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
