use std::collections::{BTreeMap, BTreeSet};

use anapao::engine::run_single;
use anapao::error::{RunError, SetupError};
use anapao::rng::rng_from_seed;
use anapao::stochastic::sample_closed_interval;
use anapao::testkit::{
    format_parity_failure, parity_fixture_case, parity_fixture_cases, ParityFixtureCase,
};
use anapao::types::{
    ActionMode, CaptureConfig, ConnectionKind, EdgeConnectionConfig, EdgeId, EdgeSpec,
    EndConditionSpec, NodeConfig, NodeId, NodeKind, NodeModeConfig, NodeSpec, PoolNodeConfig,
    RunConfig, ScenarioId, ScenarioSpec, StateConnectionConfig, StateConnectionRole,
    StateConnectionTarget, TransferSpec, VariableRuntimeConfig, VariableSourceSpec,
    VariableUpdateTiming,
};
use anapao::validation::{compile_scenario, CompiledScenario};

const PARITY_VALUE_SCALE: f64 = 1_000_000.0;

pub const EXPECTED_PARITY_FIXTURE_IDS: [&str; 15] = [
    "fx-001-pool-default-zero",
    "fx-002-pool-negative-via-state-only",
    "fx-003-pool-negative-blocks-output",
    "fx-004-pool-integer-storage",
    "fx-005-converter-input-output-required",
    "fx-006-gate-weight-unit-homogeneity",
    "fx-007-trigger-gate-state-only",
    "fx-008-end-condition-state-input-only",
    "fx-009-end-condition-immediate-stop",
    "fx-010-resource-token-positive-integer",
    "fx-011-state-connection-default-plus-one",
    "fx-012-formula-modifier-additive-next-step",
    "fx-013-pull-push-any-all-behavior",
    "fx-014-variable-case-sensitivity",
    "fx-015-random-interval-inclusive",
];

pub fn expected_fixture_ids() -> Vec<String> {
    EXPECTED_PARITY_FIXTURE_IDS.iter().map(|fixture_id| (*fixture_id).to_string()).collect()
}

pub fn assert_fixture_mapping_coverage() {
    let mut expected = expected_fixture_ids();
    expected.sort();
    let mut from_catalog = parity_fixture_cases()
        .expect("parity catalog should load")
        .into_iter()
        .map(|case| case.fixture_id)
        .collect::<Vec<_>>();
    from_catalog.sort();
    assert_eq!(expected, from_catalog, "parity mapping must cover all catalog fixtures");
}

pub fn assert_fixture_case(fixture_id: &str) {
    let case = parity_fixture_case(fixture_id).unwrap_or_else(|error| {
        panic!("failed to resolve parity fixture {fixture_id}: {error}");
    });
    run_case(&case);
}

