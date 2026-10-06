# R1 ground truth review

Per-hit decisions behind `readmethod.json`. abapGit at `b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`.

Every call-shaped hit in each target's body (the lines strictly between `METHOD` and `ENDMETHOD`) is listed once, as
`line | decision | reason`, line 1-based in the task's file. A hit is any `name(` followed by a blank, `)` or the line
end, with its receiver chain (`a->`, `b=>`, `c~`), and any `CALL METHOD`, `CALL FUNCTION`, `CREATE OBJECT`, `NEW` or
`PERFORM`, comments and string templates included. `call` lines are the truth: the callee as written, at the line of the
called method's name. A `no` line is a hit that is not a method call under the prompt's rules, with the reason. Every
body was read in full; a bare call was checked against the `METHODS` declarations of the class and its superclasses
(`zcl_abapgit_object_fugr` inherits from `zcl_abapgit_objects_program`, which inherits from `zcl_abapgit_objects_super`).
sem was neither built nor run for any of it.

| Task | Target | Hits | call | no |
|---|---|---:|---:|---:|
| r1_01 | `zcl_abapgit_object_tabl_ddl->set_builtin_type` | 13 | 13 | 0 |
| r1_02 | `zcl_abapgit_object_tabl_ddl->serialize` | 20 | 11 | 9 |
| r1_03 | `zcl_abapgit_object_fugr->zif_abapgit_object~deserialize` | 15 | 15 | 0 |
| r1_04 | `zcl_abapgit_object_fugr->zif_abapgit_object~serialize` | 17 | 17 | 0 |
| r1_05 | `zcl_abapgit_objects=>deserialize_lxe` | 7 | 7 | 0 |
| r1_06 | `zcl_abapgit_objects=>deserialize_step` | 21 | 19 | 2 |
| r1_07 | `zcl_abapgit_objects_program->deserialize_program` | 10 | 10 | 0 |
| r1_08 | `zcl_abapgit_objects_program->serialize_program` | 24 | 23 | 1 |
| total | | 127 | 115 | 12 |

No body contains a dynamic call, a `CALL METHOD`, a `PERFORM`, a `NEW` or a call in a comment. The one function module
(`RPY_PROGRAM_READ`, `r1_08`) and the built-in functions (`to_lower`, `strlen`, `repeat`, `lines`) are the only `no`
lines. Judgement calls a second reader should check:

- `r1_08` line 1381: `CREATE OBJECT li_xml TYPE zcl_abapgit_xml_output` counts as a call of `constructor`; the prompt says so.
- `r1_06` line 870: `zcl_abapgit_objects_activation=>clear( )` is a static method call, not the `CLEAR` statement.
- Chained calls (`zcl_abapgit_factory=>get_sap_report( )->read_progdir( )`, `r1_04`, `r1_05`, `r1_06`, `r1_07`) are two
  calls on one line, the factory method and the method on its result.
- `r1_04` line 1487: `zif_abapgit_object~exists( )` is a call of the class's own implementation of the interface
  method; the scorer reduces it to `exists`.

## r1_01 `zcl_abapgit_object_tabl_ddl->set_builtin_type`, src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1758-1842

