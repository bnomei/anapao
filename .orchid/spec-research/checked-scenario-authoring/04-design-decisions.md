# Design decisions: checked scenario authoring

## D001 — Preserve wire DTOs; add a separate checked domain

Decision: `ScenarioSpec`, `NodeSpec`, `EdgeSpec`, `NodeKind`, `NodeConfig`,
`EdgeConnectionConfig`, `ConnectionKind`, `StateConnectionTarget`, and all existing serde config
structs remain the wire/document layer with their current names, public fields, serde attributes,
defaults, aliases, and output shape. New checked types do not derive serde. `ScenarioSpec` converts
fallibly into a new immutable public `Scenario`, and `Scenario` converts infallibly back into a
`ScenarioSpec`.

The compatibility baseline is the parsed DTO, not raw JSON spelling:

```rust
serde_json::to_value(&parsed_scenario_spec)?
    == serde_json::to_value(ScenarioSpec::from(&checked))?
```

Serde may canonicalize `target_edge` to `target_connection`, `filter` to `resource_filter`, and
omitted defaults during the initial parse/serialize cycle. Checked conversion must not introduce
additional DTO semantic drift, but it does not preserve raw lexical alias/default spelling.

Rationale: wire compatibility and execution invariants have different evolution constraints.
Separating them allows stricter compile-time semantics without changing legacy JSON or requiring
serde attributes on the execution model.

Rejected:

- Replacing `ScenarioSpec` fields with sums: changes public Rust construction and JSON shape.
- Custom-deserializing `ScenarioSpec` directly into a valid graph: moves compile-time graph errors
  into serde, changes error timing, and prevents callers from inspecting or repairing raw specs.
- Deriving serde on the checked domain: creates a second wire contract and risks drift.

## D002 — Use complete public sums, not illustrative partial enums

Decision: introduce `ScenarioNode`, `ScenarioEdge`, and these non-exhaustive public sums:

```rust
#[non_exhaustive]
pub enum NodeBehavior {
    Source,
    Pool(PoolConfig),
    Drain(DrainConfig),
    SortingGate(SortingGateConfig),
    TriggerGate(TriggerGateConfig),
    MixedGate(MixedGateConfig),
    Converter(ConverterConfig),
    Trader(TraderConfig),
    Register(RegisterConfig),
    Delay(DelayConfig),
    Queue(QueueConfig),
    Process,
    Sink,
    Gate,
    Custom(String),
}

#[non_exhaustive]
pub enum ConnectionSpec {
    Resource(ResourceConnection),
    State(StateConnection),
}

#[non_exhaustive]
pub enum StateTarget {
    Node,
    ResourceConnection(EdgeId),
    StateConnection(EdgeId),
    Formula(EdgeId),
}
```

Each checked config is a public type with private fields, read-only accessors, `Default` where the
wire DTO has a valid default, and consuming `with_*` configuration methods. `ScenarioNode` and
`ScenarioEdge` have private fields and family-specific constructors for every variant.

Public spellings are frozen, not worker-selected:

- config types use `Default` plus field-named consuming setters; optional capacity/min/max values
  use `with_*` to set and `without_*` to clear, while positive queue/delay values accept
  `NonZeroU64`;
- `ScenarioNode::{source,pool,drain,sorting_gate,trigger_gate,mixed_gate,converter,trader,register,
  delay,queue,process,sink,gate,custom}` are the family constructors;
- common node setters are exactly `with_label`, `with_initial_value`, `with_tag`, and
  `with_metadata`;
- `ResourceConnection::default().with_token_size(NonZeroU64)` is the configurable resource path;
- `StateConnection::{default,new}` construct state semantics, and its setters are exactly
  `with_role`, `with_formula`, `with_target`, and `with_resource_filter`;
- `ScenarioEdge::{resource,state}` are the edge constructors, with common `with_enabled` and
  `with_metadata` setters;
- spec 040 expands only through these public methods and receives no macro-only helper.

The exact receiver, parameter, return, getter, and clearing signatures are normative in
`05-implementation-shape.md`.

Rationale: the variant owns its corresponding payload, and an ID exists only for state targets
that require one. `#[non_exhaustive]` is applied from the first release so later variants need not
break downstream exhaustive matches.

