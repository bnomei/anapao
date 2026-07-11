use std::collections::BTreeSet;
use std::fs;

use anapao::artifact::write_run_artifacts;
use anapao::assertions::{Expectation, MetricSelector};
use anapao::error::RunError;
use anapao::events::VecEventSink;
use anapao::types::{
    AggregationConfig, BatchConfig, BatchRunTemplate, CaptureConfig, CaptureSchedule, EdgeId,
    EndConditionSpec, MetricKey, NodeId, RunConfig, ScenarioSpec, Selection, TransferSpec,
};
use anapao::Simulator;

fn compiled_scenario() -> anapao::CompiledScenario {
    let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 });
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];
    scenario.tracked_metrics.insert(MetricKey::fixture("sink"));
    Simulator::compile(scenario).expect("fixture must compile")
}

#[test]
fn none_retains_terminal_maps_but_no_diagnostics_or_events_difference() {
    let compiled = compiled_scenario();
    let default_config = RunConfig::for_seed(8).with_max_steps(4);
    let none_config = default_config.clone().with_capture(CaptureConfig::none());

    let mut default_events = VecEventSink::new();
    let default_report = Simulator::run_with_sink(&compiled, &default_config, &mut default_events)
        .expect("default run succeeds");
    let mut none_events = VecEventSink::new();
    let none_report = Simulator::run_with_sink(&compiled, &none_config, &mut none_events)
        .expect("none run succeeds");

    assert!(none_report.node_snapshots.is_empty());
    assert!(none_report.variable_snapshots.is_empty());
    assert!(none_report.transfers.is_empty());
    assert!(none_report.series.is_empty());
    assert!(!none_report.final_node_values.is_empty());
    assert!(!none_report.final_metrics.is_empty());
    assert_eq!(none_report.final_node_values, default_report.final_node_values);
    assert_eq!(none_report.final_metrics, default_report.final_metrics);
    assert_eq!(none_events.events(), default_events.events());
}

#[test]
fn no_capture_assertion_streaming_keeps_events_final_assertions_and_artifacts_usable() {
    let compiled = compiled_scenario();
    let default_config = RunConfig::for_seed(8).with_max_steps(4);
    let none_config = default_config.clone().with_capture(CaptureConfig::none());
    let final_expectation = [Expectation::Equals {
        metric: MetricKey::fixture("sink"),
        selector: MetricSelector::Final,
        expected: 2.0,
    }];

    let mut default_events = VecEventSink::new();
    let (_default_report, default_assertions) = Simulator::run_with_assertions_and_sink(
        &compiled,
        &default_config,
        &final_expectation,
        &mut default_events,
    )
    .expect("default assertion-streaming run succeeds");
    let mut no_capture_events = VecEventSink::new();
    let (report, no_capture_assertions) = Simulator::run_with_assertions_and_sink(
        &compiled,
        &none_config,
        &final_expectation,
        &mut no_capture_events,
    )
    .expect("no-capture assertion-streaming run succeeds");
    assert!(default_assertions.is_success());
    assert!(no_capture_assertions.is_success());
    assert_eq!(no_capture_events.events(), default_events.events());
    assert!(no_capture_events
        .events()
        .iter()
        .any(|event| event.event_name() == "assertion_checkpoint"));
    assert!(no_capture_events.events().windows(2).all(|pair| pair[0].order() <= pair[1].order()));
    let checkpoint_index = no_capture_events
        .events()
        .iter()
        .position(|event| event.event_name() == "assertion_checkpoint")
        .expect("final assertion checkpoint is emitted");
    let terminal_step_end_index = no_capture_events
        .events()
        .iter()
        .rposition(|event| event.event_name() == "step_end")
        .expect("terminal step end is emitted");
    assert!(checkpoint_index < terminal_step_end_index);

    let metric = MetricKey::fixture("sink");
    let missing_step_expectations = [
        Expectation::Equals {
            metric: metric.clone(),
            selector: MetricSelector::Final,
            expected: 2.0,
        },
        Expectation::Equals { metric, selector: MetricSelector::Step(1), expected: 1.0 },
    ];
    let mut missing_step_events = VecEventSink::new();
    let (_report, assertions) = Simulator::run_with_assertions_and_sink(
        &compiled,
        &none_config,
        &missing_step_expectations,
        &mut missing_step_events,
    )
    .expect("no-capture step assertion-streaming run succeeds");
    assert_eq!(assertions.passed, 1);
    assert_eq!(assertions.failed, 1);
    assert!(assertions.results[1].actual.contains("missing metric `sink` at `run.series.step=1`"));
    assert_eq!(
        missing_step_events
            .events()
            .iter()
            .filter(|event| event.event_name() == "assertion_checkpoint")
            .count(),
        2
    );
    assert!(missing_step_events.events().windows(2).all(|pair| pair[0].order() <= pair[1].order()));

    let output = tempfile::tempdir().expect("temporary artifact directory");
    let manifest =
        write_run_artifacts(output.path(), &report, no_capture_events.events()).expect("artifacts");
    assert!(manifest.artifacts.contains_key("events"));
    assert!(manifest.artifacts.contains_key("variables"));
    assert!(manifest.artifacts.contains_key("series"));
    assert!(!fs::read_to_string(output.path().join("events.jsonl"))
        .expect("events output")
        .is_empty());
    assert_eq!(
        fs::read_to_string(output.path().join("variables.csv")).expect("variables output"),
        "variable,step,value\n"
    );
    assert_eq!(
        fs::read_to_string(output.path().join("series.csv")).expect("series output"),
        "metric,step,value\n"
    );
}

