# 2.3 Inheritance and interfaces

## Intent

A call on a base class or an interface runs one of several methods at run time.
`lo_base->describe( )` may reach `zcl_fx_order.describe` or the redefinition in
`zcl_fx_order_sub`. `lif->get_total( )` may reach any class that implements
`zif_fx_order`. A reviewer who changes a redefinition needs the callers of the
base, and one who changes an interface method needs every implementation.
Produce the edges for that. The story also makes interface method declarations
entities (candidate T2-B), so a changed signature is the method changing and not
the interface.

## Design decision: needs a human look before code

How interface and inheritance dispatch become edges. This story recommends the
trait-and-impl model that Rust and Python already use in `calls/`, with one
`Dispatch` edge per declaration and implementation. The options are in Approach.
Confirm the recommendation, or choose another, before code is written. It decides
what `sem find --callers` and `sem impact` print for every ABAP class hierarchy.

## Acceptance criteria

- Interface method declarations are entities. `METHODS add_item` and
  `METHODS get_total` in `zif_fx_order` are `method` entities with the interface
  as parent. A chained `METHODS: a ..., b ...` gives one entity per name.
  `test_abap_fixture_intf` is updated to the new list.
- `lif->get_total( )`, with `lif TYPE REF TO zif_fx_order`, has a `calls` edge to
  `zif_fx_order.get_total` and a `dispatch` edge from that method to each
  implementation: `zcl_fx_order.zif_fx_order~get_total` and the one in
  `zcl_fx_order_alt`.
- `lo_base->describe( )`, with `lo_base TYPE REF TO zcl_fx_order`, has a `calls`
  edge to `zcl_fx_order.describe` and a `dispatch` edge from it to
  `zcl_fx_order_sub.describe`. `METHODS describe REDEFINITION` is the marker.
- `ALIASES total FOR zif_fx_order~get_total` makes `lo_alt->total( )` resolve to
  the implementation of `get_total` in that class.
- `INTERFACES zif_fx_order` inside another interface is a supertrait: a class
  implementing the outer one has the inner one's methods.
- `sem impact zcl_fx_order_sub.describe` lists the caller that used the base
  type, through the dispatch edge.
- `sem find zcl_fx_order.describe --callers` and the Python equivalent behave
  alike for a base method with overrides. Check Python's output and match it.
- Non-ABAP `sem graph --json` stays byte-identical.
- Fixture tests `abap_fixture_2_3_interface_method_entities`,
  `_chained_methods`, `_interface_call_dispatches`, `_base_call_dispatches`,
  `_redefinition_pairs`, `_alias_resolves`, `_interface_includes_interface` and
  `_impact_through_dispatch` pass.

## Files

- `crates/sem-core/src/parser/plugins/code/abap_fallback.rs`: entities for
  `METHODS` statements inside an `INTERFACE` block, found by `statements`, with
  the interface as parent. It is the one place in the extractor that already
  reads `METHODS` and `INTERFACE` off the statement stream. Write the chain
  splitter (`METHODS: a ..., b ...`) as a helper. Story 2.7 reuses it for `DATA:`.
- `crates/sem-core/src/parser/plugins/code/entity_extractor.rs`: only if the
  grammar's `method_declaration` node must be excluded to avoid a duplicate.
  Verify with the 1.10 census that no interface gains two entities per name.
- `crates/sem-core/src/parser/calls/abap.rs`:
  - `INTERFACE` becomes a `TraitDecl`, its `METHODS` become `FnDecl` with
    `Owner::Trait`, and `INTERFACES zif_b` inside it becomes `supertraits`.
  - `INTERFACES zif_x` in a class becomes one `ImplDecl { trait_: Some(zif_x) }`
    per interface. The class's `zif_x~m` methods are its members, named in full.
  - `INHERITING FROM` stays in `TypeDecl.embeds` (story 2.1).
  - `virtual_methods()` returns `true`, so `override_pairs` in `mod.rs` emits a
    `Dispatch` pair from each base method to each override.
- `crates/sem-core/src/parser/calls/lang.rs`: a defaulted
  `fn member_key<'a>(&self, name: &'a str) -> &'a str { name }`. ABAP returns the
  part after the last `~`.
- `crates/sem-core/src/parser/calls/mod.rs`: `dispatch_pairs` compares a trait
  declaration's name to an implementation's with `==`. Compare
  `lang.member_key(..)` instead. Without it `get_total` never matches
  `zif_fx_order~get_total`.
