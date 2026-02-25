# Tasks — 022-expression-runtime

Meta:
- Spec: 022-expression-runtime — Expression variable runtime and math subset
- Depends on: spec:017-node-model-parity, spec:018-edge-state-connection-parity
- Global scope:
  - src/expr/**, src/engine/mod.rs, src/types/mod.rs, src/lib.rs

## In Progress

## Blocked

## Todo

## Done
- [x] T001: Implement 022-expression-runtime (owner: worker:019c903c-8e81-70f2-8490-6a1996216add) (scope: src/expr/**, src/engine/mod.rs, src/types/mod.rs, src/lib.rs) (depends: spec:017-node-model-parity, spec:018-edge-state-connection-parity)
  - Context: Provide math-expression compatibility subset for test scenarios without introducing nondeterministic evaluation.
  - DoD: Add expression runtime with whitelisted math functions and deterministic evaluation graph.
  - Validation: CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib expr:: && CARGO_HOME=/Users/bnomei/Sites/anpao/.cargo-home cargo test --lib engine::
  - Escalate if: Required code edits are needed outside the listed scope.
  - Started_at: 2026-02-24T15:35:14Z
  - Completed_at: 2026-02-24T15:41:05Z
  - Completion note: Added deterministic expression parser/evaluator, stable dependency-graph evaluation, and integrated expression formulas into engine transfer/state modifier paths.
  - Validation result: mayor recheck passed for expr and engine test suites.
