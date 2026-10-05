# 1.9 Rust tests and fixture wiring

## Intent

Every ABAP node type that sem extracts needs a Rust unit test, and every
behaviour in stories 1.1 to 1.7 needs a fixture test over a small ABAP
repository. The fixture repo is `tests/fixtures/abap/`, built in parallel by
someone else. This story writes the unit tests and wires the fixture tests, ignored
until their story lands. It does not create the fixture.

## Acceptance criteria

- `crates/sem-core/src/parser/plugins/code/mod.rs` has one test per entity type
  the ABAP extractor produces: `report`, `form`, `function`, `module`, `method`,
  `class`, `interface`, `type`, `data`, `macro`, plus the local-class and
  test-class attachment. Each is named `test_abap_<node>_extraction`, uses
  `#[cfg(feature = "lang-abap")]`, and follows the shape of
  `test_abap_entity_extraction`.
- Each test prints the extracted pairs with `eprintln!` as the existing language
  tests do, per `CONTRIBUTING.md`, and asserts both names and types.
- A fixture-test file wires every fixture behaviour. Tests are named
  `abap_fixture_<story>_<topic>` and marked `#[ignore = "waiting on story 1.N"]`.
  The reason string names the story.
- When a story lands, its commit removes the `#[ignore]` attributes for its own
  tests. After all nine stories, no fixture test is ignored.
- The fixture tests read `tests/fixtures/abap/` by path relative to the workspace
  root and scan with the default-exclude bypass: the directory name `fixtures`
  is excluded by default, so the tests call the plugin or graph builder on file
  contents directly, or pass `no_default_excludes`.
- `cd crates && cargo test -p sem-core abap` runs clean with ignored tests
  listed.
- The test names match the fixture author's list. A list of names is exchanged
  and committed in `tests/fixtures/abap/README.md` by the fixture author;
  this story reads it, never writes it.

## Files

- `crates/sem-core/src/parser/plugins/code/mod.rs`: the `#[cfg(test)]` module.
  Existing language tests, including `test_abap_entity_extraction` and
  `test_fish_entity_extraction`, sit there.
- `crates/sem-core/tests/abap_fixture.rs` (new): integration tests over
  `tests/fixtures/abap/`. Existing integration tests such as
  `crates/sem-core/tests/graph_accuracy.rs` and `dart_graph.rs` show the layout,
  and `crates/sem-core/tests/fixtures/` shows how a fixture directory is read.
- `tests/fixtures/abap/`: read only. Owned by someone else.

## Approach

Unit tests: copy `test_abap_entity_extraction`. It builds a source string,
calls `CodeParserPlugin.extract_entities(code, "src/zcl_demo.clas.abap")`,
collects `(entity_type, name)` pairs, and asserts with the pairs in the failure
message. For each new node type add the smallest source that produces it. Use the
probe findings in `docs/abap/README.md` for what the grammar emits. Where a node
exists only through the fallback (`form`, `module`, `macro`, `type`), name the
test `test_abap_<node>_fallback_extraction` so a later grammar upgrade shows
where to simplify.

Fixture tests: copy the style of `graph_accuracy.rs`. Each test builds the graph
or runs the plugin over the fixture and asserts one acceptance bullet from its
story. Keep one assertion topic per test, so a flip from ignored to passing maps
to one bullet. Use `#[ignore = "waiting on story 1.N"]`. Run the ignored set
with `--ignored` before and after each story to see the flip.

Land the test scaffolding first, before story 1.1 to 1.7 code, so each story
starts with its failing tests in place. Add unit tests for each node type with
its story (1.3 for most), not all at once.

## Verification

```bash
cd crates
cargo test -p sem-core test_abap
cargo test -p sem-core abap_fixture                        # ignored tests are listed, not run
cargo test -p sem-core abap_fixture -- --ignored           # shows what still fails
cargo test -p sem-core abap_fixture -- --include-ignored   # Gate 1: must be all green
cargo test --workspace
```

Fixture tests that flip from ignored to passing are those of stories 1.1 to
1.7. Each story file lists its own.

## Out of scope

- Creating or editing anything under `tests/fixtures/abap/`.
- Benchmarks. Story 1.1 owns the performance check.
- Hand-checked diff quality. Story 1.8 owns it.
- Tests for Tier 2 features.

## Estimate

1-2 days, spread across the other stories.

## Depends on

Runs alongside all stories. Needs the fixture repo to exist before the fixture
tests can pass, not before they can be written.
