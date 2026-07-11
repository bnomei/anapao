# Current state: scenario macro

Research snapshot: 2026-07-11. Source paths below describe the checked-in 0.1.1 tree plus the
frozen, not-yet-executed spec-039 public contract in the shared worktree.

## Macro and package state

- `Cargo.toml:1-14` names package `anapao` version `0.1.1`, edition 2021, with MSRV 1.85.
- `Cargo.toml:40-43` contains Criterion, rstest, and tempfile dev-dependencies. `trybuild` is not
  present.
- `src/types/identifiers.rs:21-85` contains one private `define_identifier!` implementation macro.
  Repository search found no `#[macro_export]` and no public macro UI harness.
- `src/lib.rs:123-150` exposes public modules and a concise root facade. `src/prelude.rs:1-12`
  re-exports common compile/run/assert types. Neither exports a macro.
- `src/lib.rs:152-154` includes `README.md` as a doctest module. `tests/readme_snippets.rs` pins
  selected public examples as integration code.

## Existing scenario vocabulary

- `src/types/scenario.rs:14-34` defines 15 node families: Source, Pool, Drain, SortingGate,
  TriggerGate, MixedGate, Converter, Trader, Register, Delay, Queue, Process, Sink, Gate, and
  Custom.
- `src/types/scenario.rs:36-75` defines trigger/action modes and their shared config.
- `src/types/scenario.rs:77-201` defines family config DTOs for Pool, Drain, SortingGate,
  TriggerGate, MixedGate, Converter, Trader, Register, Delay, and Queue. Source, Process, Sink,
  Gate, and Custom have no config variant.
- `src/types/scenario.rs:203-212` defines Fixed, Fraction, Remaining, MetricScaled, and Expression
  transfers.
- `src/types/scenario.rs:214-225` defines MaxSteps, metric/node lower and upper bounds, and recursive
  Any/All end conditions.
- `src/types/scenario.rs:227-254` defines variable update timing and constant, interval, list, and
  matrix sources inside `VariableRuntimeConfig`.
- `src/types/scenario.rs:256-344` defines resource and state connections, state roles, four target
  kinds, formula, optional target ID, and optional resource filter.
- `src/types/scenario.rs:353-404` gives nodes common label, initial value, tags, and metadata, and
  gives edges transfer, connection, enabled, and metadata fields.
- `src/types/scenario.rs:460-473` gives scenarios ID, title, description, tags, nodes, edges,
  variables, ordered end conditions, tracked metrics, and metadata.

## Existing defaults, IDs, and validation

- `src/types/scenario.rs:475-489` starts a scenario with empty collections, default variables, and
  one `MaxSteps { steps: 1 }` end condition.
- `src/types/scenario.rs:551-560` makes legacy DTO node/edge consuming helpers replace existing map
  entries with the same ID.
- `src/types/identifiers.rs:28-39` validates IDs through `new` and returns `IdentifierError` for
  blank/control values. `src/types/identifiers.rs:41-44` provides fixture constructors that panic
  on invalid values.
- `src/error.rs:17-26` exposes `SetupError::{InvalidGraphReference, CyclicGraph,
  InvalidParameter}`; there is no macro- or builder-specific public error enum.
- `src/validation/mod.rs:415-436` rejects zero resource token size and fraction denominator at
  `edges.<id>.connection.resource.token_size` and
  `edges.<id>.transfer.fraction.denominator` with “must be greater than 0.”
- `src/validation/mod.rs:795-833` rejects zero delay, queue release, and present queue capacity at
  their `nodes.<id>.config.*` paths.
- `src/validation/mod.rs:249-300` resolves tracked metrics, metric-scaled transfers, and metric end
  conditions against node IDs.
- `src/engine/mod.rs:1985-2017` evaluates the scenario's top-level end-condition vector with OR,
  then evaluates explicit nested Any/All variants recursively.

## Frozen checked-authoring predecessor

- `specs/039-checked-scenario-authoring/design.md#Architecture` freezes `Scenario` as immutable,
  checked, non-serde state and `ScenarioBuilder` as the complete public checked authoring path.
- `specs/039-checked-scenario-authoring/tasks/T001.md` requires complete public
  `NodeBehavior`, `ConnectionSpec`, `StateTarget`, checked configs, family-specific
  `ScenarioNode` constructors, and resource/state `ScenarioEdge` constructors with private fields.
- `specs/039-checked-scenario-authoring/tasks/T002.md` requires mutable and consuming insertion,
  title, description, tag, variables, end condition(s), tracked metric, metadata, and `build`, all
  returning or preserving `SetupError` as specified.
- `.orchid/spec-research/checked-scenario-authoring/05-implementation-shape.md#Public-checked-contracts`
  freezes exact checked config defaults and field-named setters, including optional-value
  `without_*` methods and positive `NonZeroU64` setters; exact family constructors; node
  `with_label`/`with_initial_value`/`with_tag`/`with_metadata`; resource
  `with_token_size`; state `new`/`with_role`/`with_formula`/`with_target`/
  `with_resource_filter`; and edge `resource`/`state`/`with_enabled`/`with_metadata`.
- `specs/039-checked-scenario-authoring/tasks/T005.md` publishes the checked API and migration
  documentation. `specs/039-checked-scenario-authoring/tasks/T006.md` then independently reviews
  and may remediate every public API, serde, plan, and engine invariant. T006 is therefore the final
  complete predecessor gate.
- `specs/039-checked-scenario-authoring/design.md#Scope` explicitly excludes `scenario!` and states
  that spec 040 consumes the completed public API.

## Current tests and validation conventions

- `tests/parity/differential.rs` and `tests/perf_determinism.rs` build broad scenario families and
  exercise deterministic behavior.
- Spec 039 adds `tests/checked_scenario_authoring.rs` to compare DTO and checked-builder scenarios,
  compile facades, and equal full reports under an explicit seed.
- Repository instructions require `cargo fmt --all -- --check`,
  `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-targets`.
- No Docker-hosted or network service is involved in scenario compilation or macro tests.

## External source facts

- The Rust Reference states that macro-by-example has mixed-site hygiene, `$crate` identifies the
  defining crate, fully qualified paths are required for non-macro items, and visibility still
  applies: https://doc.rust-lang.org/reference/macros-by-example.html.
- The Rust Reference limits tokens that may follow `expr` fragments and applies those rules to
  repetitions/separators: https://doc.rust-lang.org/reference/macro-ambiguity.html.
- The Rust API Guidelines say macro input should be evocative of output and use familiar Rust
  syntax: https://rust-lang.github.io/api-guidelines/macros.html.
- `trybuild` pass cases compile and run; compile-fail cases compare adjacent `.stderr` snapshots;
  dev-dependencies are available to cases: https://docs.rs/trybuild/latest/trybuild/.
- Cargo documents that dev-dependencies build tests/examples/benchmarks and are not propagated to
  downstream packages:
  https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#development-dependencies.
- Cargo evaluates API compatibility by whether downstream usage keeps compiling:
  https://doc.rust-lang.org/cargo/reference/semver.html.
- The Rust Reference states that `#[track_caller]` propagates caller location through attributed
  calls: https://doc.rust-lang.org/reference/attributes/codegen.html#the-track_caller-attribute.
- The Rust Book describes `Result` as the default for expected/recoverable library failure:
  https://doc.rust-lang.org/book/ch09-03-to-panic-or-not-to-panic.html.
