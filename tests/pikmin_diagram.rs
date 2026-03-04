use std::collections::BTreeMap;

use anapao::assertions::{Expectation, MetricSelector};
use anapao::error::SetupError;
use anapao::events::{RunEventPhase, VecEventSink};
use anapao::stats::prediction_indicators_by_metric;
use anapao::testkit::pikmin::{
    days_spent_metric_key, days_spent_node_id, pikmin_die_metric_key, pikmin_scenario,
    pikmin_scenario_for_profile, ship_parts_metric_key, ship_parts_node_id, PikminFixtureProfile,
    PikminFixtureTuning,
};
use anapao::types::{BatchConfig, CaptureConfig, ExecutionMode, MetricKey, RunConfig};
use anapao::Simulator;

fn pikmin_scenario_from_profile(profile: PikminFixtureProfile) -> anapao::types::ScenarioSpec {
    pikmin_scenario_for_profile(profile).expect("pikmin profile scenario should build")
}

#[test]
fn pikmin_diagram_bad_ending_hits_day_limit_first() {
    let scenario = pikmin_scenario_from_profile(PikminFixtureProfile::BadEndingBiased);
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let config = RunConfig { seed: 2026, max_steps: 60, capture: CaptureConfig::disabled() };

    let report = Simulator::run(&compiled, config, None).expect("run should succeed");
    let days_spent = days_spent_node_id();
    let ship_parts = ship_parts_node_id();

    assert!(report.completed);
    assert_eq!(report.steps_executed, 30);
    assert_eq!(report.final_node_values.get(&days_spent), Some(&30.0));
    assert!(
        report.final_node_values.get(&ship_parts).copied().expect("ship_parts node should exist")
            < 30.0
    );
}

#[test]
fn pikmin_diagram_good_ending_reaches_ship_parts_before_day_limit() {
    let scenario = pikmin_scenario_from_profile(PikminFixtureProfile::GoodEndingBiased);
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let config = RunConfig { seed: 2026, max_steps: 60, capture: CaptureConfig::disabled() };

    let report = Simulator::run(&compiled, config, None).expect("run should succeed");
    let days_spent = days_spent_node_id();
    let ship_parts = ship_parts_node_id();
    let pikmin = anapao::testkit::pikmin::pikmin_node_id();

    assert!(report.completed);
    assert!(report.steps_executed < 30);
    assert!(
        report.final_node_values.get(&ship_parts).copied().expect("ship_parts node should exist")
            >= 30.0
    );
    assert!(
        report.final_node_values.get(&days_spent).copied().expect("days_spent node should exist")
            < 30.0
    );
    assert!(report.final_node_values.get(&pikmin).copied().unwrap_or(0.0) >= 0.0);
}

#[test]
fn pikmin_diagram_is_reproducible_for_fixed_seed() {
    let tuning = PikminFixtureTuning::new(4, 2, 40.0).expect("valid custom tuning");
    let scenario = pikmin_scenario(tuning).expect("pikmin scenario should build");
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let config = RunConfig { seed: 777, max_steps: 60, capture: CaptureConfig::disabled() };

    let report_a = Simulator::run(&compiled, config.clone(), None).expect("first run should work");
    let report_b = Simulator::run(&compiled, config, None).expect("second run should work");
    assert_eq!(report_a.steps_executed, report_b.steps_executed);
    assert_eq!(report_a.completed, report_b.completed);
    assert_eq!(report_a.final_node_values, report_b.final_node_values);
    assert_eq!(report_a.final_metrics, report_b.final_metrics);
}

#[test]
fn pikmin_diagram_batch_probability_band_for_good_ending_threshold() {
    let scenario = pikmin_scenario_from_profile(PikminFixtureProfile::Balanced);
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let batch_config = BatchConfig {
        runs: 256,
        base_seed: 0x5050,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 0, max_steps: 60, capture: CaptureConfig::disabled() },
    };
    let expectations = vec![Expectation::ProbabilityBand {
        metric: ship_parts_metric_key(),
        min: 30.0,
        max: 10_000.0,
        probability_min: 0.85,
        probability_max: 0.99,
    }];

    let (_report, assertion_report) =
        Simulator::run_batch_with_assertions(&compiled, batch_config, &expectations, None)
            .expect("batch run with assertions should succeed");

    assert!(
        assertion_report.is_success(),
        "expected probability band expectation to pass, got {:?}",
        assertion_report.results
    );
}

