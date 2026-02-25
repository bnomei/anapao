# Requirements — 013-rstest-testkit

## Goal
Fixture-first harness

## EARS
- WHEN implementation starts for 013-rstest-testkit THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:002-core-types, spec:012-assertion-engine) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test".
