use crate::assertions::{self, AssertionReport, Expectation};
use crate::batch;
use crate::engine;
use crate::error::{RunError, SetupError, SimError};
use crate::events::{
    AssertionCheckpointEvent, EventSink, EventSinkError, MetricSnapshotEvent, RunEvent,
    StepEndEvent, StepStartEvent,
};
use crate::types::{BatchConfig, RunConfig, ScenarioSpec};
use crate::validation;
use crate::validation::CompiledScenario;

/// Public entrypoint for compile/run/batch workflows.
#[derive(Debug, Default, Clone, Copy)]
pub struct Simulator;

impl Simulator {
    /// Compile a scenario spec into deterministic indexes and validated structure.
    pub fn compile(spec: ScenarioSpec) -> Result<CompiledScenario, SetupError> {
        validation::compile_scenario(&spec)
    }

    /// Execute a single deterministic run and optionally stream synthetic run events to a sink.
    #[allow(clippy::needless_option_as_deref)]
    pub fn run(
        compiled: &CompiledScenario,
        config: RunConfig,
        mut sink: Option<&mut dyn EventSink>,
    ) -> Result<crate::types::RunReport, RunError> {
        validation::validate_run_config(&config).map_err(map_setup_to_run)?;
        let report = if let Some(active_sink) = sink.as_deref_mut() {
            engine::run_single_streaming(compiled, &config, "run-0", active_sink)?
        } else {
            engine::run_single(compiled, &config)?
        };

        if let Some(active_sink) = sink.as_deref_mut() {
            active_sink.flush().map_err(map_sink_error)?;
        }

        Ok(report)
    }

    /// Execute a single deterministic run, evaluate run expectations, and optionally stream run + assertion checkpoint events.
    #[allow(clippy::needless_option_as_deref)]
    pub fn run_with_assertions(
        compiled: &CompiledScenario,
        config: RunConfig,
        expectations: &[Expectation],
        mut sink: Option<&mut dyn EventSink>,
    ) -> Result<(crate::types::RunReport, AssertionReport), SimError> {
        validation::validate_run_config(&config)?;
        let report = if let Some(active_sink) = sink.as_deref_mut() {
            engine::run_single_streaming_for_assertions(compiled, &config, "run-0", active_sink)?
        } else {
            engine::run_single(compiled, &config)?
        };
        let assertion_report = assertions::evaluate_run_expectations(&report, expectations)?;

        if let Some(active_sink) = sink.as_deref_mut() {
            emit_assertion_checkpoints(
                active_sink,
                "run-0",
                report.steps_executed,
                &assertion_report,
            )
            .map_err(map_sink_error)?;
            active_sink
                .push(RunEvent::step_end(
                    "run-0",
                    report.steps_executed,
                    assertion_report.results.len() as u64,
                    StepEndEvent { completed: report.completed },
                ))
                .map_err(map_sink_error)?;
            active_sink.flush().map_err(map_sink_error)?;
        }

        Ok((report, assertion_report))
    }

    /// Execute deterministic batch runs and optionally stream summary events to a sink.
    pub fn run_batch(
        compiled: &CompiledScenario,
        config: BatchConfig,
        sink: Option<&mut dyn EventSink>,
    ) -> Result<crate::types::BatchReport, RunError> {
        validation::validate_batch_config(&config).map_err(map_setup_to_run)?;
        let report = batch::run_batch(compiled, &config)?;

        if let Some(sink) = sink {
            emit_batch_events(sink, &report)?;
            sink.flush().map_err(map_sink_error)?;
        }

        Ok(report)
    }

    /// Execute deterministic batch runs, evaluate batch expectations, and optionally stream summary + assertion checkpoint events.
    pub fn run_batch_with_assertions(
        compiled: &CompiledScenario,
        config: BatchConfig,
        expectations: &[Expectation],
        sink: Option<&mut dyn EventSink>,
    ) -> Result<(crate::types::BatchReport, AssertionReport), SimError> {
        validation::validate_batch_config(&config)?;
        let report = batch::run_batch(compiled, &config)?;
        let assertion_report = assertions::evaluate_batch_expectations(&report, expectations)?;

        if let Some(sink) = sink {
            emit_batch_events(sink, &report)?;
            emit_assertion_checkpoints(sink, "batch", 0, &assertion_report)
                .map_err(map_sink_error)?;
            sink.flush().map_err(map_sink_error)?;
        }

        Ok((report, assertion_report))
    }
}

