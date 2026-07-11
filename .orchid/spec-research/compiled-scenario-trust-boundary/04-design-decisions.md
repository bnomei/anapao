# Design Decisions

## D001: Give the Immutable Plan One Private Owner

Decision: add private module `src/plan.rs`. It owns the public opaque `CompiledScenario` handle
and crate-private `ExecutionPlan`, `CompiledNode`, `CompiledEdge`, `CompiledExpressions`,
`RoutingPlan`, `MetricPlan`, `NodeIndex`, and `EdgeIndex` types. Its fields remain private and it
provides shared-reference query methods to sibling modules.

Rationale: moving the type out of `validation` separates the product of compilation from the
compiler, creates one representation owner, and lets `engine`, `batch`, and `validation` become
private without making the public return type unnameable.

Rejected:

- Keep the six fields public and document that callers must not mutate them. Safe Rust would still
  permit invariant violations.
- Put the plan in `engine`. Compilation/validation would then depend on execution internals and the
  stable facade would expose an engine-owned type.
- Make plan fields `pub(crate)`. This would reduce downstream risk but would not establish a strong
  internal construction/query boundary.

## D002: Use One `Arc<ExecutionPlan>` and No Interior Mutability

Decision: `CompiledScenario` is a single-field, opaque `Clone` handle around one
`Arc<ExecutionPlan>`. The plan and every retained AST/routing/metric structure are immutable after
checked construction. Do not add `Mutex`, `RwLock`, `Cell`, `RefCell`, lazy cache, or one `Arc` per
substructure.

Rationale: clones become constant-size shared handles, the existing Rayon path can share the same
immutable plan, and run-specific state remains local. A compile-time `Send + Sync` assertion and a
parallel replay test prove the required trait posture.

Rejected:

- Deep-clone the plan on every `CompiledScenario::clone`; this defeats cheap reuse.
- Use `Rc`; it cannot cross Rayon threads.
- Use `OnceLock`/locks to compile lazily; successfully compiled scenarios already paid parse and
  validation cost, and lazy mutation adds failure timing and synchronization complexity.

## D003: Freeze the Public Read-Only Contract

Decision: root and prelude re-export `CompiledScenario`. Its complete public inspection surface is:

```rust
pub fn scenario_id(&self) -> &ScenarioId;
pub fn source_spec(&self) -> &ScenarioSpec;
pub fn node_ids(&self) -> &[NodeId];
pub fn edge_ids(&self) -> &[EdgeId];
pub fn node_count(&self) -> usize;
pub fn edge_count(&self) -> usize;
```

`node_ids` and `edge_ids` preserve deterministic `BTreeMap` key order. `source_spec` returns only a
shared reference; cloning and mutating that DTO cannot affect the compiled plan. Do not expose
internal indexes, ASTs, routing groups, or mutable access.

Construction remains checked through:

```rust
impl TryFrom<ScenarioSpec> for CompiledScenario {
    type Error = SetupError;
}

pub fn Simulator::compile(spec: ScenarioSpec) -> Result<CompiledScenario, SetupError>;
```

`Simulator::compile` delegates to the `TryFrom` implementation; both reach the same private
validator/compiler. There is no public unchecked constructor.

## D004: Compile Runtime Projections Once

Decision: `ExecutionPlan` stores the source DTO for inspection plus runtime projections:

- `Box<[CompiledNode]>` and `Box<[CompiledEdge]>` in deterministic key order;
- private `BTreeMap` ID-to-typed-index maps for lookup at config/capture boundaries;
- `CompiledExpressions` with edge-index-aligned optional transfer and modifier ASTs;
- `RoutingPlan` containing the resource-controller groups and trigger targets currently rebuilt by
  `EngineStepPlan`;
- `MetricPlan` containing deterministic tracked metric keys and node resolutions.

`CompiledNode` contains the runtime-facing node ID, initial value, kind/mode, capacity, and timeline
properties needed by the current engine. `CompiledEdge` contains canonical ordered edge identity,
resolved `NodeIndex` endpoints, enabled/connection/transfer data, and report-facing node IDs.
`NodeIndex` and `EdgeIndex` are distinct crate-private newtypes so node/edge positions cannot be
mixed accidentally.

The plan compiler parses each needed formula exactly once and retains the returned `CompiledExpr`.
It builds routing and metric structures after structural validation succeeds. Engine evaluation
borrows these structures and never rebuilds them per run.

