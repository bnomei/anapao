# Design Decisions

## D001 — Use explicit typed schedule and selection states

Decision: introduce these public contracts in `src/types/config.rs`:

```rust
pub enum CaptureSchedule {
    None,
    Final,
    Every {
        stride: NonZeroU64,
        include_initial: bool,
        include_final: bool,
    },
}

pub enum Selection<T> {
    None,
    All,
    Only(BTreeSet<T>),
}
```

Both enums are `#[non_exhaustive]`; external matches must include a fallback so a later policy
variant does not force another source break.

`CaptureSchedule` uses `#[serde(tag = "kind", rename_all = "snake_case")]`.
`Selection<T>` uses the adjacent representation
`#[serde(tag = "kind", content = "items", rename_all = "snake_case")]` so `none`, `all`, and a
concrete ordered set are distinguishable in human-readable formats. `Selection::Only(empty)` is
rejected by configuration validation; constructors must not create it.

Rationale:

- `NonZeroU64` makes a zero periodic stride unrepresentable in the new shape.
- An explicit `None` removes the empty-set sentinel; `All` states intent directly.
- `BTreeSet` retains deterministic serialization and iteration.
- Tagged shapes leave room for future variants and avoid ambiguous untagged selection payloads.

Rejected:

- Keep booleans and add a third `enabled` boolean: it preserves contradictory states.
- Use `Option<u64>` for schedule: it still permits `Some(0)` and does not express final-only capture.
- Keep empty sets as all and add an extra `capture_nothing` flag: it duplicates source of truth.
- Use `HashSet`: output ordering would no longer be stable.

## D002 — Make every diagnostic channel explicit; keep final results and events independent

Decision: the new `CaptureConfig` has five private typed fields with read-only accessors and
consuming builders:

```rust
pub struct CaptureConfig {
    schedule: CaptureSchedule,
    nodes: Selection<NodeId>,
    metrics: Selection<MetricKey>,
    variables: Selection<String>,
    transfers: Selection<EdgeId>,
}
```

Semantics:

- `schedule` controls step-aligned node, metric, and variable observations.
- `transfers` independently controls retained `TransferRecord`s because transfers occur as events,
  not as one observation per scheduled step.
- `CaptureConfig::default()` preserves current diagnostics: every step with initial/final dedupe,
  and all nodes, metrics, variables, and transfers.
- `CaptureConfig::none()` uses schedule `None` plus `Selection::None` for every channel. A run still
  returns scenario id, seed, steps, completion, `final_node_values`, `final_metrics`, and manifest,
  but all four diagnostic collections (`node_snapshots`, `variable_snapshots`, `transfers`,
  `series`) are empty.
- `CaptureConfig::final_only()` uses schedule `Final`, all step-aligned channels, and no transfers.
- `CaptureConfig::disabled()` remains for one migration release, is deprecated, and delegates to
  `none()` in the `0.2` API. The intentional behavior correction is documented as breaking.
- `Selection::Only` is validated against compiled node ids, tracked/resolvable metrics, scenario
  variable source names, and edge ids before execution.
- Live events remain complete. `run_with_sink` emits step, metric, transfer, and assertion events
  according to its existing contract even when the returned report retains none of them.

Rationale: variables and transfers are already large diagnostic channels. Leaving them implicit
would make `none()` dishonest and would preserve most of the benchmark allocation problem.
Private fields also make later compatible additions possible and force construction through the
documented invariant-preserving API during the intentional `0.2` break.

Rejected:

- Treat final maps as capture: final values are result data used by assertions, batch summaries,
  and callers even when no trace is requested.
- Couple `EventSink` delivery to report retention: it would silently alter streaming and artifact
  behavior.
- Apply the periodic schedule to transfer records: that would drop within-step events using a rule
  designed for snapshots and series.

## D003 — Add a separate batch aggregation contract

Decision: add a public `AggregationConfig` containing only `schedule: CaptureSchedule` and
`metrics: Selection<MetricKey>`. Rename `BatchRunTemplate.capture` to `aggregation` and add
`with_aggregation` on `BatchRunTemplate` and `BatchConfig`.

`AggregationConfig::default()` preserves current externally visible batch series (all metrics,
every step including initial/final). `AggregationConfig::none()` returns no aggregate series while
per-run summaries and their final metrics remain present. A final-only aggregation groups each
run's final metric observation by that run's terminal step, matching the existing step-aligned
aggregate meaning; `MetricSelector::Final` continues to use the per-run final-metric mean instead.

Deprecated `with_capture(CaptureConfig)` compatibility builders may remain through `0.2`; they map
only the source schedule and metric selection into `AggregationConfig`. Node, variable, and transfer
retention never had an observable batch-report destination.

Rationale: batch aggregation is not diagnostic report retention. A separate type prevents batch
callers from requesting node/variable/transfer data that the public `BatchReport` cannot return.

Rejected:

- Keep `CaptureConfig` in `BatchRunTemplate` but ignore three channels: the type would continue to
  promise behavior that cannot be observed.
- Add full run reports to `BatchReport`: this expands and destabilizes a compact public schema and
  defeats the memory goal.

## D004 — Read legacy JSON faithfully and write only the canonical new shape

Decision: implement private versioned wire intermediates and custom conversion in
`src/types/config.rs`.

- New serialization always emits the typed schedule/selection shape.
- Deserialization accepts new shape first, then the exact legacy five-field `CaptureConfig` shape
  using a private `#[serde(untagged)]` wire enum.
- Legacy empty node/metric sets map to `Selection::All`; non-empty sets map to `Only`.
- Legacy runtime variables and transfers map to `All`, matching historical retention.
- A legacy positive `every_n_steps` plus its two include flags maps to `CaptureSchedule::Every`.
  The flags are not reinterpreted as `None`, because historical positive steps were retained even
  when both flags were false.
- Legacy `every_n_steps == 0` fails deserialization with a precise error; new shape cannot express it.
- `BatchRunTemplate` deserialization accepts legacy `capture` and maps its schedule/metrics to the
  new `aggregation` field. New serialization emits only `aggregation`.
- Compatibility tests use literal JSON fixtures for default, disabled, selective, invalid-zero,
  current canonical, and nested batch forms. They assert semantic conversion and canonical
  re-serialization.

The Rust field change is released as `0.2`; source users of struct literals must migrate. Serde
input compatibility is retained because persisted configs are a separate contract from Rust source
compatibility.

Rejected:

- Emit both old and new fields: it creates two writers of truth and ambiguous mixed payloads.
- Map legacy `disabled`-looking JSON to `None`: that changes persisted behavior rather than only
  correcting the `disabled()` Rust constructor in the breaking release.
- Add an unversioned `serde_json::Value` migration: typed wire structs provide better errors and
  testable exhaustiveness.

## D005 — Introduce private collectors and a compact batch sample

Decision: refactor the engine core around crate-private, statically dispatched collectors without
making a new public result type.

- `FullReportCollector` owns a `RunReport`, applies `CaptureConfig`, and returns the public report.
- `BatchSampleCollector` applies `AggregationConfig` and returns a private `BatchSample` containing
  run metadata, final metrics, manifest, and only requested aggregate metric points.
- The simulation state transition loop, RNG sequence, end-condition checks, expression evaluation,
  and event callback remain one shared execution path.
- Transfer application produces a record only when a full collector retains it or a live event
  callback needs it. The batch collector has neither full transfer retention nor intermediate event
  streaming and therefore avoids the allocation.
- `engine::run_single*` remain crate-private wrappers for a full report after spec 037 finalizes the
  public façade;
  `engine::run_batch_sample` is crate-private and is the only path used by `batch`.
- `batch::run_batch` validates `AggregationConfig::Only` metrics once against the compiled plan
  before starting any sequential/Rayon run; sample collectors do not repeat the same selection walk
  for every seed.
- The collector seam is private and generic/static; there is no per-step trait-object dispatch.

`BatchSample` contains:

```rust
pub(crate) struct BatchSample {
    pub(crate) seed: u64,
    pub(crate) completed: bool,
    pub(crate) steps_executed: u64,
    pub(crate) final_metrics: BTreeMap<MetricKey, f64>,
    pub(crate) aggregate_series: BTreeMap<MetricKey, SeriesTable>,
    pub(crate) manifest: Option<ManifestRef>,
}
```

Rationale: the batch layer needs no node snapshots, variable snapshots, transfers, or final node
map. Static collectors keep one engine and avoid both full-report allocation and duplicated
simulation logic.

Rejected:

- Run full reports and clear fields afterward: peak allocation and construction cost remain.
- Duplicate a lightweight simulation loop in `batch`: it will drift from engine semantics.
- Expose `BatchSample`: it is an internal transport, not a stable report schema.

## D006 — Own deterministic ordering before floating-point aggregation

Decision: every batch sample carries its run index in a private `IndexedBatchSample`. After either
sequential or Rayon execution, `batch` explicitly sorts samples by `run_index`, verifies the range is
complete/no duplicates in debug assertions or tests, and folds metric/step sums sequentially in that
order using the current `(sum, count)` algorithm and ordered maps.

- Rayon parallelizes only independent run execution.
- No parallel reduction is used for `f64` aggregation.
- The final `BatchReport.runs` order is run-index order.
- SingleThread and Rayon equality tests compare all fields except the intentionally different
  `execution_mode`.
- Existing no-`parallel` fallback remains `SingleThread` in the returned report.

Rationale: explicit sorting makes the crate's determinism contract implementation-owned instead of
depending on a broad interpretation of Rayon collection examples. Fixed operand order preserves the
current floating-point result sequence; official `f64` documentation shows that operation/rounding
sequence can change results, while the ordered fold is an engineering choice validated by tests.

