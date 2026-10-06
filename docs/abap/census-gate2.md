# ABAP Gate 2 census: precision and recall of resolved callers

Story 2.7, item 2 of Gate 2: are the callers sem resolves for an ABAP method
right, and does it find them? Thirty methods of abapGit, chosen by a rule
before any sem caller output was read, scored at the caller entity against a
text where-used, with the thresholds of the story: pooled precision at least
90% and recall at least 80%.

| | |
|---|---|
| Date | 2026-10-06 |
| sem | `story/2-7` at `cb42517` (the `abap` branch at `32825dd`, stories 2.0 to 2.5 and 2.8 merged, plus T2-C), `cargo build --release -p sem-cli` |
| Corpus | abapGit at `b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`, `git status` clean |
| Ground truth | the grep fallback, `scripts/abap-whereused-grep.py` (no `sapcli` export exists yet) |
| Story 2.6 | running in parallel, not merged; not needed by 2.7 |

## The sample

Chosen before any `sem find --callers` was run. The rule is coded in
`scripts/abap-whereused-grep.py sample`, and only reads sem's entity listing
(`sem find --in src --json`, for its order and the methods' line ranges) and
the text where-used of the next section.

1. **Candidates.** Every `METHODS` and `CLASS-METHODS` declaration in a global
   class or interface file (`*.clas.abap`, `*.intf.abap`), read off the
   source with comments and literals blanked, that is:
   - not a test method or in a `FOR TESTING` class;
   - not an event handler (`FOR EVENT`) and not a constructor;
   - **declared once** in the repository: no other class or interface,
     global or local, declares a method of that name (a `REDEFINITION`
     does not count). This is B.1's rule and what lets a name match be
     adjudicated by its receiver alone;
   - a `method` entity in sem's listing (so the target exists for sem);
   - with **2 to 40 call sites** by the text where-used.
2. **Strata**, from the declaration: `interface` (declared in an interface),
   `static` (`CLASS-METHODS`), `redefined` (an instance method some subclass
   redefines), `me` (private or protected, so called through `me->`, bare,
   or from a subclass), `instance` (the rest: public instance methods), and
   `form` (a `FORM` with 2 to 40 `PERFORM` sites).
3. **Quotas**: static 7, instance 7, interface 7, redefined 4, me 3, form 2.
   A stratum with fewer candidates than its quota takes them all and hands
   the rest out one pick at a time to static, instance, interface, me, in
   that order, round robin.
4. **Picks.** In each stratum, candidates in `sem find --in src --json`
   order (file path, then line); with `n` candidates and `q` picks,
   `k = n div q` and the pick is index `0, k, 2k, ..., (q-1)k` (0-based), as
   in the Gate 1 census.

| Measure | Count |
|---|---:|
| `METHODS` / `CLASS-METHODS` declarations read (each counted under the first exclusion it meets) | 4788 |
| ... not in a global class or interface file (local classes, test includes) | 1686 |
| ... name declared more than once | 1060 |
| ... constructor or class constructor | 204 |
| ... test method or in a test class | 9 |
| ... event handler | 5 |
| ... no method entity in sem's listing | 7 |
| ... fewer than 2 or more than 40 call sites | 992 |
| Candidates | 825 |

| Stratum | Candidates | Quota | Picks |
|---|---:|---:|---:|
| static | 244 | 7 | 8 |
| instance | 119 | 7 | 8 |
| interface | 221 | 7 | 8 |
| redefined | 3 | 4 | 3 |
| me | 238 | 3 | 3 |
| form | 0 | 2 | 0 |

abapGit has six forms, none with two `PERFORM` sites, and three redefined
methods pass the filter, so four picks moved: static, instance and interface
take one each. The B.1 targets are not in the rule; they are scored on their
own below, with the same scripts.

| # | Stratum | Target | Defined in | Grep call sites | Grep caller methods | Files |
|---:|---|---|---|---:|---:|---:|
| 1 | static | `zcl_abapgit_apack_helper=>get_dependencies_met_status` | `src/apack/zcl_abapgit_apack_helper.clas.abap` | 2 | 2 | 1 |
| 2 | static | `zcl_abapgit_git_pack=>decode_commit` | `src/git/zcl_abapgit_git_pack.clas.abap` | 20 | 14 | 7 |
| 3 | static | `zcl_abapgit_http=>get_http_client` | `src/http/zcl_abapgit_http.clas.abap` | 2 | 2 | 1 |
| 4 | static | `zcl_abapgit_filename_logic=>is_obj_definition_file` | `src/objects/core/zcl_abapgit_filename_logic.clas.abap` | 5 | 2 | 2 |
| 5 | static | `zcl_abapgit_object_sicf=>get_hash_from_object` | `src/objects/zcl_abapgit_object_sicf.clas.abap` | 2 | 2 | 1 |
| 6 | static | `zcl_abapgit_zip=>encode_files` | `src/repo/utils/zcl_abapgit_zip.clas.abap` | 3 | 3 | 2 |
| 7 | static | `zcl_abapgit_gui_chunk_lib=>render_repo_palette` | `src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap` | 3 | 3 | 3 |
| 8 | static | `zcl_abapgit_services_repo=>delete_unnecessary_objects` | `src/ui/routing/zcl_abapgit_services_repo.clas.abap` | 2 | 2 | 2 |
| 9 | instance | `zcl_abapgit_apack_reader->copy_manifest_descriptor` | `src/apack/zcl_abapgit_apack_reader.clas.abap` | 2 | 2 | 2 |
| 10 | instance | `zcl_abapgit_settings->get_run_critical_tests` | `src/env/zcl_abapgit_settings.clas.abap` | 2 | 2 | 2 |
| 11 | instance | `zcl_abapgit_zlib_stream->remaining` | `src/git/zlib/zcl_abapgit_zlib_stream.clas.abap` | 9 | 5 | 2 |
| 12 | instance | `zcl_abapgit_folder_logic->path_to_package` | `src/objects/core/zcl_abapgit_folder_logic.clas.abap` | 4 | 4 | 4 |
| 13 | instance | `zcl_abapgit_oo_serializer->serialize_abap_clif_source` | `src/objects/oo/zcl_abapgit_oo_serializer.clas.abap` | 2 | 2 | 2 |
| 14 | instance | `zcl_abapgit_repo_item_state->sum_with_repo_item` | `src/repo/utils/zcl_abapgit_repo_item_state.clas.abap` | 7 | 3 | 3 |
| 15 | instance | `zcl_abapgit_dot_abapgit->set_name` | `src/repo/zcl_abapgit_dot_abapgit.clas.abap` | 4 | 4 | 3 |
| 16 | instance | `zcl_abapgit_gui_picklist->is_in_page` | `src/ui/lib/zcl_abapgit_gui_picklist.clas.abap` | 16 | 12 | 4 |
| 17 | interface | `zif_abapgit_cts_api~create_transport_entries` | `src/cts/zif_abapgit_cts_api.intf.abap` | 3 | 2 | 2 |
| 18 | interface | `zif_abapgit_exit~adjust_commit_message` | `src/exits/zif_abapgit_exit.intf.abap` | 2 | 2 | 2 |
| 19 | interface | `zif_abapgit_exit~validate_after_push` | `src/exits/zif_abapgit_exit.intf.abap` | 2 | 2 | 2 |
| 20 | interface | `zif_abapgit_ajson_iterator~has_next` | `src/json/zif_abapgit_ajson_iterator.intf.abap` | 3 | 2 | 1 |
| 21 | interface | `zif_abapgit_oo_object_fnc~read_attributes` | `src/objects/oo/zif_abapgit_oo_object_fnc.intf.abap` | 2 | 2 | 2 |
| 22 | interface | `zif_abapgit_persist_user~set_repo_show` | `src/persist/zif_abapgit_persist_user.intf.abap` | 10 | 9 | 6 |
| 23 | interface | `zif_abapgit_repo_online~get_selected_commit` | `src/repo/zif_abapgit_repo_online.intf.abap` | 8 | 5 | 3 |
| 24 | interface | `zif_abapgit_progress~show` | `src/ui/progress/zif_abapgit_progress.intf.abap` | 14 | 12 | 9 |
| 25 | redefined | `zcl_abapgit_object_common_aff->get_additional_extensions` | `src/objects/aff/zcl_abapgit_object_common_aff.clas.abap` | 2 | 2 | 1 |
| 26 | redefined | `zcl_abapgit_repo->reset_remote` | `src/repo/zcl_abapgit_repo.clas.abap` | 5 | 5 | 3 |
| 27 | redefined | `zcl_abapgit_syntax_highlighter->order_matches` | `src/syntax/zcl_abapgit_syntax_highlighter.clas.abap` | 3 | 3 | 3 |
| 28 | me | `zcl_abapgit_apack_migration->add_intf_source_and_activate` | `src/apack/zcl_abapgit_apack_migration.clas.abap` | 2 | 2 | 1 |
| 29 | me | `zcl_abapgit_object_ecatt_super->serialize_version` | `src/objects/zcl_abapgit_object_ecatt_super.clas.abap` | 2 | 1 | 1 |
| 30 | me | `zcl_abapgit_repo_online->set_objects` | `src/repo/zcl_abapgit_repo_online.clas.abap` | 2 | 2 | 1 |

## The ground truth

`scripts/abap-whereused-grep.py truth`, written for this study and sharing no
code with sem: its own stripper (`*` in column 1, `"` to the end of the line,
`'...'` and backtick literals with their doubled-quote escape, the text of
`|...|` templates with their `{ ... }` parts kept as code), its own reading of
the declarations, case-folded. A site is `->m(`, `=>m(`, `zif_x~m(` (the
declaring interface only), `CALL METHOD ...m`, a bare `m(` inside the
declaring class or a subclass, and an alias of an interface method declared
with `ALIASES a FOR zif_x~m` (`->a(` and a bare `a(` in the class that
declares it). Each site is adjudicated by its receiver: `cls=>m(` with a
`cls` outside the target's class and subclasses is dropped, and `lo->m(` is
dropped when `lo` is declared (in the enclosing method, or once in the file)
`TYPE REF TO` a type outside the repository or a repository type outside the
target's hierarchy. Declarations (`METHODS m`, `METHOD m.`, `ALIASES`) have
no `(` after the name and are never sites. Each site is attributed to the
innermost method, form or function module around it, by the line ranges of
`sem find --in src --json` (nothing else of sem is used); a local class's
method counts under its object, as SAP reports it. Literals that name a
method (`'SHOW'`), the trace of a computed call, are listed under the
target's `dynamic` and never counted.

The truth is committed as
`crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json`, in the
schema of that folder's `README.md` (`{object, type, include, line}`, the
calling method in `include`), plus `file`, `caller`, `shape` and
`adjudicated` per row, and a `_meta` key with the commit, the grep counts per
target, the rejected sites and the dynamic mentions. When `sapcli whereused`
has been run it goes next to it as `whereused.json`, and the compare script
prefers it. `scripts/abap-whereused-convert.py` turns either into the
benchmark harness's shape and back (round-trip test:
`python3 scripts/abap-whereused-convert.test.py`).

