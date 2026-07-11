# Implementation shape: scenario macro

## Ownership map

```text
src/scenario_macro.rs                              (new)
  Documented #[macro_export] scenario! definition
  Public entry matcher and reserved internal dispatch arms
  Complete node/edge/transfer/target/end desugaring

src/lib.rs
  Private module inclusion and crate-level macro documentation/example

src/prelude.rs
  Explicit scenario macro plus cohesive checked-authoring re-exports

Cargo.toml
Cargo.lock
  trybuild test-only dependency and locked resolution

tests/scenario_macro.rs                            (new)
  Runtime surface/error/evaluation/builder/report equivalence

tests/scenario_macro_ui.rs                         (new)
tests/ui/scenario_macro/pass/exact_example.rs      (new)
tests/ui/scenario_macro/pass/full_surface.rs       (new)
tests/ui/scenario_macro/pass/trailing_separators.rs (new)
tests/ui/scenario_macro/pass/hygiene_no_prelude.rs (new)
tests/ui/scenario_macro/pass/prelude_import.rs     (new)
tests/ui/scenario_macro/pass/crate_alias.rs        (new)
tests/ui/scenario_macro/pass/one_evaluation.rs     (new)
tests/ui/scenario_macro/fail/unknown_family.rs     (new)
tests/ui/scenario_macro/fail/unknown_family.stderr (new)
tests/ui/scenario_macro/fail/unknown_node_field.rs (new)
tests/ui/scenario_macro/fail/unknown_node_field.stderr (new)
tests/ui/scenario_macro/fail/wrong_family_field.rs (new)
tests/ui/scenario_macro/fail/wrong_family_field.stderr (new)
tests/ui/scenario_macro/fail/mixed_config.rs       (new)
tests/ui/scenario_macro/fail/mixed_config.stderr   (new)
tests/ui/scenario_macro/fail/duplicate_scalar.rs   (new)
tests/ui/scenario_macro/fail/duplicate_scalar.stderr (new)
tests/ui/scenario_macro/fail/malformed_transfer.rs (new)
tests/ui/scenario_macro/fail/malformed_transfer.stderr (new)
tests/ui/scenario_macro/fail/malformed_state_target.rs (new)
tests/ui/scenario_macro/fail/malformed_state_target.stderr (new)
tests/ui/scenario_macro/fail/malformed_end.rs      (new)
tests/ui/scenario_macro/fail/malformed_end.stderr  (new)
tests/ui/scenario_macro/fail/unexpected_top_level.rs (new)
tests/ui/scenario_macro/fail/unexpected_top_level.stderr (new)
  Compiler pass/fail, diagnostics, hygiene, alias, and single-evaluation proof

tests/scenario_macro_renamed.rs                    (new)
tests/fixtures/scenario-macro-renamed/Cargo.toml   (new)
tests/fixtures/scenario-macro-renamed/.gitignore   (new)
tests/fixtures/scenario-macro-renamed/src/main.rs  (new)
  Real Cargo dependency rename compiled offline with isolated target output

README.md
tests/readme_snippets.rs
  Recommended exact example, complete cookbook, compatibility/error guidance
```

No source file outside this map is required unless the completed T006 predecessor changed a public
name promised by spec 039. That condition is an escalation, not permission to edit checked types.

## Predecessor guard and discovery

Before T001 edits anything:

1. Read `specs/039-checked-scenario-authoring/tasks/T006.md`.
2. Require exactly `status = "done"` and `verification_status = "passed"`.
3. Read the completed public API in `src/types/scenario_checked.rs`, `src/types/mod.rs`,
   `src/lib.rs`, and `src/prelude.rs`.
4. Confirm T006 left the promised complete constructors/defaults/setters for all families,
   connections, common fields, and builder sections.
5. Stop if the guard or API contract differs. Do not add private-field access, a hidden raw
   constructor, or source edits under spec 039.

The active spec uses `depends_on = ["039-checked-scenario-authoring"]` and
`human_checkpoint = "before-implementation"`. T001 intentionally has no cross-spec `depends`
entry because the current validator rejects that documented syntax; its Context and Escalate If
carry the exact completion guard.

## Public macro contract

The root expression has this durable type:

```rust
Result<anapao::types::Scenario, anapao::error::SetupError>
```

The canonical path is `anapao::scenario!`; `use anapao::prelude::*` also imports `scenario!` and
the checked types needed for direct-builder comparison. The macro is not exported from
`anapao::types`.

The complete supported grammar is the one frozen in `04-design-decisions.md` and copied into the
spec tasks. The exact intake example is a required compile/run test, not illustrative pseudocode.
All lists/property blocks accept optional trailing commas. Scalar/declaration/end statements keep
their documented semicolons.

The public macro docs must name these compatibility guarantees:

