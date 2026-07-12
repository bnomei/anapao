use anapao::error::SetupError;
use anapao::types::{
    CaptureConfig, EdgeId, EdgeSpec, EndConditionSpec, MetricKey, NodeId, NodeKind, NodeSpec,
    ResourceConnection, RunConfig, Scenario, ScenarioBuilder, ScenarioEdge, ScenarioId,
    ScenarioNode, ScenarioSpec, StateConnection, StateConnectionRole, StateTarget, TransferSpec,
};
use anapao::{CompiledScenario, Simulator};
use serde_json::Value;

const LEGACY_DEFAULT_RESOURCE: &str =
    include_str!("fixtures/scenario-wire-v1/legacy-default-resource.json");
const LEGACY_STATE_ALIASES: &str =
    include_str!("fixtures/scenario-wire-v1/legacy-state-aliases.json");
const NODE_CONFIG_MISMATCH: &str =
    include_str!("fixtures/scenario-wire-v1/node-config-mismatch.json");
const MAP_KEY_ID_MISMATCH: &str =
    include_str!("fixtures/scenario-wire-v1/map-key-id-mismatch.json");

fn assert_invalid_parameter_path(error: SetupError, expected: &str) {
    assert!(
        matches!(error, SetupError::InvalidParameter { ref name, .. } if name == expected),
        "expected invalid parameter at {expected}, got {error}"
    );
}

#[test]
fn legacy_wire_fixtures_normalize_before_checked_round_trip() {
    for (name, fixture) in [
        ("legacy-default-resource", LEGACY_DEFAULT_RESOURCE),
        ("legacy-state-aliases", LEGACY_STATE_ALIASES),
    ] {
        let raw: Value = serde_json::from_str(fixture).expect("fixture is valid JSON");
        let parsed: ScenarioSpec =
            serde_json::from_str(fixture).expect("legacy fixture deserializes as ScenarioSpec");
        let parsed_json = serde_json::to_value(&parsed).expect("parsed DTO serializes");
        assert_ne!(raw, parsed_json, "{name} must demonstrate serde normalization");

        let checked = Scenario::try_from(parsed.clone()).expect("legacy fixture checks");
        assert_eq!(
            parsed_json,
            serde_json::to_value(ScenarioSpec::from(&checked)).expect("checked DTO serializes"),
            "{name} must preserve the parsed DTO's semantic serialization"
        );
        Simulator::compile(parsed.clone()).expect("legacy DTO compiles");
        Simulator::compile_checked(checked).expect("legacy checked scenario compiles");
    }
}

#[test]
fn invalid_wire_fixtures_separate_serde_from_semantic_checks() {
    let config_mismatch: ScenarioSpec =
        serde_json::from_str(NODE_CONFIG_MISMATCH).expect("wrong-family config is valid wire JSON");
    assert_invalid_parameter_path(
        Scenario::try_from(config_mismatch.clone()).expect_err("wrong-family config must fail"),
        "nodes.source.config",
    );
    assert_invalid_parameter_path(
        Simulator::compile(config_mismatch).expect_err("compile shares semantic checking"),
        "nodes.source.config",
    );

    let key_mismatch: ScenarioSpec =
        serde_json::from_str(MAP_KEY_ID_MISMATCH).expect("key drift is valid wire JSON");
    assert_invalid_parameter_path(
        Scenario::try_from(key_mismatch.clone()).expect_err("node key drift must fail"),
        "nodes.declared.id",
    );
    assert_invalid_parameter_path(
        Simulator::compile(key_mismatch).expect_err("compile shares key validation"),
        "nodes.declared.id",
    );
}

fn process() -> ScenarioNode {
    ScenarioNode::process(NodeId::fixture("process")).with_initial_value(2.0)
}

fn sink() -> ScenarioNode {
    ScenarioNode::sink(NodeId::fixture("sink"))
}

fn resource_edge(formula: &str) -> ScenarioEdge {
    ScenarioEdge::resource(
        EdgeId::fixture("resource"),
        NodeId::fixture("process"),
        NodeId::fixture("sink"),
        TransferSpec::Expression { formula: formula.into() },
        ResourceConnection::default(),
    )
}

fn state_edge(formula: &str) -> ScenarioEdge {
    ScenarioEdge::state(
        EdgeId::fixture("state"),
        NodeId::fixture("process"),
        NodeId::fixture("sink"),
        TransferSpec::Remaining,
        StateConnection::new(StateConnectionRole::Modifier, formula, StateTarget::Node),
    )
}

fn finish_builder(builder: ScenarioBuilder) -> Result<Scenario, SetupError> {
    builder
        .with_title("Checked authoring parity")
        .with_description("DTO and checked paths share behavior")
        .with_tag("compatibility")
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 2 })
        .with_tracked_metric(MetricKey::fixture("sink"))
        .with_metadata("spec", "039")
        .build()
}

