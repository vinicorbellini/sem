# B1A ground truth review

Per-hit decisions behind `whereused-ambiguous.json`. abapGit at `b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`, `src/` only.

Every line of `src/**/*.abap` that contains the method name as a whole word, case-insensitive, is listed once,
as `file:line | decision | reason`. `yes` lines are the truth, with the calling method. A `no` line is a hit that does
not call the target: the reason names the receiver's static type, or why the hit is not a call. The receiver's type
was read from its declaration (method DATA, class attributes, the method's parameters, the return type of a
functional call), not from sem. Comment and literal hits were found with the stripper of
`scripts/abap-whereused-grep.py` (`strip_line`), which blanks comments and literal text; a hit inside a literal of a
dynamic `CALL METHOD` is listed with the name it carries. Dynamic calls are never counted, as in `whereused.grep.json`.

| Task | Target | Hits | yes | no |
|---|---|---:|---:|---:|
| b1a_01 | `zif_abapgit_object~is_active` | 278 | 8 | 270 |
| b1a_02 | `zif_abapgit_object~get_metadata` | 287 | 3 | 284 |
| b1a_03 | `zif_abapgit_gui_event_handler~on_event` | 71 | 4 | 67 |
| b1a_04 | `zcl_abapgit_object_pinf->load` | 70 | 3 | 67 |
| b1a_05 | `zcl_abapgit_persistence_db->list` | 116 | 3 | 113 |
| b1a_06 | `zcl_abapgit_gui_page_flow->refresh` | 117 | 5 | 112 |
| b1a_07 | `zcl_abapgit_string_map->clear` | 1033 | 4 | 1029 |
| b1a_08 | `zcl_abapgit_string_map->to_abap` | 88 | 10 | 78 |
| b1a_09 | `zcl_abapgit_syntax_highlighter->parse_line` | 30 | 8 | 22 |
| b1a_10 | `zcl_abapgit_html_form_utils->normalize` | 49 | 21 | 28 |
| b1a_11 | `zcl_abapgit_html_form_utils->validate` | 70 | 27 | 43 |
| b1a_12 | `zcl_abapgit_html_form_utils->is_empty` | 55 | 8 | 47 |
| total | | 2264 | 104 | 2160 |

## How the twelve were picked

The rule is in `tasks/b1a_whereused.json` (`selection`) and in the README ("B1A targets"). This is the walk
through the candidate lists. A rejected pick was read only as far as the post-check needed: its callers below
come from its grep hits, read the same way, but its hits are not listed line by line.

| Stratum | Candidates | k | Index | Candidate | Outcome |
|---|---:|---:|---:|---|---|
| intf | 6 | 2 | 0 | `zif_abapgit_background~run` | rejected: 1 calling method (`zcl_abapgit_background->run`) |
| | | | 1 | `zif_abapgit_object~is_active` | **b1a_01** |
| | | | 2 | `zif_abapgit_object~changed_by` | rejected: 1 (`zcl_abapgit_objects->changed_by`) |
| | | | 3 | `zif_abapgit_object~jump` | rejected: 1 (`zcl_abapgit_objects->jump`) |
| | | | 4 | `zif_abapgit_object~get_metadata` | **b1a_02** |
| | | | 5 | `zif_abapgit_gui_event_handler~on_event` | **b1a_03** (index 4 used) |
| short | 44 | 14 | 0 | `zcl_abapgit_apack_reader->refresh` | rejected: 1 (`zcl_abapgit_repo->zif_abapgit_repo~refresh`) |
| | | | 1 | `zcl_abapgit_default_transport->clear` | rejected: 1 (its own `reset`) |
| | | | 2 | `zcl_abapgit_http_client->close` | rejected: 1 (`zcl_abapgit_git_transport->zif_abapgit_git_transport~branches`) |
| | | | 3 | `zcl_abapgit_http_digest->run` | rejected: 2 (`zcl_abapgit_http->acquire_login_details`, `zcl_abapgit_http_client->set_headers`) |
| | | | 4 | `zcl_abapgit_ajson_utilities->merge` | rejected: 2 (`ltcl_json_utils->json_merge`, `lcl_json_path->deserialize`) |
| | | | 5 to 9 | `zcl_abapgit_object_{iamu,iarp,iasp,iatu,iaxu}->save` | rejected: 1 each (a bare call in its own class) |
| | | | 10 | `zcl_abapgit_object_para->unlock` | rejected: 1 (its own `zif_abapgit_object~delete`) |
| | | | 11 | `zcl_abapgit_object_pinf->load` | **b1a_04** |
| | | | 14 | `zcl_abapgit_object_scp1->load` | skipped: name `load` picked |
| | | | 15, 16, 19 | `zcl_abapgit_object_{sfbf,sfbs,sfsw}->unlock` | rejected: 1 each (its own `zif_abapgit_object~deserialize`) |
| | | | 17, 18 | `zcl_abapgit_object_{sfpf,sfpi}->load` | skipped: name `load` picked |
| | | | 20 to 22 | `zcl_abapgit_object_{sprx,wdca,wdya}->save` | rejected: 1 each |
| | | | 23 | `zcl_abapgit_persistence_db->list` | **b1a_05** |
| | | | 28 | `zcl_abapgit_stage->get_all` | rejected: 10 calling methods, but 12 of 15 call-shaped hits are true (80%, over 75%) |
| | | | 29 | `zcl_abapgit_repo_news->get_log` | rejected: 1 (`zcl_abapgit_gui_chunk_lib->render_news`) |
| | | | 30 | `zcl_abapgit_repo_content_list->list` | rejected: 2 (`zcl_abapgit_gui_page_repo_view->zif_abapgit_gui_renderable~render`, `zcl_abapgit_services_repo->activate_objects`) |
| | | | 31 | `zcl_abapgit_repo_content_list->get_log` | rejected: 1 (`zcl_abapgit_gui_page_repo_view->zif_abapgit_gui_renderable~render`) |
| | | | 32 | `zcl_abapgit_html_parts->clear` | rejected: 2 (`zcl_abapgit_gui->render`, `ltcl_part_collections->test`) |
| | | | 33 | `zcl_abapgit_gui_page_flow->refresh` | **b1a_06** |
| polluted | 7 | 2 | 0 to 2 | `zcl_abapgit_object_{sfbf,sfbs,sfsw}->activate` | rejected: 1 each (its own `zif_abapgit_object~deserialize`) |
| | | | 3, 4 | `zcl_abapgit_objects_super->get_metadata`, `->is_active` | skipped: names picked |
| | | | 5 | `zcl_abapgit_syntax_highlighter->parse_line` | **b1a_09** |
| | | | 6 | `zcl_abapgit_gui_page_codi_base->on_event` | skipped: name picked; stratum out, 2 picks handed on |
| multi | 8 | 2 | 0 | `zcl_abapgit_object_tabl_compar->validate` | rejected: 1 (its own `zif_abapgit_comparator~compare`) |
| | | | 1 | `zcl_abapgit_object_devc->is_empty` | rejected: 1 (its own `zif_abapgit_object~delete`) |
| | | | 2 | `zcl_abapgit_objects_generic->validate` | rejected: 1 (its own `deserialize`) |
| | | | 3 | `zcl_abapgit_dot_abapgit->get_name` | rejected: 1 (`zcl_abapgit_migrations->migrate_offline_repos`) |
| | | | 4 | `zcl_abapgit_html_form_utils->normalize` | **b1a_10** |
| | | | 5 | `zcl_abapgit_html_form_utils->validate` | **b1a_11** |
| | | | 6 | `zcl_abapgit_html_form_utils->is_empty` | **b1a_12** |
| handed 1 (intf out, to short) | | | 34 | `zcl_abapgit_gui_page_flowcons->push` | rejected: 2 (its own `stage_missing_remote`, `stage_only_remote`) |
| | | | 35 | `zcl_abapgit_html_form->icon` | rejected: 1 (`zcl_abapgit_popup_to_confirm->get_form_schema`) |
| | | | 36 | `zcl_abapgit_gui_page_codi_base->jump` | rejected: 1 (its own `handle_navigation`) |
| | | | 37 | `zcl_abapgit_gui_page_runit->run` | rejected: 1 (its own `zif_abapgit_gui_renderable~render`) |
| | | | 38 | `zcl_abapgit_gui_page_diff_base->refresh` | skipped: name `refresh` picked |
| | | | 39 | `zcl_abapgit_gui_page_run_bckg->run` | rejected: 1 (its own `zif_abapgit_gui_renderable~render`) |
| | | | 40 | `zcl_abapgit_popup_to_confirm->close` | rejected: 2 (its own `zif_abapgit_gui_event_handler~on_event`, `lcl_popup_to_confirm->close`) |
| | | | 41 | `zcl_abapgit_string_map->clear` | **b1a_07** |
| handed 2 (multi out at index 7, `zcl_abapgit_string_map->is_empty`, name picked; intf out; to short) | | | 42 | `zcl_abapgit_string_map->to_abap` | **b1a_08** |

Task ids follow the stratum order of the task file (intf, short, polluted, multi), not the order of the walk.
Most rejections are private helpers called from one place in their own class: grep finds those exactly
too, so the post-check's floor of 3 callers is what removes them.


## b1a_01 `zif_abapgit_object~is_active`

Defined in `src/objects/zif_abapgit_object.intf.abap`. The name `is_active` is declared in 5 classes or interfaces: `ltcl_tests`, `zcl_abapgit_objects`, `zcl_abapgit_objects_activation`, `zcl_abapgit_objects_super`, `zif_abapgit_object`.