Rejected:

- `par_iter().reduce` sums: Rayon may associate operands differently.
- Trust collection order without a local invariant: current tests cover it, but an explicit compact
  sort is cheap and auditable.
- Change to compensated or exact summation in the same spec: that intentionally changes aggregate
  values and requires its own numerical contract.

## D007 — Baseline throughput and peak live heap before the compact-path change

Decision: add permanent developer tooling, not a one-off measurement.

- Extend `benches/simulation.rs` with clearly named, matched capture/aggregation cases for full,
  none, and periodic/selective policies in SingleThread and Rayon where applicable.
- Preserve existing benchmark IDs when they still represent the same operation; correct misleading
  `disabled` setup to explicit `none` and add companion full-capture IDs so comparisons remain
  interpretable.
- Add `dhat` as a dev-dependency and a `harness = false` bench target at
  `benches/capture_memory.rs`. One invocation runs one deterministic case in an isolated process and
  prints machine-readable `total_bytes`, `max_bytes`, and retained report checksum.
- Add `scripts/bench-capture-memory` with `save` and `compare` modes. Baselines live under `target/`;
  comparison reports absolute and relative `max_bytes` deltas and fails only when evidence is
  missing, invalid, or incomparable. It does not impose an improvement percentage.
- Use existing `scripts/bench-criterion` for throughput baselines. Matched pre/post comparisons are
  required. The repository's existing non-failing 7% summary may be retained as an optional,
  descriptive flagging view, but it is not a spec pass/fail gate.
- Capture pre-refactor Criterion and DHAT baselines after the public policy migration but before
  switching `batch` to `BatchSample`; compare after the switch.
- Document release-only, isolated-process measurement commands in `benchmarks/README.md`.

Rationale: Criterion already supplies feature-aware time/throughput workflows. DHAT directly exposes
peak live heap (`max_bytes`) and is cross-platform, while process RSS mixes allocator/runtime noise.
The original finding requires trustworthy matched evidence before accepting the representation
replacement, not invented numeric success thresholds. Identical workloads and checksums, named
pre-change baselines, host/toolchain metadata, and patient same-environment reruns make that evidence
reviewable. Repeatable regressions or results that contradict the compact-allocation premise must be
explained and escalated for an explicit owner decision before completion. Workers must not invent or
silently change thresholds. Correctness and deterministic SingleThread/Rayon behavior remain hard
pass/fail gates that performance evidence cannot waive. DHAT remains dev-only because its official
docs call it experimental and warn about global state.

Rejected:

- Use only `/usr/bin/time` RSS: it is platform-specific and too coarse for a contract about retained
  Rust heap structures.
- Add memory checks to ordinary parallel unit tests: the global allocator/profiler state would make
  them fragile.
- Measure only after the refactor: there would be no trustworthy before/after evidence.

## D008 — Complete migration, docs, and review in one release scope

Decision: update crate-root/prelude exports, rustdoc, README snippets, testkit defaults, all internal
call sites, local determinism guidance, and assertion/artifact tests as part of this spec. No legacy
field names or misleading `disabled` claims remain in active examples.

Before completion, fresh Sol/high validation must independently review:

1. public API and bidirectional Serde compatibility;
2. the validity and reproducibility of throughput/peak-memory evidence;
3. sequential/Rayon run ordering and floating-point fold invariants.

Open decisions: none.

## D009 — Consume the finalized compiled-plan façade from spec 037

Decision: this spec depends on `037-compiled-scenario-trust-boundary`, and its first implementation
task normatively requires `037-compiled-scenario-trust-boundary/T004`.

- Capture-selection validation and collectors read scenario/plan data only through the finalized
  crate-private plan API/accessors established by 037.
- Before any 038 edit, T001 reads
  `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and requires exactly
  `status = "done"` plus `verification_status = "passed"`; absence or any other value is a hard stop.
- This spec does not re-publicize `engine`, `batch`, compiled fields, or execution-plan internals.
- `src/plan.rs` is read-only context for 038 unless a defect in 037 is separately escalated; 038's
  write scope does not absorb compiled-plan design.

Rationale: targeted `orchid ready --spec` can bypass numeric queue order, while 038's engine/batch
changes consume the exact internal surface finalized by 037/T004. Encoding the task prerequisite
prevents implementation against stale public fields.

Tooling constraint discovered during spec compilation: the current `validate_task_spec.py` rejects
the documented `037-compiled-scenario-trust-boundary/T004` task dependency as unknown even though
the sibling task file exists. The active spec therefore keeps the spec-level dependency, uses a
`before-implementation` checkpoint, and places a hard stop in T001 Context/Escalate. The concrete
frontmatter edge remains a required later metadata update when sibling resolution is supported; it
is not being reclassified as optional.

Rejected: treating the overlap as merge-only after the sibling design was disclosed. The final
accessor/module-visibility contract is an actual input to 038.
