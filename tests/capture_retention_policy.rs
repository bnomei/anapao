use std::collections::BTreeSet;

use anapao::error::RunError;
use anapao::events::VecEventSink;
use anapao::types::{
    AggregationConfig, BatchRunTemplate, CaptureConfig, CaptureSchedule, EdgeId, EndConditionSpec,
    MetricKey, NodeId, RunConfig, ScenarioSpec, Selection, TransferSpec,
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
