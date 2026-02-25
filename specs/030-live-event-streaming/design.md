# Design — 030-live-event-streaming

## Scope
src/simulator.rs, src/engine/mod.rs, src/events/mod.rs, tests/*

## Approach
- Introduce an engine-to-sink emission path for single-run execution.
- Preserve the existing public API signatures (`Simulator::run`, `Simulator::run_with_assertions`).
- Keep report construction deterministic and equivalent to current semantics.
- Keep batch sink behavior unchanged in this spec (batch emits summary-level events after run completion).

## Execution Flow
1. `Simulator::run` validates config, then calls engine run with optional event-emitter bridge.
2. Engine emits step lifecycle and transfer/node/metric events as execution advances.
3. Sink errors are propagated immediately via `RunError::EventSink`.
4. `Simulator::run_with_assertions` emits assertion checkpoints only after report + assertion evaluation.

## Ordering Contract
- Event order remains stable by `(run_id, step, phase, ordinal)`.
- Assertion checkpoints remain terminal-step events.
- Event payload shape remains unchanged (`RunEvent` schema compatibility).

## Deliverable
Real-time event emission for single-run APIs with deterministic ordering and unchanged report semantics.
