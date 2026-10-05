# 1.2 ABAP comment and string stripping

## Intent

The reference scanner strips comments and strings before it looks for names, so
a name in a comment is not counted as a call. The generic stripper knows `//`,
`/* */` and `#` comments and double-quoted strings. ABAP uses none of those: a
comment is `*` in column 1 or `"` to end of line, and strings are `'...'`,
backtick strings and `|...|` templates. Without an ABAP stripper, `" call
helper( )` counts as a reference and `'` is mistaken for nothing at all. Add
`StripStrategy::Abap` and `strip_abap_content`.

## Acceptance criteria

- A name inside any of the following is never a reference:
  - a full-line comment starting with `*` in column 1;
  - a trailing comment after `"` to end of line;
  - a `'...'` literal, including the doubled-quote escape `'it''s'`;
  - a backtick string, including the doubled-backtick escape;
  - a `|...|` string template literal part.
- A name inside the embedded expression of a template, `|text { lv_name }|`, is
  still a reference. Only the literal parts are blanked.
- `*` not in column 1 is an operator, not a comment (`lv_a = 2 * lv_b.`).
- `"` inside a `'...'` literal does not start a comment.
- Line numbers are preserved: the stripper keeps newlines so reference lines
  stay aligned, as `strip_clojure_content` does.
- `sem find zcl_excel --callers` omits a method whose only mention is in a
  comment or string.
- For all other languages the stripper output is unchanged.
- Fixture tests `abap_fixture_1_2_comment_not_reference` and
  `abap_fixture_1_2_string_not_reference` pass and are no longer ignored.

## Files

- `crates/sem-core/src/parser/plugins/code/languages.rs`: add `Abap` to
  `enum StripStrategy` after `Clojure`, with a doc comment. Add
  `"abap" => StripStrategy::Abap` to `LanguageConfig::strip_strategy`.
- `crates/sem-core/src/parser/graph.rs`: add `fn strip_abap_content(content: &str)
  -> String` next to `strip_clojure_content` (near line 7375). Add the
  `StripStrategy::Abap` arm in `strip_for_language`. The match is exhaustive, so
  the compiler finds every site.
- `crates/sem-core/src/parser/plugins/code/mod.rs`: unit tests (see story 1.9).

## Approach

Copy `strip_clojure_content`. It allocates a `Vec<u8>` of spaces the length of
the input, copies code bytes through, and calls `blank_span_preserving_newlines`
for blanked spans. `strip_abap_content` is the same loop with ABAP rules.

State per byte, scanned in order:

1. At the start of a line (index 0 or after `\n`) and the byte is `*`: blank to
   end of line.
2. `"` outside a literal: blank to end of line.
3. `'`: blank to the closing `'`. A doubled `''` is an escape, not a close.
   Literals do not span lines. If no closing quote appears before a newline,
   blank to end of line and stop, so one stray quote cannot swallow the file.
4. Backtick: same as `'`, with a doubled backtick as the escape.
5. `|`: enter template mode. Blank literal bytes. On `{`, copy through until the
   matching `}`, tracking nesting, then resume literal mode. A backslash escapes
   `|`, `{` and `}`. A template may span lines in ABAP 7.5x, so only a closing
   `|` or end of input stops it, but cap the span at 4096 bytes as a guard.

Column 1 means byte offset 0 of the line, not the first non-blank. Test that an
indented `*` is not a comment.

Do the work in one pass. Unlike Clojure, no second pass is needed because the
line comment rules are in the same state machine. Dispatch in
`strip_for_language` calls `strip_abap_content` alone.

## Verification

```bash
cd crates
cargo test -p sem-core strip_abap
cargo test -p sem-core test_abap
cargo test -p sem-core abap_fixture_1_2 -- --include-ignored
cargo test --workspace
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_2_comment_not_reference`,
`abap_fixture_1_2_string_not_reference`.

Add a `CHANGELOG.md` entry under `## [Unreleased]` / `### Added` if upstreamed.

## Out of scope

- `*` comments after the first column in obsolete fixed-form code.
- Pseudo-comments (`"#EC NEEDED`). They are comments, so they are blanked, and
  nothing reads them.
- Chained statements with `:` and `,`.
- Blanking in the entity extractor. Only the reference scanner changes.

## Estimate

1-2 days.

## Depends on

Nothing. Rebase against story 1.1 if both are in flight.
