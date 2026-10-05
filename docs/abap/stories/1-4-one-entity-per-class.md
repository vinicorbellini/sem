# 1.4 One entity per class

## Intent

An ABAP class has two top-level blocks: `CLASS x DEFINITION` and `CLASS x
IMPLEMENTATION`. Today they are two entities, a `class` and an `implementation`,
with the same name. `sem find zcl_excel_style` returns both and looks
ambiguous, and `sem diff` shows a signature change and a body change as two
unrelated entities. Collapse them into one `class` entity that owns two ranges.

## Acceptance criteria

- `sem find zcl_excel_style --json` returns exactly one entity of type `class`.
  No entity of type `implementation` exists.
- That entity has two ranges: `ranges: [{role: "definition", start_line, end_line},
  {role: "implementation", start_line, end_line}]`. Its `start_line` is the
  definition start and its `end_line` is the implementation end.
- Every method of the class has `parent_id` equal to the class entity id.
- A class with a definition and no implementation in the file (an abstract or
  interface-only class, or a definition in a `.locals_def.abap` file) yields one
  `class` entity with one range.
- A local class and a global class with the same name in different files are
  distinct entities; the file path is part of the id.
- `sem diff` on a change that edits only a method body reports the method
  modified and the class modified, and on a change that edits only the
  definition reports the class modified and no method.
- Fixture tests `abap_fixture_1_4_single_class_entity` and
  `abap_fixture_1_4_two_ranges` pass and are no longer ignored.
- The existing test `test_abap_entity_extraction` is updated: it expects three
  entities (`class`, two `method`) and no `implementation`.

## Files

- `crates/sem-core/src/model/entity.rs`: `SemanticEntity`. Add an optional
  `ranges` field, skipped from serialisation when empty, following the
  `skip_serializing_if` pattern already used for `parent_id` and
  `structural_hash`.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: `visit_node` and
  the post-pass that runs after the walk. Merge the `class_declaration` and
  `class_implementation` entities with the same folded name in the same file.
  Remove `"class_implementation" => "implementation"` from `map_node_type` once
  the merge is in place, or keep it as the internal marker the merge consumes.
- `crates/sem-core/src/parser/plugins/code/languages.rs`: no config change.
- `crates/sem-core/src/parser/graph.rs`: `EntityInfo` and the places that read
  `start_line` and `end_line` for reference extraction. The entity content used
  for reference scanning must cover both ranges, not the gap between them.
- `crates/sem-core/src/parser/differ.rs` and `differ/`: content hashing for a
  two-range entity.
- `crates/sem-cli/src/commands/` formatters: print both ranges in text and JSON
  output.

## Approach

Do the merge as a post-pass in the extractor, keyed on the lower-cased class
name, the same rule story 1.1 uses for lookups. Run it after the walk, so
`visit_node` stays as it is. For each name with both a definition and an
implementation entity:

1. Keep the definition entity as the `class` entity.
2. Append the implementation's range.
3. Set the content to the definition text, a separator line, and the
   implementation text, so `content_hash` changes when either half changes.
4. Re-parent the methods to the surviving id.
5. Drop the implementation entity.

Entity ids include the entity type today (the existing test checks that the
parent id contains `implementation`). Choose the stable id as
`<file>::class::<name>`, and confirm that moving a method between classes still
reads as a move, not a delete and add, in `differ.rs`.

Content hashing must stay deterministic and must not depend on the gap text
between the two blocks. A reordering of the two blocks in the file changes
nothing semantic, so hash each range separately, sorted by role.

Pattern to copy: `impl_item` in Rust, where `impl` blocks for one type attach as
related entities. Read how the Rust path in `entity_extractor.rs` names an impl
before choosing the id.

## Verification

```bash
cd crates
cargo test -p sem-core test_abap_entity_extraction
cargo test -p sem-core test_abap_class
cargo test -p sem-core abap_fixture_1_4 -- --include-ignored
cargo test --workspace
cargo run -p sem-cli -- find zcl_excel_style --json --no-default-excludes   # on the fixture repo, from its root
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_4_single_class_entity`, `abap_fixture_1_4_two_ranges`.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Changed` if upstreamed.

## Out of scope

- Merging classes across files. A class defined in `x.clas.abap` and extended in
  `x.clas.locals_imp.abap` stays two entities, linked by parent (story 1.3).
- Splitting a class diff into definition and implementation findings in the
  review output.
- Interfaces. They have one block and need no merge.

## Estimate

1-2 days.

## Depends on

Story 1.3, which fixes the entity set and the map from node type to entity type.
