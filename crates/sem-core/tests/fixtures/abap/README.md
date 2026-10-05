# ABAP fixture repository

A small abapGit-layout repository. Every file stays under 60 lines, is named
`<name>.<type>[.<part>].<ext>`, and the global `clas`, `intf` and `prog` objects
have their `.xml` envelope next to the source. The tests read it by path from
`crates/sem-core/src/parser/plugins/code/mod.rs` (the `abap_fixture_*` helpers).
`inside-sap/` is a separate drop zone for files from a real SAP system; no test
reads it yet.

Run them with `cd crates && cargo test -p sem-core abap`. No ABAP test is
`#[ignore]`d.

## Fixture file to tests to story

Story 1.1 is case-insensitive names, 1.2 comment and string stripping, 1.3 the
full entity set, 1.4 one entity per class, 1.5 abapGit layout, 1.6 parse-error
tolerance, 1.7 test detection, 1.10 METHOD blocks the grammar loses, 2.0 names
across files, 2.4 includes and function groups.

| Fixture file | Tests that read it | Story |
|--------------|--------------------|-------|
| `zif_fx_order.intf.abap` | `test_abap_fixture_intf`, `abap_fixture_1_3_interface` | 1.3 |
| | `abap_fixture_1_2_string_not_reference` (with `zcl_fx_order.clas.abap`) | 1.2 |
| | `abap_fixture_1_1_find_any_case` (with the two class files) | 1.1 |
| `zcl_fx_order.clas.abap` | `test_abap_fixture_clas`, `abap_fixture_1_3_class` | 1.4 |
| | `abap_fixture_1_3_method`, `abap_fixture_1_3_local_classes_attach` (global class attaches nothing) | 1.3 |
| | `abap_fixture_1_2_comment_not_reference`, `abap_fixture_1_2_string_not_reference` | 1.2 |
| | `abap_fixture_1_1_find_any_case`, `abap_fixture_1_1_refs_any_case` | 1.1 |
| | `abap_fixture_1_6_errors_still_yield_entities`, `abap_fixture_1_6_error_count_reported` (broken copies of it) | 1.6, 1.10 |
| | the `abap_fixture_2_0_*` tests (the class and `create` reached from other objects) | 2.0 |
| `zcl_fx_order.clas.locals_def.abap` | `test_abap_fixture_clas_locals_def`, `abap_fixture_1_3_local_classes_attach` | 1.3 |
| | `abap_fixture_2_0_local_class_stays_in_object` | 2.0 |
| `zcl_fx_order.clas.locals_imp.abap` | `test_abap_fixture_clas_locals_imp` | 1.4 |
| | `abap_fixture_1_3_local_classes_attach` | 1.3 |
| | `abap_fixture_2_0_local_class_stays_in_object` | 2.0 |
| `zcl_fx_order.clas.testclasses.abap` | `test_abap_fixture_clas_testclasses` | 1.4 |
| | `test_abap_fixture_clas_testclasses_detection` | 1.7 |
| | `abap_fixture_1_3_local_classes_attach` | 1.3 |
| | `abap_fixture_1_6_errors_still_yield_entities` | 1.6 |
| | `abap_fixture_2_0_unique_method_across_files`, `abap_fixture_2_0_ambiguous_method_no_edge` (same-object names) | 2.0 |
| `zcl_fx_order_sub.clas.abap` | `test_abap_fixture_clas_sub` | 1.4 |
| | `abap_fixture_1_1_find_any_case` | 1.1 |
| | `abap_fixture_2_0_ambiguous_method_no_edge` (a third `describe`) | 2.0 |
| `zcl_fx_user.clas.abap` | `abap_fixture_2_0_global_class_across_files`, `abap_fixture_2_0_unique_method_across_files`, `abap_fixture_2_0_ambiguous_method_no_edge` (its `describe`) | 2.0 |
| `zcl_fx_other.clas.abap` | `abap_fixture_2_0_local_class_stays_in_object`, `abap_fixture_2_0_ambiguous_method_no_edge`, `abap_fixture_2_0_unique_method_across_files` (`zif_fx_order~get_total`) | 2.0 |
| `zcl_fx_other.clas.locals_imp.abap` | `abap_fixture_2_0_local_class_stays_in_object` (a second `lcl_helper`) | 2.0 |
| `zcl_fx_other.clas.testclasses.abap` | `abap_fixture_2_0_local_friends_does_not_shadow` (`LOCAL FRIENDS`) | 2.0 |
| `zfx_report.prog.abap` | `test_abap_fixture_prog`, `abap_fixture_1_3_report`, `abap_fixture_1_3_form`, `abap_fixture_1_3_macro` | 1.3 |
| | `abap_fixture_1_6_errors_still_yield_entities`, `abap_fixture_1_6_error_count_reported` | 1.6 |
| | `abap_fixture_2_4_forms_do_not_leak` | 2.4 |
| `zfx_report_f01.prog.abap` | `test_abap_fixture_prog_include`, `abap_fixture_1_3_form` | 1.3 |
| `zfx_dynamic.prog.abap` | `abap_fixture_2_5_*` in `crates/sem-cli/tests/abap_completeness_cli.rs` (the completeness verdict reads its computed calls); the graph-side tests join once story 2.1's lowering lands | 2.5 |
| | `abap_fixture_2_4_include_joins_forms`, `abap_fixture_2_4_forms_do_not_leak` (the include is shared with `zfx_report2`) | 2.4 |
| `zfx_report2.prog.abap` | `abap_fixture_2_4_include_joins_forms` (`INCLUDE zfx_report_f01`), `abap_fixture_2_4_forms_do_not_leak` | 2.4 |
| `zfx_other.prog.abap` | `abap_fixture_2_4_forms_do_not_leak`, `abap_fixture_2_4_in_program_target` (ignored until story 2.1), the same-name `show_order` | 2.4 |
| `zfx_fg.fugr.zfx_fm.abap` | `test_abap_fixture_fugr_function_module`, `abap_fixture_1_3_function` | 1.3 |
| | `abap_fixture_2_4_fugr_perform` | 2.4 |
| `zfx_fg.fugr.saplzfx_fg.abap` | `test_abap_fixture_fugr_main_program` (no entities) | 1.3 |
| | `abap_fixture_2_4_missing_include_recorded` (`lzfx_fguxx` is not in the repo) | 2.4 |
| `zfx_fg.fugr.lzfx_fgtop.abap` | `test_abap_fixture_fugr_top_include` (the one `variable`, `gv_extra`) | 1.3, 2.4 |
| | `abap_fixture_2_4_fugr_global_data`, `abap_fixture_2_4_top_include_data_only_in_top` | 2.4 |
| `zfx_fg.fugr.lzfx_fgf01.abap` | `test_abap_fixture_fugr_form_include`, `abap_fixture_1_3_form` | 1.3 |
| `zfx_fg.fugr.lzfx_fgo01.abap` | `test_abap_fixture_fugr_pbo_include`, `abap_fixture_1_3_module` | 1.3 |
| every file in this directory | `test_abap_fixture_layout` (envelopes, line cap, name shape) | 1.5 |

The `.xml` files and the `tabl`, `dtel`, `doma` and `ttyp` objects are read only
by `test_abap_fixture_layout`.

## Tests on inline source (no fixture file)

| Test | Story |
|------|-------|
| `test_abap_entity_extraction` | 1.4 |
| `test_abap_class_two_ranges` | 1.4 |
| `abap_fixture_1_3_types_data` (TYPES and DATA in sections, a method, and the top level) | 1.3 |
| `abap_fixture_1_10_unparseable_statement_keeps_later_methods` | 1.10 |
| `abap_fixture_1_10_stretched_method_ends_at_its_endmethod` | 1.10 |
| `abap_fixture_1_10_method_without_endmethod_dropped` (and METHOD in a comment or literal) | 1.10 |
| `abap_fixture_1_10_interface_method_names` | 1.10 |
| `abap_fixture_2_0_incremental_follows_other_files` (definitions gained and lost in other files) | 2.0 |
