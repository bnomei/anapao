# Make Specs Handoff: checked-scenario-authoring

## Status

- research_id: checked-scenario-authoring
- status: promoted
- intended_spec_slug: checked-scenario-authoring
- target_spec: `specs/039-checked-scenario-authoring/`
- shape_review: GREEN
- cheap_worker_ready: yes

## Objective

Add a complete checked Rust scenario-authoring domain beside the stable serde DTOs, with exhaustive
node/connection sum types, a conventional deterministic builder, fallible DTO conversion,
nonzero execution values, strict duplicate and identity validation, and end-to-end execution-plan
integration. Preserve valid legacy JSON and `Simulator::compile(ScenarioSpec)` while ensuring the
post-037 engine can only consume checked variants, never repairs mismatches with defaults, and
receives the AST from the single formula-validation parse through the checked `Scenario`.

## Requirements seed

- R001: WHEN existing scenario JSON is deserialized and re-exposed through the checked boundary THE
  SYSTEM SHALL preserve the semantic serialization of the parsed DTO, including established field
  names, defaults, aliases as normalized by serde, and default-resource omission.
- R002: THE SYSTEM SHALL represent every current node family, resource/state connection choice,
  and state target choice in complete public checked sum types whose checked owners have private
  fields.
- R003: WHEN converting `ScenarioSpec` THE SYSTEM SHALL reject node and edge map-key/embedded-ID
  mismatches before all later graph validation with deterministic path-rich `SetupError`s.
- R004: WHEN converting node or edge DTOs THE SYSTEM SHALL default only an omitted matching family
  payload and reject every explicit tag/payload or target/ID mismatch.
- R005: WHERE delay steps, queue release/capacity, resource token size, or compiled fraction
  denominator must be positive THE SYSTEM SHALL represent the checked value with `NonZeroU64` and
  reject zero at the existing semantic path.
- R006: WHEN a checked builder receives a duplicate node or edge ID THE SYSTEM SHALL return an
  error, retain the first definition unchanged, and never silently replace it.
- R007: THE SYSTEM SHALL provide complete mutable and consuming `ScenarioBuilder` authoring,
  family-specific node/edge constructors, scenario metadata/end-condition methods, and checked
  compilation without typestate or serde round-tripping.
- R008: WHEN a consuming scenario-authoring `with_*` result is discarded THE SYSTEM SHALL emit the
  standard `unused_must_use` diagnostic through explicit annotations on the checked and retained
  DTO authoring surfaces.
- R009: WHEN a caller compiles a valid legacy DTO or checked `Scenario` THE SYSTEM SHALL converge on
  the same private plan assembler while preserving the 037 opaque facade, accessors, and module
  privacy.
- R010: WHILE executing a compiled scenario THE SYSTEM SHALL make node behavior, connection
  behavior, state target, token size, and positive timing decisions only from checked plan values,
  with no wrong-family default, missing-target, zero-denominator, or `.max(1)` repair path.
- R011: WHEN equivalent valid DTO and checked-builder scenarios run with the same seed THE SYSTEM
  SHALL produce identical deterministic reports.
- R012: THE SYSTEM SHALL expose and document the checked types, DTO/check/compile flow, duplicate
  distinction, 0.2 strictness, and migration path while retaining the legacy DTO route.
- R013: WHEN implementation and machine validation are complete THE SYSTEM SHALL receive an
  independent Sol/high review of public API, serde compatibility, and cross-module invariants
  before completion.
- R015: WHEN `ScenarioSpec` conversion or builder build validates active formulas THE SYSTEM SHALL
  parse every resource-transfer expression and modifier-state expression exactly once, retain the
  crate-private ASTs in `Scenario`, and move them into `CompiledExpressions` without reparsing or
  public exposure.

## Scope

In scope:

- `src/types/scenario.rs`
- `src/types/scenario_checked.rs`
- `src/types/mod.rs`
- `src/validation/mod.rs`
- `src/plan.rs`
- `src/engine/mod.rs`
- `src/simulator.rs`
- `src/lib.rs`
- `src/prelude.rs`
- `tests/checked_scenario_authoring.rs`
- `tests/fixtures/scenario-wire-v1/legacy-default-resource.json`
- `tests/fixtures/scenario-wire-v1/legacy-state-aliases.json`
- `tests/fixtures/scenario-wire-v1/node-config-mismatch.json`
- `tests/fixtures/scenario-wire-v1/map-key-id-mismatch.json`
- `tests/readme_snippets.rs`
- `README.md`

Out of scope:

- `scenario!` implementation; spec 040 consumes this public API.
- Run/batch/capture/report/artifact redesign.
- Raw lexical duplicate-key detection before serde builds `ScenarioSpec`.
- Typestate, builder derive crates, a second wire format, or mutable checked maps.
- Re-publicizing engine, validation, plan assembly, or other 037-private internals.

## Current-state facts

- `src/types/scenario.rs:185-201` and `src/types/scenario.rs:353-364` store node family and config
  independently.
- `src/types/scenario.rs:256-344` stores connection kind, two payloads, state target, and optional
  target ID independently.
- `src/types/scenario.rs:551-560` makes DTO `with_node/with_edge` last-write-wins.
- `src/types/mod.rs:80-194` pins omitted defaults, default-resource omission, state aliases, default
  formula, and serde round trips.
- `src/validation/mod.rs:39-101` builds order/indexes from map keys without comparing embedded IDs.
- `src/validation/mod.rs:677-833` substitutes pool/delay/queue defaults for mismatched configs.
- `src/engine/mod.rs:1506-1661` re-matches independent DTO fields and returns missing/wrong-family
  defaults.
- `src/engine/mod.rs:1757-1764` and `src/engine/mod.rs:1826-1832` retain zero repair paths.
- After `037-compiled-scenario-trust-boundary/T004`, `src/plan.rs` owns the opaque plan and
  `src/validation/mod.rs` alone may assemble it; the DTO compile facade and read-only accessors are
  stable.

## Decisions

- Keep every existing serde DTO declaration and attribute stable; checked types do not derive
  serde.
- Add immutable `Scenario`, `ScenarioNode`, `ScenarioEdge`, complete `NodeBehavior`,
  `ConnectionSpec`, and `StateTarget` plus private checked config types.
- Preserve the parsed source DTO inside `Scenario`/the compiled plan. Semantic preservation compares
  serialization of that parsed DTO with `ScenarioSpec::from(&checked)`; raw alias spelling and
  omitted-field spelling are outside the contract because serde normalizes them before checking.
- Accept `NodeConfig::None` as the default only for the selected configurable family; reject every
  explicit wrong family. Configless/legacy/custom kinds require `None`.
- Use `NonZeroU64` only for currently established positive invariants: delay, queue
  release/capacity, resource token size, and compiled fraction denominator. Pool capacity and
  zero-step end conditions retain current semantics.
- Derive checked-builder map keys from private embedded IDs. Duplicate insertion errors through
  `BTreeMap::entry` and leaves the first value unchanged. Legacy DTO helpers still replace.
- Reuse `SetupError` and existing path grammar. Do not introduce an overlapping builder error tree.
- New public enums are `#[non_exhaustive]`; existing wire enums are not modified.
- DTO and builder conversion share one whole-graph gate; valid DTO compile and checked compile
  converge before the plan assembler.
- `Scenario` privately owns crate-private `ValidatedExpressions` maps keyed by `EdgeId`. The shared
  gate parses every resource-transfer expression and every modifier-state expression exactly once;
  plan assembly consumes/moves those ASTs into edge-indexed 037 `CompiledExpressions` without a
  second parse. Inactive transfer expressions on state edges and nonmodifier control formula
  strings do not produce ASTs.
