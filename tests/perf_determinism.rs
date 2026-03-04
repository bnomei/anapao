use std::collections::{BTreeMap, HashSet};

use anapao::batch::run_batch;
use anapao::engine::run_single;
use anapao::rng::derive_run_seed;
use anapao::types::{
    ActionMode, BatchConfig, CaptureConfig, DelayNodeConfig, EdgeId, EdgeSpec, EndConditionSpec,
    ExecutionMode, MetricKey, NodeConfig, NodeId, NodeKind, NodeModeConfig, NodeSpec,
    QueueNodeConfig, RunConfig, ScenarioId, ScenarioSpec, TransferSpec, TriggerMode,
    VariableRuntimeConfig, VariableSourceSpec, VariableUpdateTiming,
};
use anapao::validation::{compile_scenario, CompiledScenario};

fn automatic_push_any_mode() -> NodeModeConfig {
    NodeModeConfig { trigger_mode: TriggerMode::Automatic, action_mode: ActionMode::PushAny }
}

fn pool_with_mode(id: &str, initial_value: f64) -> NodeSpec {
    NodeSpec::new(NodeId::fixture(id), NodeKind::Pool)
        .with_config(NodeConfig::Pool(anapao::types::PoolNodeConfig {
            capacity: None,
            allow_negative_start: false,
            mode: automatic_push_any_mode(),
        }))
        .with_initial_value(initial_value)
}

fn expanded_semantics_scenario() -> ScenarioSpec {
    let source_delay = NodeId::fixture("source_delay");
    let source_random = NodeId::fixture("source_random");
    let delay = NodeId::fixture("delay");
    let queue = NodeId::fixture("queue");
    let sink_queue = NodeId::fixture("sink_queue");
    let sink_random = NodeId::fixture("sink_random");
    let sink_queue_metric = MetricKey::fixture("sink_queue");
    let sink_random_metric = MetricKey::fixture("sink_random");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("perf-determinism-expanded"))
        .with_node(pool_with_mode("source_delay", 64.0))
        .with_node(pool_with_mode("source_random", 512.0))
        .with_node(NodeSpec::new(delay.clone(), NodeKind::Delay).with_config(NodeConfig::Delay(
            DelayNodeConfig { delay_steps: 2, mode: automatic_push_any_mode() },
        )))
        .with_node(NodeSpec::new(queue.clone(), NodeKind::Queue).with_config(NodeConfig::Queue(
            QueueNodeConfig {
                capacity: None,
                release_per_step: 1,
                mode: automatic_push_any_mode(),
            },
        )))
        .with_node(NodeSpec::new(sink_queue.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(sink_random.clone(), NodeKind::Pool))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-delay"),
            source_delay,
            delay.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-delay-queue"),
            delay,
            queue.clone(),
            TransferSpec::Remaining,
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-random-queue"),
            source_random.clone(),
            queue.clone(),
            TransferSpec::Expression { formula: "list_pick".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-queue-sink"),
            queue,
            sink_queue,
            TransferSpec::Remaining,
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-random-sink"),
            source_random,
            sink_random,
            TransferSpec::Expression { formula: "burst + matrix_pick".to_string() },
        ));

    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::EveryStep,
        sources: std::collections::BTreeMap::from([
            ("burst".to_string(), VariableSourceSpec::RandomInterval { min: 1, max: 3 }),
            (
                "list_pick".to_string(),
                VariableSourceSpec::RandomList { values: vec![0.0, 1.0, 2.0] },
            ),
            (
                "matrix_pick".to_string(),
                VariableSourceSpec::RandomMatrix { values: vec![vec![1.0], vec![2.0, 3.0]] },
            ),
        ]),
    };
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 24 }];
    scenario.tracked_metrics.insert(sink_queue_metric);
    scenario.tracked_metrics.insert(sink_random_metric);
    scenario
}

fn compiled_expanded_semantics() -> CompiledScenario {
    compile_scenario(&expanded_semantics_scenario())
        .expect("expanded semantics scenario should compile")
}

