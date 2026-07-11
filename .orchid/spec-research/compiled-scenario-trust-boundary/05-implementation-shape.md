# Implementation Shape

## Ownership Tree

```text
src/plan.rs                    new private plan owner and public opaque handle
src/validation/mod.rs          private checked compiler/config validators
src/engine/mod.rs              private run engine consuming immutable plan queries
src/batch/mod.rs               private batch orchestrator using the shared handle
src/simulator.rs               stable public facade and checked compile entrypoint
src/types/identifiers.rs       checked identifier Serde implementation
src/types/mod.rs               identifier and DTO round-trip tests
src/lib.rs                     private core modules; root CompiledScenario export
src/prelude.rs                 CompiledScenario prelude export
src/testkit/mod.rs             facade-based public fixture helpers
src/testkit/pikmin.rs          accessor-based compiled-plan inspection
tests/public_api.rs            opaque/accessor/TryFrom/Send+Sync contract tests
tests/perf_determinism.rs      facade migration and deterministic replay checks
tests/rstest_testkit.rs        facade migration
tests/parity/differential.rs   facade migration and parity preservation
tests/readme_snippets.rs       final README accessor drift checks
benches/simulation.rs          accessor/facade use and retained-plan benchmark coverage
README.md                      0.2 compile/inspection migration example
CHANGELOG.md                   explicit breaking API migration note
```

No worker may edit `specs/index.md`, `specs/_handoff.md`, old specs, capture-policy types, or
checked-authoring/macro surfaces for this spec.

## Public Contract

`src/plan.rs` defines the durable public shape:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledScenario(Arc<ExecutionPlan>);

impl CompiledScenario {
    pub fn scenario_id(&self) -> &ScenarioId;
    pub fn source_spec(&self) -> &ScenarioSpec;
    pub fn node_ids(&self) -> &[NodeId];
    pub fn edge_ids(&self) -> &[EdgeId];
    pub fn node_count(&self) -> usize;
    pub fn edge_count(&self) -> usize;
}

impl TryFrom<ScenarioSpec> for CompiledScenario {
    type Error = SetupError;
}
```

The tuple field, `ExecutionPlan`, and every plan component remain non-public. Public methods return
shared borrows or copy counts only. `source_spec()` is explicitly the canonical authoring/wire DTO
that was compiled; engine code uses crate-private execution-plan queries instead of traversing that
public accessor.

`src/lib.rs` ends with private `mod engine`, `mod batch`, `mod validation`, and `mod plan`, plus:

```rust
pub use plan::CompiledScenario;
```

`src/prelude.rs` re-exports `crate::CompiledScenario`. No root raw compile/run alias is added;
`Simulator` remains the orchestration facade.

## Internal Plan Contract

`src/plan.rs` owns private fields and crate-private shared-reference methods for this abstract
shape:

```rust
struct ExecutionPlan {
    source_spec: ScenarioSpec,
    nodes: Box<[CompiledNode]>,
    edges: Box<[CompiledEdge]>,
    node_index_by_id: BTreeMap<NodeId, NodeIndex>,
    edge_index_by_id: BTreeMap<EdgeId, EdgeIndex>,
    expressions: CompiledExpressions,
    routing: RoutingPlan,
    metrics: MetricPlan,
}