fn run_case(case: &ParityFixtureCase) {
    match case.fixture_id.as_str() {
        "fx-001-pool-default-zero" => {
            assert_rule_id(case, "PRB-001");
            assert_descriptor(
                case,
                "Pool initializes with zero resources when initial amount is omitted.",
            );
            check_prb001(case);
        }
        "fx-002-pool-negative-via-state-only" => {
            assert_rule_id(case, "PRB-002");
            assert_descriptor(
                case,
                "Negative Pool state is only legal after a State Connection modifies the value.",
            );
            check_prb002(case);
        }
        "fx-003-pool-negative-blocks-output" => {
            assert_rule_id(case, "PRB-003");
            assert_descriptor(
                case,
                "A negative Pool blocks outbound flow and accepts inbound flow until positive.",
            );
            check_prb003(case);
        }
        "fx-004-pool-integer-storage" => {
            assert_rule_id(case, "PRB-004");
            assert_descriptor(
                case,
                "Pool storage rejects or normalizes direct fractional values to integer semantics.",
            );
            check_prb004(case);
        }
        "fx-005-converter-input-output-required" => {
            assert_rule_id(case, "PRB-005");
            assert_descriptor(
                case,
                "Converter/Trader execution requires at least one input and one output resource edge.",
            );
            check_prb005(case);
        }
        "fx-006-gate-weight-unit-homogeneity" => {
            assert_rule_id(case, "PRB-006");
            assert_descriptor(
                case,
                "Sorting Gate validation fails when percentage and ratio weights are mixed.",
            );
            check_prb006(case);
        }
        "fx-007-trigger-gate-state-only" => {
            assert_rule_id(case, "PRB-007");
            assert_descriptor(case, "Trigger Gate definitions with resource inputs are invalid.");
            check_prb007(case);
        }
        "fx-008-end-condition-state-input-only" => {
            assert_rule_id(case, "PRB-008");
            assert_descriptor(case, "End Condition accepts only State Connection inputs.");
            check_prb008(case);
        }
        "fx-009-end-condition-immediate-stop" => {
            assert_rule_id(case, "PRB-009");
            assert_descriptor(
                case,
                "Simulation stops in the same step where any End Condition evaluates true.",
            );
            check_prb009(case);
        }
        "fx-010-resource-token-positive-integer" => {
            assert_rule_id(case, "PRB-010");
            assert_descriptor(
                case,
                "Resource Connection transfer units are treated as positive integers.",
            );
            check_prb010(case);
        }
        "fx-011-state-connection-default-plus-one" => {
            assert_rule_id(case, "PRB-011");
            assert_descriptor(case, "State Connection formula defaults to +1 when omitted.");
            check_prb011(case);
        }
        "fx-012-formula-modifier-additive-next-step" => {
            assert_rule_id(case, "PRB-012");
            assert_descriptor(
                case,
                "Formula Modifier effects are additive and applied to next-step formula value with signed syntax.",
            );
            check_prb012(case);
        }
        "fx-013-pull-push-any-all-behavior" => {
            assert_rule_id(case, "PRB-013");
            assert_descriptor(
                case,
                "any modes consume available amounts; all modes require full specified amounts.",
            );
            check_prb013(case);
        }
        "fx-014-variable-case-sensitivity" => {
            assert_rule_id(case, "PRB-014");
            assert_descriptor(
                case,
                "Variable references differ by case and must match exact name.",
            );
            check_prb014(case);
        }
        "fx-015-random-interval-inclusive" => {
            assert_rule_id(case, "PRB-015");
            assert_descriptor(
                case,
                "Random interval sampling includes both lower and upper bounds.",
            );
            check_prb015(case);
        }
        unknown => fail(
            case,
            "fixture id is not mapped by parity differential suite",
            evidence(vec![("fixture_id", unknown.to_string())]),
        ),
    }
}

fn assert_rule_id(case: &ParityFixtureCase, expected_rule_id: &str) {
    if case.rule_id != expected_rule_id {
        fail(
            case,
            "fixture rule_id does not match expected mapping",
            evidence(vec![
                ("expected_rule_id", expected_rule_id.to_string()),
                ("actual_rule_id", case.rule_id.clone()),
            ]),
        );
    }
}

fn assert_descriptor(case: &ParityFixtureCase, expected_descriptor: &str) {
    if case.descriptor != expected_descriptor {
        fail(
            case,
            "fixture descriptor does not match expected baseline",
            evidence(vec![
                ("expected_descriptor", expected_descriptor.to_string()),
                ("actual_descriptor", case.descriptor.clone()),
            ]),
        );
    }
}

fn check_prb001(case: &ParityFixtureCase) {
    let pool = NodeId::fixture("pool");
    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb001"))
        .with_node(NodeSpec::new(pool.clone(), NodeKind::Pool));
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

    let compiled = compile_ok(case, &scenario, "pool default scenario should compile");
    let config = run_config(2601, 3);
    let report_a = run_ok(case, &compiled, &config, "pool default run A should succeed");
    let report_b = run_ok(case, &compiled, &config, "pool default run B should succeed");
    if report_a != report_b {
        fail(
            case,
            "run replay is not deterministic",
            evidence(vec![
                ("seed", config.seed.to_string()),
                ("steps_a", report_a.steps_executed.to_string()),
                ("steps_b", report_b.steps_executed.to_string()),
            ]),
        );
    }

    let observed = report_a.final_node_values.get(&pool).copied().unwrap_or(f64::NAN);
    if observed != 0.0 {
        fail(
            case,
            "pool did not initialize to zero when initial_value was omitted",
            evidence(vec![("observed_pool_value", observed.to_string())]),
        );
    }
}