Rationale: this removes map/index desynchronization as a representable public state, removes
invariant `expect`/default paths, and converts repeated per-seed setup to one compile-time cost.

Scope boundary: key-versus-embedded-ID validation and sum-typed authoring are owned by the later
checked-authoring spec. This plan uses deterministic collection keys as runtime identity without
introducing that separate authoring policy.

## D005: Preserve Per-Run Ownership

Decision: `EngineState`, variable values/RNG, gate RNG/balancers, timeline queues, capture state,
event emission, reports, and transfer logs remain allocated per run. The shared plan contains no
run seed or mutable execution result.

Rationale: these values vary by seed/step and cannot be safely or meaningfully shared. The split
also keeps Rayon synchronization-free after compilation.

## D006: Make Identifier Deserialization Checked and Wire-Compatible

Decision: remove derived `Deserialize` from the invariant-bearing identifier newtype. Keep
transparent string serialization. Implement `Deserialize` once in the macro by deserializing a
`String`, then invoking the existing `TryFrom<String>`/`new()` path and mapping
`IdentifierError` through `serde::de::Error::custom`.

Required compatibility tests cover all four ID types, valid JSON string round trips, IDs used as
map keys inside `ScenarioSpec`, trimmed-empty rejection, and escaped control-character rejection.

Rejected:

- Validate identifiers only during scenario compilation. Invalid identifier values would still be
  constructible from persisted/session-facing payloads before compilation.
- Change the JSON representation to an object. The existing string wire contract is sufficient.

## D007: Make the 0.2 Facade the Only Public Execution Path

Decision: in the final migration task, change `engine`, `batch`, and `validation` module
declarations in `src/lib.rs` from `pub mod` to private `mod`. Their cross-module functions become
`pub(crate)` or private. Root/prelude re-export `CompiledScenario`, and downstream examples/tests
use `Simulator::{compile,run,run_batch,...}` plus public accessors.

This is an intentional 0.2 compatibility break. `README.md` and `CHANGELOG.md` must include exact
before/after migration examples. The aggregate release/version bump remains outside this spec so
sibling 0.2 specs do not race on release ownership.

Rejected: create a permanent `advanced` module. No external consumer in the repository establishes
a supported need for raw state initialization or unchecked execution, and retaining it would keep
the refactor surface public indefinitely.

## D008: Keep the Compiled Form Ephemeral

Decision: do not implement Serde for `CompiledScenario` or internal plan types. Persist and version
`ScenarioSpec`; compile after loading.

Rationale: compiled AST/routing representation is an internal optimization and should be free to
change without a storage migration.

## D009: Require Structural, Behavioral, Parallel, and Performance Evidence

Decision: verification must prove:

- invalid identifier deserialization is rejected and valid wire formats round-trip;
- accessors preserve deterministic order and source inspection;
- cloning shares one plan and `CompiledScenario: Send + Sync`;
- formulas/routing are present after compile and are not reconstructed in run setup;
- repeated single-thread and Rayon runs preserve deterministic reports/events;
- the old public field/raw-module uses are fully migrated;
- fmt, Clippy, all-target tests, rustdoc, and Criterion smoke gates pass.

No hard speed percentage is required because there is no frozen benchmark baseline or target
machine. The structural elimination of per-run parse/plan construction is the acceptance contract;
the existing benchmarks provide regression evidence.

## D010: Delivery and Cross-Spec Ordering

Decision: use four complete, green slices:

1. checked identifier deserialization;
2. opaque immutable execution plan and core execution migration;
3. retained expression/routing/metric plans and per-run rebuild removal;
4. final 0.2 facade, consumer/docs migration, and compatibility cleanup.

The first two may start independently. Slice 3 depends on slice 2. Slice 4 depends on all previous
slices and removes any temporary compatibility re-export used to keep slice 2 buildable.

The checked-authoring spec must depend on final slice
`037-compiled-scenario-trust-boundary/T004` because it replaces plan projections and checked
assembly inputs. Capture-policy work that consumes or edits the post-refactor engine/batch surface
must use the same concrete dependency. This spec itself has no prerequisite sibling spec.

## Open Decisions

None. The requested 0.2 break, accessors, checked conversion, Arc-backed plan, retained compile
artifacts, and facade direction resolve the architecture and compatibility choices needed for
dispatch.
