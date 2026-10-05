# 1.6 Parse-error tolerance

## Intent

The ABAP grammar is incomplete. It emits `ERROR` nodes for `TYPES`, for
`FOR TESTING` and for many statements. A review tool must still list the classes,
methods and forms in such a file, and must say how much it could not parse. A
silent drop looks identical to "no entities", which is the failure to avoid.
Keep extracting around error nodes, and report counts per file.

## Acceptance criteria

- A file containing error nodes still yields its classes, methods and forms.
  Fixture: a class whose definition contains `TYPES` and `FOR TESTING` (both
  produce `ERROR`) still yields the class and all methods.
- `sem find --in <file> --json` includes, per file, `entity_count` and
  `error_node_count`. A clean file reports `error_node_count: 0`.
- `sem graph --json` includes a `parse_summary` with the totals: files, files
  with errors, entity count, error-node count.
- A file that yields zero entities and has error nodes is listed with a warning
  on stderr naming the file. It is never silently omitted.
- Entities produced by the fallback pass from story 1.3 are counted separately
  as `fallback_entity_count`.
- Output for other languages is unchanged unless they also have error nodes,
  in which case the same two fields appear.
- The census on abapGit is run and written to `docs/abap/census-gate1.md`:
  files parsed, files with errors, total error nodes, entities per file
  distribution, ten worst files.
- Fixture tests `abap_fixture_1_6_errors_still_yield_entities` and
  `abap_fixture_1_6_error_count_reported` pass and are no longer ignored.

## Files

- `crates/sem-core/src/parser/plugins/code/mod.rs`: `CodeParserPlugin`. It already
  calls `root.has_error()` in `top_level_imports` (near line 91) to refuse
  edits. Extraction itself does not refuse. Add an error-node count on the
  parse result.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: `visit_node`.
  Check that it does not stop descending at `ERROR` nodes. Add a counter walk.
- `crates/sem-core/src/parser/plugin.rs`: the plugin trait. Add a method with a
  default body, for example `fn parse_stats(&self, path, content) -> ParseStats`,
  so other plugins need no change.
- `crates/sem-core/src/parser/graph.rs`: collect per-file stats while building
  the graph; `EntityGraph` carries a `parse_stats` map.
- `crates/sem-cli/src/commands/entities.rs` (backs `sem find --in`) and
  `commands/graph.rs`: print the fields in `--json`.
- `crates/sem-cli/src/main.rs`: no new flag. The fields are always in `--json`.

## Approach

Count with one tree walk using `tree_sitter::TreeCursor`: increment on
`node.is_error()` and on `node.is_missing()`. Return both counts and the entity
count. Do not use `root.has_error()` as the count, since it returns a boolean.

Tolerance first. Write a test before any code change: a class definition with
`TYPES` and `FOR TESTING`, expecting the class and every method. Run it on the
current branch. If it already passes, the story is reporting only. If the
`ERROR` node swallows a method, handle it in `visit_node` by descending into
`ERROR` children and treating a `method_implementation` found there as normal.
The probe on this grammar showed `ERROR` inside `class_declaration` and
`method_declaration`, not around implementations, so descent is likely
unnecessary, but confirm.

Never drop silently. Three outputs, all in this story:

1. `--json` fields on every file.
2. A stderr line for a file with errors and zero entities.
3. The census document.

Pattern to copy: `parse_errors: usize` already exists in
`crates/sem-core/src/topology/graph.rs` (`SourceFile.parse_errors`). Read how it
is counted and surfaced, and reuse the helper if it is generic.

Census: run the same count over `/tmp/claude-0/abapGit/src`
with a short `examples/` binary under `crates/sem-core/examples/`, following
`parse_probe.rs`, and paste the numbers into `census-gate1.md`. Story 1.8 adds its
before and after.

## Verification

```bash
cd crates
cargo test -p sem-core test_abap_parse_errors
cargo test -p sem-core abap_fixture_1_6 -- --include-ignored
cargo test --workspace
cargo run -p sem-cli -- find --in tests/fixtures/abap --no-default-excludes --json | grep -c error_node_count
cargo run -p sem-core --example parse_probe -- /tmp/claude-0/abapGit/src   # census numbers; adapt flags to the example
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_6_errors_still_yield_entities`,
`abap_fixture_1_6_error_count_reported`.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Added` if upstreamed.

## Out of scope

- Fixing the grammar errors themselves. Report them; do not patch the grammar.
- A threshold that fails a build on error count. That is Tier 2.
- Error recovery that guesses entities from text in files where the parser fails
  entirely, beyond the fallback in story 1.3.

## Estimate

1-2 days.

## Depends on

Story 1.3, which defines the fallback entities counted separately.
