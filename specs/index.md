# Program Specs Index

Execution mode: adaptive
Concurrency cap: 3 (min 2, max 4)
Bundle depth: 2 (same write scope only)

## Dependency DAG

### Foundation (completed)
- 000-program-control
- 001-pivot-surface <- 000-program-control
- 002-core-types <- 001-pivot-surface
- 003-error-taxonomy <- 001-pivot-surface
- 004-rng-policy <- 001-pivot-surface
- 005-setup-validation <- 002-core-types, 003-error-taxonomy, 004-rng-policy
- 007-stochastic-primitives <- 004-rng-policy, 005-setup-validation
- 006-step-engine <- 005-setup-validation, 007-stochastic-primitives
- 008-events-contract <- 006-step-engine
- 009-monte-carlo-runner <- 006-step-engine, 004-rng-policy, 008-events-contract
- 010-stats-aggregator <- 009-monte-carlo-runner
- 011-artifact-writer <- 008-events-contract, 010-stats-aggregator, 003-error-taxonomy
- 012-assertion-engine <- 006-step-engine, 010-stats-aggregator, 011-artifact-writer
- 013-rstest-testkit <- 002-core-types, 012-assertion-engine
- 014-analysis-polars-optional <- 010-stats-aggregator, 011-artifact-writer
- 015-perf-guardrails <- 006-step-engine, 009-monte-carlo-runner, 011-artifact-writer

### Parity Expansion
- 016-semantic-rulebook-fixtures
- 017-node-model-parity <- 016-semantic-rulebook-fixtures
- 018-edge-state-connection-parity <- 016-semantic-rulebook-fixtures, 017-node-model-parity
- 019-trigger-action-modes <- 018-edge-state-connection-parity
- 020-gate-routing-engine <- 018-edge-state-connection-parity, 019-trigger-action-modes
- 021-delay-queue-timeline <- 018-edge-state-connection-parity, 019-trigger-action-modes
- 022-expression-runtime <- 017-node-model-parity, 018-edge-state-connection-parity
- 023-variable-update-timing <- 022-expression-runtime
- 024-accuracy-indicator <- 009-monte-carlo-runner, 010-stats-aggregator, 023-variable-update-timing
- 025-debugger-and-history <- 008-events-contract, 011-artifact-writer, 019-trigger-action-modes
- 026-parity-differential-suite <- 016-semantic-rulebook-fixtures, 020-gate-routing-engine, 021-delay-queue-timeline, 023-variable-update-timing, 024-accuracy-indicator, 025-debugger-and-history
- 027-artifact-schema-v2 <- 011-artifact-writer, 024-accuracy-indicator, 025-debugger-and-history
- 028-perf-and-determinism-hardening <- 020-gate-routing-engine, 021-delay-queue-timeline, 024-accuracy-indicator, 027-artifact-schema-v2

### Coverage Hardening (planned)
- 029-compile-reference-validation <- 005-setup-validation, 006-step-engine
- 030-live-event-streaming <- 006-step-engine, 008-events-contract, 012-assertion-engine
- 031-pikmin-fixture-refactor <- 013-rstest-testkit, 006-step-engine
- 032-event-order-contract-hardening <- 008-events-contract, 030-live-event-streaming

## Wave plan

### Completed
- Wave A: 000 -> 001
- Wave B (parallel): 002, 003, 004
- Wave C: 005, 007 -> 006
- Wave D (parallel): 008, 009, 010
- Wave E (parallel): 011, 012, 013
- Wave F: 014, 015

### New parity waves
- Wave G: 016 (completed)
- Wave H: 017 -> 018 -> 019 (completed)
- Wave I (parallel): 020, 021, 022 (completed)
- Wave J: 023 -> 024 and 025 (parallel when ready) (completed)
- Wave K: 026 and 027 (completed)
- Wave L: 028 (completed)

### Coverage hardening waves (planned)
- Wave M (parallel): 029, 031
- Wave N: 030
- Wave O: 032
