use anapao::batch::run_batch;
use anapao::engine::run_single;
use anapao::rng::derive_run_seed;
use anapao::testkit::{
    batch_config_fixture as testkit_batch_config_fixture,
    compiled_scenario_fixture as testkit_compiled_scenario_fixture, parity_fixture_cases,
    run_config_fixture as testkit_run_config_fixture, scenario_fixture as testkit_scenario_fixture,
};
use anapao::types::{BatchConfig, RunConfig, ScenarioSpec};
use anapao::validation::CompiledScenario;
use rstest::{fixture, rstest};

#[path = "parity/differential.rs"]
mod parity_differential;

#[fixture]
fn scenario() -> ScenarioSpec {
    testkit_scenario_fixture()
}

#[fixture]
fn compiled_scenario() -> CompiledScenario {
    testkit_compiled_scenario_fixture()
}

#[fixture]
fn run_config() -> RunConfig {
    testkit_run_config_fixture()
}

#[fixture]
fn batch_config() -> BatchConfig {
    testkit_batch_config_fixture()
}

#[fixture]
fn parity_fixture_ids() -> Vec<String> {
    parity_fixture_cases()
        .expect("parity fixture catalog should load")
        .into_iter()
        .map(|case| case.fixture_id)
        .collect()
}

#[rstest]
fn deterministic_single_run(
    scenario: ScenarioSpec,
    compiled_scenario: CompiledScenario,
    run_config: RunConfig,
) {
    let report_a = run_single(&compiled_scenario, &run_config).expect("run should succeed");
    let report_b = run_single(&compiled_scenario, &run_config).expect("run should succeed");

    assert_eq!(report_a, report_b);
    assert_eq!(report_a.scenario_id, scenario.id);
    assert_eq!(report_a.seed, run_config.seed);
    assert_eq!(report_a.steps_executed, 3);
    assert!(report_a.completed);
}

#[rstest]
fn deterministic_batch_run(compiled_scenario: CompiledScenario, batch_config: BatchConfig) {
    let report_a = run_batch(&compiled_scenario, &batch_config).expect("batch run should succeed");
    let report_b = run_batch(&compiled_scenario, &batch_config).expect("batch run should succeed");

    assert_eq!(report_a, report_b);
    assert_eq!(report_a.requested_runs, batch_config.runs);
    assert_eq!(report_a.completed_runs, batch_config.runs);
    assert_eq!(report_a.runs.len() as u64, batch_config.runs);

    for (expected_run_index, run) in report_a.runs.iter().enumerate() {
        let run_index = expected_run_index as u64;
        assert_eq!(run.run_index, run_index);
        assert_eq!(run.seed, derive_run_seed(batch_config.base_seed, run_index));
    }
}

#[rstest]
fn reports_satisfy_basic_run_and_batch_invariants(
    compiled_scenario: CompiledScenario,
    run_config: RunConfig,
    batch_config: BatchConfig,
) {
    let run_report = run_single(&compiled_scenario, &run_config).expect("run should succeed");
    assert!(run_report.steps_executed <= run_config.max_steps);
    assert!(!run_report.node_snapshots.is_empty());
    assert_eq!(run_report.node_snapshots.first().map(|snapshot| snapshot.step), Some(0));
    assert!(run_report.node_snapshots.windows(2).all(|window| window[0].step < window[1].step));
    assert!(run_report.final_node_values.values().all(|value| value.is_finite() && *value >= 0.0));

    let batch_report =
        run_batch(&compiled_scenario, &batch_config).expect("batch run should succeed");
    let expected_indices = (0_u64..batch_config.runs).collect::<Vec<_>>();
    let actual_indices = batch_report.runs.iter().map(|run| run.run_index).collect::<Vec<_>>();

    assert_eq!(actual_indices, expected_indices);
    assert!(batch_report.runs.iter().all(|run| run.steps_executed <= batch_config.run.max_steps));
    assert!(batch_report
        .runs
        .iter()
        .flat_map(|run| run.final_metrics.values())
        .all(|value| value.is_finite()));

    for tracked_metric in &compiled_scenario.scenario.tracked_metrics {
        assert!(batch_report.aggregate_series.contains_key(tracked_metric));
        assert!(batch_report.runs.iter().all(|run| run.final_metrics.contains_key(tracked_metric)));
    }

    for table in batch_report.aggregate_series.values() {
        assert!(table.points.windows(2).all(|window| window[0].step < window[1].step));
    }
}

mod parity {
    use super::{parity_differential, parity_fixture_ids};
    use rstest::rstest;

    #[rstest]
    #[case("fx-001-pool-default-zero")]
    #[case("fx-002-pool-negative-via-state-only")]
    #[case("fx-003-pool-negative-blocks-output")]
    #[case("fx-004-pool-integer-storage")]
    #[case("fx-005-converter-input-output-required")]
    #[case("fx-006-gate-weight-unit-homogeneity")]
    #[case("fx-007-trigger-gate-state-only")]
    #[case("fx-008-end-condition-state-input-only")]
    #[case("fx-009-end-condition-immediate-stop")]
    #[case("fx-010-resource-token-positive-integer")]
    #[case("fx-011-state-connection-default-plus-one")]
    #[case("fx-012-formula-modifier-additive-next-step")]
    #[case("fx-013-pull-push-any-all-behavior")]
    #[case("fx-014-variable-case-sensitivity")]
    #[case("fx-015-random-interval-inclusive")]
    fn fixture_differential_rule_check(#[case] fixture_id: &str, parity_fixture_ids: Vec<String>) {
        assert!(
            parity_fixture_ids.iter().any(|candidate| candidate == fixture_id),
            "fixture id `{fixture_id}` is not declared in tests/fixtures/parity/catalog.json"
        );
        parity_differential::assert_fixture_case(fixture_id);
    }

    #[rstest]
    fn fixture_mapping_matches_catalog(parity_fixture_ids: Vec<String>) {
        let mut catalog_ids = parity_fixture_ids;
        catalog_ids.sort();

        let mut mapped_ids = parity_differential::expected_fixture_ids();
        mapped_ids.sort();

        assert_eq!(
            mapped_ids, catalog_ids,
            "parity fixture mapping must match catalog fixture ids"
        );
        parity_differential::assert_fixture_mapping_coverage();
    }
}
