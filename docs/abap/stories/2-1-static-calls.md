# 2.1 Static call resolution

## Intent

After story 2.0 a name can reach across files, but only by guessing: any token
that matches a unique name becomes an edge. A call has a shape. `me->m( )`,
`zcl_x=>m( )`, `super->m( )`, `zif_x~m( )`, `CALL METHOD`, `CALL FUNCTION 'ZFOO'`
and `PERFORM f` each name their target without a receiver type. Resolve those
forms exactly, as sem does for Python and Go, and say "unknown, and why" for the
rest instead of guessing. This is the largest story of Tier 2.

## Acceptance criteria

- On the fixture repo, each of these forms gives one `calls` edge to the named
  definition, and no other edge from that site:
  - `me->m( )` and a bare `m( )` inside a method resolve to the class's own `m`.
  - `ZCL_FX_ORDER=>CREATE( )` resolves to `zcl_fx_order.create`, in any case.
  - `super->describe( )` in `zcl_fx_order_sub.describe` resolves to
    `zcl_fx_order.describe`.
  - `zif_fx_order~get_total( )` in `zcl_fx_order.describe` resolves to
    `zcl_fx_order.zif_fx_order~get_total`.
  - `CALL METHOD me->m`, `CALL METHOD zcl_x=>m` and `CALL METHOD lo->m` give the
    same edges as the functional forms.
  - `CALL FUNCTION 'ZFX_FM'` in `zfx_report` resolves to the function module,
    though the quote hides the name from the stripper.
  - `PERFORM show_order` in `zfx_report` resolves to the form. `PERFORM calc_extra`
    in `zfx_fm` resolves to the form in `lzfx_fgf01`, the same function group.
  - `NEW zcl_fx_order( )` gives an edge to the class entity.
- The eight `typeref` edges from methods to their own class's attributes, which
  `sem graph --json` shows today, are still there.
- A call whose receiver has no type yet (`lo_helper->tag( )`) is an unknown with
  the reason `unknown receiver type`, not a guessed edge. Story 2.2 binds it.
- `Stats.unresolved` for the fixture repo names each unknown by reason.
- Non-ABAP `sem graph --json` output is byte-identical to the base commit.
- Fixture tests `abap_fixture_2_1_me_call`, `_static_call`, `_super_call`,
  `_interface_prefixed_call`, `_call_method`, `_call_function`, `_perform`,
  `_new_gives_class_edge` and `_attribute_refs_kept` pass.

## Files

The plan said `LanguageConfig` gains a `scope_resolve` or a `calls` arm. It does
neither. `ScopeResolveConfig` belongs to the JS-style scope resolver, and
Python, Go and Rust all keep `scope_resolve: None`. Calls resolution is a
separate pipeline, registered by file extension. ABAP joins it there.

- `crates/sem-core/src/parser/calls/abap.rs`: new. `pub struct Abap`,
  `pub static ABAP`, `impl Lang for Abap`, `pub fn lower`, `fn layout`. Model it
  on `python.rs` (`impl Lang`, `lower`, `class`, `function`, `call_site`,
  `layout`) and on `go.rs`'s `layout`, which pools every file of a directory.
- `crates/sem-core/src/parser/calls/mod.rs`: `pub mod abap;` and one row in
  `LANGUAGES`: `(&[".abap"], &abap::ABAP)`. Add `"form"` and `"module"` to
  `FN_ENTITY_TYPES`. Today it is `["function", "method"]`, so a `FORM` would
  have no entity id and every `PERFORM` edge would be dropped as `no_entity`.
- `crates/sem-core/src/parser/calls/lang.rs`: three new defaulted methods on
  `Lang`, all `false` for the other languages.
  - `case_insensitive()`: `EntityIds::build` and `find` in `mod.rs` compare
    names byte for byte. With the flag, fold both sides.
  - `replaces_bow()`: see Approach.
  - `Layout` gains `local_home: Vec<Option<usize>>` and `dirs_fall_back: bool`,
    both defaulting to today's behaviour. See Approach.
- `crates/sem-core/src/parser/calls/scope.rs`: `ScopeTables::build` honours the
  two `Layout` additions. This is the only shared-resolver change in the story.
- `crates/sem-core/src/parser/calls/select.rs`: `Resolver::path` and
  `Resolver::pick`. ABAP calls a method with no receiver (`describe( )`), so a
  single-segment path that is not a local falls back to a member of `self_ty`.
  Guard it with a new `Lang::implicit_self()`, default `false`.
- `crates/sem-core/src/parser/graph.rs`: line 1450 skips bag-of-words for files
  `calls::language_for` owns. Change it to skip only when the language also
  `replaces_bow()`. The same predicate goes in `apply_call_edges`'s `owned`.
- `crates/sem-core/src/parser/plugins/code/abap_fallback.rs`: `statements` cuts
  the source into statements with comments and literals blanked. Make it
  `pub(super)` or move it, so the lowering reuses it. Do not write a second cutter.
- `crates/sem-core/tests/fixtures/abap/`: new `zcl_fx_calls.clas.abap` and
  `.clas.xml`, with one method per call form above, including both spellings of
  each. Add rows to the fixture `README.md`.
- `crates/sem-core/src/parser/calls/tests.rs`: lowering unit tests beside the
  Python and Go ones. `mod.rs` holds the `abap_fixture_2_1_*` graph tests.

