DEVANA-FINDING: v1
Priority: P2 | Confidence: high | Security-sensitive: no | Status: fixed
Location: src/engine/mod.rs:1978 | Slug: unvalidated-capture-metric-zero-series

# Unknown capture_metrics key silently records an all-zero series

## Finding

When `config.capture.capture_metrics` is non-empty, `capture_step` records each configured key by calling `metric_value` (src/engine/mod.rs:1977-1984). `metric_value` returns `0.0` for any key that is neither present in `state.metrics` (i.e. a tracked metric) nor resolvable to a node index (src/engine/mod.rs:1881-1890). `capture_metrics` is never validated against the compiled scenario — `validate_run_config` takes only the `RunConfig` and checks just `max_steps`/`every_n_steps`, so it cannot cross-check metric names. A misspelled or stale capture key therefore produces a plausible-looking series of `0.0` at every captured step, under the wrong label, with no error.

## Violated Invariant Or Contract

A captured metric series must reflect a real, resolvable metric value. An unresolvable capture key should be rejected at validation (as `tracked_metrics` are) rather than silently emitting fabricated `0.0` data.

## Oracle

`tracked_metrics` are validated to resolve against the scenario during compile (src/validation/mod.rs metric resolution), and `metric_value` only falls back to live `node_values` when a key is absent from `state.metrics`, returning `0.0` only for a key that resolves to nothing (src/engine/mod.rs:1886-1890). The asymmetry — tracked metrics validated, capture metrics not — is the contract source. `validate_run_config` (called at src/simulator.rs:86) does not receive the compiled scenario, so it structurally cannot validate capture keys.

## Counterexample

Set `config.capture.capture_metrics = { "snk" }` where the real tracked metric/node is `"sink"` (typo). At each captured step `metric_value(compiled, state, "snk")` finds no entry in `state.metrics`, no node index for `"snk"`, and returns `0.0`. The resulting `series`/`series.csv` shows a metric `snk` flat at `0.0` for the whole run, and the run reports success.

## Why It Might Matter

A user debugging via captured series sees a clean all-zeros line for a mistyped or renamed metric instead of an error, easily misread as "the metric never moved." This masks user error with fabricated data and can hide real regressions in CI artifacts.

## Proof

**Control-flow trace:** `capture_step` (src/engine/mod.rs:1976-1984) iterates `config.capture.capture_metrics` and pushes `SeriesPoint::new(step, metric_value(...))`. `metric_value` (1881-1890): miss in `state.metrics` -> miss in `metric_node_index` -> `return 0.0`. **Contract mismatch:** `validate_run_config` (src/simulator.rs:86; src/validation/mod.rs) validates only `max_steps`/`every_n_steps` and lacks the scenario needed to validate `capture_metrics`.

## Counterevidence Checked

- If the key happens to equal a node id, `metric_value` resolves it to that node's live value (possibly intended) — the bug is specifically the non-resolving key path returning `0.0`.
- `capture_nodes` with an unknown id silently records nothing (omission, less harmful); the metric path is worse because it fabricates a value.
- No downstream consumer re-validates capture keys before writing `series.csv`.

## Suggested Next Step

Validate `capture.capture_metrics` (and `capture_nodes`) against the compiled scenario at compile/run setup and reject unknown keys, or have `capture_step` skip/flag keys that do not resolve instead of recording `0.0`.

## Agent Handoff

After working this report, preserve the original finding body. Update line 2 `Status: ...` and the final `DEVANA-SUMMARY:` status. Use one of: `open`, `fixed`, `invalid`, `stale`, `duplicate`, `wontfix`. Add dated notes below with the evidence checked.

## Status Notes

- 2026-06-25: open by Devana. Initial report written from static source inspection.
- 2026-06-27: fixed. Confirmed `validate_run_config` lacks the scenario so it can't check capture keys, and `capture_step` records `metric_value` (which returns 0.0 for an unresolvable key). Added `validate_capture_selection(compiled, config)` called at the very top of `run_single_internal` (the central path for single, streaming, and batch runs, all of which have compiled + config). It rejects an unknown `capture_nodes` id and any `capture_metrics` key that resolves to neither a node nor a tracked metric — the exact resolution `metric_value` performs — returning `RunError::InvalidRunConfig` with `run.capture.capture_metrics.<key>` / `run.capture.capture_nodes.<id>`. Also validated `capture_nodes` (the report's milder omission case) for symmetry. Added regression test `run_single_rejects_unresolvable_capture_keys` (mistyped metric `snk` and unknown node `nope` both rejected; the real tracked metric + real node id run fine). Full `cargo test` green (no fixture captured an unresolvable key).

DEVANA-KEY: src/engine/mod.rs:1978 | P2 | unvalidated-capture-metric-zero-series
DEVANA-SUMMARY: Status=fixed | P2 high src/engine/mod.rs:1978 - An unknown capture_metrics key was never validated and metric_value returned 0.0, emitting a fabricated all-zero series. Fixed by validate_capture_selection in run_single_internal rejecting capture keys that resolve to no node or tracked metric (and unknown capture_nodes); regression test added.
