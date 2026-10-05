# 2.8 MCP

## Intent

Agents reach sem through `sem mcp`. Its three tools that matter for review are
`sem_find`, `sem_impact` and `sem_certify`, and the ABAP benchmark in
`bench/abap-agent` gives them to the sem arm. Stories 2.0 to 2.7 make the CLI right
for ABAP. This story checks that MCP gives the same answers. It uses the tools'
existing parameters and no ABAP-specific flag. It fixes the places where MCP
answers differently from the CLI.

## Acceptance criteria

- Over the real stdio protocol, on a temporary git repo holding ABAP fixture
  files in an abapGit `src/` layout, with none of `no_default_excludes`, no case
  normalisation and no ABAP-only argument:
  - `sem_find` with `query: "ZCL_FX_ORDER"` and with `query: "zcl_fx_order"`
    returns the same entity. `mode: "callers"` on `create` lists the caller from
    another file, as the CLI does.
  - `sem_find` `mode: "callers"` carries the completeness verdict: `complete`,
    `incomplete_because` and `possible_callers`, as the CLI's `find --callers
    --json` does. A method with only a dynamic caller does not print
    "(callers: none)".
  - `sem_impact` with `file_path` of a class file, `entity_name: "zcl_fx_order.create"`
    (the `Class.method` form), and `mode: "tests"` lists the test methods the
    CLI lists.
  - `sem_certify` over a two-commit range that edits `zcl_fx_order.describe`
    names the method and its callers, in text and in JSON.
- The MCP and CLI answers agree for each case above, compared in the test with
  the same `cli_json` helper the existing tests use.
- Names match case-insensitively in every `server.rs` lookup that the CLI folds,
  for ABAP files only. A Python entity named `Order` is still found only as `Order`.
- Non-ABAP MCP output is unchanged: the existing `mcp_protocol.rs` and
  `server.rs` tests pass untouched.
- Test `mcp_abap_serves_find_impact_certify` passes, and the three tools' schema
  listings are unchanged except for text.

## Files

What the code shows, which the plan's row did not say:

- `crates/sem-mcp/src/server.rs` compares names exactly in each lookup:
  `find_entity_in_graph` (near line 674), `find_entity_repo_wide` (near 726),
  `sem_find`'s `match_one` (near 2651), `sem_callers` (near 2773), the
  `sem_impact` existence check (near 2149) and `mcp_entity_by_name_at_ref` (near
  4817). Story 1.1 folded names in the CLI (`entity_matches_query`,
  `qualified::matches`), so over MCP `ZCL_FX_ORDER` finds nothing. Replace them
  with one predicate.
- `crates/sem-core/src/parser/graph.rs` or a new small module: that predicate, as
  `pub fn name_matches(file_path: &str, entity_name: &str, query: &str) -> bool`.
  It folds when `get_language_config(ext).case_insensitive()` is true. Both
  `sem-cli` and `sem-mcp` call it, so the two cannot drift again. `sem-mcp` does
  not depend on `sem-cli`, so the CLI's own helper is out of reach.
- `crates/sem-mcp/src/server.rs`, `sem_callers` (near line 2749): it lists every
  dependent of the entity, over every edge kind, and carries no verdict. The CLI's
  `find --callers` lists calls and attaches the verdict from
  `caller_verdict` in `sem-cli/src/commands/query.rs`. The tool description
  promises "exact, or marked incomplete". Make the MCP answer match, see Approach.
- `crates/sem-mcp/src/tools.rs`: `FindParams` has no `no_default_excludes`,
  though `EntitiesParams` and `ImpactAnalysisParams` do. Real ABAP repos do not
  trip the default excludes (abapGit and abap2xlsx do not), so leave it, and say so
  in a code comment. The test must not copy fixtures into a directory called
  `fixtures`, which is excluded by name.
- `crates/sem-mcp/tests/mcp_protocol.rs`: new `mcp_abap_serves_find_impact_certify`,
  built on `McpClient`, `fixture_repo` and `callers_fixture`, which already make a
  temporary git repo, and on `sem_cli_bin` and `cli_json` for the comparison.
  `sem_certify` shells out to the `sem` binary (`run_sem`), so the test needs it
  built.
- `crates/sem-core/src/parser/context.rs`: its own `is_test_entity` knows no ABAP,
  so `sem_find` `mode: "context"` counts a `.testclasses.abap` method as source.
  Reuse the graph's `is_test_entity` rules, or delegate to them. Story 2.6 left it
  here.
- `docs/shared-mcp.md` is the server's description for agents. Add one sentence
  that names in ABAP files match in any case, if the text lists such behaviour.

## Approach

Two real defects, and one non-defect to confirm.

The case folding is mechanical. One predicate in sem-core, called from the six
sites. Fold only for case-insensitive languages, so other languages pay nothing
and see no change.

The callers verdict is the larger change. `sem-mcp` cannot call
`caller_verdict`, which lives in `sem-cli` and reads the index and the working
tree. The server already shells out to the `sem` binary for `sem_certify` and
`sem_graph` through `run_sem`. Do the same for the verdict: run
`sem find <query> --callers --json` once and merge `complete`,
`incomplete_because`, `possible_callers` and `possible_caller_sites` into the
MCP row, keeping the existing `entity` and `callers` keys. The fields are added, so
no client breaks. Do it for every language. A verdict that exists for Python and
not for ABAP over MCP is the inconsistency to avoid. If the maintainers want it
ABAP-only for now, gate it on the target's file extension and record that in a
comment.

The non-defect is `sem_impact` tests. `sem_impact` calls
`graph.test_impact_with_custom_dirs` directly and has no lexical fallback, which is
what story 2.6 wants for ABAP. Confirm it with the test, do not change it.

Benchmark. The `bench/abap-agent` sem arm is this surface. Gate 2's rerun of
the B1 class measures it, so keep the tool names, parameters and schemas as they
are.

## Verification

```bash
cd crates
cargo build --release -p sem-cli -p sem-mcp
cargo test -p sem-mcp mcp_abap_serves_find_impact_certify
cargo test -p sem-mcp
cargo test -p sem-core abap
cargo test --workspace
# by hand, on the reference repo
cd /home/user/abap2xlsx
sem mcp   # then call sem_find {"query":"ZCL_EXCEL","mode":"callers"} and compare with: sem find ZCL_EXCEL --callers --json
```

Run the agent benchmark's dry run to see the tools answer end to end:
`python3 bench/abap-agent/run.py --dry-run --arm sem --class B1 --reps 1`.

## Out of scope

- New tools or parameters.
- ABAP-specific tool descriptions beyond one sentence.
- Cloud mode (`SEM_MCP_CLOUD`). It indexes the default view and is untouched.
- Making `sem_callers` follow the CLI's grouping of dispatch registrations.

## Estimate

1 day. Tier 1's stories ran far faster than their estimates when done as parallel
agents, so treat this as a ceiling.

## Depends on

Stories 2.5 (the ABAP verdict) and 2.6 (test reach), and story 1.1 (the folding
rule it moves into sem-core).
