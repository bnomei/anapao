# Intake: scenario macro

## Research progress

- [x] Intake captured
- [x] Current-state questions written
- [x] Source-backed current state documented
- [x] Decisions resolved
- [x] Implementation shape drafted
- [x] Shape review green
- [x] make-specs handoff frozen
- [x] Validator passed

## Starting finding (verbatim)

## Macro bonus

My recommended macro set deliberately contains one macro: `scenario!`.

```rust
let scenario = scenario! {
    id: "queue-flow";

    nodes {
        source: Source { initial: 64.0 };
        delay: Delay { steps: 2 };
        sink: Pool;
    }

    edges {
        source_delay: source -> delay => fixed(1.0);
        delay_sink: delay -> sink => remaining;
    }

    track [sink];
    end max_steps(24);
}?;
```

Its value is not fewer characters; it binds symbolic IDs once, preventing key/embedded-ID drift. It should:

- Expand only through the checked public builder.

- Use `$crate` paths and evaluate supplied expressions once.

- Accept trailing separators and return `Result`, never panic.

- Have rustdoc examples plus `trybuild` pass/fail, hygiene, crate-renaming, and one-evaluation tests.

Do not add `expectations!` yet. Associated constructors such as `Expectation::equals`, `approx`, and `between` will provide better IDE support and diagnostics. Likewise, prefer `#[track_caller] AssertionReport::assert_success()` over an assertion macro. This follows the Rust guidance that public macros should be small, output-shaped, and used only when normal functions cannot express the ergonomics. [Rust macro guidelines](https://rust-lang.github.io/api-guidelines/macros.html), [macro-by-example reference](https://doc.rust-lang.org/reference/macros-by-example.html).

## Delivery directive

- Run the complete `make-research` workflow and then compile a complete active `make-specs`
  artifact for this finding.
- Do not deliver an MVP, tracer-only result, first slice, partial node-family set, or partial macro
  grammar. The finished spec must cover the solution end to end.
- The macro is downstream of the complete checked-authoring public API. It must not use private
  fields, raw validation, plan assembly, or engine internals.
- The exact predecessor gate is `039-checked-scenario-authoring/T006`, because T005 publishes the
  API while T006 independently reviews and remediates the entire public contract.
- The current task validator rejects otherwise valid cross-spec task frontmatter dependencies.
  The active spec must therefore use `spec.toml.depends_on`, a `before-implementation` human
  checkpoint, and an explicit T006 done/passed stop guard in T001.

## Problem framing

The checked builder planned by spec 039 removes invalid tag/payload combinations but remains
verbose for graphs whose identities are repeated across node declarations, edge endpoints,
tracked metrics, state targets, transfer metrics, and end conditions. A single declarative macro
can make each symbolic identity the source for all generated typed IDs while retaining the
builder as the only semantic authority.

The risk is not just incomplete sugar. A public macro becomes a long-lived grammar and expansion
contract. It must preserve hygiene under dependency renaming, avoid repeated expression
evaluation, return builder errors rather than panic, cover the whole public graph vocabulary, and
have compiler-facing UI tests in addition to runtime equivalence tests.

## Success signals

- The exact starting example compiles and returns `Result<Scenario, SetupError>`.
- One documented `scenario!` macro covers every current node family; all transfer variants;
  resource and state connections; all state targets; checked config fields; node, edge, and
  scenario metadata; scenario variables; tracked metrics; and recursive end conditions.
- Symbolic node and edge declarations are the only source for generated IDs and references.
- Every caller expression is evaluated once, trailing separators are accepted, and invalid values
  or graphs return stable errors without a panic.
- Expansion uses `$crate` and only public spec-039 types, constructors, setters, and
  `ScenarioBuilder::build`.
- Rustdoc, integration, `trybuild`, no-implicit-prelude hygiene, crate-renaming, diagnostic,
  one-evaluation, error-path, and direct-builder/runtime-equivalence tests are all present.
- `scenario!` is exported intentionally at crate root and through the prelude without adding a
  second documented macro.
- An independent Sol/high review finds the grammar, expansion, diagnostics, SemVer posture, and
  complete surface ready for downstream use on MSRV 1.85.

## Initial scope

- One new macro implementation module and its crate-root/prelude exposure.
- Complete grammar and direct desugaring to checked public authoring types.
- `trybuild` as a dev-dependency plus compiler UI fixtures and snapshots.
- Public behavior tests, a real Cargo dependency-renaming fixture, rustdoc, README, and snippet
  coverage.
- Migration and SemVer documentation for a new public grammar in the intended 0.2 line.

## Non-goals

- `expectations!`, assertion macros, run/batch/report/artifact macros, derive macros, or a
  procedural-macro crate.
- Adding the suggested `Expectation` constructors or `AssertionReport::assert_success` in this
  spec; those function APIs belong to a separate assertion-ergonomics change.
- A second scenario model, serde format, validation engine, macro-only builder, or hidden access to
  private checked fields.
- Dynamic generation of Rust identifiers, typestate, token-level expression parsing, or accepting
  arbitrary Rust-like syntax outside the documented grammar.
- Changing deterministic execution, `ScenarioSpec` wire compatibility, or spec-039 duplicate and
  error semantics.
