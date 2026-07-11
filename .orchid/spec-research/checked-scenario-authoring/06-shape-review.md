# Shape review: checked scenario authoring

Reviewer profile: Sol/high-equivalent strong architecture review
Review date: 2026-07-11
Reviewed artifact: `05-implementation-shape.md`

## Verdict

The implementation shape is complete enough for narrowly scoped workers. It defines the durable
wire/checked boundary, names the complete family and connection sums, fixes local and graph
validation order, specifies duplicate semantics, identifies exactly which values become nonzero,
retains every validation parse AST across the checked-scenario boundary, and gives the post-037
plan/engine migration rather than stopping at an additive API shell.

## Rubric review

### Files and ownership

Green. All new and modified source, test, fixture, and documentation paths are concrete. The shape
uses `src/types/scenario_checked.rs` for checked public values, preserves DTO ownership in
`src/types/scenario.rs`, and follows spec 037's frozen `src/plan.rs` plus
`src/validation/mod.rs` plan-assembly contract.

### State ownership

Green. `ScenarioSpec` owns mutable wire data; successful conversion produces immutable `Scenario`
with a preserved parsed source DTO, private checked projections, and a private crate-private
edge-ID-keyed AST bundle; `ScenarioBuilder` privately owns its draft DTO; `ExecutionPlan` receives
that bundle by move and owns checked runtime projections. No mutable checked map or AST escapes.

### Data and control flow

Green. The path from serde DTO or checked builder to `Scenario`, then the validation-owned plan
assembler, then checked engine matching is explicit. DTO and checked facade paths converge once,
formula parsing occurs exactly once at their shared conversion gate, plan assembly moves the ASTs
into edge-index slots without reparsing, and source-spec inspection is separate from runtime
decisions.

### Hidden design decisions

Green. The packet resolves all node-family mappings, default normalization, explicit mismatch
behavior, key/ID order, duplicate recovery, target/ID ownership, nonzero fields, error taxonomy,
public receiver styles, serde policy, parsed-DTO compatibility baseline, enum extensibility, exact
active/inactive formula kinds, AST ownership, and exact compile facade behavior. No
worker is asked to choose typestate, naming, replacement behavior, or a migration policy.
The implementation shape freezes config defaults/setters, all node constructors/common setters,
resource/state connection construction, edge constructors/common setters, and the complete builder
signatures, leaving spec 040 no desugaring-name choice.

### Slice quality and dependencies

Green. Six serial/ordered vertical slices isolate checked types, builder behavior, the risky
cross-module migration, compatibility proof, docs, and independent review. The engine slice has a
spec-level 037 dependency, a before-implementation checkpoint, and an explicit verified-T004 stop
guard because the task validator rejects cross-spec task edges. Spec 040's downstream dependency is
stated without coupling this spec to macro implementation.

### Test seams and validation

Green. Unit, validation, plan, engine, integration, fixture, README, and rustdoc compile-fail seams
are named. Formula tests enumerate resource-transfer, modifier-state across targets, disabled,
inactive transfer/state control, invalid syntax, bundle alignment, and no-reparse cases. Commands
cover focused feedback, formatting, doctests, clippy with warnings denied, and the full all-target
suite. No service/network approval is needed.

### Compatibility and rollout

Green. Existing serde types, fields, aliases, defaults, and omission rules remain. Compatibility is
correctly measured against serialization of the parsed DTO, not raw alias/default spelling. Valid
DTO compile behavior is preserved. Newly rejected cases are limited to invalid duplicated
representations and are documented as intentional 0.2 strictness. The parsed DTO remains available
for persisted documents and no backfill is required.

### Repository convention fit

Green. The design retains deterministic `BTreeMap`/`BTreeSet` ordering, existing `SetupError`
paths, `Simulator` as facade, integration tests for behavior, and Rustdoc/README examples. New
module boundaries keep the already-large wire file from mixing checked execution types into serde
declarations.

### Worker context isolation

Green. Tasks can receive the normative contracts from this shape without reading `raw/`, current
state, design dialogue, or rejected alternatives. Required file reads can be narrow and explicit.

## Adversarial checks

- A loaded `Delay` plus `Queue` config cannot reach the engine: conversion fails at the node config
  path.
- A node map key that differs from `NodeSpec.id` cannot poison plan indexes: key/ID validation is
  first.
- A state node target cannot retain an irrelevant edge ID, and edge/formula targets cannot omit
  one: the checked enum owns the ID selectively.
- A duplicate checked-builder insertion cannot silently overwrite: `Entry::Occupied` errors and
  leaves the first value intact.
- A zero delay/release/token/denominator cannot need runtime repair: checked/compiled storage is
  nonzero.
- Checked conversion adds no DTO drift: serialization equals the parsed DTO baseline, while raw
  alias/default spelling is explicitly outside the contract.
- Every active resource-transfer and modifier-state formula AST is retained in `Scenario`; inactive
  state-edge transfer and nonmodifier control strings are explicitly excluded and tested.
- `compile_checked` consumes and moves that private bundle into 037 `CompiledExpressions`; no plan,
  simulator, or engine path reparses formula text or exposes `CompiledExpr` publicly.
- A builder user does not need to serialize or reconstruct a DTO before compile:
  `compile_checked` is defined.
- A worker cannot declare victory after introducing public sums while the engine still reads DTO
  tags: the migration task and no-fallback acceptance criteria prohibit the duplicate path.
- The macro work cannot bypass invariants: spec 040 is required to expand to this checked builder.
- The macro work cannot invent API: every expansion target is a frozen public constructor/setter,
  and macro-only helpers/private-field access are prohibited.

## Review conclusion

No semantic fixups remain. The frozen handoff may compile an active spec. The final spec must retain
the 037 spec dependency/checkpoint/T004 guard, the parse-once AST carrier contract, and the
independent Sol/high review task.

OVERALL: GREEN
cheap_worker_ready: yes
required_fixups: none
