# 1.3 Full entity set

## Intent

Commit `2618ff1` extracts classes, implementations, interfaces, methods and
function modules. A reviewer also names reports, forms, modules, local classes,
class-level types and data, and `DEFINE` macros. Several of those are not nodes
in the grammar at all. This story adds each one, using the grammar where it has
a node and a documented fallback where it does not.

## Acceptance criteria

`sem find --in <file> --json` on the fixture files lists these entity types, each
with the correct name and line range:

- `report` for `REPORT zfoo.` and `PROGRAM zfoo.` (the statement and the program
  as one entity, named `zfoo`).
- `form` for `FORM do_it ... ENDFORM.`, named `do_it`.
- `function` for `FUNCTION zfm_x ... ENDFUNCTION.`, named `zfm_x`.
- `module` for `MODULE status_0100 OUTPUT ... ENDMODULE.`, named `status_0100`.
- `method` for each `METHOD ... ENDMETHOD.`, nested under its class.
- `class` for `CLASS x DEFINITION` and `implementation` for `CLASS x
  IMPLEMENTATION` (collapsed in story 1.4).
- `interface` for `INTERFACE ... ENDINTERFACE.`
- `type` for class-level `TYPES` and `data` for class-level `DATA`, both nested
  under the class, named by the declared name.
- `macro` for `DEFINE _m. ... END-OF-DEFINITION.`, named `_m`.
- Local classes in `*.clas.locals_imp.abap` and test classes in
  `*.clas.testclasses.abap` carry the global class as parent, by file name
  (`zcl_foo.clas.locals_imp.abap` attaches to `zcl_foo`).
- Fixture tests `abap_fixture_1_3_report`, `_form`, `_function`, `_module`,
  `_method`, `_class`, `_interface`, `_types_data`, `_macro` and
  `_local_classes_attach` pass and are no longer ignored.

## Files

- `crates/sem-core/src/parser/plugins/code/languages.rs`: extend
  `ABAP_CONFIG.entity_node_types` and `container_node_types`.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`:
  - `map_node_type`: map the new node types to entity types. The existing ABAP
    lines map `class_implementation` to `implementation`, `method_implementation`
    to `method` and `function_implementation` to `function`.
  - `extract_name`: add the new node types to the ABAP name branch, and a case
    for `macro_include` that reads the second token.
  - `visit_node`: the ABAP branch for `class_implementation` pushes children.
    Add a branch for `program` children and for the section nodes
    (`public_section`, `protected_section`, `private_section`).
- A new `crates/sem-core/src/parser/plugins/code/abap_fallback.rs`, called from
  `entity_extractor.rs`, for the nodes the grammar lacks (see Approach).

## Approach

First, what the grammar produces. Checked against
`tree-sitter-abap-sqry-32.0.1/grammar-src/node-types.json` and by parsing
samples:

| Source | Grammar node | Action |
|--------|--------------|--------|
| `REPORT zfoo.` | `report_statement` with `name` child | add to `entity_node_types`; map to `report` |
| `PROGRAM zfoo.` | not seen in node list; likely `macro_include` or `ERROR` | verify; fallback if so |
| `FUNCTION ... ENDFUNCTION.` | `function_implementation` | already extracted |
| `CLASS`/`INTERFACE` | `class_declaration`, `class_implementation`, `interface_declaration` | already extracted |
| `METHOD` | `method_implementation` | already extracted |
| `DATA x TYPE i.` | `variable_declaration` inside `public_section` etc. | add as `data`; container: the section nodes |
| `FORM`, `ENDFORM` | `macro_include` named `FORM`, then body statements as siblings, then `macro_include` named `ENDFORM` | fallback |
| `MODULE`, `ENDMODULE` | `macro_include` named `MODULE` / `ENDMODULE` | fallback |
| `DEFINE`, `END-OF-DEFINITION` | `macro_include` named `DEFINE`, then an `ERROR` holding `END-OF-DEFINITION` | fallback |
| `TYPES x ...` | `ERROR` containing `name` nodes | fallback |

The grammar has no `form_definition`, `module` or `define` node. This is the
decision the story must make and record. Preferred order:

1. Fallback pass, no grammar change. After the tree walk, scan the root's
   children in order. When a `macro_include` has name `FORM`, `MODULE` or
   `DEFINE`, start an entity at its first byte and end it at the next sibling
   `macro_include` named `ENDFORM` or `ENDMODULE`, or at the sibling that
   contains `END-OF-DEFINITION`. The entity name is the first token of the
   `parameter_list`. This works on trees where the keyword statements parse as
   `macro_include`. Keep it in `abap_fallback.rs` so it is removable.
2. If the fallback proves unreliable in the fixture (nested `FORM` bodies with
   their own errors), open an issue on the grammar fork
   `mkoval1/tree-sitter-abap` asking for `form_definition`, `module_definition`
   and `macro_definition` nodes, and keep the fallback until a release lands.

`TYPES` is an `ERROR` node, so it is also handled by the fallback: inside a
section, an `ERROR` whose first `name` child text is `TYPES` yields a `type`
entity named by its second `name` child.

Entities that only exist through the fallback are tagged in
`SemanticEntity.metadata` (or the equivalent field the extractor already uses
for such markers; read `entity_extractor.rs` for the current field) as
`source: "abap-fallback"`, so story 1.6 can count them.

Local and test classes: in `entity_extractor.rs`, when the file name matches
`*.clas.locals_imp.abap`, `*.clas.testclasses.abap` or `*.clas.locals_def.abap`,
set each top-level class entity's `parent_id` to the global class entity id for
the object name taken from the file name. Story 1.5 provides the file-name
parser. If the global class is in a different file, the parent id is computed
from the name and path convention (`<dir>/<name>.clas.abap`) without loading the
other file.

Pattern to copy: Elixir's call-based entities in `languages.rs` also build
entities from statements rather than clean nodes. Read how
`call_entity_identifiers` is consumed in `visit_node` before writing the
fallback.

## Verification

```bash
cd crates
cargo test -p sem-core test_abap_entity_extraction     # the existing test must still pass
cargo test -p sem-core test_abap                       # one new test per node type, story 1.9
cargo test -p sem-core abap_fixture_1_3 -- --include-ignored
cargo test --workspace
cargo run -p sem-cli -- find --in tests/fixtures/abap --no-default-excludes --json
```

Run the last command from the repo root with the path adjusted. The fixture
directory is excluded by default, so the flag is required.

Fixture tests that flip from ignored to passing: the ten
`abap_fixture_1_3_*` tests listed above.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Added` if upstreamed.

## Out of scope

- Nested `FORM` calls and `PERFORM` resolution. `PERFORM` stays an unresolved
  `macro_include`.
- `ENHANCEMENT`, `AT SELECTION-SCREEN` and `START-OF-SELECTION` event blocks.
- Method signatures as separate entities. `METHODS` declarations in a section
  stay inside the class definition.
- Collapsing definition and implementation. That is story 1.4.
- Grammar changes themselves. This story files the issue; it does not fork.

## Estimate

3-4 days.

## Depends on

Nothing for the code. Story 1.5 supplies the file-name parser for the
local-class attachment, so land the parser from 1.5 first or stub it here.
Story 1.9 owns the Rust tests.
