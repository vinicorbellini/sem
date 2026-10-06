# C1 ground truth review

Per-hit decisions behind `whereused-rust.json`. sem at `dc2cc4d7173d9f3eab87d491b351927fbe2f1f63` (upstream's merge-base), `crates/` only, which is every `.rs` file in the repository.

Every line of `crates/**/*.rs` that contains the target's name as a whole word, case-sensitive, is listed once,
as `file:line | decision | reason`. `yes` lines are the truth, with the calling function. A `no` line is a hit that does
not call the target: the reason names the receiver's static type or the function the call resolves to, or why the hit is
not a call. Receivers were typed by reading their declarations (let bindings, struct fields, function parameters and
return types, `Self`, the impl block a method sits in, `use` statements), not from sem. Comment and string-literal hits
were found with a scanner that blanks comments and literal text (`//`, nested `/* */`, `"..."`, raw strings `r#"..."#`;
char literals, not lifetimes); a hit inside a string is fixture source code or message text. A call written on one line
counts once, also when the line calls the target twice. No function or method of a target was named as a value without
a call (`.map(Type::method)`) anywhere; the non-call hits are variables, fields, parameters, module path segments and
`use` lines.

| Task | Target | Hits | yes | no |
|---|---|---:|---:|---:|
| c1_01 | `Lang::lower` | 74 | 3 | 71 |
| c1_02 | `Lang::layout` | 90 | 7 | 83 |
| c1_03 | `SemanticParserPlugin::extract_entities_with_tree` | 40 | 6 | 34 |
| c1_04 | `arch_view::names` | 1070 | 9 | 1061 |
| c1_05 | `QueryIndex::lookup` | 189 | 26 | 163 |
| c1_06 | `Lower::expr` | 127 | 19 | 108 |
| c1_07 | `util::fingerprint` | 99 | 5 | 94 |
| c1_08 | `QueryIndex::file_count` | 89 | 9 | 80 |
| c1_09 | `FastExtractorSet::identity` | 106 | 3 | 103 |
| c1_10 | `CloudClient::auth_header` | 30 | 14 | 16 |
| c1_11 | `telemetry::now_secs` | 11 | 4 | 7 |
| c1_12 | `SvelteLowerer::make_entity` | 40 | 4 | 36 |
| total | | 1965 | 109 | 1856 |

## How the twelve were picked

The rule is in `tasks/c1_whereused.json` (`selection`) and in the README ("C1 targets"); it was written into the task
file before any candidate's callers were read. Candidates per stratum, in (file, line) order of their declaration:
`trait` 5 (k = 1), `short` 113 (k = 37), `polluted` 22 (k = 7), `multi` 34 (k = 11). A rejected pick was read only as
far as the post-check needed: its callers below come from its call-shaped grep hits, read the same way, but its hits are
not listed line by line. No stratum ran out, so the hand-off rule was never used.

| Stratum | Candidates | k | Index | Candidate | Outcome |
|---|---:|---:|---:|---|---|
| trait | 5 | 1 | 0 | `Lang::lower` | **c1_01** |
| | | | 1 | `Lang::layout` | **c1_02** |
| | | | 2 | `FastExtractor::identity` | rejected: 1 calling function (`FastExtractorSet::new`); the other `identity(` calls are `FastExtractorSet::identity` and `Mutation::identity` |
| | | | 3 | `SemanticParserPlugin::extract_entities_brief` | rejected: 2 (`ParserRegistry::extract_entities_brief`, `tests::brief_extraction_drops_json_payloads` in `json.rs`) |
| | | | 4 | `SemanticParserPlugin::extract_entities_with_tree` | **c1_03** |
| short | 113 | 37 | 0 | `arch_view::names` | **c1_04** |
| | | | 37 | `QueryIndex::lookup` | **c1_05** |
| | | | 74 | `Lower::expr` (`parser/calls/rust.rs`) | **c1_06** |
| polluted | 22 | 7 | 0 | `util::fingerprint` (`commands/check/util.rs`) | **c1_07** |
| | | | 7 | `QueryIndex::edge_count` | rejected: 2 (`graph::try_index_graph`, `graph::write_graph_json_index`) |
| | | | 8 | `QueryIndex::file_count` | **c1_08** |
| | | | 14 | `PersistedFacts::file_count` | skipped: name `file_count` picked |
| | | | 15 | `ShardIndex::candidates` | rejected: 1 (`FactsCorpus::merge_with_local`) |
| | | | 16 | `FastExtractorSet::identity` | **c1_09** |
| multi | 34 | 11 | 0 | `lint::parse_args` | rejected: 2 (`lint::eslint`, `tests::eslint_script_args`) |
| | | | 1 | `check::load_config` | rejected: 1 (`check::evaluate`) |
| | | | 2 | `tests::normalize` (`commands/check/tests.rs`) | rejected: 1 (`tests::run`) |
| | | | 3 | `cloud::now_secs` | rejected: 2 (`cloud::login_hint_due`, `cloud::mark_login_hint_shown`) |
| | | | 4 | `CloudClient::auth_header` (`sem-cli/src/commands/cloud.rs`) | **c1_10** |
| | | | 11 | `qualified::normalize` | rejected: 1 (`qualified::near_matches`) |
| | | | 12 | `update::now_secs` | rejected: 2 (`update::maybe_notify`, `update::background_check`) |
| | | | 13 | `telemetry::now_secs` | **c1_11** |
| | | | 22 | `rust::normalize` (`parser/calls/rust.rs`) | rejected: 1 (`rust::crate_names`) |
| | | | 23 | `context::is_test_entity` | rejected: 2 (`context::build_context_result_bounded`, `context::lookup_is_test`) |
| | | | 24 | `graph::is_test_entity` | rejected: 12 calling functions, but 13 of 17 call-shaped hits are true (76%, over 75%) |
| | | | 25 | `SvelteLowerer::make_entity` | **c1_12** |

As in B1A, most rejections are private helpers called from one or two places in their own module: grep finds those
exactly too, so the post-check's floor of 3 callers is what removes them.


## c1_01 `Lang::lower`

Defined in `crates/sem-core/src/parser/calls/lang.rs`. The name `lower` is declared 8 times (`fn lower`, any kind).

```
crates/sem-cli/src/commands/diff/facts_remote.rs:148 | no | a variable, parameter or field named lower, not a call
crates/sem-cli/src/commands/diff/facts_remote.rs:149 | no | lower as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/diff/facts_remote.rs:155 | no | lower as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/diff/mod.rs:141 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/diff/mod.rs:271 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/topology.rs:598 | no | comment, not code
crates/sem-core/examples/facts_corpus_probe.rs:400 | no | a variable, parameter or field named lower, not a call
crates/sem-core/examples/facts_corpus_probe.rs:401 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/examples/facts_corpus_probe.rs:407 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:15 | no | the declaration fn lower, not a call
crates/sem-core/src/dataflow/lower.rs:44 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:48 | no | bare lower( in module dataflow::lower: the free fn dataflow::lower::lower
crates/sem-core/src/dataflow/mod.rs:6 | no | comment, not code
crates/sem-core/src/dataflow/mod.rs:24 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/mod.rs:128 | no | path segment lower:: (a module of that name), not a call
crates/sem-core/src/dataflow/mod.rs:129 | no | lower::lower(: the free fn dataflow::lower::lower
crates/sem-core/src/dataflow/witness.rs:303 | no | path segment lower:: (a module of that name), not a call
crates/sem-core/src/dataflow/witness.rs:458 | no | path segment lower:: (a module of that name), not a call
crates/sem-core/src/dataflow/witness.rs:476 | no | path segment lower:: (a module of that name), not a call
crates/sem-core/src/index/format.rs:526 | no | comment, not code
crates/sem-core/src/parser/calls/fit.rs:255 | yes | fit::python_misfits: lang: &dyn Lang (fit::python_misfits)
crates/sem-core/src/parser/calls/go.rs:20 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/go.rs:21 | no | bare lower( inside the impl's lower method: the module's free fn (go|python|rust)::lower, not the trait method
crates/sem-core/src/parser/calls/go.rs:73 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/go.rs:95 | no | comment, not code
crates/sem-core/src/parser/calls/ir.rs:9 | no | comment, not code
crates/sem-core/src/parser/calls/lang.rs:98 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/mod.rs:5 | no | comment, not code
crates/sem-core/src/parser/calls/mod.rs:247 | yes | calls::lower_file: lang from language_for(path): Option<&'static dyn Lang>
crates/sem-core/src/parser/calls/mod.rs:250 | no | comment, not code
crates/sem-core/src/parser/calls/mod.rs:259 | yes | calls::lower_source: lang = language_for(path)?: &'static dyn Lang
crates/sem-core/src/parser/calls/python.rs:24 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/python.rs:25 | no | bare lower( inside the impl's lower method: the module's free fn (go|python|rust)::lower, not the trait method
crates/sem-core/src/parser/calls/python.rs:125 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/rust.rs:20 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/calls/rust.rs:21 | no | bare lower( inside the impl's lower method: the module's free fn (go|python|rust)::lower, not the trait method
crates/sem-core/src/parser/calls/rust.rs:150 | no | the declaration fn lower, not a call
crates/sem-core/src/parser/facts_store.rs:1095 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/facts_store.rs:1098 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:1104 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/fast_extractor.rs:301 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/fast_extractor.rs:302 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:105 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:106 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:108 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:110 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:112 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/latex.rs:262 | no | comment, not code
crates/sem-core/src/parser/registry.rs:832 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/registry.rs:833 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/registry.rs:834 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/registry.rs:835 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/registry.rs:836 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/parser/registry.rs:840 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/contracts.rs:45 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/system/contracts.rs:46 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/contracts.rs:47 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/contracts.rs:48 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/contracts.rs:52 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/contracts.rs:61 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/system/sql.rs:200 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/system/sql.rs:205 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/system/world.rs:1391 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/system/world.rs:1394 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/topology/pyimports.rs:48 | no | path segment lower:: (a module of that name), not a call
crates/sem-core/src/utils/scan.rs:151 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/utils/scan.rs:153 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/utils/scan.rs:164 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/utils/scan.rs:169 | no | lower as a variable, field, argument or other value, not a call
crates/sem-core/src/utils/scan.rs:191 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/utils/scan.rs:195 | no | a variable, parameter or field named lower, not a call
crates/sem-core/src/utils/scan.rs:200 | no | lower as a variable, field, argument or other value, not a call
crates/sem-mcp/src/server.rs:741 | no | a variable, parameter or field named lower, not a call
crates/sem-mcp/src/server.rs:745 | no | a variable, parameter or field named lower, not a call
```

## c1_02 `Lang::layout`

Defined in `crates/sem-core/src/parser/calls/lang.rs`. The name `layout` is declared 7 times (`fn layout`, any kind).

```
crates/sem-core/examples/index_probe.rs:892 | no | comment, not code
crates/sem-core/examples/index_probe.rs:895 | no | comment, not code
crates/sem-core/src/index/format.rs:1 | no | comment, not code
crates/sem-core/src/index/format.rs:10 | no | comment, not code
crates/sem-core/src/index/format.rs:70 | no | comment, not code
crates/sem-core/src/index/mod.rs:6 | no | comment, not code
crates/sem-core/src/parser/calls/fit.rs:260 | yes | fit::python_misfits: lang: &dyn Lang
crates/sem-core/src/parser/calls/fit.rs:261 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:24 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/go.rs:25 | no | bare layout( inside the impl's layout method: the module's free fn layout, not the trait method
crates/sem-core/src/parser/calls/go.rs:1056 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/go.rs:1062 | no | a variable, parameter or field named layout, not a call
crates/sem-core/src/parser/calls/go.rs:1067 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:1069 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:1070 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:1120 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:1123 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/lang.rs:2 | no | comment, not code
crates/sem-core/src/parser/calls/lang.rs:42 | no | comment, not code
crates/sem-core/src/parser/calls/lang.rs:99 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/mod.rs:270 | yes | calls::resolve: lang: &dyn Lang parameter of calls::resolve
crates/sem-core/src/parser/calls/mod.rs:271 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:749 | yes | calls::site_answers: lang: &dyn Lang parameter of site_answers
crates/sem-core/src/parser/calls/mod.rs:750 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:831 | yes | calls::dispatch_answers: lang: &dyn Lang parameter of dispatch_answers
crates/sem-core/src/parser/calls/mod.rs:832 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:852 | yes | calls::explain: lang from language_for(target): &'static dyn Lang
crates/sem-core/src/parser/calls/mod.rs:853 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:861 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:862 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:990 | yes | calls::explain_path: lang from language_for(target): &'static dyn Lang
crates/sem-core/src/parser/calls/mod.rs:991 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:28 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/python.rs:30 | no | bare layout( inside the impl's layout method: the module's free fn layout, not the trait method
crates/sem-core/src/parser/calls/python.rs:1361 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/python.rs:1367 | no | a variable, parameter or field named layout, not a call
crates/sem-core/src/parser/calls/python.rs:1379 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1381 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1384 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1392 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1400 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1408 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1411 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1419 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1420 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1423 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1424 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1426 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1432 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1440 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1441 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1444 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:4 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:24 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/rust.rs:25 | no | bare layout( inside the impl's layout method: the module's free fn layout, not the trait method
crates/sem-core/src/parser/calls/rust.rs:1806 | no | the declaration fn layout, not a call
crates/sem-core/src/parser/calls/rust.rs:1868 | no | a variable, parameter or field named layout, not a call
crates/sem-core/src/parser/calls/rust.rs:1873 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:1874 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:209 | no | a variable, parameter or field named layout, not a call
crates/sem-core/src/parser/calls/scope.rs:217 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:224 | no | field access .layout, not a call
crates/sem-core/src/parser/calls/scope.rs:232 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:245 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:274 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:308 | no | a variable, parameter or field named layout, not a call
crates/sem-core/src/parser/calls/scope.rs:321 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:332 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:335 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:349 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:354 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/tests.rs:27 | yes | tests::edges_at: lang = language_for(..).unwrap(): &'static dyn Lang
crates/sem-core/src/parser/calls/tests.rs:28 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:682 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1123 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1258 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1624 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1797 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2039 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2949 | no | comment, not code
crates/sem-core/src/parser/registry.rs:926 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/scope_resolve.rs:5376 | no | comment, not code
crates/sem-core/src/system/locate.rs:49 | no | comment, not code
crates/sem-core/tests/bow_import_lookup_bench.rs:13 | no | a variable, parameter or field named layout, not a call
crates/sem-core/tests/bow_import_lookup_bench.rs:15 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/tests/bow_import_lookup_bench.rs:16 | no | layout as a variable, field, argument or other value, not a call
crates/sem-core/tests/bow_import_lookup_bench.rs:19 | no | a variable, parameter or field named layout, not a call
crates/sem-core/tests/bow_import_lookup_bench.rs:20 | no | layout as a variable, field, argument or other value, not a call
crates/sem-plugin/src/bindings.rs:978 | no | a variable, parameter or field named layout, not a call
crates/sem-plugin/src/bindings.rs:979 | no | layout as a variable, field, argument or other value, not a call
```

## c1_03 `SemanticParserPlugin::extract_entities_with_tree`

Defined in `crates/sem-core/src/parser/plugin.rs`. The name `extract_entities_with_tree` is declared 3 times (`fn extract_entities_with_tree`, any kind).

```
crates/sem-core/benches/parse_profile.rs:38 | yes | parse_profile::bench_split: plugin = CodeParserPlugin; method from its impl of SemanticParserPlugin
crates/sem-core/benches/parse_profile.rs:84 | yes | parse_profile::bench_extract: plugin = CodeParserPlugin (closure in bench_extract)
crates/sem-core/examples/parse_probe.rs:191 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/examples/parse_probe.rs:239 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/examples/perf_probe.rs:136 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/examples/perf_probe.rs:262 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/diff_oracle.rs:664 | yes | MutatingExtractor::extract: plugin = CodeParserPlugin, trait imported
crates/sem-core/src/parser/fast_extractor.rs:47 | no | comment, not code
crates/sem-core/src/parser/graph.rs:2232 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:2253 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:2296 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:2314 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:3350 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:3549 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:3732 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:9788 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:12554 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/graph.rs:12570 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/plugin.rs:14 | no | the declaration fn extract_entities_with_tree, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:20 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:168 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:183 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:199 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:216 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:460 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:466 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:479 | yes | CodeParserPlugin::extract_entities: self.extract_entities_with_tree inside impl SemanticParserPlugin for CodeParserPlugin
crates/sem-core/src/parser/plugins/code/mod.rs:484 | no | the declaration fn extract_entities_with_tree, not a call
crates/sem-core/src/parser/registry.rs:309 | no | the declaration fn extract_entities_with_tree, not a call
crates/sem-core/src/parser/registry.rs:318 | yes | ParserRegistry::extract_entities_with_tree: plugin = self.get_plugin_with_content(..)?: &dyn SemanticParserPlugin
crates/sem-core/src/parser/scope_resolve.rs:633 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:2392 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:9322 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/scope_resolve.rs:9683 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/scope_resolve.rs:9710 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/scope_resolve.rs:9804 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/src/parser/scope_resolve.rs:9839 | no | receiver registry is a ParserRegistry (create_default_registry() or a &ParserRegistry parameter): the inherent ParserRegistry::extract_entities_with_tree, not the trait method
crates/sem-core/tests/parse_cache.rs:4 | no | comment, not code
crates/sem-core/tests/parse_cache.rs:88 | no | comment, not code
crates/sem-core/tests/parse_cache.rs:90 | yes | parse_cache::cached_extraction_matches_the_uncached_path: plugin = CodeParserPlugin
```

## c1_04 `arch_view::names`

Defined in `crates/sem-cli/src/commands/arch_view.rs`. The name `names` is declared 3 times (`fn names`, any kind).

```
crates/sem-cli/src/alias.rs:1 | no | comment, not code
crates/sem-cli/src/commands/arch_diff.rs:1123 | no | comment, not code
crates/sem-cli/src/commands/arch_diff.rs:1138 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/arch_diff.rs:1271 | no | comment, not code
crates/sem-cli/src/commands/arch_view.rs:246 | no | comment, not code
crates/sem-cli/src/commands/arch_view.rs:434 | no | comment, not code
crates/sem-cli/src/commands/arch_view.rs:618 | yes | arch_view::flow_items: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:621 | yes | arch_view::flow_items: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:685 | yes | arch_view::merge_minor_flows: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:714 | no | the declaration fn names, not a call
crates/sem-cli/src/commands/arch_view.rs:770 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:774 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:780 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:995 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:996 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/arch_view.rs:1050 | yes | arch_view::build_view: bare names( in arch_view.rs: the private free fn arch_view::names
crates/sem-cli/src/commands/certify.rs:164 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/certify.rs:541 | no | a variable, parameter or field named names, not a call
crates/sem-cli/src/commands/certify.rs:544 | no | names is a closure bound by let names = |g, ids| in certify::certificate
crates/sem-cli/src/commands/certify.rs:545 | no | names is a closure bound by let names = |g, ids| in certify::certificate
crates/sem-cli/src/commands/check/lint.rs:59 | no | a variable, parameter or field named names, not a call
crates/sem-cli/src/commands/check/lint.rs:60 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/check/lint.rs:61 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/completeness.rs:194 | no | comment, not code
crates/sem-cli/src/commands/completeness.rs:463 | no | comment, not code
crates/sem-cli/src/commands/consent.rs:284 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/diff/facts_remote.rs:190 | no | comment, not code
crates/sem-cli/src/commands/graph.rs:164 | no | comment, not code
crates/sem-cli/src/commands/graph.rs:446 | no | comment, not code
crates/sem-cli/src/commands/hook.rs:37 | no | comment, not code
crates/sem-cli/src/commands/hook.rs:40 | no | a variable, parameter or field named names, not a call
crates/sem-cli/src/commands/hook.rs:42 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/impact.rs:449 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:8 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:27 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:87 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:139 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:148 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:165 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:182 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:224 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:284 | no | comment, not code
crates/sem-cli/src/commands/query.rs:749 | no | comment, not code
crates/sem-cli/src/commands/query.rs:908 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/region.rs:8 | no | comment, not code
crates/sem-cli/src/commands/region.rs:18 | no | comment, not code
crates/sem-cli/src/commands/region.rs:19 | no | comment, not code
crates/sem-cli/src/commands/region.rs:21 | no | comment, not code
crates/sem-cli/src/commands/region.rs:24 | no | comment, not code
crates/sem-cli/src/commands/region.rs:29 | no | comment, not code
crates/sem-cli/src/commands/region.rs:120 | no | comment, not code
crates/sem-cli/src/commands/region.rs:126 | no | comment, not code
crates/sem-cli/src/commands/region.rs:170 | no | a variable, parameter or field named names, not a call
crates/sem-cli/src/commands/region.rs:171 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/region.rs:172 | no | comment, not code
crates/sem-cli/src/commands/region.rs:190 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/region.rs:207 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/topology.rs:64 | no | comment, not code
crates/sem-cli/src/commands/topology.rs:594 | no | comment, not code
crates/sem-cli/src/commands/update.rs:357 | no | a variable, parameter or field named names, not a call
crates/sem-cli/src/commands/update.rs:359 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/update.rs:367 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/src/hyperlinks.rs:3 | no | comment, not code
crates/sem-cli/src/main.rs:231 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/main.rs:242 | no | comment, not code
crates/sem-cli/src/main.rs:559 | no | comment, not code
crates/sem-cli/src/telemetry.rs:4 | no | comment, not code
crates/sem-cli/src/telemetry.rs:9 | no | comment, not code
crates/sem-cli/src/telemetry.rs:236 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/telemetry.rs:241 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/telemetry.rs:378 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/telemetry.rs:417 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/telemetry.rs:450 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/tests/cli_simplify.rs:710 | no | a variable, parameter or field named names, not a call
crates/sem-cli/tests/cli_simplify.rs:714 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/tests/cli_simplify.rs:715 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/tests/cli_simplify.rs:1046 | no | a variable, parameter or field named names, not a call
crates/sem-cli/tests/cli_simplify.rs:1053 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/tests/cli_simplify.rs:1141 | no | a variable, parameter or field named names, not a call
crates/sem-cli/tests/cli_simplify.rs:1148 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/tests/context_cli.rs:206 | no | comment, not code
crates/sem-cli/tests/context_cli.rs:271 | no | a variable, parameter or field named names, not a call
crates/sem-cli/tests/context_cli.rs:278 | no | names as a variable, field, argument or other value, not a call
crates/sem-cli/tests/context_cli.rs:279 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/tests/impact_direct_deps.rs:511 | no | comment, not code
crates/sem-cli/tests/multi_query_cli.rs:2 | no | comment, not code
crates/sem-cli/tests/multi_query_cli.rs:196 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/tests/qualified_names_cli.rs:1 | no | comment, not code
crates/sem-cli/tests/qualified_names_cli.rs:105 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/tests/qualified_names_cli.rs:152 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/tests/qualified_names_cli.rs:205 | no | comment, not code
crates/sem-core/examples/index_probe.rs:196 | no | a variable, parameter or field named names, not a call
crates/sem-core/examples/index_probe.rs:213 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/examples/index_probe.rs:288 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/examples/index_probe.rs:337 | no | comment, not code
crates/sem-core/examples/index_probe.rs:341 | no | comment, not code
crates/sem-core/examples/index_probe.rs:1010 | no | a variable, parameter or field named names, not a call
crates/sem-core/examples/index_probe.rs:1023 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/examples/index_probe.rs:1055 | no | a variable, parameter or field named names, not a call
crates/sem-core/examples/index_probe.rs:1069 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/examples/mul_census.rs:24 | no | comment, not code
crates/sem-core/examples/mul_census.rs:126 | no | comment, not code
crates/sem-core/examples/mul_census.rs:137 | no | comment, not code
crates/sem-core/examples/mul_census.rs:142 | no | comment, not code
crates/sem-core/examples/mul_census.rs:348 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:626 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/engine.rs:631 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:654 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:666 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:809 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:973 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:974 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:1047 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:1103 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:1238 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/engine.rs:1246 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1284 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1294 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1510 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/engine.rs:1511 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1513 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1514 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/engine.rs:1604 | no | comment, not code
crates/sem-core/src/dataflow/ir.rs:5 | no | comment, not code
crates/sem-core/src/dataflow/ir.rs:116 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:2 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:6 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:74 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:108 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:113 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:423 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:425 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:429 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:430 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:431 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:435 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:436 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:439 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:445 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:446 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:468 | no | comment, not code
crates/sem-core/src/dataflow/lower.rs:703 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:708 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:974 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/lower.rs:975 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/lower.rs:978 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/mod.rs:8 | no | comment, not code
crates/sem-core/src/dataflow/mod.rs:13 | no | comment, not code
crates/sem-core/src/dataflow/mod.rs:43 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/models.rs:4 | no | comment, not code
crates/sem-core/src/dataflow/models.rs:29 | no | comment, not code
crates/sem-core/src/dataflow/models.rs:50 | no | comment, not code
crates/sem-core/src/dataflow/models.rs:87 | no | comment, not code
crates/sem-core/src/dataflow/models.rs:132 | no | comment, not code
crates/sem-core/src/dataflow/models.rs:172 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/models.rs:177 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/models.rs:178 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/models.rs:209 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/models.rs:253 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/models.rs:339 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/dataflow/models.rs:344 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/dataflow/tests.rs:276 | no | comment, not code
crates/sem-core/src/dataflow/witness.rs:15 | no | comment, not code
crates/sem-core/src/dataflow/witness.rs:185 | no | comment, not code
crates/sem-core/src/dataflow/witness.rs:278 | no | comment, not code
crates/sem-core/src/git/bridge.rs:1448 | no | comment, not code
crates/sem-core/src/index/mod.rs:223 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/mod.rs:225 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/mod.rs:263 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/mod.rs:264 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/mod.rs:330 | no | comment, not code
crates/sem-core/src/index/reader.rs:416 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/reader.rs:420 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/reader.rs:423 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/reader.rs:424 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/reader.rs:542 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/reader.rs:543 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/reader.rs:546 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/reader.rs:547 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:589 | no | comment, not code
crates/sem-core/src/index/writer.rs:817 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/index/writer.rs:833 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:885 | no | comment, not code
crates/sem-core/src/index/writer.rs:892 | no | comment, not code
crates/sem-core/src/model/identity.rs:17 | no | comment, not code
crates/sem-core/src/parser/calls/fit.rs:3 | no | comment, not code
crates/sem-core/src/parser/calls/go.rs:3 | no | comment, not code
crates/sem-core/src/parser/calls/go.rs:95 | no | comment, not code
crates/sem-core/src/parser/calls/go.rs:300 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:301 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:308 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:354 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:363 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:364 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:534 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:535 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:551 | no | comment, not code
crates/sem-core/src/parser/calls/go.rs:571 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:573 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:581 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:585 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:589 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:661 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:662 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:667 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/go.rs:682 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:683 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/infer.rs:57 | no | comment, not code
crates/sem-core/src/parser/calls/infer.rs:431 | no | comment, not code
crates/sem-core/src/parser/calls/infer.rs:472 | no | comment, not code
crates/sem-core/src/parser/calls/ir.rs:129 | no | comment, not code
crates/sem-core/src/parser/calls/ir.rs:290 | no | comment, not code
crates/sem-core/src/parser/calls/ir.rs:344 | no | comment, not code
crates/sem-core/src/parser/calls/mod.rs:619 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:625 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:629 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:632 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/mod.rs:639 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:642 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:645 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:648 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:651 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:654 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:676 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/mod.rs:682 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:3 | no | comment, not code
crates/sem-core/src/parser/calls/python.rs:351 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/python.rs:352 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:388 | no | comment, not code
crates/sem-core/src/parser/calls/python.rs:1256 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:207 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:972 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:973 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:1068 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/rust.rs:1069 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:1070 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:1798 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:1829 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:1904 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:1907 | no | comment, not code
crates/sem-core/src/parser/calls/rust.rs:1908 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:54 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:78 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:90 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:93 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:152 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:170 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/scope.rs:172 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:174 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:175 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:179 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:336 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:385 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:422 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/scope.rs:423 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:434 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:437 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:441 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:447 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/calls/scope.rs:453 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:457 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/scope.rs:582 | no | comment, not code
crates/sem-core/src/parser/calls/select.rs:377 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:21 | no | comment, not code
crates/sem-core/src/parser/differ.rs:975 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/differ.rs:982 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:989 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1006 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/differ.rs:1016 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1023 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1040 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/differ.rs:1050 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1057 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1074 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/differ.rs:1081 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:1088 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/facts_store.rs:250 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:930 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:934 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:935 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:972 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1328 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2239 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2581 | no | comment, not code
crates/sem-core/src/parser/graph.rs:372 | no | comment, not code
crates/sem-core/src/parser/graph.rs:1745 | no | comment, not code
crates/sem-core/src/parser/graph.rs:2678 | no | comment, not code
crates/sem-core/src/parser/graph.rs:2919 | no | comment, not code
crates/sem-core/src/parser/graph.rs:3891 | no | comment, not code
crates/sem-core/src/parser/graph.rs:5249 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:5251 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5254 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5262 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5276 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5281 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5914 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:5915 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:6056 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6138 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:6139 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:6221 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6521 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:6796 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:6797 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7470 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7472 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7487 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7496 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7527 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7539 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7551 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7561 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7620 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7623 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7673 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7674 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7735 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7745 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7781 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:7791 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:7937 | no | comment, not code
crates/sem-core/src/parser/graph.rs:8267 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/graph.rs:8277 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:8278 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:11784 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:309 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/import_resolution.rs:312 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:318 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:326 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:331 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:346 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/import_resolution.rs:349 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:355 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:360 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/import_resolution.rs:418 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:479 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:233 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:239 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:2084 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:2107 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:2200 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:2533 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3179 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3192 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3193 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3194 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3197 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3201 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:3217 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/languages.rs:68 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:585 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:589 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:593 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:595 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:598 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:600 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:603 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:605 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:636 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:646 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:650 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:652 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:655 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:657 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:701 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:705 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:709 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:711 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:714 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:716 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:719 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:721 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:724 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:726 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:729 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:731 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:749 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:751 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:752 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:753 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:754 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:755 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:763 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:764 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:765 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:766 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:786 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:788 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:789 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:790 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:791 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:792 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:793 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:801 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:802 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:803 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:804 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:812 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:813 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:814 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:815 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:870 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:880 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:882 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:885 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:887 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:890 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:892 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:895 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:897 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:900 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:902 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:905 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:907 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:910 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:912 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:915 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:917 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:920 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:922 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:925 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:927 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:930 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:932 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1170 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1172 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1173 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1174 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1175 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1176 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1177 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1178 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1179 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1180 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1181 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1182 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1183 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1184 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1197 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1199 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1214 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1216 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1217 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1218 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1220 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1222 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1225 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1227 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1364 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1369 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1379 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1420 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1424 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1428 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1430 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1433 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1435 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1438 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1440 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1443 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1445 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1448 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1450 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1497 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1507 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1509 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1512 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1514 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1517 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1519 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1522 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1524 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1527 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1529 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1532 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1534 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1537 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1539 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1542 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1544 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1547 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1549 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1551 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:1553 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1555 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1558 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1560 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1602 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1604 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1606 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1609 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1611 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1632 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1634 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1636 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1639 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1641 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1700 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1702 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1704 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1707 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1709 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1729 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1731 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1733 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1736 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1738 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1741 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1743 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1746 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1748 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1775 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1779 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1783 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1785 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1788 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1790 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1796 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1852 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1856 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1858 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1861 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1863 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1866 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1868 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1871 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1873 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1875 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1886 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1888 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1889 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1891 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1892 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1924 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1926 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1928 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1930 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1933 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1935 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1937 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1956 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1957 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1958 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2016 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2018 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2032 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2034 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2035 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2095 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2105 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2107 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2110 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2112 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2176 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2178 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2179 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2190 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2192 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2193 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2228 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2234 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2236 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2237 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2238 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2247 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2249 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2361 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2372 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2374 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2377 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2379 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2382 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2384 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2387 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2389 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2392 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2394 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2397 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2399 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2443 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2451 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2452 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2453 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2454 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2455 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2456 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2482 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2490 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2491 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2492 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2529 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2541 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2542 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2543 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2546 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2547 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2548 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2570 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2572 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2574 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2577 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2579 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2582 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2584 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2605 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2607 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2609 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2612 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2614 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2628 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2630 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2632 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2635 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2637 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2711 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2714 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2715 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2716 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2717 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2718 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2734 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2737 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2738 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2739 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2740 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2812 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2827 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2828 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2829 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2830 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2831 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2858 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2862 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2866 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2868 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2871 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2873 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2876 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2878 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2900 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2904 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2908 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2910 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2913 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2915 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2918 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2920 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2982 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2993 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2995 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:2998 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3000 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3003 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3005 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3008 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3010 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3013 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3015 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3018 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3020 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3023 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3025 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3047 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3061 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3117 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:3445 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3458 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3489 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3502 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3545 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3558 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3579 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3592 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3613 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3626 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3657 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3670 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3692 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3705 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3732 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3734 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3735 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3736 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3742 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3773 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3775 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3776 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3777 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3778 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3784 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3829 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3839 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3841 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3844 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3846 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3849 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3851 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3854 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3856 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3896 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3906 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3908 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3911 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3913 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3916 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3918 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3921 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3923 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3926 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3928 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3972 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3979 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3981 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3983 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3985 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3987 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3990 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3992 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3995 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:3997 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4000 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4002 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4020 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4027 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4029 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4032 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4034 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4037 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4039 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4042 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4045 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4059 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4061 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4062 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4063 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4064 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4065 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4084 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4086 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/mod.rs:4087 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:763 | no | the declaration fn names, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:794 | no | the test helper tests::names in oxc_extractor.rs
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:800 | no | the test helper tests::names in oxc_extractor.rs
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:806 | no | the test helper tests::names in oxc_extractor.rs
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:812 | no | the test helper tests::names in oxc_extractor.rs
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:853 | no | the test helper tests::names in oxc_extractor.rs
crates/sem-core/src/parser/plugins/erb.rs:289 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/erb.rs:293 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:321 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:322 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:336 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:339 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:399 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/erb.rs:403 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/erb.rs:413 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/json.rs:668 | no | the declaration fn names, not a call
crates/sem-core/src/parser/plugins/json.rs:688 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:784 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:791 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:797 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:822 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:839 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:856 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:901 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:977 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1044 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1074 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1200 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1209 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1220 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1290 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1342 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1435 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/json.rs:1469 | no | the test helper tests::names in json.rs
crates/sem-core/src/parser/plugins/markdown.rs:311 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/markdown.rs:314 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/markdown.rs:315 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/svelte.rs:1030 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1033 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1035 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1038 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1040 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1043 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1045 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1048 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1050 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1053 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1055 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1058 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1060 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1063 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1065 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1116 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1119 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1121 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1124 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1126 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1129 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1131 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1134 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1136 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1139 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1141 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1156 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1159 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1161 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1164 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1166 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1169 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1171 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1174 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1176 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1235 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1242 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1244 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1270 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1273 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1275 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1278 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1280 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1283 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1285 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1298 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1305 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1307 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1310 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1312 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1331 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1334 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1336 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1339 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1341 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1343 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1358 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1361 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1363 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1366 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1368 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1371 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1373 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1376 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1378 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1394 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1401 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1403 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1406 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1408 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1411 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1413 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1416 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1418 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1429 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1432 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1434 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1437 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1439 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1449 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1456 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1458 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/svelte.rs:1942 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/vue.rs:238 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/vue.rs:242 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:246 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:248 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:251 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:253 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:256 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:258 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:261 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:263 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:292 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/vue.rs:302 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:304 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:307 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:309 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:312 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:314 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:317 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/vue.rs:319 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/yaml.rs:335 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/plugins/yaml.rs:336 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/yaml.rs:337 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/registry.rs:469 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:294 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:341 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:93 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:178 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:736 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:1185 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:1562 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3150 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3507 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:3538 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3554 | no | field access .names, not a call
crates/sem-core/src/parser/scope_resolve.rs:4543 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:4549 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:4556 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:4557 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:4569 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:4576 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:5012 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:5536 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:5537 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:5825 | no | field access .names, not a call
crates/sem-core/src/parser/scope_resolve.rs:5970 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:5975 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:5994 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:5999 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:6007 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:6018 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:6052 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:6072 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:6245 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:6853 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:6987 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/parser/scope_resolve.rs:7008 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7073 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7074 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7182 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7837 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7867 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8152 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8363 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8409 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:9428 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:9656 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:10747 | no | comment, not code
crates/sem-core/src/parser/session.rs:45 | no | comment, not code
crates/sem-core/src/parser/session.rs:1131 | no | comment, not code
crates/sem-core/src/parser/session.rs:1311 | no | comment, not code
crates/sem-core/src/parser/session.rs:2240 | no | comment, not code
crates/sem-core/src/parser/test_detect.rs:36 | no | comment, not code
crates/sem-core/src/parser/test_detect.rs:49 | no | comment, not code
crates/sem-core/src/parser/test_detect.rs:55 | no | comment, not code
crates/sem-core/src/parser/test_detect.rs:60 | no | comment, not code
crates/sem-core/src/system/config.rs:28 | no | comment, not code
crates/sem-core/src/system/lockfiles.rs:44 | no | comment, not code
crates/sem-core/src/system/lockfiles.rs:90 | no | comment, not code
crates/sem-core/src/system/lockfiles.rs:211 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/system/lockfiles.rs:214 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/lockfiles.rs:222 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/lockfiles.rs:229 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/lockfiles.rs:328 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/system/lockfiles.rs:329 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/models.rs:161 | no | comment, not code
crates/sem-core/src/system/scan.rs:4 | no | comment, not code
crates/sem-core/src/system/scan.rs:5 | no | comment, not code
crates/sem-core/src/system/scan.rs:7 | no | comment, not code
crates/sem-core/src/system/sql.rs:7 | no | comment, not code
crates/sem-core/src/system/sql.rs:215 | no | comment, not code
crates/sem-core/src/system/sql.rs:232 | no | comment, not code
crates/sem-core/src/system/world.rs:1331 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/system/world.rs:1340 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/world.rs:1538 | no | comment, not code
crates/sem-core/src/system/world.rs:1884 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/system/world.rs:1888 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/world.rs:1889 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/world.rs:1893 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/world.rs:1894 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/system/world.rs:1935 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/system/world.rs:1939 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/graph.rs:32 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/graph.rs:274 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/graph.rs:275 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/graph.rs:276 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/graph.rs:333 | no | comment, not code
crates/sem-core/src/topology/graph.rs:442 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/graph.rs:443 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/graph.rs:444 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/graph.rs:445 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/metrics.rs:61 | no | comment, not code
crates/sem-core/src/topology/pattern.rs:72 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/pattern.rs:78 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:4 | no | comment, not code
crates/sem-core/src/topology/pyimports.rs:5 | no | comment, not code
crates/sem-core/src/topology/pyimports.rs:51 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:54 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:59 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:67 | no | comment, not code
crates/sem-core/src/topology/pyimports.rs:75 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/pyimports.rs:79 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:81 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/pyimports.rs:115 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/pyimports.rs:130 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/query_scope.rs:16 | no | comment, not code
crates/sem-core/src/topology/query_scope.rs:193 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/query_scope.rs:195 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/query_scope.rs:247 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/topology/resolve.rs:150 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/resolve.rs:156 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/resolve.rs:157 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/resolve.rs:167 | no | comment, not code
crates/sem-core/src/topology/tsconfig.rs:2 | no | comment, not code
crates/sem-core/src/topology/workspace.rs:32 | no | comment, not code
crates/sem-core/src/topology/workspace.rs:141 | no | comment, not code
crates/sem-core/src/topology/workspace.rs:234 | no | a variable, parameter or field named names, not a call
crates/sem-core/src/topology/workspace.rs:240 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/topology/workspace.rs:241 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/src/utils/scan.rs:1 | no | comment, not code
crates/sem-core/src/utils/scan.rs:15 | no | comment, not code
crates/sem-core/tests/d_smoke.rs:71 | no | a variable, parameter or field named names, not a call
crates/sem-core/tests/d_smoke.rs:81 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:83 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:86 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:88 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:91 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:93 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:96 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:98 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:101 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:103 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:106 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:108 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:111 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:113 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:116 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:118 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:121 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:123 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:126 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:128 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:149 | no | a variable, parameter or field named names, not a call
crates/sem-core/tests/d_smoke.rs:150 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:153 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:155 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:158 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:160 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:163 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/d_smoke.rs:165 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:43 | no | a variable, parameter or field named names, not a call
crates/sem-core/tests/elm_smoke.rs:45 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:47 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:50 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:52 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:55 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:57 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:60 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:62 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:65 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:67 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:70 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:72 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:94 | no | a variable, parameter or field named names, not a call
crates/sem-core/tests/elm_smoke.rs:96 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:98 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:102 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/elm_smoke.rs:104 | no | names as a variable, field, argument or other value, not a call
crates/sem-core/tests/laws_extraction.rs:26 | no | comment, not code
crates/sem-core/tests/scope_resolve_bench.rs:49 | no | comment, not code
crates/sem-core/tests/stem_collision_ground_truth.rs:242 | no | comment, not code
crates/sem-core/tests/stem_collision_ground_truth.rs:288 | no | comment, not code
crates/sem-core/tests/stem_collision_ground_truth.rs:316 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/tests/stem_collision_ground_truth.rs:325 | no | comment, not code
crates/sem-core/tests/stem_collision_ground_truth.rs:362 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/tests/stem_collision_ground_truth.rs:370 | no | comment, not code
crates/sem-core/tests/stem_collision_ground_truth.rs:397 | no | inside a string literal (fixture source or message text), not a call
crates/sem-mcp/src/agent_review.rs:456 | no | comment, not code
crates/sem-mcp/src/render.rs:41 | no | comment, not code
crates/sem-mcp/src/render.rs:69 | no | a variable, parameter or field named names, not a call
crates/sem-mcp/src/render.rs:80 | no | names as a variable, field, argument or other value, not a call
crates/sem-mcp/src/server.rs:88 | no | comment, not code
crates/sem-mcp/src/server.rs:2454 | no | comment, not code
crates/sem-mcp/src/tools.rs:178 | no | inside a string literal (fixture source or message text), not a call
crates/sem-mcp/tests/mcp_protocol.rs:379 | no | a variable, parameter or field named names, not a call
crates/sem-mcp/tests/mcp_protocol.rs:385 | no | names as a variable, field, argument or other value, not a call
crates/sem-mcp/tests/mcp_protocol.rs:386 | no | inside a string literal (fixture source or message text), not a call
crates/sem-mcp/tests/mcp_protocol.rs:389 | no | names as a variable, field, argument or other value, not a call
crates/sem-mcp/tests/mcp_protocol.rs:390 | no | inside a string literal (fixture source or message text), not a call
crates/sem-mcp/tests/mcp_protocol.rs:428 | no | comment, not code
crates/sem-mcp/tests/mcp_protocol.rs:459 | no | comment, not code
```

## c1_05 `QueryIndex::lookup`

Defined in `crates/sem-core/src/index/reader.rs`. The name `lookup` is declared 4 times (`fn lookup`, any kind).

```
crates/sem-cli/src/commands/certify.rs:765 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/completeness.rs:562 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/context.rs:53 | no | comment, not code
crates/sem-cli/src/commands/context.rs:553 | no | comment, not code
crates/sem-cli/src/commands/entities.rs:102 | no | comment, not code
crates/sem-cli/src/commands/entities.rs:794 | no | comment, not code
crates/sem-cli/src/commands/entities.rs:795 | no | comment, not code
crates/sem-cli/src/commands/entities.rs:822 | no | inside a string literal (fixture source or message text), not a call
crates/sem-cli/src/commands/graph.rs:388 | no | comment, not code
crates/sem-cli/src/commands/hook.rs:110 | no | comment, not code
crates/sem-cli/src/commands/impact.rs:909 | no | comment, not code
crates/sem-cli/src/commands/mod.rs:66 | no | comment, not code
crates/sem-cli/src/commands/qualified.rs:228 | yes | qualified::resolve_index: idx: &QueryIndex parameter of resolve_index
crates/sem-cli/src/commands/qualified.rs:310 | yes | qualified::near_matches: idx: &QueryIndex parameter of near_matches
crates/sem-cli/src/commands/query.rs:481 | no | comment, not code
crates/sem-cli/src/commands/query.rs:651 | yes | query::index_answer_verified: idx: &QueryIndex parameter of index_answer_verified
crates/sem-cli/src/commands/query.rs:787 | yes | query::resolve_by_name_indices: idx: &QueryIndex parameter of resolve_by_name_indices
crates/sem-cli/src/commands/topology.rs:468 | no | the declaration fn lookup, not a call
crates/sem-cli/src/commands/topology.rs:473 | no | bare lookup(g, ..): the private free fn topology::lookup
crates/sem-cli/src/commands/topology.rs:483 | no | bare lookup(g, ..): the private free fn topology::lookup
crates/sem-cli/src/commands/topology.rs:496 | no | bare lookup(g, ..): the private free fn topology::lookup
crates/sem-core/benches/parse_profile.rs:82 | no | comment, not code
crates/sem-core/benches/parse_profile.rs:170 | no | comment, not code
crates/sem-core/examples/index_probe.rs:3 | no | comment, not code
crates/sem-core/examples/index_probe.rs:11 | no | comment, not code
crates/sem-core/examples/index_probe.rs:22 | no | comment, not code
crates/sem-core/examples/index_probe.rs:36 | no | comment, not code
crates/sem-core/examples/index_probe.rs:37 | no | comment, not code
crates/sem-core/examples/index_probe.rs:68 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/examples/index_probe.rs:80 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/examples/index_probe.rs:241 | yes | index_probe::oracle: index = QueryIndex::from_bytes(..)
crates/sem-core/examples/index_probe.rs:335 | no | comment, not code
crates/sem-core/examples/index_probe.rs:347 | no | comment, not code
crates/sem-core/examples/index_probe.rs:1025 | yes | index_probe::lookup_mode: index from QueryIndex::open(..)
crates/sem-core/examples/index_probe.rs:1053 | no | comment, not code
crates/sem-core/examples/index_probe.rs:1071 | yes | index_probe::refs_mode: index from QueryIndex::open(..)
crates/sem-core/examples/parse_probe.rs:279 | no | comment, not code
crates/sem-core/src/dataflow/engine.rs:587 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:672 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:781 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:884 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1053 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1064 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1281 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1370 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1486 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1625 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/engine.rs:1920 | no | self.inp.models: &Models: Models::lookup
crates/sem-core/src/dataflow/models.rs:367 | no | the declaration fn lookup, not a call
crates/sem-core/src/dataflow/models.rs:415 | no | m = Models::builtin(): Models::lookup
crates/sem-core/src/dataflow/models.rs:417 | no | m = Models::builtin(): Models::lookup
crates/sem-core/src/dataflow/tests.rs:106 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/tests.rs:109 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/tests.rs:399 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/tests.rs:402 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/tests.rs:409 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/git/bridge.rs:34 | no | comment, not code
crates/sem-core/src/index/mod.rs:187 | yes | tests::lookup_returns_every_entity_with_that_name: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:197 | yes | tests::lookup_round_trips_every_field: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:212 | yes | tests::lookup_misses_are_empty_not_errors: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:213 | yes | tests::lookup_misses_are_empty_not_errors: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:233 | yes | tests::lookup_agrees_with_the_graph_for_every_name: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:254 | yes | tests::parent_ids_survive_the_index_relative_encoding: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:383 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/index/mod.rs:430 | yes | tests::ids_that_break_the_prefix_rule_fall_back_to_whole_ids: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:432 | yes | tests::ids_that_break_the_prefix_rule_fall_back_to_whole_ids: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:452 | yes | tests::json_pointer_ids_round_trip_under_elision: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:477 | yes | tests::an_empty_graph_produces_a_valid_empty_index: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:531 | yes | tests::refs_of_returns_direct_dependencies: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:546 | yes | tests::callers_of_returns_direct_dependents_across_files: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:564 | yes | tests::refs_and_callers_are_empty_for_leaf_and_unreferenced_entities: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:566 | yes | tests::refs_and_callers_are_empty_for_leaf_and_unreferenced_entities: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:587 | yes | tests::refs_and_callers_agree_with_the_graph_for_every_entity: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:662 | yes | tests::refs_of_typed_recovers_each_edges_kind: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:688 | yes | tests::callers_of_typed_recovers_each_edges_kind: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:715 | yes | tests::untyped_refs_and_callers_still_resolve_the_right_entity_when_kind_is_nonzero: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:724 | yes | tests::untyped_refs_and_callers_still_resolve_the_right_entity_when_kind_is_nonzero: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/reader.rs:3 | no | comment, not code
crates/sem-core/src/index/reader.rs:415 | no | the declaration fn lookup, not a call
crates/sem-core/src/index/reader.rs:597 | no | comment, not code
crates/sem-core/src/model/entity_id.rs:5 | no | comment, not code
crates/sem-core/src/model/identity.rs:232 | no | comment, not code
crates/sem-core/src/parser/calls/infer.rs:469 | no | self.scopes: Scopes: Scopes::lookup
crates/sem-core/src/parser/calls/ir.rs:43 | no | comment, not code
crates/sem-core/src/parser/calls/lang.rs:104 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:58 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:61 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:64 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:66 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:68 | no | comment, not code
crates/sem-core/src/parser/calls/scope.rs:532 | no | the declaration fn lookup, not a call
crates/sem-core/src/parser/context.rs:135 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:19 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:30 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:616 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:618 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:621 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:624 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:685 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:708 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:761 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:866 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:984 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1121 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1193 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1298 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1474 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1700 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2519 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2754 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/facts_store.rs:2761 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/facts_store.rs:2788 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2832 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:3063 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:66 | no | comment, not code
crates/sem-core/src/parser/graph.rs:78 | no | comment, not code
crates/sem-core/src/parser/graph.rs:809 | no | comment, not code
crates/sem-core/src/parser/graph.rs:1524 | no | comment, not code
crates/sem-core/src/parser/graph.rs:1631 | no | comment, not code
crates/sem-core/src/parser/graph.rs:2858 | no | comment, not code
crates/sem-core/src/parser/graph.rs:2961 | no | comment, not code
crates/sem-core/src/parser/graph.rs:3329 | no | comment, not code
crates/sem-core/src/parser/graph.rs:3721 | no | comment, not code
crates/sem-core/src/parser/graph.rs:3771 | no | comment, not code
crates/sem-core/src/parser/graph.rs:4195 | no | comment, not code
crates/sem-core/src/parser/graph.rs:4627 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6196 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:91 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:767 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:11 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:23 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:24 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:193 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:239 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:240 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:272 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:281 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:364 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:1751 | no | comment, not code
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:1821 | no | comment, not code
crates/sem-core/src/parser/plugins/code/languages.rs:136 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:1050 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/mod.rs:1060 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/latex.rs:313 | no | comment, not code
crates/sem-core/src/parser/registry.rs:81 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:17 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:51 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:173 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:336 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:476 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:518 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:563 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:1005 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:596 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:1685 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:2730 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3063 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3435 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3546 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3791 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3803 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:4044 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:6622 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:6626 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:6860 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7959 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7961 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:7971 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8063 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8504 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:8835 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:9033 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/scope_resolve.rs:10609 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:462 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:1025 | no | comment, not code
crates/sem-core/src/system/models.rs:87 | no | a variable, parameter or field named lookup, not a call
crates/sem-core/src/system/world.rs:116 | no | comment, not code
crates/sem-core/src/system/world.rs:1302 | no | comment, not code
crates/sem-core/src/system/world.rs:1531 | no | field access .lookup, not a call
crates/sem-core/src/system/world.rs:1538 | no | comment, not code
crates/sem-core/src/topology/resolve.rs:194 | no | comment, not code
crates/sem-core/src/topology/resolve.rs:237 | no | comment, not code
crates/sem-core/src/utils/scan.rs:17 | no | comment, not code
crates/sem-core/tests/yaml_multidoc.rs:138 | no | comment, not code
crates/sem-mcp/src/agent_review.rs:62 | no | comment, not code
crates/sem-mcp/src/server.rs:716 | no | comment, not code
crates/sem-mcp/src/server.rs:1475 | no | comment, not code
crates/sem-mcp/src/server.rs:2326 | no | comment, not code
crates/sem-mcp/src/tools.rs:122 | no | inside a string literal (fixture source or message text), not a call
```

## c1_06 `Lower::expr`

Defined in `crates/sem-core/src/parser/calls/rust.rs`. The name `expr` is declared 4 times (`fn expr`, any kind).

```
crates/sem-core/src/dataflow/witness.rs:10 | no | comment, not code
crates/sem-core/src/dataflow/witness.rs:161 | no | comment, not code
crates/sem-core/src/dataflow/witness.rs:162 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/witness.rs:374 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/dataflow/witness.rs:481 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/git/bridge.rs:20 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/index/complete.rs:34 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:20 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:40 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/fit.rs:280 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/fit.rs:281 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/go.rs:365 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:582 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:684 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:687 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:728 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:729 | no | self.f.expr(: FileFacts::expr
crates/sem-core/src/parser/calls/go.rs:743 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:758 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/go.rs:767 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:778 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/go.rs:797 | no | the declaration fn expr, not a call
crates/sem-core/src/parser/calls/go.rs:815 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:836 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:858 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:881 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:885 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:896 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/go.rs:902 | no | self is go.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/infer.rs:312 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:333 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:367 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:436 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:459 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:642 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/infer.rs:683 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/ir.rs:305 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/ir.rs:368 | no | the declaration fn expr, not a call
crates/sem-core/src/parser/calls/mod.rs:355 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/mod.rs:519 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/mod.rs:767 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/mod.rs:796 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/mod.rs:846 | no | comment, not code
crates/sem-core/src/parser/calls/mod.rs:866 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/mod.rs:884 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/mod.rs:891 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/mod.rs:909 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/mod.rs:937 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/mod.rs:946 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/python.rs:265 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:483 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:767 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:790 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:815 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:850 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:858 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:863 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:987 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:988 | no | self.f.expr(: FileFacts::expr
crates/sem-core/src/parser/calls/python.rs:993 | no | self.f.expr(: FileFacts::expr
crates/sem-core/src/parser/calls/python.rs:1007 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1024 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1027 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/python.rs:1036 | no | the declaration fn expr, not a call
crates/sem-core/src/parser/calls/python.rs:1054 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1069 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1110 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1117 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1132 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1140 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1151 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/python.rs:1157 | no | self is python.rs's own struct Lower, a different type
crates/sem-core/src/parser/calls/rust.rs:199 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/rust.rs:670 | yes | Lower::visit: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:701 | yes | Lower::visit: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:715 | yes | Lower::visit: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:726 | yes | Lower::visit: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:772 | yes | Lower::call_site: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:773 | no | self.f.expr(: self.f is &FileFacts: FileFacts::expr
crates/sem-core/src/parser/calls/rust.rs:791 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:804 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:816 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/rust.rs:824 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:871 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:878 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:903 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/rust.rs:918 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/calls/rust.rs:923 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:924 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:942 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:943 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:947 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:951 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:958 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/rust.rs:968 | no | field access .expr, not a call
crates/sem-core/src/parser/calls/rust.rs:1124 | no | the declaration fn expr, not a call
crates/sem-core/src/parser/calls/rust.rs:1145 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1168 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1172 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1178 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1182 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1189 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1208 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1216 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1222 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1226 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1239 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1251 | yes | Lower::expr_uncached: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1291 | yes | Lower::call_expr: self.expr( on self: Lower (struct Lower in calls/rust.rs)
crates/sem-core/src/parser/calls/rust.rs:1303 | yes | Lower::call_expr: this: &mut Self in a closure inside Lower::call_expr
crates/sem-core/src/parser/calls/select.rs:226 | no | comment, not code
crates/sem-core/src/parser/calls/select.rs:255 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/select.rs:256 | no | f is a &FileFacts (facts[fi], self.files[..], a FileFacts parameter): FileFacts::expr
crates/sem-core/src/parser/calls/tests.rs:35 | no | field access .expr, not a call
crates/sem-core/src/parser/differ.rs:8 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:152 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:181 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/graph.rs:197 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:450 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:452 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:458 | no | a variable, parameter or field named expr, not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:460 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/registry.rs:9 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:33 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:53 | no | expr as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/scope_resolve.rs:5421 | no | comment, not code
crates/sem-core/src/parser/session.rs:75 | no | expr as a variable, field, argument or other value, not a call
```

## c1_07 `util::fingerprint`

Defined in `crates/sem-cli/src/commands/check/util.rs`. The name `fingerprint` is declared 5 times (`fn fingerprint`, any kind).

```
crates/sem-cli/src/build_cache.rs:197 | no | comment, not code
crates/sem-cli/src/build_cache.rs:254 | no | comment, not code
crates/sem-cli/src/commands/check/generic.rs:147 | yes | generic::go: util::fingerprint(
crates/sem-cli/src/commands/check/lint.rs:213 | yes | lint::eslint: util::fingerprint(
crates/sem-cli/src/commands/check/mod.rs:187 | no | comment, not code
crates/sem-cli/src/commands/check/store.rs:5 | no | comment, not code
crates/sem-cli/src/commands/check/tests.rs:284 | yes | tests::run: util::fingerprint(
crates/sem-cli/src/commands/check/ts.rs:76 | yes | ts::tsc: util::fingerprint( (use super::util)
crates/sem-cli/src/commands/check/util.rs:124 | no | comment, not code
crates/sem-cli/src/commands/check/util.rs:125 | no | the declaration fn fingerprint, not a call
crates/sem-cli/src/commands/check/util.rs:149 | yes | util::helper: bare fingerprint( inside check/util.rs
crates/sem-cli/src/commands/query.rs:470 | no | comment, not code
crates/sem-cli/src/corpus_columns.rs:38 | no | comment, not code
crates/sem-cli/src/corpus_columns.rs:71 | no | comment, not code
crates/sem-core/examples/facts_corpus_probe.rs:29 | no | comment, not code
crates/sem-core/examples/facts_corpus_probe.rs:143 | no | the declaration fn fingerprint, not a call
crates/sem-core/examples/facts_corpus_probe.rs:174 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_corpus_probe.rs:231 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_corpus_probe.rs:238 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_corpus_probe.rs:440 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_corpus_probe.rs:589 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_corpus_probe.rs:593 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_probe.rs:118 | no | the declaration fn fingerprint, not a call
crates/sem-core/examples/facts_probe.rs:236 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_probe.rs:314 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/facts_probe.rs:332 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/incr_probe.rs:185 | no | the declaration fn fingerprint, not a call
crates/sem-core/examples/incr_probe.rs:419 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/incr_probe.rs:456 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/incr_probe.rs:481 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/incr_probe.rs:484 | no | the example's own free fn fingerprint(graph, entities)
crates/sem-core/examples/index_probe.rs:102 | no | comment, not code
crates/sem-core/src/index/complete.rs:130 | no | comment, not code
crates/sem-core/src/index/mod.rs:414 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/index/reader.rs:451 | no | comment, not code
crates/sem-core/src/index/reader.rs:501 | no | comment, not code
crates/sem-core/src/index/writer.rs:73 | no | comment, not code
crates/sem-core/src/index/writer.rs:117 | no | comment, not code
crates/sem-core/src/index/writer.rs:153 | no | comment, not code
crates/sem-core/src/index/writer.rs:298 | no | comment, not code
crates/sem-core/src/index/writer.rs:299 | no | comment, not code
crates/sem-core/src/index/writer.rs:396 | no | a variable, parameter or field named fingerprint, not a call
crates/sem-core/src/index/writer.rs:400 | no | fingerprint as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:401 | no | fingerprint as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:402 | no | fingerprint as a variable, field, argument or other value, not a call
crates/sem-core/src/index/writer.rs:435 | no | comment, not code
crates/sem-core/src/index/writer.rs:652 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:872 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/diff_oracle.rs:891 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/facts_store.rs:337 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2189 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2285 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/graph.rs:297 | no | comment, not code
crates/sem-core/src/parser/graph.rs:1464 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6229 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6236 | no | comment, not code
crates/sem-core/src/parser/graph.rs:6822 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:59 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:212 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:362 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:430 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:460 | no | comment, not code
crates/sem-core/src/parser/incremental.rs:478 | no | comment, not code
crates/sem-core/src/parser/resolve_profile.rs:426 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:1197 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:1201 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:2632 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:2633 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:2681 | no | comment, not code
crates/sem-core/src/parser/scope_resolve.rs:3537 | no | comment, not code
crates/sem-core/src/parser/session.rs:130 | no | comment, not code
crates/sem-core/src/parser/session.rs:387 | no | comment, not code
crates/sem-core/src/parser/session.rs:680 | no | the declaration fn fingerprint, not a call
crates/sem-core/src/parser/session.rs:706 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:803 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:812 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:938 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:1182 | no | comment, not code
crates/sem-core/src/parser/session.rs:1201 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:1210 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:1235 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/session.rs:1356 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:1365 | no | the test helper tests::fingerprint(graph, entities) in session.rs
crates/sem-core/src/parser/session.rs:2912 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:332 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:1139 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:1164 | no | comment, not code
crates/sem-core/tests/laws_extraction.rs:276 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/tests/single_pass_invariants.rs:128 | no | comment, not code
crates/sem-mcp/src/server.rs:274 | no | comment, not code
crates/sem-mcp/src/server.rs:560 | no | comment, not code
crates/sem-mcp/src/server.rs:1046 | no | a variable, parameter or field named fingerprint, not a call
crates/sem-mcp/src/server.rs:1051 | no | a variable, parameter or field named fingerprint, not a call
crates/sem-mcp/src/server.rs:1059 | no | field access .fingerprint, not a call
crates/sem-mcp/src/server.rs:3323 | no | a variable, parameter or field named fingerprint, not a call
crates/sem-mcp/src/server.rs:4941 | no | comment, not code
crates/sem-mcp/src/server.rs:4947 | no | a variable, parameter or field named fingerprint, not a call
crates/sem-mcp/src/server.rs:4953 | no | fingerprint as a variable, field, argument or other value, not a call
crates/sem-mcp/src/server.rs:4959 | no | fingerprint as a variable, field, argument or other value, not a call
```

## c1_08 `QueryIndex::file_count`

Defined in `crates/sem-core/src/index/reader.rs`. The name `file_count` is declared 5 times (`fn file_count`, any kind).

```
crates/sem-cli/src/commands/cloud.rs:538 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/commands/diff/mod.rs:1854 | no | field access .file_count, not a call
crates/sem-cli/src/commands/diff/mod.rs:1936 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/commands/diff/mod.rs:1956 | no | field access .file_count, not a call
crates/sem-cli/src/commands/diff/mod.rs:1974 | no | field access .file_count, not a call
crates/sem-cli/src/commands/diff/mod.rs:1994 | no | field access .file_count, not a call
crates/sem-cli/src/commands/entities.rs:159 | no | file_count as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/entities.rs:161 | no | file_count as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/entities.rs:162 | no | file_count as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/entities.rs:417 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/commands/entities.rs:450 | no | file_count as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/grep.rs:148 | yes | grep::search_one: idx from query::open_index(..): Option<QueryIndex>
crates/sem-cli/src/commands/impact.rs:124 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/commands/impact.rs:133 | no | file_count as a variable, field, argument or other value, not a call
crates/sem-cli/src/commands/repos.rs:114 | no | field access .file_count, not a call
crates/sem-cli/src/commands/repos.rs:337 | no | field access .file_count, not a call
crates/sem-cli/src/formatters/json.rs:74 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/formatters/json.rs:99 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/formatters/markdown.rs:7 | no | use declaration naming file_count, not a call
crates/sem-cli/src/formatters/markdown.rs:242 | no | bare file_count(result, binary_changes): the free fn formatters::file_count
crates/sem-cli/src/formatters/markdown.rs:276 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/formatters/mod.rs:25 | no | the declaration fn file_count, not a call
crates/sem-cli/src/formatters/mod.rs:26 | no | field access .file_count, not a call
crates/sem-cli/src/formatters/plain.rs:7 | no | use declaration naming file_count, not a call
crates/sem-cli/src/formatters/plain.rs:134 | no | bare file_count(result, binary_changes): the free fn formatters::file_count
crates/sem-cli/src/formatters/terminal.rs:8 | no | use declaration naming file_count, not a call
crates/sem-cli/src/formatters/terminal.rs:477 | no | bare file_count(result, binary_changes): the free fn formatters::file_count
crates/sem-cli/src/formatters/terminal.rs:600 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/formatters/terminal.rs:647 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/formatters/terminal.rs:673 | no | a variable, parameter or field named file_count, not a call
crates/sem-cli/src/stats.rs:113 | no | field access .file_count, not a call
crates/sem-cloud-client/src/lib.rs:135 | no | a variable, parameter or field named file_count, not a call
crates/sem-cloud-client/src/lib.rs:167 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/examples/facts_corpus_probe.rs:221 | no | merged from FactsCorpus::merge_with_local: PersistedFacts::file_count
crates/sem-core/examples/facts_corpus_probe.rs:580 | no | merged from FactsCorpus::merge_with_local: PersistedFacts::file_count
crates/sem-core/examples/facts_probe.rs:303 | no | loaded from FactsStore::load: PersistedFacts::file_count
crates/sem-core/examples/index_probe.rs:715 | yes | index_probe::trigram_oracle: index: &QueryIndex parameter of trigram_oracle
crates/sem-core/examples/index_probe.rs:880 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/examples/index_probe.rs:881 | no | field access .file_count, not a call
crates/sem-core/examples/index_probe.rs:983 | yes | index_probe::grep_mode: index from QueryIndex::open(..)
crates/sem-core/examples/index_probe.rs:1020 | yes | index_probe::lookup_mode: index from QueryIndex::open(..)
crates/sem-core/examples/index_probe.rs:1065 | yes | index_probe::refs_mode: index from QueryIndex::open(..)
crates/sem-core/src/format/json.rs:47 | no | field access .file_count, not a call
crates/sem-core/src/format/json.rs:237 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/format/json.rs:295 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/git/bridge.rs:2142 | no | field access .file_count, not a call
crates/sem-core/src/index/format.rs:131 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/index/format.rs:145 | no | field access .file_count, not a call
crates/sem-core/src/index/format.rs:191 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/index/format.rs:585 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/index/format.rs:607 | no | field access .file_count, not a call
crates/sem-core/src/index/grep.rs:130 | yes | grep::search: idx: &QueryIndex parameter of search
crates/sem-core/src/index/grep.rs:212 | yes | grep::candidate_files: idx: &QueryIndex parameter of candidate_files
crates/sem-core/src/index/mod.rs:476 | yes | tests::an_empty_graph_produces_a_valid_empty_index: index = round_trip(..) -> QueryIndex
crates/sem-core/src/index/mod.rs:495 | yes | tests::file_fingerprints_are_carried_for_files_with_no_entities: index = QueryIndex::from_bytes_with_salt(..)
crates/sem-core/src/index/reader.rs:112 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:236 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:264 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:407 | no | the declaration fn file_count, not a call
crates/sem-core/src/index/reader.rs:408 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:435 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:459 | no | field access .file_count, not a call
crates/sem-core/src/index/reader.rs:559 | no | comment, not code
crates/sem-core/src/index/reader.rs:565 | no | field access .file_count, not a call
crates/sem-core/src/index/writer.rs:477 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/index/writer.rs:670 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:198 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/parser/diff_oracle.rs:222 | no | field access .file_count, not a call
crates/sem-core/src/parser/diff_oracle.rs:279 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/parser/diff_oracle.rs:439 | no | field access .file_count, not a call
crates/sem-core/src/parser/differ.rs:34 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/parser/differ.rs:246 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/src/parser/differ.rs:916 | no | field access .file_count, not a call
crates/sem-core/src/parser/facts_store.rs:282 | no | the declaration fn file_count, not a call
crates/sem-core/src/parser/facts_store.rs:333 | no | the declaration fn file_count, not a call
crates/sem-core/src/parser/facts_store.rs:338 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:353 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:625 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2209 | no | loaded = store.load(..): PersistedFacts::file_count
crates/sem-core/src/parser/plugins/svelte.rs:2016 | no | field access .file_count, not a call
crates/sem-core/tests/laws_common/mod.rs:324 | no | field access .file_count, not a call
crates/sem-core/tests/laws_diff.rs:216 | no | field access .file_count, not a call
crates/sem-core/tests/single_pass_invariants.rs:175 | no | a variable, parameter or field named file_count, not a call
crates/sem-core/tests/single_pass_invariants.rs:178 | no | field access .file_count, not a call
crates/sem-mcp/src/render.rs:84 | no | the declaration fn file_count, not a call
crates/sem-mcp/src/render.rs:134 | no | bare file_count(list): the private free fn render::file_count
crates/sem-mcp/src/render.rs:145 | no | bare file_count(list): the private free fn render::file_count
crates/sem-mcp/src/render.rs:159 | no | bare file_count(list): the private free fn render::file_count
crates/sem-mcp/src/render.rs:184 | no | bare file_count(list): the private free fn render::file_count
```

## c1_09 `FastExtractorSet::identity`

Defined in `crates/sem-core/src/parser/fast_extractor.rs`. The name `identity` is declared 7 times (`fn identity`, any kind).

```
crates/sem-cli/src/commands/certify.rs:382 | no | comment, not code
crates/sem-cli/src/commands/certify.rs:500 | no | comment, not code
crates/sem-cli/src/commands/certify.rs:539 | no | comment, not code
crates/sem-cli/src/commands/cloud.rs:322 | no | comment, not code
crates/sem-cli/src/commands/query.rs:19 | no | comment, not code
crates/sem-cli/src/commands/query.rs:653 | no | comment, not code
crates/sem-cli/src/main.rs:942 | no | comment, not code
crates/sem-core/src/dataflow/mod.rs:267 | no | comment, not code
crates/sem-core/src/dataflow/mod.rs:360 | no | comment, not code
crates/sem-core/src/index/writer.rs:426 | no | comment, not code
crates/sem-core/src/model/entity.rs:20 | no | comment, not code
crates/sem-core/src/model/entity.rs:29 | no | comment, not code
crates/sem-core/src/model/entity_id.rs:6 | no | comment, not code
crates/sem-core/src/model/entity_id.rs:130 | no | comment, not code
crates/sem-core/src/model/mod.rs:4 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/cache.rs:133 | no | comment, not code
crates/sem-core/src/parser/cache.rs:148 | no | a variable, parameter or field named identity, not a call
crates/sem-core/src/parser/cache.rs:149 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/calls/rust.rs:1274 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:34 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:67 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:71 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:166 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:257 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:549 | no | comment, not code
crates/sem-core/src/parser/diff_oracle.rs:557 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/diff_oracle.rs:647 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/diff_oracle.rs:648 | no | mutation / self.mutation: Mutation: Mutation::identity
crates/sem-core/src/parser/diff_oracle.rs:777 | no | mutation / self.mutation: Mutation: Mutation::identity
crates/sem-core/src/parser/diff_oracle.rs:810 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/diff_oracle.rs:885 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/differ.rs:21 | no | use declaration naming identity, not a call
crates/sem-core/src/parser/differ/phase_timing.rs:80 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:81 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:202 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:708 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1026 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1034 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1045 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1056 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:1072 | no | a variable, parameter or field named identity, not a call
crates/sem-core/src/parser/facts_store.rs:1073 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:1080 | no | a variable, parameter or field named identity, not a call
crates/sem-core/src/parser/facts_store.rs:1081 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:1082 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/facts_store.rs:1121 | no | comment, not code
crates/sem-core/src/parser/facts_store.rs:2465 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:21 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:37 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:58 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:86 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:93 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:108 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:114 | no | a variable, parameter or field named identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:118 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:120 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:122 | no | a variable, parameter or field named identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:124 | no | e: &Box<dyn FastExtractor>: the trait method FastExtractor::identity
crates/sem-core/src/parser/fast_extractor.rs:129 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/src/parser/fast_extractor.rs:133 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:134 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:135 | no | field access .identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:222 | no | comment, not code
crates/sem-core/src/parser/fast_extractor.rs:237 | yes | fast_extractor::identity_salt: s: &Arc<FastExtractorSet> from installed(): auto-deref to FastExtractorSet
crates/sem-core/src/parser/fast_extractor.rs:311 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/fast_extractor.rs:325 | yes | tests::identity_of_a_set_joins_its_members: set = FastExtractorSet::new(..)
crates/sem-core/src/parser/fast_extractor.rs:333 | yes | tests::an_empty_set_has_an_empty_identity: set = FastExtractorSet::new(..)
crates/sem-core/src/parser/graph.rs:665 | no | comment, not code
crates/sem-core/src/parser/graph.rs:950 | no | comment, not code
crates/sem-core/src/parser/graph.rs:959 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:166 | no | comment, not code
crates/sem-core/src/parser/import_resolution.rs:1219 | no | comment, not code
crates/sem-core/src/parser/plugin.rs:25 | no | path segment identity:: (a module of that name), not a call
crates/sem-core/src/parser/plugins/code/entity_extractor.rs:1408 | no | comment, not code
crates/sem-core/src/parser/plugins/code/languages.rs:1149 | no | comment, not code
crates/sem-core/src/parser/plugins/code/mod.rs:1719 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:21 | no | comment, not code
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:37 | no | comment, not code
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:43 | no | comment, not code
crates/sem-core/src/parser/plugins/code/oxc_extractor.rs:80 | no | the declaration fn identity, not a call
crates/sem-core/src/parser/plugins/json.rs:456 | no | comment, not code
crates/sem-core/src/parser/plugins/json.rs:1076 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/src/parser/plugins/toml_plugin.rs:51 | no | comment, not code
crates/sem-core/src/parser/registry.rs:527 | no | comment, not code
crates/sem-core/src/parser/session.rs:1310 | no | comment, not code
crates/sem-core/src/persist/disk_cache.rs:273 | no | comment, not code
crates/sem-core/src/utils/hash.rs:197 | no | comment, not code
crates/sem-core/tests/entity_id_interning.rs:99 | no | comment, not code
crates/sem-core/tests/kappa.rs:1 | no | comment, not code
crates/sem-core/tests/kappa.rs:443 | no | comment, not code
crates/sem-core/tests/laws_common/mod.rs:142 | no | comment, not code
crates/sem-core/tests/laws_common/mod.rs:152 | no | a variable, parameter or field named identity, not a call
crates/sem-core/tests/laws_common/mod.rs:153 | no | identity as a variable, field, argument or other value, not a call
crates/sem-core/tests/laws_common/mod.rs:311 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:12 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:234 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:246 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/tests/laws_diff.rs:448 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:537 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:582 | no | comment, not code
crates/sem-core/tests/laws_diff.rs:622 | no | inside a string literal (fixture source or message text), not a call
crates/sem-core/tests/laws_extraction.rs:16 | no | comment, not code
crates/sem-core/tests/laws_extraction.rs:47 | no | comment, not code
crates/sem-core/tests/laws_extraction.rs:79 | no | comment, not code
crates/sem-core/tests/laws_extraction.rs:94 | no | comment, not code
crates/sem-core/tests/laws_extraction.rs:139 | no | inside a string literal (fixture source or message text), not a call
```

## c1_10 `CloudClient::auth_header`

Defined in `crates/sem-cli/src/commands/cloud.rs`. The name `auth_header` is declared 3 times (`fn auth_header`, any kind).

```
crates/sem-cli/src/commands/cloud.rs:810 | no | the declaration fn auth_header, not a call
crates/sem-cli/src/commands/cloud.rs:835 | yes | CloudClient::resolve_repo: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:871 | yes | CloudClient::register_repo: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:895 | yes | CloudClient::list_repos: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:910 | yes | CloudClient::forget_repo: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:976 | yes | CloudClient::impact: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:995 | yes | CloudClient::context: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1019 | yes | CloudClient::entities: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1043 | yes | CloudClient::history: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1069 | yes | CloudClient::upload_diff_snapshot: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1097 | yes | CloudClient::put_diff_relations: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1124 | yes | CloudClient::cross_deps: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1145 | yes | CloudClient::query_facts: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1161 | yes | CloudClient::download_facts: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cli/src/commands/cloud.rs:1178 | yes | CloudClient::put_facts: self.auth_header( inside impl CloudClient in sem-cli's cloud.rs
crates/sem-cloud-client/src/lib.rs:410 | no | the declaration fn auth_header, not a call
crates/sem-cloud-client/src/lib.rs:435 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:469 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:547 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:566 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:590 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:614 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:627 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-cloud-client/src/lib.rs:640 | no | self is sem-cloud-client's own struct CloudClient, a different type
crates/sem-mcp/src/agent_review.rs:327 | no | the declaration fn auth_header, not a call
crates/sem-mcp/src/agent_review.rs:343 | no | self is AgentReviewClient: AgentReviewClient::auth_header
crates/sem-mcp/src/agent_review.rs:368 | no | self is AgentReviewClient: AgentReviewClient::auth_header
crates/sem-mcp/src/agent_review.rs:387 | no | self is AgentReviewClient: AgentReviewClient::auth_header
crates/sem-mcp/src/agent_review.rs:402 | no | self is AgentReviewClient: AgentReviewClient::auth_header
crates/sem-mcp/src/agent_review.rs:422 | no | self is AgentReviewClient: AgentReviewClient::auth_header
```

## c1_11 `telemetry::now_secs`

Defined in `crates/sem-cli/src/telemetry.rs`. The name `now_secs` is declared 3 times (`fn now_secs`, any kind).

```
crates/sem-cli/src/commands/cloud.rs:126 | no | bare now_secs( in cloud.rs: its own private fn cloud::now_secs
crates/sem-cli/src/commands/cloud.rs:138 | no | bare now_secs( in cloud.rs: its own private fn cloud::now_secs
crates/sem-cli/src/commands/cloud.rs:141 | no | the declaration fn now_secs, not a call
crates/sem-cli/src/commands/update.rs:63 | no | the declaration fn now_secs, not a call
crates/sem-cli/src/commands/update.rs:83 | no | bare now_secs( in update.rs: its own private fn update::now_secs
crates/sem-cli/src/commands/update.rs:150 | no | bare now_secs( in update.rs: its own private fn update::now_secs
crates/sem-cli/src/telemetry.rs:134 | no | the declaration fn now_secs, not a call
crates/sem-cli/src/telemetry.rs:195 | yes | telemetry::current_install_id: bare now_secs( in telemetry.rs
crates/sem-cli/src/telemetry.rs:261 | yes | telemetry::record: bare now_secs( in telemetry.rs
crates/sem-cli/src/telemetry.rs:277 | yes | telemetry::record: bare now_secs( in telemetry.rs
crates/sem-cli/src/telemetry.rs:343 | yes | telemetry::flush: bare now_secs( in telemetry.rs
```

## c1_12 `SvelteLowerer::make_entity`

Defined in `crates/sem-core/src/parser/plugins/svelte.rs`. The name `make_entity` is declared 3 times (`fn make_entity`, any kind).

```
crates/sem-core/src/model/identity.rs:852 | no | the declaration fn make_entity, not a call
crates/sem-core/src/model/identity.rs:873 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:874 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:908 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:909 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:916 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:917 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:927 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:928 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:936 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:942 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:959 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:965 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:985 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:992 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1013 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1016 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1020 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1023 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1092 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1098 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1655 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/model/identity.rs:1656 | no | the test helper tests::make_entity(id, name, ..) in identity.rs
crates/sem-core/src/parser/graph.rs:12385 | no | the declaration fn make_entity, not a call
crates/sem-core/src/parser/graph.rs:12407 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12414 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12424 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12431 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12437 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12447 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12457 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12471 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12482 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12483 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/graph.rs:12484 | no | the test helper tests::make_entity(name, file_path, content) in graph.rs
crates/sem-core/src/parser/plugins/svelte.rs:302 | yes | SvelteLowerer::lower_script: self.make_entity( inside impl SvelteLowerer
crates/sem-core/src/parser/plugins/svelte.rs:335 | yes | SvelteLowerer::lower_style: self.make_entity( inside impl SvelteLowerer
crates/sem-core/src/parser/plugins/svelte.rs:448 | yes | SvelteLowerer::lower_else_if_chain: self.make_entity( inside impl SvelteLowerer
crates/sem-core/src/parser/plugins/svelte.rs:543 | yes | SvelteLowerer::push_node_entity: self.make_entity( inside impl SvelteLowerer
crates/sem-core/src/parser/plugins/svelte.rs:567 | no | the declaration fn make_entity, not a call
```
