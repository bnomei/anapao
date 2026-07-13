use std::fs;

use anapao::error::SetupError;
use anapao::types::{
    AggregationConfig, BatchConfig, BatchRunTemplate, CaptureConfig, CaptureSchedule, EdgeId,
    EndConditionSpec, ExecutionMode, MetricKey, NodeId, ResourceConnection, RunConfig,
    ScenarioBuilder, ScenarioEdge, ScenarioId, ScenarioNode, ScenarioSpec, StateConnection,
    StateConnectionRole, StateTarget, TransferSpec,
};
use anapao::Simulator;

#[test]
fn macro_root_and_prelude_paths_match_direct_checked_builder() {
    let macro_scenario = anapao::scenario! {
        id: "macro-paths";
        nodes {
            source: Source { initial: 2.0 };
            sink: Sink;
        }
        edges {
            flow: source -> sink => remaining;
        }
        track [sink];
        end max_steps(2);
    }
    .expect("root macro path builds");

    let direct_scenario = ScenarioBuilder::new(ScenarioId::fixture("macro-paths"))
        .with_node(ScenarioNode::source(NodeId::fixture("source")).with_initial_value(2.0))
        .expect("add source")
        .with_node(ScenarioNode::sink(NodeId::fixture("sink")))
        .expect("add sink")
        .with_edge(ScenarioEdge::resource(
            EdgeId::fixture("flow"),
            NodeId::fixture("source"),
            NodeId::fixture("sink"),
            TransferSpec::Remaining,
            ResourceConnection::default(),
        ))
        .expect("add flow")
        .with_tracked_metric(MetricKey::fixture("sink"))
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 2 })
        .build()
        .expect("direct checked scenario builds");

    assert_eq!(macro_scenario.source_spec(), direct_scenario.source_spec());

    use anapao::prelude::*;
    let prelude_scenario = scenario! {
        id: "prelude-path";
        nodes { source: Source; sink: Sink; }
        edges { flow: source -> sink => remaining; }
    }
    .expect("prelude macro path builds");
    assert_eq!(prelude_scenario.id().as_str(), "prelude-path");
}

#[test]
fn macro_setup_errors_are_handled_as_results() {
    let result = anapao::scenario! {
        id: "handled-error";
        nodes { source: Source; }
        edges { flow: source -> missing => remaining; }
    };

    match result {
        Err(SetupError::InvalidGraphReference { graph, reference }) => {
            assert_eq!(graph, "scenario[handled-error].nodes");
            assert!(reference.contains("missing"));
        }
        Err(error) => panic!("unexpected setup error: {error}"),
        Ok(_) => panic!("missing node must be a recoverable setup error"),
    }
}

#[test]
fn readme_s01_build_minimal_scenario() {
    let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });
    scenario.tracked_metrics.insert(MetricKey::fixture("sink"));

    assert_eq!(scenario.nodes.len(), 2);
    assert_eq!(scenario.edges.len(), 1);
    assert!(scenario.tracked_metrics.contains(&MetricKey::fixture("sink")));
}

#[test]
fn readme_s02_compile_scenario() {
    let scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });

    let compiled = Simulator::compile(scenario).expect("compile source_sink scenario");
    assert_eq!(compiled.scenario_id().as_str(), "scenario-source-sink");
}

#[test]
fn readme_checked_authoring_compiles_runs_and_uses_opaque_accessors() {
    let source = NodeId::fixture("source");
    let pool = NodeId::fixture("pool");
    let sink = NodeId::fixture("sink");
    let scenario = ScenarioBuilder::new(ScenarioId::fixture("checked-authoring"))
        .with_title("Checked authoring")
        .with_description("resource and state flow")
        .with_tag("docs")
        .with_node(ScenarioNode::source(source.clone()).with_initial_value(2.0))
        .expect("add source")
        .with_node(ScenarioNode::pool(pool.clone(), Default::default()).with_label("buffer"))
        .expect("add pool")
        .with_node(ScenarioNode::sink(sink.clone()))
        .expect("add sink")
        .with_edge(ScenarioEdge::resource(
            EdgeId::fixture("source-pool"),
            source.clone(),
            pool.clone(),
            TransferSpec::Fixed { amount: 1.0 },
            ResourceConnection::default(),
        ))
        .expect("add source resource edge")
        .with_edge(ScenarioEdge::resource(
            EdgeId::fixture("pool-sink"),
            pool.clone(),
            sink,
            TransferSpec::Remaining,
            ResourceConnection::default(),
        ))
        .expect("add sink resource edge")
        .with_edge(ScenarioEdge::state(
            EdgeId::fixture("source-pool-state"),
            source,
            pool,
            TransferSpec::Remaining,
            StateConnection::new(StateConnectionRole::Modifier, "+1", StateTarget::Node),
        ))
        .expect("add state edge")
        .with_end_condition(EndConditionSpec::MaxSteps { steps: 2 })
        .with_tracked_metric(MetricKey::fixture("sink"))
        .with_metadata("owner", "docs")
        .build()
        .expect("checked scenario builds");

    let compiled = Simulator::compile_checked(scenario).expect("checked scenario compiles");
    assert_eq!(compiled.scenario_id().as_str(), "checked-authoring");
    assert_eq!(compiled.source_spec().title.as_deref(), Some("Checked authoring"));
    assert_eq!(compiled.node_count(), 3);
    assert_eq!(compiled.edge_count(), 3);
    assert_eq!(compiled.node_ids().len(), 3);
    assert_eq!(compiled.edge_ids().len(), 3);

    let report = Simulator::run(&compiled, &RunConfig::for_seed(39)).expect("scenario runs");
    assert!(report.completed);
}

