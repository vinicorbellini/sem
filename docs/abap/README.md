# ABAP port planning

This folder is fork-only planning for the ABAP language port. It lives on the
`abap` branch family of this fork and will not go upstream to Ataraxy-Labs/sem.
Do not include `docs/abap/` in any upstream pull request. Code changes that the
stories describe may be offered upstream separately, with their own
`CHANGELOG.md` entry as `CONTRIBUTING.md` requires.

## Contents

- `stories/README.md`: the order of the Tier 1 stories and the Gate 1 criterion.
- `stories/1-1-case-folding.md` to `stories/1-9-tests-and-fixture.md`: one file
  per Tier 1 spec row (review-grade ABAP support).
- `census-gate1.md`: the parse-error census on abapGit. Written by stories 1.6
  and 1.8, required by Gate 1. It does not exist yet.

## Starting point

Commit `2618ff1` on the `abap` branch adds the `lang-abap` feature, the
`tree-sitter-abap-sqry` grammar, `ABAP_CONFIG`, and extraction of
`class_declaration`, `class_implementation`, `interface_declaration`,
`method_implementation` and `function_implementation`. Everything in the stories
builds on that commit.

## Facts found while writing the stories

These differ from the first-draft implementation notes. Each story applies them.

1. Default excludes live in `crates/sem-core/src/utils/scan.rs`
   (`is_default_excluded`), not in `crates/sem-core/src/system/scan.rs`. The
   `system/scan.rs` file holds boundary-model syntax facts for five languages
   and is unrelated.
2. `fixtures` is itself a default-excluded directory name. A repo-wide scan skips
   `crates/sem-core/tests/fixtures/abap/` unless `--no-default-excludes` is given. Fixture tests
   must use that flag or call the plugin directly.
3. The grammar has no `form_definition`, `module`, `define` or `types` node.
   `FORM`, `ENDFORM`, `MODULE`, `ENDMODULE`, `DEFINE` and `PERFORM` all parse as
   `macro_include` whose `name` is the keyword. `TYPES` parses as an `ERROR`
   node. `FOR TESTING` produces `ERROR` nodes inside `class_declaration` and
   `method_declaration`. Stories 1.3 and 1.6 handle this.
4. `function_implementation` already exists and covers `FUNCTION ... ENDFUNCTION`.
   The `REPORT` statement is a `report_statement`; the root is `program`.
5. ABAP has `scope_resolve: None`, so `scope_resolve.rs` is not on the ABAP
   resolution path. The name lookups that matter are the `SymbolTable` in
   `graph.rs`, `entity_matches_query` and `print_name_suggestions` in
   `crates/sem-cli/src/commands/mod.rs`, `qualified::matches`, and
   `index::reader::lookup`, which does a byte-wise binary search.
6. `bench/` holds no Rust benchmark. The Rust benches are
   `crates/sem-core/benches/parse_profile.rs` and `incremental.rs`.
7. The registry maps every dot-suffix of a file name to a plugin. `.abap` is
   already registered, so abapGit names already reach the ABAP plugin. No
   extension-map change is needed for story 1.5.
8. The abapGit clone at `/tmp/claude-0/abapGit` is shallow, with one commit.
   Story 1.8 needs `git fetch --unshallow` first. Its history uses squash merges
   with `(#NNNN)` in the subject, not merge commits.
