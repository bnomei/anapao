# Evidence-only desugaring sketch

The production implementation must use the final public API reviewed by
`039-checked-scenario-authoring/T006`; names below express ownership and flow, not permission to
access fields or raw validation.

## Expression envelope

The public invocation expands to a block expression whose value is
`Result<Scenario, SetupError>`. An immediately invoked closure or equivalent labeled-result block
contains internal early returns, so builder `Err` values become the macro result rather than
silently changing the caller function's return type.

All crate-owned paths use `$crate`, and standard-library paths are absolute. Caller imports,
prelude contents, local variable names, and the dependency's Cargo name are not relied upon.

Conceptual flow:

```text
capture scenario-id expression once
  -> TryInto<ScenarioId>
  -> map IdentifierError/Infallible display to SetupError::InvalidParameter("id")
  -> ScenarioBuilder::new

first pass over node declarations
  -> stringify each symbolic ident
  -> NodeId::new without fixture/unwrap/expect
  -> retain one typed ID per symbol in a hygienic local registry

first pass over edge declarations
  -> stringify each symbolic ident
  -> EdgeId::new without fixture/unwrap/expect
  -> retain one typed ID per symbol in a separate registry

construct every ScenarioNode through its family-specific public constructor
  -> checked config Default/config expression/exact with_*/without_* setters
  -> common public setters
  -> ScenarioBuilder::insert_node

construct every ScenarioEdge through ScenarioEdge::resource or ::state
  -> transfer DTO
  -> checked connection Default/config expression/shorthand setters
  -> common public setters
  -> ScenarioBuilder::insert_edge

apply title/description/tags/variables/metadata/tracked metrics/end conditions
  -> public ScenarioBuilder methods only

ScenarioBuilder::build
  -> sole spec-039 whole-graph semantic gate
  -> Result<Scenario, SetupError>
```

The exact predecessor spellings consumed are the spec-039 normative API: all 15 family
constructors; node `with_label`, `with_initial_value`, `with_tag`, and `with_metadata`; config
field-named `with_*` plus optional capacity/min/max `without_*`; resource `with_token_size`;
`StateConnection::default` or `new(role, formula, target)` plus `with_role`, `with_formula`,
`with_target`, and `with_resource_filter`; edge `resource`, `state`, `with_enabled`, and
`with_metadata`; and the complete `ScenarioBuilder` method set. No names are left to the macro
worker to invent.

Node and edge registries are name-resolution state only. They do not validate cycles, connection
semantics, formulas, transfer semantics, end-condition shapes, references, or graph invariants.
Declared references clone retained typed IDs. For an undeclared reference, the resolver fallibly
constructs the same typed spelling from `stringify!` and passes it into the checked value;
`ScenarioBuilder::build` then emits its existing endpoint, metric, tracked, state-target, or end
reference error. The macro invents no unresolved-symbol error path or reason.

## Single evaluation

Every `$value:expr` is emitted exactly once into a fresh hygienic binding before it is converted,
inserted, formatted into an error, or passed to a setter. This applies to:

- scenario ID, title, description, tags, variables, and metadata;
- all common node and family config fields;
- custom family names;
- transfer operands and formulas;
- resource/state connection fields;
- edge enabled/metadata fields;
- end-condition values and pass-through values.

Symbolic `ident` tokens are not expressions. Each declaration is converted to its typed ID once;
references clone the retained typed value with fully qualified trait calls where required for
`#![no_implicit_prelude]` compatibility.

## Error layering

```text
unsupported/ambiguous tokens
  -> compile-time macro matcher or compile_error! diagnostic

invalid scenario ID / positive checked shorthand
  -> Result::Err(SetupError) from expansion support code

unresolved references, duplicate IDs, invalid graph, formula, target, transfer, metric, or end condition
  -> Result::Err(SetupError) from public builder insertion/build
```

There is no `panic!`, `unwrap`, `expect`, fixture-ID constructor, private helper function, raw
validation call, or macro-specific public error enum in the expansion. Compile-time diagnostics
use a stable `anapao::scenario!:` prefix for targeted syntax classes; ordinary Rust type errors are
not all snapshotted.

## Macro factoring constraint

Only `scenario!` is documented and exported. Recursive `macro_rules!` dispatch stays in reserved
`@__anapao_*` arms of that same macro rather than exporting named helper macros. The reserved arms
are absent from docs and tests invoke only the public entry form. This keeps one macro name in the
public API while acknowledging that public grammar and observable expansion behavior require
SemVer discipline after 0.2.
