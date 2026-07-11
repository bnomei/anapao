# Design: Compiled Scenario Trust Boundary

## Objective

Make `CompiledScenario` a coherent, opaque, cheaply shared product of successful compilation. Move
run-invariant expression, routing, and metric work into that immutable product; keep seed/step state
per run; enforce identifier invariants at the Serde boundary; and finish the supported 0.2 surface
through `Simulator` and read-only accessors.

## Scope

This spec owns:

- the new private plan module and compiled execution representation;
- checked plan construction and run-engine consumption;
- eager retained formula/routing/metric work;
- checked deserialization for all public identifier newtypes;
- root/prelude compiled-handle exports and raw execution-module privacy;
- migration of concrete README, test, testkit, parity, and benchmark consumers;
- compatibility notes and complete verification.

## Non-Goals

- Redesign `ScenarioSpec` authoring, node/connection sum types, key/embedded-ID validation, or a
  checked scenario builder.
- Redesign capture schedules, batch aggregation, or report retention.
- Add `scenario!` or other public macros.
- Change expression syntax, simulation algorithms, RNG derivation, event order, reports,
  assertions, artifacts, or serialized `ScenarioSpec` shape.
- Serialize `CompiledScenario` or any internal plan structure.
- Publish the crate or own the aggregate 0.2 package-version/release operation.

## Distilled Current-State Facts

- `CompiledScenario` is defined under public `validation` with six independently mutable public
  fields (`src/validation/mod.rs:20-32`). The compiler clones the source and separately derives
  order/index collections (`src/validation/mod.rs:75-101`).
- Engine initialization, source generation, and state-edge iteration rejoin those structures with
  invariant `expect` calls (`src/engine/mod.rs:470-495`, `src/engine/mod.rs:704-723`,
  `src/engine/mod.rs:1682-1688`). Other paths mask misses with skips/defaults.
- Formula validation parses and discards immutable `CompiledExpr` values
  (`src/validation/mod.rs:422-443`, `src/validation/mod.rs:464-482`,
  `src/validation/mod.rs:591-600`).
- Every run reparses expressions and rebuilds resource/trigger groups before per-run state starts
  (`src/engine/mod.rs:85-135`, `src/engine/mod.rs:529-548`,
  `src/engine/mod.rs:746-809`). Batch repeats that path for each seed
  (`src/batch/mod.rs:60-80`, `src/batch/mod.rs:87-97`).
- Identifier derive accepts deserialized strings without calling the empty/control validator, even
  though `TryFrom<String>` already delegates to `new()` (`src/types/identifiers.rs:21-39`,
  `src/types/identifiers.rs:64-77`).
- The stable facade is already `Simulator`, but public raw modules and direct field consumers remain
  throughout integration tests, README, testkit, and benchmarks (`src/lib.rs:125-150`,
  `src/simulator.rs:20-35`, `README.md:65-84`, `benches/simulation.rs:15-75`).

## Architecture

### Single Plan Owner

