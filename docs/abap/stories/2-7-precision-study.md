# 2.7 Precision check

## Intent

Tier 2 claims that resolved callers are right. Measure it. Take 30 methods of a
real repository, compare the callers sem resolves with a where-used over the
same repository, and report precision and recall. The where-used truth comes from
`sapcli whereused` when an agent with SAP access has produced it, and until then
from a careful, adjudicated grep. The story also fixes candidate T2-C, which
the study needs first: one entity per name in a chained `DATA:`.

## Acceptance criteria

- Pooled over the 30 methods, resolved callers have precision of at least 90%
  and recall of at least 80% against the where-used truth. Resolved means the
  sources of `calls` edges into the method, as in `sem find <m> --callers`
  `related`. Possible callers are reported separately and do not count.
- `crates/sem-core/tests/abap_precision.rs` runs the study. It is `#[ignore]`d
  because it needs a checkout, prints one row per method (target, truth size,
  resolved size, hits, precision, recall), and asserts the two thresholds when
  `SEM_ABAP_PRECISION_ASSERT=1`.
- The 30 targets are listed in `crates/sem-core/tests/fixtures/abap/precision/methods.json`
  with the stratum of each. The truth file in the fallback form is
  `crates/sem-core/tests/fixtures/abap/precision/abapgit-whereused.json`, in the
  schema of `inside-sap/README.md`. When the sapcli export exists it is
  `inside-sap/whereused.json` and the test prefers it.
- A short report, `docs/abap/precision-report.md`, lists every method under a
  threshold. It gives the cause of each miss: a wrong edge, a missing edge or a
  truth error. It counts the misses by cause.
- A chained `DATA: a TYPE i, b TYPE string.` gives one `variable` entity per
  name, each with its own line range, and `CLASS-DATA:` likewise. Adding a member
  to the chain adds one entity and changes no other.
- Fixture tests `abap_fixture_2_7_chained_data_one_entity_each`,
  `_chained_class_data`, `_chained_data_member_added`, and
  `_chained_types_already_split` pass.

## Files

- `crates/sem-core/src/parser/plugins/code/abap_fallback.rs`: `types_entities`
  already splits a chained `TYPES:` into one entity per declarator and skips the
  `:`. Do the same for `DATA` and `CLASS-DATA`, reusing story 2.3's chain
  splitter. Today `DATA:` is the grammar's `variable_declaration`, one entity named
  by `first_abap_name_token` in `entity_extractor.rs`.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: the
  `variable_declaration` branch (near lines 581 to 650) must not also emit the
  chain as one entity. `abap_period_before_end` already ends a declaration at
  its period.
- `crates/sem-core/tests/abap_precision.rs`: new, in the style of
  `graph_accuracy.rs` (a temp directory, `EntityGraph::build`) but over a checkout
  named by `SEM_ABAP_REPO`.
- `crates/sem-core/tests/fixtures/abap/precision/`: `methods.json`,
  `abapgit-whereused.json`. A sub-directory is skipped by
  `test_abap_fixture_layout`, as `inside-sap/` is.
- `crates/sem-core/tests/fixtures/abap/zcl_fx_chain.clas.abap` and `.clas.xml`: a
  class with `DATA:`, `CLASS-DATA:` and `TYPES:` chains. Add a row to the fixture
  `README.md`.
- `crates/sem-core/tests/fixtures/abap/inside-sap/whereused.json`: not written
  here. An agent with SAP access produces it.
- `docs/abap/precision-report.md`: the written result.

## Approach

The reference repo is abapGit at the commit `bench/abap-agent` pins,
`b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`, checked out at `/tmp/claude-0/abapGit`.
It is the repo the Gate 1 census and the agent benchmark already use. If the
plan means a different "reference repo", change this one line before starting.

Choose the 30 before looking at results, and say how. Reuse the rule in
`bench/abap-agent/tasks/b1_whereused.json` (declared once, 3 to 30 calling
methods by grep, at least 3 files) and include its 10 targets. Fill the other 20
across strata, so the number is not carried by easy cases: static calls
(`zcl_x=>m`), instance calls through a typed reference, interface methods,
redefined methods, methods called via `me->`, and forms. Write the stratum of each
target in `methods.json`.

Scoring unit. Match at the caller entity: the pair (object name, caller method or
form), case-folded. SAP's `include` field names a method include and its `line` is
a line in that include, not in the abapGit file, so lines are information and not
the key. Resolve the sapcli `include` to a method name once, in the converter that
writes `whereused.json`, and say how in the file's header.

Headline scoring counts direct `calls` edges into the target. Also report a
second column that adds callers reaching a declaration with a `dispatch` edge to
the target. SAP's where-used on a class method does not list interface callers, so
the first number is the fair one, and the second shows what story 2.3 adds.

The fallback ground truth is the risk. Use a different method from the one under
test: a case-insensitive grep for the name, then adjudicate every hit by reading
the receiver's declaration. Do not use sem's stripper or graph for it. Mark
rows an agent had to decide by hand with `"adjudicated": true`. Expect 300
or so hits. Record the grep numbers beside the ground truth so the
`bench/abap-agent` baseline can be compared.

Two ground-truth formats exist and they differ. `inside-sap/README.md` keys by
`CLASS=>METHOD` with `{object, type, include, line}` rows. The benchmark's
`bench/abap-agent/README.md` expects `targets: { b1_01: [{method, file, line}] }`.
Keep the first as the source, and write a small converter to the second so the
two uses read one file.

T2-C goes first because it changes entity ids, and the study's entity matching
should run on the final shape. Run the Gate 1 entity counts again after it.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_7
cargo test -p sem-core abap
cargo test --workspace
SEM_ABAP_REPO=/tmp/claude-0/abapGit SEM_ABAP_PRECISION_ASSERT=1 \
  cargo test -p sem-core --test abap_precision -- --ignored --nocapture
```

Run `git -C /tmp/claude-0/abapGit checkout b2b4e250747ecb4f8e2b1d79e332d2cf897d5fa0`
first and `git status` clean. Commit `docs/abap/precision-report.md` with the
numbers.

## Out of scope

- Improving the numbers. Misses become Tier 2 follow-ups or Tier 3 candidates.
- Recall of data-flow, SQL or `SUBMIT` references.
- Running `sapcli`. Needs a SAP system and an agent with access.
- A where-used over abap2xlsx. It is the acceptance repo of stories 2.0 and 2.6,
  not the study.

## Estimate

3 to 4 days, most of it the adjudicated ground truth. Tier 1's stories ran far
faster than their estimates when done as parallel agents, so treat this as a
ceiling.

## Depends on

Stories 2.1 to 2.5. Story 2.6 is not needed. T2-C itself depends on nothing and
can start earlier.
