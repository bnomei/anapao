# Design decisions: scenario macro

## D001 — One documented declarative macro

Decision: add exactly one documented public macro, `scenario!`, implemented with `macro_rules!` in
`src/scenario_macro.rs` and exported as `anapao::scenario!`.

Rationale:

- The problem is token-level binding of repeated graph symbols to typed IDs; a normal function
  cannot express those symbolic references.
- The Rust API Guidelines favor input that resembles the output. Named `nodes`, `edges`, `track`,
  and `end` sections mirror a scenario graph.
- A procedural macro crate, derive macro, and family of assertion/config macros would enlarge
  packaging, diagnostics, and SemVer surfaces without solving a distinct token-level problem.

Reserved recursive `@__anapao_*` matcher arms may live inside the same exported macro so no named
helper macro is exported. They are undocumented implementation dispatch; the supported public
entry grammar is frozen below and UI-tested.

Rejected:

- A procedural-macro workspace member: unnecessary dependency/packaging/MSRV complexity.
- Multiple public node/edge/end/assertion macros: fragmented grammar and larger compatibility
  surface.
- A runtime parser: loses Rust type checking and cannot bind Rust expressions naturally.

## D002 — Hard gate on the independently reviewed checked builder

Decision: spec 040 depends on `039-checked-scenario-authoring`, and T001 must stop unless
`specs/039-checked-scenario-authoring/tasks/T006.md` has both `status = "done"` and
`verification_status = "passed"`.

T005 creates the public exports/docs, but T006 is the fresh Sol/high review/remediation task over
the complete API and cross-module invariant. Consuming T005 alone could bind the macro to a public
surface that T006 then changes.

The current `validate_task_spec.py` rejects cross-spec task IDs in task frontmatter even though the
skill documents that syntax. The supported fallback is:

- `spec.toml.depends_on = ["039-checked-scenario-authoring"]`;
- `human_checkpoint = "before-implementation"`;
- no rejected cross-spec entry in T001 `depends`;
- an exact T006 done/passed guard in T001 Context and Escalate If.

Rejected:

- Depending only on T005.
- Reaching into spec-039 private fields or adding a macro-specific hidden constructor.
- Encoding the known-invalid cross-spec task frontmatter syntax.

## D003 — Canonical complete top-level grammar

Decision: the public entry matcher uses this stable section order:

1. required `id: <expr>;`;
2. optional `title`, `description`, `tags`, `variables`, and `metadata` sections;
3. required `nodes { ... }` and `edges { ... }` blocks, each allowed to be empty so checked build
   owns graph validity;
4. optional `track [...]`;
5. zero or more `end <condition>;` statements, preserving the builder's default when absent.

Property/list repetitions accept optional trailing commas. Statements use semicolons. Metadata
uses `key_expr => value_expr` because `=>` and commas/semicolons are valid `expr` follow tokens;
`expr: expr` would violate follow-set constraints for arbitrary key expressions.

`id:` accepts `TryInto<ScenarioId>`, supporting strings and an already checked ID. Every node and
edge declaration name is an `ident`; its ID string is exactly `stringify!(ident)`. Nodes and edges
have separate symbolic namespaces. Dynamic IDs are intentionally represented by the direct
builder, not a second macro syntax.

Rejected:

- Arbitrary top-level section order: substantially increases local ambiguity and diagnostic
  complexity without adding scenario capability.
- `tt` for scalar values: loses expression parsing and produces later, noisier errors.
- Colon-separated expression metadata entries: incompatible with `expr` follow rules.

## D004 — Complete node grammar with checked-config escape hatches

Decision: the macro covers all 15 node families in spec-039 `NodeBehavior`:

- Configless: Source, Process, Sink, Gate, and `Custom(family_expr)`.
- Configured: Pool, Drain, SortingGate, TriggerGate, MixedGate, Converter, Trader, Register, Delay,
  and Queue.

Every family supports common label, initial value, tags, and metadata. Native checked-config
shorthands cover every current field:

| Family | Macro fields |
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

Each configured family also accepts an exclusive `config: <checked config expr>` for dynamic
optional values and forward compatibility with the final checked config surface. `config` cannot
mix with field shorthands; `mode` cannot mix with `trigger`/`action`. Unknown, duplicate scalar,
and wrong-family properties receive targeted compile-time diagnostics.