Add private `src/plan.rs`. It owns:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct CompiledScenario(Arc<ExecutionPlan>);

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
```

`ExecutionPlan`, `CompiledNode`, `CompiledEdge`, `CompiledExpressions`, `RoutingPlan`,
`MetricPlan`, `NodeIndex`, and `EdgeIndex` are never public. Their fields remain private even within
the crate boundary; sibling modules consume narrow shared-reference methods. Distinct index
newtypes prevent accidental node/edge position interchange.

One `Arc` wraps the complete plan. The plan contains no interior mutability. A clone increments the
one reference count; it does not deep-clone source, nodes, edges, ASTs, or routing structures.

### Frozen Public Contract

`CompiledScenario` has no public constructor or fields. It exposes only:

```rust
pub fn scenario_id(&self) -> &ScenarioId;
pub fn source_spec(&self) -> &ScenarioSpec;
pub fn node_ids(&self) -> &[NodeId];
pub fn edge_ids(&self) -> &[EdgeId];
pub fn node_count(&self) -> usize;
pub fn edge_count(&self) -> usize;
```

The slices use original `BTreeMap` key order. `source_spec()` returns the canonical authoring/wire
DTO by shared reference; engine code uses internal projections, not this public inspection method.

Checked construction is shared:

```rust
impl TryFrom<ScenarioSpec> for CompiledScenario {
    type Error = SetupError;
}
```

`Simulator::compile` delegates to `TryFrom`. The private validation compiler is the only plan
assembler and accepts/returns a complete coherent value, never six separable collections.

### Runtime Projections

`CompiledNode` stores the runtime node ID, initial value, kind/behavior, normalized trigger/action
mode, optional capacity, and optional delay/queue timing used by the existing engine.

`CompiledEdge` stores the runtime edge ID, report-facing endpoint IDs, typed endpoint indexes,
enabled state, transfer semantics, token/state connection semantics, and resolved target edge index
where required. Collection keys remain canonical runtime identity for this spec. Validation of
collection keys against embedded DTO IDs belongs to the dependent checked-authoring spec.

`CompiledExpressions` contains edge-index-aligned transfer and state AST slots. The compiler retains
the `CompiledExpr` returned by the same parse that validates formula syntax. A successful compile
cannot later fail because an AST is missing or a formula is reparsed.

`RoutingPlan` owns the current `EngineStepPlan` resource controller groups, passive state-trigger
targets, and trigger outputs. Targets use typed indexes. `MetricPlan` owns deterministic metric keys
and node resolution.

### Shared and Per-Run State

Shared after compile:

- source inspection DTO;
- immutable compiled nodes/edges and indexes;
- expression ASTs;
- routing groups;
- metric resolution.

Allocated/mutated per run:

- `EngineState` values and step;
- variable values and RNG;
- gate RNG and weighted balancers;
- delay/queue timeline state;
- capture bookkeeping, reports, event emission, and transfer logs.

Rayon borrows the same immutable handle for each run. No plan lock exists in the hot path.

## Data Flow

```text
ScenarioSpec
  -> CompiledScenario::try_from / Simulator::compile
  -> existing validation order and SetupError precedence
  -> resolved node/edge projections + retained AST/routing/metric plans
  -> Arc<ExecutionPlan>
  -> Simulator::run / run_batch
  -> borrowed immutable plan + isolated per-run state
  -> unchanged RunReport / BatchReport / events / artifacts