#[test]
fn typed_and_legacy_capture_wire_decode_to_canonical_current_output() {
    let legacy = r#"{
        "capture_nodes": ["sink"],
        "capture_metrics": ["sink"],
        "every_n_steps": 3,
        "include_step_zero": false,
        "include_final_state": false
    }"#;
    let capture: CaptureConfig = serde_json::from_str(legacy).expect("legacy wire decodes");
    assert!(matches!(
        capture.schedule(),
        CaptureSchedule::Every { stride, include_initial: false, include_final: false }
            if stride.get() == 3
    ));
    assert_eq!(capture.nodes().only(), Some(&BTreeSet::from([NodeId::fixture("sink")])));

    let output = serde_json::to_value(&capture).expect("current wire encodes");
    assert_eq!(output["schedule"]["kind"], "every");
    assert_eq!(output["nodes"]["kind"], "only");
    assert_eq!(output["nodes"]["items"], serde_json::json!(["sink"]));
    assert!(output.get("capture_nodes").is_none());
    assert!(output.get("every_n_steps").is_none());
}

#[test]
fn empty_only_selection_is_rejected_and_final_policy_has_no_transfers() {
    let empty_only = r#"{
        "schedule": {"kind":"final"},
        "nodes": {"kind":"only","items":[]},
        "metrics": {"kind":"all"},
        "variables": {"kind":"all"},
        "transfers": {"kind":"none"}
    }"#;
    assert!(serde_json::from_str::<CaptureConfig>(empty_only).is_err());

    let final_only = CaptureConfig::final_only();
    assert!(matches!(final_only.schedule(), CaptureSchedule::Final));
    assert!(matches!(final_only.transfers(), Selection::None));
}

#[test]
fn capture_wire_rejects_unknown_and_mixed_current_legacy_fields() {
    let current_with_legacy = r#"{
        "schedule": {"kind":"none"},
        "nodes": {"kind":"none"},
        "metrics": {"kind":"none"},
        "variables": {"kind":"none"},
        "transfers": {"kind":"none"},
        "every_n_steps": 1
    }"#;
    assert!(serde_json::from_str::<CaptureConfig>(current_with_legacy).is_err());

    let legacy_with_current = r#"{
        "capture_nodes": [],
        "capture_metrics": [],
        "every_n_steps": 1,
        "include_step_zero": true,
        "include_final_state": true,
        "nodes": {"kind":"all"}
    }"#;
    assert!(serde_json::from_str::<CaptureConfig>(legacy_with_current).is_err());
}