fn check_prb002(case: &ParityFixtureCase) {
    let pool = NodeId::fixture("pool");
    let invalid = ScenarioSpec::new(ScenarioId::fixture("parity-prb002-invalid"))
        .with_node(NodeSpec::new(pool.clone(), NodeKind::Pool).with_initial_value(-1.0));
    let invalid_error = compile_invalid_parameter(
        case,
        &invalid,
        "negative pool start without allow_negative_start must fail",
    );
    expect_invalid_parameter(
        case,
        &invalid_error,
        "nodes.pool.initial_value",
        "must be non-negative unless config.allow_negative_start is true",
    );

    let valid = ScenarioSpec::new(ScenarioId::fixture("parity-prb002-valid")).with_node(
        NodeSpec::new(pool, NodeKind::Pool).with_initial_value(-1.0).with_config(NodeConfig::Pool(
            PoolNodeConfig {
                capacity: None,
                allow_negative_start: true,
                mode: NodeModeConfig::default(),
            },
        )),
    );
    let _compiled =
        compile_ok(case, &valid, "pool negative start with allow_negative_start should compile");
}

fn check_prb003(case: &ParityFixtureCase) {
    let source = NodeId::fixture("z-source");
    let pool = NodeId::fixture("a-pool");
    let sink = NodeId::fixture("m-sink");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb003"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(1.0))
        .with_node(
            NodeSpec::new(pool.clone(), NodeKind::Pool).with_initial_value(-1.0).with_config(
                NodeConfig::Pool(PoolNodeConfig {
                    capacity: None,
                    allow_negative_start: true,
                    mode: NodeModeConfig::default(),
                }),
            ),
        )
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-pool"),
            source.clone(),
            pool.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-pool-sink"),
            pool.clone(),
            sink.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ));
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

    let compiled = compile_ok(case, &scenario, "negative pool blocking scenario should compile");
    let config = run_config(2603, 3);
    let report = run_ok(case, &compiled, &config, "negative pool blocking run should succeed");

    let pool_value = report.final_node_values.get(&pool).copied().unwrap_or(f64::NAN);
    let sink_value = report.final_node_values.get(&sink).copied().unwrap_or(f64::NAN);
    if pool_value != 0.0 || sink_value != 0.0 {
        fail(
            case,
            "negative pool should accept inbound resource while blocking outbound flow in the step",
            evidence(vec![
                ("observed_pool_value", pool_value.to_string()),
                ("observed_sink_value", sink_value.to_string()),
            ]),
        );
    }
}

fn check_prb004(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");
    let scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb004"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge"),
            source,
            sink,
            TransferSpec::Fixed { amount: 1.5 },
        ));
    let error =
        compile_invalid_parameter(case, &scenario, "fractional fixed transfer should be rejected");
    expect_invalid_parameter(
        case,
        &error,
        "edges.edge.transfer.fixed.amount",
        "positive integer token quantities",
    );
}

fn check_prb005(case: &ParityFixtureCase) {
    let converter = NodeId::fixture("converter");
    let sink = NodeId::fixture("sink");
    let scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb005"))
        .with_node(NodeSpec::new(converter.clone(), NodeKind::Converter))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("converter-out"),
            converter,
            sink,
            TransferSpec::Fixed { amount: 1.0 },
        ));
    let error = compile_invalid_parameter(
        case,
        &scenario,
        "converter without inbound edge should be rejected",
    );
    expect_invalid_parameter(
        case,
        &error,
        "nodes.converter.connections",
        "at least one inbound edge",
    );
}

