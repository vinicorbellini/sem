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
`tree-sitter-abap-sqry` grammar (since replaced by the fork in
`crates/tree-sitter-abap`, see "Grammar fork" below), `ABAP_CONFIG`, and extraction of
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
   `method_declaration`. Stories 1.3 and 1.6 handle this. (The fork's grammar
   now reads `FOR TESTING`, `RISK LEVEL` and `DURATION`, see fact 12; the rest
   of this fact still holds.)
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
9. Story 1.3 found that the grammar's error recovery does not keep `ENDFORM`
   as a sibling `macro_include`: in `zfx_report.prog.abap` it lands inside the
   `ERROR` that swallowed the form body, and one `ERROR` can swallow a later
   `FORM` and `MODULE` whole. `TYPES` can likewise be folded into the
   `METHODS` or `DATA` statement before it. So the fallback in
   `abap_fallback.rs` reads `PROGRAM`, `FORM`, `MODULE`, `DEFINE` and class
   `TYPES` off the token stream (the tree's leaves, cut at each `.`), not off
   sibling nodes. It goes once mkoval1/tree-sitter-abap has nodes for them.
10. Story 1.10 found that the token stream of fact 9 was itself wrong where
    it mattered most. The grammar does not end a `'...'` literal at the end of
    its line, as ABAP does, so after a quote it misreads (`''`, or a template
    like `|{ a }*|`) one leaf runs over several lines and holds the statements
    in them: 943 of abapGit's `METHOD` lines and 942 `ENDMETHOD` lines sit
    inside a `character_literal` leaf. The same misreading is the main reason
    the grammar drops the rest of a `CLASS ... IMPLEMENTATION` or runs a
    `method_implementation` on to the end of the class. So `abap_fallback.rs`
    now cuts its statements from the source with comments and literals blanked
    by `strip_abap_content` (the reference scan's stripper, story 1.2), not
    from the leaves, and reads `METHOD` ... `ENDMETHOD` blocks off them too.
    A grammar method that starts at a block's `METHOD` with the block's name
    wins and is cut back to its own `ENDMETHOD`; a block with no grammar
    method becomes a `method` tagged `source: abap-fallback`, under its
    implementation (and the implementation and its definition are added as
    entities too when the grammar lost them). A `METHOD` with no `ENDMETHOD`
    is no entity, as for `FORM`, and a grammar method that is no block's (an
    unclosed one, or one named after the wrong token, `get_steps` for
    `zif_x~get_steps`) is dropped. On abapGit every one of the 7577 `METHOD x.`
    blocks is now a method entity and none spans another
    (`census-gate1.md`, "After story 1.10").
11. The grammar's `character_literal` was `/'[^']+'/`: no `''` escape, no
    empty literal, nothing to stop it at a line end, and no token at all for
    backtick literals or `|...|` templates. That is the runaway literal of
    fact 10. sem's fork of the grammar (below) fixes it: `'...'` and backtick
    literals end at their line with a doubled quote as the escape, and a
    `|...|` template is one `string_template` token whose `{ ... }` parts are
    not parsed. With that fixed, abapGit still has 37305 error nodes (from
    39314): the rest is statements the grammar has no rule for. Some files
    have more error nodes than before, because code that a runaway literal
    used to hide is now parsed. The fallback of fact 10 still matters: the
    grammar still loses whole implementations and stretches methods on other
    statements, for example a `*` mid-line, which `bol_comment` (`"*"` then
    the rest of the line, not anchored to column 1) reads as a comment that
    eats the statement's period.

12. Upstream's `class_declaration` took its additions in one fixed order
    and had no `FOR TESTING`, `RISK LEVEL` or `DURATION`, so every test
    class definition was an `ERROR`. The fork takes them in any order, and
    `METHODS x FOR TESTING.` too. Not the chained `METHODS: a, b FOR
    TESTING.`, 263 statements in abapGit: the grammar has no chained
    `METHODS` at all, nor chained `INTERFACES:`, and either one still puts a
    class definition in an `ERROR`. Error recovery around such an `ERROR`
    moves with every grammar change, so a fix can lose a few entities in one
    file while it gains them in others (after this fix,
    `zcl_abapgit_gui_page_repo_view` lost its 15 `DATA` attributes and the
    ajson test classes gained 17 `TYPES`); the totals are in the table below.

## Grammar fork

sem builds against its own copy of mkoval1/tree-sitter-abap, in
`crates/tree-sitter-abap/` (a workspace member, path dependency of
`sem-core`'s `lang-abap`, `publish = false`). It was taken at upstream commit
`c7604df9e25d56ae879fa25694fd9f2ddbab05d8` (2024-06-29, MIT), cloned from
GitHub, the same commit the `tree-sitter-abap-sqry` 32.0.1 crate vendored.
The crate's `README.md` is the authoritative list of what the fork changes.

- Edit `grammar.js`, never `src/`. Regenerate with tree-sitter CLI 0.26.8, the
  version of the `tree-sitter` crate in `crates/Cargo.lock`
  (`npm install -g tree-sitter-cli@0.26.8`):
  `cd crates/tree-sitter-abap && tree-sitter generate`. It warns that there is
  no `tree-sitter.json` and generates ABI 14, as upstream did; that is
  intended. Commit `grammar.js` and `src/` together.
- Grammar tests: `tree-sitter test` in the same directory runs the corpus in
  `test/corpus/` (upstream's files plus the fork's, such as `literals.txt`).
  Then `cargo test -p tree-sitter-abap` and `cargo test -p sem-core`.
- The census measures a grammar change on real code:
  `sem find --in src --parse-report --file-exts .abap` on abapGit with a
  scratch `SEM_CACHE_DIR` (`census-baseline.md`). Regenerating the unchanged
  upstream grammar gave the same tree on all 752 abapGit files and the same
  census (39314 error nodes, 9703 entities, 2917 from the fallback).
- Tests that exercise `abap_fallback.rs` need a statement the grammar still
  loses methods on. Once a grammar fix removes a test's trigger, give the test
  a new one (cut down from abapGit) rather than dropping it, and add a test
  that the old trigger now parses.

Fixes so far, with the abapGit census after each (`*.abap` under `src/` at
`b2b4e25`; method nodes are the grammar's own `method_implementation` nodes,
for 7577 `METHOD` blocks):

| Grammar | Error nodes | Files with error nodes | Entities | From the grammar | From the fallback | Method nodes |
|---|---:|---:|---:|---:|---:|---:|
| upstream `c7604df` | 39314 | 731 | 9703 | 6786 | 2917 | 5705 |
| 1. literals end at their line | 37305 | 731 | 9808 | 7966 | 1842 | 6818 |
| 2. `FOR TESTING`, `RISK LEVEL`, `DURATION` | 37198 | 727 | 9719 | 7963 | 1756 | 6912 |

Through both fixes every `METHOD` block stays a method entity (7582 method
entities each time, the grammar's plus the fallback's), so the entity totals
move only with class-level `DATA` and `TYPES`. Fix 1 also let 90 local `DATA`
in test method bodies through as class variables (the ajson test classes);
fix 2 removed them again.
