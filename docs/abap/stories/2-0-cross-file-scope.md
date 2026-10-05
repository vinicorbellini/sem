# 2.0 Cross-file candidate scope

## Intent

Tier 1 left ABAP references stuck inside one file. Run `sem graph --json` on the
fixture repo and every ABAP edge is a `typeref` from a method to an attribute of
its own class. There is no call edge at all, and none crosses a file. The cause
is in `graph.rs`. The bag-of-words resolver looks a name up in the entity's own
file only, and ABAP has no imports to widen that. Global ABAP names are unique
per system, so for ABAP the candidate scope for a global name is the whole
repository. Local names stay in their object. Everything else in Tier 2
depends on this, so it goes first and alone.

## Acceptance criteria

- `sem find add_new_worksheet --callers` on abap2xlsx lists
  `zcl_excel_reader_2007.load_workbook` as a resolved caller, in the caller
  list and not under `possible_callers`. The call is
  `io_excel->add_new_worksheet( lv_worksheet_title )` at line 2021 of
  `zcl_excel_reader_2007.clas.abap`, inside `load_workbook` (lines 1781 to 2216).
- In the fixture repo, `ZCL_FX_ORDER=>CREATE( )` written in
  `zcl_fx_user.clas.abap` gives an edge to `zcl_fx_order.create` in another file,
  whatever the case of either spelling.
- A local class never leaks. `zcl_fx_other` and `zcl_fx_order` each have a local
  `lcl_helper`. A use of `lcl_helper` in one object gives an edge to its own
  object's class only.
- A method name defined in two or more global classes of the repo gives no
  cross-file edge. `describe` is defined in `zcl_fx_order` and `zcl_fx_user`.
  A call from a third object resolves to neither.
- For every non-ABAP language, `sem graph --json` on this repo is byte-identical
  before and after.
- `parse_profile` and `incremental` show no regression above noise on a
  non-ABAP corpus, and the ABAP-only table costs nothing when no ABAP file is
  in the corpus.
- Fixture tests `abap_fixture_2_0_global_class_across_files`,
  `abap_fixture_2_0_local_class_stays_in_object`,
  `abap_fixture_2_0_ambiguous_method_no_edge` and
  `abap_fixture_2_0_unique_method_across_files` pass.

## Files

- `crates/sem-core/src/parser/graph.rs`:
  - `resolve_entity_references` (the symbol-table block near line 2009) reads
    `context.symbol_table_by_file.get(ref_name)` and then `by_file.get(entity.file_path)`.
    For an ABAP entity, also consult a new `abap_global` table, described below.
  - `build_symbol_table_by_file` (line 1365) buckets candidates by file for
    speed. Add a sibling `build_abap_global_table` that keeps, per folded name,
    the candidates visible repo-wide. Call it at the three build sites that
    construct `ReferenceResolutionContext` (near lines 3196, 3625 and 4399) and
    carry it as one more field of the context.
  - The incremental path (near lines 3938 to 4143) rebuilds candidates for
    changed files with `case_insensitive_for_file`. It must rebuild the
    `abap_global` entries too, or a new definition in file B does not reach a
    reader in file A. The read is already recorded as
    `Table::SymbolTable` by name, so the dirty set should follow. Prove it with a test.
- `crates/sem-core/src/parser/plugins/code/abap_name.rs`: add
  `fn abap_global_scope(file_path: &str, entity_type: &str) -> Scope`, next to
  `parse_abapgit_name`, and the rule table in Approach. Reuse `AbapObject`
  rather than splitting file names again.
- `crates/sem-core/src/parser/plugins/code/languages.rs`: add
  `LanguageConfig::repo_wide_names(&self) -> bool`, true for `"abap"`, beside
  `case_insensitive`.
- `crates/sem-core/tests/fixtures/abap/`: new files, listed below.
- `crates/sem-core/src/parser/plugins/code/mod.rs`: the tests. The helper
  `abap_graph` writes files flat into a temp directory; if a test needs
  sub-directories, make it call `create_dir_all` first.

## Approach

The calls pipeline is not the place for this story. It is registered by file
extension in `calls/mod.rs` (`LANGUAGES`), and ABAP is not in it. Python, Go and
Rust are, and all three have `scope_resolve: None`, as ABAP does. Moving ABAP
into that table is story 2.1. This story widens the bag-of-words resolver only,
because it is the one that runs for ABAP today. It is also the only way to get
the acceptance case before story 2.2 gives receivers a type: `io_excel` is an
importing parameter, and the resolver cannot see its type yet.