Every row of the 40 targets (the 30 and the ten B.1 targets) was then read
against the source by hand, before sem's callers were run:

| | The 30 | B.1 |
|---|---:|---:|
| Call sites | 145 | 145 |
| Caller entities (the scoring unit) | 115 | 135 |
| Rows decided by the alias rule (`"adjudicated": true`) | 6 | 3 |
| Sites dropped for their receiver | 2 | 2 |
| Rows changed by the hand check | 0 | 2 |

The two dropped among the 30 are right: `li_n_xmlref_typ->set_name( )` in
`zcl_abapgit_ecatt_helper` is an `if_ixml_node`, and
`cl_table_utilities_brf=>create_transport_entries( )` in
`zcl_abapgit_cts_api` is SAP's. The hand check found one error, in a B.1
target: B.1's rule says each target is declared once, but `get_proxy_url` is
declared in `zcl_abapgit_settings` and in `zcl_abapgit_proxy_config`, and the
two `lo_proxy_configuration->get_proxy_url( iv_url )` sites in
`zcl_abapgit_http` and `zcl_abapgit_http_agent` call the second. The receiver
rule now drops a receiver typed as a repository class outside the target's
hierarchy, which removed exactly those two. Against B.1's own grep counts
(`tasks/b1_whereused.json`), the truth has three sites more: bare calls
inside the declaring class (`get_description` calls `get_display_name( )`,
`conversion_exit_isola_output` calls `language_sap1_to_sap2( )`) and an alias
call (`get_files_local( )` in `zcl_abapgit_repo`), which B.1's patterns
(`->name(`, `=>name(`, `CALL METHOD`) do not match; and two fewer, the
`get_proxy_url` sites.

