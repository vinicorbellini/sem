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
   now reads `FOR TESTING`, `RISK LEVEL` and `DURATION`, see fact 12, and
   `TYPES`, see fact 14, though sem still takes `TYPES` from the fallback; the
   rest of this fact still holds.)
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
    eats the statement's period (fixed in the fork, fact 13).

12. Upstream's `class_declaration` took its additions in one fixed order
    and had no `FOR TESTING`, `RISK LEVEL` or `DURATION`, so every test
    class definition was an `ERROR`. The fork takes them in any order, and
    `METHODS x FOR TESTING.` too. Not the chained `METHODS: a, b FOR
    TESTING.`, 263 statements in abapGit: the grammar has no chained
    `METHODS` at all, nor chained `INTERFACES:`, and either one still puts a
    class definition in an `ERROR` (fixed in the fork, fact 14). Error recovery around such an `ERROR`
    moves with every grammar change, so a fix can lose a few entities in one
    file while it gains them in others (after this fix,
    `zcl_abapgit_gui_page_repo_view` lost its 15 `DATA` attributes and the
    ajson test classes gained 17 `TYPES`); the totals are in the table below.

13. A `*` starts a comment only in column 1. Upstream's `bol_comment` took
    any `*` and the rest of its line, so `lv = lines( lt ) * 2.` lost its
    period. A tree-sitter token cannot see the column, so the fork reads
    `bol_comment` with an external scanner (`src/scanner.c` in the grammar
    crate). On its own the fix does not reduce the methods the fallback adds:
    the scanner also stops tokens taking a leading space with them, which
    moves error recovery around the statements the grammar still cannot read.
    Three class files lost most of their grammar methods to it:
    `zcl_abapgit_object_clas` and `zcl_abapgit_object_intf`, whose
    definitions hold a `CONSTANTS: BEGIN OF` with a component named `methods`
    that recovery now reads as the `METHODS` keyword, and
    `zcl_abapgit_gui_page_repo_view`, which became one `ERROR`, as did
    `zif_abapgit_git_definitions`, which lost its interface entity. Fact
    14's fix takes all four back.

14. Upstream had no chained `METHODS:`, `CLASS-METHODS:` or `INTERFACES:`,
    and no `INTERFACES`, `CLASS-DATA`, `CONSTANTS` or `TYPES` statement at
    all; a class definition took only single `DATA`. In abapGit's class
    definitions and interfaces that is 263 chained `METHODS:`, 34 chained
    `CLASS-METHODS:`, 417 `INTERFACES`, 99 `CLASS-DATA`, 352 `CONSTANTS`,
    661 `TYPES` and 143 chained `DATA:` statements, each of which put its
    definition in an `ERROR`. The fork reads all of them, single and chained,
    and `TYPES` and `CONSTANTS` in method bodies too (the crate's README,
    fix 5, has the node shapes). Every attribute of a class or interface is
    now a `variable_declaration` right under its section, so a chained `DATA:`
    or `CLASS-DATA` gives a `variable` entity per part; before, some of these
    were variables only when error recovery happened to leave a
    `DATA x TYPE y` behind, and others were not entities at all. `TYPES` are
    still entities from the fallback: the grammar's `types_declaration`
    and `chained_types_declaration` are not in `ABAP_CONFIG` yet. With the
    definitions readable, error recovery no longer swallows some
    implementations whole, so a few files show more error nodes than before
    (`zcl_abapgit_object_iaxu`: its definition used to run to the end of the
    file).

15. The grammar's `name` stops at a `~`, so `METHOD zif_x~m.` was the method
    `zif_x` and an `ERROR`, and sem joined the name back from the `ERROR`
    (`entity_extractor.rs`) only when it held just `~m`; when it held more (a
    long name, part of the body), the method had no name and came from the
    fallback. 3290 of abapGit's 7591 `METHOD` statements name an interface
    method, and after fact 14's fix about 700 of the 1685 fallback entities
    were such methods, with 387 `TYPES` and 591 other lost methods the rest.
    The fork reads `zif_x~m` as one `name` token in `METHOD` and in
    `METHODS ... REDEFINITION`. That halves the fallback (1685 to 843) and
    changes no entity: the same 9934, each method named as before, now from
    the grammar. What the fallback still adds is 387 `TYPES` (the grammar
    reads them since fact 14, but `ABAP_CONFIG` does not list
    `types_declaration` yet), about 440 methods the grammar still loses in 66
    files (`zcl_abapgit_ajson.clas.locals_imp`, `_object_tabl_ddl`,
    `_html_form`, `_gui_page_diff_base` lose more than 20 each), 6 forms and
    a few classes. No single statement accounts for those methods.

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
| 3. `*` comments in column 1 only | 36558 | 727 | 9731 | 7874 | 1857 | 6801 |
| 4. chained declarations, `INTERFACES`, `CLASS-DATA`, `CONSTANTS`, `TYPES` | 30435 | 632 | 9934 | 8249 | 1685 | 6991 |
| 5. `zif_x~m` method names | 27637 | 626 | 9934 | 9091 | 843 | 7144 |

Through every fix every `METHOD` block stays a method entity (7582 method
entities each time, the grammar's plus the fallback's), so the entity totals
move only with class-level `DATA` and `TYPES` and the odd class or interface.
Fix 1 also let 90 local `DATA` in test method bodies through as class
variables (the ajson test classes); fix 2 removed them again. Fix 3 moved
error recovery (fact 13): 94 files have more error nodes and 79 fewer, the
fallback adds methods in 15 files and fewer in 11, and the entities gain 10
`DATA` and 3 `TYPES` and lose `zif_abapgit_git_definitions`. Fix 4 takes back
those three files' methods and the interface, makes 95 more files parse with
no error node, and adds 203 `variable` entities (239 class attributes written
as chained `DATA:` or `CLASS-DATA`, less 36 that recovery had made of local
`DATA` in method bodies and of `CONSTANTS` and `TYPES`); error nodes fall in
655 files and rise in 30, where a definition no longer runs on over the
implementation after it. Fix 5 leaves the entity set as it was, moves 842
of them from the fallback to the grammar, cleans 6 more files, and lowers
error nodes in 330 files; 13 have more (`zcl_abapgit_object_tran` +54,
`_object_wdca` +39, `zcl_abapgit_gui_page_flowcons` +37), where methods that
error recovery used to fold into one are now separate and their bodies'
unread statements count on their own. No file that had no error node has one
after any fix.
