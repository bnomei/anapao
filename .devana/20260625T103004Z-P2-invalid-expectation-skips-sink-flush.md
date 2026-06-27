DEVANA-FINDING: v1
Priority: P2 | Confidence: medium | Security-sensitive: no | Status: open
Location: src/simulator.rs:92 | Slug: invalid-expectation-skips-sink-flush

# Invalid expectation aborts streaming run after events are pushed but before flush

## Finding

In `run_with_assertions_internal`, the streaming engine pushes the full run event stream into the sink (src/simulator.rs:87-91), and only afterward does `evaluate_run_expectations` run (src/simulator.rs:92). `evaluate_run_expectations` first statically validates each expectation via `validate_expectation` (src/assertions/mod.rs:147, 601), which can fail on malformed expectations independent of the run report. On that failure the function returns via `?` at line 92, so the deferred `sink.flush()` at src/simulator.rs:110 never runs. For a buffered or file-backed `EventSink`, the already-pushed events are never committed — a partial/empty event stream despite a successful run.

## Violated Invariant Or Contract

The `EventSink` contract documents `flush` as the mechanism that commits pending buffered events. A streaming run that pushes a complete event sequence must flush before returning; an error arising from *static* expectation validation (which does not depend on the run) must not silently strand buffered events.

## Oracle

`validate_expectation` (src/assertions/mod.rs:601) inspects only the static `Expectation` (e.g. rejects `Between { min: 5.0, max: 1.0 }`), never the run report, so it can fail after a fully successful, fully streamed run. The neighboring guard `validate_run_config` is intentionally run *before* the engine (src/simulator.rs:86), establishing the project's "validate static inputs before side effects" pattern — which the expectation validation violates by running post-stream. The batch sibling `run_batch_with_assertions_internal` (src/simulator.rs:168-185) emits events only *after* `evaluate_batch_expectations`, so it is not affected — confirming the intended ordering.

## Counterexample

Call `Simulator::run_with_assertions_and_sink(&compiled, &valid_config, &expectations, &mut buffered_sink)` with `expectations = vec![Expectation::Between { metric, selector: Final, min: 5.0, max: 1.0 }]` (inverted bounds) and a buffered/file-backed sink. The engine streams all step_start/transfer/metric_snapshot/step_end events into the sink's buffer; `evaluate_run_expectations` then returns `Err` at line 92; lines 94-110 (checkpoints, terminal step_end, `flush()`) are skipped. The buffered sink loses its events while the engine reports a successful run.

## Why It Might Matter

A caller using a buffered/file-backed sink (a realistic implementation of the public `EventSink` trait) gets a truncated or empty artifact whenever an expectation is malformed, even though the run itself executed completely. The failure mode is silent (no flush, no rollback) and is masked in-repo because the only shipped sink, `VecEventSink`, has a no-op flush — so it is also untested.

## Proof

**Control-flow trace / cross-entry mismatch:** `run_with_assertions_internal` lines 87-91 push N events; line 92 `let assertion_report = evaluate_run_expectations(&report, expectations)?;` short-circuits on Err; lines 94-110 (including `flush()`) are skipped. `validate_expectation` (src/assertions/mod.rs:601-618) provably returns Err for `min > max` using only static inputs. The batch path (src/simulator.rs:176-182) orders validation before any push — a cross-entry inconsistency for the same operation.

## Counterevidence Checked

- `VecEventSink::flush` is a no-op (src/events/mod.rs), so the in-memory sink loses nothing — this is exactly a VecEventSink-vs-buffered-sink semantic gap.
- Sink-error paths deliberately skip flush (intended), but here the error is an unrelated static-validation error after a clean stream, so skipping flush is not the intended behavior.
- `validate_run_config` runs before the engine, so config errors do not stream events first — only expectation validation has this ordering defect.

## Suggested Next Step

Validate expectations (the static `validate_expectation` portion) before running the streaming engine, mirroring `validate_run_config`; or flush the sink on the error path before propagating, so buffered events are committed regardless of expectation validity.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.

DEVANA-KEY: src/simulator.rs:92 | P2 | invalid-expectation-skips-sink-flush
DEVANA-SUMMARY: Status=open | P2 medium src/simulator.rs:92 - A malformed expectation fails static validation after the streaming run already pushed events but before sink.flush(), so a buffered/file sink silently loses its buffered events.
