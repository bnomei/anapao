# Make Specs Handoff: scenario-macro

## Status

- research_id: scenario-macro
- status: promoted
- intended_spec_slug: scenario-macro
- shape_review: GREEN
- cheap_worker_ready: yes

## Objective

Add one complete public `scenario!` macro that binds symbolic graph IDs, covers the entire checked
scenario-authoring vocabulary, expands only through the independently reviewed spec-039 public
builder/types, evaluates expressions once, returns `Result<Scenario, SetupError>` without a
macro-introduced panic, remains hygienic under real Cargo dependency renaming, and ships with
compiler UI, runtime equivalence, rustdoc, README, and independent public-API validation.

## Requirements seed

- R001: WHEN `scenario!` receives the documented top-level form THE SYSTEM SHALL accept the exact
  intake example, canonical optional sections, and trailing separators and SHALL return
  `Result<Scenario, SetupError>`.
- R002: THE SYSTEM SHALL cover Source, Pool, Drain, SortingGate, TriggerGate, MixedGate, Converter,
  Trader, Register, Delay, Queue, Process, Sink, Gate, and Custom with every checked family config
  field and every common node field.
- R003: THE SYSTEM SHALL cover every transfer, resource/state connection field, common edge field,
  and Node/ResourceConnection/StateConnection/Formula state target, including forward targets.
- R004: THE SYSTEM SHALL cover scenario title, description, tags, variables, metadata, tracked
  node metrics, every leaf end condition, recursive Any/All, repeated top-level ends, and default
  end behavior.
- R005: WHEN a symbol is declared or referenced THE SYSTEM SHALL derive typed node/edge/metric IDs
  from its exact spelling once, maintain separate node/edge namespaces, and prevent independent
  key/embedded-ID authoring.
- R006: WHEN caller expressions are supplied THE SYSTEM SHALL evaluate each expression exactly once
  and SHALL not depend on caller imports, prelude, local names, or the Cargo dependency name.
- R007: IF macro syntax is unsupported, THEN THE SYSTEM SHALL emit a focused compile-time
  diagnostic; IF value or graph semantics are invalid, THEN THE SYSTEM SHALL return the existing
  `SetupError` without macro-introduced panic, unwrap, expect, indexing, or fixture constructors.
- R008: WHILE expanding outside the defining crate THE SYSTEM SHALL use `$crate`-qualified public
  spec-039 APIs and SHALL not access private checked fields, raw validation, plans, engines, or a
  macro-only semantic path.
- R009: WHEN compiler compatibility is validated THE SYSTEM SHALL pass exact-example, full-surface,
  trailing-separator, no-prelude hygiene, prelude, alias, one-evaluation, targeted compile-fail,
  and real Cargo dependency-renaming cases with reviewed diagnostics.
- R010: WHEN equivalent direct-builder and macro scenarios are compiled and run with an explicit
  seed THE SYSTEM SHALL produce equal checked scenarios and full deterministic reports.
- R011: THE SYSTEM SHALL export only the documented `scenario!` macro at crate root and through the
  prelude, preserve DTO/direct-builder routes, and document grammar/error/evaluation/SemVer
  behavior with tested rustdoc and README examples.
- R012: THE SYSTEM SHALL NOT add `expectations!`, assertion/config/report macros, a procedural macro
  crate, or the out-of-scope assertion function alternatives.
- R013: WHEN implementation and machine gates are green THE SYSTEM SHALL receive a fresh Sol/high
  review of complete grammar, public expansion dependencies, hygiene/rename, one-evaluation,
  error/no-panic layering, docs, SemVer, MSRV, and validation before completion.

## Scope

In scope:

- `src/scenario_macro.rs`
- `src/lib.rs`
- `src/prelude.rs`
- `Cargo.toml`
- `Cargo.lock`
- `tests/scenario_macro.rs`
- `tests/scenario_macro_ui.rs`
- `tests/ui/scenario_macro/pass/`
- `tests/ui/scenario_macro/fail/`
- `tests/scenario_macro_renamed.rs`
- `tests/fixtures/scenario-macro-renamed/Cargo.toml`
- `tests/fixtures/scenario-macro-renamed/.gitignore`
- `tests/fixtures/scenario-macro-renamed/src/main.rs`
- `README.md`
- `tests/readme_snippets.rs`

Out of scope:

- Any source owned by spec 039 except reading its completed public API.
- `expectations!`, assertion/config/report/artifact macros, procedural macros, or a workspace split.
- Implementing `Expectation` associated constructors or
  `#[track_caller] AssertionReport::assert_success`.
- A new serde scenario format, builder/validation path, error enum, dynamic ID grammar, implicit
  threshold scaling, or stochastic-variable sublanguage.
- Engine, plan, validation, capture, run, batch, report, artifact, or deterministic behavior
  changes.

## Current-state facts

- `Cargo.toml:1-14` is package `anapao` 0.1.1, edition 2021, MSRV 1.85;
  `Cargo.toml:40-43` has no `trybuild`.
- `src/types/scenario.rs:14-344` defines all node/config/transfer/end/variable/connection/target
  vocabulary; `src/types/scenario.rs:353-473` defines common node, edge, and scenario fields.
- `src/types/identifiers.rs:28-44` separates fallible `new` from panic-based `fixture`.
- `src/error.rs:17-26` exposes the existing `SetupError` taxonomy.
- `src/validation/mod.rs:249-300` resolves metrics to node IDs;
  `src/engine/mod.rs:1985-2017` gives top-level end vectors OR semantics and recursive Any/All.
- `src/lib.rs:123-150`, `src/prelude.rs:1-12`, and `tests/readme_snippets.rs` own facade/docs proof;
  the repository has no exported macro or UI harness.
- Spec 039 freezes complete checked constructors/configs/builder authoring in T001-T005, then T006
  independently reviews/remediates the entire public contract. Spec 040 must consume T006, not
  private checked state or raw validation.
- The Rust Reference requires `$crate` plus public visibility and defines mixed-site hygiene and
  expression follow sets. `trybuild` pass cases execute and compile-fail cases compare `.stderr`.
  Cargo dev-dependencies are not propagated downstream.

## Decisions

- Export one `macro_rules! scenario` from `src/scenario_macro.rs`; use reserved internal arms, not
  named helper macro exports.
- Use canonical top-level order: ID; optional title/description/tags/variables/metadata; nodes;
  edges; optional track; zero or more end statements.
- Use exact symbol spelling as IDs with separate node/edge registries and forward edge-target
  registration. Declared references clone retained IDs; undeclared references become the same
  typed spelling and flow to the builder's established missing-reference diagnostics. Dynamic IDs
  use the direct builder.
- Give each configured family native checked-field shorthand plus an exclusive typed `config`
  expression. Give transfers/connections/end conditions typed escape hatches.
- Use public family/connection constructors and setters only. Whole-graph semantics remain solely
  in `ScenarioBuilder::build`.
- Return `Result<Scenario, SetupError>` from a hygienic envelope; bind every expression once; ban
  macro-introduced panic/unwrap/expect/indexing/fixture IDs.
- Use focused `trybuild` snapshots, runtime equivalence, and a real offline Cargo rename fixture.
- Treat grammar, symbol mapping, evaluation count, result/error type, and export paths as public
  0.2 compatibility commitments.
- Reject all extra macros and leave normal assertion-function alternatives to another spec.

Rejected:

- T005-only predecessor, invalid cross-spec task frontmatter, partial-family implementation,
  independent DTO tags/payloads, private helpers, procedural macros, arbitrary top-level order,
  earlier-only state targets, implicit threshold scaling, and extra assertion macros.

Open:

- None. T001 stops if the completed T006 public API does not match the promised complete contract.

## Implementation shape excerpts

### Predecessor and orchestration

`spec.toml` depends on `039-checked-scenario-authoring` and uses
`human_checkpoint = "before-implementation"`. T001 has no rejected cross-spec `depends` syntax.
Before editing it reads `specs/039-checked-scenario-authoring/tasks/T006.md` and requires
`status = "done"` plus `verification_status = "passed"`; otherwise it stops. It then reads the
completed public checked source and maps only through those public constructors/defaults/setters.

### Macro implementation

Create `src/scenario_macro.rs` with one documented `#[macro_export] scenario!`. The entry matcher
uses `ident` symbols and `expr` values with legal delimiters. Reserved internal arms register typed
IDs, dispatch all 15 families/properties, all transfers/connections/targets, and recursive ends,
and produce focused `anapao::scenario!:` syntax diagnostics. Do not use unstable matcher features.

