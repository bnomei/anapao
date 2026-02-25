# Requirements — 029-compile-reference-validation

## Goal
Fail fast on unresolved node/metric references so scenario setup errors never silently degrade execution semantics.

## EARS
- WHEN `Simulator::compile` encounters `EndConditionSpec::NodeAtLeast` or `EndConditionSpec::NodeAtMost` with a missing `node_id` THE SYSTEM SHALL return `SetupError::InvalidGraphReference` with a path-scoped reference string.
- WHEN `Simulator::compile` encounters `EndConditionSpec::MetricAtLeast` or `EndConditionSpec::MetricAtMost` with an unresolved metric key THE SYSTEM SHALL return `SetupError::InvalidGraphReference` with a path-scoped reference string.
- WHEN `Simulator::compile` encounters `TransferSpec::MetricScaled` with an unresolved metric key THE SYSTEM SHALL return `SetupError::InvalidGraphReference` referencing the owning edge id.
- WHEN `Simulator::compile` encounters `scenario.tracked_metrics` entries that do not resolve to known metric sources THE SYSTEM SHALL reject compile with a typed setup error instead of allowing runtime fallback behavior.
- IF reference validation succeeds at compile time THEN engine execution SHALL not rely on unresolved-reference fallbacks for end conditions or metric-scaled transfers.
- WHEN validation runs THE SYSTEM SHALL pass: `cargo test validation:: && cargo test --test pikmin_diagram`.
