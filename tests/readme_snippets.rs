use std::fs;

use anapao::types::{
    BatchConfig, BatchRunTemplate, CaptureConfig, EndConditionSpec, ExecutionMode, MetricKey,
    RunConfig, ScenarioSpec, TransferSpec,
};
use anapao::Simulator;

#[test]
fn readme_s01_build_minimal_scenario() {
    let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });
    scenario.tracked_metrics.insert(MetricKey::fixture("sink"));

    assert_eq!(scenario.nodes.len(), 2);
    assert_eq!(scenario.edges.len(), 1);
    assert!(scenario.tracked_metrics.contains(&MetricKey::fixture("sink")));
}

#[test]
fn readme_s02_compile_scenario() {
    let scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });

    let compiled = Simulator::compile(scenario).expect("compile source_sink scenario");
    assert_eq!(compiled.scenario.id.as_str(), "scenario-source-sink");
}

#[test]
fn readme_s03_create_deterministic_run_config() {
    let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(CaptureConfig {
        every_n_steps: 5,
        include_step_zero: true,
        include_final_state: true,
        ..CaptureConfig::default()
    });

    assert_eq!(run.seed, 42);
    assert_eq!(run.max_steps, 250);
    assert_eq!(run.capture.every_n_steps, 5);
}

#[test]
fn readme_s07_create_batch_config() {
    let batch = BatchConfig::for_runs(64)
        .with_execution_mode(ExecutionMode::SingleThread)
        .with_base_seed(7)
        .with_run_template(BatchRunTemplate::default())
        .with_max_steps(50);

    assert_eq!(batch.runs, 64);
    assert_eq!(batch.base_seed, 7);
    assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
    assert_eq!(batch.run_template.max_steps, 50);
}

#[test]
fn readme_linear_pipeline_convenience_constructor_compiles_and_runs() {
    let compiled =
        Simulator::compile(ScenarioSpec::linear_pipeline(4)).expect("compile linear pipeline");
    let run =
        Simulator::run(&compiled, &RunConfig::for_seed(42)).expect("run linear pipeline scenario");

    assert!(run.completed);
    assert!(run.final_metrics.contains_key(&MetricKey::fixture("sink")));
}

#[test]
fn readme_contains_curated_builder_snippet_signatures() {
    let path = format!("{}/README.md", env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(&path).expect("read README");

    for needle in [
        "### Snippet S01 — Build a Minimal Scenario",
        "let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })",
        "### Snippet S02 — Compile a Scenario",
        "assert_eq!(compiled.scenario.id.as_str(), \"scenario-source-sink\");",
        "### Snippet S03 — Create a Deterministic RunConfig",
        "let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(CaptureConfig {",
        "### Snippet S07 — Create BatchConfig",
        "let batch = BatchConfig::for_runs(64)",
    ] {
        assert!(readme.contains(needle), "README drift: missing snippet marker `{needle}`");
    }
}