#[test]
fn pikmin_diagram_balance_guardrails_from_prediction_indicators() {
    let scenario = pikmin_scenario_from_profile(PikminFixtureProfile::Balanced);
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let batch_config = BatchConfig {
        runs: 256,
        base_seed: 0x6060,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 0, max_steps: 60, capture: CaptureConfig::disabled() },
    };

    let batch_report =
        Simulator::run_batch(&compiled, batch_config, None).expect("batch run should succeed");

    let mut values_by_metric = BTreeMap::<MetricKey, Vec<f64>>::new();
    for run in &batch_report.runs {
        for (metric, value) in &run.final_metrics {
            values_by_metric.entry(metric.clone()).or_default().push(*value);
        }
    }

    let indicators = prediction_indicators_by_metric(values_by_metric);
    let days =
        indicators.get(&days_spent_metric_key()).expect("days spent indicators should exist");
    let losses =
        indicators.get(&pikmin_die_metric_key()).expect("pikmin die indicators should exist");
    let ship_parts =
        indicators.get(&ship_parts_metric_key()).expect("ship parts indicators should exist");

    // Guardrail 1: campaign should usually progress deep into the 30-day horizon,
    // but must still terminate by day 30 due end-condition race.
    assert!(days.median >= 20.0 && days.median <= 30.0, "unexpected day median {}", days.median);
    // Guardrail 2: p90 losses are bounded by loss_roll max (6) across 30-day horizon (<=180).
    assert!(losses.p90 <= 180.0, "unexpected p90 pikmin losses {}", losses.p90);
    // Guardrail 3: convergence drift for ship part outcomes should stay bounded for CI stability.
    assert!(
        ship_parts.convergence_delta < 50.0,
        "unexpected ship parts convergence delta {}",
        ship_parts.convergence_delta
    );
}

#[test]
fn pikmin_diagram_event_contract_contains_core_phases_and_stable_ordering() {
    let tuning = PikminFixtureTuning::new(3, 2, 70.0).expect("valid custom tuning");
    let scenario = pikmin_scenario(tuning).expect("pikmin scenario should build");
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let run_config = RunConfig { seed: 9001, max_steps: 60, capture: CaptureConfig::default() };
    let expectations = vec![Expectation::Between {
        metric: ship_parts_metric_key(),
        selector: MetricSelector::Final,
        min: 0.0,
        max: 10_000.0,
    }];

    let mut sink = VecEventSink::new();
    let (report, assertion_report) =
        Simulator::run_with_assertions(&compiled, run_config, &expectations, Some(&mut sink))
            .expect("run with assertions should succeed");
    assert!(assertion_report.is_success());

    let events = sink.events();
    assert!(!events.is_empty(), "event stream should not be empty");
    assert!(events.iter().any(|event| event.event_name() == "step_start"));
    assert!(events.iter().any(|event| event.event_name() == "transfer"));
    assert!(events.iter().any(|event| event.event_name() == "metric_snapshot"));
    assert!(events.iter().any(|event| event.event_name() == "assertion_checkpoint"));
    assert!(events.iter().any(|event| event.event_name() == "step_end"));

    assert!(
        events.windows(2).all(|pair| pair[0].order() <= pair[1].order()),
        "raw sink order must already be monotonic"
    );

    let mut step_start_positions = BTreeMap::<u64, usize>::new();
    let mut step_end_positions = BTreeMap::<u64, usize>::new();

    for (index, event) in events.iter().enumerate() {
        let order = event.order();
        match order.phase {
            RunEventPhase::StepStart => {
                assert!(
                    step_start_positions.insert(order.step, index).is_none(),
                    "each step should have only one step_start"
                );
            }
            RunEventPhase::StepEnd => {
                assert!(
                    step_start_positions.contains_key(&order.step),
                    "step_end must have a corresponding step_start"
                );
                assert!(
                    step_end_positions.insert(order.step, index).is_none(),
                    "each step should have only one step_end"
                );
            }
            RunEventPhase::AssertionCheckpoint => {
                assert_eq!(order.step, report.steps_executed);
                let start = *step_start_positions
                    .get(&order.step)
                    .expect("terminal step must already have started");
                assert!(
                    index > start,
                    "assertion checkpoint must be emitted after terminal step_start"
                );
            }
            _ => {
                let start = *step_start_positions
                    .get(&order.step)
                    .expect("intermediate phases require prior step_start");
                let ended = step_end_positions.get(&order.step).copied();
                assert!(index > start, "intermediate phase must occur after step_start");
                assert!(
                    ended.is_none(),
                    "intermediate phase cannot occur after step_end for the same step"
                );
            }
        }
    }

    for step in 1..=report.steps_executed {
        let start =
            *step_start_positions.get(&step).expect("every executed step should have step_start");
        let end = *step_end_positions.get(&step).expect("every executed step should have step_end");
        assert!(start < end, "step_start must precede step_end for step {step}");
    }

    let terminal_step_end = step_end_positions
        .get(&report.steps_executed)
        .copied()
        .expect("terminal step_end should be emitted");
    for (index, event) in events.iter().enumerate() {
        if matches!(event.order().phase, RunEventPhase::AssertionCheckpoint) {
            assert!(
                index < terminal_step_end,
                "assertion checkpoints must precede terminal step_end"
            );
        }
    }
}

