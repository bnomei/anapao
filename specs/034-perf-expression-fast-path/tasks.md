# Tasks — 034-perf-expression-fast-path

Meta:
- Spec: 034-perf-expression-fast-path — Expression runtime fast path
- Depends on: spec:033-perf-matrix-and-baseline-ops
- Global scope:
  - src/expr/mod.rs
  - src/engine/mod.rs
  - src/validation/mod.rs
  - benches/simulation.rs

## In Progress

- (none)

## Blocked

- (none)

## Todo

- (none)

## Done

- [x] T001: Add compiled expression parse/evaluate API in expr runtime (owner: mayor) (scope: src/expr/mod.rs) (depends: spec:033-perf-matrix-and-baseline-ops)
  - Result: Added internal compile/evaluate APIs (`compile`, `evaluate_compiled`, `evaluate_compiled_with_resolver`) and resolver-based execution path.
  - Validation: `cargo test` with focused passing tests `expr::tests::compile_and_evaluate_compiled_are_reusable` and `expr::tests::evaluate_compiled_with_resolver_supports_lookup`.

- [x] T002: Integrate run-scoped expression caches and resolver path in engine transfer/state formulas (owner: mayor) (scope: src/engine/mod.rs, src/validation/mod.rs) (depends: T001)
  - Result: Added run-scoped `EngineExpressionCache`; engine transfer/state formula paths now reuse compiled expressions and resolver lookups instead of reparsing/building full map contexts.
  - Validation: `cargo test` and `cargo test --features parallel` passed.

- [x] T003: Add benchmark assertions and targeted fixture coverage for expression hotspots (owner: mayor) (scope: benches/simulation.rs, src/engine/mod.rs) (depends: T002)
  - Result: Bench coverage includes expression fanout hotspots across default and parallel matrices, with profiling integrated via matrix scripts.
  - Validation: `./scripts/bench-criterion summary --bench simulation --baseline hotspots-20260224-default --threshold 0.07` and `./benchmarks/run_profiles.sh`.
