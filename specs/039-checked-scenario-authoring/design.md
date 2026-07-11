# Design — 039 Checked Scenario Authoring

## Objective

Separate Anapao's stable serde document model from an immutable checked authoring/execution model.
Loaded `ScenarioSpec`s and programmatic `ScenarioBuilder`s converge on one validation boundary;
the post-037 execution plan stores checked sums so runtime code no longer repairs impossible
tag/payload combinations.

## Scope

This spec owns:

- the new checked scenario types and complete builder;
- DTO-to-checked conversion and deterministic semantic errors;
- checked plan/facade/engine integration after spec 037;
- stable JSON fixtures, behavior parity, rustdoc diagnostics, public re-exports, and migration docs.

It does not own the later `scenario!` macro, run/batch/capture/report/artifact redesign, lexical
duplicate JSON-key detection, typestate, generated builders, or a new serialization format.

## Distilled current-state facts

- `src/types/scenario.rs:185-201` and `src/types/scenario.rs:353-364` let `NodeKind` and
  `NodeConfig` disagree.
- `src/types/scenario.rs:256-344` independently stores connection kind, two payloads, target kind,
  and optional target ID.
- `src/types/scenario.rs:551-560` documents and implements DTO insertion as replacement.
- `src/types/mod.rs:80-194` pins omitted config/connection defaults, default-resource omission,
  state aliases, default formula, and serde round trips.
- `src/validation/mod.rs:39-101` derives deterministic indexes from map keys without checking
  embedded IDs.
- `src/validation/mod.rs:677-833` substitutes defaults for explicit wrong-family pool/delay/queue
  configs.
- `src/engine/mod.rs:1506-1661` re-matches the duplicated DTO state and supplies missing/wrong
  defaults; `src/engine/mod.rs:1757-1764` and `src/engine/mod.rs:1826-1832` repair zero values.
- Spec 037 makes `CompiledScenario` opaque in `src/plan.rs`, keeps `src/validation/mod.rs` as sole
  plan assembler, and privatizes engine/validation modules after task T004.

## Architecture

### Stable wire layer

Existing serde declarations in `src/types/scenario.rs` remain source and wire compatible. No serde
attribute, alias, default, public field, or output-omission rule changes. These types remain useful
for loading, inspecting, editing, and storing documents even before semantic validation.

### Checked domain layer

`src/types/scenario_checked.rs` adds immutable `Scenario`, `ScenarioNode`, and `ScenarioEdge` plus:

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

Checked configs mirror current wire fields behind private storage. `DelayConfig.delay_steps`,
`QueueConfig.release_per_step`, present queue capacity, and `ResourceConnection.token_size` are
`NonZeroU64`; the compiled fraction denominator is also nonzero. Pool capacity and zero-step end
conditions keep current types because zero is not currently rejected there.

`Scenario` retains the parsed/canonical `ScenarioSpec`, private checked node/edge maps, and a
private crate-private `ValidatedExpressions` bundle for plan assembly. The bundle is two
deterministic `BTreeMap<EdgeId, CompiledExpr>` maps for transfer and modifier-state ASTs. No AST or
bundle accessor is public and neither participates in serde. `Scenario` exposes read-only domain
accessors, `TryFrom<ScenarioSpec>`, and infallible conversion back to the retained DTO; inverse DTO
projection ignores/drops the private AST bundle.

Semantic JSON preservation is measured after serde parsing:

```rust
serde_json::to_value(&parsed_scenario_spec)?
    == serde_json::to_value(ScenarioSpec::from(&checked))?
```

Raw `target_edge`/`filter` spelling and omitted formula/default spelling may normalize during the
initial serde parse/serialize cycle and are not a checked-conversion promise.

The public authoring spellings are contract, not implementation choice:

- all checked configs implement wire-equivalent `Default` and field-named consuming setters;
  optional pool/queue capacity and register min/max have `without_*` clearers, while delay/queue
  positive setters accept `NonZeroU64`;
- every family uses `ScenarioNode::{source,pool,drain,sorting_gate,trigger_gate,mixed_gate,
  converter,trader,register,delay,queue,process,sink,gate,custom}`;
