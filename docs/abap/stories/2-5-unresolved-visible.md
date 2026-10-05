# 2.5 Unresolved calls stay visible

## Intent

ABAP makes dynamic calls on purpose: `CALL FUNCTION lv_fm`, `CALL METHOD lo->(lv_name)`,
`PERFORM (lv_form) IN PROGRAM`, `CREATE OBJECT lo TYPE (lv_class)`. A static graph
cannot bind them, and a caller list that silently omits them is worse than none.
Report each as a possible reference with its reason, never drop it, as sem does
for callbacks and registries in other languages. The mechanism is not in the
graph builder. It is in the completeness verdict, and today that verdict does not
understand ABAP.

## Acceptance criteria

- On the fixture repo, `sem find zfx_fm --callers --json` lists `zfx_report` as a
  resolved caller. It lists `zfx_dynamic` under `possible_callers` with kind
  `string_key`, because `'ZFX_FM'` in upper case names the module, and it has
  `incomplete_because` code `dynamic_call`.
- `sem find describe --callers` on `zcl_fx_order` has a reason for
  `CALL METHOD zcl_fx_order=>(lv_meth)`, whose static class is the target's, and
  a count of other dynamic call sites whose receiver is unknown.
- Every dynamic form below produces one site with a named reason, counted in
  `Stats.unresolved`, and none produces an edge: `dynamic method name`,
  `dynamic function name`, `dynamic form name`, `dynamic class name`.
- `lo->m( )` with an untyped `lo` keeps the reason `unknown receiver type`, and
  `PERFORM f IN PROGRAM (lv)` is `dynamic form name`, not `path not found`.
- Mentions are case-insensitive for ABAP: `'ZFX_FM'`, `'zfx_fm'` and `Zfx_Fm`
  are the same name. Comments (`*` in column 1 and `"`) and the literal parts of
  `|...|` are not mentions.
- `lo->describe( )` and `zcl_x=>create( )` classify as member calls, not bare
  calls: `->` and `=>` are receivers in ABAP.
- A target with no dynamic form anywhere in the corpus has no `dynamic_call`
  reason, so verdicts stay quiet where they can.
- Fixture tests `abap_fixture_2_5_dynamic_function_visible`,
  `_dynamic_method_static_class`, `_dynamic_reasons_counted`,
  `_string_key_any_case`, `_comment_is_not_a_mention` and
  `_arrow_is_member_call` pass.

## Files

- `crates/sem-cli/src/commands/completeness.rs`, the file that decides what
  `possible_callers` holds:
  - `family()` maps extensions to a comment and string style, and `.abap` falls
    to `Family::Other`, where `lex` marks every byte as code. Add `Family::Abap`.
    Its classes come from the one ABAP stripper
    (`strip_abap_content`, `pub(crate)` in sem-core), made `pub` and re-exported,
    so there are not two stripping rules to keep in step.
  - `occurrences` matches bytes exactly. For ABAP, fold both sides.
  - `scan_file`'s classifier tests `prev == Some(b'.')` for a member call. Add the
    ABAP receivers `->`, `=>` and the `~` of an interface-prefixed name.
  - `Kind` gains `Dynamic` with words "dynamic call (name computed at run time)",
    and it counts as a caller. `assess` gains the code `dynamic_call`.
  - `dynamic_prefixes` finds Python's `getattr(o, 'p' + x)`. Add
    `dynamic_sites` for ABAP, returning each site as (static class if any, form,
    file, line).
- `crates/sem-cli/src/commands/query.rs`: `caller_verdict`'s `files_containing`
  calls `index::grep::search` with `case_insensitive: false`. Pass `true` when the
  target's file is `.abap`, or an uppercase `'ZFX_FM'` never reaches the scan.
  The cold branch uses `str::contains`, and needs the same fold.
- `crates/sem-cli/src/commands/certify.rs`: its own scan (near lines 1016 and 1025)
  calls `dynamic_prefixes` and `scan_file`. Give it the same ABAP handling.
- `crates/sem-core/src/parser/calls/abap.rs`: the lowering emits a `Site` for each
  dynamic form, with an expression that resolves to `Pick::Unknown` carrying the
  reason string. The reasons are `&'static str`, as in `select.rs`.
- `crates/sem-core/src/parser/plugins/code/mod.rs` or `languages.rs`: only the
  re-export of the stripper.
- `crates/sem-core/tests/fixtures/abap/`: new `zfx_dynamic.prog.abap` and
  `.prog.xml`, with one statement per dynamic form, one static call beside them,
  and a comment and a literal that mention a target. Add a row to the fixture
  `README.md`.

## Approach

There are two layers and each needs ABAP. The graph builder knows which sites it
could not resolve and why, but it keeps only counts (`Stats.unresolved`) and the
`SEM_CALLS_SITES` dump. The user-facing "possible callers" come from a source scan
at query time in `completeness.rs`, matching mentions of the target's name. The
plan's "as sem does for callbacks today" is that scan: a callback passed as a
value is a `value_ref` mention, and a registry key is a `string_key` mention.

So a dynamic call to `ZFX_FM` is found as a string. That is how ABAP code usually
looks: `lv_fm = 'ZFX_FM'` or a `CONSTANTS` with the name, then `CALL FUNCTION lv_fm`.
The literal is a `string_key` mention once `'ZFX_FM'` matches `zfx_fm`, and the
`dynamic_call` reason says that a computed call exists too.

Aggregate the reason, as the `alias` and `decorator` reasons are aggregated. A
dynamic site that could reach any method is noise on every target. Use this rule:

- `zcl_x=>(lv)` and `CALL METHOD zcl_x=>(lv)` give a reason on targets of `zcl_x`
  and its subclasses, naming the site.
- `CALL FUNCTION lv` gives a reason on function module targets, with the count
  and the first site.
- `lo->(lv)` with `lo` of a known class behaves like the static-class form. With
  `lo` unknown it adds to a count shown in `checked`, and no reason is added.
- A target with no dynamic form in the corpus gets none.

The unresolved-site list should also reach the user directly. Add the per-reason
counts to the `sem graph --json` stats block next to the existing numbers, so
"how much of this repo does the graph not see" has an answer without an env var.
Keep the change small: the counts already exist in `Stats`.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_5
cargo test -p sem-cli completeness
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
cd crates/sem-core/tests/fixtures/abap
SEM_NO_INDEX=1 /path/to/sem find zfx_fm --callers --json --no-default-excludes
SEM_NO_INDEX=1 /path/to/sem find describe --callers --no-default-excludes
SEM_CALLS_STATS=1 SEM_NO_INDEX=1 /path/to/sem graph --json --no-default-excludes > /dev/null
```

On abap2xlsx, run `sem find add_new_worksheet --callers` and read the verdict. It
must not claim "complete" while a dynamic form exists that could reach the method.

## Out of scope

- Resolving a dynamic call by tracing the string into the call (constant
  propagation). The scan reports it. It does not bind it.
- Dynamic SQL, `ASSIGN (lv) TO <fs>` for data, and `SUBMIT (lv)`.
- Changing the verdict for non-ABAP languages. Their output stays byte-identical.

## Estimate

1 to 2 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling.

## Depends on

Story 2.0, and story 2.1 for the reason strings and the lowering skeleton. The
`completeness.rs` half can start at once, since it reads text and not the graph.