fn check_prb006(case: &ParityFixtureCase) {
    let source = NodeId::fixture("a-source");
    let gate = NodeId::fixture("z-gate");
    let sink_ratio = NodeId::fixture("zz-ratio");
    let sink_percent = NodeId::fixture("zz-percent");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb006"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
        .with_node(NodeSpec::new(gate.clone(), NodeKind::SortingGate))
        .with_node(NodeSpec::new(sink_ratio.clone(), NodeKind::Sink))
        .with_node(NodeSpec::new(sink_percent.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-gate"),
            source.clone(),
            gate.clone(),
            TransferSpec::Fixed { amount: 2.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-gate-ratio"),
            gate.clone(),
            sink_ratio,
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-gate-percent"),
            gate.clone(),
            sink_percent,
            TransferSpec::Fraction { numerator: 1, denominator: 2 },
        ));
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

    let compiled = compile_ok(case, &scenario, "gate mixed-weight scenario should compile");
    let run_error =
        run_error(case, &compiled, &run_config(2606, 3), "mixed gate weights must fail");
    match run_error {
        RunError::InvalidRunConfig { name, reason } => {
            if name != "nodes.z-gate.outputs" || !reason.contains("cannot mix percentage") {
                fail(
                    case,
                    "gate mixed-weight failure shape mismatch",
                    evidence(vec![("name", name), ("reason", reason)]),
                );
            }
        }
        other => fail(
            case,
            "unexpected run error type for mixed gate weights",
            evidence(vec![("error", other.to_string())]),
        ),
    }
}

fn check_prb007(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let trigger = NodeId::fixture("trigger");

    let scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb007"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(1.0))
        .with_node(NodeSpec::new(trigger.clone(), NodeKind::TriggerGate))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-input"),
            source,
            trigger,
            TransferSpec::Fixed { amount: 1.0 },
        ));
    let error = compile_invalid_parameter(
        case,
        &scenario,
        "trigger gate with resource input should be rejected",
    );
    expect_invalid_parameter(
        case,
        &error,
        "nodes.trigger.inputs",
        "cannot have incoming resource edges",
    );
}

fn check_prb008(case: &ParityFixtureCase) {
    let parse_result = serde_json::from_str::<NodeKind>("\"end_condition\"");
    if let Ok(parsed) = parse_result {
        fail(
            case,
            "end condition unexpectedly deserialized as a node kind",
            evidence(vec![("parsed_variant", format!("{parsed:?}"))]),
        );
    }

    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");
    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb008"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge"),
            source,
            sink.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ));
    scenario.end_conditions =
        vec![EndConditionSpec::NodeAtLeast { node_id: sink, value_scaled: scaled(1.0) }];

    let compiled = compile_ok(case, &scenario, "end condition model-level scenario should compile");
    let report = run_ok(
        case,
        &compiled,
        &run_config(2608, 5),
        "end condition model-level run should succeed",
    );
    if !report.completed || report.steps_executed != 1 {
        fail(
            case,
            "end condition did not execute as model-level predicate",
            evidence(vec![
                ("completed", report.completed.to_string()),
                ("steps_executed", report.steps_executed.to_string()),
            ]),
        );
    }
}

fn check_prb009(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb009"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge"),
            source,
            sink.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ));
    scenario.end_conditions =
        vec![EndConditionSpec::NodeAtLeast { node_id: sink.clone(), value_scaled: scaled(2.0) }];

    let compiled = compile_ok(case, &scenario, "end condition stop scenario should compile");
    let config = run_config(2609, 10);
    let report_a = run_ok(case, &compiled, &config, "end condition stop run A should succeed");
    let report_b = run_ok(case, &compiled, &config, "end condition stop run B should succeed");
    if report_a != report_b {
        fail(
            case,
            "end condition replay is not deterministic",
            evidence(vec![
                ("steps_a", report_a.steps_executed.to_string()),
                ("steps_b", report_b.steps_executed.to_string()),
            ]),
        );
    }

    let sink_value = report_a.final_node_values.get(&sink).copied().unwrap_or(f64::NAN);
    if !report_a.completed || report_a.steps_executed != 2 || sink_value != 2.0 {
        fail(
            case,
            "simulation did not stop immediately when end condition became true",
            evidence(vec![
                ("completed", report_a.completed.to_string()),
                ("steps_executed", report_a.steps_executed.to_string()),
                ("sink_value", sink_value.to_string()),
            ]),
        );
    }
}

