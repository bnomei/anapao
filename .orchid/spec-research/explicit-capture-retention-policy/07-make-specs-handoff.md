# Make Specs Handoff: explicit-capture-retention-policy

## Status

- research_id: explicit-capture-retention-policy
- status: frozen
- intended_spec_slug: explicit-capture-retention-policy
- shape_review: GREEN
- cheap_worker_ready: yes

## Objective

Replace ambiguous run capture booleans/sentinels with a typed, legacy-readable `0.2` public policy;
separate batch aggregation from diagnostic retention; execute batches through private compact samples
folded in explicit run-index order; and prove the finished migration with compatibility, determinism,
throughput, peak-live-heap, documentation, and independent Sol/high review coverage.

## Requirements Seed

- R001: THE SYSTEM SHALL expose typed `CaptureSchedule`, `Selection<T>`, `CaptureConfig`, and
  `AggregationConfig` contracts that represent none, final, periodic, all, and concrete selections
  without zero strides or empty-set sentinels.
- R002: WHEN `CaptureConfig::none()` is used THE SYSTEM SHALL return terminal result data with empty
  node snapshots, variable snapshots, transfer records, and metric series.
- R003: WHEN report retention is disabled during a streaming run THE SYSTEM SHALL preserve the same
  ordered live events as the equivalent default-capture run.
- R004: WHERE legacy run or nested batch capture JSON is supplied THE SYSTEM SHALL preserve its
  historical behavior while serializing only the canonical typed shape.
- R005: WHEN a batch executes THE SYSTEM SHALL construct only compact per-run summaries and requested
  aggregate points, without allocating discarded full `RunReport` diagnostics.
- R006: WHERE SingleThread and Rayon execute identical inputs THE SYSTEM SHALL fold samples in
  run-index order and produce equal runs/aggregate series while preserving fallback behavior.
- R007: WHEN the compact batch path replaces full reports THE SYSTEM SHALL compare matched pre/post
  Criterion throughput and isolated DHAT peak-live-heap evidence against named pre-change baselines,
  report absolute and relative deltas with host/toolchain metadata, and escalate repeatable
  regressions or evidence contradicting the compact-allocation premise for explicit owner decision.
- R008: WHEN the public migration is complete THE SYSTEM SHALL align exports, rustdoc, README,
  testkit, assertion/artifact behavior, benchmark docs, and local determinism guidance.
- R009: BEFORE completion THE SYSTEM SHALL pass fresh Sol/high reviews for public API/serde,
  performance evidence, and sequential/Rayon determinism, with all actionable findings remediated.

## Scope

In scope:

- `Cargo.toml`, `Cargo.lock`
- read-only prerequisite context: `src/plan.rs` as finalized by
  `037-compiled-scenario-trust-boundary/T004`
- `src/types/config.rs`, `src/types/mod.rs`, `src/types/reports.rs`
- `src/lib.rs`, `src/prelude.rs`, `src/validation/mod.rs`, `src/engine/mod.rs`, `src/batch/mod.rs`
- `src/simulator.rs`, `src/testkit/mod.rs`, `src/assertions/mod.rs`, `src/artifact/mod.rs`
- `tests/capture_retention_policy.rs`, `tests/perf_determinism.rs`,
  `tests/failure_path_batch_events.rs`, `tests/readme_snippets.rs`
- `benches/simulation.rs`, `benches/capture_memory.rs`, `scripts/bench-capture-memory`
- `benchmarks/README.md`, `README.md`, `CHANGELOG.md`,
  `skills/anapao/references/determinism-checklist.md`

Out of scope:

- simulation math/RNG/event-order changes, public report-schema expansion, public `BatchSample`,
  parallel floating reduction, compiled-plan refactors, checked scenario builders, and macros.

## Current-State Facts

- `CaptureConfig::disabled()` inherits stride `1`; positive steps still capture
  (`src/types/config.rs:60-76`, `src/engine/mod.rs:2113-2124`).
- Empty node/metric sets mean all; variables and transfers are always retained with no public
  selection (`src/engine/mod.rs:2038-2077`, `src/engine/mod.rs:1444-1475`).
- Terminal maps are populated separately from snapshots and series (`src/engine/mod.rs:662-674`).
- Batch executes full reports, aggregates their series, retains compact summaries, and discards the
  remaining report data (`src/batch/mod.rs:19-57`, `src/batch/mod.rs:73-85`).
- Run/batch assertions distinguish terminal values from captured/aggregate step series
  (`src/assertions/mod.rs:327-379`).