```
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:429 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:505 | no | METHOD statement, the implementation header, not a call
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:506 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/core/zcl_abapgit_objects_activation.clas.abap:32 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:504 | no | METHOD statement, the implementation header, not a call
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:12 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:24 | no | METHOD statement, the implementation header, not a call
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:33 | no | mo_cut TYPE REF TO zcl_abapgit_objects_activation
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:41 | no | mo_cut TYPE REF TO zcl_abapgit_objects_activation
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:49 | no | mo_cut TYPE REF TO zcl_abapgit_objects_activation
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:56 | no | mo_cut TYPE REF TO zcl_abapgit_objects_activation
src/objects/core/zcl_abapgit_objects_activation.clas.testclasses.abap:64 | no | mo_cut TYPE REF TO zcl_abapgit_objects_activation
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:835 | no | METHOD statement, the implementation header, not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:836 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_acid.clas.abap:166 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_acid.clas.abap:167 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_aifc.clas.abap:508 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_amsd.clas.abap:366 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_amsd.clas.abap:367 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_apis.clas.abap:224 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqbg.clas.abap:122 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqbg.clas.abap:123 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_aqqu.clas.abap:98 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqqu.clas.abap:99 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_aqsg.clas.abap:98 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqsg.clas.abap:99 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_area.clas.abap:173 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_asfc.clas.abap:88 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_asfc.clas.abap:89 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_auth.clas.abap:146 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_auth.clas.abap:147 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_avar.clas.abap:181 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_avar.clas.abap:182 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_avas.clas.abap:196 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_avas.clas.abap:197 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_bdef.clas.abap:556 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_bdef.clas.abap:557 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_char.clas.abap:253 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_char.clas.abap:254 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_chdo.clas.abap:272 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:273 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_clas.clas.abap:1008 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_clas.clas.abap:1009 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_cmod.clas.abap:153 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cmod.clas.abap:154 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_cmpt.clas.abap:168 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:169 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_cus0.clas.abap:139 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cus1.clas.abap:157 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cus2.clas.abap:154 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dcls.clas.abap:186 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dcls.clas.abap:187 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ddls.clas.abap:471 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ddls.clas.abap:472 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ddlx.clas.abap:215 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ddlx.clas.abap:216 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_devc.clas.abap:779 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_devc.clas.abap:780 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_dial.clas.abap:157 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dial.clas.abap:158 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_doct.clas.abap:118 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_doct.clas.abap:119 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_docv.clas.abap:183 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_docv.clas.abap:184 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_doma.clas.abap:507 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_doma.clas.abap:508 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_drul.clas.abap:415 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_drul.clas.abap:416 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_dsys.clas.abap:209 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dsys.clas.abap:210 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_dtdc.clas.abap:444 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dtdc.clas.abap:445 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_dtel.clas.abap:361 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:362 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ecatt_super.clas.abap:582 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ecatt_super.clas.abap:583 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_enhc.clas.abap:174 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enhc.clas.abap:175 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_enho.clas.abap:236 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enho.clas.abap:237 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_enhs.clas.abap:205 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enhs.clas.abap:206 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_enqu.clas.abap:140 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:141 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ensc.clas.abap:176 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ensc.clas.abap:177 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_fdt0.clas.abap:616 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_form.clas.abap:375 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_form.clas.abap:376 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ftgl.clas.abap:168 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ftgl.clas.abap:169 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_fugr.clas.abap:1401 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:1402 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_g4ba.clas.abap:146 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_g4ba.clas.abap:147 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_g4bs.clas.abap:146 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_g4bs.clas.abap:147 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_http.clas.abap:230 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_http.clas.abap:231 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iamu.clas.abap:385 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iamu.clas.abap:386 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iarp.clas.abap:386 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iarp.clas.abap:387 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iasp.clas.abap:360 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iasp.clas.abap:361 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iatu.clas.abap:386 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iatu.clas.abap:387 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iaxu.clas.abap:289 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iaxu.clas.abap:290 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_idoc.clas.abap:291 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_idoc.clas.abap:292 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iext.clas.abap:154 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iext.clas.abap:155 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_intf.clas.abap:750 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_intf.clas.abap:751 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iobj.clas.abap:344 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwmo.clas.abap:130 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwmo.clas.abap:131 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iwom.clas.abap:123 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwom.clas.abap:124 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iwpr.clas.abap:115 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwpr.clas.abap:116 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iwsg.clas.abap:124 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwsg.clas.abap:125 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iwsv.clas.abap:130 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwsv.clas.abap:131 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_iwvb.clas.abap:130 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwvb.clas.abap:131 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_jobd.clas.abap:179 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:180 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_msag.clas.abap:463 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_msag.clas.abap:464 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_nrob.clas.abap:278 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:279 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_nspc.clas.abap:397 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_oa2p.clas.abap:190 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_oa2p.clas.abap:191 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_odso.clas.abap:274 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_odso.clas.abap:288 | no | literal of a dynamic CALL METHOD on an SAP object (lo_odso TYPE REF TO object, line 277) naming IS_ACTIVE; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_object_otgr.clas.abap:232 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:233 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_para.clas.abap:185 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_para.clas.abap:186 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_pdts.clas.testclasses.abap:83 | yes | ltc_smoke_test->run_simple_methods: mo_cut TYPE REF TO zif_abapgit_object (line 42)
src/objects/zcl_abapgit_object_pdxx_super.clas.abap:133 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pers.clas.abap:179 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pers.clas.abap:180 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_pinf.clas.abap:332 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:333 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_prag.clas.abap:119 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_prag.clas.abap:120 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_prog.clas.abap:315 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_prog.clas.abap:316 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_saxx_super.clas.abap:289 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:290 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_scp1.clas.abap:420 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_scp1.clas.abap:421 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_scvi.clas.abap:147 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_scvi.clas.abap:149 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sfbf.clas.abap:45 | yes | zcl_abapgit_object_sfbf->activate: zif_abapgit_object~is_active( ) inside an implementing class
src/objects/zcl_abapgit_object_sfbf.clas.abap:280 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfbf.clas.abap:281 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sfbs.clas.abap:45 | yes | zcl_abapgit_object_sfbs->activate: zif_abapgit_object~is_active( ) inside an implementing class
src/objects/zcl_abapgit_object_sfbs.clas.abap:262 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfbs.clas.abap:263 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sfpf.clas.abap:436 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:437 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sfpi.clas.abap:158 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpi.clas.abap:159 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sfsw.clas.abap:45 | yes | zcl_abapgit_object_sfsw->activate: zif_abapgit_object~is_active( ) inside an implementing class
src/objects/zcl_abapgit_object_sfsw.clas.abap:270 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfsw.clas.abap:271 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_shi3.clas.abap:393 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:394 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_shi5.clas.abap:203 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi5.clas.abap:204 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_shi8.clas.abap:129 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi8.clas.abap:130 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_shlp.clas.abap:202 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shlp.clas.abap:203 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_shma.clas.abap:208 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shma.clas.abap:209 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sicf.clas.abap:610 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:611 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sktd.clas.abap:368 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sktd.clas.abap:369 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sldd.clas.abap:125 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sldd.clas.abap:126 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_smim.clas.abap:432 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_smim.clas.abap:433 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_smtg.clas.abap:384 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_smtg.clas.abap:385 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sobj.clas.abap:225 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sobj.clas.abap:226 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sod1.clas.abap:477 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sod1.clas.abap:478 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sod2.clas.abap:476 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sod2.clas.abap:477 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sots.clas.abap:348 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sots.clas.abap:349 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_splo.clas.abap:97 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_splo.clas.abap:98 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sppf.clas.abap:88 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sppf.clas.abap:89 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sprx.clas.abap:348 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sqsc.clas.abap:291 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sqsc.clas.abap:292 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_srfc.clas.abap:220 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srfc.clas.abap:221 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_srvb.clas.abap:571 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srvb.clas.abap:572 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_srvd.clas.abap:536 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srvd.clas.abap:537 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ssfo.clas.abap:455 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:176 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:177 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_stvi.clas.abap:150 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_stvi.clas.abap:152 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_styl.clas.abap:128 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_styl.clas.abap:129 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sucu.clas.abap:84 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sucu.clas.abap:85 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_susc.clas.abap:207 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_susc.clas.abap:208 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sush.clas.abap:281 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sush.clas.abap:282 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_suso.clas.abap:359 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_suso.clas.abap:360 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sxci.clas.abap:180 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sxci.clas.abap:181 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_sxsd.clas.abap:94 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sxsd.clas.abap:95 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_tobj.clas.abap:261 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_tobj.clas.abap:262 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_tran.clas.abap:883 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_tran.clas.abap:884 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ttyp.clas.abap:153 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ttyp.clas.abap:154 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_type.clas.abap:193 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_type.clas.abap:194 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ucsa.clas.abap:259 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ucsa.clas.abap:260 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_udmo.clas.abap:688 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:689 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_ueno.clas.abap:672 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:673 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_vcls.clas.abap:161 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_view.clas.abap:494 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_view.clas.abap:495 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:343 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:344 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_wapa.clas.abap:596 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:597 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_wdca.clas.abap:362 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdca.clas.abap:363 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_wdcc.clas.abap:332 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:232 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:233 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_wdyn.clas.abap:1078 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:1079 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_webi.clas.abap:482 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_webi.clas.abap:483 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_xinx.clas.abap:357 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_xinx.clas.abap:358 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_object_xslt.clas.abap:111 | yes | zcl_abapgit_object_xslt->zif_abapgit_object~deserialize: zif_abapgit_object~is_active( ) inside an implementing class
src/objects/zcl_abapgit_object_xslt.clas.abap:221 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_xslt.clas.abap:222 | no | bare is_active( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->is_active
src/objects/zcl_abapgit_objects.clas.abap:75 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_objects.clas.abap:1054 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects.clas.abap:1066 | yes | zcl_abapgit_objects->is_active: li_obj TYPE REF TO zif_abapgit_object (line 1056)
src/objects/zcl_abapgit_objects.clas.abap:1314 | yes | zcl_abapgit_objects->serialize: li_obj TYPE REF TO zif_abapgit_object (line 1281)
src/objects/zcl_abapgit_objects.clas.abap:1337 | yes | zcl_abapgit_objects->serialize: li_obj TYPE REF TO zif_abapgit_object (line 1281)
src/objects/zcl_abapgit_objects.clas.testclasses.abap:563 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_bridge.clas.abap:226 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_super.clas.abap:91 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_objects_super.clas.abap:366 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_super.clas.abap:368 | no | static call of zcl_abapgit_objects_activation=>is_active
src/objects/zif_abapgit_object.intf.abap:48 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
```

## b1a_02 `zif_abapgit_object~get_metadata`

Defined in `src/objects/zif_abapgit_object.intf.abap`. The name `get_metadata` is declared in 3 classes or interfaces: `zcl_abapgit_objects_super`, `zif_abapgit_object`, `zif_abapgit_xml_input`.

```
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:5 | no | comment or literal text, not code
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:500 | no | METHOD statement, the implementation header, not a call
src/objects/aff/zcl_abapgit_object_common_aff.clas.abap:501 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/aff/zcl_abapgit_object_eeec.clas.abap:38 | no | literal of a dynamic CALL METHOD naming /IWXBE/IF_EEEC_REG_ADAPTER_WB~GET_METADATA; dynamic calls are not counted, as in whereused.grep.json
src/objects/aff/zcl_abapgit_object_eeec.clas.abap:46 | no | literal of a dynamic CALL METHOD naming /IWXBE/IF_EEEC_REG_ADAPTER_WB~GET_METADATA; dynamic calls are not counted, as in whereused.grep.json
src/objects/aff/zcl_abapgit_object_uist.clas.abap:71 | no | literal of a dynamic CALL METHOD naming /UI2/IF_UIST_SVAL~GET_METADATA; dynamic calls are not counted, as in whereused.grep.json
src/objects/core/zcl_abapgit_objects_files.clas.testclasses.abap:133 | no | read_xml( ) returns zif_abapgit_xml_input
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:830 | no | METHOD statement, the implementation header, not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:831 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_acid.clas.abap:161 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_acid.clas.abap:162 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_aifc.clas.abap:503 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aifc.clas.abap:504 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_amsd.clas.abap:361 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_amsd.clas.abap:362 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_apis.clas.abap:218 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_apis.clas.abap:219 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_aqbg.clas.abap:117 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqbg.clas.abap:118 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_aqqu.clas.abap:93 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqqu.clas.abap:94 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_aqsg.clas.abap:93 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_aqsg.clas.abap:94 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_area.clas.abap:168 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_area.clas.abap:169 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_asfc.clas.abap:83 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_asfc.clas.abap:84 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_auth.clas.abap:141 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_auth.clas.abap:142 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_avar.clas.abap:176 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_avar.clas.abap:177 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_avas.clas.abap:189 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_avas.clas.abap:191 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_bdef.clas.abap:551 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_bdef.clas.abap:552 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_char.clas.abap:248 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_char.clas.abap:249 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_chdo.clas.abap:267 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:268 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_clas.clas.abap:1003 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_clas.clas.abap:1004 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_cmod.clas.abap:148 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cmod.clas.abap:149 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_cmpt.clas.abap:163 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:164 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_cus0.clas.abap:134 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cus0.clas.abap:135 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_cus1.clas.abap:152 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cus1.clas.abap:153 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_cus2.clas.abap:149 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_cus2.clas.abap:150 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_dcls.clas.abap:181 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dcls.clas.abap:182 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ddls.clas.abap:466 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ddls.clas.abap:467 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ddlx.clas.abap:210 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ddlx.clas.abap:211 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_devc.clas.abap:774 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_devc.clas.abap:775 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_dial.clas.abap:150 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dial.clas.abap:152 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_doct.clas.abap:113 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_doct.clas.abap:114 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_docv.clas.abap:178 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_docv.clas.abap:179 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_doma.clas.abap:502 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_doma.clas.abap:503 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_drul.clas.abap:410 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_drul.clas.abap:411 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_dsys.clas.abap:149 | no | io_xml/eo_xml TYPE REF TO zif_abapgit_xml_input: its own get_metadata
src/objects/zcl_abapgit_object_dsys.clas.abap:203 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dsys.clas.abap:204 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_dtdc.clas.abap:439 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dtdc.clas.abap:440 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_dtel.clas.abap:356 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:357 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ecatt_super.clas.abap:577 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ecatt_super.clas.abap:578 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_enhc.clas.abap:169 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enhc.clas.abap:170 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_enho.clas.abap:231 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enho.clas.abap:232 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_enhs.clas.abap:200 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enhs.clas.abap:201 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_enqu.clas.abap:135 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:136 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ensc.clas.abap:171 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ensc.clas.abap:172 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_fdt0.clas.abap:611 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_fdt0.clas.abap:612 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_form.clas.abap:370 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_form.clas.abap:371 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ftgl.clas.abap:163 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ftgl.clas.abap:164 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_fugr.clas.abap:1396 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:1397 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_g4ba.clas.abap:141 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_g4ba.clas.abap:142 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_g4bs.clas.abap:141 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_g4bs.clas.abap:142 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_http.clas.abap:225 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_http.clas.abap:226 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iamu.clas.abap:333 | no | io_xml/eo_xml TYPE REF TO zif_abapgit_xml_input: its own get_metadata
src/objects/zcl_abapgit_object_iamu.clas.abap:379 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iamu.clas.abap:380 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iarp.clas.abap:381 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iarp.clas.abap:382 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iasp.clas.abap:355 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iasp.clas.abap:356 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iatu.clas.abap:381 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iatu.clas.abap:382 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iaxu.clas.abap:284 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iaxu.clas.abap:285 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_idoc.clas.abap:286 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_idoc.clas.abap:287 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iext.clas.abap:149 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iext.clas.abap:150 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_intf.clas.abap:745 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_intf.clas.abap:746 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iobj.clas.abap:339 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iobj.clas.abap:340 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwmo.clas.abap:125 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwmo.clas.abap:126 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwom.clas.abap:118 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwom.clas.abap:119 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwpr.clas.abap:110 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwpr.clas.abap:111 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwsg.clas.abap:119 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwsg.clas.abap:120 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwsv.clas.abap:125 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwsv.clas.abap:126 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_iwvb.clas.abap:125 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_iwvb.clas.abap:126 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_jobd.clas.abap:174 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:175 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_msag.clas.abap:458 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_msag.clas.abap:459 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_nrob.clas.abap:273 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:274 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_nspc.clas.abap:392 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_nspc.clas.abap:393 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_oa2p.clas.abap:185 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_oa2p.clas.abap:186 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_odso.clas.abap:269 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_odso.clas.abap:270 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_otgr.clas.abap:227 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:228 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_para.clas.abap:180 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_para.clas.abap:181 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_pdts.clas.testclasses.abap:82 | yes | ltc_smoke_test->run_simple_methods: mo_cut TYPE REF TO zif_abapgit_object (line 42)
src/objects/zcl_abapgit_object_pdxx_super.clas.abap:128 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pdxx_super.clas.abap:129 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_pers.clas.abap:174 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pers.clas.abap:175 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_pinf.clas.abap:327 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:328 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_prag.clas.abap:114 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_prag.clas.abap:115 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_prog.clas.abap:310 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_prog.clas.abap:311 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_saxx_super.clas.abap:284 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:285 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_scp1.clas.abap:413 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_scp1.clas.abap:415 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_scvi.clas.abap:140 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_scvi.clas.abap:142 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sfbf.clas.abap:275 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfbf.clas.abap:276 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sfbs.clas.abap:257 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfbs.clas.abap:258 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sfpf.clas.abap:431 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:432 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sfpi.clas.abap:153 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpi.clas.abap:154 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sfsw.clas.abap:265 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfsw.clas.abap:266 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_shi3.clas.abap:388 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:389 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_shi5.clas.abap:198 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi5.clas.abap:199 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_shi8.clas.abap:124 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shi8.clas.abap:125 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_shlp.clas.abap:197 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shlp.clas.abap:198 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_shma.clas.abap:201 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_shma.clas.abap:203 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sicf.clas.abap:605 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:606 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sktd.clas.abap:363 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sktd.clas.abap:364 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sldd.clas.abap:120 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sldd.clas.abap:121 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_smim.clas.abap:427 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_smim.clas.abap:428 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_smtg.clas.abap:379 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_smtg.clas.abap:380 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sobj.clas.abap:220 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sobj.clas.abap:221 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sod1.clas.abap:472 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sod1.clas.abap:473 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sod2.clas.abap:471 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sod2.clas.abap:472 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sots.clas.abap:343 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sots.clas.abap:344 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_splo.clas.abap:92 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_splo.clas.abap:93 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sppf.clas.abap:83 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sppf.clas.abap:84 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sprx.clas.abap:343 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sprx.clas.abap:344 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sqsc.clas.abap:286 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sqsc.clas.abap:287 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_srfc.clas.abap:215 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srfc.clas.abap:216 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_srvb.clas.abap:566 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srvb.clas.abap:567 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_srvd.clas.abap:531 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_srvd.clas.abap:532 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ssfo.clas.abap:450 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ssfo.clas.abap:451 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ssst.clas.abap:171 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:172 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_stvi.clas.abap:143 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_stvi.clas.abap:145 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_styl.clas.abap:123 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_styl.clas.abap:124 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sucu.clas.abap:79 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sucu.clas.abap:80 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_susc.clas.abap:202 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_susc.clas.abap:203 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sush.clas.abap:276 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sush.clas.abap:277 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_suso.clas.abap:354 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_suso.clas.abap:355 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sxci.clas.abap:173 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sxci.clas.abap:175 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_sxsd.clas.abap:89 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sxsd.clas.abap:90 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_tobj.clas.abap:256 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_tobj.clas.abap:257 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_tran.clas.abap:878 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_tran.clas.abap:879 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ttyp.clas.abap:148 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ttyp.clas.abap:149 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_type.clas.abap:188 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_type.clas.abap:189 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ucsa.clas.abap:254 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ucsa.clas.abap:255 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_udmo.clas.abap:683 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:684 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_ueno.clas.abap:667 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:668 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_vcls.clas.abap:156 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:157 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_view.clas.abap:489 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_view.clas.abap:490 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:216 | no | io_xml/eo_xml TYPE REF TO zif_abapgit_xml_input: its own get_metadata
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:337 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:338 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_wapa.clas.abap:591 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:592 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_wdca.clas.abap:357 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdca.clas.abap:358 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_wdcc.clas.abap:327 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdcc.clas.abap:328 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_wdya.clas.abap:227 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:228 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_wdyn.clas.abap:1073 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:1074 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_webi.clas.abap:477 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_webi.clas.abap:478 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_xinx.clas.abap:352 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_xinx.clas.abap:353 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_object_xslt.clas.abap:216 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_xslt.clas.abap:217 | no | bare get_metadata( ) in a zcl_abapgit_objects_super subclass: the inherited zcl_abapgit_objects_super->get_metadata
src/objects/zcl_abapgit_objects.clas.abap:1274 | no | io_xml/eo_xml TYPE REF TO zif_abapgit_xml_input: its own get_metadata
src/objects/zcl_abapgit_objects.clas.abap:1330 | yes | zcl_abapgit_objects->serialize: li_obj TYPE REF TO zif_abapgit_object (line 1281)
src/objects/zcl_abapgit_objects.clas.testclasses.abap:551 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_bridge.clas.abap:198 | no | literal of a dynamic CALL METHOD naming ZIF_ABAPGITP_PLUGIN~GET_METADATA; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_objects_bridge.clas.abap:213 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_bridge.clas.abap:217 | no | literal of a dynamic CALL METHOD naming ZIF_ABAPGITP_PLUGIN~GET_METADATA; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_objects_super.clas.abap:28 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_objects_super.clas.abap:352 | no | METHOD statement, the implementation header, not a call
src/objects/zif_abapgit_object.intf.abap:70 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/zcl_abapgit_gui_page_debuginfo.clas.abap:384 | yes | zcl_abapgit_gui_page_debuginfo->render_supported_object_types: li_object TYPE REF TO zif_abapgit_object (line 323)
src/xml/zcl_abapgit_xml_input.clas.abap:54 | no | METHOD statement, the implementation header, not a call
src/xml/zif_abapgit_xml_input.intf.abap:16 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
```

