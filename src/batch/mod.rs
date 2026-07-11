//! Deterministic Monte Carlo batch orchestration and series aggregation.
//!
//! Derives per-run seeds from a batch `base_seed`, executes runs sequentially or
//! via Rayon when the `parallel` feature is enabled, then folds per-run series into
//! ordered aggregate tables. Without `parallel`, a requested Rayon mode falls back
//! to single-thread execution so reports remain reproducible.

use std::collections::BTreeMap;

use crate::engine::{
    resolve_aggregation_selection, run_batch_sample, BatchSample, ResolvedAggregationSelection,
};
use crate::error::RunError;
use crate::rng::derive_run_seed;
use crate::types::{
    BatchConfig, BatchReport, BatchRunSummary, CaptureConfig, ExecutionMode, RunConfig,
    SeriesPoint, SeriesTable,
};
use crate::CompiledScenario;

#[derive(Debug)]
struct IndexedBatchSample {
    run_index: u64,
    sample: BatchSample,
}

/// Executes all batch runs and returns summaries plus step-aligned aggregate series.
///
/// `completed_runs` counts produced run summaries, not how many runs set
/// `completed == true` on their individual reports.
pub(crate) fn run_batch(
    compiled: &CompiledScenario,
    config: &BatchConfig,
) -> Result<BatchReport, RunError> {
    let resolved_metrics =
        resolve_aggregation_selection(compiled, &config.run_template.aggregation)?;
    let execution_mode = resolved_execution_mode(&config.execution_mode);
    let mut samples = execute_runs(compiled, config, &execution_mode, &resolved_metrics)?;
    samples.sort_by_key(|entry| entry.run_index);

    let aggregate_series = aggregate_series(&samples);
    let runs = samples
        .into_iter()
        .map(|entry| BatchRunSummary {
            run_index: entry.run_index,
            seed: entry.sample.seed,
            completed: entry.sample.completed,
            steps_executed: entry.sample.steps_executed,
            final_metrics: entry.sample.final_metrics,
            manifest: entry.sample.manifest,
        })
        .collect::<Vec<_>>();

    Ok(BatchReport {
        scenario_id: compiled.scenario_id().clone(),
        requested_runs: config.runs,
        completed_runs: runs.len() as u64,
        execution_mode,
        runs,
        aggregate_series,
        manifest: None,
    })
}

fn execute_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    execution_mode: &ExecutionMode,
    resolved_metrics: &ResolvedAggregationSelection,
) -> Result<Vec<IndexedBatchSample>, RunError> {
    match execution_mode {
        ExecutionMode::SingleThread => (0..config.runs)
            .map(|run_index| execute_run(compiled, config, run_index, resolved_metrics))
            .collect(),
        ExecutionMode::Rayon => execute_parallel_runs(compiled, config, resolved_metrics),
    }
}

fn execute_run(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    run_index: u64,
    resolved_metrics: &ResolvedAggregationSelection,
) -> Result<IndexedBatchSample, RunError> {
    let run_config = per_run_config(config, run_index);
    let sample = run_batch_sample(
        compiled,
        &run_config,
        &config.run_template.aggregation,
        resolved_metrics,
    )?;
    Ok(IndexedBatchSample { run_index, sample })
}

fn per_run_config(config: &BatchConfig, run_index: u64) -> RunConfig {
    RunConfig {
        seed: derive_run_seed(config.base_seed, run_index),
        max_steps: config.run_template.max_steps,
        capture: CaptureConfig::none(),
    }
}

#[cfg(feature = "parallel")]
fn execute_parallel_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    resolved_metrics: &ResolvedAggregationSelection,
) -> Result<Vec<IndexedBatchSample>, RunError> {
    use rayon::prelude::*;

    (0..config.runs)
        .into_par_iter()
        .map(|run_index| execute_run(compiled, config, run_index, resolved_metrics))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn execute_parallel_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    resolved_metrics: &ResolvedAggregationSelection,
) -> Result<Vec<IndexedBatchSample>, RunError> {
    (0..config.runs)
        .map(|run_index| execute_run(compiled, config, run_index, resolved_metrics))
        .collect()
}

#[cfg(feature = "parallel")]
fn resolved_execution_mode(requested: &ExecutionMode) -> ExecutionMode {
    requested.clone()
}

#[cfg(not(feature = "parallel"))]
fn resolved_execution_mode(requested: &ExecutionMode) -> ExecutionMode {
    match requested {
        ExecutionMode::Rayon | ExecutionMode::SingleThread => ExecutionMode::SingleThread,
    }
}