## The result

`scripts/abap-whereused-compare.py --exclude-stratum b1`: for each target,
`sem find <owner>.<method> --callers --json` in the abapGit checkout, the
result whose entity is the target, and its `related` callers, which are the
sources of `calls` edges into it. `possible_callers` do not count. Matched at
the caller entity, (object, method), case-folded. The second group of
columns adds the callers of every entity a `dispatch` edge links to the
target, either way, from `sem graph --json`: what story 2.3's dispatch adds.

| Target | Stratum | Truth | Resolved | Hits | Precision | Recall | + dispatch: resolved, hits | Precision | Recall |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ZCL_ABAPGIT_APACK_HELPER=>GET_DEPENDENCIES_MET_STATUS` | static | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_GIT_PACK=>DECODE_COMMIT` | static | 14 | 14 | 14 | 100% | 100% | 14, 14 | 100% | 100% |
| `ZCL_ABAPGIT_HTTP=>GET_HTTP_CLIENT` | static | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_FILENAME_LOGIC=>IS_OBJ_DEFINITION_FILE` | static | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_OBJECT_SICF=>GET_HASH_FROM_OBJECT` | static | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_ZIP=>ENCODE_FILES` | static | 3 | 3 | 3 | 100% | 100% | 3, 3 | 100% | 100% |
| `ZCL_ABAPGIT_GUI_CHUNK_LIB=>RENDER_REPO_PALETTE` | static | 3 | 3 | 3 | 100% | 100% | 3, 3 | 100% | 100% |
| `ZCL_ABAPGIT_SERVICES_REPO=>DELETE_UNNECESSARY_OBJECTS` | static | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_APACK_READER=>COPY_MANIFEST_DESCRIPTOR` | instance | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_SETTINGS=>GET_RUN_CRITICAL_TESTS` | instance | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_ZLIB_STREAM=>REMAINING` | instance | 5 | 5 | 5 | 100% | 100% | 5, 5 | 100% | 100% |
| `ZCL_ABAPGIT_FOLDER_LOGIC=>PATH_TO_PACKAGE` | instance | 4 | 4 | 4 | 100% | 100% | 4, 4 | 100% | 100% |
| `ZCL_ABAPGIT_OO_SERIALIZER=>SERIALIZE_ABAP_CLIF_SOURCE` | instance | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_REPO_ITEM_STATE=>SUM_WITH_REPO_ITEM` | instance | 3 | 3 | 3 | 100% | 100% | 3, 3 | 100% | 100% |
| `ZCL_ABAPGIT_DOT_ABAPGIT=>SET_NAME` | instance | 4 | 4 | 4 | 100% | 100% | 4, 4 | 100% | 100% |
| `ZCL_ABAPGIT_GUI_PICKLIST=>IS_IN_PAGE` | instance | 12 | 12 | 12 | 100% | 100% | 12, 12 | 100% | 100% |
| `ZIF_ABAPGIT_CTS_API~CREATE_TRANSPORT_ENTRIES` | interface | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZIF_ABAPGIT_EXIT~ADJUST_COMMIT_MESSAGE` | interface | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZIF_ABAPGIT_EXIT~VALIDATE_AFTER_PUSH` | interface | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZIF_ABAPGIT_AJSON_ITERATOR~HAS_NEXT` | interface | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZIF_ABAPGIT_OO_OBJECT_FNC~READ_ATTRIBUTES` | interface | 2 | 1 | 1 | 100% | 50% | 2, 2 | 100% | 100% |
| `ZIF_ABAPGIT_PERSIST_USER~SET_REPO_SHOW` | interface | 9 | 8 | 8 | 100% | 89% | 9, 9 | 100% | 100% |
| `ZIF_ABAPGIT_REPO_ONLINE~GET_SELECTED_COMMIT` | interface | 5 | 2 | 2 | 100% | 40% | 5, 5 | 100% | 100% |
| `ZIF_ABAPGIT_PROGRESS~SHOW` | interface | 12 | 12 | 12 | 100% | 100% | 12, 12 | 100% | 100% |
| `ZCL_ABAPGIT_OBJECT_COMMON_AFF=>GET_ADDITIONAL_EXTENSIONS` | redefined | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_REPO=>RESET_REMOTE` | redefined | 5 | 5 | 5 | 100% | 100% | 5, 5 | 100% | 100% |
| `ZCL_ABAPGIT_SYNTAX_HIGHLIGHTER=>ORDER_MATCHES` | redefined | 3 | 1 | 1 | 100% | 33% | 3, 3 | 100% | 100% |
| `ZCL_ABAPGIT_APACK_MIGRATION=>ADD_INTF_SOURCE_AND_ACTIVATE` | me | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| `ZCL_ABAPGIT_OBJECT_ECATT_SUPER=>SERIALIZE_VERSION` | me | 1 | 1 | 1 | 100% | 100% | 1, 1 | 100% | 100% |
| `ZCL_ABAPGIT_REPO_ONLINE=>SET_OBJECTS` | me | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| **pooled** | | 115 | 108 | 108 | **100%** | **94%** | 115, 115 | 100% | 100% |