#[test]
fn pikmin_diagram_event_stream_is_byte_stable_for_fixed_seed() {
    let scenario = pikmin_scenario_from_profile(PikminFixtureProfile::Balanced);
    let compiled = Simulator::compile(scenario).expect("pikmin diagram scenario should compile");
    let run_config = RunConfig { seed: 1337, max_steps: 60, capture: CaptureConfig::default() };
    let expectations = vec![Expectation::Between {
        metric: ship_parts_metric_key(),
        selector: MetricSelector::Final,
        min: 0.0,
        max: 10_000.0,
    }];

    let mut sink_a = VecEventSink::new();
    let mut sink_b = VecEventSink::new();

    let (_report_a, assertion_a) = Simulator::run_with_assertions(
        &compiled,
        run_config.clone(),
        &expectations,
        Some(&mut sink_a),
    )
    .expect("first run should succeed");
    let (_report_b, assertion_b) =
        Simulator::run_with_assertions(&compiled, run_config, &expectations, Some(&mut sink_b))
            .expect("second run should succeed");

    assert_eq!(assertion_a.results, assertion_b.results);

    let encoded_a = serde_json::to_string(sink_a.events()).expect("stream A should serialize");
    let encoded_b = serde_json::to_string(sink_b.events()).expect("stream B should serialize");
    assert_eq!(encoded_a, encoded_b, "fixed-seed event stream must be byte-stable");
}

#[test]
fn pikmin_diagram_rejects_unresolved_tracked_metric_at_compile_time() {
    let mut scenario = pikmin_scenario(PikminFixtureTuning::new(3, 2, 70.0).expect("valid tuning"))
        .expect("pikmin scenario should build");
    scenario.tracked_metrics.insert(MetricKey::fixture("n99_missing_metric"));
    let available_metric_keys =
        scenario.nodes.keys().map(ToString::to_string).collect::<Vec<_>>().join(", ");

    let error = Simulator::compile(scenario).expect_err("compile should reject unresolved metric");
    match error {
        SetupError::InvalidGraphReference { graph, reference } => {
            assert_eq!(graph, "scenario[scenario-pikmin-diagram].metrics");
            assert_eq!(
                reference,
                format!(
                    "tracked_metrics[n99_missing_metric] references unresolved metric `n99_missing_metric`; hint: choose one of the available metric keys: [{available_metric_keys}]"
                )
            );
        }
        other => panic!("expected InvalidGraphReference, got {other:?}"),
    }
}