fn check_prb010(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");
    let scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb010"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge"),
            source,
            sink,
            TransferSpec::Fixed { amount: 0.0 },
        ));
    let error =
        compile_invalid_parameter(case, &scenario, "zero fixed transfer amount should be rejected");
    expect_invalid_parameter(
        case,
        &error,
        "edges.edge.transfer.fixed.amount",
        "positive integer token quantities",
    );
}

fn check_prb011(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");
    let edge_id = EdgeId::fixture("state-edge");

    let scenario = ScenarioSpec::new(ScenarioId::fixture("parity-prb011"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(
            EdgeSpec::new(edge_id.clone(), source, sink, TransferSpec::Remaining).with_connection(
                EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig::default(),
                },
            ),
        );

    let compiled = compile_ok(case, &scenario, "state default formula scenario should compile");
    let observed = compiled
        .scenario
        .edges
        .get(&edge_id)
        .map(|edge| edge.connection.state.formula.clone())
        .unwrap_or_else(|| "<missing-edge>".to_string());
    if observed != "+1" {
        fail(
            case,
            "state connection default formula diverged from +1",
            evidence(vec![("observed_formula", observed)]),
        );
    }
}

fn check_prb012(case: &ParityFixtureCase) {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");

    let invalid = ScenarioSpec::new(ScenarioId::fixture("parity-prb012-invalid"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(
            EdgeSpec::new(
                EdgeId::fixture("state-edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Remaining,
            )
            .with_connection(EdgeConnectionConfig {
                kind: ConnectionKind::State,
                resource: Default::default(),
                state: StateConnectionConfig {
                    role: StateConnectionRole::Modifier,
                    formula: "1".to_string(),
                    target: StateConnectionTarget::Node,
                    target_connection: None,
                    resource_filter: None,
                },
            }),
        );
    let invalid_error =
        compile_invalid_parameter(case, &invalid, "unsigned formula modifier must be rejected");
    expect_invalid_parameter(
        case,
        &invalid_error,
        "edges.state-edge.connection.state.formula",
        "must start with `+` or `-`",
    );

    let mut valid = ScenarioSpec::new(ScenarioId::fixture("parity-prb012-valid"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(
            EdgeSpec::new(
                EdgeId::fixture("state-edge"),
                source,
                sink.clone(),
                TransferSpec::Remaining,
            )
            .with_connection(EdgeConnectionConfig {
                kind: ConnectionKind::State,
                resource: Default::default(),
                state: StateConnectionConfig {
                    role: StateConnectionRole::Modifier,
                    formula: "+1".to_string(),
                    target: StateConnectionTarget::Node,
                    target_connection: None,
                    resource_filter: None,
                },
            }),
        );
    valid.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

    let compiled = compile_ok(case, &valid, "signed additive modifier scenario should compile");
    let report = run_ok(
        case,
        &compiled,
        &run_config(2612, 6),
        "signed additive modifier run should succeed",
    );
    let sink_value = report.final_node_values.get(&sink).copied().unwrap_or(f64::NAN);
    if sink_value != 4.0 {
        fail(
            case,
            "signed additive modifier was not applied as next-step additive delta",
            evidence(vec![
                ("observed_sink_value", sink_value.to_string()),
                ("steps_executed", report.steps_executed.to_string()),
            ]),
        );
    }
}

fn check_prb013(case: &ParityFixtureCase) {
    let push_any =
        run_push_mode_scenario(ActionMode::PushAny, case, "push-any scenario should run");
    let push_all =
        run_push_mode_scenario(ActionMode::PushAll, case, "push-all scenario should run");

    let any_source =
        push_any.final_node_values.get(&NodeId::fixture("source")).copied().unwrap_or(f64::NAN);
    let all_source =
        push_all.final_node_values.get(&NodeId::fixture("source")).copied().unwrap_or(f64::NAN);
    let any_sink_total = push_any
        .final_node_values
        .iter()
        .filter(|(node_id, _)| node_id.as_str().starts_with("sink-"))
        .map(|(_, value)| *value)
        .sum::<f64>();
    let all_sink_total = push_all
        .final_node_values
        .iter()
        .filter(|(node_id, _)| node_id.as_str().starts_with("sink-"))
        .map(|(_, value)| *value)
        .sum::<f64>();

    if any_source != 0.0 || all_source != 3.0 || any_sink_total != 3.0 || all_sink_total != 0.0 {
        fail(
            case,
            "pull/push any/all semantics diverged from expected availability/full-amount behavior",
            evidence(vec![
                ("push_any_source", any_source.to_string()),
                ("push_all_source", all_source.to_string()),
                ("push_any_sink_total", any_sink_total.to_string()),
                ("push_all_sink_total", all_sink_total.to_string()),
            ]),
        );
    }
}

fn check_prb014(case: &ParityFixtureCase) {
    let upper = run_variable_case_scenario("Roll", case, "exact-case variable scenario should run");
    let lower =
        run_variable_case_scenario("roll", case, "mismatched-case variable scenario should run");

    let upper_sink =
        upper.final_node_values.get(&NodeId::fixture("sink")).copied().unwrap_or(f64::NAN);
    let lower_sink =
        lower.final_node_values.get(&NodeId::fixture("sink")).copied().unwrap_or(f64::NAN);
    if upper_sink != 2.0 || lower_sink != 0.0 {
        fail(
            case,
            "variable lookup is not case-sensitive",
            evidence(vec![
                ("upper_case_sink", upper_sink.to_string()),
                ("lower_case_sink", lower_sink.to_string()),
            ]),
        );
    }
}

fn check_prb015(case: &ParityFixtureCase) {
    let mut rng_a = rng_from_seed(2615);
    let mut rng_b = rng_from_seed(2615);
    let draws_a = (0..256)
        .map(|_| sample_closed_interval(2, 4, &mut rng_a).expect("closed interval should sample"))
        .collect::<Vec<_>>();
    let draws_b = (0..256)
        .map(|_| sample_closed_interval(2, 4, &mut rng_b).expect("closed interval should sample"))
        .collect::<Vec<_>>();

    if draws_a != draws_b {
        fail(
            case,
            "closed interval sampling is not reproducible for fixed seed",
            evidence(vec![("draw_count", draws_a.len().to_string())]),
        );
    }
    if draws_a.iter().any(|value| *value < 2.0 || *value > 4.0) {
        fail(case, "closed interval sampled value outside inclusive bounds", evidence(vec![]));
    }
    let observed = draws_a.into_iter().map(|value| value as i64).collect::<BTreeSet<_>>();
    if !(observed.contains(&2) && observed.contains(&4)) {
        fail(
            case,
            "closed interval did not include both lower and upper bounds",
            evidence(vec![("observed_values", format!("{observed:?}"))]),
        );
    }
}

fn run_push_mode_scenario(
    action_mode: ActionMode,
    case: &ParityFixtureCase,
    context: &str,
) -> anapao::types::RunReport {
    let source = NodeId::fixture("source");
    let sink_a = NodeId::fixture("sink-a");
    let sink_b = NodeId::fixture("sink-b");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture(format!(
        "parity-prb013-{}",
        match action_mode {
            ActionMode::PushAny => "any",
            ActionMode::PushAll => "all",
            ActionMode::PullAny => "pull-any",
            ActionMode::PullAll => "pull-all",
            ActionMode::Custom(_) => "custom",
        }
    )))
    .with_node(NodeSpec::new(source.clone(), NodeKind::Pool).with_initial_value(3.0).with_config(
        NodeConfig::Pool(PoolNodeConfig {
            capacity: None,
            allow_negative_start: false,
            mode: NodeModeConfig { trigger_mode: Default::default(), action_mode },
        }),
    ))
    .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Pool))
    .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Pool))
    .with_edge(EdgeSpec::new(
        EdgeId::fixture("edge-a"),
        source.clone(),
        sink_a,
        TransferSpec::Fixed { amount: 2.0 },
    ))
    .with_edge(EdgeSpec::new(
        EdgeId::fixture("edge-b"),
        source,
        sink_b,
        TransferSpec::Fixed { amount: 2.0 },
    ));
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

    let compiled = compile_ok(case, &scenario, "push mode parity scenario should compile");
    run_ok(case, &compiled, &run_config(2613, 4), context)
}

fn run_variable_case_scenario(
    expression: &str,
    case: &ParityFixtureCase,
    context: &str,
) -> anapao::types::RunReport {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");

    let mut scenario =
        ScenarioSpec::new(ScenarioId::fixture(format!("parity-prb014-{expression}")))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source,
                sink.clone(),
                TransferSpec::Expression { formula: expression.to_string() },
            ));
    scenario.variables = VariableRuntimeConfig {
        update_timing: VariableUpdateTiming::RunStart,
        sources: BTreeMap::from([(
            "Roll".to_string(),
            VariableSourceSpec::Constant { value: 2.0 },
        )]),
    };
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

    let compiled = compile_ok(case, &scenario, "variable case-sensitivity scenario should compile");
    run_ok(case, &compiled, &run_config(2614, 4), context)
}