## b1a_03 `zif_abapgit_gui_event_handler~on_event`

Defined in `src/ui/core/zif_abapgit_gui_event_handler.intf.abap`. The name `on_event` is declared in 6 classes or interfaces: `zcl_abapgit_gui`, `zcl_abapgit_gui_page_codi_base`, `zcl_abapgit_html_viewer_gui`, `zif_abapgit_exit`, `zif_abapgit_flow_exit`, `zif_abapgit_gui_event_handler`.

```
src/exits/zcl_abapgit_exit.clas.abap:473 | no | METHOD statement, the implementation header, not a call
src/exits/zcl_abapgit_exit.clas.abap:477 | no | gi_exit is the exit interface (zif_abapgit_exit / zif_abapgit_flow_exit), its own on_event
src/exits/zif_abapgit_exit.intf.abap:177 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/core/zcl_abapgit_serialize.clas.testclasses.abap:261 | no | METHOD statement, the implementation header, not a call
src/test/zcl_abapgit_gui_page_template.clas.abap:69 | no | METHOD statement, the implementation header, not a call
src/ui/core/zcl_abapgit_gui.clas.abap:39 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/core/zcl_abapgit_gui.clas.abap:189 | yes | zcl_abapgit_gui->back_graceful: li_handler TYPE REF TO zif_abapgit_gui_event_handler (line 180)
src/ui/core/zcl_abapgit_gui.clas.abap:264 | no | event handler registration of the class's own on_event (FOR EVENT), not a call
src/ui/core/zcl_abapgit_gui.clas.abap:280 | no | bare on_event( ) in zcl_abapgit_gui: its own zcl_abapgit_gui->on_event
src/ui/core/zcl_abapgit_gui.clas.abap:309 | no | returns zif_abapgit_exit: zif_abapgit_exit~on_event
src/ui/core/zcl_abapgit_gui.clas.abap:313 | yes | zcl_abapgit_gui->handle_action: li_handler TYPE REF TO zif_abapgit_gui_event_handler (line 297)
src/ui/core/zcl_abapgit_gui.clas.abap:463 | no | METHOD statement, the implementation header, not a call
src/ui/core/zcl_abapgit_gui.clas.abap:586 | no | event handler registration of the class's own on_event (FOR EVENT), not a call
src/ui/core/zcl_abapgit_gui_utils.clas.testclasses.abap:14 | no | METHOD statement, the implementation header, not a call
src/ui/core/zcl_abapgit_html_viewer_gui.clas.abap:18 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/core/zcl_abapgit_html_viewer_gui.clas.abap:50 | no | event handler registration of the class's own on_event (FOR EVENT), not a call
src/ui/core/zcl_abapgit_html_viewer_gui.clas.abap:55 | no | METHOD statement, the implementation header, not a call
src/ui/core/zif_abapgit_gui_event_handler.intf.abap:10 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/flow/zcl_abapgit_flow_exit.clas.abap:43 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_flow_exit.clas.abap:47 | no | gi_exit is the exit interface (zif_abapgit_exit / zif_abapgit_flow_exit), its own on_event
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:509 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:564 | no | returns zif_abapgit_flow_exit: zif_abapgit_flow_exit~on_event
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:216 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zif_abapgit_flow_exit.intf.abap:36 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_gui_page.clas.abap:526 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_gui_picklist.clas.abap:230 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_code_insp.clas.abap:189 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_code_insp.clas.abap:253 | no | bare on_event( ) in a zcl_abapgit_gui_page_codi_base subclass: zcl_abapgit_gui_page_codi_base->on_event
src/ui/pages/codi/zcl_abapgit_gui_page_codi_base.clas.abap:24 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_codi_base.clas.abap:365 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_runit.clas.abap:237 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_syntax.clas.abap:85 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_syntax.clas.abap:94 | no | bare on_event( ) in a zcl_abapgit_gui_page_codi_base subclass: zcl_abapgit_gui_page_codi_base->on_event
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:199 | no | METHOD statement, the implementation header, not a call
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:570 | no | METHOD statement, the implementation header, not a call
src/ui/pages/db/zcl_abapgit_gui_page_db_entry.clas.abap:248 | no | METHOD statement, the implementation header, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1346 | no | METHOD statement, the implementation header, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:42 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:641 | no | METHOD statement, the implementation header, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:664 | yes | zcl_abapgit_gui_page_patch->zif_abapgit_gui_event_handler~on_event: super->zif_abapgit_gui_event_handler~on_event( ) in the redefinition
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:231 | no | METHOD statement, the implementation header, not a call
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:338 | no | METHOD statement, the implementation header, not a call
src/ui/pages/dlg/zcl_abapgit_gui_page_cr_repo.clas.abap:173 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_bckg.clas.abap:305 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:342 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_info.clas.abap:573 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:509 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:392 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:919 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:565 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_chg_pckg.clas.abap:472 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:489 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_cpackage.clas.abap:149 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:507 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_debuginfo.clas.abap:463 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:160 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_ex_pckage.clas.abap:136 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_merge.clas.abap:114 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_merge_res.clas.abap:497 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_merge_sel.clas.abap:152 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:211 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_ref_sel.clas.abap:180 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_ref_sel.clas.abap:191 | yes | zcl_abapgit_gui_page_ref_sel->zif_abapgit_gui_event_handler~on_event: mo_picklist TYPE REF TO zcl_abapgit_gui_picklist (an implementer), called as ->zif_abapgit_gui_event_handler~on_event
src/ui/pages/zcl_abapgit_gui_page_ref_sel.clas.abap:225 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:893 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1105 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_run_bckg.clas.abap:78 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:716 | no | METHOD statement, the implementation header, not a call
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:317 | no | METHOD statement, the implementation header, not a call
src/ui/popups/zcl_abapgit_popup_to_confirm.clas.abap:225 | no | METHOD statement, the implementation header, not a call
src/ui/routing/zcl_abapgit_gui_router.clas.abap:799 | no | METHOD statement, the implementation header, not a call
```

## b1a_04 `zcl_abapgit_object_pinf->load`

Defined in `src/objects/zcl_abapgit_object_pinf.clas.abap`. The name `load` is declared in 7 classes or interfaces: `lcl_task_definition`, `zcl_abapgit_login_manager`, `zcl_abapgit_object_pinf`, `zcl_abapgit_object_scp1`, `zcl_abapgit_object_sfpf`, `zcl_abapgit_object_sfpi`, `zcl_abapgit_zip`.

```
src/cts/zcl_abapgit_cts_api.clas.abap:387 | no | comment or literal text, not code
src/http/zcl_abapgit_http.clas.abap:214 | no | static call of zcl_abapgit_login_manager=>load
src/http/zcl_abapgit_login_manager.clas.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/http/zcl_abapgit_login_manager.clas.abap:172 | no | METHOD statement, the implementation header, not a call
src/http/zcl_abapgit_login_manager.clas.testclasses.abap:71 | no | static call of zcl_abapgit_login_manager=>load
src/http/zcl_abapgit_login_manager.clas.testclasses.abap:72 | no | static call of zcl_abapgit_login_manager=>load
src/http/zcl_abapgit_login_manager.clas.testclasses.abap:97 | no | static call of zcl_abapgit_login_manager=>load
src/objects/oo/zcl_abapgit_oo_base.clas.abap:173 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_base.clas.abap:179 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_base.clas.abap:197 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_base.clas.abap:203 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:694 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_iamu.clas.abap:94 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_iamu.clas.abap:107 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_iarp.clas.abap:209 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_iarp.clas.abap:220 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_iasp.clas.abap:188 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_iasp.clas.abap:199 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_iatu.clas.abap:198 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_iatu.clas.abap:210 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_iaxu.clas.abap:139 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_iaxu.clas.abap:153 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_pdts.clas.abap:199 | no | static call of the local lcl_task_definition=>load
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:45 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:72 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:44 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:108 | yes | zcl_abapgit_object_pinf->create_or_load: bare load( ) inside zcl_abapgit_object_pinf
src/objects/zcl_abapgit_object_pinf.clas.abap:135 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:248 | yes | zcl_abapgit_object_pinf->zif_abapgit_object~delete: bare load( ) inside zcl_abapgit_object_pinf
src/objects/zcl_abapgit_object_pinf.clas.abap:369 | yes | zcl_abapgit_object_pinf->zif_abapgit_object~serialize: bare load( ) inside zcl_abapgit_object_pinf
src/objects/zcl_abapgit_object_scp1.clas.abap:42 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_scp1.clas.abap:195 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_scp1.clas.abap:496 | no | bare load( ) in another class: that class's own load
src/objects/zcl_abapgit_object_sfpf.clas.abap:44 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:140 | no | bare load( ) in another class: that class's own load
src/objects/zcl_abapgit_object_sfpf.clas.abap:178 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:186 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_sfpf.clas.abap:188 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfpi.clas.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_sfpi.clas.abap:29 | no | bare load( ) in another class: that class's own load
src/objects/zcl_abapgit_object_sfpi.clas.abap:39 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_sfpi.clas.abap:47 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_sfpi.clas.abap:49 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfpi.clas.abap:80 | no | bare load( ) in another class: that class's own load
src/objects/zcl_abapgit_object_sprx.clas.abap:184 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_ssfo.clas.abap:564 | no | SAP class instance (cl_abap_zip / cl_ssf_fb_smart_form), not in the repository
src/objects/zcl_abapgit_object_ssst.clas.abap:95 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_ucsa.clas.abap:119 | no | literal of a dynamic CALL METHOD naming IF_UCON_SA_PERSIST~LOAD; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_object_ucsa.clas.abap:224 | no | literal of a dynamic CALL METHOD naming IF_UCON_SA_PERSIST~LOAD; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_object_ucsa.clas.abap:303 | no | literal of a dynamic CALL METHOD naming IF_UCON_SA_PERSIST~LOAD; dynamic calls are not counted, as in whereused.grep.json
src/objects/zcl_abapgit_object_wapa.clas.abap:214 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_wapa.clas.abap:327 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_wapa.clas.abap:444 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_wapa.clas.abap:473 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_wapa.clas.abap:482 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_wapa.clas.abap:508 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_wapa.clas.abap:564 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_wapa.clas.abap:637 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_xslt.clas.abap:31 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_xslt.clas.abap:41 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_xslt.clas.abap:78 | no | SAP class (cl_...), not in the repository
src/objects/zcl_abapgit_object_xslt.clas.abap:90 | no | comment or literal text, not code
src/repo/utils/zcl_abapgit_zip.clas.abap:42 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_zip.clas.abap:252 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_zip.clas.abap:323 | no | SAP class instance (cl_abap_zip / cl_ssf_fb_smart_form), not in the repository
src/syntax/zcl_abapgit_syntax_abap.clas.abap:133 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_js.clas.abap:122 | no | comment or literal text, not code
src/ui/core/zcl_abapgit_gui_asset_manager.clas.abap:110 | no | comment or literal text, not code
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:261 | no | SAP class instance (cl_abap_zip / cl_ssf_fb_smart_form), not in the repository
src/ui/routing/zcl_abapgit_gui_router.clas.abap:916 | no | static call of zcl_abapgit_zip=>load
```

