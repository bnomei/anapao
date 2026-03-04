# anapao

`anapao` is a deterministic Rust testing utility for simulation and stochastic workflows.  
This README is a linear tutorial for new users: you will build one scenario, run it deterministically, add expectations, run Monte Carlo batches, and persist CI-friendly artifacts.

## What You Will Build

By the end, you will have a repeatable testing flow that can:
- compile a `ScenarioSpec` into a validated executable model,
- execute seeded deterministic single runs,
- execute deterministic Monte Carlo batches,
- evaluate typed assertions with evidence,
- persist artifact packs (`manifest.json`, `events.jsonl`, `series.csv`, and more).

## Prerequisites

- Rust `1.70+`
- Cargo
- A Rust test project where you want deterministic simulation checks

Add the dependency:

```toml
[dependencies]
anapao = "0.1.1"
```

---

## Step 1: Create `ScenarioSpec`

`ScenarioSpec` is your declarative model: nodes, edges, end conditions, and tracked metrics.

### Snippet S01 — Build a Minimal Scenario

```rust
use anapao::types::{
    EdgeId, EdgeSpec, EndConditionSpec, MetricKey, NodeId, NodeKind, NodeSpec, ScenarioId,
    ScenarioSpec, TransferSpec,
};

let source = NodeId::fixture("source");
let sink = NodeId::fixture("sink");
let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-readme"))
    .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
    .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
    .with_edge(EdgeSpec::new(
        EdgeId::fixture("edge-source-sink"),
        source,
        sink,
        TransferSpec::Fixed { amount: 1.0 },
    ));
scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
scenario.tracked_metrics.insert(MetricKey::fixture("sink"));

assert_eq!(scenario.nodes.len(), 2);
assert_eq!(scenario.edges.len(), 1);
```

What you learned:
- how to model a minimum source->sink scenario,
- how end conditions and tracked metrics are attached.

---

## Step 2: Compile with `Simulator::compile`

Compilation validates and transforms your scenario into deterministic execution indexes.

### Snippet S02 — Compile a Scenario

```rust
use anapao::types::{
    EdgeId, EdgeSpec, EndConditionSpec, MetricKey, NodeId, NodeKind, NodeSpec, ScenarioId,
    ScenarioSpec, TransferSpec,
};
use anapao::Simulator;

let source = NodeId::fixture("source");
let sink = NodeId::fixture("sink");
let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-readme"))
    .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
    .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
    .with_edge(EdgeSpec::new(
        EdgeId::fixture("edge-source-sink"),
        source,
        sink,
        TransferSpec::Fixed { amount: 1.0 },
    ));
scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
scenario.tracked_metrics.insert(MetricKey::fixture("sink"));

let compiled = Simulator::compile(scenario).unwrap();
assert_eq!(compiled.scenario.id.as_str(), "scenario-readme");
```

What you learned:
- compilation is explicit and deterministic,
- you should compile once and reuse the compiled form for runs.

---

## Step 3: Configure `RunConfig`

`RunConfig` controls deterministic single-run execution (`seed`, `max_steps`, capture policy).

### Snippet S03 — Create a Deterministic RunConfig

```rust
use anapao::types::{CaptureConfig, RunConfig};

let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(CaptureConfig {
    every_n_steps: 5,
    include_step_zero: true,
    include_final_state: true,
    ..CaptureConfig::default()
});

assert_eq!(run.seed, 42);
assert_eq!(run.max_steps, 250);
assert_eq!(run.capture.every_n_steps, 5);
```

What you learned:
- seeds pin determinism,
- capture configuration controls trace granularity.

---

## Step 4: Execute a Deterministic Single Run

Now run one deterministic simulation and assert expected outputs.

### Snippet S04 — Run Once and Verify Outputs

```rust
use anapao::{testkit, Simulator};
use anapao::types::MetricKey;

let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
let report = Simulator::run(&compiled, testkit::deterministic_run_config(), None).unwrap();

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
- selector controls whether you validate final value vs specific step.

---

## Step 6: Run with Assertions and Event Sink

Use the integrated assertion path and capture ordered events for diagnostics.

### Snippet S06 — `run_with_assertions` + `VecEventSink`

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
let (_report, assertion_report) = Simulator::run_with_assertions(
    &compiled,
    testkit::deterministic_run_config(),
    &expectations,
    Some(&mut sink),
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
use anapao::types::{BatchConfig, ExecutionMode, RunConfig};

let mut batch = BatchConfig::for_runs(64)
    .with_execution_mode(ExecutionMode::SingleThread)
    .with_run(RunConfig::for_seed(999))
    .with_max_steps(50);
batch.base_seed = 7;

assert_eq!(batch.runs, 64);
assert_eq!(batch.base_seed, 7);
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
let batch = Simulator::run_batch(&compiled, testkit::deterministic_batch_config(), None).unwrap();

assert_eq!(batch.completed_runs, batch.requested_runs);
assert!(batch.runs.windows(2).all(|window| window[0].run_index < window[1].run_index));
assert!(batch.aggregate_series.contains_key(&MetricKey::fixture("sink")));
```

What you learned:
- batch summaries are deterministic and index-ordered.

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
let (run_report, assertion_report) = Simulator::run_with_assertions(
    &compiled,
    testkit::deterministic_run_config(),
    &expectations,
    Some(&mut sink),
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

---

## Step 10: Fixture-First Testing with `testkit` (and `rstest`)

Use `testkit` helpers to avoid duplicating setup across tests.

### Snippet S10 — Reusable Fixture-Style Test Pattern

```rust
use anapao::{testkit, Simulator};
use anapao::types::MetricKey;

fn deterministic_fixture_smoke() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
    let report = Simulator::run(&compiled, testkit::deterministic_run_config(), None).unwrap();
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
  - fix: pin `RunConfig.seed` and keep batch `base_seed` stable.
- Sparse traces:
  - symptom: insufficient snapshots for diagnostics.
  - fix: adjust `RunConfig.capture` (`every_n_steps`, step-zero/final flags).

## Feature Flags

- `parallel`: enables Rayon-backed batch execution mode (`ExecutionMode::Rayon`).
- `analysis-polars`: enables Polars DataFrame shaping helpers.
- `assertions-extended`: enables extra assertion/snapshot/property helper crates.

## Module Surface (Reference)

`anapao` exports:
- `types`
- `error`
- `rng`
- `validation`
- `engine`
- `stochastic`
- `events`
- `batch`
- `stats`
- `artifact`
- `assertions`
- `testkit`
- `analysis` (only with `analysis-polars`)
- `Simulator` (compile/run/batch facade)

## Validation Commands

```bash
cargo test --doc
cargo test
cargo test --features parallel
cargo test --features analysis-polars
cargo bench --no-run
```

## Performance Workflow (Manual Compare)

```bash
# capture baseline matrix
./scripts/bench-criterion save --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion save --bench simulation --features parallel --baseline hotspots-20260224-parallel

# compare matrix
./scripts/bench-criterion compare --bench simulation --baseline hotspots-20260224-default
./scripts/bench-criterion compare --bench simulation --features parallel --baseline hotspots-20260224-parallel

# manual non-failing regression summary (+7% threshold)
./scripts/bench-criterion summary --bench simulation --baseline hotspots-20260224-default --threshold 0.07
./scripts/bench-criterion summary --bench simulation --features parallel --baseline hotspots-20260224-parallel --threshold 0.07

# flamegraphs and csv summaries
./benchmarks/run_profiles.sh
BENCH_FEATURES=parallel ./benchmarks/run_profiles.sh
```
