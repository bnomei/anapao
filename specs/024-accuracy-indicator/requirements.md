# Requirements — 024-accuracy-indicator

## Goal
Accuracy/convergence indicator for predictions

## EARS
- WHEN implementation starts for 024-accuracy-indicator THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:009-monte-carlo-runner, spec:010-stats-aggregator, spec:023-variable-update-timing) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib stats:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::".