## b1a_05 `zcl_abapgit_persistence_db->list`

Defined in `src/persist/zcl_abapgit_persistence_db.clas.abap`. The name `list` is declared in 5 classes or interfaces: `zcl_abapgit_persistence_db`, `zcl_abapgit_repo_content_list`, `zif_abapgit_persist_background`, `zif_abapgit_persist_repo`, `zif_abapgit_repo_srv`.

```
src/background/zcl_abapgit_background.clas.abap:156 | no | get_background( ) returns zif_abapgit_persist_background
src/cts/zcl_abapgit_transport_mass.clas.abap:47 | no | comment or literal text, not code
src/data/zcl_abapgit_data_supporter.clas.abap:52 | no | comment or literal text, not code
src/diff/diff3/zcl_abapgit_diff3.clas.abap:1103 | no | comment or literal text, not code
src/env/zcl_abapgit_environment.clas.abap:110 | no | parameter name list of a function call
src/env/zcl_abapgit_user_record.clas.testclasses.abap:27 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:746 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4472 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_dependencies.clas.abap:374 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_objects_activation.clas.abap:631 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_objects_check.clas.abap:312 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_serialize.clas.abap:560 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_class.clas.abap:773 | no | comment or literal text, not code
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1732 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_i18n_params.clas.abap:229 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_i18n_params.clas.abap:233 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_i18n_params.clas.abap:265 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_i18n_params.clas.abap:269 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:355 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:583 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:591 | no | comment or literal text, not code
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:759 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_avar.clas.abap:93 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_ddls.clas.abap:326 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_fugr.clas.abap:1136 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_ssst.clas.abap:125 | no | comment or literal text, not code
src/objects/zcl_abapgit_objects.clas.abap:985 | no | comment or literal text, not code
src/objects/zcl_abapgit_objects_generic.clas.abap:468 | no | comment or literal text, not code
src/objects/zcl_abapgit_objects_program.clas.abap:490 | no | comment or literal text, not code
src/objects/zcl_abapgit_objects_program.clas.abap:947 | no | comment or literal text, not code
src/persist/zcl_abapgit_migrations.clas.abap:31 | no | get_instance( ) returns zif_abapgit_repo_srv
src/persist/zcl_abapgit_persist_background.clas.abap:93 | no | the class's own interface method zif_...~list
src/persist/zcl_abapgit_persist_background.clas.abap:103 | no | METHOD statement, the implementation header, not a call
src/persist/zcl_abapgit_persistence_db.clas.abap:34 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/persist/zcl_abapgit_persistence_db.clas.abap:154 | no | METHOD statement, the implementation header, not a call
src/persist/zcl_abapgit_persistence_repo.clas.abap:65 | no | comment or literal text, not code
src/persist/zcl_abapgit_persistence_repo.clas.abap:296 | no | METHOD statement, the implementation header, not a call
src/persist/zcl_abapgit_persistence_repo.clas.abap:341 | no | the class's own interface method zif_...~list
src/persist/zif_abapgit_persist_background.intf.abap:19 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/persist/zif_abapgit_persist_repo.intf.abap:28 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/status/zcl_abapgit_status_calc.clas.abap:505 | no | comment or literal text, not code
src/repo/zcl_abapgit_dot_abapgit.clas.abap:347 | no | comment or literal text, not code
src/repo/zcl_abapgit_dot_abapgit.clas.testclasses.abap:50 | no | comment or literal text, not code
src/repo/zcl_abapgit_dot_abapgit.clas.testclasses.abap:60 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo.clas.abap:238 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo.clas.abap:616 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo_checksums.clas.testclasses.abap:268 | no | METHOD statement, the implementation header, not a call
src/repo/zcl_abapgit_repo_content_list.clas.abap:10 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/zcl_abapgit_repo_content_list.clas.abap:267 | no | METHOD statement, the implementation header, not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:179 | no | get_repo( ) returns zif_abapgit_persist_repo
src/repo/zcl_abapgit_repo_srv.clas.abap:405 | no | the class's own interface method zif_...~list
src/repo/zcl_abapgit_repo_srv.clas.abap:436 | no | get_repo( ) returns zif_abapgit_persist_repo
src/repo/zcl_abapgit_repo_srv.clas.abap:477 | no | get_repo( ) returns zif_abapgit_persist_repo
src/repo/zcl_abapgit_repo_srv.clas.abap:510 | no | the class's own interface method zif_...~list
src/repo/zcl_abapgit_repo_srv.clas.abap:535 | no | METHOD statement, the implementation header, not a call
src/repo/zcl_abapgit_repo_srv.clas.testclasses.abap:25 | no | METHOD statement, the implementation header, not a call
src/repo/zif_abapgit_repo_srv.intf.abap:35 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:132 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_css.clas.abap:11 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_css.clas.abap:198 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_css.clas.abap:250 | no | comment or literal text, not code
src/test/zcl_abapgit_gui_page_template.clas.abap:38 | no | comment or literal text, not code
src/test/zcl_abapgit_objects_ci_tests.clas.abap:54 | no | comment or literal text, not code
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:315 | no | comment or literal text, not code
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:754 | no | comment or literal text, not code
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:871 | no | get_instance( ) returns zif_abapgit_repo_srv
src/ui/flow/zcl_abapgit_flow_logic.clas.testclasses.abap:561 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:867 | no | comment or literal text, not code
src/ui/lib/zcl_abapgit_gui_buttons.clas.abap:88 | no | comment or literal text, not code
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:607 | no | comment or literal text, not code
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:881 | no | get_instance( ) returns zif_abapgit_repo_srv
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.testclasses.abap:110 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_gui_picklist.clas.abap:18 | no | comment or literal text, not code
src/ui/pages/codi/zcl_abapgit_gui_page_codi_base.clas.abap:481 | no | comment or literal text, not code
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:150 | yes | zcl_abapgit_gui_page_db->do_backup_db: zcl_abapgit_persistence_db=>get_instance( ) returns zcl_abapgit_persistence_db
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:222 | no | comment or literal text, not code
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:311 | yes | zcl_abapgit_gui_page_db->do_restore_db: zcl_abapgit_persistence_db=>get_instance( ) returns zcl_abapgit_persistence_db
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:650 | yes | zcl_abapgit_gui_page_db->zif_abapgit_gui_renderable~render: zcl_abapgit_persistence_db=>get_instance( ) returns zcl_abapgit_persistence_db
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:654 | no | comment or literal text, not code
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:658 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1276 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1467 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:141 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:177 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:291 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:158 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:279 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:165 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:169 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:198 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:205 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:520 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_debuginfo.clas.abap:336 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:143 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:175 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_merge_res.clas.abap:573 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:372 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:427 | no | get_instance( ) returns zif_abapgit_repo_srv
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:432 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:454 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:1050 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1208 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1355 | no | lo_browser TYPE REF TO zcl_abapgit_repo_content_list
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1371 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1431 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_stage.clas.locals_imp.abap:65 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_stage.clas.testclasses.abap:130 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_tutorial.clas.abap:102 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_tutorial.clas.abap:105 | no | text of a string template begun on an earlier line
src/ui/routing/zcl_abapgit_gui_router.clas.abap:637 | no | get_instance( ) returns zif_abapgit_repo_srv
src/ui/routing/zcl_abapgit_services_repo.clas.abap:164 | no | lo_browser TYPE REF TO zcl_abapgit_repo_content_list
src/ui/routing/zcl_abapgit_services_repo.clas.abap:1053 | no | comment or literal text, not code
src/ui/zcl_abapgit_gui_hotkey_ctl.clas.abap:205 | no | comment or literal text, not code
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:277 | no | comment or literal text, not code
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:402 | no | comment or literal text, not code
src/zabapgit_forms.prog.abap:92 | no | get_instance( ) returns zif_abapgit_repo_srv
```

## b1a_06 `zcl_abapgit_gui_page_flow->refresh`

Defined in `src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap`. The name `refresh` is declared in 7 classes or interfaces: `lcl_object_decision_list`, `zcl_abapgit_apack_reader`, `zcl_abapgit_gui_buttons`, `zcl_abapgit_gui_page_diff_base`, `zcl_abapgit_gui_page_flow`, `zcl_abapgit_services_repo`, `zif_abapgit_repo`.

```
src/apack/zcl_abapgit_apack_reader.clas.abap:46 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/apack/zcl_abapgit_apack_reader.clas.abap:255 | no | METHOD statement, the implementation header, not a call
src/background/zcl_abapgit_background.clas.abap:187 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/repo/zcl_abapgit_repo.clas.abap:22 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/zcl_abapgit_repo.clas.abap:265 | no | bare refresh( ) in zcl_abapgit_repo: alias of zif_abapgit_repo~refresh
src/repo/zcl_abapgit_repo.clas.abap:660 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo.clas.abap:679 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo.clas.abap:790 | no | METHOD statement, the implementation header, not a call
src/repo/zcl_abapgit_repo.clas.abap:803 | no | get_dot_apack( ) returns zcl_abapgit_apack_reader
src/repo/zcl_abapgit_repo_checksums.clas.testclasses.abap:278 | no | METHOD statement, the implementation header, not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:376 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo_srv.clas.abap:694 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/repo/zcl_abapgit_repo_srv.clas.abap:724 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo_srv.clas.abap:725 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/repo/zcl_abapgit_repo_srv.clas.abap:730 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/repo/zif_abapgit_repo.intf.abap:54 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:152 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_js.clas.abap:123 | no | comment or literal text, not code
src/test/zcl_abapgit_gui_page_template.clas.abap:39 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/test/zcl_abapgit_gui_page_template.clas.abap:72 | no | comment or literal text, not code
src/test/zcl_abapgit_gui_page_template.clas.abap:73 | no | constant component c_action-refresh, not a call
src/test/zcl_abapgit_gui_page_template.clas.abap:90 | no | comment or literal text, not code
src/test/zcl_abapgit_gui_page_template.clas.abap:91 | no | constant component c_action-refresh, not a call
src/test/zcl_abapgit_gui_page_template.clas.abap:103 | no | comment or literal text, not code
src/test/zcl_abapgit_gui_page_template.clas.abap:104 | no | constant component c_action-refresh, not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:761 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/flow/zcl_abapgit_flow_logic.clas.testclasses.abap:433 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:26 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:49 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:277 | yes | zcl_abapgit_gui_page_flow->call_pull: bare refresh( ) inside zcl_abapgit_gui_page_flow
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:356 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:360 | yes | zcl_abapgit_gui_page_flow->call_stage_commit: bare refresh( ) inside zcl_abapgit_gui_page_flow
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:385 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:540 | yes | zcl_abapgit_gui_page_flow->zif_abapgit_gui_event_handler~on_event: bare refresh( ) inside zcl_abapgit_gui_page_flow
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:542 | no | constant component c_action-refresh, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:543 | yes | zcl_abapgit_gui_page_flow->zif_abapgit_gui_event_handler~on_event: bare refresh( ) inside zcl_abapgit_gui_page_flow
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:570 | no | structure component, not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:571 | yes | zcl_abapgit_gui_page_flow->zif_abapgit_gui_event_handler~on_event: bare refresh( ) inside zcl_abapgit_gui_page_flow
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:608 | no | comment or literal text, not code
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:609 | no | constant component c_action-refresh, not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:32 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:227 | no | constant component c_action-refresh, not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:245 | no | comment or literal text, not code
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:246 | no | constant component c_action-refresh, not a call
src/ui/flow/zif_abapgit_flow_exit.intf.abap:33 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_gui_buttons.clas.abap:32 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_gui_buttons.clas.abap:80 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_gui_buttons.clas.abap:83 | no | comment or literal text, not code
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.testclasses.abap:240 | no | METHOD statement, the implementation header, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:37 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:207 | no | constant component c_action-refresh, not a call
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:256 | no | comment or literal text, not code
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:257 | no | constant component c_action-refresh, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:40 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:46 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:47 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:50 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:51 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:100 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:752 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:784 | no | METHOD statement, the implementation header, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:810 | no | zcl_abapgit_gui_page_diff_base->refresh (or its redefinition in zcl_abapgit_gui_page_patch)
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:837 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1024 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1389 | no | zcl_abapgit_gui_page_diff_base->refresh (or its redefinition in zcl_abapgit_gui_page_patch)
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1396 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1399 | no | zcl_abapgit_gui_page_diff_base->refresh (or its redefinition in zcl_abapgit_gui_page_patch)
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1413 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1418 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:52 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:462 | no | METHOD statement, the implementation header, not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:468 | no | zcl_abapgit_gui_page_diff_base->refresh (or its redefinition in zcl_abapgit_gui_page_patch)
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:612 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:659 | no | zcl_abapgit_gui_page_diff_base->refresh (or its redefinition in zcl_abapgit_gui_page_patch)
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:684 | no | comment or literal text, not code
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:689 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:931 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.testclasses.abap:206 | no | METHOD statement, the implementation header, not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:442 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:248 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:20 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:216 | no | constant component c_action-refresh, not a call
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:217 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:235 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_pull.clas.abap:236 | no | constant component c_action-refresh, not a call
src/ui/pages/zcl_abapgit_gui_page_ref_sel.clas.abap:174 | no | static call of zcl_abapgit_services_repo=>refresh
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:910 | no | get/reload return zif_abapgit_repo: zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:1019 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:1066 | no | static call of zcl_abapgit_gui_buttons=>refresh
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:517 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1213 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:157 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:159 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:376 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:707 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:755 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:800 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:856 | no | comment or literal text, not code
src/ui/routing/zcl_abapgit_gui_router.clas.abap:693 | no | comment or literal text, not code
src/ui/routing/zcl_abapgit_gui_router.clas.abap:694 | no | static call of zcl_abapgit_services_repo=>refresh
src/ui/routing/zcl_abapgit_gui_router.clas.abap:842 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/routing/zcl_abapgit_gui_router.clas.abap:848 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/routing/zcl_abapgit_gui_router.clas.abap:917 | no | static call of zcl_abapgit_services_repo=>refresh
src/ui/routing/zcl_abapgit_services_git.clas.abap:258 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/routing/zcl_abapgit_services_repo.clas.abap:15 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/routing/zcl_abapgit_services_repo.clas.abap:198 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/routing/zcl_abapgit_services_repo.clas.abap:343 | no | receiver TYPE REF TO zif_abapgit_repo (or a repo class through the interface): zif_abapgit_repo~refresh
src/ui/routing/zcl_abapgit_services_repo.clas.abap:932 | no | METHOD statement, the implementation header, not a call
src/ui/routing/zcl_abapgit_services_repo.clas.abap:934 | no | get/reload return zif_abapgit_repo: zif_abapgit_repo~refresh
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:105 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:121 | no | METHOD statement, the implementation header, not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:128 | no | mo_alv TYPE REF TO cl_salv_table (SAP)
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:408 | no | bare refresh( ) in lcl_object_decision_list: its own refresh
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:412 | no | bare refresh( ) in lcl_object_decision_list: its own refresh
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:416 | no | bare refresh( ) in lcl_object_decision_list: its own refresh
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:420 | no | bare refresh( ) in lcl_object_decision_list: its own refresh
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:673 | no | bare refresh( ) in lcl_object_decision_list: its own refresh
```