Pool/Queue `capacity` and Register `min_value`/`max_value` accept either a value expression or the
literal `none`. The latter calls the exact spec-039 `without_capacity`, `without_min_value`, or
`without_max_value` method; omission starts from the same `Default` state.

Positive shorthand values are converted to `NonZeroU64` once and return the established
`SetupError::InvalidParameter` path/reason on zero. Omitted config fields retain the reviewed
checked default. The macro does not construct `NodeBehavior` plus config independently; it calls
the family-specific `ScenarioNode` constructor and public checked setters.

Rejected:

- Only Source/Pool/Delay/Queue support: an MVP that would immediately fragment authoring.
- Raw `NodeKind`/`NodeConfig`: recreates the invalid independent tag/payload model fixed by 039.
- Field-order-dependent node blocks: unlike Rust named fields and unnecessarily brittle.

## D005 — Complete edge, transfer, and state-target grammar

Decision: the macro covers every transfer:

- `fixed(amount)`;
- `fraction(numerator, denominator)`;
- `remaining`;
- `metric_scaled(node_symbol, factor)`;
- `expression(formula)`;
- `transfer(TransferSpec)` as a dynamic pass-through.

An edge without a connection suffix is a default resource edge. A `resource { ... }` suffix
accepts either an exclusive checked `connection` expression or native `token_size`, plus enabled
and metadata. A `state { ... }` suffix accepts either an exclusive checked `connection` expression
or role, formula, target, and resource filter, plus enabled and metadata.

State targets cover `node`, `resource_connection(edge)`, `state_connection(edge)`, and
`formula(edge)`. All edge symbols are registered before construction so forward state-target
references resolve without changing declaration order. Each edge still goes through
`ScenarioEdge::resource` or `ScenarioEdge::state` and `ScenarioBuilder::insert_edge`.

The fraction DTO remains `TransferSpec`; its zero-denominator semantic check remains in the
checked builder. Resource token size is a checked config shorthand and is converted to nonzero at
the established edge path before calling its setter.

Rejected:

- Arrow punctuation to encode connection type: it makes state semantics visually subtle and
  overloads graph direction.
- Constructing `EdgeConnectionConfig` or independent `ConnectionKind` payloads.
- Requiring referenced state-target edges to appear earlier.

## D006 — Complete scenario fields, tracking, variables, and end conditions

Decision:

- Title, description, scenario tags, variables, and scenario metadata call the corresponding
  public `ScenarioBuilder` method.
- `variables: <VariableRuntimeConfig expr>;` is a typed pass-through because spec 039 deliberately
  owns variables as one builder field and does not introduce a second checked variable DSL.
- `track [node_symbol, ...];` derives `MetricKey`s from node symbols. Metric-scaled transfers and
  metric end conditions use the same mapping, matching the current node-backed metric contract.
- End grammar covers MaxSteps, metric/node at-least/at-most, recursive Any/All, and a typed
  `condition(EndConditionSpec)` pass-through.
- Multiple top-level end statements are passed as one ordered list, preserving current top-level
  OR semantics. No statement preserves `ScenarioBuilder::new`'s default MaxSteps(1).

Empty recursive Any/All and invalid references are semantic values and reach
`ScenarioBuilder::build`, which returns the established error. Macro syntax only determines which
variant to construct.

Rejected:

- A new stochastic-variable mini-language: outside the symbolic-ID pain and duplicates an already
  typed DTO.
- Implicit scaling of f64 end thresholds: no source-owned scaling contract exists for the macro to
  invent.
- Treating multiple top-level end statements as All: contradicts current engine behavior.

## D007 — Hygienic result expression with one evaluation and no panic

Decision: `scenario!` evaluates to `Result<Scenario, SetupError>`. Its expansion uses:

- `$crate`-qualified paths for every Anapao type/method/macro;
- absolute `::core`/`::std` paths where standard items are needed;
- a hygienic result envelope for early `Err` propagation;
- one fresh local binding per captured expression before any conversion, formatting, or setter;
- typed node/edge/metric symbol registries populated once from declarations;
- no caller prelude, local name, or package dependency name assumption.

`ScenarioId` conversion uses `TryInto`. Symbol IDs use `NodeId::new`/`EdgeId::new`, never
`fixture`. Nonzero shorthands use `NonZeroU64::new`. Conversion failures map to the existing
`SetupError` paths; builder insertion/build errors pass through unchanged.