- `crates/sem-core/src/parser/calls/ir.rs`: `TypeDecl` gains
  `aliases: Vec<(Name, Name)>` for `ALIASES a FOR zif_x~m`. Every constructor in
  `rust.rs`, `go.rs` and `python.rs` gets an empty `Vec`.
- `crates/sem-core/src/parser/calls/select.rs`: `Resolver::method`, in the
  `Ty::Adt` arm, consults `TypeDecl.aliases` before `inherited`.
- `crates/sem-core/tests/fixtures/abap/`: new `zif_fx_audit.intf.abap` (it holds
  `INTERFACES zif_fx_order` and a chained `METHODS: audit, log`),
  `zcl_fx_order_alt.clas.abap` (implements `zif_fx_order` with an `ALIASES`),
  `zcl_fx_dispatch.clas.abap` (the callers), each with its `.xml` envelope. Add
  rows to the fixture `README.md`. `zcl_fx_order_sub` already has the
  `REDEFINITION` and `super->`.

## Approach

The options for dispatch:

1. **Trait and impl model (recommended).** An interface is a trait, an
   implementing class has an impl of it, and a base class is an embedded type
   with virtual methods. A call through an interface or base type gets a `Calls`
   edge to the declaration and the pipeline adds one `Dispatch` edge from
   declaration to each implementation. This is what `dispatch_pairs` and
   `override_pairs` already do for Rust traits and Python overrides. Impact
   flows through the dispatch edge, and `find --callers` on an implementation
   shows the dispatch source, not a call it does not make.
2. **Flat edges.** A call on a base or interface type gets a `Calls` edge to every
   implementation directly. It is a smaller change. It reads as exact when it is
   not, loses the declaration as a node, and a new subclass changes the edges of
   every caller.
3. **Name matching across the hierarchy.** Any method of the same name in a
   subclass or implementer. No types needed, and it is the guess the pipeline
   rejects. Reject.

One imprecision to accept and test. ABAP allows a subclass to define a method
with the same name as a private method of its superclass, which is not an
override. `override_pairs` pairs by name and would link them. The `REDEFINITION`
marker can tell them apart. Use it if lowering can carry it cheaply, for example
by emitting the override `FnDecl` only for methods with `REDEFINITION`. Otherwise
record the case in a test and move on.

Interface method entities do more than name the signature. The fallback in
`EntityIds::build` already lets a call into an interface land on the interface's
entity when the method has none. With T2-B the call lands on the method, and
`sem diff` names `get_total` as changed and not `zif_fx_order`. Compare the ten
hand-checked PRs of `census-gate1.md`: #4432 `zif_abapgit_popups` is the case.

Naming. A class's method is `zif_x~m` in the entity list, and the interface's
declaration is `m`. On a class-typed receiver ABAP requires the full form (or an
alias). On an interface-typed receiver the bare name is used. `member_key` reads
both as `m` for dispatch and nothing else changes.

## Verification

```bash
cd crates
cargo test -p sem-core abap_fixture_2_3
cargo test -p sem-core abap_fixture_1_10         # interface method names still hold
cargo test -p sem-core abap
cargo test --workspace
cargo build --release -p sem-cli
cd crates/sem-core/tests/fixtures/abap
SEM_NO_INDEX=1 /path/to/sem impact zcl_fx_order_sub.describe --no-default-excludes
SEM_NO_INDEX=1 /path/to/sem find zif_fx_order.get_total --callers --no-default-excludes
```

Re-run the Gate 1 entity census on abapGit and confirm interface entity counts
rise and no existing entity count falls.

## Out of scope

- Events, `ENHANCEMENT` hooks, BAdI calls through `CL_EXITHANDLER`: these are runtime lookups.
- Generic interface methods and `FINAL` or `ABSTRACT` checks.
- `TYPES` and `CONSTANTS` in an interface as entities.
- Instance-creation-time binding (`CREATE OBJECT lo TYPE (name)`), which is story 2.5.

## Estimate

4 to 5 days. Tier 1's stories ran far faster than their estimates when done as
parallel agents, so treat this as a ceiling.

## Depends on

Story 2.1 for the skeleton. Story 2.2 runs beside it. The entity half (interface
method entities, the `TraitDecl` and `ImplDecl` lowering, `member_key`, aliases) needs
no receiver types and can go first. The call-edge tests need 2.2's typed receivers,
since a dispatch edge needs a typed receiver, so they land after it.
