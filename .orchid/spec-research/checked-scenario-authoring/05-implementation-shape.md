# Implementation shape: checked scenario authoring

## Ownership map

```text
src/types/scenario.rs
  Existing serde DTOs and their compatibility attributes
  Scenario-related DTO consuming-method must_use annotations

src/types/scenario_checked.rs                     (new)
  Scenario, ScenarioBuilder, ScenarioNode, ScenarioEdge
  NodeBehavior, ConnectionSpec, StateTarget
  Checked family/connection config types and DTO projection helpers

src/types/mod.rs
  Private module declaration and public checked-type re-exports

src/validation/mod.rs
  Sole ScenarioSpec -> Scenario whole-graph validation/conversion
  Sole Scenario -> ExecutionPlan assembly after spec 037

src/plan.rs                                      (created by spec 037)
  Checked CompiledNode/CompiledEdge projections and opaque plan queries

src/engine/mod.rs
  Direct consumption of checked behavior/connection/positive values

src/simulator.rs
  Preserved DTO compile facade plus compile_checked

src/lib.rs
src/prelude.rs
  Cohesive public re-exports and rustdoc examples

tests/checked_scenario_authoring.rs              (new)
tests/fixtures/scenario-wire-v1/legacy-default-resource.json  (new)
tests/fixtures/scenario-wire-v1/legacy-state-aliases.json      (new)
tests/fixtures/scenario-wire-v1/node-config-mismatch.json      (new)
tests/fixtures/scenario-wire-v1/map-key-id-mismatch.json       (new)
tests/readme_snippets.rs
README.md
  Public compatibility, migration, and documentation proof
```

`src/error.rs` remains unchanged unless the implementation cannot express a stable path through
the existing `SetupError` variants. A worker must escalate before adding a new public error enum or
variant.

## Public checked contracts

### `Scenario`

`Scenario` is immutable and does not derive serde. It owns:

- the canonical `ScenarioSpec` used to create it (preserved for wire round-trip and 037
  `source_spec()` compatibility);
- private checked `BTreeMap<NodeId, ScenarioNode>` and
  `BTreeMap<EdgeId, ScenarioEdge>` projections;
- a private crate-private `ValidatedExpressions` bundle containing deterministic
  `BTreeMap<EdgeId, CompiledExpr>` transfer and modifier-state maps.

Required API shape:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario { /* private fields */ }

impl Scenario {
    pub fn builder(id: ScenarioId) -> ScenarioBuilder;
    pub fn id(&self) -> &ScenarioId;
    pub fn source_spec(&self) -> &ScenarioSpec;
    pub fn nodes(&self) -> impl ExactSizeIterator<Item = (&NodeId, &ScenarioNode)>;
    pub fn edges(&self) -> impl ExactSizeIterator<Item = (&EdgeId, &ScenarioEdge)>;
    pub fn node(&self, id: &NodeId) -> Option<&ScenarioNode>;
    pub fn edge(&self, id: &EdgeId) -> Option<&ScenarioEdge>;
    pub fn into_spec(self) -> ScenarioSpec;
}

impl TryFrom<ScenarioSpec> for Scenario {
    type Error = SetupError;
}

impl From<Scenario> for ScenarioSpec;
impl From<&Scenario> for ScenarioSpec;
```

The iterator return types may use named iterator wrappers if MSRV/rustdoc requires them; workers
must not expose the backing maps mutably. There is no public expression-bundle or AST accessor.
`Scenario::into_spec` and both `From` implementations ignore/drop that bundle and return only the
retained source DTO.

### Node contract

`NodeBehavior` contains every current `NodeKind` family exactly as frozen in D002. Checked family
config structs have private fields, getters, `Default` when valid, DTO `TryFrom`/`From` helpers,
and `#[must_use]` consuming setters. `DelayConfig` stores `NonZeroU64`; `QueueConfig` stores a
`NonZeroU64` release and `Option<NonZeroU64>` capacity.

The checked configs preserve these exact wire fields and defaults:

| Checked type | Private checked fields | Default |
| --- | --- | --- |
| `PoolConfig` | `capacity: Option<u64>`, `allow_negative_start: bool`, `mode: NodeModeConfig` | wire default |
| `DrainConfig` | `mode: NodeModeConfig` | wire default |
| `SortingGateConfig` | `mode: NodeModeConfig` | wire default |
| `TriggerGateConfig` | `mode: NodeModeConfig` | wire default |
| `MixedGateConfig` | `mode: NodeModeConfig` | wire default |
| `ConverterConfig` | `ignore_disabled_inputs: bool`, `mode: NodeModeConfig` | wire default |
| `TraderConfig` | `ignore_disabled_inputs: bool`, `mode: NodeModeConfig` | wire default |
| `RegisterConfig` | `interactive: bool`, `min_value: Option<i64>`, `max_value: Option<i64>` | wire default |
| `DelayConfig` | `delay_steps: NonZeroU64`, `mode: NodeModeConfig` | one step, default mode |
| `QueueConfig` | `capacity: Option<NonZeroU64>`, `release_per_step: NonZeroU64`, `mode: NodeModeConfig` | no capacity, one release, default mode |

Every checked config implements `Default` with the table values. These signatures are normative;
all `with_*`/`without_*` methods are `#[must_use]` and return `Self`:

```rust
impl PoolConfig {
    pub fn capacity(&self) -> Option<u64>;
    pub fn allow_negative_start(&self) -> bool;
    pub fn mode(&self) -> &NodeModeConfig;
    pub fn with_capacity(self, capacity: u64) -> Self;
    pub fn without_capacity(self) -> Self;
    pub fn with_allow_negative_start(self, allow: bool) -> Self;
    pub fn with_mode(self, mode: NodeModeConfig) -> Self;
}

impl DrainConfig { pub fn mode(&self) -> &NodeModeConfig; pub fn with_mode(self, mode: NodeModeConfig) -> Self; }
impl SortingGateConfig { pub fn mode(&self) -> &NodeModeConfig; pub fn with_mode(self, mode: NodeModeConfig) -> Self; }
impl TriggerGateConfig { pub fn mode(&self) -> &NodeModeConfig; pub fn with_mode(self, mode: NodeModeConfig) -> Self; }
impl MixedGateConfig { pub fn mode(&self) -> &NodeModeConfig; pub fn with_mode(self, mode: NodeModeConfig) -> Self; }

impl ConverterConfig {
    pub fn ignore_disabled_inputs(&self) -> bool;
    pub fn mode(&self) -> &NodeModeConfig;
    pub fn with_ignore_disabled_inputs(self, ignore: bool) -> Self;
    pub fn with_mode(self, mode: NodeModeConfig) -> Self;
}

impl TraderConfig {
    pub fn ignore_disabled_inputs(&self) -> bool;
    pub fn mode(&self) -> &NodeModeConfig;
    pub fn with_ignore_disabled_inputs(self, ignore: bool) -> Self;
    pub fn with_mode(self, mode: NodeModeConfig) -> Self;
}

impl RegisterConfig {
    pub fn interactive(&self) -> bool;
    pub fn min_value(&self) -> Option<i64>;
    pub fn max_value(&self) -> Option<i64>;
    pub fn with_interactive(self, interactive: bool) -> Self;
    pub fn with_min_value(self, min: i64) -> Self;
    pub fn without_min_value(self) -> Self;
    pub fn with_max_value(self, max: i64) -> Self;
    pub fn without_max_value(self) -> Self;
}

impl DelayConfig {
    pub fn delay_steps(&self) -> NonZeroU64;
    pub fn mode(&self) -> &NodeModeConfig;
    pub fn with_delay_steps(self, steps: NonZeroU64) -> Self;
    pub fn with_mode(self, mode: NodeModeConfig) -> Self;
}

impl QueueConfig {
    pub fn capacity(&self) -> Option<NonZeroU64>;
    pub fn release_per_step(&self) -> NonZeroU64;
    pub fn mode(&self) -> &NodeModeConfig;
    pub fn with_capacity(self, capacity: NonZeroU64) -> Self;
    pub fn without_capacity(self) -> Self;
    pub fn with_release_per_step(self, release: NonZeroU64) -> Self;
    pub fn with_mode(self, mode: NodeModeConfig) -> Self;
}
```