## Approach

Lower from statements, not from the syntax tree. Facts 3, 9 and 10 in
`docs/abap/README.md` say why: the grammar loses `FORM`, `TYPES` and whole
`METHOD` blocks, and it misreads `'`. Story 1.10 already cuts statements from the
source for that reason. `Lang::lower(tree, src)` receives both. Use `src`, and
leave `tree` unread. Confirm that `strip_abap_content` blanks with spaces and
keeps byte length, because `Site.at` and `Local.at` are byte offsets into `src`.
For `CALL FUNCTION 'ZFOO'`, read the literal from the unblanked `src` at the same
offsets.

A small expression scanner turns each statement into `Expr` nodes. `->` is
`Expr::Method`, or `Field` with no parentheses. `=>` is an `Expr::Path` of two
segments. `~` joins an interface prefix into the method name (`zif_x~m`). A `(`
after a name marks a call. Chains (`a->b( )->c( )`) share their prefix in the
arena as the IR intends. Classic `CALL METHOD x->m EXPORTING ...` lowers to the
same `Expr` as `x->m( )`. A keyword statement such as `PERFORM` or `CALL FUNCTION`
lowers to a `Site` with an `Expr::Call` of a one-segment path.

Fold every name to ASCII lower case when lowering. `Path` and `scope.rs` compare
exact strings and ABAP is case-insensitive. Display is not affected: edges map to
sem entities by row and name through `EntityIds`, which folds under
`case_insensitive()`.

Entities map to declarations like this:

| ABAP | IR |
|------|----|
| global class, local class | `TypeDecl`, plus an inherent `ImplDecl` for its methods |
| `INHERITING FROM zcl_b` | `TypeDecl.embeds` (story 2.3 adds dispatch) |
| `METHODS` implementation | `FnDecl`, `Owner::Impl`, `has_self` true |
| `CLASS-METHODS`, static call target | `FnDecl`, `Owner::Impl`, `has_self` false |
| `FORM`, function module, `MODULE` | `FnDecl`, `Owner::Free` |
| class attribute (`DATA`) | `ValueDecl`, so a bare use gives a `typeref` edge |

`self_value()` is `me`, and `Expr::Super` models `super->`. ABAP has no `Self` type
name, so `self_type()` is empty, as Go's is.

Names, scopes and the pool. Global names are repo-wide. Local ones are per
object. The Layout needs two homes, and it is the one real design question in
this story, so settle it with a one-day spike first. The recommended shape:

- Lowering puts global classes, interfaces, function modules and reports in
  scope 0, and local classes, forms, modules and macros in a child block, scope 1.
- `layout()` builds one root directory and one directory per `AbapObject.name`.
  Every file is `pooled` into the root for scope 0. `local_home[fi]` names the
  object directory that homes the file's scope 1, so the parts of one class
  (`.clas.abap`, `locals_imp`, `testclasses`) share their locals.
- With `dirs_fall_back` set, a directory scope is a lookup block that falls back
  to its parent, so an object's names are seen first and the root's second.

If the spike finds a smaller shape, take it. The rule it must implement is the
table in story 2.0. Do not let a local class become visible outside its object.

Ownership and the seam. Moving ABAP into `LANGUAGES` makes
`apply_call_edges` delete bag-of-words edges into ABAP functions, and
`graph.rs:1450` stop running bag-of-words for ABAP at all. That would turn story
2.0's acceptance case (`io_excel->add_new_worksheet`) into an unknown until story 2.2
types `io_excel`. So ABAP's `replaces_bow()` returns `false` here. The
pipeline's edges are added, bag-of-words keeps running, and the existing
deduplication keeps the pipeline's edge kind where both find one pair. Story 2.2
flips the flag to `true` and deletes the unique-name rule from story 2.0.

Anything the resolver cannot pin returns `Pick::Unknown(reason)` and counts in
`Stats.unresolved`. That is the base for story 2.5. Never add a same-name guess to
`select.rs`.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_1
cargo test -p sem-core calls::
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
# non-ABAP identity, base commit versus this one
SEM_NO_INDEX=1 target/release/sem graph --json > /tmp/graph-after.json
# unknown rate and reasons on both reference repos
cd /tmp/claude-0/abapGit && SEM_CALLS_STATS=1 SEM_CALLS_SITES=/tmp/sites.jsonl SEM_NO_INDEX=1 /path/to/sem graph --json > /dev/null
cd /home/user/abap2xlsx   && SEM_CALLS_STATS=1 SEM_NO_INDEX=1 /path/to/sem graph --json > /dev/null
cargo bench -p sem-core --bench parse_profile -- --baseline before
cargo bench -p sem-core --bench incremental -- --baseline before
```

Record the `calls[.abap]` line (resolved, external and unresolved counts by
reason) for both repos in the commit message. It is the number story 2.2 improves.

## Out of scope

- Receiver types: `DATA lo TYPE REF TO`, `NEW`, `CAST` (story 2.2).
- `INHERITING FROM` dispatch, `INTERFACES` dispatch and `ALIASES` (story 2.3).
- `INCLUDE` joining forms into a program (story 2.4).
- Dynamic calls and how they are reported (story 2.5).
- Macro calls. A statement that starts with a `DEFINE` name is not resolved.

## Estimate

8 to 10 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling. The Layout spike is the risk to
watch.

## Depends on

Story 2.0.