| Stratum | Truth | Resolved | Hits | Precision | Recall |
|---|---:|---:|---:|---:|---:|
| static | 30 | 30 | 30 | 100% | 100% |
| instance | 34 | 34 | 34 | 100% | 100% |
| interface | 36 | 31 | 31 | 100% | 86% |
| redefined | 10 | 8 | 8 | 100% | 80% |
| me | 5 | 5 | 5 | 100% | 100% |

**Gate 2 precision check: PASS.** Pooled over the 30 methods, resolved
callers have precision 100% (108 of 108) and recall 94% (108 of 115),
against thresholds of 90% and 80%. With the callers of dispatch-linked
entities added, both are 100% (115 of 115).

Every miss, all seven false negatives; there is no false positive:

| Target | Caller | Cause |
|---|---|---|
| `zif_abapgit_oo_object_fnc~read_attributes` | `zcl_abapgit_oo_class->zif_abapgit_oo_object_fnc~create` | missing edge: `zif_abapgit_oo_object_fnc~read_attributes( )` called on the object itself is bound to the inherited implementation in `zcl_abapgit_oo_base`, not to the interface's declaration |
| `zif_abapgit_persist_user~set_repo_show` | `zcl_abapgit_persistence_user->zif_abapgit_persist_user~get_repo_show` | missing edge: the same self call through the interface prefix, bound to the class's own implementation |
| `zif_abapgit_repo_online~get_selected_commit` | `zcl_abapgit_repo_online->fetch_remote` | missing edge: a bare call of the alias `get_selected_commit`, bound to the class's implementation |
| `zif_abapgit_repo_online~get_selected_commit` | `zcl_abapgit_repo_online->zif_abapgit_repo_online~get_remote_settings` | missing edge: the same alias call |
| `zif_abapgit_repo_online~get_selected_commit` | `zcl_abapgit_repo_online->zif_abapgit_repo_online~push` | missing edge: the same alias call |
| `zcl_abapgit_syntax_highlighter->order_matches` | `zcl_abapgit_syntax_abap` test `ltcl_syntax_cases->do_test` | missing edge: `lo_syntax` is typed `zcl_abapgit_syntax_abap`, so the call is bound to that subclass's redefinition, not to the base declaration SAP lists it under |
| `zcl_abapgit_syntax_highlighter->order_matches` | `zcl_abapgit_syntax_xml` test `ltcl_syntax_cases->do_test` | missing edge: the same, through `zcl_abapgit_syntax_xml`'s redefinition |