## b1a_07 `zcl_abapgit_string_map->clear`

Defined in `src/utils/zcl_abapgit_string_map.clas.abap`. The name `clear` is declared in 9 classes or interfaces: `lcl_nodes_helper`, `lcl_sha1_stack`, `zcl_abapgit_default_transport`, `zcl_abapgit_html_parts`, `zcl_abapgit_login_manager`, `zcl_abapgit_objects_activation`, `zcl_abapgit_string_map`, `zif_abapgit_ajson`, `zif_abapgit_log`.

```
src/apack/zcl_abapgit_apack_helper.clas.abap:145 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:149 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:153 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:160 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:179 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:207 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_helper.clas.abap:249 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_reader.clas.abap:229 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_reader.clas.abap:256 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_writer.clas.abap:51 | no | CLEAR statement (keyword), not a call
src/apack/zcl_abapgit_apack_writer.clas.abap:54 | no | CLEAR statement (keyword), not a call
src/background/zcl_abapgit_background.clas.abap:194 | no | comment or literal text, not code
src/background/zcl_abapgit_background.clas.abap:195 | no | static call of zcl_abapgit_login_manager=>clear
src/background/zcl_abapgit_background_push_au.clas.abap:122 | no | CLEAR statement (keyword), not a call
src/background/zcl_abapgit_background_push_au.clas.abap:133 | no | CLEAR statement (keyword), not a call
src/background/zcl_abapgit_background_push_au.clas.abap:140 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_cts_api.clas.abap:484 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_cts_api.clas.abap:485 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_cts_api.clas.abap:546 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_cts_api.clas.abap:838 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_default_transport.clas.abap:24 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/cts/zcl_abapgit_default_transport.clas.abap:36 | no | METHOD statement, the implementation header, not a call
src/cts/zcl_abapgit_default_transport.clas.abap:117 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_default_transport.clas.abap:158 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_default_transport.clas.abap:164 | no | bare clear( ) in zcl_abapgit_default_transport: its own clear
src/cts/zcl_abapgit_transport.clas.abap:164 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_transport.clas.abap:191 | no | CLEAR statement (keyword), not a call
src/cts/zcl_abapgit_transport_mass.clas.locals_imp.abap:74 | no | CLEAR statement (keyword), not a call
src/data/zcl_abapgit_data_config.clas.abap:80 | no | CLEAR statement (keyword), not a call
src/data/zcl_abapgit_data_config.clas.testclasses.abap:49 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:135 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:197 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:205 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:230 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:235 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:273 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:394 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:444 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:446 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:461 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:462 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:467 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:506 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:529 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:531 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:610 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:650 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:742 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:820 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:852 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:925 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:936 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:964 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.abap:1287 | no | CLEAR statement (keyword), not a call
src/diff/diff3/zcl_abapgit_diff3.clas.testclasses.abap:2712 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.abap:147 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.testclasses.abap:87 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.testclasses.abap:88 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.testclasses.abap:89 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.testclasses.abap:90 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff.clas.testclasses.abap:128 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:57 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:106 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:128 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:130 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:145 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:214 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:224 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_diff3.clas.abap:234 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_std.clas.abap:89 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_std.clas.abap:113 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_std.clas.abap:122 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_std.clas.abap:223 | no | CLEAR statement (keyword), not a call
src/diff/zcl_abapgit_diff_std.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/env/zcl_abapgit_settings.clas.abap:384 | no | CLEAR statement (keyword), not a call
src/env/zcl_abapgit_settings.clas.abap:497 | no | CLEAR statement (keyword), not a call
src/env/zcl_abapgit_user_record.clas.abap:154 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_branch_list.clas.abap:156 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_commit.clas.abap:107 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_commit.clas.abap:112 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_commit.clas.abap:128 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_commit.clas.abap:146 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_commit.clas.abap:245 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_delta.clas.abap:128 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_delta.clas.testclasses.abap:83 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.abap:247 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:45 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:67 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:73 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:95 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:362 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:371 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:378 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:387 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:393 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:402 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:437 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:466 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:575 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_pack.clas.testclasses.abap:576 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:216 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:242 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:458 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:606 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:607 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:689 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:710 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.abap:778 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.testclasses.abap:33 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_porcelain.clas.testclasses.abap:34 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_transport.clas.abap:241 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_transport.clas.abap:408 | no | CLEAR statement (keyword), not a call
src/git/zcl_abapgit_git_transport.clas.abap:443 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib.clas.abap:137 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib.clas.abap:227 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib.clas.abap:287 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib_stream.clas.abap:63 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib_stream.clas.abap:75 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib_stream.clas.abap:110 | no | CLEAR statement (keyword), not a call
src/git/zlib/zcl_abapgit_zlib_stream.clas.testclasses.abap:171 | no | comment or literal text, not code
src/git/zlib/zcl_abapgit_zlib_stream.clas.testclasses.abap:216 | no | comment or literal text, not code
src/http/zcl_abapgit_login_manager.clas.abap:21 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/http/zcl_abapgit_login_manager.clas.abap:107 | no | METHOD statement, the implementation header, not a call
src/http/zcl_abapgit_login_manager.clas.abap:109 | no | CLEAR statement (keyword), not a call
src/http/zcl_abapgit_login_manager.clas.testclasses.abap:20 | no | static call of zcl_abapgit_login_manager=>clear
src/http/zcl_abapgit_login_manager.clas.testclasses.abap:24 | no | static call of zcl_abapgit_login_manager=>clear
src/http/zcx_abapgit_auth_required.clas.abap:79 | no | CLEAR statement (keyword), not a call
src/inspect/zcl_abapgit_where_used_tools.clas.abap:432 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.abap:25 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.abap:290 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.abap:348 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.abap:360 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.abap:363 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.abap:913 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.abap:1002 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:451 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:452 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:910 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:912 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:976 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:1088 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:1862 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:2087 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:2121 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:2122 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:2209 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:11 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:50 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:51 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:120 | no | mo_nodes TYPE REF TO lcl_nodes_helper (local class of the ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:127 | no | mo_nodes TYPE REF TO lcl_nodes_helper (local class of the ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:134 | no | mo_nodes TYPE REF TO lcl_nodes_helper (local class of the ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:141 | no | mo_nodes TYPE REF TO lcl_nodes_helper (local class of the ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1523 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1530 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1993 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2030 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2447 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2867 | no | li_writer TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3258 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3409 | no | li_writer TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3423 | no | li_writer TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3437 | no | li_writer TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3521 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3626 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3768 | no | li_writer TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:3972 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5116 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5184 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:384 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:417 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:466 | no | CLEAR statement (keyword), not a call
src/json/zcl_abapgit_ajson_utilities.clas.testclasses.abap:600 | no | CLEAR statement (keyword), not a call
src/json/zcx_abapgit_ajson_error.clas.abap:82 | no | CLEAR statement (keyword), not a call
src/json/zif_abapgit_ajson.intf.abap:159 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/aff/zcl_abapgit_json_handler.clas.abap:104 | no | CLEAR statement (keyword), not a call
src/objects/aff/zcl_abapgit_json_path.clas.testclasses.abap:81 | no | CLEAR statement (keyword), not a call
src/objects/aff/zcl_abapgit_json_path.clas.testclasses.abap:130 | no | CLEAR statement (keyword), not a call
src/objects/aff/zcl_abapgit_object_aplo.clas.abap:22 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_dependencies.clas.abap:257 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_dependencies.clas.abap:307 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_file_deserialize.clas.abap:181 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_filename_logic.clas.abap:162 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_filename_logic.clas.abap:216 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_filename_logic.clas.abap:226 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_folder_logic.clas.locals_imp.abap:58 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_folder_logic.clas.locals_imp.abap:128 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:26 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:392 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:469 | no | METHOD statement, the implementation header, not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:470 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_activation.clas.abap:471 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_check.clas.abap:365 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_check.clas.abap:384 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_check.clas.abap:531 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_check.clas.abap:587 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_check.clas.abap:692 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_objects_files.clas.abap:500 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:450 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:481 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:485 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:517 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:531 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:534 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.abap:730 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.testclasses.abap:349 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_serialize.clas.testclasses.abap:482 | no | CLEAR statement (keyword), not a call
src/objects/core/zcl_abapgit_tadir.clas.abap:524 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_script_downl.clas.abap:94 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_script_downl.clas.abap:166 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_script_downl.clas.abap:169 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_sp_upload.clas.abap:154 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_sp_upload.clas.abap:180 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_system_downl.clas.abap:94 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:64 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:111 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:159 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:189 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:312 | no | CLEAR statement (keyword), not a call
src/objects/ecatt/zcl_abapgit_ecatt_val_obj_upl.clas.abap:333 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_badi.clas.abap:117 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_badi.clas.abap:122 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:148 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:156 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:166 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:174 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:181 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:190 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_clif.clas.abap:196 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_fugr.clas.abap:126 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_hook.clas.abap:69 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_hook.clas.abap:96 | no | CLEAR statement (keyword), not a call
src/objects/enh/zcl_abapgit_object_enho_hook.clas.abap:297 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_base.clas.abap:164 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_base.clas.abap:188 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_base.clas.abap:212 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_base.clas.abap:240 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_class.clas.abap:875 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_class.clas.abap:876 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_interface.clas.abap:335 | no | CLEAR statement (keyword), not a call
src/objects/oo/zcl_abapgit_oo_interface.clas.abap:336 | no | comment or literal text, not code
src/objects/oo/zcl_abapgit_oo_serializer.clas.abap:176 | no | CLEAR statement (keyword), not a call
src/objects/rules/zcl_abapgit_field_rules.clas.abap:95 | no | CLEAR statement (keyword), not a call
src/objects/sap/zcl_abapgit_sap_package.clas.abap:334 | no | CLEAR statement (keyword), not a call
src/objects/sap/zcl_abapgit_sap_package.clas.abap:338 | no | CLEAR statement (keyword), not a call
src/objects/sap/zcl_abapgit_sap_report.clas.abap:150 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:107 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:116 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:122 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:127 | no | comment or literal text, not code
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:129 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:137 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:153 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:362 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:489 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:909 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:915 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:918 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:921 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:924 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:927 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:933 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:939 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:944 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:950 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:964 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:965 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:974 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:977 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:983 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:984 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:985 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:986 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl.clas.abap:987 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_compar.clas.abap:87 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_compar.clas.abap:116 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:417 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:440 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:707 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:746 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:772 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:848 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:850 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:878 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:932 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:961 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:973 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1033 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1433 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1450 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1511 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1529 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1611 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1626 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:1723 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap:2063 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.locals_imp.abap:27 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:764 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1043 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1057 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1086 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1110 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1116 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1123 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1167 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1179 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1188 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1376 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1390 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1422 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1449 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1603 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1701 | no | CLEAR statement (keyword), not a call
src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap:1702 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_i18n_params.clas.testclasses.abap:72 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_i18n_params.clas.testclasses.abap:132 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_i18n_params.clas.testclasses.abap:150 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_longtexts.clas.abap:108 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_longtexts.clas.abap:125 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_longtexts.clas.abap:444 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:242 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:308 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:358 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:540 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:763 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.abap:864 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.testclasses.abap:96 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_lxe_texts.clas.testclasses.abap:129 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_po_file.clas.abap:209 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_po_file.clas.testclasses.abap:151 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_sotr_handler.clas.abap:117 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_sotr_handler.clas.abap:361 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_sotr_handler.clas.abap:369 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_sots_handler.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/objects/texts/zcl_abapgit_sots_handler.clas.abap:252 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_aifc.clas.abap:385 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_amsd.clas.abap:58 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_avas.clas.abap:50 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_avas.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_avas.clas.abap:248 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_bdef.clas.abap:65 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_bdef.clas.abap:190 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_char.clas.abap:293 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:287 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:293 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:298 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_chdo.clas.abap:359 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_clas.clas.abap:221 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_clas.clas.abap:638 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_clas.clas.abap:735 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_clas.clas.abap:777 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmod.clas.abap:207 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:216 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:220 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:224 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:228 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cmpt.clas.abap:232 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cus0.clas.abap:188 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cus1.clas.abap:218 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_cus2.clas.abap:193 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dcls.clas.abap:246 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ddls.clas.abap:207 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ddls.clas.abap:589 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ddlx.clas.abap:299 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:92 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:95 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:583 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:859 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:862 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:863 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:866 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:867 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:873 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:874 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:878 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:879 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:889 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:896 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:899 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_devc.clas.abap:900 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:910 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:917 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:920 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:954 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:41 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:46 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:51 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:56 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:62 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dial.clas.abap:72 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doct.clas.abap:149 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doct.clas.abap:154 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_docv.clas.abap:214 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:186 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_doma.clas.abap:188 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:314 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_doma.clas.abap:317 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:569 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:576 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:582 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:586 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.abap:596 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.locals_imp.abap:153 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.locals_imp.abap:166 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.testclasses.abap:162 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.testclasses.abap:168 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.testclasses.abap:346 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_doma.clas.testclasses.abap:454 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_drul.clas.abap:60 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtdc.clas.abap:64 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:422 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:427 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_dtel.clas.abap:428 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:441 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:445 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:450 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.abap:453 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.testclasses.abap:449 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.testclasses.abap:473 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.testclasses.abap:475 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_dtel.clas.testclasses.abap:478 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ecatt_super.clas.abap:207 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_enqu.clas.abap:202 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:210 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:236 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:237 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:238 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:239 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:240 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:241 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:242 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:243 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:245 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:246 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:247 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:248 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:249 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:250 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:251 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:252 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:253 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:254 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_enqu.clas.abap:255 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_fdt0.clas.abap:237 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_fdt0.clas.abap:322 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_form.clas.abap:227 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:447 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:448 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:476 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:484 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:498 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_form.clas.abap:562 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ftgl.clas.abap:45 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:942 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:945 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:946 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_fugr.clas.abap:979 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iamu.clas.abap:157 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iamu.clas.abap:427 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iarp.clas.abap:100 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iasp.clas.abap:98 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iatu.clas.abap:71 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iaxu.clas.abap:55 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_idoc.clas.abap:46 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_intf.clas.abap:462 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_intf.clas.locals_imp.abap:149 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_intf.clas.locals_imp.abap:716 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_iobj.clas.abap:56 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:251 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:254 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:257 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:260 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:263 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:266 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:269 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_jobd.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:61 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:219 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:324 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:383 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:511 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_msag.clas.abap:518 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:52 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:298 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:303 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:356 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:360 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:364 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:368 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:372 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:377 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:381 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:385 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:389 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:393 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_nrob.clas.abap:397 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_oa2p.clas.abap:294 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_odso.clas.abap:61 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:185 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_otgr.clas.abap:340 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:365 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:367 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:372 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:374 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:385 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_otgr.clas.abap:390 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:126 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:130 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:135 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:140 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pdts.clas.locals_imp.abap:145 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pers.clas.abap:202 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pers.clas.abap:207 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pers.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:376 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:379 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:389 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:393 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_pinf.clas.abap:401 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:345 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:349 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:353 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:357 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:361 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:365 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:369 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_saxx_super.clas.abap:373 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_scvi.clas.abap:199 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_scvi.clas.abap:200 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sfbf.clas.abap:82 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfbf.clas.abap:96 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfbf.clas.abap:328 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sfbs.clas.abap:82 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfbs.clas.abap:96 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfbs.clas.abap:307 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:220 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:229 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sfpf.clas.abap:483 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfsw.clas.abap:82 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfsw.clas.abap:96 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sfsw.clas.abap:315 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:62 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:63 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:64 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:65 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:68 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:69 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shi3.clas.abap:70 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shlp.clas.abap:264 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shlp.clas.abap:270 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shlp.clas.abap:274 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_shlp.clas.abap:275 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shma.clas.abap:228 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shma.clas.abap:233 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_shma.clas.abap:271 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:373 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:374 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:375 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:376 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:406 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:407 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:408 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:409 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:412 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:416 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:441 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:713 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:714 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:715 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:716 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:717 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:718 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:719 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:720 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:724 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:726 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:728 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_sicf.clas.abap:730 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:733 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:736 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sicf.clas.abap:739 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sktd.clas.abap:55 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_smim.clas.abap:184 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_smim.clas.abap:501 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_smtg.clas.abap:91 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sod1.clas.abap:107 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sod2.clas.abap:107 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sots.clas.abap:160 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sots.clas.abap:176 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sots.clas.abap:184 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sots.clas.abap:260 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sots.clas.abap:391 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_splo.clas.abap:140 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sprx.clas.abap:412 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sprx.clas.abap:422 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_srfc.clas.abap:273 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_srfc.clas.abap:278 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_srfc.clas.abap:283 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_srvb.clas.abap:77 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_srvd.clas.abap:65 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssfo.clas.abap:267 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssfo.clas.abap:275 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:197 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:202 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:266 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:267 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:268 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:269 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:270 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:271 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ssst.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_stvi.clas.abap:203 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_stvi.clas.abap:204 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_styl.clas.abap:148 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_styl.clas.abap:153 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_styl.clas.abap:158 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_styl.clas.abap:163 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_styl.clas.abap:205 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sush.clas.abap:371 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_suso.clas.abap:408 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sxci.clas.abap:281 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_sxsd.clas.abap:183 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tobj.clas.abap:40 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tobj.clas.abap:343 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tobj.clas.abap:370 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_tobj.clas.abap:371 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:163 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_tran.clas.abap:175 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_tran.clas.abap:178 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:194 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:477 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:524 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:547 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:564 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:601 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:612 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_tran.clas.abap:965 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ttyp.clas.abap:217 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ttyp.clas.abap:222 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ttyp.clas.abap:227 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ucsa.clas.abap:84 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:265 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:419 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:420 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:421 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:422 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:423 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:424 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:468 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:469 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:487 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:488 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:489 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:491 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:492 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:493 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:523 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:524 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:525 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:526 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:527 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_udmo.clas.abap:528 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:294 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:300 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:317 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:323 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:577 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:578 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:579 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:580 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:582 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:583 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:584 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_ueno.clas.abap:585 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:211 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:216 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:221 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:226 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:232 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:237 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:295 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_vcls.clas.abap:296 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:261 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:550 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:556 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:559 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:564 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:586 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:588 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:589 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:590 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_view.clas.abap:591 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3ht.clas.abap:21 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3mi.clas.abap:21 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:142 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:144 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:147 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:260 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:374 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:384 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:389 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_w3xx_super.clas.abap:460 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_wapa.clas.abap:261 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:265 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:350 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:373 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wapa.clas.abap:656 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdca.clas.abap:128 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdca.clas.abap:154 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdca.clas.abap:162 | no | comment or literal text, not code
src/objects/zcl_abapgit_object_wdca.clas.abap:165 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:36 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:37 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdya.clas.abap:51 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:431 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:480 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:481 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:670 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:714 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:723 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:774 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:881 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_wdyn.clas.abap:889 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:564 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:565 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:566 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:567 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:568 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:569 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:570 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:571 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:575 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_webi.clas.abap:580 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_xinx.clas.abap:405 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_object_xslt.clas.abap:257 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:528 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:533 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:637 | no | static call of zcl_abapgit_objects_activation=>clear
src/objects/zcl_abapgit_objects.clas.abap:692 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:693 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:870 | no | static call of zcl_abapgit_objects_activation=>clear
src/objects/zcl_abapgit_objects.clas.abap:1255 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:1366 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.abap:1421 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects.clas.testclasses.abap:42 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_bridge.clas.abap:97 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:380 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:405 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:514 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:515 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:543 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:558 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:564 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:571 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:604 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:690 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:698 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:790 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:841 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:869 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:914 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:936 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:944 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1252 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1266 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1273 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1279 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1282 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1298 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1433 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.abap:1444 | no | comment or literal text, not code
src/objects/zcl_abapgit_objects_program.clas.abap:1446 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_program.clas.testclasses.abap:63 | no | CLEAR statement (keyword), not a call
src/objects/zcl_abapgit_objects_super.clas.abap:135 | no | CLEAR statement (keyword), not a call
src/persist/zcl_abapgit_persist_migrate.clas.locals_imp.abap:53 | no | static call of zcl_abapgit_objects_activation=>clear
src/persist/zcl_abapgit_persistence_user.clas.abap:289 | no | CLEAR statement (keyword), not a call
src/persist/zcl_abapgit_persistence_user.clas.testclasses.abap:120 | no | CLEAR statement (keyword), not a call
src/persist/zcl_abapgit_persistence_user.clas.testclasses.abap:144 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.abap:97 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.abap:141 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.abap:175 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.abap:176 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.abap:177 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_object_filter_tran.clas.testclasses.abap:29 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_repo_filter.clas.abap:84 | no | CLEAR statement (keyword), not a call
src/repo/filter/zcl_abapgit_repo_filter.clas.abap:85 | no | CLEAR statement (keyword), not a call
src/repo/stage/zcl_abapgit_merge.clas.abap:407 | no | CLEAR statement (keyword), not a call
src/repo/stage/zcl_abapgit_merge.clas.abap:420 | no | CLEAR statement (keyword), not a call
src/repo/status/zcl_abapgit_status_calc.clas.abap:225 | no | CLEAR statement (keyword), not a call
src/repo/status/zcl_abapgit_status_calc.clas.abap:353 | no | CLEAR statement (keyword), not a call
src/repo/status/zcl_abapgit_status_calc.clas.abap:399 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:136 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.abap:141 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:94 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:105 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:116 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:131 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_repo_requirements.clas.abap:233 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:28 | no | CLEAR statement (keyword), not a call
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:38 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo.clas.abap:377 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo.clas.abap:569 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo.clas.abap:796 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo.clas.abap:800 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo.clas.abap:829 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_checksums.clas.locals_imp.abap:209 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_checksums.clas.testclasses.abap:33 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_checksums.clas.testclasses.abap:333 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_content_list.clas.abap:91 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_content_list.clas.abap:168 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_content_list.clas.abap:271 | no | receiver TYPE REF TO zif_abapgit_log
src/repo/zcl_abapgit_repo_content_list.clas.abap:279 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_online.clas.abap:259 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_online.clas.abap:274 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:177 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:295 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:471 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_srv.clas.abap:497 | no | CLEAR statement (keyword), not a call
src/repo/zcl_abapgit_repo_status.clas.locals_imp.abap:290 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_abap.clas.abap:94 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_abap.clas.abap:261 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:265 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:268 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:115 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:175 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:190 | no | comment or literal text, not code
src/syntax/zcl_abapgit_syntax_css.clas.abap:405 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:449 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:461 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:473 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:510 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:512 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:542 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_factory.clas.abap:41 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_highlighter.clas.abap:178 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_highlighter.clas.abap:228 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:81 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:108 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:162 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:205 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:217 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:229 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:263 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:265 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:295 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_json.clas.abap:95 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_json.clas.abap:108 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_json.clas.abap:110 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_xml.clas.abap:53 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_xml.clas.abap:99 | no | CLEAR statement (keyword), not a call
src/syntax/zcl_abapgit_syntax_xml.clas.abap:270 | no | CLEAR statement (keyword), not a call
src/test/zcl_abapgit_objects_ci_tests.clas.abap:109 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_gui.clas.abap:277 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_gui.clas.abap:483 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_gui.clas.abap:484 | no | receiver TYPE REF TO zcl_abapgit_html_parts
src/ui/core/zcl_abapgit_gui_event.clas.abap:167 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_gui_html_processor.clas.abap:103 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_html.clas.abap:254 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_html.clas.abap:653 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_html_parts.clas.abap:25 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/core/zcl_abapgit_html_parts.clas.abap:63 | no | METHOD statement, the implementation header, not a call
src/ui/core/zcl_abapgit_html_parts.clas.abap:64 | no | CLEAR statement (keyword), not a call
src/ui/core/zcl_abapgit_html_parts.clas.testclasses.abap:84 | no | receiver TYPE REF TO zcl_abapgit_html_parts
src/ui/core/zcx_abapgit_cancel.clas.abap:38 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.abap:80 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.abap:185 | no | lo_visit TYPE REF TO lcl_sha1_stack (local class)
src/ui/flow/zcl_abapgit_flow_git.clas.abap:215 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.abap:218 | no | lo_visit TYPE REF TO lcl_sha1_stack (local class)
src/ui/flow/zcl_abapgit_flow_git.clas.abap:273 | no | lo_visit TYPE REF TO lcl_sha1_stack (local class)
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:147 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:165 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:184 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:342 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:362 | no | METHOD statement, the implementation header, not a call
src/ui/flow/zcl_abapgit_flow_git.clas.locals_imp.abap:363 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.testclasses.abap:356 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.testclasses.abap:468 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.testclasses.abap:564 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_git.clas.testclasses.abap:630 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:233 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:250 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:274 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:281 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:327 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:335 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:427 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:469 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:483 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:512 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:560 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:601 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:607 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:770 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.abap:1094 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.testclasses.abap:163 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_logic.clas.testclasses.abap:404 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_flow_page_utils.clas.abap:122 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:330 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap:386 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:221 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:225 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flowcons.clas.abap:228 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flowuns.clas.abap:86 | no | CLEAR statement (keyword), not a call
src/ui/flow/zcl_abapgit_gui_page_flowuns.clas.abap:99 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_exception_viewer.clas.abap:158 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_exception_viewer.clas.abap:163 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_exception_viewer.clas.abap:168 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_exception_viewer.clas.abap:173 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:319 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:474 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:691 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap:692 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_page.clas.abap:416 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_picklist.clas.abap:178 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_gui_picklist.clas.abap:233 | yes | zcl_abapgit_gui_picklist->zif_abapgit_gui_event_handler~on_event: mo_validation_log TYPE REF TO zcl_abapgit_string_map (line 66)
src/ui/lib/zcl_abapgit_html_form.clas.abap:832 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_form.clas.abap:895 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_form.clas.abap:914 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:179 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_table.clas.abap:398 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_table.clas.abap:431 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_table.clas.abap:469 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_table.clas.abap:475 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_table.clas.abap:495 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_html_toolbar.clas.abap:244 | no | CLEAR statement (keyword), not a call
src/ui/lib/zcl_abapgit_log_viewer.clas.abap:233 | no | CLEAR statement (keyword), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_runit.clas.abap:83 | no | CLEAR statement (keyword), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_runit.clas.abap:160 | no | CLEAR statement (keyword), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_runit.clas.abap:318 | no | CLEAR statement (keyword), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_runit.clas.abap:347 | no | CLEAR statement (keyword), not a call
src/ui/pages/codi/zcl_abapgit_gui_page_whereused.clas.abap:215 | no | CLEAR statement (keyword), not a call
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:272 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:542 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:756 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:952 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1159 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap:1238 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:365 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:439 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:701 | no | CLEAR statement (keyword), not a call
src/ui/pages/diff/zcl_abapgit_gui_page_patch.clas.abap:706 | no | CLEAR statement (keyword), not a call
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:155 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_bckg.clas.abap:144 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_bckg.clas.abap:179 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_bckg.clas.abap:214 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_info.clas.abap:339 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_info.clas.abap:417 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_info.clas.abap:443 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_info.clas.abap:544 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:331 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:554 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:952 | yes | zcl_abapgit_gui_page_sett_remo->zif_abapgit_gui_event_handler~on_event: mo_validation_log TYPE REF TO zcl_abapgit_string_map (line 64)
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.testclasses.abap:275 | no | CLEAR statement (keyword), not a call
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:352 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:211 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:166 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:460 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:521 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:526 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:581 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_debuginfo.clas.abap:390 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_merge.clas.abap:234 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_merge_res.clas.abap:388 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_merge_res.clas.abap:453 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_merge_res.clas.abap:527 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:240 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:386 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:583 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:947 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap:954 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:271 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:278 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1300 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1325 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap:1377 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:253 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:358 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:520 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_stage.clas.abap:567 | no | CLEAR statement (keyword), not a call
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:342 | yes | zcl_abapgit_gui_page_tags->zif_abapgit_gui_event_handler~on_event: mo_validation_log TYPE REF TO zcl_abapgit_string_map (line 54)
src/ui/progress/zcl_abapgit_progress.clas.abap:83 | no | comment or literal text, not code
src/ui/progress/zcl_abapgit_progress.clas.abap:93 | no | CLEAR statement (keyword), not a call
src/ui/progress/zcl_abapgit_progress.clas.abap:94 | no | CLEAR statement (keyword), not a call
src/ui/routing/zcl_abapgit_services_repo.clas.abap:169 | no | static call of zcl_abapgit_objects_activation=>clear
src/ui/zcl_abapgit_gui_hotkey_ctl.clas.abap:128 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_gui_hotkey_ctl.clas.abap:182 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.abap:133 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.abap:384 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.abap:598 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.abap:630 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:149 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:187 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:339 | no | CLEAR statement (keyword), not a call
src/ui/zcl_abapgit_popups.clas.locals_imp.abap:402 | no | comment or literal text, not code
src/utils/zcl_abapgit_convert.clas.abap:320 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_convert.clas.abap:427 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_convert.clas.locals_imp.abap:289 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_convert.clas.testclasses.abap:334 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_log.clas.abap:175 | no | METHOD statement, the implementation header, not a call
src/utils/zcl_abapgit_log.clas.abap:176 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_log.clas.abap:205 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_log.clas.abap:214 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_log.clas.abap:224 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_log.clas.testclasses.abap:128 | no | receiver TYPE REF TO zif_abapgit_log
src/utils/zcl_abapgit_path.clas.abap:126 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_string_buffer.clas.abap:44 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_string_buffer.clas.abap:52 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_string_buffer.clas.abap:60 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_string_map.clas.abap:55 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/utils/zcl_abapgit_string_map.clas.abap:90 | no | METHOD statement, the implementation header, not a call
src/utils/zcl_abapgit_string_map.clas.abap:92 | no | comment or literal text, not code
src/utils/zcl_abapgit_string_map.clas.abap:94 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:98 | yes | ltcl_sm_test->freeze: lo_cut TYPE REF TO zcl_abapgit_string_map (line 76)
src/utils/zcl_abapgit_utils.clas.locals_imp.abap:55 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_utils.clas.locals_imp.abap:81 | no | CLEAR statement (keyword), not a call
src/utils/zcl_abapgit_utils.clas.locals_imp.abap:96 | no | CLEAR statement (keyword), not a call
src/utils/zif_abapgit_log.intf.abap:81 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/xml/zcl_abapgit_xml_input.clas.abap:73 | no | CLEAR statement (keyword), not a call
src/xml/zcl_abapgit_xml_input.clas.testclasses.abap:66 | no | CLEAR statement (keyword), not a call
src/zabapgit_forms.prog.abap:151 | no | CLEAR statement (keyword), not a call
src/zabapgit_password_dialog.prog.abap:68 | no | CLEAR statement (keyword), not a call
src/zabapgit_password_dialog.prog.abap:89 | no | CLEAR statement (keyword), not a call
src/zabapgit_password_dialog.prog.abap:92 | no | CLEAR statement (keyword), not a call
src/zabapgit_password_dialog.prog.abap:177 | no | CLEAR statement (keyword), not a call
src/zcl_abapgit_injector.clas.abap:112 | no | CLEAR statement (keyword), not a call
src/zcx_abapgit_exception.clas.abap:157 | no | CLEAR statement (keyword), not a call
src/zcx_abapgit_exception.clas.abap:279 | no | CLEAR statement (keyword), not a call
src/zcx_abapgit_exception.clas.abap:354 | no | CLEAR statement (keyword), not a call
src/zcx_abapgit_exception.clas.testclasses.abap:113 | no | CLEAR statement (keyword), not a call
```

