# Design — 040 Complete Checked Scenario Macro

## Objective

Add one public, declarative `scenario!` macro whose symbolic graph syntax desugars exclusively to
the complete checked authoring API from spec 039. The macro is an ergonomic source form, not a
second model or validator: all invariant-bearing values use checked constructors/setters and the
finished graph passes through `ScenarioBuilder::build`.

## Scope

This spec owns:

- the complete public macro grammar and expansion;
- crate-root/prelude exposure and public 0.2 compatibility documentation;
- compiler UI, hygiene, expression-evaluation, real Cargo rename, runtime error, direct-builder,
  deterministic report, rustdoc, and README proof;
- an independent Sol/high public macro/API review.

It does not own:

- any implementation surface in spec 039 or access to its private checked/AST/plan state;
- a new serde representation, validation engine, error enum, runtime behavior, or dynamic-ID DSL;
- `expectations!`, assertion/config/report/artifact macros, a procedural macro crate, or assertion
  function ergonomics.

## Distilled current-state facts

- `Cargo.toml:1-14` defines package `anapao` 0.1.1, edition 2021, MSRV 1.85;
  `Cargo.toml:40-43` currently has no `trybuild`.
- `src/types/scenario.rs:14-201` defines all 15 node families and family DTO fields;
  `src/types/scenario.rs:203-344` defines all transfer, end, variable, connection, and target
  variants; `src/types/scenario.rs:353-473` defines common node/edge/scenario fields.
- `src/types/identifiers.rs:28-44` provides fallible `new` and panic-based `fixture`; public macro
  expansion must use the fallible route.
- `src/error.rs:17-26` exposes the established `SetupError` taxonomy.
- `src/validation/mod.rs:249-300` resolves metrics to nodes. `src/engine/mod.rs:1985-2017`
  evaluates top-level end conditions with OR and recursive Any/All explicitly.
- `src/lib.rs:123-150` and `src/prelude.rs:1-12` own the current concise facade. The only existing
  `macro_rules!` is a private identifier implementation macro; there is no exported macro/UI
  harness.
- Spec 039 makes `Scenario`, checked configs, nodes, edges, and builder public while keeping fields,
  expression carrier, plan, validation, and engine ownership private. Its T006 is the independent
  review/remediation gate for the complete public contract.

## Hard predecessor guard

`spec.toml.depends_on` orders this spec after `039-checked-scenario-authoring`, and
`human_checkpoint = "before-implementation"` prevents unattended implementation start. Before
T001 edits anything, it reads `specs/039-checked-scenario-authoring/tasks/T006.md` and requires
exactly:

```toml
status = "done"
verification_status = "passed"
```

T005 in that spec publishes the API, but T006 independently reviews and may remediate the whole
contract; therefore T006 is the true no-MVP predecessor.

The current task-schema validator rejects cross-spec task IDs in task frontmatter, despite the
documented intended syntax. T001 deliberately keeps `depends = []`; spec-level dependency,
checkpoint, and the exact Context/Escalate If status guard are the validator-supported fallback.
A targeted `orchid ready --spec 040-scenario-macro` must not bypass the human guard.

## Public grammar

### Canonical top-level order

```rust
scenario! {
    id: scenario_id_expr;
    title: title_expr;                         // optional
    description: description_expr;             // optional
    tags [tag_expr, another_tag_expr,];         // optional
    variables: variable_runtime_config_expr;   // optional typed pass-through
    metadata { key_expr => value_expr, }        // optional

    nodes { /* declarations */ }
    edges { /* declarations */ }

    track [node_symbol, another_symbol,];       // optional
    end max_steps(step_expr);                  // zero or more
}
```

Top-level order is fixed to keep one unambiguous entry matcher and focused diagnostics. Property
and list repetitions accept optional trailing commas; scalar/declaration/end statements use
semicolons. Metadata uses `=>` because it is a legal expression follow token.

