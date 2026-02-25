use std::collections::BTreeMap;
use std::fs;

use anapao::artifact::{
    read_manifest_compat, read_manifest_compat_from_slice, write_batch_artifacts,
    write_run_artifacts,
};
use anapao::events::{RunEvent, StepEndEvent, StepStartEvent};
use anapao::types::{
    ArtifactKind, BatchReport, BatchRunSummary, ExecutionMode, MetricKey, RunReport, ScenarioId,
    SeriesPoint, SeriesTable, ARTIFACT_SCHEMA_VERSION_V2,
};
use tempfile::tempdir;

#[test]
fn artifact_schema_v2_write_artifacts_emit_manifest_sections() {
    let tempdir = tempdir().expect("tempdir");

    let mut run_report = RunReport::new(ScenarioId::fixture("schema-v2-run"), 7);
    run_report.series.insert(
        MetricKey::fixture("alpha"),
        SeriesTable { metric: MetricKey::fixture("alpha"), points: vec![SeriesPoint::new(1, 1.0)] },
    );

    let events = vec![
        RunEvent::step_end("run-a", 1, 1, StepEndEvent { completed: true }),
        RunEvent::step_start("run-a", 1, 0, StepStartEvent { seed: 7 }),
    ];

    let run_manifest =
        write_run_artifacts(tempdir.path(), &run_report, &events).expect("write run artifacts");
    assert_eq!(run_manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    assert_eq!(
        run_manifest.sections.debug.as_ref().map(|section| section.artifact_key.as_str()),
        Some("events")
    );
    assert_eq!(
        run_manifest.sections.history.as_ref().map(|section| section.artifact_key.as_str()),
        Some("history")
    );
    assert_eq!(
        run_manifest.sections.replay.as_ref().map(|section| section.artifact_key.as_str()),
        Some("replay")
    );

    let mut batch_report =
        BatchReport::new(ScenarioId::fixture("schema-v2-batch"), 1, ExecutionMode::SingleThread);
    batch_report.runs = vec![BatchRunSummary {
        run_index: 1,
        seed: 1,
        completed: true,
        steps_executed: 2,
        final_metrics: BTreeMap::from([(MetricKey::fixture("alpha"), 10.0)]),
        manifest: None,
    }];
    batch_report.completed_runs = 1;

    let batch_manifest =
        write_batch_artifacts(tempdir.path(), &batch_report).expect("write batch artifacts");
    assert_eq!(batch_manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    assert_eq!(
        batch_manifest.sections.accuracy.as_ref().map(|section| section.artifact_key.as_str()),
        Some("prediction")
    );

    let manifest_path = tempdir.path().join("manifest.json");
    let manifest_from_disk = read_manifest_compat(&manifest_path).expect("read manifest");
    assert_eq!(manifest_from_disk.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    assert!(manifest_from_disk.sections.accuracy.is_some());

    let raw_manifest =
        serde_json::from_slice::<serde_json::Value>(&fs::read(&manifest_path).expect("read file"))
            .expect("parse manifest json");
    assert_eq!(raw_manifest["schema_version"], serde_json::json!(2));
    assert!(raw_manifest["sections"].is_object());
}

#[test]
fn artifact_schema_v2_compat_reader_upgrades_v1_shape_with_inferred_sections() {
    let raw_v1 = r#"{
        "scenario_id":"legacy-scenario",
        "artifacts":{
            "events":{"kind":"event_log","path":"events.jsonl","content_type":"application/x-ndjson"},
            "history":{"kind":"history_index","path":"history.json","content_type":"application/json"},
            "replay":{"kind":"replay_index","path":"replay.json","content_type":"application/json"},
            "summary":{"kind":"summary","path":"summary.csv","content_type":"text/csv"}
        }
    }"#;

    let manifest = read_manifest_compat_from_slice(raw_v1.as_bytes()).expect("compat read");
    assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    assert_eq!(manifest.scenario_id, ScenarioId::fixture("legacy-scenario"));

    let accuracy = manifest.sections.accuracy.expect("accuracy section");
    assert_eq!(accuracy.artifact_key, "summary");
    assert_eq!(accuracy.kind, ArtifactKind::Summary);

    let debug = manifest.sections.debug.expect("debug section");
    assert_eq!(debug.artifact_key, "events");
    assert_eq!(debug.kind, ArtifactKind::EventLog);

    let history = manifest.sections.history.expect("history section");
    assert_eq!(history.artifact_key, "history");
    assert_eq!(history.kind, ArtifactKind::HistoryIndex);

    let replay = manifest.sections.replay.expect("replay section");
    assert_eq!(replay.artifact_key, "replay");
    assert_eq!(replay.kind, ArtifactKind::ReplayIndex);
}