- Remove, rather than leave dormant, the named runtime fallbacks after plan migration.

Rejected:

- Replacing DTOs with domain sums.
- Partial node-family coverage.
- Typestate or generated builders.
- Silent mismatch normalization or duplicate replacement.
- Direct engine consumption of preserved source DTOs.
- Adding serde to the checked domain.

Open:

- None.

## Implementation-shape excerpts

### Checked types

Create `src/types/scenario_checked.rs`. `Scenario` owns the parsed `ScenarioSpec`, private checked
node/edge maps, and private crate-private `ValidatedExpressions` transfer/state AST maps keyed by
edge ID. It exposes only read access, `into_spec`, `TryFrom<ScenarioSpec,
Error = SetupError>`, and `From<Scenario> for ScenarioSpec`. `NodeBehavior` covers Source, Pool,
Drain, SortingGate, TriggerGate, MixedGate, Converter, Trader, Register, Delay, Queue, Process,
Sink, Gate, and Custom. `ConnectionSpec` is Resource or State. `StateTarget` is Node or an
ID-carrying ResourceConnection, StateConnection, or Formula variant.

Checked config fields mirror current DTO fields. Delay steps, queue release, optional queue
capacity, and resource token size use `NonZeroU64`; other fields retain their current types and
defaults. Every config implements wire-equivalent `Default`. Config setters are field-named;
optional pool/queue capacity and register min/max additionally provide `without_capacity`,
`without_min_value`, and `without_max_value`; positive delay/queue setters accept `NonZeroU64`.

Freeze these public spellings for workers and spec 040:

- node constructors: `ScenarioNode::{source,pool,drain,sorting_gate,trigger_gate,mixed_gate,
  converter,trader,register,delay,queue,process,sink,gate,custom}`;
- node common setters: `with_label`, `with_initial_value`, `with_tag`, `with_metadata`;
- resource construction: `ResourceConnection::default().with_token_size(NonZeroU64)`;
- state construction: `StateConnection::default()` or
  `StateConnection::new(StateConnectionRole, impl Into<String>, StateTarget)`, then
  `with_role`, `with_formula`, `with_target`, `with_resource_filter`;
- edge constructors: `ScenarioEdge::{resource,state}` with common `with_enabled` and
  `with_metadata`.

All consuming setters return `Self` and are `#[must_use]`. The full getter/parameter signatures in
the implementation shape are normative. No macro-only helper or private-field desugaring is allowed.

### Builder

`ScenarioBuilder` privately stores a canonical `ScenarioSpec`. It provides `new`, mutable
`insert_node/insert_edge`, consuming `with_node/with_edge`, and consuming title, description, tag,
variables, end-condition, tracked-metric, and metadata methods. `build` delegates to
`Scenario::try_from`. Occupied map entries error at `nodes.<id>`/`edges.<id>` without mutation.
Annotate the builder and all consuming scenario-authoring `with_*` methods, including retained DTO
methods in `src/types/scenario.rs`, with custom `#[must_use]` messages. Builder insertion/build must
not pre-parse formulas; the delegated conversion owns the sole parse and returned AST bundle.

### Conversion order

In `src/validation/mod.rs`, compare node keys/IDs, then edge keys/IDs, then convert nodes, then
edges/connections/targets, then run existing graph passes. Preserve BTree order and pin first-error
ordering. `NodeConfig::None` normalizes only for the matching kind; explicit mismatches fail at
`nodes.<key>.config`. Resource/state inactive payloads must be default. Node targets forbid an ID;
all other state targets require and own one. Formula validation returns and retains `CompiledExpr`
instead of discarding it. Parse resource-connection `TransferSpec::Expression` values and every
modifier-state formula, including disabled edges/all targets; explicitly exclude inactive
state-edge transfer expressions and nonmodifier control strings.

### Plan and engine

