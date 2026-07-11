# Requirements — 040 Complete Checked Scenario Macro

## Objective

Publish one complete declarative `scenario!` macro over the independently reviewed checked
authoring API. It must bind symbolic graph identities, cover the full scenario vocabulary, remain
hygienic and single-evaluation, return existing setup errors without introducing panic, and be
proved equivalent to direct checked-builder authoring and execution.

## Functional and compatibility requirements

### R001 — Accept the complete public entry form

R001: WHEN `scenario!` receives its documented canonical top-level form THE SYSTEM SHALL accept the
exact queue-flow example, optional title/description/tags/variables/metadata, nodes, edges,
tracking, zero or more end statements, and optional trailing separators and SHALL return
`Result<Scenario, SetupError>`.

Acceptance anchors:

- a runtime integration test compiles and builds the exact queue-flow example;
- trybuild pass cases cover omitted optional sections and every supported trailing separator;
- the macro preserves the checked builder's default end condition when no `end` is supplied.

### R002 — Cover every node and checked config

R002: THE SYSTEM SHALL support Source, Pool, Drain, SortingGate, TriggerGate, MixedGate, Converter,
Trader, Register, Delay, Queue, Process, Sink, Gate, and Custom nodes with every family config
field, typed config escape hatch, and common label/initial/tags/metadata field.

Acceptance anchors:

- runtime tests collectively build all 15 families and inspect their checked behavior/accessors;
- Pool/Queue capacity and Register bounds cover value, `none`, omission, and typed config paths;
- Delay/Queue positive shorthands use checked nonzero setters;
- compile-fail tests reject unknown, duplicate, wrong-family, and mutually exclusive fields.

### R003 — Cover every edge, transfer, connection, and state target

R003: THE SYSTEM SHALL support fixed, fraction, remaining, metric-scaled, expression, and typed
pass-through transfers; default/configured resource and state connections; enabled/metadata
fields; and Node, ResourceConnection, StateConnection, and Formula targets including forward edge
references.

Acceptance anchors:

- runtime tests exercise every transfer and connection field through checked accessors;
- all four state targets and a forward referenced target build successfully when valid;
- typed transfer/resource/state escape hatches produce the same checked values as native fields;
- malformed transfer and target syntax has focused UI diagnostics.

### R004 — Cover scenario fields, metrics, variables, and end conditions

R004: THE SYSTEM SHALL author title, description, tags, `VariableRuntimeConfig`, metadata, tracked
node metrics, every leaf end condition, recursive Any/All composites, repeated top-level ends, and
typed end-condition pass-through without changing established ordering or OR semantics.

Acceptance anchors:

- public tests inspect every scenario-level field and tracked metric;
- end-condition tests cover all five leaves, nested Any/All, multiple top-level entries, and typed
  pass-through in declaration order;
- macro and direct-builder scenarios contain equal end-condition vectors.

### R005 — Bind symbolic IDs as one source of truth

R005: WHEN node or edge symbols are declared or referenced THE SYSTEM SHALL derive their typed IDs
from exact `stringify!` spelling once, SHALL keep node and edge namespaces distinct, and SHALL use
retained values for declared endpoints, metrics, tracking, state targets, and end conditions while
routing undeclared typed spellings through checked-builder reference validation.

Acceptance anchors:

- accessors prove exact symbol-to-ID spelling and valid same-spelling node/edge declarations;
- forward targets resolve from the pre-registered edge namespace;
- undeclared endpoint/metric/tracked/target/end symbols retain the builder's exact missing-reference
  errors rather than a macro-owned registry error;
- duplicate symbols reach the checked builder's stable first-definition duplicate error;
- no macro path authors independent map keys and embedded IDs.

### R006 — Guarantee hygiene and one evaluation

R006: WHEN caller expressions are supplied THE SYSTEM SHALL evaluate each expression exactly once
and SHALL not depend on caller imports, implicit prelude, local variable names, or the Cargo
dependency name.

Acceptance anchors:

- side-effect counters cover every expression category and observe one evaluation;
- a trybuild pass binary uses `#![no_implicit_prelude]` and colliding caller-local names;
- prelude, crate-alias, and real Cargo dependency-rename cases compile and run/check;
- source review confirms `$crate`-qualified Anapao paths and absolute standard-library paths.

### R007 — Separate syntax errors from recoverable semantic errors