Rejected:

- Covering only Source/Pool/Delay/Queue: that would leave the same invalid-state problem for the
  remaining families and would be an MVP.
- Reusing public wire config structs as the checked payloads: their public integer fields can still
  express locally invalid checked values.
- Typestate per node/connection family: field-setting order is not itself an invariant and the
  generic state space would dominate a data-oriented API.

## D003 — Normalize only an omitted matching config; reject an explicit mismatch

Decision: DTO-to-domain conversion applies this matrix in deterministic node-key order:

| `NodeKind` | Accepted `NodeConfig` | Checked variant |
| --- | --- | --- |
| Source | `None` | `Source` |
| Pool | `None` or `Pool` | `Pool(PoolConfig)`; `None` uses default |
| Drain | `None` or `Drain` | `Drain(DrainConfig)`; `None` uses default |
| SortingGate | `None` or `SortingGate` | matching checked config; `None` uses default |
| TriggerGate | `None` or `TriggerGate` | matching checked config; `None` uses default |
| MixedGate | `None` or `MixedGate` | matching checked config; `None` uses default |
| Converter | `None` or `Converter` | matching checked config; `None` uses default |
| Trader | `None` or `Trader` | matching checked config; `None` uses default |
| Register | `None` or `Register` | matching checked config; `None` uses default |
| Delay | `None` or `Delay` | `Delay(DelayConfig)`; `None` uses default |
| Queue | `None` or `Queue` | `Queue(QueueConfig)`; `None` uses default |
| Process | `None` | `Process` |
| Sink | `None` | `Sink` |
| Gate | `None` | `Gate` |
| Custom | `None` | `Custom` |

Any other explicit config is an `InvalidParameter` at `nodes.<map-key>.config`, naming both
families. Omitted config remains compatible with the current default behavior; an explicit wrong
payload is never silently discarded.

## D004 — Encode positive checked values with `NonZeroU64`

Decision: checked values use `NonZeroU64` for:

- `DelayConfig.delay_steps`;
- `QueueConfig.release_per_step`;
- `QueueConfig.capacity` when present, because current validation rejects configured zero;
- `ResourceConnection.token_size`;
- the crate-private checked/compiled fraction denominator used by the execution plan.

DTO fields remain `u64`/`Option<u64>`. Conversion uses `NonZeroU64::new` and reports the existing
path with “must be greater than 0.” Domain-to-DTO conversion calls `.get()`. Defaults use one.
`PoolConfig.capacity` remains `Option<u64>` because current behavior permits a zero-capacity pool.
`EndConditionSpec::MaxSteps { steps: 0 }` remains representable because current scenario validation
does not reject it. The work does not silently broaden zero rejection beyond established
invariants.

Rejected:

- Changing DTO field types to `NonZeroU64`: unnecessary wire/Rust API churn.
- Calling `.max(1)` in the checked conversion: repairs caller mistakes silently.
- Converting every scenario integer to nonzero without current semantic evidence.

## D005 — One conventional builder with two ergonomic styles

Decision: `ScenarioBuilder` owns private deterministic `BTreeMap`/`BTreeSet` state and starts with
the same scenario defaults as `ScenarioSpec::new`. It supports:

- mutation-style `insert_node(&mut self, ScenarioNode) -> Result<(), SetupError>` and
  `insert_edge(&mut self, ScenarioEdge) -> Result<(), SetupError>`;
- consuming `with_node(self, ScenarioNode) -> Result<Self, SetupError>` and
  `with_edge(self, ScenarioEdge) -> Result<Self, SetupError>` conveniences;
- consuming setters for title, description, tags, variables, end conditions, tracked metrics, and
  metadata;
- `build(self) -> Result<Scenario, SetupError>` as the single whole-graph validation gate.

The builder is `#[must_use]`. Every consuming checked or DTO scenario-authoring `with_*` method is
also `#[must_use = "..."]`. Mutation-style insertion follows `std::process::Command`-like builder
ergonomics while consuming methods preserve the project's existing chaining style.

Rejected:

- Only mutation-style methods: unnecessary migration friction for current Anapao users.
- Only consuming methods: less convenient for loops and conditional authoring.
- Generated/derive builders: adds dependency and does not solve graph-specific validation.

