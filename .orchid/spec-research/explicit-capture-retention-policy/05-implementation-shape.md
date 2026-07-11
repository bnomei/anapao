# Implementation Shape

## Ownership map

```text
src/types/config.rs                 public policies, builders, custom serde wires
specs/037-compiled-scenario-trust-boundary/tasks/T004.md
                                      read-only prerequisite status evidence
src/plan.rs                         read-only compiled-plan/accessor contract from 037/T004
src/types/mod.rs                    type/serde compatibility tests
src/lib.rs                          crate-root exports and rustdoc concepts
src/prelude.rs                      prelude exports
src/validation/mod.rs               structural policy validation
src/engine/mod.rs                   shared run core, collectors, capture/event separation
src/batch/mod.rs                    indexed compact samples, ordered fold, batch tests
src/simulator.rs                    façade validation and event/assertion compatibility tests
src/testkit/mod.rs                  canonical typed run/batch fixtures
src/assertions/mod.rs               missing-series/final-value compatibility tests
src/types/reports.rs                report docs; no public shape expansion
tests/capture_retention_policy.rs   public behavior and serde-facing integration coverage
tests/perf_determinism.rs           sequential/Rayon report equivalence
tests/failure_path_batch_events.rs  batch event/failure compatibility
tests/readme_snippets.rs            compiled README contract
benches/simulation.rs               matched throughput cases
benches/capture_memory.rs           isolated DHAT peak-live-heap target
scripts/bench-capture-memory        save/compare memory workflow
benchmarks/README.md                throughput and memory commands
README.md                           public migration and examples
CHANGELOG.md                        `0.2` breaking-source/compatible-wire migration note
skills/anapao/references/determinism-checklist.md
                                      local capture/determinism guidance
Cargo.toml                           `dhat = "0.3.3"` dev dependency and bench target
Cargo.lock                           locked dev dependency graph
```

No other source file is required. New tests go at the exact paths above rather than a broad test
glob. `BatchSample`, collectors, and wire structs stay crate-private/private.

## Public contracts

### Schedule and selection

