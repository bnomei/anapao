use std::collections::BTreeMap;

use anapao::types::{
    BatchConfig, BatchRunTemplate, CaptureConfig, EdgeId, EdgeSpec, EndConditionSpec,
    ExecutionMode, NodeId, NodeKind, NodeSpec, ScenarioId, ScenarioSpec, TransferSpec,
    VariableRuntimeConfig, VariableSourceSpec, VariableUpdateTiming,
};
use anapao::Simulator;

#[cfg(feature = "parallel")]
use anapao::error::RunError;
#[cfg(feature = "parallel")]
use anapao::events::{EventSink, EventSinkError, RunEvent};

const VALUE_SCALE: f64 = 1_000_000.0;

fn scaled(value: f64) -> i64 {
    (value * VALUE_SCALE).round() as i64
}

fn partial_completion_scenario() -> ScenarioSpec {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("batch-partial-completion"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-sink-coin"),
            source,
            sink.clone(),
            TransferSpec::Expression { formula: "coin_flip".to_string() },
        ));

    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::EveryStep,
        sources: BTreeMap::from([(
            "coin_flip".to_string(),
            VariableSourceSpec::RandomList { values: vec![0.0, 1.0] },
        )]),
    };
    scenario.end_conditions =
        vec![EndConditionSpec::NodeAtLeast { node_id: sink, value_scaled: scaled(3.0) }];
    scenario
}

#[test]
fn batch_report_explicitly_retains_non_completed_run_summaries() {
    let compiled =
        Simulator::compile(partial_completion_scenario()).expect("partial scenario should compile");
    let config = BatchConfig {
        runs: 128,
        base_seed: 0x0D15_EA5E_u64,
        execution_mode: ExecutionMode::SingleThread,
        run_template: BatchRunTemplate { max_steps: 3, capture: CaptureConfig::default() },
    };

    let report = Simulator::run_batch(&compiled, &config).expect("batch run should succeed");

    let completed_count = report.runs.iter().filter(|run| run.completed).count() as u64;
    assert!(
        completed_count > 0 && completed_count < config.runs,
        "fixture should produce a deterministic mix of completed and non-completed runs"
    );
    assert_eq!(report.requested_runs, config.runs);
    assert_eq!(report.runs.len() as u64, config.runs);
    assert_eq!(
        report.completed_runs, config.runs,
        "completed_runs tracks reported run summaries; inspect per-run `completed` for partial completion"
    );
    assert!(report.runs.iter().any(|run| !run.completed));
    assert_eq!(report.runs.first().map(|run| run.run_index), Some(0));
    assert_eq!(report.runs.last().map(|run| run.run_index), Some(config.runs - 1));
}

#[cfg(feature = "parallel")]
#[test]
fn batch_rayon_sink_push_failure_maps_to_event_sink_error() {
    let compiled =
        Simulator::compile(anapao::testkit::fixture_scenario()).expect("fixture should compile");
    let config = BatchConfig {
        runs: 8,
        base_seed: 0xACED_u64,
        execution_mode: ExecutionMode::Rayon,
        run_template: BatchRunTemplate { max_steps: 10, capture: CaptureConfig::default() },
    };

    let mut sink = FailAfterPushesSink::new(0);
    let error = Simulator::run_batch_with_sink(&compiled, &config, &mut sink)
        .expect_err("sink should fail");

    assert!(matches!(
        error,
        RunError::EventSink { ref message }
            if message.contains("rayon_fail_sink") && message.contains("forced batch push failure")
    ));
    assert_eq!(sink.pushes, 1, "batch event emission must stop on first push error");
    assert_eq!(sink.flushes, 0, "flush must not run after a push failure");
}

#[cfg(feature = "parallel")]
struct FailAfterPushesSink {
    fail_after_pushes: usize,
    pushes: usize,
    flushes: usize,
}

#[cfg(feature = "parallel")]
impl FailAfterPushesSink {
    fn new(fail_after_pushes: usize) -> Self {
        Self { fail_after_pushes, pushes: 0, flushes: 0 }
    }
}

#[cfg(feature = "parallel")]
impl EventSink for FailAfterPushesSink {
    fn push(&mut self, _event: RunEvent) -> Result<(), EventSinkError> {
        self.pushes = self.pushes.saturating_add(1);
        if self.pushes > self.fail_after_pushes {
            Err(EventSinkError::custom("rayon_fail_sink", "forced batch push failure"))
        } else {
            Ok(())
        }
    }

    fn flush(&mut self) -> Result<(), EventSinkError> {
        self.flushes = self.flushes.saturating_add(1);
        Ok(())
    }
}
