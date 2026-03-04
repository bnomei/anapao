use anapao::artifact::write_run_artifacts_with_assertions;
use anapao::assertions::{Expectation, MetricSelector};
use anapao::events::VecEventSink;
use anapao::types::MetricKey;
use anapao::{testkit, Simulator};
use tempfile::tempdir;

#[test]
fn readme_playbook_end_to_end_artifacts_and_assertions() {
    let compiled =
        Simulator::compile(testkit::fixture_scenario()).expect("compile fixture scenario");
    let run_config = testkit::deterministic_run_config();
    let expectations = vec![Expectation::Equals {
        metric: MetricKey::fixture("sink"),
        selector: MetricSelector::Final,
        expected: 3.0,
    }];

    let mut sink = VecEventSink::new();
    let (run_report, assertion_report) =
        Simulator::run_with_assertions_and_sink(&compiled, &run_config, &expectations, &mut sink)
            .expect("run with assertions");

    assert!(run_report.completed);
    assert_eq!(run_report.steps_executed, 3);
    assert_eq!(run_report.final_metrics.get(&MetricKey::fixture("sink")), Some(&3.0));
    assert!(assertion_report.is_success());
    assert!(sink.events().iter().any(|event| event.event_name() == "assertion_checkpoint"));

    let rerun = Simulator::run(&compiled, &run_config).expect("rerun with same deterministic seed");
    assert_eq!(rerun.steps_executed, run_report.steps_executed);
    assert_eq!(rerun.final_metrics, run_report.final_metrics);

    let output = tempdir().expect("tempdir");
    let manifest = write_run_artifacts_with_assertions(
        output.path(),
        &run_report,
        sink.events(),
        Some(&assertion_report),
    )
    .expect("persist run artifacts");

    for key in ["manifest", "events", "assertions", "series", "variables"] {
        assert!(manifest.artifacts.contains_key(key), "manifest missing key {key}");
    }
    assert!(manifest.sections.assertions.is_some());
    assert!(manifest.sections.debug.is_some());
    assert!(manifest.sections.history.is_some());
    assert!(manifest.sections.replay.is_some());
}
