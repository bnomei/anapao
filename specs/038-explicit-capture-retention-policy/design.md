# Design — 038 Explicit Capture And Retention Policy

## Objective

Ship a complete `0.2` capture-policy migration: typed diagnostic and aggregation states, faithful
legacy reads and canonical writes, one engine transition loop with full-report and compact-batch
collectors, deterministic ordered aggregation, persistent throughput/heap tooling, and aligned public
documentation.

## Prerequisite and source-of-truth boundary

This spec depends on `037-compiled-scenario-trust-boundary`; T001 normatively requires
`037-compiled-scenario-trust-boundary/T004`. The current task validator rejects the documented
cross-spec `other-spec/T004` frontmatter syntax even when the target exists, so the machine edge must
be added when Orchid/validator supports it. Until then, `spec.toml.depends_on`, the
`before-implementation` human checkpoint, and T001's hard Context/Escalate guard prevent execution
against an unverified plan. Before any edit, T001 reads
`specs/037-compiled-scenario-trust-boundary/tasks/T004.md` and proceeds only when its frontmatter has
exactly `status = "done"` and `verification_status = "passed"`; either missing predicate is a hard
stop. Only then is compiled data read through the private `src/plan.rs` contract/accessors and raw
`engine`/`batch` modules are crate-private. Workers must consume that surface, not restore direct
public fields or module exports.

No checked-authoring or macro sibling output is required.
The aggregate Cargo package-version bump/publish operation remains outside this spec, matching 037;
record the source/wire migration under `CHANGELOG.md` Unreleased.

## Current-state facts

- `CaptureConfig` currently exposes empty sets, raw `u64`, and two booleans directly through Serde.
  `disabled()` only clears initial/final flags and therefore captures every positive step
  (`src/types/config.rs:47-76`, `src/engine/mod.rs:2113-2124`).
- Empty node/metric selections mean all; variables and transfers have no selection and are retained
  whenever present (`src/engine/mod.rs:2038-2077`, `src/engine/mod.rs:1444-1475`).
- Final maps are calculated separately from diagnostic capture and support final assertions
  (`src/engine/mod.rs:662-674`, `src/assertions/mod.rs:327-379`).
- Batch execution constructs a `Vec` of full `RunReport`s, folds their series, and discards node
  snapshots, variable snapshots, transfers, and final node maps (`src/batch/mod.rs:19-57`).
- Default and Rayon tests already require stable run order and equal aggregates
  (`src/batch/mod.rs:163-279`, `tests/perf_determinism.rs:249-315`).
- Existing Criterion workflows cover default/parallel throughput but not peak live heap
  (`scripts/bench-criterion`, `benchmarks/README.md`).

## Public type model

`src/types/config.rs` introduces:

```rust
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum CaptureSchedule {
    None,
    Final,
    Every {
        stride: NonZeroU64,
        include_initial: bool,
        include_final: bool,
    },
}

#[serde(tag = "kind", content = "items", rename_all = "snake_case")]
#[non_exhaustive]
pub enum Selection<T> {
    None,
    All,
    Only(BTreeSet<T>),
}

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

`Selection::Only(empty)` is rejected by deserialization/config validation before execution; no
separate public error type is added only for a convenience constructor. `CaptureConfig::default()`
retains all diagnostics at every step with initial/final de-duplication. Both config types use
private fields, read-only accessors, and `#[must_use]` consuming builders so future channels can be
added without another public-field break. `CaptureConfig::none()` disables all four diagnostic report
collections, while `final_node_values` and `final_metrics` remain mandatory results.
`final_only()` retains final step-aligned values but no transfers.

`CaptureConfig::disabled()` remains deprecated for the `0.2` transition and aliases `none()`.
Current examples use `none`, never `disabled`. `AggregationConfig` has analogous default/none/final
schedule semantics but only selects metrics. `BatchRunTemplate.capture` becomes `aggregation`, with
`with_aggregation` on template/config.

## Wire compatibility

Private typed wire enums accept current shape first and the exact legacy shape second using
`#[serde(untagged)]`. New serialization emits one canonical tagged shape.

Legacy conversion is fixed:

- empty `capture_nodes`/`capture_metrics` become `Selection::All`;
- non-empty sets become `Only`;
- variables and transfers become `All` because they were historically retained;
- positive interval plus both flags becomes `Every`, even if both flags are false;
- zero interval is a deserialization error;
- nested legacy batch `capture` contributes only schedule and metrics to `aggregation`.

The Rust struct-field break lands in `0.2`; persisted input compatibility remains. Mixed old/new
payloads do not gain a second source of truth, and output contains no legacy capture field names.

## Engine collector architecture

One crate-private transition loop remains after 037. It accepts a statically dispatched private
collector and an independent optional event callback:

```text
public Simulator façade
  -> validate config/selections
  -> crate-private engine run core
       FullReportCollector -> RunReport
       BatchSampleCollector -> private BatchSample
  -> optional live EventSink remains independent
```

`FullReportCollector` applies step schedule to node/metric/variable selections, applies edge
selection to transfer records, de-duplicates final capture, and always finalizes terminal maps.
`BatchSampleCollector` retains only seed/completion/steps/final metrics/manifest and requested
metric points. It never allocates discarded report channels or a final-node map.

Transfer application produces a record only when the full collector retains it or live emission
needs it. Disabling returned diagnostics must not suppress any event from streaming façade methods.

## Batch data flow and determinism

`src/batch/mod.rs` uses private `IndexedBatchSample { run_index, sample }` values.

1. Seeds remain `derive_run_seed(base_seed, run_index)`.
2. The batch entry validates concrete aggregate metric selections once against the compiled plan
   before starting any run.
3. Independent runs execute sequentially or through Rayon without repeating that validation.
4. Collected samples are explicitly sorted by `run_index` in both paths.
5. Ordered samples create public `BatchRunSummary` rows.
6. Requested metric points fold into the existing `BTreeMap<MetricKey, BTreeMap<step,
   (sum,count)>>` in run-index order.
7. Division happens only after ordered addition completes.

Rayon does not reduce `f64`. The explicit sort makes order a local invariant rather than relying on
a generalized reading of Rayon examples. The no-feature fallback and current meaning of
`completed_runs`, final metrics, and per-step averages do not change.

## Assertion, event, and artifact compatibility

- Final run assertions use `RunReport.final_metrics`; final batch assertions use per-run final
  metrics. They work under capture/aggregation none.
- Step, monotonic, and series probability assertions report missing evidence when their series was
  explicitly not requested.
- Batch summary events continue to use per-run final metrics, not aggregate series.
- `run_with_sink` and assertion-streaming methods emit complete ordered events independently of
  report retention.
- Artifact writers continue producing their current manifest-owned files; absent variable/series
  data yields valid empty CSVs and supplied events still drive event/history/replay artifacts.

## Performance proof and tooling

The batch representation changes only after a pre-change baseline is saved.

- `benches/simulation.rs` gains matched full, none, final, and selective periodic single-run cases;
  SingleThread/Rayon batch cases compare aggregation all and none. Existing IDs remain when their
  operation remains equivalent; misleading disabled setups become explicit none.
- `benches/capture_memory.rs` is a custom `harness = false`, one-case-per-process
  `dhat = "0.3.3"` target. It
  prints case metadata, checksum, `total_bytes`, `max_bytes`, and `curr_bytes` as JSON.
- Its fixed cases are `single_full_256`, `batch_none_256x256`, and `batch_all_256x256`. The batch
  cases use 256 runs of the deterministic source/sink fixture with its end condition set to 256
  steps, ensuring the pre-change path materializes a meaningful discarded-report high-water mark.
- `scripts/bench-capture-memory` saves/compares named JSON baselines under `target/`, reports
  absolute and relative deltas, and fails only for missing, invalid, or incomparable evidence. It
  does not enforce an improvement percentage.
- Existing `scripts/bench-criterion` saves/compares default and `parallel` baselines. Its existing
  non-failing 7% summary may be used as an optional descriptive flagging view, but no percentage is
  a spec pass/fail gate.
- Matched pre/post evidence uses identical workloads and consumed checksums, records host/toolchain
  metadata and exact case IDs, and is patiently rerun in the same environment when noise makes a
  conclusion unstable. Repeatable regressions or results contradicting the compact-allocation
  premise must be explained and escalated for explicit owner decision before completion; workers
  must not invent or silently change thresholds.