By cause: 7 missing edges, 0 wrong edges, 0 truth errors. All seven are one
gap: a call sem binds to an implementation or a redefinition is not also a
caller of the declaration it implements, which is what a where-used on the
declaration reports. Five are a class calling its own interface method (by
the `zif~m` prefix or by an alias), two a receiver typed as a subclass that
redefines the method. Each is an edge sem has, to the right code, and each
is found once dispatch is followed (the second columns). That makes it a
question of what `--callers` on a declaration should list, a Tier 2
follow-up, not a resolution error. Not a miss, but worth knowing: no
target's callers include a computed call (`CALL METHOD lo->(lv)`); the
truth's only dynamic mentions are five `'SHOW'` literals near
`zif_abapgit_progress~show`, none of which is a call.

What the number does not cover. The truth is a text where-used adjudicated
by hand, not SAP's, and the 30 are methods declared once (a name match
needs no type to adjudicate), so a method name shared by several classes,
where binding by receiver type matters most, is outside the sample; B.1's
`get_proxy_url` is the one such case scored, and sem got it right. The
truth sets are small (115 caller entities, the story expected about 300
hits): 23 of the 30 have 2 to 5 call sites, as the rule picks evenly
through candidates that are mostly small.

## The B.1 targets

The same scripts on the ten targets of `bench/abap-agent/tasks/b1_whereused.json`
(`--stratum b1`), so the Gate 2 benchmark run can score B.1:
`scripts/abap-whereused-convert.py to-harness` wrote
`bench/abap-agent/ground_truth/whereused.grep.json` from the truth, and the
task file's `ground_truth` now points to it.