## D006 — Duplicate insertions are errors and never replace

Decision: checked node and edge insertion uses `BTreeMap::entry`. `Occupied` returns
`SetupError::InvalidParameter` at `nodes.<id>` or `edges.<id>` with a stable duplicate-ID reason.
The first value remains unchanged. Consuming `with_*` returns `Err`; mutation-style callers can
continue using the unchanged builder after handling the error. The legacy DTO helpers retain their
documented replace behavior for compatibility.

JSON object keys that repeat lexically are outside this guarantee because serde may have already
collapsed them before a `ScenarioSpec` exists. This limitation is documented rather than implied
to be solved.

Rejected:

- Last-write-wins on the checked builder: repeats the current silent mistake.
- First-write-wins success: hides the second caller error.
- Changing `ScenarioSpec::with_node/with_edge`: breaks existing documented behavior.

## D007 — Validate key/ID equality before every other graph pass

Decision: the shared `ScenarioSpec -> Scenario` conversion first iterates nodes, then edges, in
`BTreeMap` order and requires `map_key == embedded.id`. Errors use
`nodes.<key>.id`/`edges.<key>.id` and name the embedded value. Only after those checks succeed may
the converter normalize local sums and run references, cycles, formulas, modes, variable, and end
condition validation. The builder cannot produce a key/ID mismatch because it derives keys from
private checked values.

Rationale: all later error paths and deterministic indexes need one canonical ID.

Rejected:

- Rewriting the embedded ID to match the key: silently mutates the document.
- Re-keying the map from embedded IDs: can collapse two definitions and changes deterministic
  ordering.
- Deferring the check until execution-plan assembly: earlier validation could report paths against
  a different identity.

## D008 — Use one validation implementation and the existing error taxonomy

Decision: `TryFrom<ScenarioSpec> for Scenario` and `ScenarioBuilder::build` converge on one
crate-private validation/conversion pipeline. `ScenarioBuilder::build` materializes the stable DTO
once, then delegates through that same conversion. `SetupError` remains the public error type and
existing path grammar is retained. No `serde_path_to_error` dependency is added because serde
syntax/type errors and post-deserialize semantic errors remain distinct operations.

The public opaque `Scenario` also privately owns this crate-private carrier:

```rust
pub(crate) struct ValidatedExpressions {
    transfer_by_edge: BTreeMap<EdgeId, CompiledExpr>,
    state_by_edge: BTreeMap<EdgeId, CompiledExpr>,
}
```

The shared conversion creates one `ExprRuntime`, parses each active formula exactly once in
deterministic edge-key order, and retains the returned AST in this bundle rather than mapping it to
`()`. “Active formula” is exact:

- every resource connection whose `TransferSpec` is `Expression`, including a disabled edge,
  because current validation checks it regardless of `enabled`;
- every state connection whose role is `Modifier`, for every `StateTarget`, because current
  validation parses modifier formulas regardless of target or `enabled`.

State Trigger/Activator/Filter formula strings and the `*` control literal are checked only by their
existing role/blank/target rules and are not parsed expression ASTs. A transfer expression attached
to a state connection is an inactive wire field and is not parsed. `MetricScaled`, variable source,
and end-condition variants contain no expression AST. Tests enumerate these included and excluded
formula kinds so “parse exactly once” cannot be interpreted narrowly.

`ValidatedExpressions` and `CompiledExpr` remain crate-private, have no public accessor, and do not
participate in serde or DTO equality. Inverse DTO projection ignores/drops the private bundle and
returns the retained source DTO.

Rationale: one gate prevents drift between hand-authored DTOs, checked builder output, and the
compile facade. The current error taxonomy already exposes path-rich parameter and graph failures.
Retaining the AST returned by validation preserves spec 037's parse-once guarantee across the new
intermediate `Scenario` boundary.

Rejected:

- Parallel validators for DTO and builder inputs.
- A new overlapping `ScenarioBuildError` tree without a demonstrated matchability requirement.
- Validating only in `Simulator::compile`: a successful `ScenarioBuilder::build` must itself mean
  checked.

## D009 — Sequence plan/engine migration after spec 037

