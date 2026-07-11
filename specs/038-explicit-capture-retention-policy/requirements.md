# Requirements — 038 Explicit Capture And Retention Policy

## Objective

Replace ambiguous capture sentinels with typed, legacy-readable policies and remove discarded full
reports from batch aggregation without changing simulation results, streamed events, public report
schemas, or deterministic ordering.

## Functional and quality requirements

R001: THE SYSTEM SHALL expose `CaptureSchedule`, `Selection<T>`, `CaptureConfig`, and
`AggregationConfig` types that directly represent none, final-only, periodic, all, and concrete
selection states without a zero stride or empty-set-means-all sentinel.

R002: WHEN a caller uses `CaptureConfig::none()` THE SYSTEM SHALL return scenario/run metadata and
terminal node/metric maps with empty node snapshots, variable snapshots, transfer records, and
metric series.

R003: WHEN a caller supplies a concrete node, metric, variable, or transfer selection THE SYSTEM
SHALL retain only matching diagnostics and reject unknown or empty concrete selections before state
execution produces an observable partial result.

R004: WHEN report diagnostics are disabled for `Simulator::run_with_sink` or an assertion-streaming
run THE SYSTEM SHALL preserve the same ordered live simulation and assertion events as the
equivalent default-capture run.

R005: WHERE legacy five-field capture JSON or a legacy nested `BatchRunTemplate.capture` payload is
supplied THE SYSTEM SHALL deserialize its historical behavior, reject a zero legacy stride, and
serialize subsequent output only in the canonical typed shape.

R006: WHEN a batch is configured THE SYSTEM SHALL use an `AggregationConfig` containing only a
schedule and metric selection, while per-run final summaries remain available independently of
aggregate step series.

R007: WHEN batch execution runs THE SYSTEM SHALL retain only compact per-run metadata, final metrics,
and requested aggregate metric points, without allocating node snapshots, variable snapshots,
transfer logs, final node maps, or other discarded full-report diagnostics.

R008: WHERE SingleThread and Rayon execute the same compiled scenario, seeds, limits, and aggregation
policy THE SYSTEM SHALL normalize samples into complete run-index order and fold floating-point
metric sums sequentially in that order so run summaries and aggregate series are equal; without the
`parallel` feature a Rayon request SHALL retain the existing deterministic SingleThread fallback.

R009: WHEN aggregation/capture series are explicitly absent THE SYSTEM SHALL keep final-value
assertions, batch summary events, and terminal artifact data usable while step/series assertions
report missing evidence and series/variable artifacts remain valid empty outputs.

R010: BEFORE the compact batch path replaces full reports THE SYSTEM SHALL save named default and
parallel Criterion baselines plus isolated DHAT peak-live-heap baselines; after replacement THE
SYSTEM SHALL compare identical workloads and checksums, report absolute and relative deltas with
host/toolchain metadata, patiently rerun noisy results in the same environment, and require explicit
owner decision before completion for repeatable regressions or evidence that contradicts the
compact-allocation premise, without inventing a numeric performance pass/fail threshold.

R011: WHEN the migration is complete THE SYSTEM SHALL align crate/prelude exports, rustdoc, README
snippets, CHANGELOG migration notes, testkit fixtures, benchmark instructions, and the local
determinism checklist with the typed final API and shall contain no active example that recommends legacy fields or
`CaptureConfig::disabled()`.

R012: BEFORE the spec is completed THE SYSTEM SHALL receive fresh independent Sol/high reviews of
the public API/Serde contract, performance evidence, and sequential/Rayon determinism, and SHALL
remediate every actionable finding within scope.

## Acceptance anchors

| Requirement | Acceptance evidence |
| --- | --- |
| R001 | Type/unit tests for constructors, variants, `NonZeroU64`, and canonical tagged JSON in `src/types/mod.rs` and `tests/capture_retention_policy.rs`. |
| R002 | Public integration test asserting every diagnostic collection is empty and both terminal maps are populated. |
| R003 | Engine/integration tests for all/none/only on four channels plus path-specific invalid-selection errors. |
| R004 | Event-stream equality tests under default versus no retention, including assertion checkpoints. |
| R005 | Literal legacy/current JSON vector tests, zero-stride rejection, and canonical re-serialization without legacy keys. |
| R006 | Batch config/wire tests plus final and step assertion tests under aggregation none/final/every. |
| R007 | Private compact-sample tests and DHAT evidence showing discarded report channels are not allocated/retained. |
| R008 | Repeated default/parallel `BatchReport` equality in `src/batch/mod.rs` and `tests/perf_determinism.rs`. |
| R009 | Focused assertion, simulator-event, and artifact writer tests with absent series/variables. |
| R010 | Named matched pre/post `bench-criterion` output and isolated `bench-capture-memory` absolute/relative delta comparison, including checksums, host/toolchain metadata, reruns, and any required owner decision, recorded in task validation. |
| R011 | README snippet tests, doctests, CHANGELOG/rustdoc review, and local reference review. |
| R012 | T005 fresh Sol/high review report/checklist, remediation diff if needed, and complete validation gate. |

## Boundaries

- This spec does not change simulation math, RNG draws, end conditions, event ordering, public
  `RunReport`/`BatchReport` field shapes, or aggregate averaging semantics.
- `BatchSample`, collector types, and Serde wire intermediates remain private.
- `src/plan.rs` from `037-compiled-scenario-trust-boundary/T004` is consumed as read-only context;
  this spec does not reopen compiled-plan invariants or public module visibility. The concrete
  cross-task frontmatter edge must be added when the validator supports sibling task resolution;
  until then the spec dependency, human checkpoint, and T001 stop condition are mandatory. T001 must
  read `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` before any edit and proceed only
  when it contains exactly `status = "done"` and `verification_status = "passed"`.
- No parallel `f64` reduction is permitted.
- Package-version bumping and publishing are outside this spec; migration notes go under
  `CHANGELOG.md` Unreleased.