fn generated_determinism_property_scenario(case_index: u64) -> ScenarioSpec {
    let source = NodeId::fixture(format!("source-{case_index}"));
    let sink = NodeId::fixture(format!("sink-{case_index}"));
    let edge = EdgeId::fixture(format!("edge-{case_index}"));

    let mut scenario =
        ScenarioSpec::new(ScenarioId::fixture(format!("perf-determinism-generated-{case_index}")))
            .with_node(pool_with_mode(source.as_str(), 240.0 + case_index as f64))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                edge,
                source,
                sink.clone(),
                TransferSpec::Expression { formula: "base + roll + step".to_string() },
            ));

    let roll_source = if case_index % 2 == 0 {
        VariableSourceSpec::RandomInterval { min: 0, max: 4 }
    } else {
        VariableSourceSpec::RandomList { values: vec![0.0, 1.0, 2.0, 3.0, 4.0] }
    };
    scenario.variables = VariableRuntimeConfig {
        update_timing: if case_index % 3 == 0 {
            VariableUpdateTiming::RunStart
        } else {
            VariableUpdateTiming::EveryStep
        },
        sources: BTreeMap::from([
            ("base".to_string(), VariableSourceSpec::Constant { value: (case_index % 3) as f64 }),
            ("roll".to_string(), roll_source),
        ]),
    };
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 + (case_index % 4) }];
    scenario.tracked_metrics.insert(MetricKey::fixture(sink.as_str()));
    scenario
}

#[test]
fn perf_determinism_single_replay_expanded_semantics_stress() {
    let compiled = compiled_expanded_semantics();
    let config =
        RunConfig { seed: 0x000A_11CE_55ED_u64, max_steps: 64, capture: CaptureConfig::default() };
    let baseline = run_single(&compiled, &config).expect("run should succeed");

    for replay in 0..32 {
        let replayed = run_single(&compiled, &config).expect("replay run should succeed");
        assert_eq!(
            replayed, baseline,
            "single-run replay diverged at iteration {replay}; seed={}",
            config.seed
        );
    }

    assert_eq!(baseline.steps_executed, 24);
    assert!(baseline.completed);
    assert!(
        baseline.final_node_values.get(&NodeId::fixture("sink_queue")).copied().unwrap_or_default()
            > 0.0
    );
    assert!(
        baseline
            .final_node_values
            .get(&NodeId::fixture("sink_random"))
            .copied()
            .unwrap_or_default()
            > 0.0
    );
}

#[test]
fn perf_determinism_single_seed_variation_changes_randomized_trace() {
    let compiled = compiled_expanded_semantics();
    let config_a = RunConfig { seed: 101, max_steps: 64, capture: CaptureConfig::default() };
    let config_b = RunConfig { seed: 202, max_steps: 64, capture: CaptureConfig::default() };

    let report_a = run_single(&compiled, &config_a).expect("run A should succeed");
    let report_b = run_single(&compiled, &config_b).expect("run B should succeed");

    assert_ne!(report_a, report_b, "different seeds should produce different random traces");
    assert_ne!(
        report_a.final_node_values.get(&NodeId::fixture("sink_random")),
        report_b.final_node_values.get(&NodeId::fixture("sink_random")),
        "randomized sink outcome should vary across distinct seeds"
    );
}

#[test]
fn perf_determinism_generated_valid_scenarios_replay_by_seed_property() {
    for case_index in 0_u64..12 {
        let compiled = compile_scenario(&generated_determinism_property_scenario(case_index))
            .expect("generated scenario should compile");
        let seeds = [0_u64, 1_u64, 17_u64, 97_u64, 313_u64, case_index + 1_000_u64];
        for seed in seeds {
            let config = RunConfig { seed, max_steps: 64, capture: CaptureConfig::default() };
            let run_a = run_single(&compiled, &config).expect("first replay run should succeed");
            let run_b = run_single(&compiled, &config).expect("second replay run should succeed");
            assert_eq!(
                run_a, run_b,
                "deterministic replay diverged for generated case {case_index} at seed {seed}"
            );
        }
    }
}