#[test]
fn capture_wire_rejects_unknown_nested_schedule_and_selection_fields() {
    let every_with_legacy_flag = r#"{
        "schedule": {
            "kind":"every",
            "stride":2,
            "include_initial":true,
            "include_final":true,
            "include_step_zero":true
        },
        "nodes": {"kind":"all"},
        "metrics": {"kind":"all"},
        "variables": {"kind":"all"},
        "transfers": {"kind":"all"}
    }"#;
    assert!(serde_json::from_str::<CaptureConfig>(every_with_legacy_flag).is_err());

    let only_with_unknown_field = r#"{
        "schedule": {"kind":"none"},
        "nodes": {"kind":"only","items":["sink"],"unexpected":true},
        "metrics": {"kind":"all"},
        "variables": {"kind":"all"},
        "transfers": {"kind":"all"}
    }"#;
    assert!(serde_json::from_str::<CaptureConfig>(only_with_unknown_field).is_err());
}

#[test]
fn batch_aggregation_uses_a_canonical_wire_and_reads_legacy_capture() {
    let legacy = r#"{
        "max_steps": 12,
        "capture": {
            "capture_nodes": ["sink"],
            "capture_metrics": ["sink"],
            "every_n_steps": 3,
            "include_step_zero": false,
            "include_final_state": true
        }
    }"#;
    let template: BatchRunTemplate = serde_json::from_str(legacy).expect("legacy template decodes");
    assert_eq!(template.max_steps, 12);
    assert!(matches!(
        template.aggregation.schedule(),
        CaptureSchedule::Every { stride, include_initial: false, include_final: true }
            if stride.get() == 3
    ));
    assert_eq!(
        template.aggregation.metrics().only(),
        Some(&BTreeSet::from([MetricKey::fixture("sink")]))
    );

    let output = serde_json::to_value(&template).expect("canonical template encodes");
    assert!(output.get("capture").is_none());
    assert_eq!(output["aggregation"]["metrics"]["kind"], "only");

    let current = BatchRunTemplate::default().with_aggregation(AggregationConfig::none());
    let canonical = serde_json::to_value(&current).expect("current template encodes");
    assert_eq!(canonical["aggregation"]["schedule"]["kind"], "none");
}

#[test]
fn batch_aggregation_rejects_unknown_or_empty_metric_selection() {
    let empty_only = r#"{
        "max_steps": 12,
        "aggregation": {
            "schedule": {"kind":"none"},
            "metrics": {"kind":"only","items":[]}
        }
    }"#;
    assert!(serde_json::from_str::<BatchRunTemplate>(empty_only).is_err());

    let mixed = r#"{
        "max_steps": 12,
        "aggregation": {"schedule":{"kind":"none"},"metrics":{"kind":"none"}},
        "capture": {"capture_nodes":[],"capture_metrics":[],"every_n_steps":1,"include_step_zero":true,"include_final_state":true}
    }"#;
    assert!(serde_json::from_str::<BatchRunTemplate>(mixed).is_err());

    let compiled = compiled_scenario();
    let unknown_metric = MetricKey::fixture("missing");
    let config = BatchConfig::for_runs(2).with_max_steps(4).with_aggregation(
        AggregationConfig::default()
            .with_metrics(Selection::Only(BTreeSet::from([unknown_metric]))),
    );
    let error = Simulator::run_batch(&compiled, &config)
        .expect_err("unknown aggregation metric must fail before any seed runs");
    assert!(matches!(
        error,
        RunError::InvalidRunConfig { name, .. } if name == "batch.aggregation.metrics.missing"
    ));
}

#[test]
fn unknown_variable_and_transfer_selections_fail_before_execution() {
    let compiled = compiled_scenario();
    let unknown_variable = CaptureConfig::default()
        .with_variables(Selection::Only(BTreeSet::from(["missing".to_string()])));
    let variable_error =
        Simulator::run(&compiled, &RunConfig::for_seed(1).with_capture(unknown_variable))
            .expect_err("unknown variable must fail before execution");
    assert!(matches!(
        variable_error,
        RunError::InvalidRunConfig { name, .. } if name == "run.capture.variables.missing"
    ));

    let unknown_transfer = CaptureConfig::default()
        .with_transfers(Selection::Only(BTreeSet::from([EdgeId::fixture("missing")])));
    let transfer_error =
        Simulator::run(&compiled, &RunConfig::for_seed(1).with_capture(unknown_transfer))
            .expect_err("unknown transfer must fail before execution");
    assert!(matches!(
        transfer_error,
        RunError::InvalidRunConfig { name, .. } if name == "run.capture.transfers.missing"
    ));
}