fn run_config(seed: u64, max_steps: u64) -> RunConfig {
    RunConfig { seed, max_steps, capture: CaptureConfig::disabled() }
}

fn compile_ok(case: &ParityFixtureCase, scenario: &ScenarioSpec, detail: &str) -> CompiledScenario {
    compile_scenario(scenario).unwrap_or_else(|error| {
        fail(
            case,
            detail,
            evidence(vec![
                ("scenario_id", scenario.id.as_str().to_string()),
                ("compile_error", error.to_string()),
            ]),
        )
    })
}

fn compile_invalid_parameter(
    case: &ParityFixtureCase,
    scenario: &ScenarioSpec,
    detail: &str,
) -> SetupError {
    match compile_scenario(scenario) {
        Ok(_) => fail(
            case,
            detail,
            evidence(vec![
                ("scenario_id", scenario.id.as_str().to_string()),
                ("result", "compiled successfully".to_string()),
            ]),
        ),
        Err(error) => error,
    }
}

fn run_ok(
    case: &ParityFixtureCase,
    compiled: &CompiledScenario,
    config: &RunConfig,
    detail: &str,
) -> anapao::types::RunReport {
    run_single(compiled, config).unwrap_or_else(|error| {
        fail(
            case,
            detail,
            evidence(vec![
                ("scenario_id", compiled.scenario.id.as_str().to_string()),
                ("seed", config.seed.to_string()),
                ("max_steps", config.max_steps.to_string()),
                ("run_error", error.to_string()),
            ]),
        )
    })
}