Expansion returns `Result<Scenario, SetupError>`, uses `$crate` for Anapao and absolute standard
paths, binds each expression once, uses fallible ID/nonzero conversion, calls public checked
constructors/setters, inserts through `ScenarioBuilder`, and finishes at `build`. It contains no
panic/unwrap/expect/index/private/raw-validation path.

The macro performs no missing-reference validation. A registry miss produces the typed symbol
spelling and lets `ScenarioBuilder::build` retain exact endpoint/metric/tracked/state-target/end
error variants, paths, reasons, and ordering.

The exact 039 calls are normative: every checked config starts from `Default`; Pool/Queue capacity
and Register min/max use their `with_*`/`without_*` pairs; delay/queue positive setters and resource
`with_token_size` accept `NonZeroU64`; all 15 `ScenarioNode` family constructors and node
`with_label`/`with_initial_value`/`with_tag`/`with_metadata` are public; state uses
`StateConnection::default` or `new(role, formula, target)` plus
`with_role`/`with_formula`/`with_target`/`with_resource_filter`; edges use
`ScenarioEdge::{resource,state}` plus `with_enabled`/`with_metadata`; all graph/scenario insertion
and fields use the exact `ScenarioBuilder` methods frozen in the predecessor shape.

### Compiler and runtime proof

Add `trybuild = "1"`, explicit pass/fail fixture paths and reviewed snapshots. Prove exact example,
full grammar, separators, no-prelude hygiene, prelude, alias, one evaluation, and targeted syntax
errors. Add a nested fixture whose dependency is named `simulation`, plus an offline Cargo-check
harness. Add runtime tests collectively covering every grammar surface, stable returned errors/no
unwind, direct-builder equality, and fixed-seed full report equality.

### Exports and docs

Expose canonical root and prelude paths; do not export from `types`. README/crate docs preserve DTO
and direct-builder routes, then teach exact and complete macro examples, error handling, grammar,
single-evaluation, builder-only validation, and 0.2 SemVer behavior. Snippet/doctests compile them.

## Suggested spec shape

- spec_kind: feature
- fanout_policy: serial
- execution_policy: auto-continue
- human_checkpoint: before-implementation
- commit_policy: after-validation
- review_policy: required
- depends_on: `039-checked-scenario-authoring`
- task_slices:
  - T001: implement the complete macro after exact 039/T006 guard (`sol`/`high`).
  - T002: add trybuild UI, snapshots, and real Cargo rename proof (`terra`/`high`), depends T001.
  - T003: add complete runtime/error/evaluation/builder/report equivalence (`terra`/`high`),
    depends T002.
  - T004: complete exports, rustdoc, README, snippets, migration/SemVer guidance
    (`terra`/`medium`), depends T003.
  - T005: independently review and close the public macro contract (`sol`/`high`, required),
    depends T004.

## Validation

```bash
cargo fmt --all -- --check
cargo test --test scenario_macro_ui
cargo test --test scenario_macro_renamed
cargo test --test scenario_macro
cargo test --test checked_scenario_authoring
cargo test --test readme_snippets
cargo test --doc
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
```

Snapshot generation is followed by diff review and a normal rerun. No Docker, external service, or
network is required; the nested Cargo rename fixture runs `--offline` with isolated target output.

## Worker context policy

- T001 may read completed: `specs/039-checked-scenario-authoring/tasks/T006.md`,
  `src/types/scenario_checked.rs`, `src/types/scenario.rs`, `src/types/mod.rs`, `src/error.rs`,
  `src/lib.rs`, and `src/prelude.rs`.
- T002 may read: `src/scenario_macro.rs`, `Cargo.toml`, existing integration-test conventions, and
  the concrete new UI/rename paths in its task.
- T003 may read: `src/scenario_macro.rs`, `tests/checked_scenario_authoring.rs`, `src/simulator.rs`,
  and the complete checked public types needed for direct-builder comparison.
- T004 may read: `src/scenario_macro.rs`, `src/lib.rs`, `src/prelude.rs`, `README.md`,
  `tests/readme_snippets.rs`, and runtime/UI examples.
- T005 may read every concrete in-scope source/test/doc/manifest path and completed task evidence.
- Workers must not be sent to raw research, prototypes, broad current-state/dialogue artifacts,
  `specs/index.md`, or `specs/_handoff.md`.