- Correctness and deterministic SingleThread/Rayon behavior remain hard pass/fail gates that a
  performance interpretation or owner performance decision cannot waive.
- DHAT remains dev-only and isolated because its own documentation describes it as experimental and
  warns about global profiler state.

## Delivery sequence

1. After the sibling T004 task file contains exactly `status = "done"` and
   `verification_status = "passed"`, T001 completes the typed single-run policy, legacy wire, full
   collector, and public call-site migration.
2. T002 separates batch aggregation, lands permanent Criterion/DHAT tooling, and saves the
   pre-compact baseline while batch still executes full reports. Its bounded bridge starts from
   default full retention and replaces only schedule/metrics, preserving discarded transfer
   allocation for an honest before measurement.
3. T003 replaces full reports with compact samples, owns ordering/folding, runs feature equality,
   compares performance, and removes any interim full-report aggregation adapter.
4. T004 closes README/CHANGELOG/rustdoc, fixtures, assertion/artifact compatibility, and benchmark
   guidance.
5. T005 independently reviews and remediates API/Serde, performance, and determinism invariants.

Execution is serial because T002 must precede T003 and the first three tasks share central files.
The completed product contains no tracer/prototype names, flags, or temporary adapters.

## Traceability

| Requirement | Task(s) | Validation | Risk / fixed decision |
| --- | --- | --- | --- |
| R001 | T001 | Type tests, compile, rustdoc | Tagged enum/newtype contract is frozen. |
| R002 | T001 | Public no-retention integration test | Finals are never capture-controlled. |
| R003 | T001 | Channel selection/error tests | Empty `Only` is invalid. |
| R004 | T001, T004 | Stream equality and assertion event tests | Event callback is independent. |
| R005 | T001, T002 | Literal JSON compatibility matrix | Read legacy, write current only. |
| R006 | T002, T003 | Batch config/assertion tests | Batch selects only metric aggregation. |
| R007 | T003 | Compact sample tests and DHAT comparison | No clear-after-full-report workaround. |
| R008 | T003 | Default/parallel repeated equality | Explicit sort plus sequential fold. |
| R009 | T003, T004 | Assertion/event/artifact focused tests | Missing series is intentional evidence absence. |
| R010 | T002, T003 | Matched named Criterion and DHAT save/compare, absolute/relative deltas, metadata, rerun evidence, and owner decision when required | Baseline is captured before replacement; no invented numeric gate. |
| R011 | T004 | Doctests, snippet tests, doc review | No active legacy examples. |
| R012 | T005 | Three independent review lanes and full gate | Sol/high required; findings remediated. |

## Validation gate

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

No Docker/service access is required. Initial `dhat` dependency resolution may require approved
network access if it is not cached. Benchmark evidence records host/toolchain metadata, exact case
IDs, identical-workload checksums, and absolute/relative deltas. A noisy result is patiently rerun in
the same environment. The optional non-failing 7% Criterion summary is descriptive only, not a spec
pass/fail gate. Repeatable regressions or evidence contradicting the compact-allocation premise
require explanation and an explicit owner decision before completion; workers must not invent or
silently change thresholds. Correctness and determinism remain hard gates.

## Risks and stop conditions

- Stop before any edit unless 037/T004's task file contains exactly `status = "done"` and
  `verification_status = "passed"`; also stop if a worker would need to bypass private plan
  invariants.
- Stop if legacy behavior is ambiguous; do not reinterpret persisted payloads.
- Stop if state transitions, RNG draws, event order, final maps, or public report fields would change.
- Stop if aggregation cannot be normalized and folded in run-index order.
- Stop if the only available memory evidence is post-change or process RSS.
- Stop for explicit owner decision if repeatable measurements regress or contradict the
  compact-allocation premise after same-environment reruns and investigation; do not invent a
  threshold or silently recast the evidence.
- Do not publish collectors, compact samples, wire structs, or benchmark-only types.
- Do not perform the aggregate `0.2` package-version bump or publish operation.

Open decisions: none.