#[test]
fn perf_determinism_expression_cache_reuse_tracks_variable_context_changes() {
    let compiled = compile_scenario(&generated_determinism_property_scenario(77))
        .expect("generated scenario should compile");
    let baseline_seed = 101_u64;
    let config_for_seed =
        |seed| RunConfig { seed, max_steps: 64, capture: CaptureConfig::default() };
    let baseline = run_single(&compiled, &config_for_seed(baseline_seed))
        .expect("baseline run should succeed");

    let (alternate_seed, alternate_report) = ((baseline_seed + 1)..(baseline_seed + 2_048))
        .map(|seed| {
            let report = run_single(&compiled, &config_for_seed(seed))
                .expect("alternate run should succeed");
            (seed, report)
        })
        .find(|(_, report)| {
            report.variable_snapshots != baseline.variable_snapshots
                || report.transfers != baseline.transfers
                || report.final_node_values != baseline.final_node_values
        })
        .expect("expected at least one seed with a different variable context");

    assert!(
        alternate_report != baseline,
        "alternate seed {alternate_seed} should produce a different run context"
    );

    let baseline_replay = run_single(&compiled, &config_for_seed(baseline_seed))
        .expect("baseline replay should succeed");
    assert_eq!(
        baseline_replay, baseline,
        "baseline seed replay diverged after alternating run seed {alternate_seed}"
    );
}

#[test]
fn perf_determinism_batch_replay_stress_guardrails() {
    let compiled = compiled_expanded_semantics();
    let config = BatchConfig {
        runs: 192,
        base_seed: 0xD1FF_EE11_u64,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 999_999, max_steps: 64, capture: CaptureConfig::default() },
    };

    let report_a = run_batch(&compiled, &config).expect("batch run should succeed");
    let report_b = run_batch(&compiled, &config).expect("batch replay should succeed");

    assert_eq!(report_a, report_b, "batch replay must be deterministic");
    assert_eq!(report_a.runs.len() as u64, config.runs);

    let expected_indexes = (0_u64..config.runs).collect::<Vec<_>>();
    let actual_indexes = report_a.runs.iter().map(|run| run.run_index).collect::<Vec<_>>();
    assert_eq!(actual_indexes, expected_indexes, "run indexes must stay ordered and complete");

    let mut seen_seeds = HashSet::with_capacity(config.runs as usize);
    for run in &report_a.runs {
        let expected_seed = derive_run_seed(config.base_seed, run.run_index);
        assert_eq!(run.seed, expected_seed, "per-run seed derivation must be stable");
        assert!(seen_seeds.insert(run.seed), "derived seeds should be unique in sampled range");
    }

    let sink_random_metric = MetricKey::fixture("sink_random");
    let sink_queue_metric = MetricKey::fixture("sink_queue");
    let sink_random_series = report_a
        .aggregate_series
        .get(&sink_random_metric)
        .expect("sink_random aggregate series should exist");
    let sink_queue_series = report_a
        .aggregate_series
        .get(&sink_queue_metric)
        .expect("sink_queue aggregate series should exist");

    assert!(
        sink_random_series.points.windows(2).all(|window| window[0].step < window[1].step),
        "sink_random aggregate points must remain sorted by step"
    );
    assert!(
        sink_queue_series.points.windows(2).all(|window| window[0].step < window[1].step),
        "sink_queue aggregate points must remain sorted by step"
    );
}

#[cfg(feature = "parallel")]
#[test]
fn perf_determinism_batch_parallel_matches_sequential_stress() {
    let compiled = compiled_expanded_semantics();
    let sequential = BatchConfig {
        runs: 160,
        base_seed: 0x1A2B_3C4D_u64,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 0, max_steps: 64, capture: CaptureConfig::default() },
    };
    let parallel = BatchConfig { execution_mode: ExecutionMode::Rayon, ..sequential.clone() };

    let report_sequential =
        run_batch(&compiled, &sequential).expect("sequential batch should succeed");
    let report_parallel = run_batch(&compiled, &parallel).expect("parallel batch should succeed");

    assert_eq!(report_parallel.execution_mode, ExecutionMode::Rayon);
    assert_eq!(report_parallel.runs, report_sequential.runs);
    assert_eq!(report_parallel.aggregate_series, report_sequential.aggregate_series);
}

#[cfg(not(feature = "parallel"))]
#[test]
fn perf_determinism_batch_parallel_request_falls_back_deterministically() {
    let compiled = compiled_expanded_semantics();
    let config = BatchConfig {
        runs: 48,
        base_seed: 0x7777_u64,
        execution_mode: ExecutionMode::Rayon,
        run: RunConfig { seed: 123, max_steps: 64, capture: CaptureConfig::default() },
    };

    let report = run_batch(&compiled, &config).expect("batch run should succeed");
    assert_eq!(report.execution_mode, ExecutionMode::SingleThread);
    assert_eq!(report.runs.len() as u64, config.runs);
}
