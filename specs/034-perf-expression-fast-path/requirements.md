# Requirements — 034-perf-expression-fast-path

## Goal
Eliminate expression parse/context hot-path overhead while preserving deterministic behavior.

## EARS
- WHEN implementation starts for 034-perf-expression-fast-path THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN transfer and state expressions execute across simulation steps THE SYSTEM SHALL avoid reparsing equivalent formulas.
- WHEN expression evaluation runs in hot loops THE SYSTEM SHALL avoid rebuilding full `BTreeMap` contexts when resolver-based lookup can provide the same values.
- IF optimized and baseline expression paths diverge THEN THE SYSTEM SHALL preserve deterministic numeric outputs for equivalent inputs.
- WHEN performance comparison runs THE SYSTEM SHALL improve these cases versus the default baseline:
  - `single_run_expression_fanout`: at least 15%
  - `single_run_state_modifiers`: at least 20%
  - `batch_run_expression_fanout` (SingleThread): at least 10%
- WHEN validation runs THE SYSTEM SHALL pass:
  - `cargo test`
  - `./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default`
  - `./benchmarks/run_profiles.sh`
