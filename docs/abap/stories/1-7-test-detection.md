# 1.7 Test detection

## Intent

abapGit stores unit tests in `*.clas.testclasses.abap`, and a test class is any
class declared `FOR TESTING`. sem does not recognise either, so `sem impact
--tests` and the test-versus-source split in reviews miss ABAP tests. Teach the
two detectors about both forms.

## Acceptance criteria

- `is_test_path("src/zcl_foo.clas.testclasses.abap")` returns true.
- `is_test_path("src/zcl_foo.clas.abap")` and `src/zcl_foo.clas.locals_imp.abap`
  return false.
- A class whose definition contains `FOR TESTING`, in any file, is a test
  entity: `is_test_entity` returns true for the class and for its methods.
- A local test class in `zcl_foo.clas.locals_imp.abap` with `FOR TESTING` is
  detected even though the file name is not a test file.
- A method named `test_x` in a non-test class is not a test, matching existing
  name-pattern rules.
- `sem impact zcl_foo --tests` lists the test class from
  `zcl_foo.clas.testclasses.abap` as a test.
- Fixture tests `abap_fixture_1_7_testclasses_file` and
  `abap_fixture_1_7_for_testing_class` pass and are no longer ignored.

## Files

- `crates/sem-core/src/parser/test_detect.rs`: `TEST_FILE_PATTERNS` (line 27)
  holds `_test.`, `.test.`, `_spec.`, `.spec.`. `is_test_path_with_custom_dirs`
  does a `contains` on the lower-cased file name. Add `.testclasses.` to the
  array. No new logic is needed, because `zcl_foo.clas.testclasses.abap`
  contains `.testclasses.`.
- `crates/sem-core/src/parser/graph.rs`: `is_test_entity` (line 4970). It uses
  name patterns and a content-marker list that matches `#[test]`, `@Test` and
  so on, gated on the file being a test path. `FOR TESTING` classes in
  non-test files fall through. Add a content check for `FOR TESTING`,
  case-insensitive, that returns true without the file gate, since the marker
  is unambiguous.
- `crates/sem-core/src/parser/test_detect.rs` unit tests: extend
  `test_file_name_patterns` and `no_false_positives`.

## Approach

Two small changes, one per detector. The first-draft note named only
`test_detect.rs`. That file decides paths. Entity-level detection is separate and
lives in `graph.rs::is_test_entity`, so both must change.

Copy the existing array entry style for the path pattern. For the entity rule,
add one clause beside the content markers, but outside the `in_test_file &&`
conjunction:

- ABAP is case-insensitive, so test with `eq_ignore_ascii_case` over a sliding
  window, or lower-case the entity content once. Gate it on
  `entity.file_path` ending in `.abap` so other languages do not scan for the
  phrase.
- Match `FOR TESTING` as two words separated by whitespace, not inside a
  comment. Story 1.2's stripper is the right tool when it has landed; until
  then accept the false positive from a comment and note it in the test.

Methods inherit: a method whose parent class is a test entity is a test. Check
how `is_test_entity` treats parents today and follow it.

## Verification

```bash
cd crates
cargo test -p sem-core test_detect
cargo test -p sem-core test_abap_test_detect
cargo test -p sem-core abap_fixture_1_7 -- --include-ignored
cargo test --workspace
cargo run -p sem-cli -- impact zcl_foo --tests --no-default-excludes   # on the fixture repo
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_7_testclasses_file`, `abap_fixture_1_7_for_testing_class`.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Added` if upstreamed.

## Out of scope

- ABAP Unit risk level and duration attributes.
- Linking a test class to the class under test through `CLASS ... DEFINITION
  FOR TESTING` and `cut` attributes.
- Selecting affected tests, which is Tier 2.

## Estimate

0.5 days.

## Depends on

Nothing.