fn map_setup_to_run(error: SetupError) -> RunError {
    match error {
        SetupError::InvalidParameter { name, reason } => {
            RunError::InvalidRunConfig { name, reason }
        }
        SetupError::InvalidGraphReference { graph, reference } => {
            RunError::InvalidRunConfig { name: graph, reason: reference }
        }
        SetupError::CyclicGraph { graph, cycle_path } => RunError::InvalidRunConfig {
            name: graph,
            reason: format!("resource cycle detected: {}", cycle_path.join(" -> ")),
        },
    }
}

fn map_sink_error(error: EventSinkError) -> RunError {
    RunError::EventSink { message: error.to_string() }
}

fn emit_batch_events(
    sink: &mut dyn EventSink,
    report: &crate::types::BatchReport,
) -> Result<(), RunError> {
    for run in &report.runs {
        let run_id = format!("run-{}", run.run_index);
        sink.push(RunEvent::step_start(run_id.clone(), 0, 0, StepStartEvent { seed: run.seed }))
            .map_err(map_sink_error)?;

        let mut ordinal = 0_u64;
        for (metric, value) in &run.final_metrics {
            sink.push(RunEvent::metric_snapshot(
                run_id.clone(),
                run.steps_executed,
                ordinal,
                MetricSnapshotEvent { metric: metric.clone(), value: *value },
            ))
            .map_err(map_sink_error)?;
            ordinal = ordinal.saturating_add(1);
        }

        sink.push(RunEvent::step_end(
            run_id,
            run.steps_executed,
            ordinal,
            StepEndEvent { completed: run.completed },
        ))
        .map_err(map_sink_error)?;
    }

    Ok(())
}

