use std::collections::BTreeMap;

use polars::prelude::{Column, DataFrame, PolarsResult};

use crate::types::{BatchReport, MetricKey, RunReport, SeriesPoint, SeriesTable};

/// Build a frame for single-run metric series.
///
/// Output schema:
/// - `metric`: metric key
/// - `step`: simulation step
/// - `value`: metric value at step
pub fn run_series_frame(run_report: &RunReport) -> PolarsResult<DataFrame> {
    series_frame(&run_report.series)
}

/// Build a frame for aggregate batch metric series.
///
/// Output schema:
/// - `metric`: metric key
/// - `step`: simulation step
/// - `value`: aggregate metric value at step
pub fn batch_series_frame(batch_report: &BatchReport) -> PolarsResult<DataFrame> {
    series_frame(&batch_report.aggregate_series)
}

/// Build a per-run final-metrics frame for batch summaries.
///
/// Output schema:
/// - `metric`: metric key
/// - `step`: run index, exposed as plotting step
/// - `value`: final metric value
/// - `run_index`: run index
pub fn batch_final_metrics_frame(batch_report: &BatchReport) -> PolarsResult<DataFrame> {
    let mut metrics = Vec::new();
    let mut steps = Vec::new();
    let mut values = Vec::new();
    let mut run_indexes = Vec::new();

    let mut ordered_runs = batch_report.runs.iter().collect::<Vec<_>>();
    ordered_runs.sort_by_key(|run| run.run_index);

    for run in ordered_runs {
        for (metric, value) in &run.final_metrics {
            metrics.push(metric.as_str().to_owned());
            steps.push(run.run_index);
            values.push(*value);
            run_indexes.push(run.run_index);
        }
    }

    let height = metrics.len();
    DataFrame::new(
        height,
        vec![
            Column::new("metric".into(), metrics),
            Column::new("step".into(), steps),
            Column::new("value".into(), values),
            Column::new("run_index".into(), run_indexes),
        ],
    )
}

fn series_frame(series: &BTreeMap<MetricKey, SeriesTable>) -> PolarsResult<DataFrame> {
    let mut metrics = Vec::new();
    let mut steps = Vec::new();
    let mut values = Vec::new();

    for (metric, table) in series {
        let mut points = table.points.clone();
        sort_points(&mut points);
        for point in points {
            metrics.push(metric.as_str().to_owned());
            steps.push(point.step);
            values.push(point.value);
        }
    }

    let height = metrics.len();
    DataFrame::new(
        height,
        vec![
            Column::new("metric".into(), metrics),
            Column::new("step".into(), steps),
            Column::new("value".into(), values),
        ],
    )
}

fn sort_points(points: &mut [SeriesPoint]) {
    points.sort_by(|left, right| {
        left.step.cmp(&right.step).then_with(|| left.value.total_cmp(&right.value))
    });
}

#[cfg(all(test, feature = "analysis-polars"))]
mod tests {
    use std::collections::BTreeMap;

    use polars::prelude::{AnyValue, DataFrame};

    use crate::types::{
        BatchReport, BatchRunSummary, ExecutionMode, MetricKey, RunReport, ScenarioId, SeriesPoint,
        SeriesTable,
    };

    use super::{batch_final_metrics_frame, batch_series_frame, run_series_frame};

    #[test]
    fn run_series_frame_builds_plotting_columns_in_stable_order() {
        let mut report = RunReport::new(ScenarioId::fixture("scenario"), 7);
        report.series.insert(
            MetricKey::fixture("beta"),
            SeriesTable {
                metric: MetricKey::fixture("beta"),
                points: vec![SeriesPoint::new(3, 9.0), SeriesPoint::new(1, 3.0)],
            },
        );
        report.series.insert(
            MetricKey::fixture("alpha"),
            SeriesTable {
                metric: MetricKey::fixture("alpha"),
                points: vec![SeriesPoint::new(2, 2.0), SeriesPoint::new(1, 1.0)],
            },
        );

        let frame = run_series_frame(&report).expect("frame");
        assert_eq!(frame.shape(), (4, 3));
        assert_eq!(frame.get_column_names(), vec!["metric", "step", "value"]);
        assert_eq!(
            frame_rows(&frame, &["metric", "step", "value"]),
            vec![
                vec!["alpha".to_string(), "1".to_string(), "1".to_string()],
                vec!["alpha".to_string(), "2".to_string(), "2".to_string()],
                vec!["beta".to_string(), "1".to_string(), "3".to_string()],
                vec!["beta".to_string(), "3".to_string(), "9".to_string()],
            ]
        );
    }