#[test]
fn readme_s03_create_deterministic_run_config() {
    let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(
        CaptureConfig::default().with_schedule(CaptureSchedule::Every {
            stride: std::num::NonZeroU64::new(5).expect("positive stride"),
            include_initial: true,
            include_final: true,
        }),
    );

    assert_eq!(run.seed, 42);
    assert_eq!(run.max_steps, 250);
    assert!(matches!(
        run.capture.schedule(),
        CaptureSchedule::Every { stride, .. } if stride.get() == 5
    ));
}

#[test]
fn readme_s07_create_batch_config() {
    let batch = BatchConfig::for_runs(64)
        .with_execution_mode(ExecutionMode::SingleThread)
        .with_base_seed(7)
        .with_run_template(BatchRunTemplate::default())
        .with_max_steps(50);

    assert_eq!(batch.runs, 64);
    assert_eq!(batch.base_seed, 7);
    assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
    assert_eq!(batch.run_template.max_steps, 50);
}

#[test]
fn readme_batch_aggregation_is_separate_from_diagnostic_capture() {
    let batch = BatchConfig::for_runs(64)
        .with_execution_mode(ExecutionMode::SingleThread)
        .with_aggregation(AggregationConfig::default().with_schedule(CaptureSchedule::Final));

    assert!(matches!(batch.run_template.aggregation.schedule(), CaptureSchedule::Final));
}

#[test]
fn readme_linear_pipeline_convenience_constructor_compiles_and_runs() {
    let compiled =
        Simulator::compile(ScenarioSpec::linear_pipeline(4)).expect("compile linear pipeline");
    let run =
        Simulator::run(&compiled, &RunConfig::for_seed(42)).expect("run linear pipeline scenario");

    assert!(run.completed);
    assert!(run.final_metrics.contains_key(&MetricKey::fixture("sink")));
}

#[test]
fn readme_contains_curated_builder_snippet_signatures() {
    let path = format!("{}/README.md", env!("CARGO_MANIFEST_DIR"));
    let readme = fs::read_to_string(&path).expect("read README");

    for needle in [
        "## Scenario Representations and the Validation Boundary",
        "let checked = Scenario::try_from(dto).unwrap();",
        "let scenario = ScenarioBuilder::new(ScenarioId::fixture(\"checked-authoring\"))",
        "let compiled = Simulator::compile_checked(scenario)?;",
        "assert_eq!(compiled.source_spec().title.as_deref(), Some(\"Checked authoring\"));",
        "Raw JSON lexical duplicate keys are not detected",
        "### Snippet S01 — Build a Minimal Scenario",
        "let mut scenario = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 })",
        "### Snippet S02 — Compile a Scenario",
        "assert_eq!(compiled.scenario_id().as_str(), \"scenario-source-sink\");",
        "### Snippet S03 — Create a Deterministic RunConfig",
        "CaptureConfig::default().with_schedule(CaptureSchedule::Every {",
        "### Snippet S07 — Create BatchConfig",
        "let batch = BatchConfig::for_runs(64)",
        "Batch aggregation is separate from per-run diagnostic capture.",
        "`CaptureConfig::none()` leaves `RunReport::final_node_values` and `RunReport::final_metrics`",
        "monotonic-series assertions, and series probability assertions require captured or aggregated",
        "`CaptureConfig::{none, final_only, default}`",
        "## Declarative `scenario!` Authoring",
        "let scenario = anapao::scenario! {",
        "contains exactly one macro: `scenario!`. There are no `expectations!`",
        "`Result<Scenario, SetupError>`",
    ] {
        assert!(readme.contains(needle), "README drift: missing snippet marker `{needle}`");
    }
}