fn aggregate_series(
    samples: &[IndexedBatchSample],
) -> BTreeMap<crate::types::MetricKey, SeriesTable> {
    let mut metric_steps = BTreeMap::<crate::types::MetricKey, BTreeMap<u64, (f64, u64)>>::new();

    for entry in samples {
        for (metric, table) in &entry.sample.series {
            let step_values = metric_steps.entry(metric.clone()).or_default();
            for point in &table.points {
                let (sum, count) = step_values.entry(point.step).or_insert((0.0, 0));
                *sum += point.value;
                *count += 1;
            }
        }
    }

    metric_steps
        .into_iter()
        .map(|(metric, steps)| {
            let points = steps
                .into_iter()
                .map(|(step, (sum, count))| SeriesPoint::new(step, sum / count as f64))
                .collect::<Vec<_>>();
            let mut table = SeriesTable::new(metric.clone());
            table.points = points;
            (metric, table)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::{
        collections::{BTreeSet, HashSet},
        num::NonZeroU64,
    };

    use crate::rng::derive_run_seed;
    use crate::types::{
        AggregationConfig, BatchConfig, BatchRunTemplate, CaptureSchedule, EdgeSpec,
        EndConditionSpec, ExecutionMode, MetricKey, NodeId, NodeKind, NodeSpec, ScenarioId,
        ScenarioSpec, Selection, TransferSpec,
    };
    use crate::validation::compile_scenario;

    use super::run_batch;

    #[test]
    fn run_batch_sequential_is_reproducible() {
        let compiled = compiled_fixture();
        let config = fixture_batch_config(6, 0x000A_11CE_55ED_u64, ExecutionMode::SingleThread);

        let report_a = run_batch(&compiled, &config).expect("batch run should succeed");
        let report_b = run_batch(&compiled, &config).expect("batch run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.requested_runs, 6);
        assert_eq!(report_a.completed_runs, 6);
        assert_eq!(report_a.execution_mode, ExecutionMode::SingleThread);
        assert_eq!(report_a.runs.len(), 6);

        for (expected_index, run) in report_a.runs.iter().enumerate() {
            let run_index = expected_index as u64;
            assert_eq!(run.run_index, run_index);
            assert_eq!(run.seed, derive_run_seed(config.base_seed, run_index));
            assert!(run.completed);
            assert_eq!(run.steps_executed, 3);
            assert_eq!(run.final_metrics.get(&MetricKey::fixture("sink")), Some(&3.0));
        }

        let sink_series = report_a
            .aggregate_series
            .get(&MetricKey::fixture("sink"))
            .expect("aggregate metric series should exist");
        let steps = sink_series.points.iter().map(|point| point.step).collect::<Vec<_>>();
        let values = sink_series.points.iter().map(|point| point.value).collect::<Vec<_>>();
        assert_eq!(steps, vec![0, 1, 2, 3]);
        assert_eq!(values, vec![0.0, 1.0, 2.0, 3.0]);
    }

    #[test]
    fn run_batch_stable_order_by_run_index() {
        let compiled = compiled_fixture();
        let config = fixture_batch_config(32, 42, ExecutionMode::SingleThread);

        let report = run_batch(&compiled, &config).expect("batch run should succeed");
        let run_indexes = report.runs.iter().map(|run| run.run_index).collect::<Vec<_>>();
        let expected = (0_u64..32_u64).collect::<Vec<_>>();

        assert_eq!(run_indexes, expected);
    }

    #[test]
    fn run_batch_stress_reproducible_for_large_run_count() {
        let compiled = compiled_fixture();
        let config = fixture_batch_config(256, 0xBADC_0FFE_u64, ExecutionMode::SingleThread);

        let report_a = run_batch(&compiled, &config).expect("batch run should succeed");
        let report_b = run_batch(&compiled, &config).expect("batch run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.completed_runs, 256);

        let sink_series = report_a
            .aggregate_series
            .get(&MetricKey::fixture("sink"))
            .expect("aggregate sink series should exist");
        assert!(
            sink_series.points.windows(2).all(|window| window[0].step < window[1].step),
            "aggregate series steps must remain strictly ordered under stress"
        );
    }

    #[test]
    fn run_batch_derives_seed_for_every_run() {
        let compiled = compiled_fixture();
        let config =
            fixture_batch_config(64, 0x1234_5678_9ABC_DEF0_u64, ExecutionMode::SingleThread);

        let report = run_batch(&compiled, &config).expect("batch run should succeed");
        let mut seen = HashSet::with_capacity(report.runs.len());

        for run in &report.runs {
            let expected_seed = derive_run_seed(config.base_seed, run.run_index);
            assert_eq!(run.seed, expected_seed);
            assert!(
                seen.insert(run.seed),
                "derived per-run seeds must stay unique for sampled run range"
            );
        }
    }

    #[test]
    fn compact_samples_honor_aggregation_schedule_and_selection() {
        let compiled = compiled_fixture();
        let source = MetricKey::fixture("source");
        let sink = MetricKey::fixture("sink");

        let none = fixture_batch_config(4, 7, ExecutionMode::SingleThread)
            .with_aggregation(AggregationConfig::none());
        let none_report = run_batch(&compiled, &none).expect("none aggregation succeeds");
        assert!(none_report.aggregate_series.is_empty());
        assert!(none_report.runs.iter().all(|run| run.final_metrics.contains_key(&sink)));

        let final_only = fixture_batch_config(4, 7, ExecutionMode::SingleThread)
            .with_aggregation(AggregationConfig::final_only());
        let final_report = run_batch(&compiled, &final_only).expect("final aggregation succeeds");
        assert_eq!(
            final_report.aggregate_series[&sink]
                .points
                .iter()
                .map(|point| point.step)
                .collect::<Vec<_>>(),
            vec![3]
        );

        let periodic = fixture_batch_config(4, 7, ExecutionMode::SingleThread).with_aggregation(
            AggregationConfig::default()
                .with_schedule(CaptureSchedule::Every {
                    stride: NonZeroU64::new(2).expect("positive stride"),
                    include_initial: true,
                    include_final: true,
                })
                .with_metrics(Selection::Only(BTreeSet::from([source.clone(), sink.clone()]))),
        );
        let periodic_report =
            run_batch(&compiled, &periodic).expect("periodic aggregation succeeds");
        assert_eq!(periodic_report.aggregate_series.len(), 2);
        assert!(periodic_report.aggregate_series.contains_key(&source));
        assert_eq!(
            periodic_report.aggregate_series[&sink]
                .points
                .iter()
                .map(|point| point.step)
                .collect::<Vec<_>>(),
            vec![0, 2, 3]
        );
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn run_batch_parallel_matches_sequential() {
        let compiled = compiled_fixture();
        let sequential_config = fixture_batch_config(16, 777, ExecutionMode::SingleThread);
        let parallel_config = fixture_batch_config(16, 777, ExecutionMode::Rayon);

        let sequential = run_batch(&compiled, &sequential_config).expect("sequential run succeeds");
        let parallel = run_batch(&compiled, &parallel_config).expect("parallel run succeeds");

        assert_eq!(parallel.execution_mode, ExecutionMode::Rayon);
        assert_eq!(parallel.requested_runs, sequential.requested_runs);
        assert_eq!(parallel.completed_runs, sequential.completed_runs);
        assert_eq!(parallel.runs, sequential.runs);
        assert_eq!(parallel.aggregate_series, sequential.aggregate_series);
        assert_eq!(parallel.manifest, sequential.manifest);
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn run_batch_parallel_matches_sequential_under_stress() {
        let compiled = compiled_fixture();
        let sequential_config = fixture_batch_config(192, 0xF00D_u64, ExecutionMode::SingleThread);
        let parallel_config = fixture_batch_config(192, 0xF00D_u64, ExecutionMode::Rayon);

        let sequential = run_batch(&compiled, &sequential_config).expect("sequential run succeeds");
        let parallel = run_batch(&compiled, &parallel_config).expect("parallel run succeeds");

        assert_eq!(parallel.execution_mode, ExecutionMode::Rayon);
        assert_eq!(parallel.runs, sequential.runs);
        assert_eq!(parallel.aggregate_series, sequential.aggregate_series);
    }

    #[cfg(not(feature = "parallel"))]
    #[test]
    fn run_batch_parallel_request_falls_back_to_sequential() {
        let compiled = compiled_fixture();
        let config = fixture_batch_config(4, 99, ExecutionMode::Rayon);
        let report = run_batch(&compiled, &config).expect("batch run should succeed");

        assert_eq!(report.execution_mode, ExecutionMode::SingleThread);
        assert_eq!(report.runs.len(), 4);
    }

    fn fixture_batch_config(
        runs: u64,
        base_seed: u64,
        execution_mode: ExecutionMode,
    ) -> BatchConfig {
        BatchConfig {
            runs,
            base_seed,
            execution_mode,
            run_template: BatchRunTemplate {
                max_steps: 10,
                aggregation: AggregationConfig::default(),
            },
        }
    }

    fn compiled_fixture() -> crate::CompiledScenario {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let sink_metric = MetricKey::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("batch-scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge"),
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
        scenario.tracked_metrics.insert(sink_metric);

        compile_scenario(&scenario).expect("fixture scenario should compile")
    }
}
