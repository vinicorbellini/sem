# 2.6 Tests reached

## Intent

`sem impact <entity> --tests` should list the ABAP unit test methods that run the
entity, through resolved calls and not through a shared name. Tier 1 taught sem
which entities are tests (story 1.7). Tier 2's call edges give it the paths. Two
ABAP habits break the plain walk: a test class runs its `setup` without any call
to it, and the command's fallback quietly falls back to matching names when the
graph finds nothing. This story closes both.

## Acceptance criteria

- On the fixture repo, `sem impact zcl_fx_order.create --tests --no-default-excludes`
  lists `ltc_order.total_starts_at_zero` and `ltc_order.describe_mentions_id`,
  and `ltc_order.setup`. The two `FOR TESTING` methods reach `create` only
  through `setup`, which ABAP Unit runs before each of them.
- A test that calls a method of the same name in another class is not listed.
  The new `zcl_fx_other.describe` has a test, and it must not appear for
  `zcl_fx_order.describe`.
- A test two calls away is listed: a test calls `zcl_fx_user.run`, which calls
  `zcl_fx_order=>create`.
- A test reaching a base method through a subclass, and an implementation through
  its interface, are listed through the dispatch edges of story 2.3.
- When the graph finds no test, the command does not fall back to matching names
  for an ABAP target. It prints the "NO TEST REACHES" line and, when the caller
  set is incomplete, the possible callers. The same holds for `--json`
  (`noTestReaches: true`).
- On abap2xlsx the output is correct in both directions:
  - `sem impact zcl_excel_common.convert_column2alpha --tests` lists the test
    methods of `zcl_excel_common.clas.testclasses.abap` that call it.
  - `sem impact zcl_excel.add_new_worksheet --tests` agrees with a hand trace of
    the calls. If no test reaches it, it prints the "NO TEST REACHES" line (see
    Approach).
- The answer is the same on the cached-index path and the graph path.
- Fixture tests `abap_fixture_2_6_tests_through_setup`,
  `_same_name_other_class_not_listed`, `_two_hops`, `_through_dispatch`,
  `_no_lexical_fallback_for_abap` and `_index_and_graph_agree` pass.

## Files

- `crates/sem-core/src/parser/calls/abap.rs`: implicit fixture calls. For a class
  with a method declared `FOR TESTING`, emit a synthetic call site in each such
  method. It targets the class's `setup`, `class_setup`, `teardown` and
  `class_teardown`, whichever the class defines. The framework runs them and no
  source line says so. Mark them in the `Stats` dump so they can be told apart.
- `crates/sem-core/src/parser/graph.rs`: `filter_test_entities_with_custom_dirs`
  (near line 4661) already marks the members of a test class as tests, and
  `is_test_entity` (near line 5008) holds the `FOR TESTING` and
  `.testclasses.abap` rules from story 1.7. No change expected. The test suite
  pins them.
- `crates/sem-cli/src/commands/impact.rs`: `print_tests` (near line 1336) falls
  back to `word_hit`, a case-sensitive whole-word match of the entity name
  against test bodies, when the walk is empty. For an `.abap` target, skip it.
  `try_index_impact_transitive` (near line 745) returns `false` when the test
  list is empty so that fallback can run. For ABAP it should answer instead,
  since the lexical fallback is off.
- `crates/sem-cli/src/build_cache.rs` (near line 165): the index's test flags come
  from `filter_test_entities_with_custom_dirs`, so the inheritance of test status
  by members carries over. Check that the index writer gets the same set.
- `crates/sem-core/src/parser/context.rs`: its own `is_test_entity` (near line
  296) knows no ABAP. It serves `sem find --context`, not `impact --tests`. Note it
  and leave it for story 2.8, which runs `find` over MCP.
- `crates/sem-core/tests/fixtures/abap/`: new `zcl_fx_user.clas.testclasses.abap`
  (a test whose method calls `zcl_fx_user.run`) and
  `zcl_fx_other.clas.testclasses.abap` (a test class with a method calling
  `zcl_fx_other.describe`), from story 2.0's two classes. Add rows to the fixture
  `README.md`. The existing `zcl_fx_order.clas.testclasses.abap` covers `setup`.

## Approach

`impact --tests` walks the dependents of the target over every edge kind, then
keeps the test entities. Once stories 2.1 to 2.3 supply `calls` and `dispatch`
edges, that walk does the right thing with no new logic. Two things are left.

Implicit calls. `ltc_order.total_starts_at_zero` does not call `create`, and neither
does anything it calls. `setup` does, and the framework runs `setup` first. A
walk that follows source calls alone lists `setup` and misses the tests that
depend on it. Emit the implicit edge as a `Calls` edge from each `FOR TESTING`
method to the class's fixture methods. They are real runtime calls. The cost is
that `sem find setup --callers` lists the test methods, which is true.

The lexical fallback is the opposite of what the story promises. It answers "no
edge reaches a test" by listing every test whose body contains the entity's name,
which is the name matching this tier removes. It was written for dynamic languages
where `xr.where(...)` hides a call. ABAP has its own honest answer: the verdict's
possible callers (story 2.5). Print that, not names.

The plan's example is doubtful on abap2xlsx. Only `zcl_excel_reader_2007` and
`zcl_excel` itself mention `add_new_worksheet`, and none of the repo's ten
testclasses files names it. A test could still reach it through the reader's
`load_workbook`. Trace that by hand before the story starts. If none does, the
correct run says so, and the example is the negative case. Use
`zcl_excel_common=>convert_column2alpha`, which
`zcl_excel_common.clas.testclasses.abap` calls directly (line 172), as the
positive one, and pick a two-hop target by grep.

Local test classes. A class declared `FOR TESTING` in `locals_imp` is a test class
by story 1.7. When its `METHODS x FOR TESTING` declaration is in `locals_def` and the
implementation in `locals_imp`, the lowering cannot see the marker, so the
implicit `setup` edges are missing. Count how often on abapGit and record it.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_6
cargo test -p sem-core abap_fixture_1_7          # detection rules still hold
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
cd crates/sem-core/tests/fixtures/abap
SEM_NO_INDEX=1 /path/to/sem impact zcl_fx_order.create --tests --no-default-excludes
SEM_NO_INDEX=1 /path/to/sem impact zcl_fx_order.describe --tests --json --no-default-excludes
/path/to/sem impact zcl_fx_order.create --tests --no-default-excludes   # index path: same answer
cd /home/user/abap2xlsx
/path/to/sem impact zcl_excel_common.convert_column2alpha --tests
/path/to/sem impact zcl_excel.add_new_worksheet --tests                 # the NO TEST REACHES case
```

## Out of scope

- ABAP Unit risk level, duration and `FOR TESTING` attributes.
- Choosing which test class to run (an `aunit` runner).
- A test reaching code through `CALL TRANSACTION`, `SUBMIT` or a dynamic call.
- Teaching `context.rs` about ABAP (story 2.8).

## Estimate

2 to 3 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling.

## Depends on

Stories 2.2 and 2.3 for typed receivers and dispatch edges, and story 1.7 for
test detection.