`id:` accepts `TryInto<ScenarioId>` (`&str`, `String`, or checked ID). Node and edge declaration
names are Rust `ident` fragments; exact `stringify!` spelling is the generated ID. Node and edge
symbols occupy separate namespaces. Dynamic IDs remain available through the direct builder.

### Nodes

All families are supported:

- Source, Process, Sink, Gate;
- `Custom(family_expr)`;
- Pool, Drain, SortingGate, TriggerGate, MixedGate, Converter, Trader, Register, Delay, Queue.

Common node fields are `label`, `initial`, `tags`, and `metadata`. Configured families support:

| Family | Native fields |
| --- | --- |
| Pool | `capacity`, `allow_negative_start`, `mode` or `trigger`/`action` |
| Drain | `mode` or `trigger`/`action` |
| SortingGate | `mode` or `trigger`/`action` |
| TriggerGate | `mode` or `trigger`/`action` |
| MixedGate | `mode` or `trigger`/`action` |
| Converter | `ignore_disabled_inputs`, `mode` or `trigger`/`action` |
| Trader | `ignore_disabled_inputs`, `mode` or `trigger`/`action` |
| Register | `interactive`, `min_value`, `max_value` |
| Delay | `steps`, `mode` or `trigger`/`action` |
| Queue | `capacity`, `release_per_step`, `mode` or `trigger`/`action` |

Each configured family also accepts exclusive `config: <checked config expr>`. Pool/Queue
`capacity` and Register min/max accept a value or literal `none`; omission starts from `Default`.
`config` cannot mix with config shorthands, and `mode` cannot mix with trigger/action. Scalar
duplicates, unknown fields, and wrong-family fields are compile-time syntax errors.

The exact intake forms remain valid:

```rust
source: Source { initial: 64.0 };
delay: Delay { steps: 2 };
sink: Pool;
```

### Transfers, edges, and targets

Every `TransferSpec` form has syntax:

```text
fixed(amount)
fraction(numerator, denominator)
remaining
metric_scaled(node_symbol, factor)
expression(formula)
transfer(transfer_spec_expr)
```

No connection suffix means default resource. A `resource { ... }` block accepts either exclusive
`connection: ResourceConnection` or `token_size`, plus `enabled` and `metadata`. A
`state { ... }` block accepts either exclusive `connection: StateConnection` or native `role`,
`formula`, `target`, and `resource_filter`, plus `enabled` and `metadata`.

State targets cover:

```text
node
resource_connection(edge_symbol)
state_connection(edge_symbol)
formula(edge_symbol)
```

All edge IDs are registered before edge construction, so forward targets work. Every edge still
uses `ScenarioEdge::resource`/`state` and `ScenarioBuilder::insert_edge`.

### Scenario fields, tracking, and ends

Title, description, tags, variables, and metadata call their matching builder methods. Variables
are a typed `VariableRuntimeConfig` pass-through because the predecessor intentionally exposes one
builder field, not a second variable DSL.

`track [node, ...]`, `metric_scaled`, and metric end conditions derive `MetricKey` from retained
node symbols, matching the current node-backed metric contract.

Every end variant is supported:

```text
max_steps(steps)
metric_at_least(node_symbol, scaled_value)
metric_at_most(node_symbol, scaled_value)
node_at_least(node_symbol, scaled_value)
node_at_most(node_symbol, scaled_value)
any [<condition>, ...]
all [<condition>, ...]
condition(end_condition_spec_expr)
```

Multiple top-level `end` statements become one ordered `with_end_conditions` list and preserve
current OR semantics. No `end` call preserves the builder default MaxSteps(1). Thresholds remain
the existing scaled i64 contract; the macro does not invent implicit f64 scaling.

## Normative checked API calls

The predecessor freezes the exact desugaring surface. T001 may call only the following public
contracts and normal DTO leaf constructors.

### Configs