#[test]
fn checked_builder_styles_compile_and_run_identically_through_every_public_path() {
    let mut mutable = ScenarioBuilder::new(ScenarioId::fixture("checked-public-parity"));
    mutable.insert_node(process()).expect("insert process");
    mutable.insert_node(sink()).expect("insert sink");
    mutable.insert_edge(resource_edge("1")).expect("insert resource expression");
    mutable.insert_edge(state_edge("+1")).expect("insert state expression");
    let mutable = finish_builder(mutable).expect("mutable builder checks");

    let consuming = finish_builder(
        ScenarioBuilder::new(ScenarioId::fixture("checked-public-parity"))
            .with_node(process())
            .expect("add process")
            .with_node(sink())
            .expect("add sink")
            .with_edge(resource_edge("1"))
            .expect("add resource expression")
            .with_edge(state_edge("+1"))
            .expect("add state expression"),
    )
    .expect("consuming builder checks");
    assert_eq!(ScenarioSpec::from(&mutable), ScenarioSpec::from(&consuming));

    let dto = ScenarioSpec::from(&mutable);
    let checked_try_from = Scenario::try_from(dto.clone()).expect("DTO TryFrom checks");
    assert_eq!(ScenarioSpec::from(&mutable), ScenarioSpec::from(&checked_try_from));

    let compiled = [
        Simulator::compile(dto.clone()).expect("DTO facade compiles"),
        Simulator::compile_checked(mutable.clone()).expect("checked facade compiles"),
        CompiledScenario::try_from(dto).expect("DTO TryFrom compiles"),
        CompiledScenario::try_from(consuming).expect("checked TryFrom compiles"),
    ];
    let config =
        RunConfig::for_seed(39).with_max_steps(2).with_capture(CaptureConfig::final_only());
    let reports = compiled
        .iter()
        .map(|scenario| Simulator::run(scenario, &config).expect("seeded run succeeds"))
        .collect::<Vec<_>>();
    for report in &reports[1..] {
        assert_eq!(
            report, &reports[0],
            "all public compile paths must produce a full equal report"
        );
    }
    assert!(
        reports[0].final_node_values[&NodeId::fixture("sink")] > 2.0,
        "resource and state expressions must both affect the evaluated result"
    );
}

#[test]
fn dto_and_checked_builders_report_identical_formula_error_paths() {
    for (edge, expected_path) in [
        (resource_edge("("), "edges.resource.transfer.expression.formula"),
        (state_edge("1"), "edges.state.connection.state.formula"),
    ] {
        let edge_id = edge.id().clone();
        let dto = ScenarioSpec::new(ScenarioId::fixture("invalid-formula"))
            .with_node(NodeSpec::new(NodeId::fixture("process"), NodeKind::Process))
            .with_node(NodeSpec::new(NodeId::fixture("sink"), NodeKind::Sink))
            .with_edge(edge.clone().into());
        let dto_error = Simulator::compile(dto).expect_err("invalid DTO formula must fail");

        let checked_error = ScenarioBuilder::new(ScenarioId::fixture("invalid-formula"))
            .with_node(process())
            .expect("add process")
            .with_node(sink())
            .expect("add sink")
            .with_edge(edge)
            .expect("add edge")
            .build()
            .expect_err("invalid checked formula must fail");
        assert_eq!(dto_error.to_string(), checked_error.to_string(), "edge {edge_id}");
        assert_invalid_parameter_path(dto_error, expected_path);
        assert_invalid_parameter_path(checked_error, expected_path);
    }
}

#[test]
fn checked_duplicates_retain_first_while_legacy_helpers_replace() {
    let process_id = NodeId::fixture("process");
    let sink_id = NodeId::fixture("sink");
    let edge_id = EdgeId::fixture("resource");

    let mut mutable = ScenarioBuilder::new(ScenarioId::fixture("checked-duplicates"));
    let first_node = process().with_label("first");
    mutable.insert_node(first_node.clone()).expect("first node inserts");
    assert_invalid_parameter_path(
        mutable
            .insert_node(ScenarioNode::custom(process_id.clone(), "replacement"))
            .expect_err("duplicate node is rejected"),
        "nodes.process",
    );
    mutable.insert_node(sink()).expect("sink inserts");
    let first_edge = resource_edge("1").with_metadata("definition", "first");
    mutable.insert_edge(first_edge.clone()).expect("first edge inserts");
    assert_invalid_parameter_path(
        mutable
            .insert_edge(resource_edge("2").with_enabled(false))
            .expect_err("duplicate edge is rejected"),
        "edges.resource",
    );
    let retained = mutable.build().expect("retained first definitions remain valid");
    assert_eq!(&retained.nodes()[&process_id], &first_node);
    assert_eq!(&retained.edges()[&edge_id], &first_edge);

    let consuming_error = ScenarioBuilder::new(ScenarioId::fixture("consuming-duplicate"))
        .with_node(process())
        .expect("first node inserts")
        .with_node(ScenarioNode::custom(process_id.clone(), "replacement"))
        .expect_err("consuming duplicate is rejected");
    assert_invalid_parameter_path(consuming_error, "nodes.process");

    let replacement_node =
        NodeSpec::new(process_id.clone(), NodeKind::Process).with_initial_value(7.0);
    let replacement_edge = EdgeSpec::new(
        edge_id.clone(),
        process_id.clone(),
        sink_id.clone(),
        TransferSpec::Fixed { amount: 2.0 },
    );
    let legacy = ScenarioSpec::new(ScenarioId::fixture("legacy-replaces"))
        .with_node(NodeSpec::new(process_id.clone(), NodeKind::Process))
        .with_node(replacement_node.clone())
        .with_node(NodeSpec::new(sink_id.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            edge_id.clone(),
            process_id,
            sink_id,
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(replacement_edge.clone());
    assert_eq!(legacy.nodes[&replacement_node.id], replacement_node);
    assert_eq!(legacy.edges[&edge_id], replacement_edge);
}
