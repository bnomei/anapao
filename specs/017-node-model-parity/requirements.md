# Requirements — 017-node-model-parity

## Goal
Expand node model to full machinations primitive set

## EARS
- WHEN implementation starts for 017-node-model-parity THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:016-semantic-rulebook-fixtures) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib types:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib validation::".
