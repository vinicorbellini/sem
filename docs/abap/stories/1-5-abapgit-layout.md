# 1.5 abapGit layout

## Intent

abapGit serialises each ABAP object to files named `<name>.<type>[.<part>].<ext>`:
`zcl_foo.clas.abap`, `zfoo.prog.abap`, `zfg.fugr.lzfg_f01.abap`. The file name
carries the object name and type, and sem should read them from there rather
than guess from content. The same repository holds `*.xml` metadata files,
`package.devc.xml` and `.abapgit.xml` that produce no useful entities and add
noise. Parse the name, and drop the metadata from default scans.

## Acceptance criteria

- A function `parse_abapgit_name(path) -> Option<AbapObject>` returns the object
  name and type:
  - `src/zcl_foo.clas.abap` gives name `zcl_foo`, type `clas`;
  - `src/zfoo.prog.abap` gives `zfoo`, `prog`;
  - `src/zfg.fugr.lzfg_f01.abap` gives `zfg`, `fugr`, part `lzfg_f01`;
  - `src/zcl_foo.clas.testclasses.abap` gives `zcl_foo`, `clas`, part `testclasses`;
  - `src/zif_x.intf.abap` gives `zif_x`, `intf`;
  - namespaced names, `#ns#zcl_foo.clas.abap`, give `/ns/zcl_foo`: abapGit
    writes `/` as `#`.
  - a plain `foo.abap` returns `None`.
- `sem find --in src` over an abapGit repo skips `*.xml`, `package.devc.xml` and
  `.abapgit.xml` by default.
- `sem find --in src --no-default-excludes` includes them.
- A non-abapGit repository with an `*.xml` file, such as a Maven `pom.xml`, is
  unaffected: its XML files are still scanned.
- `sem find ZFOO` resolves a report entity from `zfoo.prog.abap` even when the
  source line says `REPORT zfoo_alt.`, and the fixture flags the mismatch in
  the entity metadata rather than silently trusting the source.
- Fixture tests `abap_fixture_1_5_name_from_filename`,
  `abap_fixture_1_5_xml_excluded` and `abap_fixture_1_5_xml_kept_with_flag`
  pass and are no longer ignored.

## Files

- `crates/sem-core/src/utils/scan.rs`: `is_default_excluded`. This is the real
  home of default excludes. The first-draft note named `system/scan.rs`, which
  is a different, unrelated file.
- `crates/sem-core/src/parser/plugins/code/abap_name.rs` (new): the
  `parse_abapgit_name` function and the `AbapObject` struct.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: call the parser
  when `config.id == "abap"` to set the object name for the top-level entity.
- `crates/sem-core/src/parser/registry.rs`: no change expected. `get_extensions`
  already yields every dot-suffix of the file name, so `.abap` already matches
  `zcl_foo.clas.abap`. Confirm by test; if a suffix like `.clas.abap` is wanted
  as an explicit key, add it to `ABAP_CONFIG.extensions` in `languages.rs`.
- `crates/sem-cli/src/commands/files.rs`: both call sites of
  `is_default_excluded` (lines near 61 and 89) already honour
  `--no-default-excludes`. No change unless the exclude needs repo context
  (see Approach).

## Approach

Default excludes today are context-free: `is_default_excluded(rel_path)` sees
only the path. Excluding all `*.xml` would silently drop `pom.xml` and every
other XML in non-ABAP repos, which breaks the "other languages pay nothing"
rule. Use a path-only rule that matches abapGit's shape:

- the file name is `.abapgit.xml` or `package.devc.xml`; or
- the file name matches `<name>.<type>.xml` where `<type>` is a four-letter
  abapGit object type (`clas`, `intf`, `prog`, `fugr`, `devc`, `tabl`, `dtel`,
  `doma`, `msag`, `tran`, `enho` and so on).

A `pom.xml` has no middle segment, so it stays. Put the list of type codes in
one `const ABAPGIT_TYPES: &[&str]` next to `DEFAULT_EXCLUDED_SUFFIXES`, copying
that constant's style and keeping the match case-insensitive as the existing
function does.

If a future need arises for repo-aware excludes (for instance, only exclude XML
when `.abapgit.xml` exists at the root), that is a signature change to
`is_default_excluded` and out of scope here.

Name parsing: split the file name on `.`. Segment 0 is the object name, segment
1 is the type, any middle segment is the part, the last is the extension. Replace
a leading `#...#` pair with `/.../`. Lower-case the type. Do not lower-case the
name here; story 1.1 folds at comparison time.

Object name from file name versus source: abapGit's file name is authoritative
for the object. For `fugr`, include files (`lzfg_f01`) belong to the function
group `zfg`, so entities in them take `zfg` as the group name.

Pattern to copy: the lockfile entries in `DEFAULT_EXCLUDED_FILES` for exact file
names, and `DEFAULT_EXCLUDED_SUFFIXES` for suffix rules, both in the same file.

## Verification

```bash
cd crates
cargo test -p sem-core default_excludes
cargo test -p sem-core abap_name
cargo test -p sem-core abap_fixture_1_5 -- --include-ignored
cargo test --workspace
cargo run -p sem-cli -- find --in src --json | head          # from the abapGit clone: no XML
cargo run -p sem-cli -- find --in src --json --no-default-excludes | head   # XML present
```

Fixture tests that flip from ignored to passing:
`abap_fixture_1_5_name_from_filename`, `abap_fixture_1_5_xml_excluded`,
`abap_fixture_1_5_xml_kept_with_flag`.

Extend `default_excludes_generated_support_and_build_paths` in `scan.rs` with
ABAP and non-ABAP XML cases. Add a `CHANGELOG.md` entry under
`## [Unreleased]` / `### Added` if upstreamed.

## Out of scope

- Parsing the XML metadata for descriptions or package structure.
- Objects serialised without abapGit (SAPlink, ABAP-in-Eclipse `.abap` with
  other naming).
- Resolving include files to their main program by content.
- Repo-aware excludes.

## Estimate

1-2 days.

## Depends on

Nothing. Story 1.3 consumes `parse_abapgit_name` for local class attachment.