- common node setters are `with_label`, `with_initial_value`, `with_tag`, and `with_metadata`;
- resource connections use `ResourceConnection::default().with_token_size(NonZeroU64)`;
- state connections use `StateConnection::default()` or
  `StateConnection::new(StateConnectionRole, impl Into<String>, StateTarget)`, plus
  `with_role`, `with_formula`, `with_target`, and `with_resource_filter`;
- edges use `ScenarioEdge::{resource,state}` plus `with_enabled` and `with_metadata`.

All consuming setters return `Self`, carry `#[must_use]`, and preserve current DTO map insertion
semantics. Spec 040 may expand only through these methods, never a private or macro-only hook.

### Conventional builder

`ScenarioBuilder` privately owns a canonical `ScenarioSpec` draft. It has mutation-style
`insert_node/insert_edge`, consuming `with_node/with_edge`, and complete metadata, variable, end
condition, and tracked-metric methods. Family-specific node/edge constructors bind each sum variant
to its payload.

Insertion uses `BTreeMap::entry`. An occupied ID returns `SetupError::InvalidParameter` at
`nodes.<id>` or `edges.<id>` and leaves the first value unchanged. The old DTO helpers retain
last-write-wins. `build()` delegates to `Scenario::try_from`; it does not implement a second
validator or preliminary formula parse.

The builder and every consuming scenario-authoring `with_*` method carry custom `#[must_use]`
messages. This includes relevant existing methods on `NodeSpec`, `EdgeSpec`, and `ScenarioSpec`, but
does not expand into unrelated config/report builders.

## Conversion and validation

`src/validation/mod.rs` owns one DTO-to-checked gate. It operates in fixed order:

1. node map key versus `NodeSpec.id`;
2. edge map key versus `EdgeSpec.id`;
3. node local conversion in sorted key order;
4. edge/connection/target conversion in sorted key order;
5. existing endpoint, end-condition, metric, cycle, connection, node, formula, and variable passes,
   with formula validation returning its `CompiledExpr` instead of discarding it;
6. deterministic edge-ID insertion of returned ASTs into `ValidatedExpressions`;
7. private checked `Scenario` construction.

Configured node families accept `NodeConfig::None` as the established default or their exact
matching variant. Configless, legacy, and custom node families accept only `None`. Every explicit
wrong family fails at `nodes.<key>.config`.

Resource connections require a default inactive state payload; state connections require a default
inactive resource payload. Node state targets forbid a target ID. Resource/state/formula targets
require and own one. `NonZeroU64::new` preserves existing zero error paths and reasons.

Errors remain `SetupError`; a second overlapping builder error enum is not introduced. BTree order
and fixed pass order make the first error deterministic.

The shared conversion uses one `ExprRuntime` and parses exactly these active formula kinds once:

- `TransferSpec::Expression` on every resource connection, including disabled edges;
- the state formula on every `StateConnectionRole::Modifier`, regardless of target or enabled
  state.

It does not parse transfer expressions attached to state connections or nonmodifier
Trigger/Activator/Filter formula/control strings such as `*`; existing blank/role/target rules still
validate those fields. `MetricScaled`, variable sources, and end conditions contain no AST. Tests
enumerate every included and excluded kind plus invalid syntax/error paths.

## Compile and execution data flow

```text
ScenarioSpec --TryFrom + one parse--> checked Scenario <--build-- ScenarioBuilder
      |                         |
      +-- Simulator::compile ---+-- Simulator::compile_checked
                                |
                                v
                src/validation/mod.rs plan assembler
                                |
                +--------------+---------------+
                |                              |
         preserved parsed DTO         checked CompiledNode/Edge
                                                + moved AST slots
                 |                              |
          source_spec()                       engine
```

T003 requires completed, verified `037-compiled-scenario-trust-boundary/T004`. Because the current
task-schema validator rejects cross-spec task edges as unknown, orchestration uses
`spec.toml.depends_on`, `human_checkpoint = "before-implementation"`, and an explicit T003
frontmatter-status guard. T003 preserves the predecessor's opaque facade, source accessor, and
private modules. `Simulator::compile_checked(Scenario)` and
`TryFrom<Scenario> for CompiledScenario` are additive; the existing DTO signatures remain.

