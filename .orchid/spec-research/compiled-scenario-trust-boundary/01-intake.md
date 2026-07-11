# Intake: Compiled Scenario Trust Boundary

## Verbatim Starting Finding

### 1. Turn `CompiledScenario` into a real trust boundary

`CompiledScenario` exposes six independently mutable fields, including its source scenario and derived indexes ([validation/mod.rs](/Users/bnomei/PROJECTS/anpao/src/validation/mod.rs:25)). Safe downstream code can desynchronize them, while the engine assumes consistency and calls `expect` ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:471)).

The boundary also leaves performance on the table: validation compiles formulas only to discard them ([validation/mod.rs](/Users/bnomei/PROJECTS/anpao/src/validation/mod.rs:595)); every run rebuilds expression caches and step plans ([engine/mod.rs](/Users/bnomei/PROJECTS/anpao/src/engine/mod.rs:92)). A batch repeats that for every seed.

Refactor toward:

```rust
#[derive(Clone)]
pub struct CompiledScenario(Arc<ExecutionPlan>);

struct ExecutionPlan {
    scenario_id: ScenarioId,
    nodes: Box<[CompiledNode]>,
    edges: Box<[CompiledEdge]>,
    expressions: CompiledExpressions,
    routing: RoutingPlan,
    metrics: MetricPlan,
}
```

Expose read-only methods such as `scenario_id()`, `source_spec()`, `node_ids()`, and `node_count()`. Make `engine`, `batch`, and raw validation entry points private or explicitly `advanced`.

Also fix identifier deserialization here: the ID newtypes derive transparent `Deserialize` directly ([identifiers.rs](/Users/bnomei/PROJECTS/anpao/src/types/identifiers.rs:21)), bypassing the validation performed by `new()` ([identifiers.rs](/Users/bnomei/PROJECTS/anpao/src/types/identifiers.rs:30)).

Tavily validation: Rust guidance favors private fields for invariant-bearing types and `TryFrom` for checked conversion. Cargo classifies privatizing public fields as breaking, so add accessors first and finish this in `0.2`. [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/future-proofing.html), [TryFrom](https://doc.rust-lang.org/std/convert/trait.TryFrom.html), [Cargo SemVer](https://doc.rust-lang.org/cargo/reference/semver.html).

Confidence: very high. This improves correctness, performance, and future API freedom together.

## Execution Directive

Produce a complete `make-research` packet and an active `make-specs` specification for this
finding. The implementation contract must cover the trust boundary, retained run-invariant plans,
identifier deserialization, public API migration, documentation, compatibility, and verification
end to end. It must not stop at an MVP, tracer-only result, or first slice.

## Success Signals

- Safe downstream Rust cannot mutate or replace any source or derived component inside a
  `CompiledScenario`.
- Cloning `CompiledScenario` shares one immutable plan and remains suitable for Rayon batch use.
- A successful compile retains expression ASTs and routing/metric execution plans; repeated runs
  do not parse or rebuild those structures.
- Engine code consumes resolved node/edge plan entries and no longer treats missing entries in a
  compiled plan as recoverable or reaches invariant `expect` calls caused by public mutation.
- Every identifier type enforces `new()` invariants during Serde deserialization while preserving
  its existing JSON string representation.
- The 0.2 public surface uses `Simulator` plus an opaque root/prelude `CompiledScenario`; raw
  `engine`, `batch`, and validation entry points are no longer downstream API.
- README snippets, integration tests, testkit helpers, benchmarks, and migration notes use the
  final public API.
- All local validation gates are green, including all-feature lint/test coverage and focused
  determinism/parallel checks.

## Constraints

- Preserve deterministic `BTreeMap` key order and all valid-scenario run, event, report, and
  artifact behavior.
- Preserve `ScenarioSpec` as the serde-compatible source/wire DTO and expose it only by shared
  reference from the compiled handle.
- Keep the plan immutable after construction; do not introduce locks or lazy mutable caches.
- Treat public-field and public-module removal as a coordinated 0.2 compatibility break.
- Use exact, root-relative worker paths and complete task slices.
- This research/spec effort does not implement source changes or commit.

## Initial Scope

- `src/plan.rs` as the single owner of the opaque handle and immutable execution-plan model.
- `src/validation/mod.rs` as the checked compiler and plan assembler.
- `src/engine/mod.rs`, `src/batch/mod.rs`, and `src/simulator.rs` as execution/facade consumers.
- `src/types/identifiers.rs` and identifier tests for checked deserialization.
- `src/lib.rs` and `src/prelude.rs` for the 0.2 public surface.
- Existing README, tests, testkit code, benchmarks, and changelog entries that use the old API.

## Non-Goals

- Redesigning `ScenarioSpec` authoring into sum types or adding a scenario builder/macro; those are
  separate checked-authoring and macro findings.
- Changing capture/retention semantics or batch report memory shape.
- Changing simulation algorithms, expression syntax, RNG policy, event order, report schemas, or
  artifact schemas.
- Serializing/deserializing `CompiledScenario` or persisting compiled AST/routing internals.
- Publishing the crate or coordinating the aggregate 0.2 release/version bump across sibling
  specs.
