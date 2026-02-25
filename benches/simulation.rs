// SPDX-FileCopyrightText: 2026 Bruno Meilick
// SPDX-License-Identifier: LicenseRef-Anapao-FreeUse-NoCopy-NoDerivatives
//
// All rights reserved.
//
// This file is part of Anapao and is proprietary software.
// Unauthorized copying, modification, or distribution is prohibited.

use std::collections::BTreeMap;
use std::fs;
use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use anapao::artifact::write_run_artifacts;
use anapao::events::VecEventSink;
use anapao::simulator::Simulator;
use anapao::testkit::{deterministic_batch_config, deterministic_run_config, fixture_scenario};
use anapao::types::{
    ActionMode, BatchConfig, BatchReport, CaptureConfig, ConnectionKind, DelayNodeConfig,
    EdgeConnectionConfig, EdgeId, EdgeSpec, EndConditionSpec, ExecutionMode, ManifestRef,
    MetricKey, NodeConfig, NodeId, NodeKind, NodeModeConfig, NodeSpec, PoolNodeConfig,
    QueueNodeConfig, RunConfig, RunReport, ScenarioId, ScenarioSpec, SortingGateNodeConfig,
    StateConnectionConfig, StateConnectionRole, StateConnectionTarget, TransferSpec, TriggerMode,
    VariableRuntimeConfig, VariableSourceSpec, VariableUpdateTiming,
};
use anapao::validation::CompiledScenario;
use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};

mod profiler;

static BENCH_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

struct BenchTempDir {
    path: PathBuf,
}