R007: IF documented macro syntax is malformed, THEN THE SYSTEM SHALL emit a focused compile-time
diagnostic; IF an ID, positive shorthand, symbol reference, duplicate, or graph semantic is
invalid, THEN THE SYSTEM SHALL return the existing `SetupError` without macro-introduced panic,
unwrap, expect, indexing, fixture constructor, or unchecked extraction.

Acceptance anchors:

- targeted `.stderr` snapshots contain the stable `anapao::scenario!:` diagnostic prefix;
- invalid scenario ID, zero checked shorthand, missing reference, duplicate, and invalid graph
  cases return `Err` under no-unwind assertions;
- fraction-denominator and whole-graph errors retain checked-builder paths/reasons;
- source review finds no banned panic/unchecked construct in expansion.

### R008 — Desugar only through the checked public API

R008: WHILE expanding for a downstream crate THE SYSTEM SHALL construct values only through the
exact public config, node, connection, edge, and `ScenarioBuilder` APIs reviewed by
`039-checked-scenario-authoring/T006` and SHALL NOT access checked private fields, raw validation,
plans, engines, or a macro-only semantic hook.

Acceptance anchors:

- T001 refuses to edit unless 039/T006 is `done` with `verification_status = "passed"`;
- expansion calls the normative `Default`, `with_*`/`without_*`, family, connection, edge, and
  builder methods and finishes at `ScenarioBuilder::build`;
- a fresh API review confirms there is one whole-graph validation path and no private dependency.

### R009 — Prove compiler-facing compatibility

R009: WHEN the public macro is validated THE SYSTEM SHALL pass exact-example, full-surface,
trailing-separator, no-prelude hygiene, prelude, crate-alias, one-evaluation, targeted compile-fail,
and real Cargo dependency-renaming cases with reviewed compiler diagnostics.

Acceptance anchors:

- `cargo test --test scenario_macro_ui` passes committed pass/fail fixtures and snapshots;
- `cargo test --test scenario_macro_renamed` checks a fixture whose dependency is named
  `simulation` using offline isolated target output;
- `TRYBUILD=overwrite` is used only to generate reviewed snapshots, followed by a normal green run.

### R010 — Preserve direct-builder and runtime equivalence

R010: WHEN equivalent direct-builder and macro-authored scenarios are compiled and run with the
same explicit seed/config THE SYSTEM SHALL produce equal checked `Scenario`s and equal complete
`RunReport`s.

Acceptance anchors:

- integration coverage compares the checked scenarios before compilation;
- both paths compile through the public checked facade;
- full reports compare equal under an explicit deterministic run config;
- checked-authoring and existing parity tests remain green.

### R011 — Publish a complete 0.2 macro contract

R011: THE SYSTEM SHALL expose `scenario!` at crate root and through the prelude, preserve legacy DTO
and direct checked-builder routes, and document grammar, symbol mapping, single evaluation,
error/panic layering, builder-only validation, dependency renaming, and public 0.2 SemVer behavior
with executable rustdoc and README examples.

Acceptance anchors:

- root and prelude import examples compile without a `types` macro re-export;
- README includes the exact intake example and a complete surface example without placeholder
  families or `...`;
- rustdoc and `tests/readme_snippets.rs` execute recommended and handled-error paths;
- docs name incompatible grammar/result/evaluation/export changes.

### R012 — Keep the macro set deliberately singular

R012: THE SYSTEM SHALL NOT add `expectations!`, assertion/config/report/artifact macros, a
procedural-macro crate, named exported helper macros, or the out-of-scope `Expectation` constructor
and `AssertionReport::assert_success` function alternatives.

Acceptance anchors:

- public exports contain one documented `scenario!` macro and no named helper macro;
- source/manifest review finds no procedural macro package or extra macro surface;
- docs explicitly route future assertion ergonomics toward normal functions without implementing
  them in this spec.

### R013 — Require independent public macro review

R013: WHEN implementation and machine validation are green THE SYSTEM SHALL receive a fresh
Sol/high review of complete grammar, public expansion dependencies, hygiene and real renaming,
single evaluation, recoverable errors/no panic, UI diagnostics, runtime equivalence, docs, SemVer,
MSRV, and every validation gate before completion.

Acceptance anchors:

- T005 is performed by an independent Sol/high validator after T001-T004;
- every R001-R012 anchor and focused/full command is inspected from the final tree;
- unresolved public API, hygiene, evaluation, panic, diagnostic, or equivalence findings block
  completion.