- Existing default/parallel benchmark infrastructure has no peak-live-heap target
  (`benches/simulation.rs:416-467`, `scripts/bench-criterion`, `benchmarks/README.md`).

## Decisions

- Use `CaptureSchedule::{None, Final, Every { stride: NonZeroU64, ... }}` and
  `Selection::{None, All, Only(BTreeSet<_>)}` with tagged Serde forms.
- `CaptureConfig` explicitly selects nodes, metrics, variables, and transfers; schedule applies to
  step channels, transfers are selected independently, and final maps/live events remain independent.
- `AggregationConfig` owns only schedule and metrics; `BatchRunTemplate.capture` becomes
  `aggregation` with typed legacy conversion.
- Read legacy wire structs via a private untagged current/legacy adapter; legacy empty sets remain
  all and legacy disabled-looking JSON remains periodic. Write only canonical new fields.
- Use private static `FullReportCollector` and `BatchSampleCollector` paths through one engine loop.
- Sort indexed samples explicitly and fold `f64` sums sequentially by run index. Never parallel-reduce.
- Depend on `037-compiled-scenario-trust-boundary/T004`; consume its private plan/accessor contract
  and do not restore public engine/batch modules or compiled fields. Current validator tooling cannot
  resolve the concrete sibling task edge, so retain the spec dependency, before-implementation
  checkpoint, and hard T001 stop until that metadata edge can be added later. Before any edit, T001
  reads `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and requires exactly
  `status = "done"` and `verification_status = "passed"`; either absent predicate stops the task.
- Land Criterion/DHAT tooling and save named baselines before replacing full reports; compare
  identical workloads/checksums afterward. No numeric performance threshold is a spec gate; memory
  comparison fails only for missing, invalid, or incomparable evidence. The existing non-failing 7%
  Criterion summary is optional descriptive context.
- Deprecate `CaptureConfig::disabled()` as an alias of `none()` for the `0.2` source transition.

Rejected:

- More booleans, `Option<u64>`, empty sentinels, ignored batch capture channels, full reports in
  `BatchReport`, clearing full reports after execution, duplicated engine loops, parallel reduction,
  old+new serialization, and RSS-only memory proof.

Open:

- None.

## Implementation Shape Excerpts

- Slice 1: complete typed single-run policy + custom legacy wire + full collector + call-site/public
  behavior migration across `src/types/config.rs`, `src/engine/mod.rs`, `src/validation/mod.rs`,
  exports, and `tests/capture_retention_policy.rs`; depends on
  `037-compiled-scenario-trust-boundary/T004`, reads its task file for exact done/passed predicates,
  and only then reads finalized `src/plan.rs`.
- Slice 2: separate batch aggregation wire/API, matched Criterion cases, isolated
  `benches/capture_memory.rs`, `scripts/bench-capture-memory`, and saved pre-compact baselines.
- Slice 3: private `BatchSample`, no discarded diagnostics, explicit run-index sort, ordered fold,
  batch/assertion/event/determinism tests, and post-change comparisons.
- Slice 4: README/rustdoc/testkit/artifact/readme-snippet/determinism-checklist closeout.
- Slice 5: fresh Sol/high integration review and remediation covering API/serde, performance, and
  determinism.
- Tasks must be serial: the baseline slice must precede the compact-path slice, and engine/config
  surfaces overlap.

## Suggested Spec Shape

- spec_kind: migration
- fanout_policy: serial
- execution_policy: auto-continue
- task_slices:
  - T001: Typed diagnostic policy and full-run collector migration
  - T002: Batch aggregation policy plus pre-change throughput/heap baselines
  - T003: Compact deterministic batch sample execution and comparisons
  - T004: Public docs, fixture, assertion, and artifact compatibility closeout
  - T005: Independent Sol/high invariant review and remediation

## Validation

- `cargo fmt --all -- --check`
- `cargo check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test --all-targets`
- `cargo test --all-targets --features parallel`
- `cargo test --doc`
- `./scripts/bench-criterion save|compare --bench simulation` in default and `parallel` modes using
  the frozen capture-retention baseline names
- `./scripts/bench-capture-memory save|compare --baseline capture-retention-pre`
- No Docker or external service access is required.

## Worker Context Policy

- Workers may read:
  - `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` for prerequisite status evidence
  - only the concrete source/test/benchmark/doc paths named in their task frontmatter and Context
- Workers must not be sent to:
  - `.orchid/spec-research/explicit-capture-retention-policy/raw/`
  - broad current-state research
  - decision dialogue history
  - stale options or rejected approaches