impl BenchTempDir {
    fn new(prefix: &str) -> Self {
        let index = BENCH_DIR_COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut path = std::env::temp_dir();
        path.push(format!("anapao_bench_{prefix}_{}_{}", std::process::id(), index));

        if path.exists() {
            fs::remove_dir_all(&path).expect("clear stale bench temp dir");
        }
        fs::create_dir_all(&path).expect("create bench temp dir");

        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for BenchTempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn checksum_compiled(compiled: &CompiledScenario) -> u64 {
    let mut acc = 0u64;
    acc = acc.wrapping_mul(131).wrapping_add(compiled.node_order.len() as u64);
    acc = acc.wrapping_mul(131).wrapping_add(compiled.edge_order.len() as u64);

    for node_id in &compiled.node_order {
        acc = acc.wrapping_mul(131).wrapping_add(node_id.as_str().len() as u64);
    }
    for edge_id in &compiled.edge_order {
        acc = acc.wrapping_mul(131).wrapping_add(edge_id.as_str().len() as u64);
    }

    acc
}

fn checksum_run_report(report: &RunReport) -> u64 {
    let mut acc = 0u64;
    acc = acc.wrapping_mul(131).wrapping_add(report.seed);
    acc = acc.wrapping_mul(131).wrapping_add(report.steps_executed);
    acc = acc.wrapping_mul(131).wrapping_add(report.completed as u64);
    acc = acc.wrapping_mul(131).wrapping_add(report.node_snapshots.len() as u64);
    acc = acc.wrapping_mul(131).wrapping_add(report.series.len() as u64);

    for snapshot in &report.node_snapshots {
        acc = acc.wrapping_mul(131).wrapping_add(snapshot.step);
        acc = acc.wrapping_mul(131).wrapping_add(snapshot.values.len() as u64);
    }

    for value in report.final_metrics.values() {
        acc = acc.wrapping_mul(131).wrapping_add(value.to_bits());
    }

    acc
}

fn checksum_batch_report(report: &BatchReport) -> u64 {
    let mut acc = 0u64;
    acc = acc.wrapping_mul(131).wrapping_add(report.requested_runs);
    acc = acc.wrapping_mul(131).wrapping_add(report.completed_runs);
    acc = acc.wrapping_mul(131).wrapping_add(report.runs.len() as u64);
    acc = acc.wrapping_mul(131).wrapping_add(report.aggregate_series.len() as u64);

    for run in &report.runs {
        acc = acc.wrapping_mul(131).wrapping_add(run.run_index);
        acc = acc.wrapping_mul(131).wrapping_add(run.seed);
        acc = acc.wrapping_mul(131).wrapping_add(run.steps_executed);
        acc = acc.wrapping_mul(131).wrapping_add(run.completed as u64);
    }

    acc
}

fn checksum_written_artifacts(output_dir: &Path, manifest: &ManifestRef) -> u64 {
    let mut acc = 0u64;

    for (key, artifact) in &manifest.artifacts {
        acc = acc.wrapping_mul(131).wrapping_add(key.len() as u64);
        acc = acc.wrapping_mul(131).wrapping_add(artifact.path.len() as u64);
        if let Some(content_type) = &artifact.content_type {
            acc = acc.wrapping_mul(131).wrapping_add(content_type.len() as u64);
        }

        let size =
            fs::metadata(output_dir.join(&artifact.path)).expect("artifact file metadata").len();
        acc = acc.wrapping_mul(131).wrapping_add(size);
    }

    acc
}

fn automatic_push_any_mode() -> NodeModeConfig {
    NodeModeConfig { trigger_mode: TriggerMode::Automatic, action_mode: ActionMode::PushAny }
}

fn pool_with_mode(id: &str, initial_value: f64) -> NodeSpec {
    NodeSpec::new(NodeId::fixture(id), NodeKind::Pool)
        .with_config(NodeConfig::Pool(PoolNodeConfig {
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

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("bench-expanded-semantics"))
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
        .with_node(NodeSpec::new(sink_queue, NodeKind::Pool))
        .with_node(NodeSpec::new(sink_random, NodeKind::Pool))
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
            NodeId::fixture("sink_queue"),
            TransferSpec::Remaining,
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-random-sink"),
            source_random,
            NodeId::fixture("sink_random"),
            TransferSpec::Expression { formula: "burst + matrix_pick".to_string() },
        ));

    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::EveryStep,
        sources: BTreeMap::from([
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
    scenario.tracked_metrics.insert(MetricKey::fixture("sink_queue"));
    scenario.tracked_metrics.insert(MetricKey::fixture("sink_random"));
    scenario
}

fn compile_large_topology_scenario(
    layers: usize,
    nodes_per_layer: usize,
    fanout: usize,
) -> ScenarioSpec {
    assert!(layers >= 2, "layers must be >= 2");
    assert!(nodes_per_layer > 0, "nodes_per_layer must be > 0");
    assert!(fanout > 0, "fanout must be > 0");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("bench-compile-large-topology"));

    for layer in 0..layers {
        for idx in 0..nodes_per_layer {
            let node_id = format!("layer_{layer:02}_node_{idx:03}");
            let initial = if layer == 0 { 64.0 } else { 0.0 };
            scenario = scenario.with_node(pool_with_mode(&node_id, initial));
        }
    }

    let mut edge_index = 0usize;
    for layer in 0..(layers - 1) {
        for idx in 0..nodes_per_layer {
            for branch in 0..fanout {
                let to_idx = (idx + branch) % nodes_per_layer;
                let from = NodeId::fixture(format!("layer_{layer:02}_node_{idx:03}"));
                let to = NodeId::fixture(format!("layer_{:02}_node_{to_idx:03}", layer + 1));
                let edge_id = EdgeId::fixture(format!("edge_{edge_index:06}"));
                scenario = scenario.with_edge(EdgeSpec::new(
                    edge_id,
                    from,
                    to,
                    TransferSpec::Fixed { amount: 1.0 },
                ));
                edge_index += 1;
            }
        }
    }

    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 4 }];
    scenario.tracked_metrics.insert(MetricKey::fixture("layer_00_node_000"));
    scenario
}

fn expression_fanout_scenario() -> ScenarioSpec {
    let source = NodeId::fixture("expr_source");
    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("bench-expression-fanout"))
        .with_node(pool_with_mode("expr_source", 100_000.0));

    for idx in 0..32 {
        let sink = NodeId::fixture(format!("expr_sink_{idx:02}"));
        let edge = EdgeId::fixture(format!("expr_edge_{idx:02}"));
        let formula = format!(
            "min(available, burst + list_pick + matrix_pick + 1 + (step / 8) + ({idx} / 16))"
        );

        scenario = scenario.with_node(NodeSpec::new(sink.clone(), NodeKind::Pool)).with_edge(
            EdgeSpec::new(edge, source.clone(), sink, TransferSpec::Expression { formula }),
        );
    }

    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::EveryStep,
        sources: BTreeMap::from([
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
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 64 }];
    scenario.tracked_metrics.insert(MetricKey::fixture("expr_source"));
    scenario
}

