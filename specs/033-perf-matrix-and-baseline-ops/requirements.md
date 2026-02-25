# Requirements — 033-perf-matrix-and-baseline-ops

## Goal
Feature-aware benchmark and profiling operations with manual regression summaries.

## EARS
- WHEN implementation starts for 033-perf-matrix-and-baseline-ops THE SYSTEM SHALL satisfy this spec within the declared write scope.
- WHEN benchmark matrix commands run THE SYSTEM SHALL execute reproducible runs for default and `parallel` feature sets.
- WHERE the `parallel` feature is enabled THE SYSTEM SHALL benchmark both `ExecutionMode::SingleThread` and `ExecutionMode::Rayon` for batch hotspot cases.
- WHEN profiling scripts run THE SYSTEM SHALL emit flamegraphs and derived CSV summaries with stable names that include feature/mode variants.
- IF stale nereid-derived benchmark fixture references exist in benchmark or profiling scripts THEN THE SYSTEM SHALL remove those references.
- WHEN manual comparisons run THE SYSTEM SHALL provide a non-failing regression summary that highlights any case with relative delta above +7%.
- WHEN validation runs THE SYSTEM SHALL pass:
  - `./scripts/bench-criterion save --bench simulation --baseline hotspots-20260224-default`
  - `./scripts/bench-criterion save --bench simulation --features parallel --baseline hotspots-20260224-parallel`
  - `./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default`
  - `./scripts/bench-criterion compare --bench simulation --features parallel --baseline hotspots-20260224-parallel`
  - `./benchmarks/run_profiles.sh`
  - `BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh`
