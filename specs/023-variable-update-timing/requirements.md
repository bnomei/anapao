# Requirements — 023-variable-update-timing

## Goal
Variable update timing and random/list refresh policies

## EARS
- WHEN implementation starts for 023-variable-update-timing THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:022-expression-runtime) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stochastic::".
