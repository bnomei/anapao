# Design — 033-perf-matrix-and-baseline-ops

## Scope
- benches/simulation.rs
- scripts/bench-criterion
- benchmarks/run_profiles.sh
- benchmarks/run_profiles_all.sh
- benchmarks/README.md
- README.md

## Approach
- Keep existing benchmark case IDs unchanged for continuity.
- Add parallel-only Rayon case IDs without renaming existing SingleThread case IDs:
  - `simulation.guardrails/batch_run_expanded_semantics_rayon`
  - `simulation.hotspots/batch_run_expression_fanout_rayon`
- Extend profiling scripts with feature pass-through using `BENCH_FEATURES` and stable output suffixing per feature set.
- Add a manual regression summary command in `scripts/bench-criterion` that reports >+7% deltas and exits successfully.
- Keep regression checks manual-only (no CI fail gate).

## Data Flow
1. Bench command receives baseline + feature arguments.
2. Criterion writes estimates under `target/criterion/.../change/estimates.json`.
3. Summary command scans change estimates and reports regressions over threshold.
4. Profiling scripts emit SVG + CSV summaries to `benchmarks/profiles` with deterministic names.

## Deliverable
Feature-matrix benchmark/profiling workflow with explicit Rayon benchmark cases and manual +7% regression summaries.