Conversion from legacy integer DTO fields is the only fallible config construction path. Public
checked construction uses `Default` plus typed setters, so no `try_*` or macro-only helper is added.

`ScenarioNode` has private ID, behavior, label, initial value, tags, and metadata. These public
constructor and common-setter signatures are normative:

```rust
impl ScenarioNode {
    pub fn source(id: NodeId) -> Self;
    pub fn pool(id: NodeId, config: PoolConfig) -> Self;
    pub fn drain(id: NodeId, config: DrainConfig) -> Self;
    pub fn sorting_gate(id: NodeId, config: SortingGateConfig) -> Self;
    pub fn trigger_gate(id: NodeId, config: TriggerGateConfig) -> Self;
    pub fn mixed_gate(id: NodeId, config: MixedGateConfig) -> Self;
    pub fn converter(id: NodeId, config: ConverterConfig) -> Self;
    pub fn trader(id: NodeId, config: TraderConfig) -> Self;
    pub fn register(id: NodeId, config: RegisterConfig) -> Self;
    pub fn delay(id: NodeId, config: DelayConfig) -> Self;
    pub fn queue(id: NodeId, config: QueueConfig) -> Self;
    pub fn process(id: NodeId) -> Self;
    pub fn sink(id: NodeId) -> Self;
    pub fn gate(id: NodeId) -> Self;
    pub fn custom(id: NodeId, family: impl Into<String>) -> Self;

    pub fn id(&self) -> &NodeId;
    pub fn behavior(&self) -> &NodeBehavior;
    pub fn label(&self) -> Option<&str>;
    pub fn initial_value(&self) -> f64;
    pub fn tags(&self) -> &BTreeSet<String>;
    pub fn metadata(&self) -> &BTreeMap<String, String>;

    pub fn with_label(self, label: impl Into<String>) -> Self;
    pub fn with_initial_value(self, initial_value: f64) -> Self;
    pub fn with_tag(self, tag: impl Into<String>) -> Self;
    pub fn with_metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self;
}
```

All four common setters are `#[must_use]`. `with_tag` inserts into the deterministic set;
`with_metadata` inserts/replaces that metadata key exactly like the wire map.

Constructors bind family and config in one call. There is no constructor accepting separate
`NodeKind` plus `NodeConfig`.

### Edge/connection contract

`ResourceConnection` and `StateConnection` use these exact public signatures. `Default` mirrors the
wire defaults (token size one; Modifier, `+1`, Node, no filter). Every consuming setter is
`#[must_use]`:

```rust
impl ResourceConnection {
    pub fn token_size(&self) -> NonZeroU64;
    pub fn with_token_size(self, token_size: NonZeroU64) -> Self;
}

impl StateConnection {
    pub fn new(
        role: StateConnectionRole,
        formula: impl Into<String>,
        target: StateTarget,
    ) -> Self;
    pub fn role(&self) -> &StateConnectionRole;
    pub fn formula(&self) -> &str;
    pub fn target(&self) -> &StateTarget;
    pub fn resource_filter(&self) -> Option<&str>;
    pub fn with_role(self, role: StateConnectionRole) -> Self;
    pub fn with_formula(self, formula: impl Into<String>) -> Self;
    pub fn with_target(self, target: StateTarget) -> Self;
    pub fn with_resource_filter(self, filter: impl Into<String>) -> Self;
}
```

`ResourceConnection::default()` and `StateConnection::default()` are the ordinary trait-generated
constructors. `StateTarget` owns the edge ID for every non-node variant. No separate optional target
ID setter exists.

`ScenarioEdge` has private ID/from/to/transfer/connection/enabled/metadata and provides:

```rust
pub fn resource(
    id: EdgeId,
    from: NodeId,
    to: NodeId,
    transfer: TransferSpec,
    connection: ResourceConnection,
) -> Self;

pub fn state(
    id: EdgeId,
    from: NodeId,
    to: NodeId,
    transfer: TransferSpec,
    connection: StateConnection,
) -> Self;

pub fn id(&self) -> &EdgeId;
pub fn from(&self) -> &NodeId;
pub fn to(&self) -> &NodeId;
pub fn transfer(&self) -> &TransferSpec;
pub fn connection(&self) -> &ConnectionSpec;
pub fn enabled(&self) -> bool;
pub fn metadata(&self) -> &BTreeMap<String, String>;

pub fn with_enabled(self, enabled: bool) -> Self;
pub fn with_metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self;
```

The state constructor retains `TransferSpec` because the stable DTO requires the field. Existing
semantic validation decides which transfer variants are meaningful; this spec does not change the
wire field.

Both common edge setters are `#[must_use]`. `with_metadata` inserts/replaces one deterministic map
entry. These checked APIs are the only supported desugaring targets for spec 040; do not add
macro-only constructors, setters, or private-field access.

### `ScenarioBuilder`

The builder stores one private canonical `ScenarioSpec`, preserving its deterministic containers
and defaults. Required shape:

```rust
#[must_use = "a ScenarioBuilder must be built or its configured scenario is discarded"]
pub struct ScenarioBuilder { /* private ScenarioSpec */ }

impl ScenarioBuilder {
    pub fn new(id: ScenarioId) -> Self;
    pub fn insert_node(&mut self, node: ScenarioNode) -> Result<(), SetupError>;
    pub fn insert_edge(&mut self, edge: ScenarioEdge) -> Result<(), SetupError>;

    #[must_use = "use the returned builder to retain the inserted node"]
    pub fn with_node(self, node: ScenarioNode) -> Result<Self, SetupError>;
    #[must_use = "use the returned builder to retain the inserted edge"]
    pub fn with_edge(self, edge: ScenarioEdge) -> Result<Self, SetupError>;

    pub fn with_title(self, title: impl Into<String>) -> Self;
    pub fn with_description(self, description: impl Into<String>) -> Self;
    pub fn with_tag(self, tag: impl Into<String>) -> Self;
    pub fn with_variables(self, variables: VariableRuntimeConfig) -> Self;
    pub fn with_end_condition(self, condition: EndConditionSpec) -> Self;
    pub fn with_end_conditions<I>(self, conditions: I) -> Self
    where I: IntoIterator<Item = EndConditionSpec>;
    pub fn push_end_condition(self, condition: EndConditionSpec) -> Self;
    pub fn with_tracked_metric(self, metric: MetricKey) -> Self;
    pub fn with_metadata(self, key: impl Into<String>, value: impl Into<String>) -> Self;
    pub fn build(self) -> Result<Scenario, SetupError>;
}
```

`insert_node` and `insert_edge` convert the checked value to its DTO once and use `BTreeMap::entry`.
An occupied entry returns a stable `InvalidParameter`; no mutation occurs. Consuming variants call
the same insert functions. `build` calls `Scenario::try_from` on the stored DTO and never owns an
independent validation implementation or performs a preliminary formula parse.

## DTO conversion rules

### Conversion order

`src/validation/mod.rs` adds one crate-private conversion entrypoint used by the public trait impl:

```rust
fn checked_scenario_from_spec(spec: ScenarioSpec) -> Result<Scenario, SetupError>;
```

It executes in this fixed order:

1. compare each node map key to `NodeSpec.id`;
2. compare each edge map key to `EdgeSpec.id`;
3. convert node DTOs to `ScenarioNode` using D003's complete matrix;
4. convert edge DTOs to `ScenarioEdge`, collapsing kind/payload and target/ID;
5. run the existing endpoint, end-condition, metric, cycle, connection, node, formula, and variable
   validation semantics against the checked view; formula validation returns `CompiledExpr` rather
   than discarding it;
6. store every returned AST in a crate-private `ValidatedExpressions` bundle keyed by `EdgeId`;
7. construct `Scenario` through a crate-private constructor unavailable to downstream callers.

Map iteration is lexicographic and the first error is stable. Exact key mismatch paths are
`nodes.<key>.id` and `edges.<key>.id`. Exact config mismatch path is `nodes.<key>.config`.

### Compatibility normalization