| Target | Stratum | Truth | Resolved | Hits | Precision | Recall | + dispatch: resolved, hits | Precision | Recall |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `ZCL_ABAPGIT_GIT_BRANCH_UTILS=>GET_DISPLAY_NAME` | b1 | 12 | 12 | 12 | 100% | 100% | 12, 12 | 100% | 100% |
| `ZCL_ABAPGIT_OBJECTS_FILES=>READ_STRING` | b1 | 16 | 16 | 16 | 100% | 100% | 16, 16 | 100% | 100% |
| `ZCL_ABAPGIT_OBJECTS_ACTIVATION=>ADD_ITEM` | b1 | 26 | 26 | 26 | 100% | 100% | 26, 26 | 100% | 100% |
| `ZIF_ABAPGIT_SAP_PACKAGE~LIST_SUBPACKAGES` | b1 | 16 | 17 | 16 | 94% | 100% | 17, 16 | 94% | 100% |
| `ZCL_ABAPGIT_I18N_PARAMS=>IS_LXE_APPLICABLE` | b1 | 21 | 21 | 21 | 100% | 100% | 21, 21 | 100% | 100% |
| `ZIF_ABAPGIT_REPO~GET_FILES_LOCAL` | b1 | 11 | 10 | 10 | 100% | 91% | 11, 11 | 100% | 100% |
| `ZCL_ABAPGIT_GUI_CHUNK_LIB=>RENDER_REPO_TOP` | b1 | 16 | 16 | 16 | 100% | 100% | 16, 16 | 100% | 100% |
| `ZCL_ABAPGIT_HTML_FORM=>TEXTAREA` | b1 | 9 | 9 | 9 | 100% | 100% | 9, 9 | 100% | 100% |
| `ZCL_ABAPGIT_CONVERT=>LANGUAGE_SAP1_TO_SAP2` | b1 | 6 | 6 | 6 | 100% | 100% | 6, 6 | 100% | 100% |
| `ZCL_ABAPGIT_SETTINGS=>GET_PROXY_URL` | b1 | 2 | 2 | 2 | 100% | 100% | 2, 2 | 100% | 100% |
| **pooled** | | 135 | 135 | 134 | **99%** | **99%** | 136, 135 | 99% | 100% |

