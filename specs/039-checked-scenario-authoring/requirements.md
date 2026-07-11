# Requirements — 039 Checked Scenario Authoring

## Objective

Provide a complete checked Rust authoring and execution boundary for scenarios while preserving
`ScenarioSpec` as the stable serde document contract and preserving deterministic behavior for all
already-valid scenarios.

## Functional and compatibility requirements

### R001 — Preserve the wire document contract

R001: WHEN existing scenario JSON is deserialized to `ScenarioSpec`, checked, and converted back THE
SYSTEM SHALL preserve the parsed DTO's semantic serialization, including canonical field names,
normalized aliases/defaults, and default-resource omission.

Acceptance anchors:

- valid legacy JSON fixtures deserialize as `ScenarioSpec`;
- `Scenario::try_from` succeeds for those fixtures;
- `serde_json::to_value(&parsed_scenario_spec)` equals
  `serde_json::to_value(ScenarioSpec::from(&checked))`;
- tests do not require equality with raw fixture `Value`, because alias spelling and omitted
  defaults normalize during serde parsing/serialization before checking;
- existing serde unit tests remain green.

### R002 — Represent complete checked choices

R002: THE SYSTEM SHALL represent every current node family, resource/state connection choice, and state
target choice with public non-exhaustive sum types owned by immutable checked scenario values.

Acceptance anchors:

- unit tests cover every `NodeKind` mapping, both connection variants, and all target variants;
- no checked constructor accepts independent kind/config or target/optional-ID inputs;
- checked scenario/node/edge fields have no public mutable access.

### R003 — Reject key and embedded-ID drift first

R003: WHEN converting a `ScenarioSpec` THE SYSTEM SHALL reject node and edge map-key/embedded-ID
mismatches before later graph validation using deterministic `nodes.<key>.id` or `edges.<key>.id`
error paths.

Acceptance anchors:

- focused tests combine key drift with later errors and prove the key error wins;
- error text names both the map key and embedded ID;
- plan order and indexes are never assembled from a mismatched document.

### R004 — Reconcile node family and config exactly

R004: WHEN converting a node DTO THE SYSTEM SHALL use a default config only when config is omitted for
the selected configurable family and SHALL reject every explicit wrong-family config at
`nodes.<key>.config`.

Acceptance anchors:

- a conversion table test covers all configured and configless families;
- omitted matching configs preserve current defaults;
- explicit mismatches fail rather than reaching pool/delay/queue runtime defaults.

### R005 — Reconcile connection and target choices exactly

R005: WHEN converting an edge DTO THE SYSTEM SHALL collapse connection kind and its active payload into
`ConnectionSpec` and SHALL collapse state target plus required ID into `StateTarget`, rejecting
inactive non-default payloads, node-plus-ID, and missing-ID combinations.

Acceptance anchors:

- conversion tests cover resource/state payload combinations and every state target;
- valid legacy aliases still decode before semantic conversion;
- checked state targets cannot represent a required-but-missing ID.

### R006 — Encode established positive values

R006: WHERE delay steps, queue release, configured queue capacity, resource token size, or compiled
fraction denominator must be positive THE SYSTEM SHALL store the checked value as `NonZeroU64` and
reject zero at its established semantic path.

Acceptance anchors:

- zero/nonzero conversion tests cover every listed field;
- checked/compiled accessors expose `NonZeroU64` or its `.get()` value without clamping;
- pool capacity and zero-step end conditions retain their existing semantics.

### R007 — Reject checked-builder duplicates without replacement

R007: WHEN `ScenarioBuilder` receives a duplicate node or edge ID THE SYSTEM SHALL return a stable
`SetupError`, retain the first definition unchanged, and perform no replacement.

Acceptance anchors:

- mutation-style insertion can recover and build with the first value;
- consuming insertion returns `Err`;
- legacy `ScenarioSpec::with_node` and `with_edge` remain last-write-wins.

### R008 — Provide complete conventional authoring

R008: THE SYSTEM SHALL provide mutable and consuming `ScenarioBuilder` styles, family-specific
`ScenarioNode` and `ScenarioEdge` constructors, checked config accessors/setters, and scenario
title, description, tags, variables, end conditions, tracked metrics, and metadata authoring.

Acceptance anchors:

- mutable and consuming examples build equal checked scenarios;
- every current node family has its frozen `ScenarioNode::{source,pool,drain,sorting_gate,
  trigger_gate,mixed_gate,converter,trader,register,delay,queue,process,sink,gate,custom}`
  constructor;
- common node setters are exactly `with_label`, `with_initial_value`, `with_tag`, and
  `with_metadata`; common edge setters are exactly `with_enabled` and `with_metadata`;