The plan migration task must read `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and
stop unless it is `done` with `verification_status = "passed"`. The task validator rejects the
cross-spec task-edge syntax, so spec-level dependency plus a before-implementation checkpoint carry
queue ordering. Preserve 037 public accessors and private module ownership. Add
`Simulator::compile_checked(Scenario)` and
`TryFrom<Scenario> for CompiledScenario`; keep `Simulator::compile(ScenarioSpec)` and
`TryFrom<ScenarioSpec>` signatures. `CompiledNode` owns `NodeBehavior`; `CompiledEdge` owns
`ConnectionSpec` and checked compiled transfer data. `compile_checked` consumes the scenario and
moves its edge-ID-keyed AST bundle into 037's edge-index-aligned transfer/state slots without
calling `ExprRuntime::compile`. Remove the wrong-family timing/mode/capacity
fallbacks, optional state-target-ID fallback, zero fraction branch, and token-size `.max(1)` from
engine decisions.

### Tests and migration

Add all four exact fixtures and `tests/checked_scenario_authoring.rs`. Prove semantic JSON equality
between serialization of the parsed DTO and `ScenarioSpec::from(&checked)`; do not compare against
raw fixture `Value`. Prove invalid DTO semantic failures at exact paths, duplicate policy, both
builder styles, facade convergence, and report equality under an explicit seed. Add unit
conversion/projection coverage for every family, target, included/excluded formula kind, retained
AST alignment, invalid formula path, and no-reparse handoff. Add rustdoc compile-fail examples
with `#![deny(unused_must_use)]`. Update README, crate docs, prelude, and snippet tests with both the
preferred checked path and retained DTO migration route.

## Suggested spec shape

- spec_kind: feature
- fanout_policy: serial
- execution_policy: auto-continue
- human_checkpoint: before-implementation
- commit_policy: after-validation
- review_policy: required
- depends_on: `037-compiled-scenario-trust-boundary`
- task_slices:
  - T001: Add the complete checked scenario domain, DTO projections, and validated-expression
    carrier (`sol`/`high`).
  - T002: Add the checked builder, duplicate contract, and scenario `must_use` surface
    (`sol`/`high`), depends on T001.
  - T003: Migrate validation, plan, facade, and engine to checked values (`sol`/`high`), depends on
    T001 and T002, and stops unless 037/T004 is complete and verified.
  - T004: Add wire fixtures and public behavior compatibility coverage (`terra`/`high`), depends on
    T003.
  - T005: Complete re-exports, docs, doctests, and migration guidance (`terra`/`medium`), depends on
    T004.
  - T006: Independently review and close public API/serde/plan invariants (`sol`/`high`), depends on
    T005 and uses `verification_mode = "required"`.

## Validation

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

No network, Docker, or external service is required.

## Worker context policy

- T001 may read: `src/types/scenario.rs`, `src/types/mod.rs`, `src/error.rs`, `src/expr/mod.rs`.
- T002 may read: `src/types/scenario.rs`, `src/types/scenario_checked.rs`, `src/types/mod.rs`.
- T003 may read: `src/types/scenario.rs`, `src/types/scenario_checked.rs`, `src/validation/mod.rs`,
  `src/plan.rs`, `src/engine/mod.rs`, `src/simulator.rs`, `src/error.rs`, `src/expr/mod.rs`.
- T004 may read: the preceding source files, `src/types/mod.rs`, `tests/parity/differential.rs`, and
  `tests/readme_snippets.rs`.
- T005 may read: `src/lib.rs`, `src/prelude.rs`, `README.md`, `tests/readme_snippets.rs`, and the new
  checked public source file.
- T006 may read every concrete in-scope source/test/doc path listed in Scope and the completed task
  reports, but no raw research files.
- Workers must not be sent to `raw/`, prototypes, broad current-state research, decision dialogue,
  old spec aggregates, `specs/index.md`, or `specs/_handoff.md`.
