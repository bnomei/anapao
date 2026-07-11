# Make Specs Handoff: compiled-scenario-trust-boundary

## Status

- research_id: compiled-scenario-trust-boundary
- status: frozen
- intended_spec_slug: compiled-scenario-trust-boundary
- shape_review: GREEN
- cheap_worker_ready: yes

## Objective

Replace the publicly mutable `CompiledScenario` aggregate with one opaque, cheaply cloneable,
immutable execution plan; retain expression/routing/metric work at compile time; enforce identifier
invariants through Serde; and complete the coordinated 0.2 migration to a `Simulator`-only
execution facade without changing valid simulation behavior.

## Requirements Seed

- R001: WHEN a valid `ScenarioSpec` is compiled THE SYSTEM SHALL return an opaque
  `CompiledScenario` whose source and derived execution state cannot be mutated independently.
- R002: WHEN a `CompiledScenario` is cloned or shared with Rayon THE SYSTEM SHALL share one
  immutable `Send + Sync` execution plan without locks or deep plan copies.
- R003: WHEN compilation succeeds THE SYSTEM SHALL resolve deterministic ordered node/edge
  projections and typed lookup indexes so execution does not depend on fallible source-map
  re-lookup.
- R004: WHEN a compiled scenario is run repeatedly THE SYSTEM SHALL reuse expression ASTs,
  routing groups, and metric resolution built once during compilation.
- R005: WHEN any public identifier is deserialized THE SYSTEM SHALL enforce the same empty/control
  invariants as its checked constructor while preserving valid JSON string representation.
- R006: WHERE the 0.2 public API applies THE SYSTEM SHALL expose `CompiledScenario` at root/prelude,
  checked compile through `Simulator`/`TryFrom`, and read-only inspection through the six frozen
  accessors while keeping engine/batch/raw validation private.
- R007: WHEN repository examples, tests, testkit helpers, and benchmarks consume compiled scenarios
  THE SYSTEM SHALL use the final facade/accessors and preserve deterministic behavior.
- R008: WHEN the migration is complete THE SYSTEM SHALL pass focused invariant tests, parity,
  deterministic single/Rayon replay, rustdoc, fmt, Clippy, all-target/all-feature tests, and
  Criterion smoke validation.

## Scope

In scope:

- Add private `src/plan.rs` for the opaque handle and immutable plan model.
- Refactor validation/engine/batch/simulator around a checked, compile-owned execution plan.
- Retain compiled formulas, routing groups, and metric resolution.
- Add checked Serde for all identifier newtypes.
- Finalize root/prelude exports and private raw execution modules.
- Migrate README, changelog, integration tests, testkit helpers, and benchmarks.

Out of scope:

- Checked authoring sum types, builder, and key/embedded-ID validation.
- Capture-policy/batch-retention redesign.
- Scenario macros.
- Simulation, expression-language, RNG, event, report, or artifact semantic changes.
- Compiled-plan serialization or aggregate 0.2 release publication/version ownership.

## Current-State Facts

- `CompiledScenario` exposes six mutable public fields (`src/validation/mod.rs:20-32`).
- Compilation clones the source and builds order/index collections independently
  (`src/validation/mod.rs:75-101`).
- Engine invariant `expect` paths rejoin those collections at run time
  (`src/engine/mod.rs:470-495`, `src/engine/mod.rs:704-723`,
  `src/engine/mod.rs:1682-1688`).
- Formula validation discards parsed ASTs (`src/validation/mod.rs:591-600`).
- Every run rebuilds expression and routing caches (`src/engine/mod.rs:529-548`), and every batch
  seed calls that run path (`src/batch/mod.rs:60-80`, `src/batch/mod.rs:87-97`).
- Identifier derive bypasses `new()` during deserialization
  (`src/types/identifiers.rs:21-39`, `src/types/identifiers.rs:64-77`).
- README, benchmarks, and integration tests directly consume fields/raw module paths
  (`README.md:65-84`, `benches/simulation.rs:15-28`,
  `tests/perf_determinism.rs:1-12`, `tests/rstest_testkit.rs:1-10`,
  `tests/parity/differential.rs:1-17`).

