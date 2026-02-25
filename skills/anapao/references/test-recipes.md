# Anapao Test Recipes

Use these templates as starting points. Keep names and fixture IDs explicit.

## Deterministic Single-Run Replay

```rust
use anapao::{Simulator, testkit};

#[test]
fn deterministic_single_run_replays_exactly() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).expect("compile");
    let config = testkit::deterministic_run_config();

    let a = Simulator::run(&compiled, config.clone(), None).expect("run A");
    let b = Simulator::run(&compiled, config, None).expect("run B");

    assert_eq!(a, b);
}
```

## Run With Assertions and `VecEventSink`

```rust
use anapao::assertions::{Expectation, MetricSelector};
use anapao::events::VecEventSink;
use anapao::types::MetricKey;
use anapao::{Simulator, testkit};

#[test]
fn run_with_assertions_emits_checkpoint_events() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).expect("compile");
    let expectations = vec![Expectation::Equals {
        metric: MetricKey::fixture("sink"),
        selector: MetricSelector::Final,
        expected: 3.0,
    }];

    let mut sink = VecEventSink::new();
    let (_run, report) = Simulator::run_with_assertions(
        &compiled,
        testkit::deterministic_run_config(),
        &expectations,
        Some(&mut sink),
    )
    .expect("run with assertions");

    assert!(report.is_success());
    assert!(sink
        .events()
        .iter()
        .any(|event| event.event_name() == "assertion_checkpoint"));
}
```

## Batch Replay With Seed-Derivation Checks

```rust
use anapao::rng::derive_run_seed;
use anapao::{Simulator, testkit};

#[test]
fn batch_replay_and_seed_schedule_are_stable() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).expect("compile");
    let config = testkit::deterministic_batch_config();

    let a = Simulator::run_batch(&compiled, config.clone(), None).expect("batch A");
    let b = Simulator::run_batch(&compiled, config.clone(), None).expect("batch B");
    assert_eq!(a, b);

    for run in &a.runs {
        assert_eq!(run.seed, derive_run_seed(config.base_seed, run.run_index));
    }
}
```

## Compile-Time Invalid Reference Validation

```rust
use anapao::error::SetupError;
use anapao::types::{MetricKey, ScenarioSpec, ScenarioId};
use anapao::Simulator;

#[test]
fn compile_rejects_unresolved_tracked_metric() {
    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-invalid-ref"));
    scenario.tracked_metrics.insert(MetricKey::fixture("missing_metric"));

    let error = Simulator::compile(scenario).expect_err("compile must fail");
    match error {
        SetupError::InvalidGraphReference { graph, reference } => {
            assert!(graph.contains(".metrics"));
            assert!(reference.contains("unresolved metric"));
        }
        other => panic!("expected InvalidGraphReference, got {other:?}"),
    }
}
```

## Artifact Schema v2 + Compat Reader

```rust
use anapao::artifact::{read_manifest_compat_from_slice, write_batch_artifacts};
use anapao::types::{BatchReport, ExecutionMode, ScenarioId, ARTIFACT_SCHEMA_VERSION_V2};
use tempfile::tempdir;

#[test]
fn artifact_schema_and_compat_reader_stay_stable() {
    let dir = tempdir().expect("tempdir");
    let report =
        BatchReport::new(ScenarioId::fixture("schema-check"), 1, ExecutionMode::SingleThread);

    let manifest = write_batch_artifacts(dir.path(), &report).expect("write artifacts");
    assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);

    let raw_v1 = br#"{
      "scenario_id":"legacy",
      "artifacts":{"summary":{"kind":"summary","path":"summary.csv","content_type":"text/csv"}}
    }"#;
    let upgraded = read_manifest_compat_from_slice(raw_v1).expect("compat read");
    assert_eq!(upgraded.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
}
```
