# Requirements — 022-expression-runtime

## Goal
Expression variable runtime and math subset

## EARS
- WHEN implementation starts for 022-expression-runtime THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:017-node-model-parity, spec:018-edge-state-connection-parity) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib expr:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::".
