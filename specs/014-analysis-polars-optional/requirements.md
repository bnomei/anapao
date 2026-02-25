# Requirements — 014-analysis-polars-optional

## Goal
Optional polars analysis surface

## EARS
- WHEN implementation starts for 014-analysis-polars-optional THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN dependencies (spec:010-stats-aggregator, spec:011-artifact-writer) are not complete THE SYSTEM SHALL keep tasks blocked until dependencies are satisfied.
- WHEN validation runs THE SYSTEM SHALL pass: "cargo test --features analysis-polars".
