use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde::Serialize;

use crate::assertions::AssertionReport;
use crate::error::ArtifactError;
use crate::events::{sort_events_by_order, RunEvent};
use crate::stats::prediction_indicators_by_metric;
use crate::types::{
    ArtifactKind, ArtifactRef, BatchReport, HistoryIndexEntry, HistoryIndexReport, ManifestRef,
    MetricKey, PredictionSummaryReport, ReplayIndexReport, ReplayRunIndex, RunReport,
    VariableSnapshot,
};

const MANIFEST_FILE: &str = "manifest.json";
const EVENTS_FILE: &str = "events.jsonl";
const VARIABLES_FILE: &str = "variables.csv";
const ASSERTIONS_FILE: &str = "assertions.json";
const HISTORY_FILE: &str = "history.json";
const REPLAY_FILE: &str = "replay.json";
const SERIES_FILE: &str = "series.csv";
const SUMMARY_FILE: &str = "summary.csv";
const PREDICTION_FILE: &str = "prediction.json";

const CONTENT_TYPE_JSON: &str = "application/json";
const CONTENT_TYPE_JSONL: &str = "application/x-ndjson";
const CONTENT_TYPE_CSV: &str = "text/csv";

pub fn write_run_artifacts(
    output_dir: impl AsRef<Path>,
    run_report: &RunReport,
    events: &[RunEvent],
) -> Result<ManifestRef, ArtifactError> {
    write_run_artifacts_with_assertions(output_dir, run_report, events, None)
}

/// Persist a run artifact pack and return its manifest.
///
/// Output includes deterministic JSON/CSV files and a typed `manifest.json`
/// entrypoint. When `assertion_report` is provided, `assertions.json` is also written.
///
/// # Full playbook (setup -> run -> assert -> artifacts)
/// ```no_run
/// use anapao::{Simulator, testkit};
/// use anapao::artifact::write_run_artifacts_with_assertions;
/// use anapao::assertions::{Expectation, MetricSelector};
/// use anapao::events::VecEventSink;
/// use anapao::types::MetricKey;
///
/// let compiled = Simulator::compile(testkit::fixture_scenario()).unwrap();
/// let run_config = testkit::deterministic_run_config();
/// let expectations = vec![Expectation::Equals {
///     metric: MetricKey::fixture("sink"),
///     selector: MetricSelector::Final,
///     expected: 3.0,
/// }];
///
/// let mut sink = VecEventSink::new();
/// let (report, assertion_report) = Simulator::run_with_assertions(
///     &compiled,
///     run_config,
///     &expectations,
///     Some(&mut sink),
/// )
/// .unwrap();
///
/// let output_dir = std::env::temp_dir().join("anapao-doc-playbook");
/// let manifest = write_run_artifacts_with_assertions(
///     &output_dir,
///     &report,
///     sink.events(),
///     Some(&assertion_report),
/// )
/// .unwrap();
///
/// assert!(manifest.artifacts.contains_key("manifest"));
/// assert!(manifest.artifacts.contains_key("events"));
/// assert!(manifest.artifacts.contains_key("assertions"));
/// ```
pub fn write_run_artifacts_with_assertions(
    output_dir: impl AsRef<Path>,
    run_report: &RunReport,
    events: &[RunEvent],
    assertion_report: Option<&AssertionReport>,
) -> Result<ManifestRef, ArtifactError> {
    let output_dir = output_dir.as_ref();
    ensure_output_dir(output_dir)?;

    let ordered_events_storage = if events_are_sorted_by_order(events) {
        None
    } else {
        let mut copy = events.to_vec();
        sort_events_by_order(&mut copy);
        Some(copy)
    };
    let ordered_events = ordered_events_storage.as_deref().unwrap_or(events);

    write_events_jsonl(&output_dir.join(EVENTS_FILE), &ordered_events)?;
    write_variable_csv(&output_dir.join(VARIABLES_FILE), &run_report.variable_snapshots)?;
    let history = history_index_report(run_report, &ordered_events);
    write_history_json(&output_dir.join(HISTORY_FILE), &history)?;
    let replay = replay_index_report(run_report, &history);
    write_replay_json(&output_dir.join(REPLAY_FILE), &replay)?;
    write_series_csv(&output_dir.join(SERIES_FILE), &run_report.series)?;

    let mut manifest = ManifestRef::new(run_report.scenario_id.clone())
        .with_seed_strategy(run_seed_strategy(run_report))
        .with_generated_at_unix_seconds(run_report.seed)
        .with_artifact(
            "events",
            artifact_ref(ArtifactKind::EventLog, EVENTS_FILE, CONTENT_TYPE_JSONL),
        )
        .with_artifact(
            "variables",
            artifact_ref(ArtifactKind::VariableSeries, VARIABLES_FILE, CONTENT_TYPE_CSV),
        )
        .with_artifact(
            "history",
            artifact_ref(ArtifactKind::HistoryIndex, HISTORY_FILE, CONTENT_TYPE_JSON),
        )
        .with_artifact(
            "manifest",
            artifact_ref(ArtifactKind::Manifest, MANIFEST_FILE, CONTENT_TYPE_JSON),
        )
        .with_artifact(
            "replay",
            artifact_ref(ArtifactKind::ReplayIndex, REPLAY_FILE, CONTENT_TYPE_JSON),
        )
        .with_artifact("series", artifact_ref(ArtifactKind::Series, SERIES_FILE, CONTENT_TYPE_CSV));

    if let Some(assertion_report) = assertion_report {
        write_assertions_json(&output_dir.join(ASSERTIONS_FILE), assertion_report)?;
        manifest = manifest.with_artifact(
            "assertions",
            artifact_ref(ArtifactKind::AssertionReport, ASSERTIONS_FILE, CONTENT_TYPE_JSON),
        );
    }

    let setup_hash =
        manifest_setup_hash(&manifest.scenario_id, &manifest.seed_strategy, &manifest.artifacts);
    manifest = manifest.with_setup_hash(setup_hash).with_inferred_sections();
    write_manifest_json(&output_dir.join(MANIFEST_FILE), &manifest)?;

    Ok(manifest)
}