- `NodeConfig::None` becomes the default checked config only when the selected `NodeKind` owns a
  config family; explicit wrong-family variants fail.
- A resource DTO connection becomes `ConnectionSpec::Resource` only when its state payload is the
  current default; a state DTO connection becomes `ConnectionSpec::State` only when its resource
  payload is the current default.
- `StateConnectionTarget::Node` requires `target_connection = None` and becomes
  `StateTarget::Node`; every other target requires `Some(id)` and moves the ID into its variant.
- DTO positive integers convert with `NonZeroU64::new`; zero retains the existing error path and
  reason.
- The parsed `ScenarioSpec` is retained unchanged after successful conversion. Compatibility is
  measured as `to_value(&parsed_scenario_spec) == to_value(ScenarioSpec::from(&checked))`; raw
  fixture spelling is not a baseline because serde aliases and omitted defaults may normalize
  during the initial deserialize/serialize cycle.
- Builder-created checked values synthesize DTOs using the current canonical field names and
  default-resource omission rules.

### Validated expression bundle

`src/types/scenario_checked.rs` owns this crate-private, non-serde carrier:

```rust
pub(crate) struct ValidatedExpressions {
    transfer_by_edge: BTreeMap<EdgeId, CompiledExpr>,
    state_by_edge: BTreeMap<EdgeId, CompiledExpr>,
}
```

`checked_scenario_from_spec` creates one `ExprRuntime` and performs the only parse. It retains:

- one transfer AST for every resource connection with `TransferSpec::Expression`, regardless of
  `enabled`, in edge-key order;
- one state AST for every modifier state connection, regardless of `StateTarget` or `enabled`, in
  edge-key order.

It does not parse inactive formula-shaped wire fields: transfer expressions on state connections,
or Trigger/Activator/Filter state formulas including the `*` control literal. Those keep their
existing blank/role/target checks. `MetricScaled`, variable sources, and end conditions add no AST
entries. Unit tests cover every included and excluded formula kind plus invalid syntax/error paths.

## Plan and engine data flow

Spec 037 is a hard prerequisite. Before the plan migration edits anything, its worker reads
`specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and requires `status = "done"` plus
`verification_status = "passed"`. Queue ordering uses the spec dependency and a
`before-implementation` checkpoint because the current task validator rejects cross-spec task
edges. After the guard succeeds, implement this flow:

```text
legacy JSON / caller DTO
        |
        v
  ScenarioSpec --TryFrom/check+parse once--> Scenario
        |                            ^
        |                            |
        |                     ScenarioBuilder
        v                            |
Simulator::compile -----------------+ compile_checked
        |
        v
src/validation/mod.rs assembles ExecutionPlan
        |
        +-- source DTO retained for CompiledScenario::source_spec()
        +-- CompiledNode owns NodeBehavior
        +-- CompiledEdge owns ConnectionSpec + compiled transfer data
        +-- ValidatedExpressions moved into CompiledExpressions slots
        v
engine matches checked variants only
```

Concrete migration points:

- `src/simulator.rs`: preserve `compile(ScenarioSpec)` and add
  `compile_checked(Scenario) -> Result<CompiledScenario, SetupError>`; both converge before plan
  assembly. Add `TryFrom<Scenario> for CompiledScenario` in `src/plan.rs`.
- `src/validation/mod.rs`: keep sole plan assembly ownership and accept the checked `Scenario`.
  Preserve the parsed source DTO in the plan while populating node/edge execution projections
  from checked types. Destructure the owned scenario and move its validated-expression maps into
  the plan; do not call `ExprRuntime::compile` during plan assembly.
- `src/plan.rs`: make `CompiledNode` carry `NodeBehavior`; make `CompiledEdge` carry
  `ConnectionSpec`; store fraction denominators and resource token sizes as `NonZeroU64` in the
  relevant compiled projection. Convert the moved edge-ID-keyed AST maps into 037's edge-index
  aligned optional transfer/state slots, fail internally if a required checked AST is missing, and
  assert no unexpected bundle entry remains. Do not change 037's public accessors or expose ASTs.
- `src/engine/mod.rs`: replace DTO tag/payload matches with checked sums. Remove
  `delay_steps_for_node`, `queue_release_per_step_for_node`, the wrong-family branches in
  `node_capacity_for_node`/`node_mode_for_node`, optional-ID handling in
  `trigger_targets_for_state_connection`, the zero fraction-denominator branch, and token-size
  `.max(1)`. Equivalent helpers may remain only if their inputs are checked types and their
  signatures cannot express fallback.
- Routing to referenced resource/state connections uses 037's precomputed plan/routing indexes;
  execution must not rediscover target validity from DTO maps.

No engine function may accept `NodeSpec`, `EdgeSpec`, `NodeConfig`, `ConnectionKind`, or
`StateConnectionTarget` after the migration, except code that emits or exposes the preserved source
DTO without making execution decisions.

## `must_use` surface

Add a custom `#[must_use]` message to:

- `ScenarioBuilder`;
- every consuming `with_*` method on `ScenarioBuilder`, `ScenarioNode`, `ScenarioEdge`, checked
  config types, and `StateConnection`;
- existing consuming scenario-authoring methods on `NodeSpec`, `EdgeSpec`, and `ScenarioSpec` in
  `src/types/scenario.rs`.

Do not broaden this task to run/batch/report/artifact builders. Add rustdoc `compile_fail` examples
using `#![deny(unused_must_use)]` for one checked builder method and one retained DTO method.

## Test seam map

### Unit tests

In `src/types/scenario_checked.rs`:

- exhaustive table tests for all 15 `NodeBehavior` outcomes and every accepted `None`/matching
  config combination;
- wrong-family tests covering each configured family at least once and all configless families;
- all `ConnectionSpec` and `StateTarget` conversion combinations;
- nonzero success/failure for delay, queue release/capacity, resource token size, and fraction
  denominator projection;
- checked-to-DTO conversion and equality with the serialized parsed DTO baseline;
- expression-bundle coverage for resource-transfer expressions, modifier state expressions across
  all targets, disabled edges, inactive state-edge transfer expressions, nonmodifier control
  formulas, invalid syntax, and formula error paths;
- duplicate node and edge insertion leaves the first stored DTO unchanged;
- both mutable and consuming builder styles produce the same checked scenario.

In `src/validation/mod.rs` and `src/plan.rs`:

- node key mismatch precedes edge/key and later graph errors;
- edge key mismatch uses the map key path and names embedded ID;
- mismatched kind/config fails instead of receiving defaults;
- plan projections contain the expected behavior/connection variants and positive values;
- plan source DTO remains equal to the parsed input DTO;
- each retained edge-ID AST moves into the matching edge-index slot, all bundle entries are
  consumed, and plan/simulator/engine paths contain no formula parse.

In `src/engine/mod.rs`:

- existing delay, queue, capacity, state-target, resource quantization, and fraction tests operate
  through checked projections;
- no test constructs `CompiledScenario` or invalid plan fields directly after 037/039.

### Integration and fixture tests

`tests/checked_scenario_authoring.rs` must:

- load each exact fixture path listed in the ownership map;
- prove the two valid legacy fixtures preserve semantic JSON by comparing
  `serde_json::to_value(&parsed_scenario_spec)` with
  `serde_json::to_value(ScenarioSpec::from(&checked))`, never with raw fixture `Value`;
- prove mismatch fixtures deserialize successfully as DTOs but fail `Scenario::try_from` and
  `Simulator::compile` at exact paths;
- build an equivalent scenario with mutable and consuming builder styles;
- compare `RunReport` equality for DTO compile and checked compile under an explicit seed;
- prove `ScenarioSpec::with_node/with_edge` still replaces while `ScenarioBuilder` rejects;
- prove `TryFrom<ScenarioSpec> for CompiledScenario` and `TryFrom<Scenario>` converge with the
  facade.

`tests/readme_snippets.rs` must pin the new recommended snippet and use 037 read-only compiled
accessors.

### Documentation tests

- One normal doctest shows full checked authoring through compile and run.
- Two `compile_fail` doctests deny `unused_must_use` and discard a consuming result.
- README migration examples compile as tests, not prose-only snippets.

## Compatibility and rollout