```text
PoolConfig::default()
  .with_capacity(u64) / .without_capacity()
  .with_allow_negative_start(bool).with_mode(NodeModeConfig)
DrainConfig::default().with_mode(NodeModeConfig)
SortingGateConfig::default().with_mode(NodeModeConfig)
TriggerGateConfig::default().with_mode(NodeModeConfig)
MixedGateConfig::default().with_mode(NodeModeConfig)
ConverterConfig::default().with_ignore_disabled_inputs(bool).with_mode(NodeModeConfig)
TraderConfig::default().with_ignore_disabled_inputs(bool).with_mode(NodeModeConfig)
RegisterConfig::default()
  .with_interactive(bool)
  .with_min_value(i64) / .without_min_value()
  .with_max_value(i64) / .without_max_value()
DelayConfig::default().with_delay_steps(NonZeroU64).with_mode(NodeModeConfig)
QueueConfig::default()
  .with_capacity(NonZeroU64) / .without_capacity()
  .with_release_per_step(NonZeroU64).with_mode(NodeModeConfig)
```

### Nodes, connections, and edges

```text
ScenarioNode::{source,pool,drain,sorting_gate,trigger_gate,mixed_gate}
ScenarioNode::{converter,trader,register,delay,queue,process,sink,gate,custom}
ScenarioNode::{with_label,with_initial_value,with_tag,with_metadata}

ResourceConnection::default().with_token_size(NonZeroU64)
StateConnection::default()
StateConnection::new(StateConnectionRole, impl Into<String>, StateTarget)
StateConnection::{with_role,with_formula,with_target,with_resource_filter}

ScenarioEdge::{resource,state}
ScenarioEdge::{with_enabled,with_metadata}
```

### Builder

```text
ScenarioBuilder::new
ScenarioBuilder::{insert_node,insert_edge}
ScenarioBuilder::{with_title,with_description,with_tag,with_variables}
ScenarioBuilder::{with_end_condition,with_end_conditions,push_end_condition}
ScenarioBuilder::{with_tracked_metric,with_metadata,build}
```

Every checked consuming setter is `#[must_use]` and returns `Self`. Native mode shorthand creates
the public `NodeModeConfig` DTO; transfer/end/variable escape hatches use their existing public DTO
leaf types. No expansion constructs independent `NodeBehavior`/`ConnectionSpec` payloads, accesses
private fields, calls raw validation, or sees the predecessor's private `ValidatedExpressions`.

## Macro implementation

`src/scenario_macro.rs` contains one documented
`#[macro_export] macro_rules! scenario`:

1. The public entry matcher captures canonical sections with narrow fragments.
2. Reserved `@__anapao_register_nodes`/`@__anapao_register_edges` arms create separate typed ID
   registries once.
3. Family/property arms construct all node variants and track seen fields so duplicate/mixed/
   wrong-family forms get targeted diagnostics.
4. Transfer and connection arms cover every native and typed escape form.
5. Recursive end arms cover every leaf/composite/pass-through form.
6. Focused fallbacks emit `compile_error!` text beginning `anapao::scenario!:`.

Use `ident` for symbols/keywords and `expr` for values. Use `tt` only inside already-delimited
recursive property/end streams. Matchers use legal expression follow tokens. The implementation
must remain edition 2021/MSRV 1.85 and use no unstable macro metavariable features.

Only `scenario!` is a documented/named public macro. Do not export named helper macros. Reserved
internal dispatch arms are not documented user grammar.

## Expansion data flow

```text
scenario id expression (one evaluation)
  -> TryInto<ScenarioId>
  -> ScenarioBuilder::new

node declarations
  -> NodeId::new(stringify!(symbol)) once each
  -> separate hygienic node registry

edge declarations
  -> EdgeId::new(stringify!(symbol)) once each
  -> separate hygienic edge registry before construction

property expressions (one binding each)
  -> checked Default / exact public setters
  -> family ScenarioNode constructors
  -> ScenarioBuilder::insert_node
  -> ScenarioEdge::resource/state
  -> ScenarioBuilder::insert_edge

scenario fields / track / end list
  -> exact ScenarioBuilder methods
  -> ScenarioBuilder::build
  -> Result<Scenario, SetupError>
```