1760 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1766 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1776 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1782 | call | set_character_type: `set_character_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1787 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1792 | call | set_numeric_type: `set_numeric_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1800 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1805 | call | set_decfloat_type: `set_decfloat_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1812 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1818 | call | set_integer_type: `set_integer_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1825 | call | set_date_type: `set_date_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1830 | call | set_decfloat_type: `set_decfloat_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
1837 | call | parse_error: `parse_error( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)

## r1_02 `zcl_abapgit_object_tabl_ddl->serialize`, src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:2030-2120

2043 | call | serialize_top: `serialize_top( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2044 | no | to_lower: built-in function or constructor operator `to_lower( )`
2054 | no | strlen: built-in function or constructor operator `strlen( )`
2054 | call | escape_name: `escape_name( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2056 | no | strlen: built-in function or constructor operator `strlen( )`
2056 | call | escape_name: `escape_name( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2067 | call | escape_name: `escape_name( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2069 | call | escape_name: `escape_name( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2071 | no | strlen: built-in function or constructor operator `strlen( )`
2072 | no | repeat: built-in function or constructor operator `repeat( )`
2074 | no | strlen: built-in function or constructor operator `strlen( )`
2085 | no | to_lower: built-in function or constructor operator `to_lower( )`
2088 | no | to_lower: built-in function or constructor operator `to_lower( )`
2090 | no | to_lower: built-in function or constructor operator `to_lower( )`
2092 | call | serialize_extend: `serialize_extend( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2103 | call | serialize_field_annotations: `serialize_field_annotations( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2106 | call | serialize_fkey_annotations: `serialize_fkey_annotations( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2109 | call | serialize_type: `serialize_type( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2111 | call | serialize_field_foreign_key: `serialize_field_foreign_key( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)
2114 | call | serialize_value_help: `serialize_value_help( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited)

## r1_03 `zcl_abapgit_object_fugr->zif_abapgit_object~deserialize`, src/objects/zcl_abapgit_object_fugr.clas.abap:1292-1353

1301 | call | get_abap_version: `get_abap_version( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1303 | call | deserialize_xml: `deserialize_xml( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1309 | call | io_xml->read: `io_xml->read( )`
1312 | call | deserialize_functions: `deserialize_functions( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1319 | call | deserialize_includes: `deserialize_includes( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1324 | call | main_name: `main_name( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1326 | call | mo_i18n_params->is_lxe_applicable: `mo_i18n_params->is_lxe_applicable( )`
1327 | call | deserialize_texts: `deserialize_texts( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1331 | call | io_xml->read: `io_xml->read( )`
1334 | call | deserialize_dynpros: `deserialize_dynpros( )`, a method of zcl_abapgit_objects_program (own or inherited)
1336 | call | io_xml->read: `io_xml->read( )`
1339 | call | deserialize_cua: `deserialize_cua( )`, a method of zcl_abapgit_objects_program (own or inherited)
1342 | call | io_xml->read: `io_xml->read( )`
1345 | call | deserialize_varis: `deserialize_varis( )`, a method of zcl_abapgit_objects_program (own or inherited)
1348 | call | deserialize_function_docs: `deserialize_function_docs( )`, a method of zcl_abapgit_object_fugr (own or inherited)

## r1_04 `zcl_abapgit_object_fugr->zif_abapgit_object~serialize`, src/objects/zcl_abapgit_object_fugr.clas.abap:1474-1528

1487 | call | zif_abapgit_object~exists: `zif_abapgit_object~exists( )`
1491 | call | serialize_xml: `serialize_xml( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1493 | call | serialize_functions: `serialize_functions( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1495 | call | io_xml->add: `io_xml->add( )`
1498 | call | serialize_includes: `serialize_includes( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1500 | call | main_name: `main_name( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1502 | call | zcl_abapgit_factory=>get_sap_report: `zcl_abapgit_factory=>get_sap_report( )`
1502 | call | read_progdir: `read_progdir( )`, chained on the result of the factory call before it on the same line
1504 | call | mo_i18n_params->is_lxe_applicable: `mo_i18n_params->is_lxe_applicable( )`
1505 | call | serialize_texts: `serialize_texts( )`, a method of zcl_abapgit_object_fugr (own or inherited)
1511 | call | serialize_dynpros: `serialize_dynpros( )`, a method of zcl_abapgit_objects_program (own or inherited)
1512 | call | io_xml->add: `io_xml->add( )`
1515 | call | serialize_cua: `serialize_cua( )`, a method of zcl_abapgit_objects_program (own or inherited)
1516 | call | io_xml->add: `io_xml->add( )`
1519 | call | serialize_varis: `serialize_varis( )`, a method of zcl_abapgit_objects_program (own or inherited)
1520 | call | io_xml->add: `io_xml->add( )`
1524 | call | serialize_function_docs: `serialize_function_docs( )`, a method of zcl_abapgit_object_fugr (own or inherited)

## r1_05 `zcl_abapgit_objects=>deserialize_lxe`, src/objects/zcl_abapgit_objects.clas.abap:820-856

828 | call | ii_log->add_success: `ii_log->add_success( )`
833 | call | zcl_abapgit_factory=>get_lxe_texts: `zcl_abapgit_factory=>get_lxe_texts( )`
833 | call | deserialize: `deserialize( )`, a method of zcl_abapgit_object_tabl_ddl (own or inherited), chained on the result of the call before it
842 | call | lo_base->get_accessed_files: `lo_base->get_accessed_files( )`
844 | call | ii_log->add_success: `ii_log->add_success( )`
848 | call | ii_log->add_exception: `ii_log->add_exception( )`
850 | call | ii_log->add_error: `ii_log->add_error( )`

## r1_06 `zcl_abapgit_objects=>deserialize_step`, src/objects/zcl_abapgit_objects.clas.abap:859-942

870 | call | zcl_abapgit_objects_activation=>clear( ): a static method call, not the CLEAR statement
872 | call | ii_log->add_success: `ii_log->add_success( )`
874 | call | zcl_abapgit_progress=>get_instance: `zcl_abapgit_progress=>get_instance( )`
874 | no | lines: built-in function or constructor operator `lines( )`
877 | call | li_progress->show: `li_progress->show( )`
882 | call | zcl_abapgit_factory=>get_cts_api: `zcl_abapgit_factory=>get_cts_api( )`
882 | call | is_object_type_customizing: `is_object_type_customizing( )`, chained on the result of the factory call before it on the same line
889 | call | <ls_obj>-obj->deserialize: `<ls_obj>-obj->deserialize( )`
896 | call | lo_base->get_accessed_files: `lo_base->get_accessed_files( )`
898 | call | ii_log->add_success: `ii_log->add_success( )`
902 | call | ii_log->add_exception: `ii_log->add_exception( )`
904 | call | ii_log->add_error: `ii_log->add_error( )`
910 | call | li_progress->show: `li_progress->show( )`
910 | no | lines: built-in function or constructor operator `lines( )`
915 | call | zcl_abapgit_objects_activation=>activate: `zcl_abapgit_objects_activation=>activate( )`
919 | call | zcl_abapgit_objects_activation=>activate: `zcl_abapgit_objects_activation=>activate( )`
924 | call | zcl_abapgit_objects_activation=>activate: `zcl_abapgit_objects_activation=>activate( )`
927 | call | zcl_abapgit_objects_activation=>activate: `zcl_abapgit_objects_activation=>activate( )`
932 | call | li_progress->off: `li_progress->off( )`
935 | call | zcl_abapgit_exit=>get_instance: `zcl_abapgit_exit=>get_instance( )`
937 | call | li_exit->deserialize_postprocess: `li_exit->deserialize_postprocess( )`

## r1_07 `zcl_abapgit_objects_program->deserialize_program`, src/objects/zcl_abapgit_objects_program.clas.abap:668-717

674 | call | is_exit_include: `is_exit_include( )`, a method of zcl_abapgit_objects_program (own or inherited)
675 | call | deserialize_exit_include: `deserialize_exit_include( )`, a method of zcl_abapgit_objects_program (own or inherited)
683 | call | zcl_abapgit_factory=>get_cts_api: `zcl_abapgit_factory=>get_cts_api( )`
683 | call | insert_transport_object: `insert_transport_object( )`, chained on the result of the factory call before it on the same line
689 | call | get_program_title: `get_program_title( )`, a method of zcl_abapgit_objects_program (own or inherited)
697 | call | update_program: `update_program( )`, a method of zcl_abapgit_objects_program (own or inherited)
702 | call | insert_program: `insert_program( )`, a method of zcl_abapgit_objects_program (own or inherited)
709 | call | zcl_abapgit_factory=>get_sap_report: `zcl_abapgit_factory=>get_sap_report( )`
709 | call | update_progdir: `update_progdir( )`, chained on the result of the factory call before it on the same line
713 | call | zcl_abapgit_objects_activation=>add: `zcl_abapgit_objects_activation=>add( )`

## r1_08 `zcl_abapgit_objects_program->serialize_program`, src/objects/zcl_abapgit_objects_program.clas.abap:1311-1418

1330 | call | zcl_abapgit_language=>set_current_language: `zcl_abapgit_language=>set_current_language( )`
1332 | no | CALL FUNCTION 'RPY_PROGRAM_READ': function module, not counted
1347 | call | zcl_abapgit_language=>restore_login_language: `zcl_abapgit_language=>restore_login_language( )`
1350 | call | zcl_abapgit_language=>restore_login_language: `zcl_abapgit_language=>restore_login_language( )`
1351 | call | zcx_abapgit_exception=>raise_t100: `zcx_abapgit_exception=>raise_t100( )`
1354 | call | zcl_abapgit_language=>restore_login_language: `zcl_abapgit_language=>restore_login_language( )`
1357 | call | zcl_abapgit_factory=>get_sap_report: `zcl_abapgit_factory=>get_sap_report( )`
1361 | call | li_report->read_progdir: `li_report->read_progdir( )`
1366 | call | li_report->read_report: `li_report->read_report( )`
1372 | call | li_report->read_progdir: `li_report->read_progdir( )`
1376 | call | clear_abap_language_version: `clear_abap_language_version( )`, a method of zcl_abapgit_objects_super (own or inherited)
1381 | call | CREATE OBJECT li_xml TYPE zcl_abapgit_xml_output: object creation, counted as `constructor` as the prompt says
1384 | call | li_xml->add: `li_xml->add( )`
1387 | call | serialize_dynpros: `serialize_dynpros( )`, a method of zcl_abapgit_objects_program (own or inherited)
1388 | call | li_xml->add: `li_xml->add( )`
1391 | call | serialize_cua: `serialize_cua( )`, a method of zcl_abapgit_objects_program (own or inherited)
1392 | call | li_xml->add: `li_xml->add( )`
1395 | call | serialize_varis: `serialize_varis( )`, a method of zcl_abapgit_objects_program (own or inherited)
1396 | call | li_xml->add: `li_xml->add( )`
1405 | call | li_xml->add: `li_xml->add( )`
1406 | call | add_tpool: `add_tpool( )`, a method of zcl_abapgit_objects_program (own or inherited)
1409 | call | io_files->add_xml: `io_files->add_xml( )`
1413 | call | strip_generation_comments: `strip_generation_comments( )`, a method of zcl_abapgit_objects_program (own or inherited)
1415 | call | io_files->add_abap: `io_files->add_abap( )`
