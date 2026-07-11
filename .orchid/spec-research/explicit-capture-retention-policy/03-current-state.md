# Current State

## Public configuration contract

- The crate is version `0.1.1`, uses Rust edition 2021, declares MSRV 1.85, and exposes optional
  Rayon through the `parallel` feature (`Cargo.toml:1-18`).
- `types` describes its contents as serde-stable cross-module contracts and publicly re-exports the
  entire private `config` and `reports` modules (`src/types/mod.rs:1-17`).
- `CaptureConfig` has five public fields: two `BTreeSet` selections, a raw `u64` interval, and two
  booleans. Derived `Serialize`/`Deserialize` directly expose that struct shape
  (`src/types/config.rs:47-58`).
- `CaptureConfig::default()` uses empty node/metric sets, stride `1`, and both initial/final flags
  true (`src/types/config.rs:60-69`).
- `CaptureConfig::disabled()` changes only the two flags and inherits stride `1` plus empty sets
  (`src/types/config.rs:72-76`). Its rustdoc says it is throughput-oriented even though it does not
  stop positive-step capture (`src/types/config.rs:47-50`).
- `RunConfig` contains public `seed`, `max_steps`, and `capture` fields and has fluent setters
  (`src/types/config.rs:79-125`). `BatchRunTemplate` contains public `max_steps` and the same
  `CaptureConfig`; `to_run_config` clones it into every run (`src/types/config.rs:127-159`).
- `BatchConfig` publicly embeds `BatchRunTemplate` and forwards `with_capture` into that nested
  capture field (`src/types/config.rs:161-235`).
- `CaptureConfig`, `BatchRunTemplate`, and the containing configs are re-exported from the crate root
  and prelude (`src/lib.rs:144-150`, `src/prelude.rs:6-11`).
- Unit tests cover defaults and builder mutation through direct field access, but there is no
  capture-specific legacy/current JSON compatibility matrix (`src/types/mod.rs:329-381`).

## Validation and step capture

- Setup validation rejects `max_steps == 0` and `every_n_steps == 0` for both run and batch nested
  configs (`src/validation/mod.rs:104-158`). A focused test protects the zero interval error path
  (`src/validation/mod.rs:2214-2236`).
- The engine separately coerces the interval through `.max(1)` before modulo capture
  (`src/engine/mod.rs:2113-2124`), even though the public façade validates first.
- `run_single_internal` validates concrete node/metric selections against the compiled scenario,
  creates a full `RunReport`, an unconditional transfer log, and a captured-step set, then attempts
  capture at initialization, after every executed step, and optionally once more at the final state
  (`src/engine/mod.rs:529-559`, `src/engine/mod.rs:580-660`).
- Empty `capture_nodes` means all compiled nodes. A non-empty set filters node snapshots
  (`src/engine/mod.rs:2038-2048`).
- Runtime variables are always fully snapshotted at each selected step when any runtime variable
  exists; `CaptureConfig` has no variable selection (`src/engine/mod.rs:2050-2058`).
- Empty `capture_metrics` means all current metrics. A non-empty set filters series keys
  (`src/engine/mod.rs:2060-2077`).
- Concrete node and metric selections are validated at execution time; unknown nodes or metrics
  become `RunError::InvalidRunConfig` (`src/engine/mod.rs:2080-2110`).
- Every accepted transfer unconditionally creates and pushes a `TransferRecord`
  (`src/engine/mod.rs:1444-1475`). The complete vector is copied into `RunReport.transfers` at the
  end (`src/engine/mod.rs:662-675`).
- Terminal node values and terminal metrics are populated independently of scheduled snapshots
  (`src/engine/mod.rs:662-674`).
- `RunReport` stores node snapshots, variable snapshots, transfers, metric series, final node
  values, and final metrics as public serde fields (`src/types/reports.rs:82-115`).

## Live events, assertions, and artifacts

- The same run core accepts an event callback. Transfer events are emitted from newly appended
  transfer records during each step; metric snapshot events iterate every current metric after each
  step (`src/engine/mod.rs:529-534`, `src/engine/mod.rs:595-632`,
  `src/engine/mod.rs:679-694`).
- `Simulator::run` and `run_with_sink` select non-streaming or streaming engine wrappers after
  validating the same `RunConfig`; event streaming is not configured through `CaptureConfig`
  (`src/simulator.rs:37-79`).
- Run assertions read `final_metrics` for `MetricSelector::Final` and `series` for a step selector;
  batch assertions read the mean of per-run final metrics for `Final` and `aggregate_series` for a
  step selector (`src/assertions/mod.rs:327-379`). Monotonic and run probability-band assertions
  also require a captured series (`src/assertions/mod.rs:397-420`,
  `src/assertions/mod.rs:498-519`).
- Run artifact writing always emits events, variable CSV, history/replay indexes, and series CSV;
  empty collections therefore produce valid empty data files rather than changing manifest shape
  (`src/artifact/mod.rs:110-175`, `src/artifact/mod.rs:308-360`).
