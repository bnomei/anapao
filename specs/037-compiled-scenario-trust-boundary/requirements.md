# Requirements: Compiled Scenario Trust Boundary

## R001 — Opaque Coherent Compilation Product

R001: WHEN a valid `ScenarioSpec` is compiled THE SYSTEM SHALL return an opaque
`CompiledScenario` whose source DTO and derived execution state cannot be mutated or replaced
independently through safe downstream Rust.

Acceptance anchors:

- `src/plan.rs` exposes no public field, plan type, mutable accessor, or unchecked constructor.
- Public API tests compile and run through `Simulator::compile` and `TryFrom<ScenarioSpec>`.
- A cloned `source_spec()` DTO can be mutated without changing accessors or subsequent run results
  from the compiled handle.

## R002 — Cheap Immutable Sharing

R002: WHEN a `CompiledScenario` is cloned or shared by sequential or Rayon batch execution THE
SYSTEM SHALL share one immutable `Send + Sync` execution plan without deep-copying it or acquiring
execution-plan locks.

Acceptance anchors:

- A plan-module unit test proves clones use the same `Arc` allocation.
- A compile-time assertion proves `CompiledScenario: Send + Sync`.
- Sequential and `parallel` feature replay tests return deterministic equivalent reports.

## R003 — Resolved Deterministic Runtime Projections

R003: WHEN compilation succeeds THE SYSTEM SHALL build deterministic key-ordered compiled node and
edge projections with distinct typed indexes so execution does not depend on fallible source-map
re-lookup or interchange node and edge positions.

Acceptance anchors:

- Validation/plan tests prove ordered slice, index, endpoint, and metric alignment.
- Engine paths no longer contain invariant `expect`, silent skip, zero, or default branches caused
  by a compiled order/index/source mismatch.
- Existing parity, event-order, report, and deterministic run tests remain green.

## R004 — Compile Run-Invariant Work Once

R004: WHEN a successfully compiled scenario is run repeatedly THE SYSTEM SHALL reuse expression
ASTs, resource/trigger routing groups, and metric resolution built once during compilation rather
than parsing or planning them per run or per batch seed.

Acceptance anchors:

- `EngineExpressionCache::from_compiled` and `EngineStepPlan::from_compiled` are removed from run
  setup and have no replacement per-run builder.
- Validation/plan tests prove transfer and state AST slots plus routing/metric plans are retained.
- Formula syntax errors remain compile-time `SetupError`s; run-time evaluation errors that depend
  on live values remain `RunError`s.
- Expression, gate, state-modifier, sequential batch, and Rayon benchmark/test cases complete.

## R005 — Checked Identifier Deserialization

R005: WHEN `ScenarioId`, `NodeId`, `EdgeId`, or `MetricKey` is deserialized THE SYSTEM SHALL enforce
the same trimmed-empty and control-character rejection as its checked constructor while preserving
the existing valid JSON string representation and map-key behavior.

Acceptance anchors:

- Focused tests reject whitespace-only and escaped-control JSON strings for all four types.
- Valid scalar IDs and a `ScenarioSpec` containing ID map keys round-trip without shape changes.
- The generated `Deserialize` path delegates to existing `TryFrom<String>`/`new()` validation.

## R006 — Final 0.2 Facade

R006: WHERE the 0.2 public API applies THE SYSTEM SHALL expose `CompiledScenario` at the crate root
and in the prelude, expose checked compilation through `Simulator::compile` and
`TryFrom<ScenarioSpec>`, expose read-only inspection through the frozen accessors, and keep engine,
batch, and raw validation modules private.

The frozen accessor signatures are:

```rust
pub fn scenario_id(&self) -> &ScenarioId;
pub fn source_spec(&self) -> &ScenarioSpec;
pub fn node_ids(&self) -> &[NodeId];
pub fn edge_ids(&self) -> &[EdgeId];
pub fn node_count(&self) -> usize;
pub fn edge_count(&self) -> usize;
```

Acceptance anchors:

- `src/lib.rs` declares `plan`, `engine`, `batch`, and `validation` privately and re-exports only
  the supported compiled handle/facade contract.
- `src/prelude.rs` exports `CompiledScenario`.
- No permanent `advanced` module, raw alias, unchecked compile function, or transitional
  validation re-export remains.
- README and changelog contain exact old-to-new migration examples.

## R007 — Complete Consumer Migration Without Semantic Drift

R007: WHEN repository examples, integration tests, testkit helpers, parity diagnostics, and
benchmarks consume a compiled scenario THE SYSTEM SHALL use the final facade/accessors and preserve
valid-scenario RNG, ordering, event, report, assertion, and artifact behavior.

Acceptance anchors:

- Repository search finds no direct compiled fields or downstream `anapao::engine`,
  `anapao::batch`, or `anapao::validation` imports.
- README drift tests and rustdoc examples use `scenario_id()`/other accessors.
- Existing all-target, parity, event-streaming, artifact, and testkit suites remain green.

## R008 — Repository Quality Gates and Independent Review

R008: WHEN this migration is complete THE SYSTEM SHALL pass formatting, all-feature Clippy,
all-target/default/parallel/all-feature tests, rustdoc, focused determinism/parity checks, and
Criterion smoke validation, followed by a fresh independent Sol/high review of every
invariant-owning public-API change.

Acceptance anchors:

- Every command listed in `design.md` completes successfully.
- Independent review verifies plan immutability, checked construction/Serde, thread safety,
  deterministic behavior, public API migration, and absence of temporary scaffolding.
- Any same-machine benchmark comparison is recorded as evidence but is not subject to an invented
  percentage threshold.