Precision 99% (134 of 135), recall 99% (134 of 135): both over the
thresholds.

| Target | Kind | Caller | Cause |
|---|---|---|---|
| `zif_abapgit_sap_package~list_subpackages` | FP | `zcl_abapgit_sap_package_test` test `check_list_subpackages` | outside the truth's scope: the caller is in abapGit's `test/src/`, a transpiler test folder that is not an abapGit package and never reaches SAP; the truth reads `src/` only. The call is real |
| `zif_abapgit_repo~get_files_local` | FN | `zcl_abapgit_repo->zif_abapgit_repo~refresh_local_objects` | missing edge: a bare call of the alias `get_files_local`, bound to the class's own implementation (the study's alias case) |

## T2-C on abapGit

T2-C went in first (`cb42517`), so the study matched entities of the final
shape. `sem find --in src --json --file-exts .abap --no-default-excludes`,
before (`abap` at `32825dd`) and after:

| Entity type | Before | After |
|---|---:|---:|
| all | 11300 | 11331 |
| variable | 1032 | 1056 |
| type | 773 | 778 |
| constant | 421 | 423 |
| method | 8141 | 8141 |
| class / interface | 804 / 120 | 804 / 120 |

No entity changed or went. In a class definition the grammar already gave
one `variable` per name of a chained `DATA:` (fact 14 of `README.md`), so
the 1032 were already split; the 24 new variables are 7 in interfaces (an
interface's `DATA` was not an entity) and 17 in local classes the grammar
loses whole (`zcl_abapgit_ajson`'s `lcl_json_serializer`, `lcl_json_to_abap`
and `lcl_filter_runner`, three test classes), which the fallback now reads,
with their 5 `TYPES` and 2 `CONSTANTS`.

## Where this differs from the story

`stories/2-7-precision-study.md` was written before the study's brief; the
brief decided these, and the story's file names are not used:

- The result is this file, not `docs/abap/precision-report.md` (Gate 2 item 2
  in `stories/README.md` still names that one).
- The study runs as scripts (`scripts/abap-whereused-*.py`), not as an
  `#[ignore]`d `crates/sem-core/tests/abap_precision.rs`, and the targets and
  truth are one file, `inside-sap/whereused.grep.json` (its `_meta` holds the
  stratum of each), not `precision/methods.json` and
  `precision/abapgit-whereused.json`.
- The 30 are chosen by the rule above, not "B.1's ten plus twenty across
  strata"; the B.1 ten are scored separately.
- T2-C: the story says `DATA:` is the grammar's `variable_declaration`, one
  entity named after one member. Since the grammar fork's chained
  declarations (fact 14) that is no longer so in a class definition, which
  already gave one entity per name; the change was for interfaces and for
  the fallback (see the CHANGELOG and `cb42517`).

## Reproducing

```bash
git -C /tmp/claude-0/abapGit checkout b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0
(cd crates && cargo build --release -p sem-cli)
python3 scripts/abap-whereused-grep.py sample --out /tmp/sample.json        # the rule, the 30
python3 scripts/abap-whereused-grep.py truth --targets <targets.json> \
  --out crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json
python3 scripts/abap-whereused-compare.py --exclude-stratum b1              # the study
python3 scripts/abap-whereused-compare.py --stratum b1                      # B.1
python3 scripts/abap-whereused-convert.py to-harness \
  crates/sem-core/tests/fixtures/abap/inside-sap/whereused.grep.json \
  --tasks bench/abap-agent/tasks/b1_whereused.json \
  --out bench/abap-agent/ground_truth/whereused.grep.json
python3 scripts/abap-whereused-convert.test.py
```

The targets file is a JSON list of `{kind, owner, method, stratum}`: the 40
are the keys of the committed truth, with each one's kind and stratum in its
`_meta.targets`. The scripts default to `/tmp/claude-0/abapGit` and the
worktree's `crates/target/release/sem`.
