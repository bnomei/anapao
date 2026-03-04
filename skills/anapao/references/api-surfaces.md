# Anapao API Surfaces

## Concept Map

Use this flow to pick APIs:

1. Setup model: `types::*` + `ScenarioSpec` builders
2. Validate/compile: `Simulator::compile` -> `CompiledScenario`
3. Execute: `Simulator::run*` or `Simulator::run_batch*`
4. Assert: `assertions::*` or integrated `run*_with_assertions`
5. Observe timeline: `events::*` + `EventSink`
6. Persist evidence: `artifact::*`
7. Analyze distributions: `stats::*` (and `analysis::*` when enabled)

## Surface Guide

| Surface | Use this when | Key entrypoints |
| --- | --- | --- |
| `types` | Defining scenarios/configs and inspecting reports | `ScenarioSpec` (+ `source_sink`/`linear_pipeline`), `RunConfig`, `BatchConfig`, `ConfidenceLevel`, `RunReport`, `BatchReport`, `ManifestRef` |
| `Simulator` | Running public end-to-end workflows | `compile`, `run`, `run_with_assertions`, `run_batch`, `run_batch_with_assertions` |
| `assertions` | Expressing metric expectations declaratively | `Expectation`, `MetricSelector`, `evaluate_run_expectations`, `evaluate_batch_expectations` |
| `events` | Capturing ordered run diagnostics | `RunEvent`, `RunEventPhase`, `VecEventSink`, `EventSink` |
| `artifact` | Writing/reading CI-friendly outputs | `write_run_artifacts`, `write_run_artifacts_with_assertions`, `write_batch_artifacts`, `write_batch_artifacts_with_confidence_level`, `read_manifest_compat` |
| `stats` | Computing prediction and confidence indicators | `prediction_indicators`, `prediction_indicators_with_confidence`, `prediction_indicators_by_metric`, `prediction_indicators_by_metric_with_confidence`, `summarize_by_metric` |
| `testkit` | Reusing deterministic fixtures and parity helpers | `fixture_scenario`, `deterministic_run_config`, `deterministic_batch_config`, parity loaders |
| `analysis` | Producing dataframe views (feature-gated) | `run_series_frame`, `batch_series_frame`, `batch_final_metrics_frame` |

## Minimal Usage Snippets

### Setup + compile + run

```rust
use anapao::{Simulator, testkit};

let compiled = Simulator::compile(testkit::fixture_scenario())?;
let report = Simulator::run(&compiled, testkit::deterministic_run_config(), None)?;
assert!(report.completed);
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Integrated assertions

```rust
use anapao::assertions::{Expectation, MetricSelector};
use anapao::types::MetricKey;
use anapao::{Simulator, testkit};

let compiled = Simulator::compile(testkit::fixture_scenario())?;
let expectations = vec![Expectation::Equals {
    metric: MetricKey::fixture("sink"),
    selector: MetricSelector::Final,
    expected: 3.0,
}];

let (_run, assertions) = Simulator::run_with_assertions(
    &compiled,
    testkit::deterministic_run_config(),
    &expectations,
    None,
)?;
assert!(assertions.is_success());
# Ok::<(), Box<dyn std::error::Error>>(())
```

### Batch determinism

```rust
use anapao::{Simulator, testkit};

let compiled = Simulator::compile(testkit::fixture_scenario())?;
let config = testkit::deterministic_batch_config();
let a = Simulator::run_batch(&compiled, config.clone(), None)?;
let b = Simulator::run_batch(&compiled, config, None)?;
assert_eq!(a, b);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Confidence Semantics Notes

- Treat `confidence_lower_95`, `confidence_upper_95`, and `confidence_margin_95` as strict 95% fields.
- For selected confidence levels (`P90`/`P95`/`P99`), use report metadata and selected fields:
  - `selected_confidence_level`
  - `confidence_lower_selected`
  - `confidence_upper_selected`
  - `confidence_margin_selected`

### Event stream capture

```rust
use anapao::events::VecEventSink;
use anapao::{Simulator, testkit};

let compiled = Simulator::compile(testkit::fixture_scenario())?;
let mut sink = VecEventSink::new();
let _ = Simulator::run(&compiled, testkit::deterministic_run_config(), Some(&mut sink))?;
assert!(sink.events().iter().all(|e| e.order().run_id == "run-0"));
# Ok::<(), Box<dyn std::error::Error>>(())
```
