# Tasks — 028-perf-and-determinism-hardening

Meta:
- Spec: 028-perf-and-determinism-hardening — Performance and determinism hardening for expanded semantics
- Depends on: spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:024-accuracy-indicator, spec:027-artifact-schema-v2
- Global scope:
  - benches/**, benchmarks/**, tests/perf_determinism.rs, src/rng/mod.rs, src/batch/mod.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 028-perf-and-determinism-hardening (owner: worker:019c9067-ad36-7e61-bf48-1fa6c28564cb) (scope: benches/**, benchmarks/**, tests/perf_determinism.rs, src/rng/mod.rs, src/batch/mod.rs) (depends: spec:020-gate-routing-engine, spec:021-delay-queue-timeline, spec:024-accuracy-indicator, spec:027-artifact-schema-v2)
  - Context: Guarantee parity expansion preserves deterministic behavior and acceptable throughput.
  - DoD: Add determinism stress tests and performance guardrails for expanded semantic surface.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test perf_determinism && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo bench --no-run
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T16:38:31Z
  - Completed_at: 2026-02-24T16:48:15Z
  - Completion note: Added determinism stress tests for rng/batch/expanded semantics and expanded deterministic benchmark coverage with no-run bench compilation guardrails.
  - Validation result: mayor recheck passed for perf_determinism tests and bench no-run build.
