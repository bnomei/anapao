# Requirements — 025-debugger-and-history

## Goal
Rule violation debugger events and replay history

## EARS
- WHEN implementation starts for 025-debugger-and-history THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:008-events-contract, spec:011-artifact-writer, spec:019-trigger-action-modes) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib events:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::".