In `src/types/config.rs`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureSchedule {
    None,
    Final,
    Every {
        stride: NonZeroU64,
        include_initial: bool,
        include_final: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", content = "items", rename_all = "snake_case")]
pub enum Selection<T> {
    None,
    All,
    Only(BTreeSet<T>),
}
```

Required helpers:

- `CaptureSchedule::every(stride: NonZeroU64) -> Self` with initial/final enabled.
- read-only predicates used by engine code (`is_none`, `includes`) without duplicating match logic.
- `Default` only where the meaning is unambiguous and documented; `CaptureConfig::default` and
  `AggregationConfig::default` explicitly construct all-selection policies.

Do not introduce a second public empty-selection error solely for a convenience constructor.
Callers construct `Selection::Only(BTreeSet<_>)` directly; config validation and deserialization
reject an empty payload before execution. Config `with_*` builders accept the typed `Selection`
value unchanged.

### Diagnostic and aggregation configs

```rust
pub struct CaptureConfig {
    schedule: CaptureSchedule,
    nodes: Selection<NodeId>,
    metrics: Selection<MetricKey>,
    variables: Selection<String>,
    transfers: Selection<EdgeId>,
}

pub struct AggregationConfig {
    schedule: CaptureSchedule,
    metrics: Selection<MetricKey>,
}
```

Both types derive/implement `Debug`, `Clone`, `PartialEq`, `Eq`, `Serialize`, and `Deserialize` at the
public contract. Required constructors/builders are `default`, `none`, `final_only`,
`with_schedule`, and one `with_*` per selection channel. Fields remain private; expose read-only
`schedule()`, `nodes()`, `metrics()`, `variables()`, and `transfers()` accessors as applicable.
Consuming builders are `#[must_use]`.

`CaptureConfig::disabled` is retained with `#[deprecated(note = "use CaptureConfig::none")]
and delegates to `none`. `BatchRunTemplate` owns `aggregation: AggregationConfig`, and its/current
`BatchConfig` builders use `with_aggregation`. Deprecated `with_capture` adapters are allowed only
as the documented `0.2` source transition and must have tests proving their metric/schedule mapping.

Crate root and prelude re-export `AggregationConfig`, `CaptureSchedule`, and `Selection` beside the
existing config types.

## Wire migration

`src/types/config.rs` owns private `CaptureConfigWire`, `CaptureConfigV2Wire`,
`LegacyCaptureConfigWire`, `BatchRunTemplateWire`, and legacy/current nested structs. Conversion is
typed; it does not inspect raw JSON keys manually.

Canonical examples:

```json
{
  "schedule": {
    "kind": "every",
    "stride": 5,
    "include_initial": true,
    "include_final": true
  },
  "nodes": { "kind": "all" },
  "metrics": { "kind": "only", "items": ["sink"] },
  "variables": { "kind": "none" },
  "transfers": { "kind": "none" }
}
```

```json
{
  "max_steps": 50,
  "aggregation": {
    "schedule": { "kind": "none" },
    "metrics": { "kind": "none" }
  }
}
```

Legacy inputs retain the current five capture fields. Empty legacy node/metric arrays become `All`;
positive interval and flags become `Every`; legacy variables/transfers become `All`. New output never
contains `capture_nodes`, `capture_metrics`, `every_n_steps`, `include_step_zero`, or
`include_final_state`. Nested legacy batch `capture` is accepted but new output uses `aggregation`.

Test literals in `src/types/mod.rs` and `tests/capture_retention_policy.rs` must cover:

- current default, none, final-only, and selective canonical round trips;
- legacy default and legacy disabled-looking payloads preserving actual positive-step behavior;
- legacy selective sets and nested `BatchConfig`/`BatchRunTemplate` payloads;
- zero legacy stride rejection;
- empty current `Selection::Only` rejection before execution;
- canonical re-serialization containing no legacy field names;
- readable path-specific errors for unknown Only selections.

## Run-core and collector flow

After `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` contains exactly
`status = "done"` and `verification_status = "passed"`, `src/engine/mod.rs` retains one crate-private
state-transition function. Its generic collector is selected by wrapper, while event emission
remains a separate callback. All compiled data is read through the finalized `src/plan.rs`
accessors; this spec does not restore direct public fields:

```text
Simulator::run[*]
  -> validate RunConfig + concrete selections
  -> run_core(FullReportCollector, optional EventSink callback)
  -> RunReport

batch::execute_run
  -> validate BatchRunTemplate/AggregationConfig
  -> engine::run_batch_sample(BatchSampleCollector, no intermediate EventSink)
  -> BatchSample
```

The full collector:

- evaluates the schedule once per state step;
- de-duplicates an interval/final collision;
- applies node, metric, and variable selections independently;
- accepts transfer records according to `Selection<EdgeId>`;
- always materializes terminal node and metric maps;
- returns empty diagnostics for `CaptureConfig::none()`.

The batch collector:

- evaluates only aggregation schedule and metric selection;
- does not allocate node snapshots, variable snapshots, transfer logs, or final node maps;
- retains terminal metrics for `BatchRunSummary` and requested metric points for folding.

Transfer application must not require an ever-growing vector merely to emit live events. A
per-transfer value may be sent to the collector and event callback; full retention clones/moves it
only when requested. Tests must prove `run_with_sink(..., CaptureConfig::none())` returns empty
diagnostics while emitting the same ordered event stream as default capture for the same run.

## Ordered batch fold

`src/batch/mod.rs` replaces `IndexedRunReport` with private `IndexedBatchSample`.

1. Derive every seed exactly as today.
2. Validate an `AggregationConfig::Only` metric set once against the compiled plan before any run.
3. Execute independent samples sequentially or with Rayon without repeating selection validation.
4. Explicitly `sort_by_key(|sample| sample.run_index)` for both modes.
5. Build `BatchRunSummary` in that order.
6. Fold each sample's ordered metric/step points into the existing ordered `(sum, count)` map in
   run-index order.
7. Divide only after all ordered additions for a metric/step.
8. Return the resolved execution mode exactly as today.

Do not use Rayon reduction for sums. Do not change seed derivation, current point averaging (runs
that reach a step), final-metric summary contents, completed-run counting, or fallback mode.

Required batch tests:

- aggregation `None`, `Final`, `Every`, `All`, and `Only` semantics;
- unknown/empty metric selection errors;
- ordered, complete run indices and derived seeds;
- repeated replay equality for both aggregation-none and aggregation-all;
- SingleThread/Rayon equality for runs and aggregate series;
- no-feature Rayon fallback;
- mixed terminal steps preserve current per-step count/mean semantics;
- final-value assertions pass with aggregation none; step/series assertions report missing data;
- batch summary events remain complete because they use final metrics, not aggregate series.

## Performance evidence seam

Before changing `batch` to compact samples, land the final benchmark cases and save baselines.

Criterion cases in `benches/simulation.rs`:

- single run: full/default, none, final-only, and periodic/selective;
- batch SingleThread: aggregation all and none;
- batch Rayon (feature-gated): aggregation all and none.

Use existing deterministic fixtures and `Throughput::Elements(config.runs)`. Checksums must consume
report fields so the optimizer cannot discard results.

`benches/capture_memory.rs` is a custom `harness = false` target with the
`dhat = "0.3.3"` allocator. It accepts
one case per process, runs release-mode deterministic single/batch workloads, calls
`dhat::HeapStats::get()`, and prints a JSON object containing case, total/max/current bytes, run
count, step count, and checksum. It must not run as part of ordinary `cargo test --all-targets`.

The required memory workload is named `batch_none_256x256`: start from the deterministic source/sink
fixture, set its end condition to 256 steps, run 256 seeds in SingleThread mode, and use aggregation
none. Companion cases `batch_all_256x256` and `single_full_256` use the same scenario/step budget.
This guarantees the pre-compact batch path creates a meaningful transfer/report high-water mark and
keeps before/after inputs identical.

`scripts/bench-capture-memory` supports:

- `save --baseline <name>`: run each target case separately and store JSON under
  `target/capture-memory/<name>/`;
- `compare --baseline <name>`: rerun and report absolute and relative deltas; fail only when the
  named evidence is missing, invalid, or incomparable, not for an unapproved improvement percentage.

Execution order:

1. add final benchmark tooling;
2. save pre-compact Criterion/DHAT baselines through a bounded bridge that starts from default full
   retention and replaces only aggregation schedule/metrics, leaving discarded transfers retained;
3. implement `BatchSample` path;
4. compare post-change;
5. retain benchmark tooling and production names, removing no temporary tracer flags/files.

Performance-evidence acceptance:

- matched pre/post Criterion throughput and isolated DHAT peak-live-heap evidence exists for the
  named baselines, with identical workloads and consumed checksums;
- evidence records host/toolchain metadata, exact case IDs, absolute/relative deltas, and patient
  same-environment reruns where noise makes a conclusion unstable;
- the repository's existing non-failing 7% Criterion summary may be shown as an optional,
  descriptive flagging view, but no percentage is a spec pass/fail threshold;
- default and `parallel` feature cases pass correctness and determinism as hard gates before
  performance results are accepted;
- repeatable regressions or results contradicting the compact-allocation premise are explained and
  escalated for explicit owner decision before completion. Workers do not invent or silently change
  thresholds.

## Documentation and compatibility

- `README.md` Step 3 shows `CaptureConfig::none`, final-only, and periodic/selective builders rather
  than struct fields; Step 7 uses `AggregationConfig` and explains final-vs-step assertions.
- `src/lib.rs` concept docs distinguish final results, retained diagnostics, streamed events, and
  batch aggregate sampling.
- `CHANGELOG.md` records the `0.2` source break, deprecated compatibility method, and legacy-input /
  canonical-output wire behavior.
- `skills/anapao/references/determinism-checklist.md` removes the false `disabled` claim and states
  the ordered-fold invariant.
- `src/testkit/mod.rs` uses explicit typed defaults but preserves existing fixture outputs.
- `tests/readme_snippets.rs` compiles final public examples and checks new snippet markers.
- Artifact tests assert that explicit no-capture/no-aggregation writes valid empty series/variables
  files without changing manifest ownership, while event artifacts remain driven by supplied events.

## Vertical implementation slices

1. Typed single-run policy, custom legacy wire, engine full collector, call-site migration, and
   public behavior tests. This slice is green only when all targets compile and run behavior is
   complete; it is not a types-only stub.
2. Separate batch aggregation type/wire plus permanent Criterion/DHAT harness; save the pre-compact
   baseline while batch still uses full reports.
3. Private compact batch collector/sample, explicit ordering, deterministic fold, feature-matrix
   tests, and post-change performance comparison.
4. Public docs, fixtures, artifact/assertion compatibility, and migration notes.
5. Fresh independent Sol/high review/remediation for API/serde, performance evidence, and
   deterministic concurrency invariants.

Slices are serial because slice 2 must measure the full-report path before slice 3 replaces it and
because the first three slices overlap `src/types/config.rs`/`src/engine/mod.rs`.

The first slice has the normative cross-spec prerequisite
`037-compiled-scenario-trust-boundary/T004`. Current task validation cannot resolve sibling task
edges, so the compiled spec uses `spec.toml.depends_on`, a before-implementation checkpoint, and a
hard T001 stop condition until the concrete frontmatter edge can be added. Before any edit, the
worker must read `specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and require exactly
`status = "done"` and `verification_status = "passed"`; either absent predicate stops the task.
Only then may the worker inspect finalized `src/plan.rs` and facade/module visibility, and it must
escalate rather than bypass those invariants.

## Validation

```text
cargo fmt --all -- --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo test --all-targets --features parallel
cargo test --doc
./scripts/bench-criterion save --bench simulation --baseline capture-retention-pre
./scripts/bench-criterion compare --bench simulation --baseline capture-retention-pre
./scripts/bench-criterion save --bench simulation --features parallel --baseline capture-retention-pre-parallel
./scripts/bench-criterion compare --bench simulation --features parallel --baseline capture-retention-pre-parallel
./scripts/bench-capture-memory save --baseline capture-retention-pre
./scripts/bench-capture-memory compare --baseline capture-retention-pre
```

No Docker or external service access is required. Initial `dhat` resolution may require approved
network access when not cached. Benchmark comparisons must record the host/toolchain metadata and
exact case IDs so a reviewer can distinguish signal from environmental noise.

## Anti-goals and stop conditions

- Stop if implementation would change RNG draws, state transitions, event order, final result maps,
  or public report schemas; route that as a separate design decision.
- Stop if legacy JSON behavior cannot be identified unambiguously; do not silently reinterpret it.
- Stop if Rayon output cannot be normalized and folded in run-index order.
- Stop if performance evidence is collected only after the compact refactor or without an isolated
  peak-live-heap measurement.
- Do not publish collectors, `BatchSample`, wire structs, or benchmark-only types.
- Do not leave deprecated field names in current serialization or active README examples.
- Do not own the aggregate `0.2` Cargo version bump or crate publication; use CHANGELOG Unreleased.