- resource/state construction uses the frozen `ResourceConnection::default/with_token_size` and
  `StateConnection::{default,new,with_role,with_formula,with_target,with_resource_filter}` surface;
- checked config defaults/setters match every current DTO field and positive setters accept
  `NonZeroU64`;
- `ScenarioBuilder::build` validates the whole graph through the same gate as DTO conversion.

### R009 — Diagnose discarded consuming authoring

R009: WHEN a caller discards a consuming scenario-authoring `with_*` result THE SYSTEM SHALL emit
`unused_must_use` through explicit custom-message annotations on the checked builder/value methods
and the retained DTO scenario-authoring methods.

Acceptance anchors:

- rustdoc `compile_fail` examples with `#![deny(unused_must_use)]` cover one checked method and one
  DTO method;
- normal doctests demonstrate correct result retention;
- run/batch/report/artifact builders are unchanged by this requirement.

### R010 — Preserve and extend the compile facade

R010: WHEN a caller compiles a valid `ScenarioSpec` or checked `Scenario` THE SYSTEM SHALL converge on
the sole private plan assembler while preserving the 037 opaque `CompiledScenario`, DTO compile
signature, `TryFrom<ScenarioSpec>`, accessors, and module privacy.

Acceptance anchors:

- `Simulator::compile_checked(Scenario)` and `TryFrom<Scenario>` are available;
- all four compile/conversion entrypoints produce equivalent opaque handles;
- `source_spec()` continues to expose the canonical DTO read-only;
- no raw engine or validation API is made public.

### R011 — Execute only checked behavior

R011: WHILE executing a compiled scenario THE SYSTEM SHALL make node behavior, connection behavior,
state target, positive timing, denominator, and token-size decisions only from checked plan values
and SHALL contain no wrong-family default, missing-target, zero-denominator, or `.max(1)` repair
path for those invariants.

Acceptance anchors:

- `CompiledNode` and `CompiledEdge` carry checked projections;
- the named legacy engine fallback helpers/branches are removed or accept only checked inputs with
  no fallback result;
- a fresh Sol/high validator inspects validation, plan, and engine data flow.

### R012 — Preserve deterministic valid behavior

R012: WHEN equivalent valid DTO and checked-builder scenarios execute with the same explicit seed THE
SYSTEM SHALL produce identical `RunReport`s and retain existing parity, delay, queue, gate, state,
and resource-transfer behavior.

Acceptance anchors:

- public integration coverage compares full reports;
- focused engine and parity tests remain green;
- `cargo test --all-targets` passes.

### R013 — Publish a complete migration path

R013: THE SYSTEM SHALL re-export and document checked scenario authoring, DTO checking, checked compile,
duplicate-policy differences, and intentional 0.2 strictness while retaining the legacy DTO route.

Acceptance anchors:

- README and crate docs contain tested complete examples;
- `anapao::types` and the prelude expose the intended checked types;
- `tests/readme_snippets.rs` pins the new examples and 037 accessor usage.

### R014 — Require independent contract review

R014: WHEN implementation and machine validation are green THE SYSTEM SHALL receive an independent
Sol/high review of public API ergonomics, serde compatibility, and DTO-to-plan-to-engine invariants
before the spec is completed.

Acceptance anchors:

- T006 is performed by a fresh Sol/high validator;
- its review checks every requirement and all full validation commands;
- unresolved public API, serde, or unchecked-engine findings block completion.

### R015 — Preserve formula parse-once semantics

R015: WHEN `ScenarioSpec` conversion or `ScenarioBuilder::build` validates active formulas THE
SYSTEM SHALL parse every resource-transfer expression and modifier-state expression exactly once,
retain the crate-private ASTs inside opaque `Scenario`, and move them into 037
`CompiledExpressions` without reparsing or public exposure.

Acceptance anchors:

- `Scenario` privately owns edge-ID-keyed transfer/state `CompiledExpr` maps with no public
  accessor or serde participation;
- formula tests cover resource transfer expressions, modifier state formulas for every target,
  disabled edges, inactive state-edge transfer expressions, nonmodifier `*`/control strings, and
  invalid syntax paths;
- `Simulator::compile_checked` consumes the scenario and moves each AST into the matching
  edge-index-aligned slot, with no call to `ExprRuntime::compile` in plan/simulator/engine assembly;
- inverse DTO projection ignores the AST bundle and remains equal to the parsed DTO baseline;
- the independent Sol/high reviewer confirms AST parse count, bundle completeness, move-only plan
  handoff, error timing, and runtime formula parity.
