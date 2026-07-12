# anapao

[![Crates.io Version](https://img.shields.io/crates/v/anapao)](https://crates.io/crates/anapao)
[![CI](https://img.shields.io/github/actions/workflow/status/bnomei/anapao/ci.yml?branch=main)](https://github.com/bnomei/anapao/actions/workflows/ci.yml)
[![CodSpeed Badge](https://img.shields.io/endpoint?url=https://codspeed.io/badge.json)](https://codspeed.io/bnomei/anapao?utm_source=badge)
[![Crates.io Downloads](https://img.shields.io/crates/d/anapao)](https://crates.io/crates/anapao)
[![License](https://img.shields.io/crates/l/anapao)](https://crates.io/crates/anapao)
[![Discord](https://flat.badgen.net/badge/discord/bnomei?color=7289da&icon=discord&label)](https://discordapp.com/users/bnomei)
[![Buymecoffee](https://flat.badgen.net/badge/icon/donate?icon=buymeacoffee&color=FF813F&label)](https://www.buymeacoffee.com/bnomei)

`anapao` is a library-only deterministic Rust testing utility for simulation and stochastic workflows. It is intended to be used from Rust tests and tooling through the crate API, not as a command-line program.
This README is a linear tutorial for new users: you will build one scenario, run it deterministically, add expectations, run Monte Carlo batches, and persist CI-friendly artifacts.

The README and generated crate documentation are self-contained public documentation. Any ignored local `docs/` directory is reserved for private research notes and is not tracked, packaged, shipped, or required to use the crate.

## What You Will Build

By the end, you will have a repeatable testing flow that can:
- load a stable `ScenarioSpec` document or author an immutable checked `Scenario`,
- compile either representation into the same opaque executable model,
- execute seeded deterministic single runs,
- execute deterministic Monte Carlo batches,
- evaluate typed assertions with evidence,
- persist artifact packs (`manifest.json`, `events.jsonl`, `series.csv`, and more).

## Prerequisites

- Rust `1.85+`
- Cargo
- A Rust test project where you want deterministic simulation checks

Add the library dependency:

```toml
[dependencies]
anapao = "0.1.0"
```

The crate does not install or expose a binary target; import `anapao` from your Rust code.

---

## Scenario Representations and the Validation Boundary

Anapao has four deliberately distinct stages:

1. `ScenarioSpec` is the stable serde wire DTO used to load, inspect, edit, and store documents.
2. `Scenario::try_from` checks a DTO and produces an immutable semantic domain value.
3. `ScenarioBuilder` and the `ScenarioNode`/`ScenarioEdge` family constructors author that checked
   domain directly from Rust.
4. `Simulator::compile` (legacy DTO input) or `Simulator::compile_checked` (checked input) produces
   an opaque `CompiledScenario`, which `Simulator::run` executes.

Checked types are not a second serde representation. Deserialize the stable DTO first:

```rust
use anapao::types::{Scenario, ScenarioSpec};

let document = serde_json::to_string(&anapao::testkit::fixture_scenario()).unwrap();
let dto: ScenarioSpec = serde_json::from_str(&document).unwrap();
let checked = Scenario::try_from(dto).unwrap();

assert_eq!(checked.id().as_str(), "scenario-testkit");
```

For programmatic authoring, use the complete checked builder. Its consuming insertion methods
return `Result` because duplicate IDs are rejected:

```rust
use std::num::NonZeroU64;
use anapao::types::{
    EdgeId, EndConditionSpec, MetricKey, NodeId, ResourceConnection, RunConfig,
    ScenarioBuilder, ScenarioEdge, ScenarioId, ScenarioNode, StateConnection,
    StateConnectionRole, StateTarget, TransferSpec,
};
use anapao::Simulator;

let source = NodeId::fixture("source");
let pool = NodeId::fixture("pool");
let sink = NodeId::fixture("sink");
let scenario = ScenarioBuilder::new(ScenarioId::fixture("checked-authoring"))
    .with_title("Checked authoring")
    .with_description("resource and state flow")
    .with_tag("docs")
    .with_node(ScenarioNode::source(source.clone()).with_initial_value(2.0))?
    .with_node(ScenarioNode::pool(pool.clone(), Default::default()).with_label("buffer"))?
    .with_node(ScenarioNode::sink(sink.clone()))?
    .with_edge(ScenarioEdge::resource(
        EdgeId::fixture("source-pool"),
        source.clone(),
        pool.clone(),
        TransferSpec::Fixed { amount: 1.0 },
        ResourceConnection::default().with_token_size(NonZeroU64::new(1).unwrap()),
    ))?
    .with_edge(ScenarioEdge::resource(
        EdgeId::fixture("pool-sink"),
        pool.clone(),
        sink,
        TransferSpec::Remaining,
        ResourceConnection::default(),
    ))?
    .with_edge(ScenarioEdge::state(
        EdgeId::fixture("source-pool-state"),
        source,
        pool,
        TransferSpec::Remaining,
        StateConnection::new(StateConnectionRole::Modifier, "+1", StateTarget::Node),
    ))?
    .with_end_condition(EndConditionSpec::MaxSteps { steps: 2 })
    .with_tracked_metric(MetricKey::fixture("sink"))
    .with_metadata("owner", "docs")
    .build()?;

let compiled = Simulator::compile_checked(scenario)?;
assert_eq!(compiled.source_spec().title.as_deref(), Some("Checked authoring"));
let report = Simulator::run(&compiled, &RunConfig::for_seed(39)).unwrap();
assert!(report.completed);
# Ok::<(), anapao::error::SetupError>(())
```

The common `anapao::prelude` exports the checked scenario entrypoints. Individual family config
types remain available from `anapao::types` when their defaults need customization.

---

## Step 1: Create `ScenarioSpec`

`ScenarioSpec` is your declarative model: nodes, edges, end conditions, and tracked metrics.

### Snippet S01 — Build a Minimal Scenario

```rust
use anapao::types::{EndConditionSpec, MetricKey, ScenarioSpec, TransferSpec};

let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
    .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });
scenario.tracked_metrics.insert(MetricKey::fixture("sink"));

assert_eq!(scenario.nodes.len(), 2);
assert_eq!(scenario.edges.len(), 1);
```

What you learned:
- how to bootstrap a minimum source->sink scenario with a convenience constructor,
- how end conditions and tracked metrics are attached.

---

## Step 2: Compile with `Simulator::compile`

Compilation validates and transforms your scenario into deterministic execution indexes.

### Snippet S02 — Compile a Scenario

```rust
use anapao::types::{EndConditionSpec, ScenarioSpec, TransferSpec};
use anapao::Simulator;

let scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
    .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });

let compiled = Simulator::compile(scenario).unwrap();
assert_eq!(compiled.scenario_id().as_str(), "scenario-source-sink");
```

What you learned:
- compilation is explicit and deterministic,
- you should compile once and reuse the compiled form for runs.

### 0.2 API Migration

`CompiledScenario` is now an opaque, immutable execution product. Use the root-level
`Simulator` facade instead of the old raw compiler/engine/batch paths:

```rust
let scenario = anapao::testkit::fixture_scenario();
let run_config = anapao::testkit::deterministic_run_config();

// Before: anapao::validation::compile_scenario(&scenario)
// After:
let compiled = anapao::Simulator::compile(scenario).unwrap();

// Before: anapao::engine::run_single(&compiled, &run_config)
// After:
let report = anapao::Simulator::run(&compiled, &run_config).unwrap();

// Before: compiled.scenario.id / compiled.node_order / compiled.edge_order
// After:  compiled.scenario_id() / compiled.node_ids() / compiled.edge_ids()
```

For checked conversion in generic code, use `let compiled: anapao::CompiledScenario =
scenario.try_into()?;`. Read inspection data through `scenario_id()`, `source_spec()`,
`node_ids()`, `edge_ids()`, `node_count()`, and `edge_count()`; raw execution modules are private.

The legacy DTO route remains supported and its `with_node`/`with_edge` helpers keep
last-write-wins replacement semantics. The checked `ScenarioBuilder` instead returns a stable
error for duplicate node or edge IDs and retains the first definition.

Version 0.2 intentionally rejects semantic combinations that older execution paths could repair
or reinterpret:

- a node or edge map key that differs from the embedded `id`;
- an explicit node-family tag paired with another family's config payload;
- a resource/state connection tag paired with an active payload for the other connection kind;
- a node state target carrying a target connection ID; and
- a resource-connection, state-connection, or formula target missing its required target ID.

These checks happen after serde parsing. Raw JSON lexical duplicate keys are not detected at this
boundary, and no stored-data backfill or second checked serde format is introduced.

---

## Step 3: Configure `RunConfig`

`RunConfig` controls deterministic single-run execution (`seed`, `max_steps`, capture policy).

### Snippet S03 — Create a Deterministic RunConfig

```rust
use anapao::types::{CaptureConfig, CaptureSchedule, RunConfig};

let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(
    CaptureConfig::default().with_schedule(CaptureSchedule::Every {
        stride: std::num::NonZeroU64::new(5).expect("positive stride"),
        include_initial: true,
        include_final: true,
    }),
);

assert_eq!(run.seed, 42);
assert_eq!(run.max_steps, 250);
assert!(matches!(
    run.capture.schedule(),
    CaptureSchedule::Every { stride, .. } if stride.get() == 5
));
```

What you learned:
- seeds pin determinism,
- capture configuration controls diagnostic trace granularity.

### Retention, Events, and Aggregation Are Separate

`CaptureConfig` controls **diagnostic report retention**, not whether the simulation completes.
`CaptureConfig::none()` leaves `RunReport::final_node_values` and `RunReport::final_metrics`
available, while intentionally leaving node snapshots, variable snapshots, transfer records, and
metric series empty. Use `CaptureConfig::final_only()` when final step-aligned diagnostics are
useful without retaining transfers, or `CaptureSchedule::Every` with typed `Selection` values for
periodic/selective diagnostics.

Live streamed events are independent of report retention: `Simulator::run_with_sink` and the
assertion-streaming APIs emit the same ordered simulation events when capture is `none()` as they
do with default capture. Batch aggregate sampling is separate again: `AggregationConfig` controls
only the metric series in `BatchReport`, while every `BatchRunSummary` retains terminal metrics.

Consequently, final-value assertions work with no captured series. Step selectors,
monotonic-series assertions, and series probability assertions require captured or aggregated
series evidence; when it was not requested, they report missing evidence instead of inferring it.

Batch aggregation is separate from per-run diagnostic capture. Configure only the
metric schedule and selection that belong in the `BatchReport`:

```rust
use anapao::types::{AggregationConfig, BatchConfig, CaptureSchedule, ExecutionMode};

let batch = BatchConfig::for_runs(64)
    .with_execution_mode(ExecutionMode::SingleThread)
    .with_aggregation(AggregationConfig::default().with_schedule(CaptureSchedule::Final));

assert!(matches!(batch.run_template.aggregation.schedule(), CaptureSchedule::Final));
```

---

## Step 4: Execute a Deterministic Single Run

Now run one deterministic simulation and assert expected outputs.

### Snippet S04 — Run Once and Verify Outputs

```rust
use anapao::{testkit, Simulator};
use anapao::types::MetricKey;

let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
let report = Simulator::run(&compiled, &testkit::deterministic_run_config()).unwrap();

assert!(report.completed);
assert_eq!(report.steps_executed, 3);
assert_eq!(report.final_metrics.get(&MetricKey::fixture("sink")), Some(&3.0));
```

What you learned:
- deterministic single-run output can be asserted directly in tests.

---

## Step 5: Create an `Expectation` Set

`Expectation` provides typed assertion semantics for run and batch reports.

### Snippet S05 — Declare Expectations

```rust
use anapao::assertions::{Expectation, MetricSelector};
use anapao::types::MetricKey;

let metric = MetricKey::fixture("sink");
let expectations = vec![
    Expectation::Equals {
        metric: metric.clone(),
        selector: MetricSelector::Final,
        expected: 3.0,
    },
    Expectation::Approx {
        metric: metric.clone(),
        selector: MetricSelector::Final,
        expected: 3.0,
        abs_tol: 0.0001,
        rel_tol: 0.0,
    },
    Expectation::Between {
        metric,
        selector: MetricSelector::Final,
        min: 0.0,
        max: 10.0,
    },
];

assert_eq!(expectations.len(), 3);
```

What you learned:
- expectations are data, not ad-hoc assertion code,
- final selectors read always-retained terminal metrics, while specific-step selectors require
  captured series evidence.

---

## Step 6: Run with Assertions and Event Sink

Use the integrated assertion path and capture ordered events for diagnostics.

### Snippet S06 — `run_with_assertions_and_sink` + `VecEventSink`

```rust
use anapao::assertions::{Expectation, MetricSelector};
use anapao::events::VecEventSink;
use anapao::types::MetricKey;
use anapao::{testkit, Simulator};

let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
let expectations = vec![Expectation::Equals {
    metric: MetricKey::fixture("sink"),
    selector: MetricSelector::Final,
    expected: 3.0,
}];

let mut sink = VecEventSink::new();
let (_report, assertion_report) = Simulator::run_with_assertions_and_sink(
    &compiled,
    &testkit::deterministic_run_config(),
    &expectations,
    &mut sink,
)
.unwrap();

assert!(assertion_report.is_success());
assert!(sink
    .events()
    .iter()
    .any(|event| event.event_name() == "assertion_checkpoint"));
```

What you learned:
- assertions and execution can be done in one call,
- event streams provide structured debugging context.

---

## Step 7: Configure `BatchConfig`

`BatchConfig` controls deterministic Monte Carlo execution.

### Snippet S07 — Create BatchConfig

```rust
use anapao::types::{BatchConfig, BatchRunTemplate, ExecutionMode};

let batch = BatchConfig::for_runs(64)
    .with_execution_mode(ExecutionMode::SingleThread)
    .with_base_seed(7)
    .with_run_template(BatchRunTemplate::default())
    .with_max_steps(50);

assert_eq!(batch.runs, 64);
assert_eq!(batch.base_seed, 7);
assert_eq!(batch.run_template.max_steps, 50);
```

What you learned:
- `runs` scales the Monte Carlo sample size,
- `base_seed` + run index derivation preserve reproducibility.

---

## Step 8: Execute a Deterministic Batch Run

Run many deterministic simulations and check aggregate outputs.

### Snippet S08 — Run Batch and Verify Ordering/Aggregates

```rust
use anapao::{testkit, Simulator};
use anapao::types::MetricKey;

let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
let batch = Simulator::run_batch(&compiled, &testkit::deterministic_batch_config()).unwrap();

assert_eq!(batch.completed_runs, batch.requested_runs);
assert!(batch.runs.windows(2).all(|window| window[0].run_index < window[1].run_index));
assert!(batch.aggregate_series.contains_key(&MetricKey::fixture("sink")));
```

What you learned:
- batch summaries are deterministic and index-ordered.
- `completed_runs` counts reported run summaries; inspect each `run.completed` for semantic completion.

---

## Step 9: Persist Artifacts and Inspect `ManifestRef`

Persist reports for CI diffing and post-run diagnostics.

### Snippet S09 — Full Playbook (Setup -> Run -> Assert -> Artifacts)

```rust,no_run
use anapao::artifact::write_run_artifacts_with_assertions;
use anapao::assertions::{Expectation, MetricSelector};
use anapao::events::VecEventSink;
use anapao::types::MetricKey;
use anapao::{testkit, Simulator};

let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
let expectations = vec![Expectation::Equals {
    metric: MetricKey::fixture("sink"),
    selector: MetricSelector::Final,
    expected: 3.0,
}];

let mut sink = VecEventSink::new();
let (run_report, assertion_report) = Simulator::run_with_assertions_and_sink(
    &compiled,
    &testkit::deterministic_run_config(),
    &expectations,
    &mut sink,
)
.unwrap();
assert!(run_report.completed);
assert!(assertion_report.is_success());

let output_dir = std::env::temp_dir().join("anapao-readme-playbook");
let manifest = write_run_artifacts_with_assertions(
    &output_dir,
    &run_report,
    sink.events(),
    Some(&assertion_report),
)
.unwrap();

assert!(manifest.artifacts.contains_key("manifest"));
assert!(manifest.artifacts.contains_key("events"));
assert!(manifest.artifacts.contains_key("assertions"));
```

What you learned:
- persisted artifacts become your CI and debugging contract,
- manifest keys are stable assertions for artifact expectations.

Artifact file ownership does not change when diagnostics are disabled. Where a run or batch writer
is invoked, its manifest-owned `variables.csv` and `series.csv` files remain valid header-only CSVs
when no variable snapshots or series were retained. Supplied events still produce `events.jsonl`
and drive the history/replay indexes.

---

## Step 10: Fixture-First Testing with `testkit` (and `rstest`)

Use `testkit` helpers to avoid duplicating setup across tests.

### Snippet S10 — Reusable Fixture-Style Test Pattern

```rust
use anapao::{testkit, Simulator};
use anapao::types::MetricKey;

fn deterministic_fixture_smoke() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
    let report = Simulator::run(&compiled, &testkit::deterministic_run_config()).unwrap();
    assert_eq!(report.final_metrics.get(&MetricKey::fixture("sink")), Some(&3.0));
}

deterministic_fixture_smoke();
```

What you learned:
- fixture helpers keep tests concise and deterministic,
- you can wrap these helpers in your own `rstest` fixture macros for larger matrices.

---

## Common Failure Modes and Debugging Hints

- Missing tracked metric:
  - symptom: expectation fails with missing observed value.
  - fix: ensure metric key is in `scenario.tracked_metrics`.
- Non-terminating scenarios:
  - symptom: run ends at `max_steps` unexpectedly.
  - fix: verify `end_conditions` are configured and reachable.
- Seed confusion:
  - symptom: output differs between runs.
  - fix: pin `RunConfig.seed` for single runs and keep batch `base_seed` stable (batch seeds derive from `base_seed` + run index).
- Sparse traces:
  - symptom: insufficient snapshots for diagnostics.
  - fix: use `CaptureConfig::final_only()` or adjust `RunConfig.capture` with
    `CaptureSchedule::Every`.

## Feature Flags

- `parallel`: enables Rayon-backed batch execution mode (`ExecutionMode::Rayon`).
- `analysis-polars`: enables Polars DataFrame shaping helpers.
- `assertions-extended`: enables extra assertion/snapshot/property helper crates.

CI intentionally validates a targeted feature surface instead of an exhaustive feature
combination matrix. The supported check surface is the default feature set, each
individual optional feature (`parallel`, `analysis-polars`, and
`assertions-extended`), and the combined `--all-features` build.

## Module Surface (Reference)

`anapao` exports:
- `types`
- `error`
- `rng`
- `stochastic`
- `events`
- `stats`
- `artifact`
- `assertions`
- `testkit`
- `analysis` (only with `analysis-polars`)
- `Simulator` (compile/run/batch facade)

## Validation Commands

```bash
cargo test --doc
cargo test --all-targets
cargo test --all-targets --features parallel
cargo test --all-targets --features analysis-polars
cargo test --all-targets --features assertions-extended
cargo test --all-targets --all-features
cargo audit --deny warnings
cargo bench --no-run
```

## Performance Workflow (Manual Compare)

```bash
# capture matching default and parallel Criterion baselines (these runs can take time)
./scripts/bench-criterion save --bench simulation --baseline capture-retention-default
./scripts/bench-criterion save --bench simulation --features parallel --baseline capture-retention-parallel

# compare matrix
./scripts/bench-criterion compare --bench simulation --baseline capture-retention-default
./scripts/bench-criterion compare --bench simulation --features parallel --baseline capture-retention-parallel

# manual non-failing regression summary (+7% threshold)
./scripts/bench-criterion summary --bench simulation --baseline capture-retention-default --threshold 0.07
./scripts/bench-criterion summary --bench simulation --features parallel --baseline capture-retention-parallel --threshold 0.07

# run isolated DHAT capture-retention evidence in separate processes
./scripts/bench-capture-memory save --baseline capture-retention-default
./scripts/bench-capture-memory compare --baseline capture-retention-default

# flamegraphs and csv summaries
./benchmarks/run_profiles.sh
BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh
```

## Dependency and Security Maintenance

CI runs `cargo audit --deny warnings` on every push and pull request to report RustSec advisories and dependency problems from `Cargo.lock`. Treat a failing audit as a release blocker unless the advisory is not reachable for this crate; if an advisory is not actionable immediately, document the reason and the planned follow-up in the pull request.

When updating dependencies:

1. Prefer the smallest compatible version bump that resolves the advisory or maintenance need.
2. Review changelogs for public API, MSRV, feature, and license changes before merging.
3. Keep optional feature dependencies (`parallel`, `analysis-polars`, and `assertions-extended`) checked with the normal CI matrix instead of adding one-off release automation.
4. Regenerate and commit `Cargo.lock`, then run `cargo audit --deny warnings` plus the standard repository validation commands.

## 0.2 Capture Policy Migration

Rust configuration fields are intentionally no longer a stable struct-literal surface. Construct
policies with `CaptureConfig::{none, final_only, default}` and consuming builders such as
`with_schedule`, `with_metrics`, and `with_variables`; configure batch aggregate sampling with
`AggregationConfig` and `BatchConfig::with_aggregation`. `CaptureConfig::disabled()` and batch
`with_capture` adapters are deprecated compatibility spellings, not recommended examples.

Persisted JSON remains migration-friendly: anapao reads the historical five-field capture object
(and historical nested `BatchRunTemplate.capture`) with its old behavior, rejects a zero legacy
stride, and writes only the canonical tagged typed representation. New JSON should use the current
`schedule`, channel selections, and batch `aggregation` fields.

## Local Pre-commit

This repo ships a native `prek.toml` for fast local commit gates.

```bash
prek validate-config
prek run --all-files
prek install
```

The hooks intentionally stay lightweight: `cargo fmt --all -- --check` and `cargo clippy --all-targets --all-features -- -D warnings`.