## b1a_08 `zcl_abapgit_string_map->to_abap`

Defined in `src/utils/zcl_abapgit_string_map.clas.abap`. The name `to_abap` is declared in 6 classes or interfaces: `lcl_json_to_abap`, `ltcl_fields`, `ltcl_test_mappers`, `zcl_abapgit_string_map`, `zif_abapgit_ajson`, `zif_abapgit_ajson_mapping`.

```
src/data/zcl_abapgit_data_config.clas.abap:87 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/data/zcl_abapgit_data_config.clas.abap:162 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/data/zcl_abapgit_data_deserializer.clas.abap:98 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.abap:21 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.abap:998 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.abap:1009 | no | lo_to_abap TYPE REF TO lcl_json_to_abap (local class)
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:804 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:906 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:948 | no | receiver TYPE REF TO zif_abapgit_ajson_mapping
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1769 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1804 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1830 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1847 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1869 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1885 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1907 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1929 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1954 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:1985 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2021 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2057 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2089 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2117 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2149 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2186 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2221 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2238 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2255 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2272 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2289 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2307 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2324 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2343 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2375 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2405 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2437 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2449 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2476 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2512 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2545 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2578 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2611 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:2644 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4200 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4228 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4277 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5192 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5487 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5618 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5623 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5664 | no | comment or literal text, not code
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5669 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5706 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson.clas.testclasses.abap:5735 | no | lo_cut TYPE REF TO lcl_json_to_abap or zcl_abapgit_ajson (ajson test include)
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:16 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:57 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:107 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:109 | no | receiver TYPE REF TO zif_abapgit_ajson_mapping
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:147 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:149 | no | receiver TYPE REF TO zif_abapgit_ajson_mapping
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:188 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:190 | no | receiver TYPE REF TO zif_abapgit_ajson_mapping
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:271 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:290 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.locals_imp.abap:333 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:51 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:66 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:360 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:371 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:395 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zcl_abapgit_ajson_mapping.clas.testclasses.abap:426 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/json/zif_abapgit_ajson.intf.abap:140 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zif_abapgit_ajson_mapping.intf.abap:24 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/aff/zcl_abapgit_json_handler.clas.abap:116 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/objects/texts/zcl_abapgit_properties_file.clas.abap:62 | no | ajson receiver (zif_abapgit_ajson / zcl_abapgit_ajson, alias to_abap): ajson's to_abap
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:580 | yes | zcl_abapgit_gui_page_db->zif_abapgit_gui_event_handler~on_event: lo_query TYPE REF TO zcl_abapgit_string_map (line 573)
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:592 | yes | zcl_abapgit_gui_page_db->zif_abapgit_gui_event_handler~on_event: lo_query TYPE REF TO zcl_abapgit_string_map (line 573)
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:274 | yes | zcl_abapgit_gui_page_addofflin->zif_abapgit_gui_event_handler~on_event: mo_form_data TYPE REF TO zcl_abapgit_string_map (line 45)
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:394 | yes | zcl_abapgit_gui_page_addonline->zif_abapgit_gui_event_handler~on_event: mo_form_data TYPE REF TO zcl_abapgit_string_map (line 49)
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:528 | yes | zcl_abapgit_gui_page_commit->zif_abapgit_gui_event_handler~on_event: mo_form_data TYPE REF TO zcl_abapgit_string_map (line 53)
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:351 | yes | zcl_abapgit_gui_page_tags->zif_abapgit_gui_event_handler~on_event: mo_form_data TYPE REF TO zcl_abapgit_string_map (line 53)
src/ui/routing/zcl_abapgit_gui_router.clas.abap:212 | yes | zcl_abapgit_gui_router->db_actions: lo_query TYPE REF TO zcl_abapgit_string_map (line 207)
src/ui/routing/zcl_abapgit_gui_router.clas.abap:218 | yes | zcl_abapgit_gui_router->db_actions: lo_query TYPE REF TO zcl_abapgit_string_map (line 207)
src/utils/zcl_abapgit_string_map.clas.abap:58 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/utils/zcl_abapgit_string_map.clas.abap:217 | no | METHOD statement, the implementation header, not a call
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:133 | yes | ltcl_sm_test->strict: lo_cut TYPE REF TO zcl_abapgit_string_map (line 110)
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:142 | yes | ltcl_sm_test->strict: lo_map TYPE REF TO zcl_abapgit_string_map (line 111)
```

