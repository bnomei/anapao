# Requirements — 030-live-event-streaming

## Goal
Emit run events while simulation executes (not post-run replay) so sinks can observe and react to execution in real time.

## EARS
- WHEN `Simulator::run` is invoked with an event sink THE SYSTEM SHALL emit run events during step execution instead of synthesizing the full stream after engine completion.
- WHEN `Simulator::run_with_assertions` is invoked with an event sink THE SYSTEM SHALL emit run events during execution and emit assertion checkpoint events at terminal step after assertion evaluation.
- WHEN an event sink returns an error during run execution THEN THE SYSTEM SHALL stop further event emission and return `RunError::EventSink`.
- WHERE no event sink is provided THE SYSTEM SHALL preserve existing deterministic run/batch report behavior.
- WHERE `Simulator::run_batch` is used with a sink THE SYSTEM SHALL preserve existing batch-level event semantics unless an explicit batch-streaming mode is introduced.
- WHEN validation runs THE SYSTEM SHALL pass: `cargo test simulator:: && cargo test --test pikmin_diagram`.
