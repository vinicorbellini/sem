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
  They are `#[ignore]` until the story that serves them lands. The ignored tests
  that exist today, in `crates/sem-core/src/parser/plugins/code/mod.rs`, and the
  story each one waits on:
  `test_abap_fixture_prog`, `test_abap_fixture_prog_include`,
  `test_abap_fixture_fugr_form_include`, `test_abap_fixture_fugr_pbo_include` (1.3);
  `test_abap_fixture_clas`, `test_abap_fixture_clas_locals_imp`,
  `test_abap_fixture_clas_sub`, `test_abap_fixture_clas_testclasses` (1.4);
  `test_abap_fixture_clas_testclasses_detection` (1.7).
  A test named `abap_fixture_<story>_<topic>` in a story's Verification section
  does not exist yet: the story adds it, in the same file and style.

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