The visibility rule, by what the candidate is:

| Candidate | Visible from |
|-----------|--------------|
| global class or interface (main `.clas.abap` or `.intf.abap`, part none) | the whole repo |
| function module (`<group>.fugr.<fm>.abap`) and report | the whole repo |
| local class (`locals_def`, `locals_imp`, `testclasses` parts) and its members | the same object only |
| form, module, macro | the same object only (includes: story 2.4) |
| method or attribute of a global class | the same object, or the whole repo when the name is unique |

"Same object" means equal `AbapObject.name` after case folding, as
`parse_abapgit_name` returns it. The function group `zfx_fg` is one object, so
`PERFORM calc_extra` in `zfx_fg.fugr.zfx_fm.abap` already reaches the form in
`zfx_fg.fugr.lzfx_fgf01.abap`. A program include such as
`zfx_report_f01.prog.abap` is its own object. That case is story 2.4.

The unique-name rule exists because method names are not unique. In abap2xlsx,
777 distinct method names cover 1035 `METHOD` blocks, and 95 of the names, on 353
blocks, are defined more than once. The other 682 names are safe to bind by name
before receivers have types. A name defined twice stays unbound. The completeness
verdict then lists the call as possible, which is honest.

Interface-implementing methods are named `zif_fx_order~get_total`, so the bare
token `get_total` matches nothing. For a token pair `zif_x`, `~`, `m` in the
content, probe the key `zif_x~m`. Keep the tokenizer's split on `~` from story
1.1 and do the pairing in the lookup.

This rule is an interim. Story 2.2 replaces name guesses with receiver types,
and story 2.1 puts a seam in place so the two do not fight. Write the rule as
one function in `graph.rs` that can be deleted, with a comment saying so.

Case folding is already in place from story 1.1: `symbol_key` folds the entity
name, and `build_file_reference_index` folds tokens. Key the new table with
`symbol_key`.

Beware `CLASS zcl_excel_reader_2007 DEFINITION LOCAL FRIENDS ltc_x.` in
abap2xlsx's `*.clas.testclasses.abap`. It names the global class from inside the
test file. The scope rule must not let such a class entity shadow the real one.
Check what story 1.3 makes of it and record the answer in a test.

Fixture additions, each under 60 lines with an `.xml` envelope for `clas`:

- `zcl_fx_user.clas.abap` and `.clas.xml`: a global class with `run`, which calls
  `ZCL_FX_ORDER=>CREATE( 1 )` in upper case, and `describe`, a name that
  `zcl_fx_order` also defines.
- `zcl_fx_other.clas.abap`, `.clas.xml` and `.clas.locals_imp.abap`: a second
  global class whose local `lcl_helper` has its own `tag` method.
- Add a row for each file to `crates/sem-core/tests/fixtures/abap/README.md`.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_0
cargo test -p sem-core abap
cargo test --workspace
cargo bench -p sem-core --bench parse_profile -- --save-baseline before   # on the base commit
cargo bench -p sem-core --bench incremental -- --save-baseline before
cargo bench -p sem-core --bench parse_profile -- --baseline before        # after
cargo bench -p sem-core --bench incremental -- --baseline before
cargo build --release -p sem-cli
# non-ABAP identity: run on the base commit and on this one, then diff
SEM_NO_INDEX=1 target/release/sem graph --json > /tmp/graph-after.json
# acceptance on the reference repo
cd /home/user/abap2xlsx && SEM_NO_INDEX=1 /path/to/sem find add_new_worksheet --callers --json
```

The last command must show `zcl_excel_reader_2007.load_workbook` in `related`
and not in `possible_callers`.

## Out of scope

- Receiver types. A call through `io_excel` binds by the unique-name rule here
  and by type in story 2.2.
- `INCLUDE` and `PERFORM` across programs (2.4).
- Interface and inheritance dispatch (2.3).
- Showing unbound calls as possible callers (2.5).

## Estimate

2 to 3 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so read this as a ceiling.

## Depends on

Tier 1 (stories 1.1 and 1.5 in particular). Nothing else. Stories 2.1 to 2.8
all depend on this one.
