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
across files, 2.4 includes and function groups, 2.1 static calls, 2.2 receiver
types, 2.3 inheritance and interfaces, 2.6 tests reached.
types, 2.3 inheritance and interfaces, 2.7 chained declarations (T2-C).

| Fixture file | Tests that read it | Story |
|--------------|--------------------|-------|
| `zif_fx_order.intf.abap` | `test_abap_fixture_intf`, `abap_fixture_1_3_interface` | 1.3 |
| | `abap_fixture_2_3_interface_method_entities` (`add_item` and `get_total` are methods of the interface), `_interface_call_dispatches` (the declarations dispatched from) | 2.3 |
| | `abap_fixture_1_2_string_not_reference` (with `zcl_fx_order.clas.abap`) | 1.2 |
| | `abap_fixture_1_1_find_any_case` (with the two class files) | 1.1 |
| `zcl_fx_order.clas.abap` | `test_abap_fixture_clas`, `abap_fixture_1_3_class` | 1.4 |
| | `abap_fixture_1_3_method`, `abap_fixture_1_3_local_classes_attach` (global class attaches nothing) | 1.3 |
| | `abap_fixture_1_2_comment_not_reference`, `abap_fixture_1_2_string_not_reference` | 1.2 |
| | `abap_fixture_1_1_find_any_case`, `abap_fixture_1_1_refs_any_case` | 1.1 |
| | `abap_fixture_1_6_errors_still_yield_entities`, `abap_fixture_1_6_error_count_reported` (broken copies of it) | 1.6, 1.10 |
| | the `abap_fixture_2_0_*` tests (the class and `create` reached from other objects) | 2.0 |
| | `abap_fixture_2_1_interface_prefixed_call` (`zif_fx_order~get_total( )` and `lo_helper->tag( )` in `describe`), `abap_fixture_2_1_attribute_refs_kept` | 2.1 |
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
| | `abap_fixture_2_6_tests_through_setup` (`setup` calls `create`, the two `FOR TESTING` methods reach it through it), `_same_name_other_class_not_listed`, `_index_and_graph_agree`; the `abap_fixture_2_6_*` tests in `crates/sem-cli/tests/abap_tests_reached_cli.rs` | 2.6 |
| `zcl_fx_order_sub.clas.abap` | `test_abap_fixture_clas_sub` | 1.4 |
| | `abap_fixture_2_3_base_call_dispatches` (`describe REDEFINITION`), `abap_fixture_2_3_impact_through_dispatch` | 2.3 |
| | `abap_fixture_1_1_find_any_case` | 1.1 |
| | `abap_fixture_2_0_ambiguous_method_no_edge` (a third `describe`) | 2.0 |
| | `abap_fixture_2_1_super_call` (`super->describe( )`) | 2.1 |
| `zcl_fx_user.clas.abap` | `abap_fixture_2_0_global_class_across_files`, `abap_fixture_2_0_unique_method_across_files`, `abap_fixture_2_0_ambiguous_method_no_edge` (its `describe`) | 2.0 |
| | `abap_fixture_2_1_static_call` (`ZCL_FX_ORDER=>CREATE( 1 )`) | 2.1 |
| `zcl_fx_calls.clas.abap` | the `abap_fixture_2_1_*` tests: one method per static call form, each in its functional and `CALL METHOD` spelling or in both cases, a `NEW`, and receivers typed by their declarations (2.2) | 2.1 |
| `zif_fx_audit.intf.abap` | `abap_fixture_2_3_chained_methods` (`METHODS: audit ..., log ...`), `abap_fixture_2_3_interface_includes_interface` (`INTERFACES zif_fx_order`) | 2.3 |
| `zcl_fx_order_alt.clas.abap` | `abap_fixture_2_3_alias_resolves` (`ALIASES total FOR zif_fx_order~get_total`, called bare and as `me->total( )`), `abap_fixture_2_3_interface_call_dispatches` and `_interface_includes_interface` (it implements `zif_fx_order` through `zif_fx_audit`), `abap_fixture_2_3_impact_through_dispatch` | 2.3 |
| `zcl_fx_dispatch.clas.abap` | the `abap_fixture_2_3_*` tests: one method per call through an interface, a base class, an alias and an interface that includes another | 2.3 |
| `zcl_fx_types.clas.abap` | the `abap_fixture_2_2_*` tests: one method per binding form (an `IMPORTING` parameter, a `CHANGING` one typed through `TYPES`, `RETURNING`, a chain, `CAST`, `CREATE OBJECT`, `NEW #` on an attribute), and one per unknown reason (`REF TO object` and `data`, undeclared, a class outside the repo, a local class of another object) | 2.2 |
| `zcl_fx_order.clas.abap`, `.testclasses`, `zfx_fg.fugr.zfx_fm.abap` | `abap_fixture_2_2_new_binds_local` (`NEW lcl_helper( )`), `_new_hash_takes_target_type` (`create`), `_attribute_type` (`mo_cut`), `_return_type_chain` (`lo_order`) | 2.2 |
| `zcl_fx_other.clas.abap` | `abap_fixture_2_0_local_class_stays_in_object`, `abap_fixture_2_0_ambiguous_method_no_edge`, `abap_fixture_2_0_unique_method_across_files` (`zif_fx_order~get_total`) | 2.0 |
| `zcl_fx_other.clas.locals_imp.abap` | `abap_fixture_2_0_local_class_stays_in_object` (a second `lcl_helper`) | 2.0 |
| `zcl_fx_other.clas.testclasses.abap` | `abap_fixture_2_0_local_friends_does_not_shadow` (`LOCAL FRIENDS`) | 2.0 |
| | `abap_fixture_2_6_same_name_other_class_not_listed` (`label_has_tag` reaches `zcl_fx_order.describe` through `label`, and `create` directly) | 2.6 |
| `zcl_fx_user.clas.testclasses.abap` | the `abap_fixture_2_6_*` tests: a test two calls from `create` (`run_labels_order`), one calling zcl_fx_user's own `describe` that must not count for zcl_fx_order's (`describe_is_user`), one through `zif_fx_order` (`total_through_interface`) and one through a base reference to a redefinition (`describe_through_base`) | 2.6 |
| `zfx_report.prog.abap` | `test_abap_fixture_prog`, `abap_fixture_1_3_report`, `abap_fixture_1_3_form`, `abap_fixture_1_3_macro` | 1.3 |
| | `abap_fixture_1_6_errors_still_yield_entities`, `abap_fixture_1_6_error_count_reported` | 1.6 |
| | `abap_fixture_2_4_forms_do_not_leak` | 2.4 |
| | `abap_fixture_2_1_perform`, `abap_fixture_2_1_call_function`, `abap_fixture_2_1_new_gives_class_edge` (the report's own statements and `show_order`) | 2.1 |
| `zfx_report_f01.prog.abap` | `test_abap_fixture_prog_include`, `abap_fixture_1_3_form` | 1.3 |
| `zfx_dynamic.prog.abap` | `abap_fixture_2_5_*` in `crates/sem-cli/tests/abap_completeness_cli.rs` (the completeness verdict reads its computed calls); `abap_fixture_2_5_dynamic_reasons_counted` for the `sem graph --json` counts | 2.5 |
| | `abap_fixture_2_4_include_joins_forms`, `abap_fixture_2_4_forms_do_not_leak` (the include is shared with `zfx_report2`) | 2.4 |
| `zfx_report2.prog.abap` | `abap_fixture_2_4_include_joins_forms` (`INCLUDE zfx_report_f01`), `abap_fixture_2_4_forms_do_not_leak` | 2.4 |
| `zfx_other.prog.abap` | `abap_fixture_2_4_forms_do_not_leak`, `abap_fixture_2_4_in_program_target`, the same-name `show_order` | 2.4 |
| `zfx_fg.fugr.zfx_fm.abap` | `test_abap_fixture_fugr_function_module`, `abap_fixture_1_3_function` | 1.3 |
| | `abap_fixture_2_4_fugr_perform` | 2.4 |
| | `abap_fixture_2_1_call_function` (the target), `abap_fixture_2_1_perform` (`PERFORM calc_extra`), `abap_fixture_2_1_static_call` | 2.1 |
| `zfx_fg.fugr.saplzfx_fg.abap` | `test_abap_fixture_fugr_main_program` (no entities) | 1.3 |
| | `abap_fixture_2_4_missing_include_recorded` (`lzfx_fguxx` is not in the repo) | 2.4 |
| `zfx_fg.fugr.lzfx_fgtop.abap` | `test_abap_fixture_fugr_top_include` (the one `variable`, `gv_extra`) | 1.3, 2.4 |
| | `abap_fixture_2_4_fugr_global_data`, `abap_fixture_2_4_top_include_data_only_in_top` | 2.4 |
| `zfx_fg.fugr.lzfx_fgf01.abap` | `test_abap_fixture_fugr_form_include`, `abap_fixture_1_3_form` | 1.3 |
| | `abap_fixture_2_1_perform` (the form, in the same function group) | 2.1 |
| `zfx_fg.fugr.lzfx_fgo01.abap` | `test_abap_fixture_fugr_pbo_include`, `abap_fixture_1_3_module` | 1.3 |
| `zcl_fx_chain.clas.abap` | `abap_fixture_2_7_chained_data_one_entity_each` (`DATA: mv_id ..., mv_name ...`), `_chained_class_data` (`CLASS-DATA:` with a `BEGIN OF` block), `_chained_data_member_added` (a copy with one more member), `_chained_types_already_split` (`TYPES: ty_id ..., ty_name ...`) | 2.7 |
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
| `abap_fixture_2_1_incremental_static_call` (a static call's target lost and regained in another file) | 2.1 |
| `abap_fixture_2_1_keyword_is_not_a_unique_name` (`CREATE PUBLIC` against the one `create`, and since 2.2 typed and untyped receivers; also reads the 2.0 fixture objects) | 2.0, 2.2 |
| `abap_fixture_2_2_param_type` (a form's `USING`, a function module's comment-block signature, an interface's and a base class's parameters in another file) | 2.2 |
| `abap_fixture_2_2_untyped_stays_unknown` (a local class's `METHODS` in `locals_def` and its `METHOD` in `locals_imp`; `NEW #( )` with no target) | 2.2 |
| `abap_fixture_2_3_redefinition_pairs` (only `REDEFINITION` overrides: not a method named like a private base method, nor a constructor) | 2.3 |
| `abap_fixture_2_6_tests_through_setup`, second half (all four fixture methods, `CLASS-METHODS` and a chained `METHODS:`, from each `FOR TESTING` method only; a constructor of a class in no file is not unresolved) | 2.6 |
| `abap_fixture_2_7_interface_data` (an interface's `DATA:` chain and `CLASS-DATA`) | 2.7 |
| `abap_fixture_2_7_chained_data_from_the_fallback` (a class the grammar loses whole, cut down from abapGit's `zcl_abapgit_ajson` locals) | 2.7 |