```

Formula parse/syntax errors stay at compile time. Expression errors that depend on live variables
or values remain runtime errors. No compiled plan is serialized; persisted `ScenarioSpec` is
compiled after loading.

## Identifier Deserialization

Keep transparent `Serialize`; replace derived `Deserialize` with the macro-generated equivalent of:

```rust
let value = String::deserialize(deserializer)?;
Self::try_from(value).map_err(serde::de::Error::custom)
```

Do not duplicate validation logic. All four newtypes continue to serialize as JSON strings and work
as `BTreeMap` keys in `ScenarioSpec`. Invalid whitespace-only and escaped-control values become
Serde errors before compilation.

## Public Surface and Compatibility

The final state in `src/lib.rs` uses private `mod plan`, `mod validation`, `mod engine`, and
`mod batch`, and root-re-exports `plan::CompiledScenario`. `src/prelude.rs` re-exports the root
handle. Raw execution functions become `pub(crate)`/private; there is no permanent advanced module.

Consumer replacements are exact:

| Before | After |
| --- | --- |
| `compiled.scenario.id` | `compiled.scenario_id()` |
| `compiled.scenario` (read) | `compiled.source_spec()` |
| `compiled.node_order` | `compiled.node_ids()` |
| `compiled.edge_order` | `compiled.edge_ids()` |
| `validation::compile_scenario(&spec)` | `Simulator::compile(spec)` or `spec.try_into()` |
| `engine::run_single` | `Simulator::run` |
| `batch::run_batch` | `Simulator::run_batch` |
| `validation::CompiledScenario` | root/prelude `CompiledScenario` |

README and `CHANGELOG.md` Unreleased record these breaking replacements. Aggregate release/version
ownership stays outside this spec to avoid collision with sibling 0.2 work.

## Delivery Sequence

T001 and T002 are independent and may run in parallel. T003 consumes T002's plan. T004 consumes
T001 and T003 and is the final productionization/public-contract gate.

- T001: checked ID Serde and wire tests.
- T002: opaque Arc plan, typed runtime projections, accessors/TryFrom, and core engine migration.
  Existing raw module paths may remain temporarily so the independent slice stays buildable; all
  surviving implementation uses final production names.
- T003: retained expression/routing/metric plans and removal of per-run builders.
- T004: private modules, root/prelude facade, complete repository consumer/docs migration, removal
  of transitional compatibility paths, and full validation/review.

The checked-authoring and capture-policy specs depend concretely on
`037-compiled-scenario-trust-boundary/T004` before they edit or consume the final plan/engine/batch
surface. This spec has no prerequisite sibling spec.

## Reuse Targets

- Preserve validation order and `SetupError` construction in `src/validation/mod.rs`.
- Reuse immutable `CompiledExpr` and shared-reference evaluation in `src/expr/mod.rs:34-89`.
- Move, rather than reinterpret, `EngineStepPlan` data from `src/engine/mod.rs:746-809`.
- Preserve `Simulator` orchestration/config validation in `src/simulator.rs`.
- Preserve deterministic `BTreeMap` ordering and existing parity/event/report tests.
- Reuse existing expression, gate, state-modifier, sequential batch, and Rayon Criterion cases.

## Traceability

| Requirement | Tasks | Validation | Risk/Open Decision |
| --- | --- | --- | --- |
| R001 | T002, T004 | plan/public API tests, diff review | public compatibility break; resolved for 0.2 |
| R002 | T002, T003, T004 | Arc identity, Send+Sync, parallel replay | no interior mutability allowed |
| R003 | T002, T003 | plan/validation/engine tests, parity | index migration semantic drift |
| R004 | T003, T004 | structural tests, expression/gate/state benches | no invented speed threshold |
| R005 | T001, T004 | invalid/valid scalar and DTO round trips | valid wire shape must remain exact |
| R006 | T002, T004 | public API/rustdoc/search checks | T004 removes every compatibility shim |
| R007 | T002, T003, T004 | all targets, parity, README, benchmark compile | broad consumer migration |
| R008 | T001, T002, T003, T004 | full command set plus Sol/high review | none |

## Validation Plan

Focused during implementation:

```bash
cargo test --lib identifiers
cargo test --lib validation::tests
cargo test --lib engine::tests
cargo test --test public_api
cargo test --test perf_determinism
cargo test --test parity_rulebook
```

Final gate:

```bash
cargo fmt --all -- --check
cargo test --all-targets
cargo test --all-targets --features parallel
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
cargo test --doc
cargo bench --bench simulation
```

Criterion is a smoke/regression gate. When a same-machine saved baseline exists, use
`./scripts/bench-criterion save`/`compare` and attach the result; no unsupported percentage target
is introduced.

Each task uses `verification_mode = "required"`. After worker commands pass, a fresh Sol/high
validator independently reviews the diff and applicable test evidence. T004's validator must also
search for old paths, temporary re-exports, prototype/tracer names, `TODO`s, interior mutability,
unchecked constructors, and compiled-plan Serde.

## Risks and Escalation

- Resolved-index migration can accidentally alter ordering or fallback behavior. Stop if a valid
  fixture needs changed RNG, transfer, event, report, or artifact output.
- Making modules private is intentionally breaking. Stop if a supported consumer cannot be
  expressed through the frozen facade/accessors without expanding the public contract.
- Stop if any retained plan component is not `Send + Sync` or would require mutable shared state.
- Stop if known persisted data intentionally relies on empty/control identifier values.
- There are no open design decisions at dispatch.

## Pre-Dispatch Review

Result: GREEN.

- The promoted research handoff has a GREEN semantic shape review and no unresolved decisions.
- The task schema validator reports four tasks and eight requirements; the forbidden-reference pass
  confirms no worker-facing artifact depends on the research packet.
- Every requirement has stable EARS wording, acceptance anchors, task coverage, and validation.
- T001 and T002 have disjoint write scopes and may run in parallel. T003 depends on T002; T004
  depends on T001 and T003 and is the explicit productionization/cross-spec gate.
- All four tasks deliberately use `sol`/`high`: T001 owns persisted public invariants, T002 owns the
  public/architectural trust boundary, T003 owns cross-module deterministic execution planning, and
  T004 owns the breaking public API and independent final review.
- Every task uses `verification_mode = "required"` and explicitly requests a fresh Sol/high
  validator after machine checks. No Luna task is asked to define or validate an invariant.
- Scopes, read allowlists, reuse targets, stop conditions, and command evidence are concrete. No
  task reads raw research or requires Docker/network access.
- The final contract contains no tracer/prototype survivor, placeholder, unchecked constructor,
  interior mutability, compiled-plan serialization, or unresolved release decision.
- Downstream dependency `037-compiled-scenario-trust-boundary/T004` is concrete and present for the
  capture-policy and checked-authoring specs to consume.
