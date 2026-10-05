# 2.2 Type binding

## Intent

`lo->m( )` is the commonest call in ABAP, and its target depends on the class of
`lo`. Story 2.1 leaves it unknown. ABAP declares that class almost every time:
`DATA lo TYPE REF TO zcl_x`, `NEW zcl_x( )`, `CAST zcl_x( lo )`,
`CREATE OBJECT lo`, or an importing parameter typed `TYPE REF TO`. Bind the
receiver to its class from those declarations, so `lo->m( )` resolves to `zcl_x.m`.
No name guessing: where the type is not written down, the call stays unknown.

## Design decision: needs a human look before code

How receivers bind to classes. This story recommends declared types only,
resolved by the shared `infer.rs`, the way Python annotations work. The options
and the reasoning are in Approach. Read them, and confirm or change the
recommendation, before anyone writes code. The choice sets how many calls
resolve and how often a resolved edge is wrong.

## Acceptance criteria

- On the fixture repo each of these gives a `calls` edge to the named method:
  - `lo_helper->tag( )` in `zcl_fx_order.describe`, where `lo_helper` comes from
    `DATA(lo_helper) = NEW lcl_helper( )`, reaches `lcl_helper.tag` in
    `zcl_fx_order.clas.locals_imp.abap`.
  - `mo_cut->describe( )` in the test class `ltc_order` reaches
    `zcl_fx_order.describe`, through the attribute `DATA mo_cut TYPE REF TO zcl_fx_order`.
  - `lo_order->zif_fx_order~get_total( )` in `zfx_fm` reaches the interface-prefixed
    method, with `lo_order` from `DATA(lo_order) = zcl_fx_order=>create( iv_id )`.
    The type comes from the return type of `create`.
  - The same call through a parameter typed `TYPE REF TO zcl_fx_order`.
  - `CAST zif_fx_order( lo )->get_total( )`, and a variable made by
    `CREATE OBJECT lo` or `CREATE OBJECT lo TYPE zcl_fx_order_sub`.
  - `NEW #( )` takes the declared type of its target, so `ro_order = NEW #( )` in
    `create` gives an edge to the class.
  - A chain `zcl_fx_order=>create( 1 )->describe( )` resolves both links.
- On abap2xlsx, `io_excel->add_new_worksheet( ... )` in
  `zcl_excel_reader_2007.load_workbook` still resolves, now through the declared
  type of `io_excel`, with the unique-name rule from story 2.0 deleted.
- A receiver typed `TYPE REF TO object`, `data` or `any`, or untyped, is an
  unknown with a reason. A class outside the repo (`cl_abap_unit_assert`) is
  `External`, not unknown and not an edge.
- `lcl_helper->tag( )` in an object that has no such local class gives no edge to
  another object's `lcl_helper`.
- `ABAP.replaces_bow()` is `true`. Bag-of-words no longer runs for ABAP files and
  the attribute `typeref` edges from story 2.1 are unchanged.
- The unresolved count on abapGit, from `SEM_CALLS_STATS`, is lower than story
  2.1's, and the commit message gives both.
- Fixture tests `abap_fixture_2_2_new_binds_local`, `_attribute_type`,
  `_return_type_chain`, `_param_type`, `_cast`, `_create_object`,
  `_new_hash_takes_target_type`, `_untyped_stays_unknown`,
  `_external_class_not_unknown` and `_local_class_not_shared` pass.

## Files

- `crates/sem-core/src/parser/calls/abap.rs` (from story 2.1):
  - A `type_expr` that lowers `TYPE REF TO zcl_x` to `TypeExpr::Named`. The IR
    has `TypeExpr::Ref`, transparent for lookup, and ABAP's `REF TO` is the same
    idea. It lowers `object`, `data`, `any`, `simple` and generic types to
    `TypeExpr::Unknown`.
  - Locals: each `DATA lo TYPE REF TO x`, `DATA(lo) = <expr>`, `FINAL(lo) = ...`
    and each signature parameter becomes a `Local { func, name, at, until, ty, init }`.
    `until` is the end of the method, because ABAP has procedure scope and no
    block scope.
  - `TypeDecl.fields` for `DATA` and `CLASS-DATA` in a class definition.
  - `FnDecl.params` and `FnDecl.ret` from `METHODS ... IMPORTING ... RETURNING
    VALUE(r) TYPE REF TO y`.
  - `TYPES ty_ref TYPE REF TO zcl_x` as `TypeKind::Alias`.
  - `function_scoped_names()` returns `true`, as Python's does.