    #[test]
    fn batch_series_frame_builds_plotting_columns() {
        let mut report =
            BatchReport::new(ScenarioId::fixture("batch"), 2, ExecutionMode::SingleThread);
        report.aggregate_series.insert(
            MetricKey::fixture("latency"),
            SeriesTable {
                metric: MetricKey::fixture("latency"),
                points: vec![SeriesPoint::new(2, 1.8), SeriesPoint::new(1, 2.0)],
            },
        );

        let frame = batch_series_frame(&report).expect("frame");
        assert_eq!(frame.shape(), (2, 3));
        assert_eq!(frame.get_column_names(), vec!["metric", "step", "value"]);
        assert_eq!(
            frame_rows(&frame, &["metric", "step", "value"]),
            vec![
                vec!["latency".to_string(), "1".to_string(), "2".to_string()],
                vec!["latency".to_string(), "2".to_string(), "1.8".to_string()],
            ]
        );
    }

    #[test]
    fn batch_final_metrics_frame_exposes_run_index_for_batch_summaries() {
        let mut report =
            BatchReport::new(ScenarioId::fixture("batch"), 2, ExecutionMode::SingleThread);
        report.runs = vec![
            BatchRunSummary {
                run_index: 9,
                seed: 9,
                completed: true,
                steps_executed: 10,
                final_metrics: BTreeMap::from([
                    (MetricKey::fixture("alpha"), 3.0),
                    (MetricKey::fixture("beta"), 6.0),
                ]),
                manifest: None,
            },
            BatchRunSummary {
                run_index: 2,
                seed: 2,
                completed: true,
                steps_executed: 10,
                final_metrics: BTreeMap::from([
                    (MetricKey::fixture("alpha"), 1.0),
                    (MetricKey::fixture("beta"), 2.0),
                ]),
                manifest: None,
            },
        ];

        let frame = batch_final_metrics_frame(&report).expect("frame");
        assert_eq!(frame.shape(), (4, 4));
        assert_eq!(frame.get_column_names(), vec!["metric", "step", "value", "run_index"]);
        assert_eq!(
            frame_rows(&frame, &["metric", "step", "value", "run_index"]),
            vec![
                vec!["alpha".to_string(), "2".to_string(), "1".to_string(), "2".to_string(),],
                vec!["beta".to_string(), "2".to_string(), "2".to_string(), "2".to_string(),],
                vec!["alpha".to_string(), "9".to_string(), "3".to_string(), "9".to_string(),],
                vec!["beta".to_string(), "9".to_string(), "6".to_string(), "9".to_string(),],
            ]
        );
    }

    fn frame_rows(frame: &DataFrame, columns: &[&str]) -> Vec<Vec<String>> {
        (0..frame.height())
            .map(|row_idx| {
                columns
                    .iter()
                    .map(|column| {
                        let value =
                            frame.column(column).expect("column").get(row_idx).expect("row value");
                        any_value_to_string(value)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    fn any_value_to_string(value: AnyValue<'_>) -> String {
        match value {
            AnyValue::String(value) => value.to_owned(),
            AnyValue::StringOwned(value) => value.to_string(),
            AnyValue::Float64(value) => {
                if value.fract() == 0.0 {
                    format!("{value:.0}")
                } else {
                    value.to_string()
                }
            }
            AnyValue::UInt64(value) => value.to_string(),
            AnyValue::UInt32(value) => value.to_string(),
            AnyValue::Int64(value) => value.to_string(),
            AnyValue::Int32(value) => value.to_string(),
            other => other.to_string(),
        }
    }
}