struct CompiledExpressions {
    transfer_by_edge: Box<[Option<CompiledExpr>]>,
    state_by_edge: Box<[Option<CompiledExpr>]>,
}
```

`NodeIndex(usize)` and `EdgeIndex(usize)` are separate crate-private newtypes. Ordered slices and
expression slots have the same length and key-derived order as their source maps. The compiler
constructs the complete value before wrapping it in `Arc`; no partially initialized public state
exists.

`CompiledNode` must carry every immutable value the current run engine reads repeatedly from a
node: canonical runtime ID, initial value, node kind/behavior, normalized trigger/action mode,
optional capacity, and optional delay/queue timing. `CompiledEdge` must carry canonical runtime ID,
report-facing endpoint IDs, typed endpoint indexes, enabled status, transfer semantics, token/state
connection semantics, and any target edge index needed by routing. Fields remain private to the
plan module with narrow crate-private queries.

`RoutingPlan` is the production home of the current `EngineStepPlan` data:

- resource edge groups keyed by controller `NodeIndex` and transfer-control mode;
- passive state-trigger targets;
- trigger outputs keyed by source `NodeIndex`;
- targets expressed as `NodeIndex`/`EdgeIndex`, not unresolved IDs.

`MetricPlan` owns deterministic tracked keys plus metric-key-to-node-index resolution for all
node-backed metrics. Existing end conditions remain behaviorally identical and resolve node/metric
lookups through plan indexes.

## Compiler Flow

1. `Simulator::compile(spec)` calls `CompiledScenario::try_from(spec)`.
2. `TryFrom` delegates to the private compiler in `src/validation/mod.rs`.
3. Existing structural and semantic validation runs in its current deterministic order so error
   precedence and `SetupError` paths remain stable for valid and invalid fixtures.
4. The compiler consumes the validated owned `ScenarioSpec`, enumerates its `BTreeMap` keys once,
   resolves typed endpoint indexes, and creates immutable node/edge projections.
5. The same `ExprRuntime::compile` result that proves syntax valid is inserted into the aligned
   expression plan rather than mapped to `()`.
6. Routing and metric plans are built from resolved entries.
7. The complete `ExecutionPlan` is wrapped once in `Arc` and returned as `CompiledScenario`.

Only `src/validation/mod.rs` may assemble a plan. `src/plan.rs` may expose a crate-private
`from_validated_parts` constructor, but it must not be callable downstream and must accept a
complete coherent plan, not six independent collections.

## Run and Batch Flow

`run_single_internal` receives `&CompiledScenario`, borrows the immutable plan once, and allocates
only per-run state. Delete `EngineExpressionCache::from_compiled` and
`EngineStepPlan::from_compiled`; expression evaluation and routing borrow the retained plan.
Iteration uses compiled node/edge slices and typed indexes, so invariant `expect`, missing-map
`continue`, and default/zero fallbacks attributable to plan inconsistency are removed.

`VariableRuntimeState`, `GateRuntimeState`, `TimelineRuntimeState`, `EngineState`, reports, events,
captures, and transfer logs remain per-run. Batch sequential/Rayon paths continue borrowing the
same compiled handle. There is no lock acquisition in the hot path.

Formula syntax errors remain `SetupError` compile failures. Runtime expression evaluation errors
that depend on live variables/values remain `RunError`; a successfully compiled plan cannot report
that its expression AST was absent or fail because a formula was reparsed.

## Identifier Serde Flow

In `src/types/identifiers.rs`, keep derived transparent `Serialize`, remove derived `Deserialize`,
and generate one manual `Deserialize<'de>` implementation per identifier through the existing
macro:

1. deserialize the wire value as `String`;
2. call the existing `TryFrom<String>` implementation;
3. map `IdentifierError` with `serde::de::Error::custom`.

Do not duplicate the empty/control checks inside the Serde implementation. Tests in
`src/types/mod.rs` must exercise all four identifier types, invalid JSON strings, valid scalar
round trips, and a `ScenarioSpec` round trip whose IDs occupy map-key and value positions.

## Compatibility and Consumer Migration

The final migration is intentionally atomic at the spec boundary:

- change README/`tests/readme_snippets.rs` from `compiled.scenario.id` to
  `compiled.scenario_id()`;
- change benchmark field reads to `node_count`, `edge_count`, `node_ids`, and `edge_ids`;
- change Pikmin tracked-metric inspection to `compiled.source_spec().tracked_metrics`;
- change integration tests from `anapao::validation::{compile_scenario, CompiledScenario}` to
  root `CompiledScenario` plus `Simulator::compile`/`TryFrom`;
- change raw `anapao::engine::run_single` and `anapao::batch::run_batch` calls to
  `Simulator::run` and `Simulator::run_batch`;
- update testkit compile helpers to the facade where they are public-facing;
- remove any temporary `validation::CompiledScenario` re-export introduced to keep an earlier
  slice green;
- document the breaking module/field paths and exact replacements under `CHANGELOG.md` Unreleased
  and in the README tutorial.

The aggregate `Cargo.toml` 0.2 version bump/publication is release-manager work outside this spec.

## Test Seam Map

- `src/types/mod.rs`: invalid/valid identifier Serde and map-key DTO round trips.
- `src/plan.rs` unit tests: key order, index alignment, accessor results, `Arc::ptr_eq` clone proof,
  plan equality, and compile-time `Send + Sync` assertion.
