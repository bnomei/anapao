//! Batch execution orchestration and aggregation helpers.

use std::collections::BTreeMap;

use crate::engine::run_single;
use crate::error::RunError;
use crate::rng::derive_run_seed;
use crate::types::{
    BatchConfig, BatchReport, BatchRunSummary, ExecutionMode, RunConfig, RunReport, SeriesPoint,
    SeriesTable,
};
use crate::validation::CompiledScenario;

#[derive(Debug)]
struct IndexedRunReport {
    run_index: u64,
    report: RunReport,
}

/// Executes deterministic multi-run simulation and aggregates run outputs.
pub fn run_batch(
    compiled: &CompiledScenario,
    config: &BatchConfig,
) -> Result<BatchReport, RunError> {
    let execution_mode = resolved_execution_mode(&config.execution_mode);
    let run_reports = execute_runs(compiled, config, &execution_mode)?;

    let aggregate_series = aggregate_series(&run_reports);
    let runs = run_reports
        .into_iter()
        .map(|entry| BatchRunSummary {
            run_index: entry.run_index,
            seed: entry.report.seed,
            completed: entry.report.completed,
            steps_executed: entry.report.steps_executed,
            final_metrics: entry.report.final_metrics,
            manifest: entry.report.manifest,
        })
        .collect::<Vec<_>>();

    Ok(BatchReport {
        scenario_id: compiled.scenario.id.clone(),
        requested_runs: config.runs,
        completed_runs: runs.len() as u64,
        execution_mode,
        confidence_level: config.confidence_level,
        runs,
        aggregate_series,
        manifest: None,
    })
}

fn execute_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    execution_mode: &ExecutionMode,
) -> Result<Vec<IndexedRunReport>, RunError> {
    match execution_mode {
        ExecutionMode::SingleThread => {
            (0..config.runs).map(|run_index| execute_run(compiled, config, run_index)).collect()
        }
        ExecutionMode::Rayon => execute_parallel_runs(compiled, config),
    }
}

fn execute_run(
    compiled: &CompiledScenario,
    config: &BatchConfig,
    run_index: u64,
) -> Result<IndexedRunReport, RunError> {
    let run_config = per_run_config(config, run_index);
    let report = run_single(compiled, &run_config)?;
    Ok(IndexedRunReport { run_index, report })
}

fn per_run_config(config: &BatchConfig, run_index: u64) -> RunConfig {
    let mut run_config = config.run.clone();
    run_config.seed = derive_run_seed(config.base_seed, run_index);
    run_config
}

#[cfg(feature = "parallel")]
fn execute_parallel_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
) -> Result<Vec<IndexedRunReport>, RunError> {
    use rayon::prelude::*;

    (0..config.runs)
        .into_par_iter()
        .map(|run_index| execute_run(compiled, config, run_index))
        .collect()
}

#[cfg(not(feature = "parallel"))]
fn execute_parallel_runs(
    compiled: &CompiledScenario,
    config: &BatchConfig,
) -> Result<Vec<IndexedRunReport>, RunError> {
    (0..config.runs).map(|run_index| execute_run(compiled, config, run_index)).collect()
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
    run_reports: &[IndexedRunReport],
) -> BTreeMap<crate::types::MetricKey, SeriesTable> {
    let mut metric_steps = BTreeMap::<crate::types::MetricKey, BTreeMap<u64, (f64, u64)>>::new();

    for entry in run_reports {
        for (metric, table) in &entry.report.series {
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
    use std::collections::HashSet;

    use crate::rng::derive_run_seed;
    use crate::types::{
        BatchConfig, CaptureConfig, ConfidenceLevel, EdgeSpec, EndConditionSpec, ExecutionMode,
        MetricKey, NodeId, NodeKind, NodeSpec, RunConfig, ScenarioId, ScenarioSpec, TransferSpec,
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
    fn run_batch_rewrites_template_seed_for_every_run() {
        let compiled = compiled_fixture();
        let mut config =
            fixture_batch_config(64, 0x1234_5678_9ABC_DEF0_u64, ExecutionMode::SingleThread);
        config.run.seed = u64::MAX;

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
    fn run_batch_preserves_configured_confidence_level() {
        let compiled = compiled_fixture();
        let mut config = fixture_batch_config(4, 2024, ExecutionMode::SingleThread);
        config.confidence_level = ConfidenceLevel::P99;

        let report = run_batch(&compiled, &config).expect("batch run should succeed");
        assert_eq!(report.confidence_level, ConfidenceLevel::P99);
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
            confidence_level: ConfidenceLevel::default(),
            run: RunConfig {
                // Batch execution should overwrite this with derived per-run seeds.
                seed: 123_456,
                max_steps: 10,
                capture: CaptureConfig::default(),
            },
        }
    }

    fn compiled_fixture() -> crate::validation::CompiledScenario {
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