All Anapao paths use `$crate`; standard paths are absolute. Registry lookups use checked branching,
not indexing. The result envelope handles internal early errors without requiring the caller
function to return `SetupError`.

Every captured expression appears once in emitted executable code before conversion/use. This
includes ID, scenario fields, node properties/configs, custom family, transfers, connection
fields, edge fields, ends, and typed escape hatches. Symbol `ident` tokens are not expressions;
declarations create their typed IDs once and references clone retained values.

If a node/edge/metric reference is absent from its declaration registry, resolve the exact
`stringify!` spelling into the corresponding typed ID and pass it into the checked node, edge, end,
or tracking value. `ScenarioBuilder::build` then emits its existing missing endpoint, metric,
tracked, state-target, or end-reference error with unchanged variant/path/reason/order. The macro
owns no separate unresolved-symbol validation or error semantics.

## Errors and panic boundary

Syntax misuse is compile-time. Macro-owned diagnostics are focused and carry the stable
`anapao::scenario!:` prefix. Only intentional grammar messages are snapshot-stable; arbitrary Rust
type errors are not exhaustively frozen.

Runtime conversion/semantic errors use existing `SetupError`:

- invalid scenario ID -> `InvalidParameter` at `id` with identifier reason;
- zero Delay steps -> `nodes.<id>.config.delay_steps`;
- zero Queue release/present capacity -> established `nodes.<id>.config.*` path/reason;
- zero resource token size -> `edges.<id>.connection.resource.token_size`;
- duplicates and whole-graph semantics -> unchanged builder insertion/build error;
- unresolved references -> unchanged checked-builder endpoint/metric/tracked/target/end error;
- zero fraction denominator -> unchanged checked-build transfer error.

The expansion contains no `panic!`, `unwrap`, `expect`, `unreachable!`, indexing, fixture ID, or
unchecked extraction. A panic deliberately executed by a caller expression remains caller code;
the macro evaluates it once and does not catch it.

## Export and documentation

- `src/scenario_macro.rs` owns macro docs/definition; `src/lib.rs` includes the module and teaches
  the macro inside the checked-authoring flow.
- `#[macro_export]` provides `anapao::scenario!`.
- `src/prelude.rs` explicitly re-exports `scenario` with the cohesive checked authoring facade.
- The macro is not re-exported from `types`.
- README preserves DTO and direct-builder routes, then includes the exact intake example and a
  complete macro cookbook without elided family/surface placeholders.
- Docs explain error handling, one evaluation, builder-only validation, real dependency renaming,
  and public 0.2 compatibility. They reject extra assertion macros and mention normal functions as
  the preferred future direction without implementing them.

## Test architecture

### Compiler UI

Add `trybuild = "1"` as a dev-dependency. `tests/scenario_macro_ui.rs` names concrete pass and fail
fixtures under `tests/ui/scenario_macro/`.

Pass cases cover exact example, full surface, trailing separators, `#![no_implicit_prelude]`
hygiene with internal-name collisions, wildcard prelude import, crate alias, and executed
single-evaluation assertions. Compile-fail cases cover unknown family/field, wrong-family field,
mixed config, duplicate scalar, malformed transfer/target/end, and unexpected top-level syntax.
Each fail case has reviewed adjacent `.stderr`.

`TRYBUILD=overwrite` is snapshot-generation only; inspect its diff and rerun normally.

### Real Cargo rename

`tests/fixtures/scenario-macro-renamed/Cargo.toml` depends on the root package as:

```toml
[dependencies]
simulation = { package = "anapao", path = "../../.." }
```

The fixture binary invokes `simulation::scenario!` with no `anapao` crate name. The integration
harness runs Cargo check offline with isolated target output. Generated nested lock/target files are
ignored. This catches literal crate-name expansion that a Rust `use` alias alone cannot.

### Runtime equivalence