- `crates/sem-core/src/parser/calls/infer.rs`: no change expected. `resolve_type`
  already turns a repo class into `Ty::Adt`, an interface into `Ty::Param`, and an
  outside name into `Ty::Ext`. `type_of`, `variable_type` and `fn_ret` use `Local.ty`,
  `Local.init` and `FnDecl.ret`. Confirm with the tests rather than by reading.
- `crates/sem-core/src/parser/calls/select.rs`: with `implicit_self()` from 2.1,
  a bare `mo_cut` falls back to a field of `self_ty`. That is `Resolver::field_type`.
- `crates/sem-core/src/parser/calls/abap.rs`: `replaces_bow` becomes `true`.
- `crates/sem-core/src/parser/graph.rs`: delete the unique-name rule from story 2.0
  (the function carries a comment saying so) and its table.
- `crates/sem-core/tests/fixtures/abap/`: new `zcl_fx_types.clas.abap` and
  `.clas.xml`, with one method per binding form above and one negative per
  unknown reason. Add rows to the fixture `README.md`.

## Approach

The options for binding a receiver:

1. **Declared types only (recommended).** A receiver has a class when its
   declaration or initialiser says so. That means a `TYPE REF TO` on a variable,
   attribute or parameter, a constructor form (`NEW`, `CREATE OBJECT`, `CAST`)
   as initialiser, or the declared `RETURNING` type of a called method. Nothing
   else. It is exact-or-unknown, the stance of `calls/mod.rs`, and it reuses
   `infer.rs` unchanged.
2. **Flow typing.** Follow assignments (`lo = lo2`) and branches. More calls
   resolve, and each one is a small proof that can be wrong. Python's
   `Ty::union` already exists for the join. Defer until the numbers from option 1
   show it is worth it.
3. **Naming conventions.** `lo_` and `io_` prefixes suggest a reference. This
   is a guess, ABAP shops differ, and it is the opposite of the pipeline's rule.
   Reject.

Two points make option 1 enough in ABAP. Types are declared far more often than
in Python, and `DATA` is procedure-scoped, so there is no shadowing to model.

Interface-typed references. `TYPE REF TO zif_x` resolves to `Ty::Param([trait])`.
A call through it reaches the interface's method declaration. Story 2.3 turns
that into edges to every implementation. Until then the call lands on the
interface entity, as story 2.3 prepares.

`NEW #( )` needs the target's declared type. Lowering finds it from the assignment
target when it is a local or a parameter of the same method, and emits
`Expr::Typed` of that type. For an attribute target it can use the field type,
once the class is known. Otherwise the site is unknown with a reason.

A known gap to record, not hide. When a local class's `METHODS` signature is in
`.clas.locals_def.abap` and its `METHOD` is in `.clas.locals_imp.abap`, the
lowering of the second file does not see the first. The parameters stay untyped
and their calls unknown, with the reason `signature in another file`. abap2xlsx
keeps definition and implementation together in `locals_imp`, so it is rare there.
Count it on abapGit before deciding it needs a cross-file join.

Flipping `replaces_bow()` is the end of the interim from stories 2.0 and 2.1.
Do it in this story's last commit, after the fixture tests pass with it on.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_2
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
cd /home/user/abap2xlsx
SEM_NO_INDEX=1 /path/to/sem find add_new_worksheet --callers --json   # load_workbook in `related`
cd /tmp/claude-0/abapGit
SEM_CALLS_STATS=1 SEM_NO_INDEX=1 /path/to/sem graph --json > /dev/null   # compare unresolved with story 2.1
```

Non-ABAP `sem graph --json` stays byte-identical to the base commit.

## Out of scope

- Flow typing, branches and table element types (`LOOP AT lt INTO lo`).
- Dispatch edges for interface and inherited calls (story 2.3).
- Dynamic type (`CREATE OBJECT lo TYPE (lv_name)`), reported by story 2.5.
- Types from DDIC structures (`TYPE zfx_order`). Not needed for call targets.

## Estimate

5 to 7 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling.

## Depends on

Story 2.1 (the lowering, the layout and the `replaces_bow` seam).