fn run_error(
    case: &ParityFixtureCase,
    compiled: &CompiledScenario,
    config: &RunConfig,
    detail: &str,
) -> RunError {
    match run_single(compiled, config) {
        Ok(report) => fail(
            case,
            detail,
            evidence(vec![
                ("scenario_id", compiled.scenario.id.as_str().to_string()),
                ("seed", config.seed.to_string()),
                ("steps_executed", report.steps_executed.to_string()),
                ("result", "run unexpectedly succeeded".to_string()),
            ]),
        ),
        Err(error) => error,
    }
}

fn expect_invalid_parameter(
    case: &ParityFixtureCase,
    error: &SetupError,
    expected_name: &str,
    expected_reason_contains: &str,
) {
    match error {
        SetupError::InvalidParameter { name, reason } => {
            if name != expected_name || !reason.contains(expected_reason_contains) {
                fail(
                    case,
                    "invalid-parameter error mismatch",
                    evidence(vec![
                        ("expected_name", expected_name.to_string()),
                        ("actual_name", name.clone()),
                        ("expected_reason_contains", expected_reason_contains.to_string()),
                        ("actual_reason", reason.clone()),
                    ]),
                );
            }
        }
        other => fail(
            case,
            "unexpected setup error variant",
            evidence(vec![
                ("expected_variant", "InvalidParameter".to_string()),
                ("actual_error", other.to_string()),
            ]),
        ),
    }
}

fn scaled(value: f64) -> i64 {
    (value * PARITY_VALUE_SCALE).round() as i64
}

fn evidence(entries: Vec<(&str, String)>) -> BTreeMap<String, String> {
    entries.into_iter().map(|(key, value)| (key.to_string(), value)).collect::<BTreeMap<_, _>>()
}

fn fail(case: &ParityFixtureCase, detail: &str, evidence: BTreeMap<String, String>) -> ! {
    panic!("{}", format_parity_failure(case, detail, &evidence));
}