## b1a_09 `zcl_abapgit_syntax_highlighter->parse_line`

Defined in `src/syntax/zcl_abapgit_syntax_highlighter.clas.abap`. The name `parse_line` is declared in 3 classes or interfaces: `ltcl_news`, `zcl_abapgit_repo_news`, `zcl_abapgit_syntax_highlighter`.

```
src/repo/utils/zcl_abapgit_repo_news.clas.abap:61 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.abap:227 | no | bare parse_line( ) in zcl_abapgit_repo_news: its own static parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.abap:263 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:63 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:75 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:79 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:84 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:89 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:95 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:106 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:117 | no | static call of zcl_abapgit_repo_news=>parse_line
src/repo/utils/zcl_abapgit_repo_news.clas.testclasses.abap:132 | no | static call of zcl_abapgit_repo_news=>parse_line
src/syntax/zcl_abapgit_syntax_abap.clas.abap:39 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:287 | no | METHOD statement, the implementation header, not a call
src/syntax/zcl_abapgit_syntax_abap.clas.abap:293 | yes | zcl_abapgit_syntax_abap->parse_line: super->parse_line( ) in the subclass's redefinition
src/syntax/zcl_abapgit_syntax_abap.clas.testclasses.abap:186 | yes | ltcl_syntax_cases->do_test: lo_syntax TYPE REF TO zcl_abapgit_syntax_abap (subclass, line 182)
src/syntax/zcl_abapgit_syntax_css.clas.abap:93 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:528 | no | METHOD statement, the implementation header, not a call
src/syntax/zcl_abapgit_syntax_css.clas.abap:534 | yes | zcl_abapgit_syntax_css->parse_line: super->parse_line( ) in the subclass's redefinition
src/syntax/zcl_abapgit_syntax_highlighter.clas.abap:46 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_highlighter.clas.abap:206 | no | METHOD statement, the implementation header, not a call
src/syntax/zcl_abapgit_syntax_highlighter.clas.abap:259 | yes | zcl_abapgit_syntax_highlighter->process_line: bare parse_line( ) inside zcl_abapgit_syntax_highlighter
src/syntax/zcl_abapgit_syntax_js.clas.abap:59 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:281 | no | METHOD statement, the implementation header, not a call
src/syntax/zcl_abapgit_syntax_js.clas.abap:287 | yes | zcl_abapgit_syntax_js->parse_line: super->parse_line( ) in the subclass's redefinition
src/syntax/zcl_abapgit_syntax_xml.clas.abap:38 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/syntax/zcl_abapgit_syntax_xml.clas.abap:203 | no | METHOD statement, the implementation header, not a call
src/syntax/zcl_abapgit_syntax_xml.clas.abap:256 | yes | zcl_abapgit_syntax_xml->parse_line: super->parse_line( ) in the subclass's redefinition
src/syntax/zcl_abapgit_syntax_xml.clas.abap:280 | yes | zcl_abapgit_syntax_xml->parse_line: super->parse_line( ) in the subclass's redefinition
src/syntax/zcl_abapgit_syntax_xml.clas.testclasses.abap:188 | yes | ltcl_syntax_cases->do_test: lo_syntax TYPE REF TO zcl_abapgit_syntax_xml (subclass, line 184)
```