`tests/scenario_macro.rs` uses multiple small valid graphs so family-specific connection rules are
not distorted by one artificial all-family graph. Collectively it covers every grammar surface,
typed escape hatch, exact ID/namespace behavior, forward targets, defaults, returned error/no
unwind case, and side-effect counter category.

One representative graph is authored both directly and by macro; checked `Scenario`s compare
equal, then both compile and run with the same explicit seed/config and full `RunReport`s compare
equal.

### Docs and full gates

Rustdoc, `tests/readme_snippets.rs`, checked-authoring compatibility, clippy all features/targets,
and all-target tests close the public contract. No Docker, network, clock, external service, or OS
random seed is required. The nested Cargo fixture is explicitly offline.

## Compatibility and SemVer

This is additive over retained `ScenarioSpec` and direct `ScenarioBuilder` routes and requires no
persistence migration. The new macro produces the existing checked `Scenario`; it is not serde.

The supported grammar, symbol-to-ID mapping, separate namespaces, evaluation count, result/error
types, and root/prelude paths become public 0.2 behavior. Removing a form, changing those semantics,
evaluating more than once, or requiring a caller import is incompatible. Adding an unambiguous
optional form may be compatible only after the UI/equivalence matrix remains green. Anapao 0.1.1
to 0.2 is already an incompatible pre-1.0 Cargo line and is the intended point to establish this
contract.

## Rejected alternatives

- Partial Source/Pool/Delay/Queue grammar or a tracer-only shipped state.
- Independent `NodeKind`/`NodeConfig` or `ConnectionKind`/payload construction.
- Private/macro-only builder hooks, raw validation, or a second error enum.
- Procedural macro/workspace split, dynamic identifier concatenation, arbitrary section order, or
  earlier-only state targets.
- Variable mini-language or implicit end-threshold scaling.
- `expectations!`, assertion/config/report macros, or opportunistic assertion function changes.

## Traceability

| Requirement | Tasks | Validation | Risk/open decision |
| --- | --- | --- | --- |
| R001 | T001, T002, T004, T005 | exact runtime/UI/docs examples | entry grammar drift; no open decision |
| R002 | T001, T002, T003, T005 | family matrix and UI errors | incomplete family/config set |
| R003 | T001, T002, T003, T005 | transfer/connection/target matrix | forward target or payload drift |
| R004 | T001, T003, T004, T005 | field/end equality tests and docs | end OR/default semantics |
| R005 | T001, T003, T005 | ID/accessor/duplicate tests | namespace or repeated conversion |
| R006 | T001, T002, T003, T005 | counters, no-prelude, alias, Cargo rename | hygiene/evaluation regression |
| R007 | T001, T002, T003, T005 | UI snapshots and no-unwind errors | panic or error-path drift |
| R008 | T001, T003, T005 | source/API review and builder equality | private/second semantic path |
| R009 | T002, T005 | trybuild and renamed fixture harness | compiler snapshot/rename drift |
| R010 | T003, T005 | checked value and full report equality | deterministic behavior drift |
| R011 | T004, T005 | doctest/README snippets/export tests | incomplete migration/SemVer docs |
| R012 | T001, T004, T005 | export/manifest/source review | macro surface creep |
| R013 | T005 | fresh Sol/high final report | reviewer independence |

## Validation plan

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

Snapshot generation only:

```bash
TRYBUILD=overwrite cargo test --test scenario_macro_ui
git diff -- tests/ui/scenario_macro
```

## Risks and stop conditions

- Stop before any edit unless 039/T006 is exactly done and passed.
- Stop if the implemented public API differs from the exact normative predecessor contract.
- Stop if any documented form needs private checked state, raw validation, or unstable/MSRV-newer
  macro features.
- Stop if one-evaluation/no-panic requires dropping a documented surface rather than correcting
  expansion.
- Stop if actual Cargo rename fails despite alias tests, or direct-builder checked/report equality
  fails.
- Stop rather than adding another macro, partial family set, second validator, or source changes
  under spec 039.

No unresolved design or product decisions remain.