Known references clone retained IDs. If a referenced symbol is undeclared, the resolver creates
the corresponding typed ID/metric from the same `stringify!` spelling and passes it onward;
`ScenarioBuilder::build` retains sole ownership of missing endpoint, metric, tracked, state-target,
and end-reference variants, paths, reasons, and ordering. The macro does not reject registry misses
with a new `InvalidParameter` choice.

The expansion contains no `panic!`, `unwrap`, `expect`, indexing, or unchecked registry lookup.
Invalid syntax fails compilation; invalid values or graphs return `Err`.

Rejected:

- Expanding raw `?` into the caller and returning `Scenario`: contradicts the documented macro
  result and makes invocation depend on the caller's return type.
- Calling a private or doc-hidden support function: `$crate` does not bypass visibility and such a
  function would become a de facto public expansion dependency.
- Evaluating expressions directly in both error and success branches.

## D008 — Intentional export and documentation paths

Decision:

- Define the implementation in `src/scenario_macro.rs` and include it from `src/lib.rs`.
- `#[macro_export]` provides canonical `anapao::scenario!` root invocation.
- `src/prelude.rs` explicitly re-exports `scenario` so `use anapao::prelude::*` imports the one
  recommended macro alongside checked authoring types.
- Crate docs and README show the exact starting example, a complete grammar example, equivalence to
  direct checked builder use, error handling with `?`, and the fact that the macro does not define
  a serde format or validation path.

No macro is re-exported from `types`, and no public named helper macro is added.

## D009 — Compiler UI, runtime, rename, and documentation proof

Decision: add `trybuild = "1"` under `[dev-dependencies]`; Cargo documents that it is test-only and
not propagated. The lockfile records the resolved version.

The complete proof set is:

- trybuild pass cases for the exact example, full grammar, optional/trailing separators,
  `#![no_implicit_prelude]` hygiene, prelude import, crate alias, and one-evaluation assertions;
- targeted trybuild compile-fail cases for unknown family/field, wrong-family field, mixed
  config forms, duplicate scalar fields, malformed transfer/target/end syntax, and reserved or
  unsupported top-level grammar, with reviewed `.stderr` snapshots;
- an actual nested Cargo fixture that depends on `anapao` under a different dependency name and is
  checked offline by an integration test with an isolated target directory;
- runtime integration tests for every family/transfer/connection/target/end form, stable returned
  errors without unwind, duplicate policy, one evaluation, direct-builder `Scenario` equality,
  and fixed-seed full `RunReport` equality;
- rustdoc plus README snippet tests.

Only diagnostics intentionally owned by the macro are snapshotted. Arbitrary downstream Rust type
errors are not exhaustively frozen. Snapshot refresh uses `TRYBUILD=overwrite` followed by diff
review.

## D010 — Public grammar is a 0.2 compatibility contract

Decision: document the supported grammar, return type, ID derivation, evaluation count, error
layering, and export paths as public 0.2 API. Removing an accepted form, changing symbol-to-ID
mapping, evaluating an expression more than once, changing `Result` success/error types, or
requiring a new caller import is incompatible downstream behavior.

Adding an unambiguous optional property may be compatible, but must pass the UI and equivalence
matrix. Internal expansion details that do not alter observable behavior may change. Anapao is
currently 0.1.1; Cargo's pre-1.0 compatibility ranges already treat 0.2 as a new incompatible
line, making it the intended point to establish this grammar.

## D011 — Reject extra macros; keep function alternatives separate

Decision: do not add `expectations!`, assertion, report, or config macros. Normal associated
constructors such as `Expectation::equals`/`approx`/`between` and a
`#[track_caller] AssertionReport::assert_success` method would have better IDE/navigation and
callsite diagnostics, but implementing them is explicitly outside spec 040.

This decision is recorded in README/API guidance so later ergonomics work does not infer that
every verbose enum should become a macro.

Rejected:

- An `expectations!` subgrammar inside `scenario!`.
- An `assert_scenario!` wrapper around `AssertionReport`.
- Adding the function alternatives opportunistically to this macro spec.

## Open decisions

None. Spec 039 has frozen the exact constructor/default/setter contract consumed here. If the
completed T006 implementation differs from that normative contract, T001 stops and routes the
mismatch back to spec 039 rather than inventing macro-only access.