- This is additive for valid DTO construction and wire shape. Existing JSON parsing remains
  `serde_json::from_*::<ScenarioSpec>`.
- `Simulator::compile(ScenarioSpec)` remains the compatibility entrypoint.
- Explicit kind/config mismatch and key/ID drift become newly rejected setup errors. Document this
  intentional strictness under the 0.2 migration notes.
- Existing replace-on-duplicate DTO helper behavior remains; only `ScenarioBuilder` rejects.
- New public enums start `#[non_exhaustive]`; no `#[non_exhaustive]` is retrofitted onto existing
  wire enums in this spec.
- No persistence backfill is needed because stored DTOs are not rewritten.
- Spec 040 may begin only after the final public checked-builder task from this spec is complete;
  its macro must expand to this API and must not access private checked fields.

## Vertical slices and dependencies

### Slice 1 — Checked domain and conversions

Own `src/types/scenario_checked.rs`, its unit tests, and re-exports. Add all complete sums/configs,
private checked values, DTO projections, local nonzero conversion, and the private deterministic
validated-expression bundle. The shared conversion must parse every active formula exactly once
and retain the returned AST. No partial family or formula-kind set may land as completion state.

### Slice 2 — Checked builder and deterministic duplicate policy

Depends on slice 1. Add the whole builder surface, duplicate behavior, all scenario metadata/end
condition methods, and related `must_use` annotations including the existing DTO scenario methods.

### Slice 3 — Plan/facade/engine conversion

Depends on slices 1-2 and logically requires completed, verified 037/T004 through the explicit
guard above. Migrate
`src/validation/mod.rs`, `src/plan.rs`, `src/simulator.rs`, and `src/engine/mod.rs` end to end.
Completion requires removing all named runtime fallbacks, not merely adding checked types beside
the old path. It also requires moving T001's AST bundle into 037 `CompiledExpressions` without a
second parse, clone of whole plan state, or public AST exposure.

### Slice 4 — Wire and behavior compatibility suite

Depends on slice 3. Add all four exact JSON fixtures and `tests/checked_scenario_authoring.rs`.
Cover valid legacy round trips and invalid semantic conversion separately.

### Slice 5 — Docs, re-exports, and migration

Depends on slices 3-4. Update `src/lib.rs`, `src/prelude.rs`, `README.md`, and
`tests/readme_snippets.rs`; add rustdoc examples and explicit migration semantics. Do not remove the
DTO path.

### Slice 6 — Independent contract review

Depends on all implementation slices. A fresh Sol/high reviewer runs machine gates, audits public
API and serde compatibility, verifies no unchecked engine path remains, and either fixes narrow
issues within spec scope or rejects completion with concrete findings.

## Validation commands

Run after focused tests, in this order:

```bash
cargo fmt --all -- --check
cargo test --lib types::
cargo test --lib validation::
cargo test --lib engine::
cargo test --lib expr::
cargo test --test checked_scenario_authoring
cargo test --test readme_snippets
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

No Docker-hosted or network service is involved.

## Constraints and anti-goals

- Preserve BTree ordering and existing valid deterministic reports.
- Preserve all serde names, aliases, defaults, and omission rules.
- Preserve 037 facade signatures/accessors and module privacy.
- Do not add typestate, derive-builder, `serde_path_to_error`, macro implementation, or mutable
  accessors on checked scenario values.
- Do not leave a duplicate execution path that reads DTO tag/payload combinations at runtime.
- Do not change run/batch/capture/report/artifact public APIs.
- Do not call this complete with only four node variants, only local validation, or docs without
  engine migration.

## Escalation and stop conditions

Stop and escalate if:

- 037 changes `T004`, `src/plan.rs` ownership, facade signatures, or required accessors;
- preserving exact legacy serde value shape requires changing an existing serde attribute;
- a currently valid parity fixture would change its `RunReport` rather than merely fail an invalid
  setup earlier;
- `SetupError` cannot express deterministic duplicate/mismatch paths without a new public variant;
- an engine consumer must retain independent DTO kind/payload decisions after plan assembly;
- spec 040 requests private-field or raw-validation access instead of the checked public builder.