- symbol spelling maps exactly through `stringify!` to node/edge IDs;
- node and edge symbol namespaces are distinct;
- each captured expression is evaluated once;
- syntactic misuse is a compile error; semantic invalidity is `Err(SetupError)`;
- the macro adds no serde representation and invokes the checked builder's sole graph gate;
- no macro-introduced panic occurs;
- supported grammar/return/export behavior is public 0.2 API.

## Normative checked API desugaring

Spec 039 has frozen the exact spellings below in
`.orchid/spec-research/checked-scenario-authoring/05-implementation-shape.md:114-173`,
`:179-210`, `:222-290`, and `:301-321`. T001 consumes this contract verbatim after T006 is done and
passed:

```rust
ScenarioBuilder::new(id)
ScenarioBuilder::{insert_node, insert_edge}
ScenarioBuilder::{with_title, with_description, with_tag, with_variables}
ScenarioBuilder::{with_end_condition, with_end_conditions, push_end_condition}
ScenarioBuilder::{with_tracked_metric, with_metadata, build}

ScenarioNode::{source, pool, drain, sorting_gate, trigger_gate, mixed_gate}
ScenarioNode::{converter, trader, register, delay, queue, process, sink, gate, custom}
ScenarioNode::{with_label, with_initial_value, with_tag, with_metadata}

ScenarioEdge::{resource, state}
ScenarioEdge::{with_enabled, with_metadata}

PoolConfig, DrainConfig, SortingGateConfig, TriggerGateConfig, MixedGateConfig
ConverterConfig, TraderConfig, RegisterConfig, DelayConfig, QueueConfig
ResourceConnection, StateConnection, StateTarget
```

The exact checked config/connection calls available to desugaring are:

```text
PoolConfig::default()
  .with_capacity(u64) / .without_capacity()
  .with_allow_negative_start(bool)
  .with_mode(NodeModeConfig)
DrainConfig::default().with_mode(NodeModeConfig)
SortingGateConfig::default().with_mode(NodeModeConfig)
TriggerGateConfig::default().with_mode(NodeModeConfig)
MixedGateConfig::default().with_mode(NodeModeConfig)
ConverterConfig::default()
  .with_ignore_disabled_inputs(bool).with_mode(NodeModeConfig)
TraderConfig::default()
  .with_ignore_disabled_inputs(bool).with_mode(NodeModeConfig)
RegisterConfig::default()
  .with_interactive(bool)
  .with_min_value(i64) / .without_min_value()
  .with_max_value(i64) / .without_max_value()
DelayConfig::default()
  .with_delay_steps(NonZeroU64).with_mode(NodeModeConfig)
QueueConfig::default()
  .with_capacity(NonZeroU64) / .without_capacity()
  .with_release_per_step(NonZeroU64).with_mode(NodeModeConfig)

ResourceConnection::default().with_token_size(NonZeroU64)
StateConnection::default()
StateConnection::new(StateConnectionRole, impl Into<String>, StateTarget)
StateConnection::{with_role, with_formula, with_target, with_resource_filter}
```

Every consuming setter returns `Self` and is `#[must_use]`. The macro uses `Default` plus these exact
setters for native fields or accepts the typed checked config/connection expression. It must never
construct `NodeBehavior`/`ConnectionSpec` with separately selected payloads and must never read or
mutate checked private fields. No macro-specific helper constructor/setter is permitted.

The macro may construct stable DTO leaf values that the checked public API explicitly consumes:

- `NodeModeConfig`, `TriggerMode`, and `ActionMode` for mode shorthands;
- all `TransferSpec` variants;
- `VariableRuntimeConfig` pass-through;
- all `EndConditionSpec` variants;
- validated `ScenarioId`, `NodeId`, `EdgeId`, and `MetricKey` values.

## Matcher architecture

`src/scenario_macro.rs` contains one `#[macro_export] macro_rules! scenario` definition:

1. A public entry arm captures canonical top-level sections with narrow fragments.
2. Reserved `@__anapao_register_nodes` and `@__anapao_register_edges` arms populate separate
   hygienic registries once.
3. Family dispatch arms construct all 15 node families and recursively apply allowed properties.
   State markers in the matcher reject duplicate scalar fields, `config` plus shorthands,
   `mode` plus trigger/action, unknown fields, and wrong-family fields.
4. Transfer dispatch covers all six documented forms.
5. Resource/state dispatch applies checked connection construction and common edge properties.
6. Recursive end dispatch covers all leaf/composite/pass-through forms.
7. Focused fallback arms emit stable `compile_error!` text beginning `anapao::scenario!:` for
   syntax the macro owns.