fn sorting_gate_routing_scenario() -> ScenarioSpec {
    let source = NodeId::fixture("gate_source");
    let gate = NodeId::fixture("gate_router");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("bench-sorting-gate-routing"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(120.0))
        .with_node(NodeSpec::new(gate.clone(), NodeKind::SortingGate).with_config(
            NodeConfig::SortingGate(SortingGateNodeConfig { mode: automatic_push_any_mode() }),
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-gate"),
            source,
            gate.clone(),
            TransferSpec::Remaining,
        ));

    for idx in 0..12 {
        let sink = NodeId::fixture(format!("gate_sink_{idx:02}"));
        let edge = EdgeId::fixture(format!("edge-gate-{idx:02}"));
        let weight = (idx % 4 + 1) as f64;

        scenario = scenario.with_node(NodeSpec::new(sink.clone(), NodeKind::Pool)).with_edge(
            EdgeSpec::new(edge, gate.clone(), sink, TransferSpec::Fixed { amount: weight }),
        );
    }

    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 48 }];
    scenario.tracked_metrics.insert(MetricKey::fixture("gate_router"));
    scenario
}

fn state_modifier_stress_scenario() -> ScenarioSpec {
    let source = NodeId::fixture("state_source");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("bench-state-modifier-stress"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Pool).with_initial_value(1.0));

    for idx in 0..24 {
        let target = NodeId::fixture(format!("state_target_{idx:02}"));
        let edge = EdgeId::fixture(format!("state_edge_{idx:02}"));
        let formula = format!("+min(source + drift + ({idx} / 12), target + next_step)");

        scenario = scenario
            .with_node(NodeSpec::new(target.clone(), NodeKind::Pool).with_initial_value(4.0))
            .with_edge(
                EdgeSpec::new(edge, source.clone(), target, TransferSpec::Remaining)
                    .with_connection(EdgeConnectionConfig {
                        kind: ConnectionKind::State,
                        resource: Default::default(),
                        state: StateConnectionConfig {
                            role: StateConnectionRole::Modifier,
                            formula,
                            target: StateConnectionTarget::Node,
                            target_connection: None,
                            resource_filter: None,
                        },
                    }),
            );
    }

    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::EveryStep,
        sources: BTreeMap::from([(
            "drift".to_string(),
            VariableSourceSpec::RandomInterval { min: 1, max: 3 },
        )]),
    };
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 48 }];
    scenario.tracked_metrics.insert(MetricKey::fixture("state_source"));
    scenario
}

