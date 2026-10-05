# 1.1 Case-insensitive names

## Intent

ABAP names are case-insensitive. `ZCL_EXCEL`, `zcl_excel` and `Zcl_Excel` are the
same class, and abapGit files use lowercase while code often uses uppercase. sem
compares names byte for byte today, so a reviewer who types the wrong case gets
no result and a caller written in another case is missed. Add a
`case_insensitive()` flag to `LanguageConfig` and fold names only when the flag
is set, so no other language pays for it.

## Acceptance criteria

- `sem find ZCL_EXCEL`, `sem find zcl_excel` and `sem find Zcl_Excel` return the
  same entity list, in the same order, on the fixture repo.
- `sem find zcl_excel --callers` lists a caller whose source writes
  `ZCL_EXCEL=>create( )` and one that writes `zcl_excel=>create( )`.
- `sem find zcl_excel --json` reports the entity name as written in source, not
  folded. Display is never rewritten.
- `sem graph --json` has an edge for each of: lowercase definition with
  uppercase use, uppercase definition with lowercase use.
- Types and kinds still filter: `sem find "method Run"` matches `RUN` and `run`.
- For every non-ABAP language, `sem graph --json` output on the repo itself is
  byte-identical before and after the change.
- Benchmark: `parse_profile` and `incremental` show no regression above noise
  (see Verification) on a non-ABAP corpus.
- Fixture tests `abap_fixture_1_1_find_any_case` and
  `abap_fixture_1_1_refs_any_case` pass and are no longer ignored.

## Files

- `crates/sem-core/src/parser/plugins/code/languages.rs`: add
  `pub(crate) fn case_insensitive(&self) -> bool` on `LanguageConfig`, next to
  `extra_ident_chars` and `strip_strategy`. Return true for `"abap"`.
- `crates/sem-core/src/parser/graph.rs`:
  - `identifier_tokens`, `token_iter`, `content_contains_identifier`,
    `text_mentions_any_name`: add a folded comparison path.
  - `extract_references_from_content`, `extract_references_with_stripped` and
    `maybe_push_reference_token`: fold the token before the symbol-table probe.
  - The `SymbolTable` build sites (`symbol_table: SymbolTable = HashMap::default()`
    in several build paths): insert folded keys for case-insensitive files.
  - `extra_ident_chars_for_file` and `strip_strategy_for_file` show the pattern
    for a per-file config query; add `case_insensitive_for_file` beside them.
- `crates/sem-core/src/parser/scope_resolve.rs`: no change for ABAP, because
  `ABAP_CONFIG.scope_resolve` is `None`. Add a comment at `defs` and
  `binding_rows` saying they are case-sensitive by design.
- `crates/sem-cli/src/commands/mod.rs`: `entity_matches_query`,
  `entity_matches_qualified`, `print_name_suggestions`.
- `crates/sem-cli/src/commands/qualified.rs`: `matches` compares `q.bare()` to
  `name` exactly; make it fold when the entity's language is case-insensitive.
- `crates/sem-core/src/index/reader.rs`: `lookup` binary-searches the sorted
  `NAMES` table by raw bytes. See Approach for the index decision.
- `crates/sem-core/src/index/writer.rs` and `index/format.rs`: only if the index
  stores folded names (bump `FORMAT_VERSION`).

## Approach

Copy the `extra_ident_chars` pattern. It is a per-language method on
`LanguageConfig`, read once per file through a `*_for_file` helper, and passed
down as a small `Copy` value. Do the same: resolve `case_insensitive` once per
file and carry a `bool` beside `extra_ident_chars`.

Fold with `to_ascii_lowercase`. ABAP identifiers are ASCII in practice, and
ASCII folding avoids Unicode allocation surprises. Fold only when the flag is
true: write the comparison as `if fold { eq_ignore_ascii_case } else { == }` so
the non-ABAP branch compiles to the existing code.

Symbol table: key by the folded name for ABAP files only. Lookups from an ABAP
file fold the token. A lookup from a non-ABAP file never reaches an ABAP key in
practice, so no cross-language rule is needed. Keep `EntityInfo.name` as
written.

Index decision, made at the start of the story: either (a) store the folded name
in `NAMES` and fold the query in `lookup` when any ABAP file is in the index, or
(b) keep `NAMES` exact and make `find` retry with a folded scan for ABAP. Pick
(a) if a `FORMAT_VERSION` bump is acceptable, because it keeps the binary
search. Note the choice in the commit message.

Token shape: ABAP identifiers include `-` in structure components
(`ls_row-field`) and `~` in interface methods (`lif~run`). Do not add `-` to
`extra_ident_chars` here, because it would merge `a-b` subtraction. Split on
`~` and `=>` and `->` so `lif~run` yields `lif` and `run`. Record the choice in
the story's commit message.

Before and after benchmark: run the two Criterion benches on a non-ABAP corpus
and compare. Record both numbers in the commit message.

## Verification

```bash
cd crates
cargo test -p sem-core abap
cargo test -p sem-core abap_fixture_1_1 -- --include-ignored
cargo test --workspace
cargo bench -p sem-core --bench parse_profile -- --save-baseline before   # on the base commit
cargo bench -p sem-core --bench incremental -- --save-baseline before
cargo bench -p sem-core --bench parse_profile -- --baseline before        # after the change
cargo bench -p sem-core --bench incremental -- --baseline before
cargo run -p sem-cli -- graph --json > /tmp/graph-after.json              # on this repo; diff with the same run on the base commit
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_1_find_any_case`, `abap_fixture_1_1_refs_any_case`.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Added` if the change is
offered upstream.

## Out of scope

- Unicode case folding.
- Case-insensitive grep. `sem grep` already has `--ignore-case`.
- Case-insensitive file paths. Paths stay exact.
- Making any other language case-insensitive. The flag exists for future use only.
- Resolving `lif~run` to the interface method. That is Tier 2.

## Estimate

3-4 days.

## Depends on

Nothing. Rebase against story 1.2 if both are in flight, as both edit `graph.rs`
and `languages.rs`.
