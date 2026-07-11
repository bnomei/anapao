# Shape review: scenario macro

Reviewer posture: strong-model semantic review after local source inspection, full predecessor
contract inspection, Tavily PRO synthesis, and authoritative Rust/Cargo/trybuild source top-up.

## Scope completeness

GREEN. The shape is not an MVP or first slice. The grammar enumerates all 15 node families, every
configured-family field, common node/edge fields, every transfer, both connection variants, all
state targets, scenario metadata/tags/variables, tracking, and every recursive end variant. Typed
config/connection/transfer/condition escape hatches preserve dynamic public API use without
reintroducing raw tag/payload combinations.

The one-macro boundary is explicit. `expectations!`, assertion macros, procedural macros, extra
named helper macros, and assertion-function implementation are rejected rather than left for a
worker choice.

## File and ownership readiness

GREEN. Every production, manifest, test, UI fixture, rename fixture, docs, and snippet path is
concrete in `05-implementation-shape.md`. `src/scenario_macro.rs` owns grammar/desugaring;
spec-039 public checked types own invariant-bearing values; `ScenarioBuilder::build` owns graph
semantics; integration/UI/docs files own their corresponding proof. No worker is sent to raw
research, prototypes, `specs/index.md`, or `specs/_handoff.md`.

The macro does not require edits to private plan/engine/validation ownership. A missing public
checked constructor or setter is an explicit stop condition routed back to spec 039.

## Predecessor and orchestration readiness

GREEN. The exact gate is `039-checked-scenario-authoring/T006` done and verification-passed, not
the earlier T005 export task. The shape explains why: T006 independently reviews and may remediate
the whole public contract.

The current task-validator limitation is resolved without invalid frontmatter: spec-level
dependency, `before-implementation` checkpoint, empty T001 same-spec dependency list, and an exact
status guard in Context/Escalate If. This is concrete enough for orchestration and does not pretend
the queue dependency alone blocks targeted dispatch.

## State and data-flow readiness

GREEN. The shape assigns state explicitly:

- scenario ID expression -> checked ScenarioId -> ScenarioBuilder;
- symbolic node/edge declarations -> once-created typed-ID registries;
- family/connection tokens -> public checked constructors/config setters;
- expressions -> one hygienic binding each;
- insertions and final build -> unchanged SetupError flow;
- graph semantics -> sole public builder gate.

Node and edge namespaces, forward edge targets, declaration order, top-level OR end semantics,
recursive Any/All, defaults, and metric-to-node mapping are all resolved. There is no hidden choice
about private access, duplicate policy, dynamic IDs, scaling, variables, or error taxonomy.

## Macro-language and compatibility readiness

GREEN. The canonical section order, separators, fragments, symbol mapping, native fields,
mutual-exclusion rules, trailing commas, escape hatches, return type, export paths, and SemVer
promises are frozen. The design follows official evidence for `$crate`, visibility, mixed-site
hygiene, expression follow sets, Rust-shaped syntax, Result, dev-dependency isolation, and
trybuild snapshots.

The reserved internal-arm tradeoff is acknowledged without creating a second public macro name.
The supported entry grammar—not raw internal dispatch—is what docs and UI tests freeze.

## Error and safety readiness

GREEN. Syntax errors, macro ID/nonzero conversion errors, and builder semantic errors have separate
ownership. Undeclared references flow as typed spellings to the builder's existing endpoint,
metric, tracked, target, and end diagnostics, so the macro invents no unresolved-symbol semantics.
Established nonzero paths are named. Fraction validation remains with the builder. The no-panic
rule bans panic/unwrap/expect/indexing/fixture constructors introduced by the macro while correctly
excluding panic deliberately executed by a supplied expression.

Single-evaluation ownership covers every expression category and has both trybuild-pass execution
and runtime counter proof. `#![no_implicit_prelude]` and a real Cargo rename fixture prove that
hygiene is not merely inferred from source review.

## Slice readiness

GREEN. T001 is one complete invariant-owning implementation slice rather than a partial-family
tracer. T002 adds compiler and Cargo-boundary proof, T003 proves runtime semantics/equivalence, T004
publishes tested docs, and T005 performs fresh Sol/high independent review. Dependencies are
serial and every follow-up may make only narrow corrections within the macro spec.

Worker model routing is clear: Sol/high for the public macro invariant and independent validator;
Terra/high for compiler/runtime compatibility vectors; Terra/medium for docs/re-exports after the
surface is proven.

## Test and validation readiness

GREEN. Concrete pass/fail fixture names cover exact example, complete grammar, trailing
separators, no-prelude hygiene, prelude import, alias, one evaluation, targeted family/property/
transfer/target/end errors, and snapshots. A real renamed Cargo dependency catches literal crate
paths. Runtime tests cover every surface across valid focused scenarios, returned errors/no unwind,
direct checked-builder equality, and fixed-seed full report equality. Focused, docs, clippy, and
all-target gates are explicit and service-free.

## Compatibility, migration, and anti-goals

GREEN. The macro is additive over retained DTO and checked-builder paths and is positioned as a
new 0.2 grammar. The shape names incompatible future changes and preserves MSRV, edition, serde,
determinism, duplicate policy, and module privacy. No persistence migration is required because
the macro produces the existing checked `Scenario`.

## Required fixups

None. The spec-039 owner confirmed its exact checked config/node/connection/edge/builder spellings
are normative and both predecessor packet/spec validators are green. Implementation must still
perform the exact T006 status/API guard at task start; failure is an orchestration block, not an
unresolved research decision.

OVERALL: GREEN
cheap_worker_ready: yes
required_fixups: none