pub fn write_batch_artifacts(
    output_dir: impl AsRef<Path>,
    batch_report: &BatchReport,
) -> Result<ManifestRef, ArtifactError> {
    let output_dir = output_dir.as_ref();
    ensure_output_dir(output_dir)?;

    write_series_csv(&output_dir.join(SERIES_FILE), &batch_report.aggregate_series)?;
    let prediction = prediction_summary_report(batch_report);
    write_summary_csv(&output_dir.join(SUMMARY_FILE), &prediction)?;
    write_prediction_json(&output_dir.join(PREDICTION_FILE), &prediction)?;

    let mut manifest = ManifestRef::new(batch_report.scenario_id.clone())
        .with_seed_strategy(batch_seed_strategy(batch_report))
        .with_generated_at_unix_seconds(batch_generated_at_unix_seconds(batch_report))
        .with_artifact(
            "manifest",
            artifact_ref(ArtifactKind::Manifest, MANIFEST_FILE, CONTENT_TYPE_JSON),
        )
        .with_artifact(
            "prediction",
            artifact_ref(ArtifactKind::Prediction, PREDICTION_FILE, CONTENT_TYPE_JSON),
        )
        .with_artifact("series", artifact_ref(ArtifactKind::Series, SERIES_FILE, CONTENT_TYPE_CSV))
        .with_artifact(
            "summary",
            artifact_ref(ArtifactKind::Summary, SUMMARY_FILE, CONTENT_TYPE_CSV),
        );
    let setup_hash =
        manifest_setup_hash(&manifest.scenario_id, &manifest.seed_strategy, &manifest.artifacts);
    manifest = manifest.with_setup_hash(setup_hash).with_inferred_sections();
    write_manifest_json(&output_dir.join(MANIFEST_FILE), &manifest)?;

    Ok(manifest)
}

fn artifact_ref(kind: ArtifactKind, path: &str, content_type: &str) -> ArtifactRef {
    ArtifactRef { kind, path: path.to_string(), content_type: Some(content_type.to_string()) }
}

fn ensure_output_dir(output_dir: &Path) -> Result<(), ArtifactError> {
    fs::create_dir_all(output_dir)
        .map_err(|source| ArtifactError::io(path_to_string(output_dir), source))
}

fn write_manifest_json(path: &Path, manifest: &ManifestRef) -> Result<(), ArtifactError> {
    write_pretty_json(path, MANIFEST_FILE, manifest)
}