`CompiledNode` owns `NodeBehavior`. `CompiledEdge` owns `ConnectionSpec` plus 037's compiled
transfer/expression data. `Simulator::compile_checked` consumes the owned `Scenario`; plan assembly
moves its edge-ID-keyed ASTs into 037's edge-index-aligned optional transfer/state
`CompiledExpressions` slots, verifies every required slot/entry, and never calls
`ExprRuntime::compile`. The engine removes wrong-family timing/mode/capacity fallbacks,
optional-ID target fallback, fraction zero branch, and token `.max(1)`. Source DTOs may be exposed
for inspection but cannot drive execution decisions.

## Compatibility and migration

- Existing JSON continues to deserialize into the unchanged DTOs.
- Valid legacy DTOs compile and run identically.
- Checking a loaded DTO preserves serialization of the parsed DTO; raw fixture alias/default
  spelling is outside the checked-conversion baseline.
- Explicit tag/payload mismatch and key/ID drift now fail before plan assembly.
- The new builder rejects duplicates; old DTO helpers continue replacement.
- New public enums start non-exhaustive. Existing wire enums are not retrofitted.
- No stored-data backfill is required.
- Spec 040 depends on the completed checked-builder/public API and must expand through it.

## Test strategy

Unit tests cover every node and connection conversion, target mapping, nonzero field, inverse DTO
projection, duplicate behavior, first-error ordering, every included/excluded formula kind, invalid
formula paths, and private AST-bundle contents. Plan/engine tests prove checked projections, AST
slot alignment/complete bundle consumption, no reparse, and removal of runtime repair paths.

`tests/checked_scenario_authoring.rs` loads four frozen fixtures:

- `tests/fixtures/scenario-wire-v1/legacy-default-resource.json`
- `tests/fixtures/scenario-wire-v1/legacy-state-aliases.json`
- `tests/fixtures/scenario-wire-v1/node-config-mismatch.json`
- `tests/fixtures/scenario-wire-v1/map-key-id-mismatch.json`

It proves parsed-DTO semantic value preservation (not raw fixture value equality), invalid semantic
failures after successful serde, duplicate differences, both builder styles, facade convergence,
formula compile/run parity, and equal full reports for a fixed seed. Rustdoc compile-fail examples
deny `unused_must_use` without a new test dependency.

## Traceability

| Requirement | Tasks | Validation | Risk/open decision |
| --- | --- | --- | --- |
| R001 | T001, T004, T006 | parsed-DTO vs checked DTO value equality | wire drift; no open decision |
| R002 | T001, T006 | exhaustive conversion tests; API review | incomplete family set |
| R003 | T001, T004, T006 | mismatch ordering tests | identity drift before indexes |
| R004 | T001, T004, T006 | family matrix tests | accidental defaulting |
| R005 | T001, T004, T006 | connection/target matrix tests | inactive payload leakage |
| R006 | T001, T003, T006 | zero/nonzero and engine tests | semantic overreach |
| R007 | T002, T004, T006 | duplicate recovery/replacement tests | silent overwrite |
| R008 | T002, T004, T005 | builder equivalence and docs | partial authoring surface |
| R009 | T002, T005, T006 | compile-fail doctests | missed consuming method |
| R010 | T003, T004, T006 | facade convergence and privacy review | 037 contract drift |
| R011 | T003, T006 | focused engine tests and source review | duplicate unchecked path |
| R012 | T003, T004, T006 | full report equality; all-target suite | valid behavior drift |
| R013 | T005, T006 | doctests and README snippet test | confusing migration |
| R014 | T006 | fresh Sol/high review report | reviewer independence |
| R015 | T001, T002, T003, T004, T006 | formula-kind matrix; AST bundle/slot/no-reparse review | parse-twice regression |

## Validation plan

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

No Docker, external service, or approved network access is required.

## Risks and stop conditions

- Stop if 037 changes T004, `src/plan.rs` ownership, facade signatures, or accessors.
- Stop if T003 starts before 037/T004 is `done` and independently verified; this is an explicit
  guard because the task validator cannot encode the cross-spec task edge.
- Stop if checked conversion cannot retain and move the exact AST returned by validation without
  reparsing, exposing ASTs, or changing formula error precedence.
- Stop if exact serde preservation requires modifying an existing attribute.
- Stop if a valid parity scenario changes report behavior rather than an invalid scenario failing
  earlier.
- Stop before inventing a new public error tree or leaving an engine DTO-decision path.
- Stop if the macro spec requests private checked fields or raw validator access.

No open architecture or product decisions remain.
