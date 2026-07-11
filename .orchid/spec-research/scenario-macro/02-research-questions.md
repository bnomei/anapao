# Current-state research questions: scenario macro

## Public ownership and predecessor contract

1. Which current files own scenario DTOs, typed IDs, builder-like methods, public re-exports,
   crate docs, README examples, and snippet tests?
2. Does the repository already export any declarative or procedural macros, and are there local
   macro layout, naming, hygiene, or UI-test conventions?
3. What exact public checked types and methods does spec 039 promise for `ScenarioBuilder`,
   `ScenarioNode`, `ScenarioEdge`, checked configs, `ResourceConnection`, `StateConnection`,
   `NodeBehavior`, `ConnectionSpec`, and `StateTarget`?
4. Which spec-039 task is the final independently verified public-contract gate rather than only
   the implementation/export task?
5. Which modules and fields are intentionally private after specs 037 and 039?

## Existing scenario vocabulary

6. What node families and per-family configuration fields exist in the current source?
7. What common node fields, edge fields, transfer variants, connection variants, state targets,
   scenario metadata, variable configuration, tracked metrics, and end-condition variants exist?
8. Which numeric fields already have established positive-value validation, and what exact error
   paths/reasons are pinned?
9. How do tracked metric and metric-referenced transfer/end-condition keys resolve today?
10. What ordering, defaulting, duplicate, and deterministic behavior is already source-owned?

## Errors, IDs, and evaluation

11. What do validated ID constructors return, and can their errors flow into `SetupError` without
    adding a second public error taxonomy?
12. Does the public builder return `SetupError` for insertion and whole-graph validation?
13. Which parts of authoring are syntactic and which remain semantic runtime validation?
14. Are there any existing panic-based fixture constructors that a public macro must avoid?

## Export, docs, tests, and manifest

15. Which crate-root and prelude paths are the established concise public facade?
16. Does `Cargo.toml` already contain `trybuild`, and how are dev-dependencies locked?
17. Which existing integration tests compare public compile/run behavior and pin README snippets?
18. What repository validation commands cover formatting, doctests, clippy, all targets, and MSRV?

## External constraints to verify

19. What does the Rust Reference require for `$crate`, visibility, mixed-site hygiene, fragment
    follow sets, and repetition separators?
20. What do the Rust API Guidelines say about macro input syntax?
21. What contract does `trybuild` provide for pass tests, compile-fail tests, `.stderr` snapshots,
    and dev-dependency visibility?
22. What does Cargo document about dev-dependency propagation and public SemVer compatibility?
23. What does the Rust Reference establish for `#[track_caller]`, and does that support rejecting
    an assertion macro from this scope?
