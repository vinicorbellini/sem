# ABAP parse-error census: baseline

The parse-error census on abapGit, taken with story 1.6's report on the entity
set of stories 1.1, 1.2, 1.3, 1.5 and 1.7. It is the baseline `census-gate1.md`
compares against once the other stories have landed.

| | |
|---|---|
| Date | 2026-10-05 |
| sem | `abap` branch at `ad282ca` plus story 1.6 (`story/1-6`) |
| Grammar | `tree-sitter-abap-sqry` 32.0.1 (mkoval1/tree-sitter-abap) |
| Corpus | abapGit `src/` at `b2b4e25` (shallow clone), `*.abap` files only |

## Command

```bash
cd abapGit
sem find --in src --parse-report --json --file-exts .abap > census.json
sem find --in src --parse-report --file-exts .abap          # the same, as text, ending in the totals
```

One row per file: `file`, `entity_count`, `grammar_entity_count`,
`fallback_entity_count`, `error_node_count`. `error_node_count` counts every
tree-sitter `ERROR` and `MISSING` node once. A file with errors and no
entities is also named on stderr. The run takes about a second.

## Totals

| Measure | Count |
|---|---:|
| Files parsed | 752 |
| Files with error nodes | 731 |
| Files with no error nodes | 21 |
| Error nodes | 39314 |
| Entities | 8389 |
| Entities from the grammar | 7999 |
| Entities from the fallback pass | 390 |
| Files with at least one fallback entity | 167 |
| Files with no entities | 5 |

Per file, the median is 8 entities and 31 error nodes; the largest is 158
entities and 1764 error nodes.

## Entities per file

| Entities | Files |
|---|---:|
| 0 | 5 |
| 1-5 | 296 |
| 6-20 | 333 |
| 21-100 | 117 |
| over 100 | 1 |

## Error nodes per file

| Error nodes | Files |
|---|---:|
| 0 | 21 |
| 1-10 | 171 |
| 11-50 | 301 |
| 51-100 | 158 |
| over 100 | 101 |

## Ten files with the most error nodes

| Error nodes | Entities | Grammar | Fallback | File |
|---:|---:|---:|---:|---|
| 1764 | 92 | 79 | 13 | `src/json/zcl_abapgit_ajson.clas.testclasses.abap` |
| 627 | 12 | 12 | 0 | `src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.testclasses.abap` |
| 510 | 18 | 15 | 3 | `src/ui/lib/zcl_abapgit_gui_chunk_lib.clas.abap` |
| 485 | 21 | 14 | 7 | `src/objects/tabl/zcl_abapgit_object_tabl_ddl.clas.abap` |
| 399 | 14 | 12 | 2 | `src/ui/pages/diff/zcl_abapgit_gui_page_diff_base.clas.abap` |
| 380 | 32 | 32 | 0 | `src/ui/pages/zcl_abapgit_gui_page_repo_view.clas.abap` |
| 325 | 23 | 23 | 0 | `src/ui/routing/zcl_abapgit_services_repo.clas.abap` |
| 323 | 17 | 17 | 0 | `src/ui/flow/zcl_abapgit_gui_page_flow.clas.abap` |
| 320 | 23 | 21 | 2 | `src/ui/pages/zcl_abapgit_gui_page_repo_over.clas.abap` |
| 309 | 25 | 25 | 0 | `src/ui/routing/zcl_abapgit_gui_router.clas.abap` |

## Files with no entities

| Error nodes | File |
|---:|---|
| 1 | `src/objects/aff_types/zif_abapgit_aff_prog_v1.intf.abap` |
| 2 | `src/objects/core/zabapgit_parallel.fugr.lzabapgit_paralleltop.abap` |
| 0 | `src/objects/core/zabapgit_parallel.fugr.saplzabapgit_parallel.abap` |
| 5 | `src/objects/zcl_abapgit_object_fdt0.clas.abap` |
| 3 | `src/persist/zcl_abapgit_persistence_db.clas.abap` |

Four of the five have error nodes, so they are the ones the grammar failed on
rather than files with nothing to extract. Check them by hand before reading
the entity totals as complete.
