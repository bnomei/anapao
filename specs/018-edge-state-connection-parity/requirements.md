# Requirements — 018-edge-state-connection-parity

## Goal
Add resource/state connection model parity

## EARS
- WHEN implementation starts for 018-edge-state-connection-parity THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:016-semantic-rulebook-fixtures, spec:017-node-model-parity) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib types:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib validation:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::".