Decision: spec 039 depends on `037-compiled-scenario-trust-boundary` and pauses at
`human_checkpoint = "before-implementation"`. Its plan migration task must inspect
`specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and stop unless it is `done` with
`verification_status = "passed"`. The current task-schema validator rejects a cross-spec task edge
as unknown even when the sibling exists, so the explicit runtime guard is the supported fallback.
The implementation preserves the 037 public facade and privacy contract.

After checked types exist:

1. `Simulator::compile(ScenarioSpec)` performs `Scenario::try_from` and delegates to a new
   `Simulator::compile_checked(Scenario)` path.
2. `TryFrom<ScenarioSpec> for CompiledScenario` continues to delegate to the facade.
3. `src/validation/mod.rs` remains the sole `ExecutionPlan` assembler.
4. `src/plan.rs` stores the parsed/canonical source DTO for `source_spec()` and checked
   node/edge projections for execution.
5. `CompiledNode` owns `NodeBehavior`; `CompiledEdge` owns `ConnectionSpec` plus the 037 compiled
   transfer/expression data.
6. `Simulator::compile_checked` consumes `Scenario`, moves its `ValidatedExpressions` maps into
   edge-index-aligned 037 `CompiledExpressions` slots, and never reparses formula text.
7. Engine branches directly on the checked sums and positive values. Wrong-family defaults,
   missing-target empty lists, zero denominator guards, and token `.max(1)` defenses are removed.

`Simulator::compile_checked(Scenario) -> Result<CompiledScenario, SetupError>` is public so builder
users do not need to convert back to a DTO. A `TryFrom<Scenario> for CompiledScenario` convenience
delegates to it. Returning `Result` preserves one stable facade shape even though the checked
scenario has already passed semantic validation and plan assembly may currently be infallible.

Rejected:

- Landing before 037 and then rewriting the same compile/engine boundary twice.
- Re-publicizing raw validation or engine functions.
- Round-tripping builder output through serde.
- Removing `source_spec()` or changing its established DTO return contract.

## D010 — Preserve valid behavior; make invalid behavior strictly fail earlier

Decision: already-valid DTO scenarios must produce the same deterministic reports through
`Simulator::compile` and `Simulator::compile_checked`. Previously accepted explicit mismatches,
key/ID drift, and silent duplicate checked-builder insertions become errors. Error ordering is:

1. node key/ID mismatch;
2. edge key/ID mismatch;
3. local node conversion in node-key order;
4. local edge/connection conversion in edge-key order;
5. existing graph/reference/cycle/formula/variable/end-condition passes in their documented order.

This ordering is pinned in tests so deterministic diagnostics do not drift.

## D011 — Test the contract at four levels

Decision:

1. Unit tests in `src/types/scenario_checked.rs` cover every node mapping, connection/target mapping,
   config default, nonzero conversion, inverse DTO conversion, and private expression-bundle
   inclusion/exclusion.
2. Validation/plan tests cover key/ID mismatch ordering, duplicate behavior, and the absence of
   fallback execution projections.
3. `tests/checked_scenario_authoring.rs` covers public builder ergonomics, parsed-DTO semantic JSON
   equality, identical reports for a legacy DTO versus its checked equivalent, and compile-path
   formula parity.
4. Frozen JSON fixtures pin omitted defaults, aliases, explicit connection representation, and
   invalid conversion paths. Rustdoc `compile_fail` examples with
   `#![deny(unused_must_use)]` pin the annotations without adding `trybuild`.

An independent Sol/high task reviews public API naming, serde shape, cross-module invariants, and
the 037 boundary after all machine gates pass.

## D012 — Documentation promotes but does not force migration

Decision: README and crate docs present `ScenarioBuilder` as the preferred Rust authoring path and
`ScenarioSpec` as the stable wire/document path. A migration section shows:

- old DTO construction remains accepted;
- `Scenario::try_from(spec)` checks loaded documents;
- `ScenarioBuilder::build` checks programmatic authoring;
- `Simulator::compile_checked` avoids reconversion;
- legacy replace-on-duplicate DTO helpers differ intentionally from checked duplicate errors.

The checked types are re-exported from `anapao::types` and the common prelude. Selected primary
types may be top-level re-exports only if the existing top-level surface remains cohesive during
the independent API review.

## Open decisions

None. The implementation workers have no architecture or product choices left to resolve.