- Batch artifact writing consumes only `BatchReport.aggregate_series` and compact per-run summary
  data (`src/artifact/mod.rs:177-226`).

## Batch execution and deterministic aggregation

- `run_batch` asks `execute_runs` for a `Vec<IndexedRunReport>`, aggregates series by borrowing all
  full reports, then consumes each report to retain only seed, completion, step count, final metrics,
  and manifest in `BatchRunSummary` (`src/batch/mod.rs:19-57`).
- Each batch entry calls the full `engine::run_single` with a cloned nested capture configuration
  (`src/batch/mod.rs:60-85`). Node snapshots, variable snapshots, transfer records, and final node
  values have no path into `BatchReport`.
- Sequential execution collects increasing run indices. With `parallel`, the same integer range is
  mapped with Rayon and collected (`src/batch/mod.rs:60-105`). Without the feature, requested Rayon
  execution falls back to the sequential path (`src/batch/mod.rs:100-118`).
- Aggregation iterates the collected reports, then each ordered metric table and point, accumulating
  `(f64 sum, u64 count)` by metric and step before dividing (`src/batch/mod.rs:120-148`).
- Unit and integration tests require stable run-index ordering, replay equality, and equality of
  sequential/Rayon run summaries and aggregate series (`src/batch/mod.rs:163-279`,
  `tests/perf_determinism.rs:249-315`).
- `BatchReport.runs` and `aggregate_series` are public serde outputs. `BatchRunSummary.final_metrics`
  supports final-value batch assertions and batch summary events (`src/types/reports.rs:117-177`,
  `src/simulator.rs:244-275`).

## Documentation and performance tooling

- README Step 3 teaches direct construction of the five-field capture struct and explicitly reads
  `every_n_steps` (`README.md:88-111`). `tests/readme_snippets.rs:29-41` compiles the same pattern.
- README Step 7 describes batch config but does not explain that nested capture drives aggregate
  series or that other captured diagnostics are discarded (`README.md:220-266`).
- The local determinism checklist describes `CaptureConfig::disabled` as throughput-oriented and
  says it removes some snapshot evidence (`skills/anapao/references/determinism-checklist.md:27-28`).
- Hotspot benchmarks use `CaptureConfig::disabled()` for single-run expression, gate, state, and
  batch expression cases (`benches/simulation.rs:516-614`). Under current semantics these cases
  retain every positive-step node/variable/metric capture and all transfers.
- Existing Criterion helpers support default/parallel baseline save, compare, a non-failing `+7%`
  regression summary, and feature-aware flamegraph profiling (`scripts/bench-criterion`,
  `benchmarks/run_profiles.sh`, `benchmarks/README.md`).
- Existing benchmark code already sets `Throughput::Elements` for batch cases
  (`benches/simulation.rs:416-467`, `benches/simulation.rs:574-614`).
- The repository has no dedicated allocation-count or peak-live-heap benchmark target and no
  checked memory baseline command.

## External evidence captured on 2026-07-11

- Tavily request `63fdbce4-e56b-4ce3-82ba-5bc1eaa9d842` found primary support for richer enum types,
  `NonZeroU64`, Serde enum representations, and Cargo breaking-change classification. It explicitly
  marked compact batch samples and ordered folding as engineering inference, not a library mandate
  (`raw/tavily-rust-api-research.md`).
- Tavily request `e3d6336c-be05-4a18-936d-d636ba17f477` found Rayon indexed-iterator examples but no
  single normative sentence that should be treated as a blanket `collect::<Vec<_>>()` ordering
  guarantee. The current repository behavior is therefore protected primarily by its equality
  tests (`raw/tavily-rayon-benchmark-research.md`).
- Official Serde documentation defines adjacently tagged enums and untagged alternatives; official
  Cargo documentation classifies changing all-public structs in ways that break struct literals as
  major changes; official `dhat` docs expose `HeapStats.max_bytes` and recommend isolated processes
  for high-water-mark checks; Criterion documentation describes `BenchmarkGroup::throughput`
  (`raw/primary-doc-followup.md`).

## Dependency finding

- Sibling spec `037-compiled-scenario-trust-boundary` is scheduled to move immutable compiled data
  behind `src/plan.rs`, replace direct `CompiledScenario` field reads with accessors, and make raw
  `engine`/`batch` modules crate-private. Its T004 finalizes that façade. Capture selection and batch
  sample code must be written against that verified done/passed internal plan/accessor surface, so
  `037-compiled-scenario-trust-boundary/T004` is a concrete prerequisite for this spec's first task,
  not merely a merge-conflict concern.
- Completion is evidenced by the sibling task file frontmatter containing exactly
  `status = "done"` and `verification_status = "passed"`; neither task existence nor prose claiming
  completion is sufficient.
- The checked-authoring and macro sibling specs are not semantic prerequisites for capture or batch
  aggregation.
