# Requirements — 027-artifact-schema-v2

## Goal
Schema v2 artifacts with compatibility path

## EARS
- WHEN implementation starts for 027-artifact-schema-v2 THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:011-artifact-writer, spec:024-accuracy-indicator, spec:025-debugger-and-history) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN parity behavior is ambiguous THE SYSTEM SHALL use docs/machinations.md normative statements as source of truth for this spec.
- WHEN validation runs THE SYSTEM SHALL pass: "CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test artifact_schema_v2 && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib artifact::".