Use `ident` for symbols and field/family keywords and `expr` for caller values. Use `tt` only for
recursive property/end token streams after the public entry has established unambiguous delimiters.
Expressions are always followed by `,`, `;`, `=>`, `)`, `]`, or `}` as allowed by the Rust
Reference. Do not use 2024-only `expr_2021`/metavariable-expression features; the definition must
compile on edition 2021/MSRV 1.85.

Only `scenario!` is documented and named publicly. Do not add `__scenario_node!` or other exported
helper macro names. The reserved dispatch prefix is never accepted as documented user input and
has a compile-fail guard at the public boundary where practical.

## State, ID, and expression ownership

The expansion is a hygienic block/closure returning `Result<Scenario, SetupError>`.

- Capture the scenario ID expression exactly once; convert through `TryInto<ScenarioId>` and map a
  displayable conversion error to `SetupError::InvalidParameter { name: "id", ... }`.
- For each node declaration, call `NodeId::new(stringify!(symbol))` once and retain the result plus
  its node-backed `MetricKey` in a local node-symbol registry.
- For each edge declaration, call `EdgeId::new(stringify!(symbol))` once and retain the result in a
  separate edge-symbol registry before constructing any edge.
- Resolve declared endpoints, metrics, tracking, end symbols, and state targets by cloning retained
  typed IDs with fully qualified paths/traits where `no_implicit_prelude` needs them. If a reference
  is undeclared, construct the same typed spelling fallibly and pass it to the checked value so
  `ScenarioBuilder::build` emits its established missing-reference diagnostic. The macro must not
  define new unresolved-symbol variants, paths, reasons, or ordering.
- Populate nodes and edges in declaration order. `ScenarioBuilder` still owns duplicate and graph
  semantics; the registries only implement macro name resolution and forward target references.
- Bind every `$expr` once before conversion or use. An expression must not appear both in a match
  success branch and an error formatter.

Node/edge symbolic spelling is Rust syntax and therefore cannot be blank or contain controls, but
the expansion still uses fallible `new`, not `fixture` or an infallible assertion. Node/edge
namespace collision is valid because registries are separate. Duplicate declarations reach the
builder's stable duplicate policy.

## Error and panic contract

Macro-owned runtime conversions return `SetupError`:

| Input | Error path/reason |
| --- | --- |
| invalid scenario ID | `id`; preserve `IdentifierError` display text |
| zero Delay `steps` | `nodes.<id>.config.delay_steps`; `must be greater than 0` |
| zero Queue `release_per_step` | `nodes.<id>.config.release_per_step`; `must be greater than 0` |
| zero present Queue `capacity` | `nodes.<id>.config.capacity`; established present-capacity reason |
| zero resource `token_size` | `edges.<id>.connection.resource.token_size`; `must be greater than 0` |

Fraction denominator remains a `TransferSpec` value and reaches the sole checked build gate, which
already owns its exact semantic error path. Typed `config`, `connection`, `transfer`, `variables`,
and `condition` expressions are not revalidated by macro-specific code.

The expansion must not contain `panic!`, `unwrap`, `expect`, indexing, `unreachable!`, fixture
constructors, or unchecked `Option` extraction. This guarantee excludes panics deliberately
performed inside a caller-supplied expression; the macro evaluates such an expression once and
does not attempt to catch user code.

## Export and documentation integration

- `src/lib.rs` declares the implementation module and documents the macro in the crate-level
  checked-authoring flow. `#[macro_export]` places it at the root.
- `src/prelude.rs` uses `pub use crate::scenario;` alongside the complete checked-authoring facade
  chosen by T006.
- README preserves legacy DTO authoring and direct checked builder docs, then presents `scenario!`
  as equivalent sugar over the builder. It includes the exact intake example and one complete
  example without eliding families/surfaces as `...`.
- `tests/readme_snippets.rs` compiles the recommended macro path and compares it with a checked
  builder result. Rustdoc examples cover normal `?` use and handled `Err` use.
- Docs explicitly reject `expectations!`/assertion macros and point to normal function APIs as the
  preferred future direction without implementing those functions here.

## Test seam map

### Runtime integration: `tests/scenario_macro.rs`

Use multiple small valid scenarios rather than forcing all node families into one graph that
violates family-specific connection rules. Collectively prove:

- all 15 node families and every family config/common property;
- every transfer, resource/state connection field, and all state targets including forward refs;
- scenario title/description/tags/variables/metadata, track, every leaf end condition, recursive
  Any/All, multiple top-level ends, and default end behavior;
- dynamic checked config/connection/transfer/condition escape hatches;
- exact symbolic IDs, distinct node/edge namespaces, and builder duplicate errors;
- invalid ID/nonzero/reference/graph inputs return `Err` and do not unwind;
- side-effect counters observe exactly one evaluation per expression category;
- an equivalent direct `ScenarioBuilder` graph equals the macro-built `Scenario`;
- compiling/running both under one explicit seed/config produces equal full `RunReport`s.