// Benchmark identity (keep stable):
// - Group name in this file: `simulation.guardrails`
// - Case IDs (the string after the `/`) must remain stable across refactors.
fn benches_simulation_guardrails(c: &mut Criterion) {
    let mut group = c.benchmark_group("simulation.guardrails");

    let compile_fixture = fixture_scenario();
    group.throughput(Throughput::Elements(compile_fixture.nodes.len() as u64));
    group.bench_function("compile_scenario", move |b| {
        b.iter(|| {
            let compiled =
                Simulator::compile(black_box(compile_fixture.clone())).expect("compile scenario");
            black_box(checksum_compiled(&compiled))
        })
    });

    let compiled_single = Simulator::compile(fixture_scenario()).expect("compile scenario fixture");
    let run_config = deterministic_run_config();
    group.bench_function("single_run", move |b| {
        b.iter(|| {
            let report =
                Simulator::run(black_box(&compiled_single), black_box(run_config.clone()), None)
                    .expect("single run");
            black_box(checksum_run_report(&report))
        })
    });

    let expanded_compiled_single = Simulator::compile(expanded_semantics_scenario())
        .expect("compile expanded scenario fixture");
    let expanded_run_config =
        RunConfig { seed: 0xA11C_E55E_D_u64, max_steps: 64, capture: CaptureConfig::default() };
    group.bench_function("single_run_expanded_semantics", move |b| {
        b.iter(|| {
            let report = Simulator::run(
                black_box(&expanded_compiled_single),
                black_box(expanded_run_config.clone()),
                None,
            )
            .expect("single run expanded semantics");
            black_box(checksum_run_report(&report))
        })
    });

    let compiled_batch = Simulator::compile(fixture_scenario()).expect("compile scenario fixture");
    let batch_config = deterministic_batch_config();
    group.throughput(Throughput::Elements(batch_config.runs));
    group.bench_function("batch_run_sequential", move |b| {
        b.iter(|| {
            let report = Simulator::run_batch(
                black_box(&compiled_batch),
                black_box(batch_config.clone()),
                None,
            )
            .expect("batch run");
            black_box(checksum_batch_report(&report))
        })
    });

    let expanded_compiled_batch = Simulator::compile(expanded_semantics_scenario())
        .expect("compile expanded scenario fixture");
    let expanded_batch_config = BatchConfig {
        runs: 96,
        base_seed: 0xD1FF_EE11_u64,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 123_456, max_steps: 64, capture: CaptureConfig::default() },
    };
    group.throughput(Throughput::Elements(expanded_batch_config.runs));
    group.bench_function("batch_run_expanded_semantics", move |b| {
        b.iter(|| {
            let report = Simulator::run_batch(
                black_box(&expanded_compiled_batch),
                black_box(expanded_batch_config.clone()),
                None,
            )
            .expect("batch run expanded semantics");
            black_box(checksum_batch_report(&report))
        })
    });

    #[cfg(feature = "parallel")]
    {
        let expanded_compiled_batch_rayon = Simulator::compile(expanded_semantics_scenario())
            .expect("compile expanded scenario fixture");
        let expanded_batch_config_rayon = BatchConfig {
            runs: 96,
            base_seed: 0xD1FF_EE11_u64,
            execution_mode: ExecutionMode::Rayon,
            run: RunConfig { seed: 123_456, max_steps: 64, capture: CaptureConfig::default() },
        };
        group.throughput(Throughput::Elements(expanded_batch_config_rayon.runs));
        group.bench_function("batch_run_expanded_semantics_rayon", move |b| {
            b.iter(|| {
                let report = Simulator::run_batch(
                    black_box(&expanded_compiled_batch_rayon),
                    black_box(expanded_batch_config_rayon.clone()),
                    None,
                )
                .expect("batch run expanded semantics rayon");
                black_box(checksum_batch_report(&report))
            })
        });
    }

    let compiled_artifact =
        Simulator::compile(fixture_scenario()).expect("compile scenario fixture");
    let mut sink = VecEventSink::new();
    let artifact_run_report =
        Simulator::run(&compiled_artifact, deterministic_run_config(), Some(&mut sink))
            .expect("seed run");
    let artifact_events = sink.into_events();

    group.bench_function("artifact_write_path", move |b| {
        b.iter_batched(
            || BenchTempDir::new("simulation_artifact_write"),
            |dir| {
                let manifest = write_run_artifacts(
                    black_box(dir.path()),
                    black_box(&artifact_run_report),
                    black_box(&artifact_events),
                )
                .expect("write run artifacts");
                black_box(checksum_written_artifacts(dir.path(), &manifest))
            },
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn benches_simulation_hotspots(c: &mut Criterion) {
    let mut group = c.benchmark_group("simulation.hotspots");

    let compile_stress = compile_large_topology_scenario(16, 24, 4);
    group.throughput(Throughput::Elements(
        (compile_stress.nodes.len() + compile_stress.edges.len()) as u64,
    ));
    group.bench_function("compile_large_topology", move |b| {
        b.iter(|| {
            let compiled =
                Simulator::compile(black_box(compile_stress.clone())).expect("compile scenario");
            black_box(checksum_compiled(&compiled))
        })
    });

    let expression_compiled =
        Simulator::compile(expression_fanout_scenario()).expect("compile expression scenario");
    let expression_run_config =
        RunConfig { seed: 0xC0DE_4510_u64, max_steps: 64, capture: CaptureConfig::disabled() };
    group.throughput(Throughput::Elements(expression_run_config.max_steps));
    group.bench_function("single_run_expression_fanout", move |b| {
        b.iter(|| {
            let report = Simulator::run(
                black_box(&expression_compiled),
                black_box(expression_run_config.clone()),
                None,
            )
            .expect("run expression fanout");
            black_box(checksum_run_report(&report))
        })
    });

    let expression_events_compiled =
        Simulator::compile(expression_fanout_scenario()).expect("compile expression scenario");
    let expression_events_run_config =
        RunConfig { seed: 0xC0DE_4510_u64, max_steps: 64, capture: CaptureConfig::disabled() };
    group.bench_function("single_run_expression_fanout_with_events", move |b| {
        b.iter(|| {
            let mut sink = VecEventSink::new();
            let report = Simulator::run(
                black_box(&expression_events_compiled),
                black_box(expression_events_run_config.clone()),
                Some(&mut sink),
            )
            .expect("run expression fanout with events");
            let event_count = sink.into_events().len() as u64;
            black_box(checksum_run_report(&report).wrapping_add(event_count))
        })
    });

    let gate_compiled =
        Simulator::compile(sorting_gate_routing_scenario()).expect("compile gate scenario");
    let gate_run_config =
        RunConfig { seed: 0xBEE5_0001_u64, max_steps: 48, capture: CaptureConfig::disabled() };
    group.throughput(Throughput::Elements(gate_run_config.max_steps));
    group.bench_function("single_run_sorting_gate_routing", move |b| {
        b.iter(|| {
            let report =
                Simulator::run(black_box(&gate_compiled), black_box(gate_run_config.clone()), None)
                    .expect("run sorting gate routing");
            black_box(checksum_run_report(&report))
        })
    });

    let state_compiled =
        Simulator::compile(state_modifier_stress_scenario()).expect("compile state scenario");
    let state_run_config =
        RunConfig { seed: 0x5A7E_0001_u64, max_steps: 48, capture: CaptureConfig::disabled() };
    group.throughput(Throughput::Elements(state_run_config.max_steps));
    group.bench_function("single_run_state_modifiers", move |b| {
        b.iter(|| {
            let report = Simulator::run(
                black_box(&state_compiled),
                black_box(state_run_config.clone()),
                None,
            )
            .expect("run state modifiers");
            black_box(checksum_run_report(&report))
        })
    });

    let batch_expression_compiled =
        Simulator::compile(expression_fanout_scenario()).expect("compile expression scenario");
    let batch_expression_config = BatchConfig {
        runs: 24,
        base_seed: 0xA0A0_4242_u64,
        execution_mode: ExecutionMode::SingleThread,
        run: RunConfig { seed: 0xC0DE_4510_u64, max_steps: 48, capture: CaptureConfig::disabled() },
    };
    group.throughput(Throughput::Elements(batch_expression_config.runs));
    group.bench_function("batch_run_expression_fanout", move |b| {
        b.iter(|| {
            let report = Simulator::run_batch(
                black_box(&batch_expression_compiled),
                black_box(batch_expression_config.clone()),
                None,
            )
            .expect("batch expression fanout");
            black_box(checksum_batch_report(&report))
        })
    });

    #[cfg(feature = "parallel")]
    {
        let batch_expression_compiled_rayon =
            Simulator::compile(expression_fanout_scenario()).expect("compile expression scenario");
        let batch_expression_config_rayon = BatchConfig {
            runs: 24,
            base_seed: 0xA0A0_4242_u64,
            execution_mode: ExecutionMode::Rayon,
            run: RunConfig {
                seed: 0xC0DE_4510_u64,
                max_steps: 48,
                capture: CaptureConfig::disabled(),
            },
        };
        group.throughput(Throughput::Elements(batch_expression_config_rayon.runs));
        group.bench_function("batch_run_expression_fanout_rayon", move |b| {
            b.iter(|| {
                let report = Simulator::run_batch(
                    black_box(&batch_expression_compiled_rayon),
                    black_box(batch_expression_config_rayon.clone()),
                    None,
                )
                .expect("batch expression fanout rayon");
                black_box(checksum_batch_report(&report))
            })
        });
    }

    let artifact_compiled =
        Simulator::compile(expression_fanout_scenario()).expect("compile expression scenario");
    let mut sink = VecEventSink::new();
    let artifact_run_report = Simulator::run(
        &artifact_compiled,
        RunConfig { seed: 0xA77E_9001_u64, max_steps: 96, capture: CaptureConfig::default() },
        Some(&mut sink),
    )
    .expect("seed artifact run");
    let artifact_events = sink.into_events();
    let artifact_run_report_io = artifact_run_report.clone();
    let artifact_events_io = artifact_events.clone();

    group.throughput(Throughput::Elements(
        (artifact_run_report.node_snapshots.len() + artifact_events.len()) as u64,
    ));
    group.bench_function("artifact_write_expanded_capture", move |b| {
        b.iter_batched(
            || BenchTempDir::new("simulation_artifact_write_expanded"),
            |dir| {
                let manifest = write_run_artifacts(
                    black_box(dir.path()),
                    black_box(&artifact_run_report),
                    black_box(&artifact_events),
                )
                .expect("write expanded artifacts");
                black_box(checksum_written_artifacts(dir.path(), &manifest))
            },
            BatchSize::SmallInput,
        )
    });

    let artifact_io_dir = BenchTempDir::new("simulation_artifact_write_expanded_io_only");
    group.bench_function("artifact_write_expanded_capture_io_only", move |b| {
        b.iter(|| {
            let manifest = write_run_artifacts(
                black_box(artifact_io_dir.path()),
                black_box(&artifact_run_report_io),
                black_box(&artifact_events_io),
            )
            .expect("write expanded artifacts io-only");
            black_box(checksum_written_artifacts(artifact_io_dir.path(), &manifest))
        })
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = profiler::criterion();
    targets = benches_simulation_guardrails, benches_simulation_hotspots
}
criterion_main!(benches);