## Decisions

- `src/plan.rs` owns the public opaque handle and all private/crate-private plan types.
- `CompiledScenario` wraps one immutable `Arc<ExecutionPlan>`; no interior mutability or locks.
- Public accessors are exactly `scenario_id`, `source_spec`, `node_ids`, `edge_ids`, `node_count`,
  and `edge_count` with borrowed slice/reference or copied-count returns.
- `TryFrom<ScenarioSpec, Error = SetupError>` and `Simulator::compile` share the private checked
  compiler; no public unchecked constructor exists.
- Runtime projections use typed node/edge indexes, deterministic boxed slices, retained AST slots,
  routing groups, and metric resolution.
- Identifier `Deserialize` deserializes `String` and delegates to existing `TryFrom<String>`.
- Final 0.2 surface makes engine/batch/validation private and migrates all repository consumers.
- `ScenarioSpec` remains the serializable source of truth; `CompiledScenario` is ephemeral.

Rejected:

- Documentation-only public-field invariants, `Rc`, locks/lazy caches, permanent advanced raw
  execution APIs, compiled-plan Serde, and scope overlap with sibling findings.

Open:

- None.

## Implementation Shape Excerpts

- Add `src/plan.rs` with `CompiledScenario(Arc<ExecutionPlan>)`, private fields, typed indexes,
  compiled nodes/edges, `CompiledExpressions`, `RoutingPlan`, and `MetricPlan`.
- Keep plan assembly in `src/validation/mod.rs`; preserve validation order/error paths and retain
  the AST returned by formula validation.
- Migrate `src/engine/mod.rs` to plan slices/indexes, delete per-run expression/routing builders,
  and keep all seed/step state local.
- Keep `src/batch/mod.rs` sharing `&CompiledScenario`; prove sequential/Rayon parity.
- Finalize checked facade in `src/simulator.rs`, private module declarations/root export in
  `src/lib.rs`, and prelude export in `src/prelude.rs`.
- Implement checked identifier Serde in `src/types/identifiers.rs` and complete wire tests in
  `src/types/mod.rs`.
- Migrate concrete consumers in `src/testkit/mod.rs`, `src/testkit/pikmin.rs`,
  `tests/public_api.rs`, `tests/perf_determinism.rs`, `tests/rstest_testkit.rs`,
  `tests/parity/differential.rs`, `tests/readme_snippets.rs`, `benches/simulation.rs`, `README.md`,
  and `CHANGELOG.md`.
- Remove every transitional compatibility re-export in final T004.

## Suggested Spec Shape

- spec_kind: migration
- fanout_policy: parallel-when-ready
- execution_policy: auto-continue
- task_slices:
  - T001: Enforce identifier invariants during deserialization (`sol` / `high`, independent).
  - T002: Establish the opaque immutable execution plan (`sol` / `high`, independent).
  - T003: Retain expressions and routing once (`sol` / `high`, depends T002).
  - T004: Complete the 0.2 facade migration and public API contract (`sol` / `high`, depends
    T001 and T003; final productionization and cross-spec gate).

Cross-spec:

- Checked-authoring plan migration must depend on
  `037-compiled-scenario-trust-boundary/T004`.
- Capture-policy work that consumes the post-refactor engine/batch plan must also depend on T004.
- This spec has no prerequisite sibling spec.

## Validation

- `cargo fmt --all -- --check`
- Focused identifier, validation, engine, public API, performance-determinism, and parity tests.
- `cargo test --all-targets`
- `cargo test --all-targets --features parallel`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets --all-features`
- `cargo test --doc`
- `cargo bench --bench simulation`
- Fresh Sol/high independent review for every public-API/invariant task and final T004 diff.

## Worker Context Policy

- Workers may read only the concrete source/test/docs paths listed in their task frontmatter.
- Workers must receive the relevant public/internal contract excerpt directly in task `Context`.
- Workers must not be sent to:
  - `.orchid/spec-research/compiled-scenario-trust-boundary/raw/`
  - broad current-state research
  - decision dialogue history
  - stale alternatives or rejected approaches
  - sibling spec research packets