### Compiler UI: `tests/scenario_macro_ui.rs`

Instantiate `trybuild::TestCases` with each concrete pass/fail path in the ownership map; do not use
a broad source-doc aggregate as fixture input. Pass binaries execute, so `one_evaluation.rs`
asserts counters at runtime as well as compiling.

`hygiene_no_prelude.rs` uses `#![no_implicit_prelude]`, imports only the macro path required to
invoke it, and deliberately declares caller locals resembling internal names. `crate_alias.rs`
invokes through an alias. `prelude_import.rs` proves the intended wildcard prelude path.

Compile-fail snapshots cover only intentional grammar diagnostics. Each `.stderr` must point near
the offending token and include the `anapao::scenario!:` prefix. Generate snapshots through
`TRYBUILD=overwrite cargo test --test scenario_macro_ui`, inspect the diff, then rerun normally.

### Real dependency rename

`tests/fixtures/scenario-macro-renamed/Cargo.toml` declares:

```toml
[dependencies]
simulation = { package = "anapao", path = "../../.." }
```

Its binary invokes `simulation::scenario!` without an `anapao` crate name. The integration harness
runs `cargo check --offline --manifest-path ...` with an isolated target directory and asserts
success. The fixture ignores its generated lock/target artifacts. This catches a literal crate
name in expansion that a Rust `use` alias alone would not prove.

### Docs and full gates

Run doctests and README snippet tests, then clippy/all-target gates. UI snapshots and the nested
rename check run without network once dependencies are present. No Docker or external service is
required.

## Vertical task slices

### T001 — Complete macro implementation

After the exact 039/T006 guard, implement all grammar/desugaring/error/export mechanics in
`src/scenario_macro.rs` and the minimum `src/lib.rs` module inclusion. This task is complete only
with every family/surface; it is not a four-family tracer or MVP.

### T002 — Compiler UI and real rename proof

Depends on T001. Add the manifest dependency, lock update, trybuild harness/pass/fail/snapshots, and
the real dependency-renaming fixture/harness. It may correct narrow macro diagnostic/hygiene defects
found by UI tests within `src/scenario_macro.rs`.

### T003 — Runtime and equivalence proof

Depends on T002 so it tests the final diagnostic/hygiene-correct expansion. Add full surface,
single-evaluation, error/no-unwind, direct-builder equality, and fixed-seed report equality in
`tests/scenario_macro.rs`. It may correct narrow semantic expansion defects in the macro file.

### T004 — Public exports and complete documentation

Depends on T003. Complete prelude/root docs, README, rustdoc, and snippet tests. Preserve DTO and
direct-builder paths. Do not add extra macros or assertion function implementations.

### T005 — Independent public macro review

Depends on T004. A fresh Sol/high validator reviews every grammar category, `$crate`/rename/hygiene,
one-evaluation, no-panic/error layering, builder-only semantic flow, diagnostics, docs, Cargo/MSRV,
and all gates. It may make narrow corrections but must refuse completion for unresolved API or
coverage gaps.

## Validation commands

Focused and full validation, in order:

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

UI snapshot creation only, not a final green command:

```bash
TRYBUILD=overwrite cargo test --test scenario_macro_ui
git diff -- tests/ui/scenario_macro
```

No approved network or Docker access is required for implementation validation. The renamed Cargo
fixture explicitly runs offline.

## Constraints and anti-goals

- Preserve MSRV 1.85, edition 2021, `#![forbid(unsafe_code)]`, BTree determinism, and spec-039
  duplicate/error semantics.
- Preserve all legacy DTO and direct checked-builder public routes.
- Do not access checked private fields, raw validation, execution plan, engine, or preserved DTO
  maps from macro expansion.
- Do not add a second error enum, serde format, procedural macro crate, dynamic identifier
  concatenation dependency, or named helper macro export.
- Do not ship partial node/edge/config/metadata/end/track coverage.
- Do not add `expectations!`, assertion macros, or opportunistic assertion function APIs.
- Do not accept syntax that is not documented and UI-tested; do not snapshot arbitrary type
  mismatch diagnostics.

## Escalation and stop conditions

Stop and escalate if:

- `039-checked-scenario-authoring/T006` is not done and passed;
- T006's public API lacks a complete constructor/default/setter needed by the frozen grammar;
- a required expansion path would need private visibility or raw validation;
- preserving one evaluation or no-panic behavior would require dropping a documented form;
- MSRV 1.85 cannot express the matcher/expansion without unstable features;
- actual Cargo dependency renaming fails even though alias-only tests pass;
- a complete direct-builder-equivalent scenario produces a different checked value or report;
- an extra macro or partial family set appears necessary.
