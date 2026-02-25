# Design — 034-perf-expression-fast-path

## Scope
- src/expr/mod.rs
- src/engine/mod.rs
- src/validation/mod.rs
- benches/simulation.rs

## Approach
- Add internal compiled-expression support in `expr` for parse-once/reuse-many execution.
- Introduce resolver-based evaluation API to avoid repeated temporary `BTreeMap` allocation in engine hot loops.
- Build run-scoped expression caches keyed by edge ID for transfer/state formula execution.
- Replace per-call synthetic `evaluate_graph` scaffolding for `next_step` and `is_positive_total` with direct deterministic derived values.

## Determinism Guardrails
- Preserve variable precedence semantics from existing context assembly.
- Preserve invalid-expression behavior by returning deterministic zero/skip behavior where previous runtime path produced no value.

## Deliverable
Expression runtime and engine transfer/state paths that are parse-cached and resolver-driven, with regression tests and benchmark deltas.
