use anapao::{
    prelude::CompiledScenario as PreludeCompiledScenario, testkit, CompiledScenario, Simulator,
};

fn compile_through_try_from(spec: anapao::ScenarioSpec) -> CompiledScenario {
    spec.try_into().expect("fixture scenario should compile through TryFrom")
}

#[test]
fn compiled_scenario_is_available_from_root_and_prelude_with_frozen_accessors() {
    let compiled = Simulator::compile(testkit::fixture_scenario()).expect("fixture should compile");
    let _: &PreludeCompiledScenario = &compiled;

    assert_eq!(compiled.scenario_id().as_str(), "scenario-testkit");
    assert_eq!(compiled.source_spec().id, *compiled.scenario_id());
    assert_eq!(compiled.node_ids().len(), compiled.node_count());
    assert_eq!(compiled.edge_ids().len(), compiled.edge_count());
}

#[test]
fn checked_try_from_builds_the_same_public_compilation_product() {
    let compiled = compile_through_try_from(testkit::fixture_scenario());
    let report = Simulator::run(&compiled, &testkit::deterministic_run_config())
        .expect("TryFrom product should execute through the facade");

    assert!(report.completed);
}