fn emit_assertion_checkpoints(
    sink: &mut dyn EventSink,
    run_id: &str,
    step: u64,
    assertion_report: &AssertionReport,
) -> Result<(), EventSinkError> {
    for (index, result) in assertion_report.results.iter().enumerate() {
        let payload = AssertionCheckpointEvent {
            checkpoint_id: format!("assertion-{index}"),
            passed: result.passed,
            expected: result.expected.clone(),
            actual: result.actual.clone(),
            evidence_refs: result
                .evidence_refs
                .iter()
                .map(|reference| format!("{}@{}", reference.metric, reference.context))
                .collect::<Vec<_>>(),
        };
        sink.push(RunEvent::assertion_checkpoint(run_id, step, index as u64, payload))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::assertions::{Expectation, MetricSelector};
    use crate::error::{RunError, SetupError, SimError};
    use crate::events::{EventSink, EventSinkError, RunEvent, RunEventPhase, VecEventSink};
    use crate::testkit::{
        deterministic_batch_config, deterministic_run_config, fixture_compiled_scenario,
        fixture_scenario,
    };
    use crate::types::MetricKey;
    use crate::types::{
        BatchConfig, CaptureConfig, EndConditionSpec, ExecutionMode, NodeId, RunConfig,
    };
    use crate::validation::compile_scenario;

    use super::{map_setup_to_run, map_sink_error, Simulator};

    #[test]
    fn simulator_compile_run_and_batch_workflow() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let mut sink = VecEventSink::new();

        let run_report = Simulator::run(&compiled, deterministic_run_config(), Some(&mut sink))
            .expect("single run should succeed");
        assert!(run_report.completed);
        assert!(!sink.events().is_empty());

        let mut batch_sink = VecEventSink::new();
        let batch_report =
            Simulator::run_batch(&compiled, deterministic_batch_config(), Some(&mut batch_sink))
                .expect("batch run should succeed");
        assert_eq!(batch_report.completed_runs, batch_report.requested_runs);
        assert!(!batch_sink.events().is_empty());
    }

    #[test]
    fn simulator_run_with_assertions_emits_assertion_checkpoints_and_transfers() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let mut sink = VecEventSink::new();
        let expectations = vec![Expectation::Equals {
            metric: MetricKey::fixture("sink"),
            selector: MetricSelector::Final,
            expected: 3.0,
        }];

        let (_report, assertion_report) = Simulator::run_with_assertions(
            &compiled,
            deterministic_run_config(),
            &expectations,
            Some(&mut sink),
        )
        .expect("run with assertions should succeed");
        assert_eq!(assertion_report.total, 1);
        assert_eq!(assertion_report.failed, 0);

        let names = sink.events().iter().map(|event| event.event_name()).collect::<Vec<_>>();
        assert!(names.contains(&"transfer"));
        assert!(names.contains(&"assertion_checkpoint"));
    }

    #[test]
    fn simulator_run_streams_events_in_monotonic_step_lifecycle_order() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let mut sink = VecEventSink::new();
        let report = Simulator::run(&compiled, deterministic_run_config(), Some(&mut sink))
            .expect("run should succeed");
        let events = sink.events();

        assert!(!events.is_empty(), "stream should emit events");
        assert!(
            events.windows(2).all(|pair| pair[0].order() <= pair[1].order()),
            "raw stream must already be monotonic"
        );

        let mut step_start_positions = BTreeMap::<u64, usize>::new();
        let mut step_end_positions = BTreeMap::<u64, usize>::new();
        for (index, event) in events.iter().enumerate() {
            let order = event.order();
            match order.phase {
                RunEventPhase::StepStart => {
                    step_start_positions.insert(order.step, index);
                }
                RunEventPhase::StepEnd => {
                    step_end_positions.insert(order.step, index);
                }
                _ => {}
            }
        }

        for step in 1..=report.steps_executed {
            let start = *step_start_positions
                .get(&step)
                .expect("every executed step should emit step_start");
            let end =
                *step_end_positions.get(&step).expect("every executed step should emit step_end");
            assert!(start < end, "step_start must precede step_end for step {step}");
        }

        for (index, event) in events.iter().enumerate() {
            let order = event.order();
            if matches!(order.phase, RunEventPhase::StepStart | RunEventPhase::StepEnd) {
                continue;
            }
            let Some(start) = step_start_positions.get(&order.step).copied() else {
                continue;
            };
            let Some(end) = step_end_positions.get(&order.step).copied() else {
                continue;
            };
            assert!(
                index > start && index < end,
                "phase {:?} must stay within step lifecycle boundaries",
                order.phase
            );
        }
    }

    #[test]
    fn simulator_run_step_zero_completion_emits_terminal_step_end() {
        let compiled = immediate_completion_compiled_scenario();
        let mut sink = VecEventSink::new();
        let report = Simulator::run(&compiled, deterministic_run_config(), Some(&mut sink))
            .expect("step-zero complete run should succeed");
        let events = sink.events();

        assert_eq!(report.steps_executed, 0);
        assert!(report.completed);

        let step_start_index = events
            .iter()
            .position(|event| {
                let order = event.order();
                order.step == 0 && matches!(order.phase, RunEventPhase::StepStart)
            })
            .expect("step-zero run should emit step_start");
        let step_end_index = events
            .iter()
            .position(|event| {
                let order = event.order();
                order.step == 0 && matches!(order.phase, RunEventPhase::StepEnd)
            })
            .expect("step-zero run should emit terminal step_end");
        assert!(step_start_index < step_end_index);
    }

    #[test]
    fn simulator_run_with_assertions_step_zero_places_checkpoints_before_step_end() {
        let compiled = immediate_completion_compiled_scenario();
        let expectations = vec![Expectation::Equals {
            metric: MetricKey::fixture("sink"),
            selector: MetricSelector::Final,
            expected: 0.0,
        }];
        let mut sink = VecEventSink::new();
        let (report, assertion_report) = Simulator::run_with_assertions(
            &compiled,
            deterministic_run_config(),
            &expectations,
            Some(&mut sink),
        )
        .expect("step-zero run_with_assertions should succeed");
        let events = sink.events();

        assert_eq!(report.steps_executed, 0);
        assert!(report.completed);
        assert_eq!(assertion_report.total, 1);
        assert_eq!(assertion_report.failed, 0);

        let checkpoint_index = events
            .iter()
            .position(|event| event.event_name() == "assertion_checkpoint")
            .expect("assertion checkpoint should be emitted");
        let terminal_step_end_index = events
            .iter()
            .position(|event| {
                let order = event.order();
                order.step == 0 && matches!(order.phase, RunEventPhase::StepEnd)
            })
            .expect("terminal step_end should be emitted");
        assert!(checkpoint_index < terminal_step_end_index);
    }

    #[test]
    fn simulator_run_batch_with_assertions_emits_batch_checkpoints() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let mut sink = VecEventSink::new();
        let expectations = vec![Expectation::Between {
            metric: MetricKey::fixture("sink"),
            selector: MetricSelector::Final,
            min: 1.0,
            max: 3.0,
        }];

        let (_report, assertion_report) = Simulator::run_batch_with_assertions(
            &compiled,
            deterministic_batch_config(),
            &expectations,
            Some(&mut sink),
        )
        .expect("batch with assertions should succeed");
        assert_eq!(assertion_report.total, 1);

        let names = sink.events().iter().map(|event| event.event_name()).collect::<Vec<_>>();
        assert!(names.contains(&"metric_snapshot"));
        assert!(names.contains(&"assertion_checkpoint"));
    }

    #[test]
    fn simulator_run_and_batch_map_invalid_config_errors() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let invalid_run = RunConfig { seed: 1, max_steps: 0, capture: CaptureConfig::default() };

        let run_error = Simulator::run(&compiled, invalid_run, None).expect_err("must fail");
        assert!(matches!(
            run_error,
            RunError::InvalidRunConfig { name, reason }
                if name == "run.max_steps" && reason == "must be greater than 0"
        ));

        let invalid_batch = BatchConfig {
            runs: 2,
            base_seed: 1,
            execution_mode: ExecutionMode::SingleThread,
            run: RunConfig { seed: 1, max_steps: 0, capture: CaptureConfig::default() },
        };
        let batch_error =
            Simulator::run_batch(&compiled, invalid_batch, None).expect_err("must fail");
        assert!(matches!(
            batch_error,
            RunError::InvalidRunConfig { name, reason }
                if name == "batch.run.max_steps" && reason == "must be greater than 0"
        ));
    }

    #[test]
    fn simulator_sink_errors_are_mapped_for_push_and_flush() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let mut push_failing_sink = FailingSink { fail_on_push: true, fail_on_flush: false };
        let push_error =
            Simulator::run(&compiled, deterministic_run_config(), Some(&mut push_failing_sink))
                .expect_err("push failure should surface");
        assert!(
            matches!(push_error, RunError::EventSink { message } if message.contains("push failed"))
        );

        let mut flush_failing_sink = FailingSink { fail_on_push: false, fail_on_flush: true };
        let flush_error =
            Simulator::run(&compiled, deterministic_run_config(), Some(&mut flush_failing_sink))
                .expect_err("flush failure should surface");
        assert!(
            matches!(flush_error, RunError::EventSink { message } if message.contains("flush failed"))
        );

        let mut fail_after_two_pushes =
            FailAfterNSink { fail_after_pushes: 1, pushes: 0, flushes: 0 };
        let streaming_error =
            Simulator::run(&compiled, deterministic_run_config(), Some(&mut fail_after_two_pushes))
                .expect_err("streaming push failure should surface");
        assert!(
            matches!(streaming_error, RunError::EventSink { message } if message.contains("forced push failure"))
        );
        assert_eq!(
            fail_after_two_pushes.pushes, 2,
            "event streaming should stop immediately after the first failing push"
        );
        assert_eq!(fail_after_two_pushes.flushes, 0, "flush should not run after a push failure");
    }

    #[test]
    fn simulator_run_with_assertions_maps_checkpoint_push_failure() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let expectations = vec![Expectation::Equals {
            metric: MetricKey::fixture("sink"),
            selector: MetricSelector::Final,
            expected: 3.0,
        }];
        let mut sink = FailOnEventNameSink::new("assertion_checkpoint");
        let error = Simulator::run_with_assertions(
            &compiled,
            deterministic_run_config(),
            &expectations,
            Some(&mut sink),
        )
        .expect_err("assertion checkpoint push failure should surface");
        assert!(matches!(
            error,
            SimError::Run(RunError::EventSink { message })
                if message.contains("assertion_checkpoint")
        ));
        assert_eq!(sink.flushes, 0, "flush should not run after checkpoint push failure");
    }

    #[test]
    fn simulator_run_with_assertions_maps_flush_failure() {
        let compiled = fixture_compiled_scenario().expect("compiled fixture");
        let expectations = vec![Expectation::Equals {
            metric: MetricKey::fixture("sink"),
            selector: MetricSelector::Final,
            expected: 3.0,
        }];
        let mut sink = FailingSink { fail_on_push: false, fail_on_flush: true };
        let error = Simulator::run_with_assertions(
            &compiled,
            deterministic_run_config(),
            &expectations,
            Some(&mut sink),
        )
        .expect_err("flush failure should surface");
        assert!(
            matches!(error, SimError::Run(RunError::EventSink { message }) if message.contains("flush failed"))
        );
    }

    #[test]
    fn simulator_maps_setup_and_sink_errors_via_helpers() {
        let mapped_graph = map_setup_to_run(SetupError::InvalidGraphReference {
            graph: "scenario[g].nodes".to_string(),
            reference: "missing edge".to_string(),
        });
        assert!(matches!(
            mapped_graph,
            RunError::InvalidRunConfig { name, reason }
                if name == "scenario[g].nodes" && reason == "missing edge"
        ));

        let mapped_cycle = map_setup_to_run(SetupError::CyclicGraph {
            graph: "scenario[g].resource_connections".to_string(),
            cycle_path: vec!["a".to_string(), "b".to_string(), "a".to_string()],
        });
        assert!(matches!(
            mapped_cycle,
            RunError::InvalidRunConfig { name, reason }
                if name == "scenario[g].resource_connections"
                    && reason == "resource cycle detected: a -> b -> a"
        ));

        let mapped_sink = map_sink_error(EventSinkError::custom("sink-x", "boom"));
        assert!(
            matches!(mapped_sink, RunError::EventSink { message } if message.contains("sink-x"))
        );

        let sim_error = Simulator::run_with_assertions(
            &fixture_compiled_scenario().expect("compiled fixture"),
            RunConfig { seed: 1, max_steps: 0, capture: CaptureConfig::default() },
            &[],
            None,
        )
        .expect_err("invalid config should map to sim error");
        assert!(matches!(
            sim_error,
            SimError::Setup(SetupError::InvalidParameter { name, .. }) if name == "run.max_steps"
        ));
    }

    struct FailingSink {
        fail_on_push: bool,
        fail_on_flush: bool,
    }

    impl EventSink for FailingSink {
        fn push(&mut self, _event: RunEvent) -> Result<(), EventSinkError> {
            if self.fail_on_push {
                Err(EventSinkError::Custom {
                    sink: "failing_sink".to_string(),
                    message: "push failed".to_string(),
                })
            } else {
                Ok(())
            }
        }

        fn flush(&mut self) -> Result<(), EventSinkError> {
            if self.fail_on_flush {
                Err(EventSinkError::Custom {
                    sink: "failing_sink".to_string(),
                    message: "flush failed".to_string(),
                })
            } else {
                Ok(())
            }
        }
    }

    struct FailAfterNSink {
        fail_after_pushes: usize,
        pushes: usize,
        flushes: usize,
    }

    impl EventSink for FailAfterNSink {
        fn push(&mut self, _event: RunEvent) -> Result<(), EventSinkError> {
            self.pushes = self.pushes.saturating_add(1);
            if self.pushes > self.fail_after_pushes {
                Err(EventSinkError::Custom {
                    sink: "fail_after_n_sink".to_string(),
                    message: "forced push failure".to_string(),
                })
            } else {
                Ok(())
            }
        }

        fn flush(&mut self) -> Result<(), EventSinkError> {
            self.flushes = self.flushes.saturating_add(1);
            Ok(())
        }
    }

    struct FailOnEventNameSink {
        event_name: &'static str,
        flushes: usize,
    }

    impl FailOnEventNameSink {
        fn new(event_name: &'static str) -> Self {
            Self { event_name, flushes: 0 }
        }
    }

    impl EventSink for FailOnEventNameSink {
        fn push(&mut self, event: RunEvent) -> Result<(), EventSinkError> {
            if event.event_name() == self.event_name {
                return Err(EventSinkError::Custom {
                    sink: "fail_on_event_name_sink".to_string(),
                    message: format!("forced failure on {}", self.event_name),
                });
            }
            Ok(())
        }

        fn flush(&mut self) -> Result<(), EventSinkError> {
            self.flushes = self.flushes.saturating_add(1);
            Ok(())
        }
    }

    fn immediate_completion_compiled_scenario() -> crate::validation::CompiledScenario {
        let mut scenario = fixture_scenario();
        scenario.end_conditions = vec![EndConditionSpec::NodeAtLeast {
            node_id: NodeId::fixture("sink"),
            value_scaled: 0,
        }];
        compile_scenario(&scenario).expect("step-zero completion fixture should compile")
    }
}