## b1a_10 `zcl_abapgit_html_form_utils->normalize`

Defined in `src/ui/lib/zcl_abapgit_html_form_utils.clas.abap`. The name `normalize` is declared in 6 classes or interfaces: `ltcl_tags`, `ltcl_test_form`, `ltcl_version`, `zcl_abapgit_html_form_utils`, `zcl_abapgit_repo_labels`, `zcl_abapgit_version`.

```
src/objects/zcl_abapgit_object_scp1.clas.abap:87 | no | comment or literal text, not code
src/repo/utils/zcl_abapgit_repo_labels.clas.abap:37 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.abap:100 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:51 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:54 | no | static call of zcl_abapgit_repo_labels=>normalize
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:58 | no | static call of zcl_abapgit_repo_labels=>normalize
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:62 | no | static call of zcl_abapgit_repo_labels=>normalize
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:66 | no | static call of zcl_abapgit_repo_labels=>normalize
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:70 | no | static call of zcl_abapgit_repo_labels=>normalize
src/repo/utils/zcl_abapgit_repo_news.clas.abap:87 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_repo_news.clas.abap:88 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_repo_news.clas.abap:166 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_repo_news.clas.abap:277 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_version.clas.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_version.clas.abap:225 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:10 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:50 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:53 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:57 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:61 | no | static call of zcl_abapgit_version=>normalize
src/repo/utils/zcl_abapgit_version.clas.testclasses.abap:65 | no | static call of zcl_abapgit_version=>normalize
src/repo/zcl_abapgit_repo.clas.abap:297 | no | static call of zcl_abapgit_repo_labels=>normalize
src/ui/lib/zcl_abapgit_gui_picklist.clas.abap:232 | yes | zcl_abapgit_gui_picklist->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 65)
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:23 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:158 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:118 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:363 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:482 | yes | ltcl_test_form->normalize: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:237 | yes | zcl_abapgit_gui_page_addofflin->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 46)
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:344 | yes | zcl_abapgit_gui_page_addonline->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 50)
src/ui/pages/dlg/zcl_abapgit_gui_page_cr_repo.clas.abap:178 | yes | zcl_abapgit_gui_page_cr_repo->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 37)
src/ui/pages/sett/zcl_abapgit_gui_page_sett_bckg.clas.abap:307 | yes | zcl_abapgit_gui_page_sett_bckg->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:344 | yes | zcl_abapgit_gui_page_sett_glob->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:423 | no | static call of zcl_abapgit_repo_labels=>normalize
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:511 | yes | zcl_abapgit_gui_page_sett_locl->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:394 | yes | zcl_abapgit_gui_page_sett_pers->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:925 | yes | zcl_abapgit_gui_page_sett_remo->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:567 | yes | zcl_abapgit_gui_page_sett_repo->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/zcl_abapgit_gui_page_chg_pckg.clas.abap:474 | yes | zcl_abapgit_gui_page_chg_pckg->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 59)
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:495 | yes | zcl_abapgit_gui_page_commit->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 54)
src/ui/pages/zcl_abapgit_gui_page_cpackage.clas.abap:153 | yes | zcl_abapgit_gui_page_cpackage->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 35)
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:514 | yes | zcl_abapgit_gui_page_data->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 58)
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:534 | yes | zcl_abapgit_gui_page_data->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 58)
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:545 | yes | zcl_abapgit_gui_page_data->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 58)
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:164 | yes | zcl_abapgit_gui_page_ex_object->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 41)
src/ui/pages/zcl_abapgit_gui_page_ex_pckage.clas.abap:137 | yes | zcl_abapgit_gui_page_ex_pckage->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 40)
src/ui/pages/zcl_abapgit_gui_page_merge_sel.clas.abap:154 | yes | zcl_abapgit_gui_page_merge_sel->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 43)
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:324 | yes | zcl_abapgit_gui_page_tags->zif_abapgit_gui_event_handler~on_event: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
```

## b1a_11 `zcl_abapgit_html_form_utils->validate`

Defined in `src/ui/lib/zcl_abapgit_html_form_utils.clas.abap`. The name `validate` is declared in 7 classes or interfaces: `lcl_aff_metadata_handler`, `ltcl_tags`, `zcl_abapgit_html_form_utils`, `zcl_abapgit_object_tabl_compar`, `zcl_abapgit_objects_generic`, `zcl_abapgit_repo_labels`, `zcl_abapgit_url`.

```
src/http/zcl_abapgit_http.clas.abap:175 | no | comment or literal text, not code
src/http/zcl_abapgit_url.clas.abap:8 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/http/zcl_abapgit_url.clas.abap:162 | no | METHOD statement, the implementation header, not a call
src/http/zcl_abapgit_url.clas.testclasses.abap:192 | no | static call of zcl_abapgit_url=>validate
src/http/zcl_abapgit_url.clas.testclasses.abap:202 | no | static call of zcl_abapgit_url=>validate
src/http/zcl_abapgit_url.clas.testclasses.abap:212 | no | static call of zcl_abapgit_url=>validate
src/json/zcl_abapgit_ajson.clas.locals_imp.abap:1070 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_objects_compare.clas.abap:73 | no | comment or literal text, not code
src/objects/core/zcl_abapgit_objects_compare.clas.abap:85 | no | comment or literal text, not code
src/objects/tabl/zcl_abapgit_object_tabl_compar.clas.abap:42 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/tabl/zcl_abapgit_object_tabl_compar.clas.abap:150 | no | METHOD statement, the implementation header, not a call
src/objects/tabl/zcl_abapgit_object_tabl_compar.clas.abap:241 | no | bare validate( ) in zcl_abapgit_object_tabl_compar: its own validate
src/objects/zcl_abapgit_object_dtel.clas.locals_imp.abap:458 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_dtel.clas.locals_imp.abap:480 | no | bare validate( ) in lcl_aff_metadata_handler: its own static validate
src/objects/zcl_abapgit_object_dtel.clas.locals_imp.abap:514 | no | bare validate( ) in lcl_aff_metadata_handler: its own static validate
src/objects/zcl_abapgit_object_dtel.clas.locals_imp.abap:536 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:108 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:305 | no | bare validate( ) in zcl_abapgit_objects_generic: its own validate
src/objects/zcl_abapgit_objects_generic.clas.abap:709 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_objects_generic.clas.abap:737 | no | comment or literal text, not code
src/repo/utils/zcl_abapgit_repo_labels.clas.abap:27 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.abap:235 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:7 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:37 | no | METHOD statement, the implementation header, not a call
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:39 | no | static call of zcl_abapgit_repo_labels=>validate
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:40 | no | static call of zcl_abapgit_repo_labels=>validate
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:41 | no | static call of zcl_abapgit_repo_labels=>validate
src/repo/utils/zcl_abapgit_repo_labels.clas.testclasses.abap:44 | no | static call of zcl_abapgit_repo_labels=>validate
src/repo/utils/zcl_abapgit_repo_news.clas.abap:86 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo_srv.clas.abap:520 | no | comment or literal text, not code
src/repo/zcl_abapgit_repo_srv.clas.abap:820 | no | static call of zcl_abapgit_url=>validate
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:30 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:245 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:158 | yes | ltcl_test_form->validate1: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:171 | yes | ltcl_test_form->validate1: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:184 | yes | ltcl_test_form->validate1: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:215 | yes | ltcl_test_form->validate2: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:228 | yes | ltcl_test_form->validate2: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:241 | yes | ltcl_test_form->validate2: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:272 | yes | ltcl_test_form->validate3: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:285 | yes | ltcl_test_form->validate3: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:298 | yes | ltcl_test_form->validate3: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:311 | yes | ltcl_test_form->validate3: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:342 | yes | ltcl_test_form->validate4: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:355 | yes | ltcl_test_form->validate4: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/pages/db/zcl_abapgit_gui_page_db.clas.abap:287 | no | comment or literal text, not code
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:198 | yes | zcl_abapgit_gui_page_addofflin->validate_form: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 46)
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:221 | no | static call of zcl_abapgit_repo_labels=>validate
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:288 | yes | zcl_abapgit_gui_page_addonline->validate_form: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 50)
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:328 | no | static call of zcl_abapgit_repo_labels=>validate
src/ui/pages/dlg/zcl_abapgit_gui_page_cr_repo.clas.abap:182 | yes | zcl_abapgit_gui_page_cr_repo->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 37)
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:323 | yes | zcl_abapgit_gui_page_sett_glob->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:353 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:456 | yes | zcl_abapgit_gui_page_sett_locl->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:499 | no | static call of zcl_abapgit_repo_labels=>validate
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:539 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:379 | yes | zcl_abapgit_gui_page_sett_pers->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:403 | no | comment or literal text, not code
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:839 | yes | zcl_abapgit_gui_page_sett_remo->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:468 | yes | zcl_abapgit_gui_page_sett_repo->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:576 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_chg_pckg.clas.abap:453 | yes | zcl_abapgit_gui_page_chg_pckg->validate_form: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 59)
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:457 | yes | zcl_abapgit_gui_page_commit->validate_form: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 54)
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:520 | no | comment or literal text, not code
src/ui/pages/zcl_abapgit_gui_page_cpackage.clas.abap:157 | yes | zcl_abapgit_gui_page_cpackage->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 35)
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:433 | yes | zcl_abapgit_gui_page_data->validate_form: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 58)
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:169 | yes | zcl_abapgit_gui_page_ex_object->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 41)
src/ui/pages/zcl_abapgit_gui_page_ex_pckage.clas.abap:142 | yes | zcl_abapgit_gui_page_ex_pckage->zif_abapgit_gui_event_handler~on_event: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 40)
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:291 | yes | zcl_abapgit_gui_page_tags->validate_form: zcl_abapgit_html_form_utils=>create( ) returns zcl_abapgit_html_form_utils
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:345 | no | comment or literal text, not code
```

## b1a_12 `zcl_abapgit_html_form_utils->is_empty`

Defined in `src/ui/lib/zcl_abapgit_html_form_utils.clas.abap`. The name `is_empty` is declared in 7 classes or interfaces: `ltcl_integrated`, `ltcl_test_form`, `zcl_abapgit_html_form_utils`, `zcl_abapgit_object_devc`, `zcl_abapgit_string_map`, `zif_abapgit_ajson`, `zif_abapgit_html`.

```
src/json/zcl_abapgit_ajson.clas.abap:10 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.abap:545 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4174 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4480 | no | METHOD statement, the implementation header, not a call
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4488 | no | receiver TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson.clas.testclasses.abap:4496 | no | receiver TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson_utilities.clas.abap:329 | no | receiver TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson_utilities.clas.abap:330 | no | receiver TYPE REF TO zif_abapgit_ajson
src/json/zcl_abapgit_ajson_utilities.clas.abap:331 | no | receiver TYPE REF TO zif_abapgit_ajson
src/json/zif_abapgit_ajson.intf.abap:64 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:45 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/objects/zcl_abapgit_object_devc.clas.abap:125 | no | METHOD statement, the implementation header, not a call
src/objects/zcl_abapgit_object_devc.clas.abap:460 | no | bare is_empty( ) in zcl_abapgit_object_devc: its own is_empty
src/ui/core/zcl_abapgit_html.clas.abap:539 | no | METHOD statement, the implementation header, not a call
src/ui/core/zif_abapgit_html.intf.abap:49 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_gui_page.clas.abap:616 | no | li_script TYPE REF TO zif_abapgit_html
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:37 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:111 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.abap:171 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:119 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:490 | no | METHOD statement, the implementation header, not a call
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:520 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:528 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:537 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:546 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:555 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/lib/zcl_abapgit_html_form_utils.clas.testclasses.abap:576 | yes | ltcl_test_form->is_empty: lo_cut TYPE REF TO zcl_abapgit_html_form_utils (declared in the test method)
src/ui/pages/dlg/zcl_abapgit_gui_page_addofflin.clas.abap:273 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/dlg/zcl_abapgit_gui_page_addonline.clas.abap:393 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/dlg/zcl_abapgit_gui_page_cr_repo.clas.abap:184 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_glob.clas.abap:356 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_locl.clas.abap:542 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_pers.clas.abap:406 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:384 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:982 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.abap:1019 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_remo.clas.testclasses.abap:521 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/sett/zcl_abapgit_gui_page_sett_repo.clas.abap:579 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_chg_pckg.clas.abap:481 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:523 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_commit.clas.abap:566 | yes | zcl_abapgit_gui_page_commit->zif_abapgit_gui_renderable~render: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 54)
src/ui/pages/zcl_abapgit_gui_page_cpackage.clas.abap:159 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_cpackage.clas.abap:207 | yes | zcl_abapgit_gui_page_cpackage->zif_abapgit_gui_renderable~render: mo_form_util TYPE REF TO zcl_abapgit_html_form_utils (line 35)
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:443 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:517 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_data.clas.abap:539 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_ex_object.clas.abap:178 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_ex_pckage.clas.abap:143 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:154 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/ui/pages/zcl_abapgit_gui_page_tags.clas.abap:348 | no | receiver TYPE REF TO zcl_abapgit_string_map: zcl_abapgit_string_map->is_empty
src/utils/zcl_abapgit_string_map.clas.abap:47 | no | declaration in a CLASS/INTERFACE definition (METHODS, ALIASES, DATA, parameter), not a call
src/utils/zcl_abapgit_string_map.clas.abap:154 | no | METHOD statement, the implementation header, not a call
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:31 | no | lo_cut TYPE REF TO zcl_abapgit_string_map
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:39 | no | lo_cut TYPE REF TO zcl_abapgit_string_map
src/utils/zcl_abapgit_string_map.clas.testclasses.abap:61 | no | lo_cut TYPE REF TO zcl_abapgit_string_map
```
