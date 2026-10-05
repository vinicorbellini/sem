# 2.4 Includes and function groups

## Intent

ABAP programs are compiled from their includes. `INCLUDE zfx_report_f01.` pastes
that file's `FORM`s into `zfx_report`, so a `PERFORM format_total` in the report
reaches a form that lives in another file. A function group is the same idea with
a fixed shape: its function modules see the group's `TOP` include globals and
every `FORM` in its includes. Story 2.1 resolves a form only inside one object.
This story makes the compiled unit, a program with its includes or a function
group, the scope for forms and globals.

## Acceptance criteria

- `PERFORM format_total` in the new `zfx_report2.prog.abap` resolves to the form
  in `zfx_report_f01.prog.abap`, which `zfx_report2` includes.
- A form in an include sees the main program's forms and its sibling includes'.
- Two programs that do not include each other keep their forms apart. A
  `PERFORM show_order` in `zfx_other` resolves to `zfx_other`'s own form and not
  to `zfx_report`'s.
- `PERFORM show_order IN PROGRAM zfx_report` in `zfx_other` resolves to
  `zfx_report`'s form, and never to `zfx_other`'s own, which ABAP would not call.
  `PERFORM f IN PROGRAM (lv_name)` is not resolved (story 2.5 reports it).
- In the function group `zfx_fg`, `PERFORM calc_extra` in the module `zfx_fm`
  reaches the form in `lzfx_fgf01`, and `calc_extra`'s use of `gv_extra` gives a
  `typeref` edge to the `DATA gv_extra` of `lzfx_fgtop`.
- `INCLUDE lzfx_fguxx.` names a file that is not in the repo. That is recorded as
  an unresolved include with a reason, not dropped and not an error. abapGit does
  not serialise the generated `uxx` include, so this is the normal case.
- `INCLUDE STRUCTURE zfx_order` and `INCLUDE TYPE x` inside `DATA` and `TYPES`
  are not program includes and give no include link.
- Fixture tests `abap_fixture_2_4_include_joins_forms`,
  `_forms_do_not_leak`, `_in_program_target`, `_fugr_perform`,
  `_fugr_global_data`, `_missing_include_recorded` and
  `_include_structure_is_not_include` pass.

## Files

- `crates/sem-core/src/parser/calls/ir.rs`: `FileFacts` gains
  `includes: Vec<Name>`. `FileFacts` derives `Default`, so no constructor in
  `rust.rs`, `go.rs` or `python.rs` changes.
- `crates/sem-core/src/parser/calls/abap.rs`: the lowering fills `includes` from
  each `INCLUDE x.` statement, read off the statement stream and never from
  `INCLUDE STRUCTURE` or `INCLUDE TYPE`. `layout()` uses it, see Approach.
  `PERFORM f IN PROGRAM x` and the old `PERFORM f(x)` lower to a two-segment path
  `x`, `f`, so the resolver looks in program `x` only.
- `crates/sem-core/src/parser/plugins/code/abap_name.rs`: `parse_abapgit_name`
  already returns `object_type` and `part`. Add a helper that maps an include
  name to the files that can satisfy it: the program `<name>.prog.abap`, or a
  function group part `<group>.fugr.<name>.abap`.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: top-level `DATA`
  in a `TOP` include becomes a `variable` entity. `is_abap_local_data` keeps every
  `DATA` outside a class section from being an entity on purpose, so it needs an
  exception for these files. Today `test_abap_fixture_fugr_top_include` expects
  none. Without an entity, `calc_extra`'s use of `gv_extra` has nothing to point to.
- `crates/sem-core/src/parser/calls/mod.rs`: nothing new if the unresolved
  include rides on `Stats.unresolved`, as the 2.5 reasons do. Add the reason
  `include not in repo` there.
- `crates/sem-core/tests/fixtures/abap/`: new `zfx_report2.prog.abap`,
  `zfx_other.prog.abap` and their `.xml` envelopes. Do not edit
  `zfx_report.prog.abap`: the `abap_fixture_1_3_*` tests assert its line ranges.
  `zfx_fg.fugr.saplzfx_fg.abap` already includes the missing `lzfx_fguxx` and
  covers the unresolved case. Add rows to the fixture `README.md`.

## Approach

Story 2.1 gives each abapGit object, by `AbapObject.name`, its own directory
scope, with the root above it. That already covers a function group, whose
modules and includes are all parts of one object `zfx_fg`, so `PERFORM calc_extra`
needs nothing from `INCLUDE`. The generated `uxx` include is not in abapGit, so
the include chain could not be followed anyway.

A program include is different. `zfx_report_f01.prog.abap` is its own object, and
only the `INCLUDE` statement links it to `zfx_report`. So `layout()` widens the
rule from "one directory per object name" to "one directory per compiled unit":

1. Collect every `INCLUDE` name from `FileFacts.includes`.
2. Resolve each to files with the helper in `abap_name.rs`, folding case.
3. Union the including file's object with each included file's object
   (union-find). The component, not the object, is the directory.
4. Files whose include is not in the repo contribute an unresolved-include row.

This needs no change to `scope.rs` beyond story 2.1's `Layout` additions, and it
makes visibility symmetric, which compilation is: the include sees the main
program, the main program sees the include, and siblings see each other.

The known imprecision is an include shared by two programs. The union merges
both programs into one directory, so each sees the other's forms. Shared includes
are rare, and the unit is documented as "may over-reach". Count them on abapGit
before the story closes and record the number.

Global data. A function group's `TOP` include declares `DATA gv_extra`, and every
function module and form of the group reads it. With the entity added and the
group in one directory, a bare `gv_extra` in `calc_extra` resolves as a `Value`,
like a class attribute does in story 2.1. Limit the new `variable` entities to
`TOP` includes (`<group>.fugr.l<group>top.abap` and `.prog.abap` files named `*top`)
so the entity lists of existing programs stay as they are. If the implementer
finds that too narrow, widen it deliberately and update `test_abap_fixture_prog`
in the same commit.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_4
cargo test -p sem-core abap_fixture_1_3          # entity lists and ranges still hold
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
cd crates/sem-core/tests/fixtures/abap
SEM_NO_INDEX=1 /path/to/sem find format_total --callers --no-default-excludes
SEM_CALLS_STATS=1 SEM_NO_INDEX=1 /path/to/sem graph --json --no-default-excludes > /dev/null
```

Run `SEM_CALLS_STATS=1` on abapGit and abap2xlsx too. Record how many `INCLUDE`
statements resolve, how many name a file not in the repo, and how many includes
are shared.

## Out of scope

- `INCLUDE` of `<icon>`, `<symbol>` and other system includes.
- Report events as entities, and `START-OF-SELECTION` as an entry point.
- `SUBMIT zprog` and `CALL TRANSACTION`, which are cross-program calls by name.
- Reporting a dynamic `PERFORM` (story 2.5).
- Enhancement implicit includes.

## Estimate

3 to 4 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling.

## Depends on

Story 2.1 (the object directories and the `Layout` additions). It starts in
parallel with 2.1 and 2.5, once 2.0 has merged, and merges after 2.1 lands the skeleton.