pub fn read_manifest_compat(path: impl AsRef<Path>) -> Result<ManifestRef, ArtifactError> {
    let path = path.as_ref();
    let bytes = fs::read(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    read_manifest_compat_from_slice(&bytes)
}

pub fn read_manifest_compat_from_slice(bytes: &[u8]) -> Result<ManifestRef, ArtifactError> {
    let manifest = serde_json::from_slice::<ManifestRef>(bytes)
        .map_err(|source| ArtifactError::serialization(MANIFEST_FILE, source))?;
    Ok(manifest.upgrade_compat())
}

fn write_prediction_json(
    path: &Path,
    prediction: &PredictionSummaryReport,
) -> Result<(), ArtifactError> {
    write_pretty_json(path, PREDICTION_FILE, prediction)
}

fn write_assertions_json(
    path: &Path,
    assertion_report: &AssertionReport,
) -> Result<(), ArtifactError> {
    write_pretty_json(path, ASSERTIONS_FILE, assertion_report)
}

fn write_history_json(path: &Path, history: &HistoryIndexReport) -> Result<(), ArtifactError> {
    write_pretty_json(path, HISTORY_FILE, history)
}

fn write_replay_json(path: &Path, replay: &ReplayIndexReport) -> Result<(), ArtifactError> {
    write_pretty_json(path, REPLAY_FILE, replay)
}

fn write_pretty_json<T: Serialize>(
    path: &Path,
    context: &str,
    value: &T,
) -> Result<(), ArtifactError> {
    let file =
        File::create(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, value)
        .map_err(|source| ArtifactError::serialization(context, source))?;
    writer.write_all(b"\n").map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn write_events_jsonl(path: &Path, events: &[RunEvent]) -> Result<(), ArtifactError> {
    let file =
        File::create(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    let mut writer = BufWriter::new(file);
    for event in events {
        serde_json::to_writer(&mut writer, &event)
            .map_err(|source| ArtifactError::serialization(EVENTS_FILE, source))?;
        writer
            .write_all(b"\n")
            .map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    }

    writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn write_series_csv(
    path: &Path,
    series: &BTreeMap<MetricKey, crate::types::SeriesTable>,
) -> Result<(), ArtifactError> {
    let file =
        File::create(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    let mut writer = BufWriter::new(file);
    write_csv_row(&mut writer, path, &["metric", "step", "value"])?;

    for (metric, table) in series {
        if series_points_are_sorted(&table.points) {
            for point in &table.points {
                let step = point.step.to_string();
                let value = format_f64(point.value);
                write_csv_row(
                    &mut writer,
                    path,
                    &[metric.as_str(), step.as_str(), value.as_str()],
                )?;
            }
            continue;
        }

        let mut order = (0..table.points.len()).collect::<Vec<_>>();
        order.sort_by(|left, right| {
            let left_point = &table.points[*left];
            let right_point = &table.points[*right];
            left_point
                .step
                .cmp(&right_point.step)
                .then_with(|| left_point.value.total_cmp(&right_point.value))
        });

        for index in order {
            let point = &table.points[index];
            let step = point.step.to_string();
            let value = format_f64(point.value);
            write_csv_row(&mut writer, path, &[metric.as_str(), step.as_str(), value.as_str()])?;
        }
    }

    writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn write_variable_csv(path: &Path, snapshots: &[VariableSnapshot]) -> Result<(), ArtifactError> {
    let file =
        File::create(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    let mut writer = BufWriter::new(file);
    write_csv_row(&mut writer, path, &["variable", "step", "value"])?;

    if variable_snapshots_are_sorted(snapshots) {
        for snapshot in snapshots {
            let step = snapshot.step.to_string();
            for (name, value) in &snapshot.values {
                let value = format_f64(*value);
                write_csv_row(&mut writer, path, &[name.as_str(), step.as_str(), value.as_str()])?;
            }
        }
        return writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source));
    }

    let mut ordered_indices = (0..snapshots.len()).collect::<Vec<_>>();
    ordered_indices.sort_by_key(|index| snapshots[*index].step);
    for index in ordered_indices {
        let snapshot = &snapshots[index];
        for (name, value) in &snapshot.values {
            let step = snapshot.step.to_string();
            let value = format_f64(*value);
            write_csv_row(&mut writer, path, &[name.as_str(), step.as_str(), value.as_str()])?;
        }
    }

    writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn write_summary_csv(
    path: &Path,
    prediction_report: &PredictionSummaryReport,
) -> Result<(), ArtifactError> {
    let file =
        File::create(path).map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    let mut writer = BufWriter::new(file);
    write_csv_row(
        &mut writer,
        path,
        &[
            "metric",
            "n",
            "mean",
            "variance",
            "std_dev",
            "min",
            "max",
            "median",
            "p90",
            "p95",
            "p99",
            "ci95_lower",
            "ci95_upper",
            "ci95_margin",
            "reliability_score",
            "convergence_delta",
            "convergence_ratio",
        ],
    )?;

    for (metric, summary) in &prediction_report.metrics {
        let n = summary.n.to_string();
        let mean = format_f64(summary.mean);
        let variance = format_f64(summary.variance);
        let std_dev = format_f64(summary.std_dev);
        let min = format_f64(summary.min);
        let max = format_f64(summary.max);
        let median = format_f64(summary.median);
        let p90 = format_f64(summary.p90);
        let p95 = format_f64(summary.p95);
        let p99 = format_f64(summary.p99);
        let ci95_lower = format_f64(summary.confidence_lower_95);
        let ci95_upper = format_f64(summary.confidence_upper_95);
        let ci95_margin = format_f64(summary.confidence_margin_95);
        let reliability_score = format_f64(summary.reliability_score);
        let convergence_delta = format_f64(summary.convergence_delta);
        let convergence_ratio = format_f64(summary.convergence_ratio);

        write_csv_row(
            &mut writer,
            path,
            &[
                metric.as_str(),
                n.as_str(),
                mean.as_str(),
                variance.as_str(),
                std_dev.as_str(),
                min.as_str(),
                max.as_str(),
                median.as_str(),
                p90.as_str(),
                p95.as_str(),
                p99.as_str(),
                ci95_lower.as_str(),
                ci95_upper.as_str(),
                ci95_margin.as_str(),
                reliability_score.as_str(),
                convergence_delta.as_str(),
                convergence_ratio.as_str(),
            ],
        )?;
    }

    writer.flush().map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn write_csv_row(
    writer: &mut impl Write,
    path: &Path,
    fields: &[&str],
) -> Result<(), ArtifactError> {
    for (index, field) in fields.iter().enumerate() {
        if index > 0 {
            writer
                .write_all(b",")
                .map_err(|source| ArtifactError::io(path_to_string(path), source))?;
        }

        let encoded = encode_csv_field(field);
        writer
            .write_all(encoded.as_bytes())
            .map_err(|source| ArtifactError::io(path_to_string(path), source))?;
    }

    writer.write_all(b"\n").map_err(|source| ArtifactError::io(path_to_string(path), source))
}

fn encode_csv_field(value: &str) -> String {
    if value.bytes().any(|byte| matches!(byte, b',' | b'"' | b'\n' | b'\r')) {
        let escaped = value.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        value.to_string()
    }
}

fn format_f64(value: f64) -> String {
    let mut buffer = ryu::Buffer::new();
    buffer.format(value).to_owned()
}

fn run_seed_strategy(run_report: &RunReport) -> String {
    format!("single(seed={})", run_report.seed)
}

fn batch_seed_strategy(batch_report: &BatchReport) -> String {
    let first = batch_report
        .runs
        .iter()
        .min_by_key(|run| run.run_index)
        .map(|run| run.seed)
        .unwrap_or_default();
    let last = batch_report
        .runs
        .iter()
        .max_by_key(|run| run.run_index)
        .map(|run| run.seed)
        .unwrap_or_default();
    format!(
        "derived_splitmix64(runs={},first_seed={},last_seed={})",
        batch_report.runs.len(),
        first,
        last
    )
}

fn batch_generated_at_unix_seconds(batch_report: &BatchReport) -> u64 {
    batch_report.runs.iter().min_by_key(|run| run.run_index).map(|run| run.seed).unwrap_or_default()
}

fn manifest_setup_hash(
    scenario_id: &crate::types::ScenarioId,
    seed_strategy: &str,
    artifacts: &BTreeMap<String, ArtifactRef>,
) -> String {
    #[derive(Serialize)]
    struct Payload<'a> {
        scenario_id: &'a str,
        seed_strategy: &'a str,
        artifacts: &'a BTreeMap<String, ArtifactRef>,
    }

    let payload = Payload { scenario_id: scenario_id.as_str(), seed_strategy, artifacts };
    let bytes = serde_json::to_vec(&payload).unwrap_or_default();
    let hash = stable_fnv1a_64(&bytes);
    format!("{hash:016x}")
}

fn stable_fnv1a_64(bytes: &[u8]) -> u64 {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET_BASIS;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}

fn path_to_string(path: &Path) -> String {
    path.display().to_string()
}

fn events_are_sorted_by_order(events: &[RunEvent]) -> bool {
    events.windows(2).all(|window| window[0].order() <= window[1].order())
}

fn series_points_are_sorted(points: &[crate::types::SeriesPoint]) -> bool {
    points.windows(2).all(|window| {
        window[0]
            .step
            .cmp(&window[1].step)
            .then_with(|| window[0].value.total_cmp(&window[1].value))
            .is_le()
    })
}

fn variable_snapshots_are_sorted(snapshots: &[VariableSnapshot]) -> bool {
    snapshots.windows(2).all(|window| window[0].step <= window[1].step)
}

fn history_index_report(run_report: &RunReport, ordered_events: &[RunEvent]) -> HistoryIndexReport {
    let entries = ordered_events
        .iter()
        .enumerate()
        .map(|(event_index, event)| {
            let order = event.order();
            let (severity, code) = event
                .diagnostic_marker()
                .map(|(severity, code)| (Some(severity), code.map(str::to_owned)))
                .unwrap_or((None, None));

            HistoryIndexEntry {
                event_index: event_index as u64,
                run_id: order.run_id.clone(),
                step: order.step,
                phase: order.phase.as_str().to_string(),
                ordinal: order.ordinal,
                event: event.event_name().to_string(),
                severity,
                code,
            }
        })
        .collect::<Vec<_>>();

    HistoryIndexReport {
        scenario_id: run_report.scenario_id.clone(),
        source: EVENTS_FILE.to_string(),
        entries,
    }
}

fn replay_index_report(run_report: &RunReport, history: &HistoryIndexReport) -> ReplayIndexReport {
    let mut report = ReplayIndexReport::new(
        run_report.scenario_id.clone(),
        EVENTS_FILE,
        history.entries.len() as u64,
    );

    for entry in &history.entries {
        report
            .runs
            .entry(entry.run_id.clone())
            .and_modify(|existing: &mut ReplayRunIndex| {
                existing.last_event_index = entry.event_index;
                existing.last_step = entry.step;
            })
            .or_insert(ReplayRunIndex {
                first_event_index: entry.event_index,
                last_event_index: entry.event_index,
                first_step: entry.step,
                last_step: entry.step,
            });
    }

    report
}

fn prediction_summary_report(batch_report: &BatchReport) -> PredictionSummaryReport {
    let mut ordered_runs = batch_report.runs.iter().collect::<Vec<_>>();
    ordered_runs.sort_by(|left, right| {
        left.run_index.cmp(&right.run_index).then_with(|| left.seed.cmp(&right.seed))
    });

    let mut values_by_metric: BTreeMap<MetricKey, Vec<f64>> = BTreeMap::new();
    for run in ordered_runs {
        for (metric, value) in &run.final_metrics {
            values_by_metric.entry(metric.clone()).or_default().push(*value);
        }
    }

    PredictionSummaryReport::new(
        batch_report.scenario_id.clone(),
        prediction_indicators_by_metric(values_by_metric),
    )
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;
    use std::path::Path;

    use tempfile::tempdir;

    use crate::assertions::AssertionReport;
    use crate::error::ArtifactError;
    use crate::events::{
        DebugEvent, NodeUpdateEvent, RunEvent, StepEndEvent, StepStartEvent, ViolationEvent,
    };
    use crate::types::{
        BatchReport, BatchRunSummary, DiagnosticSeverity, ExecutionMode, HistoryIndexReport,
        MetricKey, NodeId, PredictionSummaryReport, ReplayIndexReport, RunReport, ScenarioId,
        SeriesPoint, SeriesTable, ARTIFACT_SCHEMA_VERSION_V2,
    };

    use super::{
        batch_generated_at_unix_seconds, batch_seed_strategy, encode_csv_field, format_f64,
        manifest_setup_hash, path_to_string, read_manifest_compat, read_manifest_compat_from_slice,
        run_seed_strategy, stable_fnv1a_64, write_batch_artifacts, write_run_artifacts,
        write_run_artifacts_with_assertions, ASSERTIONS_FILE, EVENTS_FILE, HISTORY_FILE,
        MANIFEST_FILE, PREDICTION_FILE, REPLAY_FILE, SERIES_FILE, SUMMARY_FILE, VARIABLES_FILE,
    };

    #[test]
    fn write_run_artifacts_persists_manifest_events_and_series() {
        let tempdir = tempdir().expect("tempdir");
        let mut run_report = RunReport::new(ScenarioId::fixture("scenario-run"), 7);

        run_report.series.insert(
            MetricKey::fixture("beta"),
            SeriesTable {
                metric: MetricKey::fixture("beta"),
                points: vec![SeriesPoint::new(3, 9.0), SeriesPoint::new(1, 3.0)],
            },
        );
        run_report.series.insert(
            MetricKey::fixture("alpha"),
            SeriesTable {
                metric: MetricKey::fixture("alpha"),
                points: vec![SeriesPoint::new(2, 2.0)],
            },
        );

        let events = vec![
            RunEvent::violation(
                "run-1",
                1,
                8,
                ViolationEvent {
                    severity: DiagnosticSeverity::Warning,
                    code: "FLOW-001".to_string(),
                    message: "insufficient input".to_string(),
                    evidence: BTreeMap::from([("edge_id".to_string(), "edge-a".to_string())]),
                },
            ),
            RunEvent::step_end("run-1", 1, 9, StepEndEvent { completed: true }),
            RunEvent::debug(
                "run-1",
                1,
                7,
                DebugEvent {
                    topic: "engine".to_string(),
                    message: "step finalized".to_string(),
                    fields: BTreeMap::from([("run".to_string(), "run-1".to_string())]),
                },
            ),
            RunEvent::node_update(
                "run-1",
                1,
                1,
                NodeUpdateEvent {
                    node_id: NodeId::fixture("node-a"),
                    previous_value: 1.0,
                    next_value: 2.0,
                },
            ),
            RunEvent::step_start("run-1", 1, 0, StepStartEvent { seed: 7 }),
        ];

        let manifest =
            write_run_artifacts(tempdir.path(), &run_report, &events).expect("write run artifacts");

        let keys = manifest.artifacts.keys().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(keys, vec!["events", "history", "manifest", "replay", "series", "variables"]);
        assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
        assert_eq!(manifest.seed_strategy, "single(seed=7)");
        assert_eq!(manifest.generated_at_unix_seconds, 7);
        assert!(!manifest.setup_hash.is_empty());
        assert_eq!(
            manifest.sections.debug.as_ref().map(|section| section.artifact_key.as_str()),
            Some("events")
        );
        assert_eq!(
            manifest.sections.history.as_ref().map(|section| section.artifact_key.as_str()),
            Some("history")
        );
        assert_eq!(
            manifest.sections.replay.as_ref().map(|section| section.artifact_key.as_str()),
            Some("replay")
        );

        let manifest_path = tempdir.path().join(MANIFEST_FILE);
        let events_path = tempdir.path().join(EVENTS_FILE);
        let history_path = tempdir.path().join(HISTORY_FILE);
        let replay_path = tempdir.path().join(REPLAY_FILE);
        let series_path = tempdir.path().join(SERIES_FILE);
        let variables_path = tempdir.path().join(VARIABLES_FILE);
        assert!(manifest_path.is_file());
        assert!(events_path.is_file());
        assert!(history_path.is_file());
        assert!(replay_path.is_file());
        assert!(series_path.is_file());
        assert!(variables_path.is_file());

        let disk_manifest = serde_json::from_slice::<crate::types::ManifestRef>(
            &fs::read(&manifest_path).expect("read manifest"),
        )
        .expect("parse manifest");
        assert_eq!(disk_manifest, manifest);

        let event_lines = fs::read_to_string(&events_path)
            .expect("read events")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(event_lines.len(), 5);
        assert!(event_lines[0].contains("\"event\":\"step_start\""));
        assert!(event_lines[1].contains("\"event\":\"node_update\""));
        assert!(event_lines[2].contains("\"event\":\"step_end\""));
        assert!(event_lines[3].contains("\"event\":\"debug\""));
        assert!(event_lines[4].contains("\"event\":\"violation\""));

        let history = serde_json::from_slice::<HistoryIndexReport>(
            &fs::read(&history_path).expect("read history"),
        )
        .expect("parse history");
        assert_eq!(history.source, EVENTS_FILE);
        assert_eq!(history.entries.len(), 5);
        assert_eq!(history.entries[0].event_index, 0);
        assert_eq!(history.entries[0].event, "step_start");
        assert_eq!(history.entries[4].event, "violation");
        assert_eq!(history.entries[4].severity, Some(DiagnosticSeverity::Warning));
        assert_eq!(history.entries[4].code.as_deref(), Some("FLOW-001"));

        let replay = serde_json::from_slice::<ReplayIndexReport>(
            &fs::read(&replay_path).expect("read replay"),
        )
        .expect("parse replay");
        assert_eq!(replay.source, EVENTS_FILE);
        assert_eq!(replay.event_count, 5);
        assert_eq!(replay.runs.len(), 1);
        let run_bounds = replay.runs.get("run-1").expect("run-1 replay bounds");
        assert_eq!(run_bounds.first_event_index, 0);
        assert_eq!(run_bounds.last_event_index, 4);
        assert_eq!(run_bounds.first_step, 1);
        assert_eq!(run_bounds.last_step, 1);

        let series_rows = fs::read_to_string(&series_path)
            .expect("read series")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(series_rows[0], "metric,step,value");
        assert_eq!(series_rows[1], "alpha,2,2.0");
        assert_eq!(series_rows[2], "beta,1,3.0");
        assert_eq!(series_rows[3], "beta,3,9.0");

        let variable_rows = fs::read_to_string(&variables_path)
            .expect("read variables")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(variable_rows, vec!["variable,step,value"]);
    }

    #[test]
    fn write_batch_artifacts_persists_manifest_series_and_summary() {
        let tempdir = tempdir().expect("tempdir");
        let mut batch_report =
            BatchReport::new(ScenarioId::fixture("scenario-batch"), 2, ExecutionMode::SingleThread);

        batch_report.runs = vec![
            BatchRunSummary {
                run_index: 2,
                seed: 2,
                completed: true,
                steps_executed: 3,
                final_metrics: BTreeMap::from([
                    (MetricKey::fixture("alpha"), 20.0),
                    (MetricKey::fixture("beta"), 5.0),
                ]),
                manifest: None,
            },
            BatchRunSummary {
                run_index: 1,
                seed: 1,
                completed: true,
                steps_executed: 3,
                final_metrics: BTreeMap::from([
                    (MetricKey::fixture("alpha"), 10.0),
                    (MetricKey::fixture("beta"), 3.0),
                ]),
                manifest: None,
            },
        ];
        batch_report.completed_runs = batch_report.runs.len() as u64;
        batch_report.aggregate_series.insert(
            MetricKey::fixture("beta"),
            SeriesTable {
                metric: MetricKey::fixture("beta"),
                points: vec![SeriesPoint::new(2, 5.0), SeriesPoint::new(1, 4.0)],
            },
        );
        batch_report.aggregate_series.insert(
            MetricKey::fixture("alpha"),
            SeriesTable {
                metric: MetricKey::fixture("alpha"),
                points: vec![SeriesPoint::new(1, 1.0)],
            },
        );

        let manifest =
            write_batch_artifacts(tempdir.path(), &batch_report).expect("write batch artifacts");

        let keys = manifest.artifacts.keys().map(String::as_str).collect::<Vec<_>>();
        assert_eq!(keys, vec!["manifest", "prediction", "series", "summary"]);
        assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
        assert!(manifest.seed_strategy.starts_with("derived_splitmix64("));
        assert_eq!(manifest.generated_at_unix_seconds, 1);
        assert!(!manifest.setup_hash.is_empty());
        assert_eq!(
            manifest.sections.accuracy.as_ref().map(|section| section.artifact_key.as_str()),
            Some("prediction")
        );
        assert!(manifest.sections.debug.is_none());
        assert!(manifest.sections.history.is_none());

        let manifest_path = tempdir.path().join(MANIFEST_FILE);
        let prediction_path = tempdir.path().join(PREDICTION_FILE);
        let series_path = tempdir.path().join(SERIES_FILE);
        let summary_path = tempdir.path().join(SUMMARY_FILE);
        assert!(manifest_path.is_file());
        assert!(prediction_path.is_file());
        assert!(series_path.is_file());
        assert!(summary_path.is_file());

        let disk_manifest = serde_json::from_slice::<crate::types::ManifestRef>(
            &fs::read(&manifest_path).expect("read manifest"),
        )
        .expect("parse manifest");
        assert_eq!(disk_manifest, manifest);

        let prediction = serde_json::from_slice::<PredictionSummaryReport>(
            &fs::read(&prediction_path).expect("read prediction"),
        )
        .expect("parse prediction");
        let prediction_keys = prediction.metrics.keys().map(MetricKey::as_str).collect::<Vec<_>>();
        assert_eq!(prediction_keys, vec!["alpha", "beta"]);

        let alpha_prediction =
            prediction.metrics.get(&MetricKey::fixture("alpha")).expect("alpha prediction");
        assert_eq!(alpha_prediction.n, 2);
        assert_eq!(alpha_prediction.convergence_delta, 10.0);

        let series_rows = fs::read_to_string(&series_path)
            .expect("read series")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(series_rows[0], "metric,step,value");
        assert_eq!(series_rows[1], "alpha,1,1.0");
        assert_eq!(series_rows[2], "beta,1,4.0");
        assert_eq!(series_rows[3], "beta,2,5.0");

        let summary_rows = fs::read_to_string(&summary_path)
            .expect("read summary")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(
            summary_rows[0],
            "metric,n,mean,variance,std_dev,min,max,median,p90,p95,p99,ci95_lower,ci95_upper,ci95_margin,reliability_score,convergence_delta,convergence_ratio"
        );
        assert_eq!(summary_rows.len(), 3);

        let alpha_cols = summary_rows[1].split(',').collect::<Vec<_>>();
        let beta_cols = summary_rows[2].split(',').collect::<Vec<_>>();
        assert_eq!(alpha_cols[0], "alpha");
        assert_eq!(beta_cols[0], "beta");
        assert_eq!(alpha_cols[1], "2");
        assert_eq!(beta_cols[1], "2");
        assert_eq!(alpha_cols.len(), 17);
        assert_eq!(beta_cols.len(), 17);
        assert_eq!(alpha_cols[15], "10.0");
        assert_eq!(beta_cols[15], "2.0");
    }

    #[test]
    fn write_run_artifacts_optionally_persists_assertion_report() {
        let tempdir = tempdir().expect("tempdir");
        let run_report = RunReport::new(ScenarioId::fixture("scenario-run-assert"), 42);
        let events = vec![RunEvent::step_start("run-1", 0, 0, StepStartEvent { seed: 42 })];
        let assertions = AssertionReport { total: 0, passed: 0, failed: 0, results: vec![] };

        let manifest = write_run_artifacts_with_assertions(
            tempdir.path(),
            &run_report,
            &events,
            Some(&assertions),
        )
        .expect("write run artifacts with assertions");

        assert!(manifest.artifacts.contains_key("assertions"));
        assert_eq!(
            manifest.sections.assertions.as_ref().map(|section| section.artifact_key.as_str()),
            Some("assertions")
        );

        let assertions_path = tempdir.path().join(ASSERTIONS_FILE);
        assert!(assertions_path.is_file());
    }

    #[test]
    fn read_manifest_compat_supports_path_slice_and_errors() {
        let tempdir = tempdir().expect("tempdir");
        let manifest_path = tempdir.path().join(MANIFEST_FILE);
        let manifest_v1 = serde_json::json!({
            "schema_version": 1,
            "scenario_id": "scenario-compat",
            "seed_strategy": "",
            "setup_hash": "",
            "crate_version": "",
            "generated_at_unix_seconds": 12,
            "artifacts": {
                "prediction": {
                    "kind": "prediction",
                    "path": "prediction.json",
                    "content_type": "application/json"
                }
            }
        });
        fs::write(&manifest_path, serde_json::to_vec(&manifest_v1).expect("serialize manifest"))
            .expect("write manifest");

        let from_path = read_manifest_compat(&manifest_path).expect("read path");
        assert_eq!(from_path.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
        assert!(!from_path.seed_strategy.is_empty());
        assert!(!from_path.setup_hash.is_empty());
        assert_eq!(
            from_path.sections.accuracy.as_ref().map(|section| section.artifact_key.as_str()),
            Some("prediction")
        );

        let bytes = fs::read(&manifest_path).expect("read manifest bytes");
        let from_slice = read_manifest_compat_from_slice(&bytes).expect("read slice");
        assert_eq!(from_slice, from_path);

        let missing_err = read_manifest_compat(tempdir.path().join("missing-manifest.json"))
            .expect_err("missing path must fail");
        assert!(matches!(missing_err, ArtifactError::Io { .. }));

        let parse_err = read_manifest_compat_from_slice(b"{not-json").expect_err("must fail");
        assert!(matches!(
            parse_err,
            ArtifactError::Serialization { context, .. } if context == MANIFEST_FILE
        ));
    }

    #[test]
    fn write_artifacts_csv_escapes_special_characters_and_variable_rows() {
        let tempdir = tempdir().expect("tempdir");
        let metric = MetricKey::fixture("alpha,\"beta\"");
        let mut run_report = RunReport::new(ScenarioId::fixture("scenario-csv"), 99);
        run_report.series.insert(
            metric.clone(),
            SeriesTable { metric: metric.clone(), points: vec![SeriesPoint::new(1, 2.0)] },
        );
        run_report.variable_snapshots = vec![crate::types::VariableSnapshot {
            step: 3,
            values: BTreeMap::from([("var,\"name\"".to_string(), 4.5)]),
        }];
        let events = vec![RunEvent::step_start("run-1", 0, 0, StepStartEvent { seed: 99 })];

        let _manifest =
            write_run_artifacts(tempdir.path(), &run_report, &events).expect("write artifacts");

        let series_rows = fs::read_to_string(tempdir.path().join(SERIES_FILE))
            .expect("read series csv")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(series_rows[0], "metric,step,value");
        assert_eq!(series_rows[1], "\"alpha,\"\"beta\"\"\",1,2.0");

        let variable_rows = fs::read_to_string(tempdir.path().join(VARIABLES_FILE))
            .expect("read variable csv")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(variable_rows[0], "variable,step,value");
        assert_eq!(variable_rows[1], "\"var,\"\"name\"\"\",3,4.5");
    }

    #[test]
    fn write_batch_artifacts_handles_empty_runs_metadata() {
        let tempdir = tempdir().expect("tempdir");
        let batch_report = BatchReport::new(
            ScenarioId::fixture("scenario-empty-batch"),
            0,
            ExecutionMode::SingleThread,
        );

        let manifest =
            write_batch_artifacts(tempdir.path(), &batch_report).expect("write empty batch");
        assert_eq!(manifest.generated_at_unix_seconds, 0);
        assert_eq!(manifest.seed_strategy, "derived_splitmix64(runs=0,first_seed=0,last_seed=0)");

        let summary_rows = fs::read_to_string(tempdir.path().join(SUMMARY_FILE))
            .expect("read summary")
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>();
        assert_eq!(summary_rows.len(), 1);
    }

    #[test]
    fn helper_functions_are_deterministic_and_stable() {
        assert_eq!(encode_csv_field("plain"), "plain");
        assert_eq!(encode_csv_field("a,b"), "\"a,b\"");
        assert_eq!(encode_csv_field("a\"b"), "\"a\"\"b\"");
        assert_eq!(encode_csv_field("a\nb"), "\"a\nb\"");

        assert_eq!(format_f64(1.25), "1.25");

        let run_report = RunReport::new(ScenarioId::fixture("scenario-seed"), 77);
        assert_eq!(run_seed_strategy(&run_report), "single(seed=77)");

        let mut batch_report = BatchReport::new(
            ScenarioId::fixture("scenario-batch-seed"),
            2,
            ExecutionMode::SingleThread,
        );
        batch_report.runs = vec![
            BatchRunSummary {
                run_index: 9,
                seed: 900,
                completed: true,
                steps_executed: 1,
                final_metrics: BTreeMap::new(),
                manifest: None,
            },
            BatchRunSummary {
                run_index: 1,
                seed: 100,
                completed: true,
                steps_executed: 1,
                final_metrics: BTreeMap::new(),
                manifest: None,
            },
        ];

        assert_eq!(
            batch_seed_strategy(&batch_report),
            "derived_splitmix64(runs=2,first_seed=100,last_seed=900)"
        );
        assert_eq!(batch_generated_at_unix_seconds(&batch_report), 100);

        let artifacts = BTreeMap::from([(
            "manifest".to_string(),
            crate::types::ArtifactRef::new(crate::types::ArtifactKind::Manifest, "manifest.json"),
        )]);
        let hash_a = manifest_setup_hash(
            &ScenarioId::fixture("scenario-hash"),
            "single(seed=1)",
            &artifacts,
        );
        let hash_b = manifest_setup_hash(
            &ScenarioId::fixture("scenario-hash"),
            "single(seed=1)",
            &artifacts,
        );
        assert_eq!(hash_a, hash_b);

        assert_ne!(stable_fnv1a_64(b"a"), stable_fnv1a_64(b"b"));
        assert_eq!(path_to_string(Path::new("foo/bar")), "foo/bar");
    }
}