- `src/validation/mod.rs` unit tests: existing error precedence plus retained transfer/state AST,
  routing groups, metrics, and deterministic compilation equality.
- `src/engine/mod.rs` unit tests: no run-time AST/route construction, identical formula/state/gate
  results, and absence of compiled-plan-missing error branches.
- `tests/public_api.rs`: root/prelude type naming, `TryFrom`, read-only accessor behavior, source
  clone isolation, repeat runs from clones, and public facade usage.
- `tests/perf_determinism.rs` and `tests/parity/differential.rs`: deterministic and parity behavior
  through `Simulator`.
- Existing `tests/failure_path_batch_events.rs`, `tests/rstest_testkit.rs`,
  `tests/readme_playbook.rs`, and `tests/readme_snippets.rs`: facade/event/docs behavior.
- `benches/simulation.rs`: compile cost remains separately measured; repeated expression/gate/state
  and sequential/Rayon run benches exercise retained plan reuse.

## Vertical Slices

### T001 — Enforce Identifier Invariants During Deserialization

Write only `src/types/identifiers.rs` and `src/types/mod.rs`. Land the checked Serde implementation
and full wire-compatibility tests. This slice is independent and must receive fresh Sol/high review
because it changes a persisted public type boundary.

### T002 — Establish the Opaque Immutable Execution Plan

Add `src/plan.rs`; update `src/validation/mod.rs`, `src/engine/mod.rs`, `src/batch/mod.rs`,
`src/simulator.rs`, and the minimum `src/lib.rs` wiring. Build ordered compiled node/edge/metric
projections, typed indexes, `Arc` clone semantics, accessors, and `TryFrom`; migrate core execution
to immutable plan queries while preserving current per-run cache construction temporarily so the
slice is green. If a compatibility re-export is needed, label it for mandatory T004 removal.

### T003 — Retain Expressions and Routing Once

Update `src/plan.rs`, `src/validation/mod.rs`, `src/engine/mod.rs`, and focused benchmark/tests.
Move expression ASTs, `EngineStepPlan`, and metric resolution into compile-owned structures; remove
their per-run builders and impossible runtime error/fallback paths. This depends on T002 and must
prove deterministic single/Rayon parity.

### T004 — Complete the 0.2 Facade Migration and Public API Contract

After T001 and T003, make engine/batch/validation modules private; finalize root/prelude exports;
migrate every repository consumer, README snippet, benchmark, and testkit use; add public API tests
and changelog migration notes; remove every temporary adapter or transitional name. This is the
productionization/final API gate and the cross-spec dependency target.

## Validation Commands

Run after each focused slice as applicable, and run the full set before T004 completion:

```bash
cargo fmt --all -- --check
cargo test --lib types::tests::identifiers
cargo test --lib validation::tests
cargo test --lib engine::tests
cargo test --test public_api
cargo test --test perf_determinism
cargo test --test parity_rulebook
cargo test --all-targets
cargo test --all-targets --features parallel
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc
cargo bench --bench simulation
```

The Criterion command is a smoke/regression check, not a hard wall-clock threshold. Use the
repository `scripts/bench-criterion` save/compare flow when a same-machine baseline is available.

Every invariant-owning or public-API task requires an independent fresh Sol/high validator after
its commands pass. That validator must inspect the diff, plan immutability, unsafe absence,
determinism, wire compatibility, and public-path migration rather than accepting worker claims.

## Anti-Goals and Escalation

- Do not change simulation semantics to make the refactor easier.
- Do not expose `ExecutionPlan`, compiled node/edge/index types, ASTs, routing, or metric plans.
- Do not introduce an unchecked plan constructor, mutable accessor, lock, lazy cache, or compiled
  payload serialization.
- Do not add builder/sum-type/key-ID validation, capture-policy changes, or macros here.
- Do not leave temporary compatibility re-exports, `TODO`, prototype/tracer names, or raw module
  usage after T004.
- Escalate if preserving valid fixture behavior requires a report/event/RNG/order change; if a plan
  component is not `Send + Sync`; if a malformed-ID wire payload is known to be intentionally
  accepted; or if another active spec changes the frozen accessor/constructor contract.
