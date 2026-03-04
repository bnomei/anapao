//! Deterministic fixtures and parity helpers for tests and docs.

pub mod pikmin;

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

use crate::error::SetupError;
use crate::types::{
    BatchConfig, CaptureConfig, EdgeId, EdgeSpec, EndConditionSpec, ExecutionMode, MetricKey,
    NodeId, NodeKind, NodeSpec, RunConfig, ScenarioId, ScenarioSpec, TransferSpec,
};
use crate::validation::{compile_scenario, CompiledScenario};

/// Default seed used by deterministic run fixtures.
pub const FIXTURE_RUN_SEED: u64 = 42;
/// Default max-step budget used by deterministic run fixtures.
pub const FIXTURE_RUN_MAX_STEPS: u64 = 10;
/// Default run count used by deterministic batch fixtures.
pub const FIXTURE_BATCH_RUNS: u64 = 6;
/// Default base seed used by deterministic batch fixtures.
pub const FIXTURE_BATCH_BASE_SEED: u64 = 0x000A_11CE_55ED_u64;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Root document for parity fixture metadata.
pub struct ParityCatalog {
    pub version: u32,
    pub rules: Vec<ParityRuleDeclaration>,
    pub fixtures: Vec<ParityFixtureDeclaration>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Rule declaration linked by parity fixtures.
pub struct ParityRuleDeclaration {
    pub rule_id: String,
    pub title: String,
    pub source_section: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Fixture declaration loaded from the parity catalog.
pub struct ParityFixtureDeclaration {
    pub fixture_id: String,
    pub rule_id: String,
    pub expected: ParityExpectedDeclaration,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
/// Expected descriptor payload for a parity fixture.
pub struct ParityExpectedDeclaration {
    pub descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Fully-resolved parity case with rule metadata attached.
pub struct ParityFixtureCase {
    pub fixture_id: String,
    pub rule_id: String,
    pub title: String,
    pub source_section: String,
    pub descriptor: String,
}

/// Builds a deterministic source->sink scenario suitable for engine/batch tests.
pub fn fixture_scenario() -> ScenarioSpec {
    let source = NodeId::fixture("source");
    let sink = NodeId::fixture("sink");
    let sink_metric = MetricKey::fixture("sink");

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-testkit"))
        .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
        .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("edge-source-sink"),
            source,
            sink,
            TransferSpec::Fixed { amount: 1.0 },
        ));
    scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
    scenario.tracked_metrics.insert(sink_metric);
    scenario
}

/// Compiles the default fixture scenario into deterministic node/edge indexes.
pub fn fixture_compiled_scenario() -> Result<CompiledScenario, SetupError> {
    compile_scenario(&fixture_scenario())
}

/// Build a run config fixture with explicit seed/max steps.
pub fn fixture_run_config(seed: u64, max_steps: u64) -> RunConfig {
    RunConfig { seed, max_steps, capture: CaptureConfig::default() }
}

/// Build a deterministic run config fixture.
pub fn deterministic_run_config() -> RunConfig {
    fixture_run_config(FIXTURE_RUN_SEED, FIXTURE_RUN_MAX_STEPS)
}

/// Build a batch config fixture with deterministic defaults.
pub fn fixture_batch_config(
    runs: u64,
    base_seed: u64,
    execution_mode: ExecutionMode,
) -> BatchConfig {
    BatchConfig {
        runs,
        base_seed,
        execution_mode,
        run: fixture_run_config(FIXTURE_RUN_SEED, FIXTURE_RUN_MAX_STEPS),
    }
}

/// Build a deterministic sequential batch config fixture.
pub fn deterministic_batch_config() -> BatchConfig {
    fixture_batch_config(FIXTURE_BATCH_RUNS, FIXTURE_BATCH_BASE_SEED, ExecutionMode::SingleThread)
}

/// Return the canonical fixture-catalog path used by parity tests.
pub fn parity_catalog_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/catalog.json")
}

fn invalid_parity_catalog(reason: impl Into<String>) -> SetupError {
    SetupError::InvalidParameter {
        name: "tests.fixtures.parity.catalog".to_string(),
        reason: reason.into(),
    }
}

fn parity_is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Load and parse the parity fixture catalog with deterministic schema checks.
pub fn load_parity_catalog() -> Result<ParityCatalog, SetupError> {
    let path = parity_catalog_path();
    let raw = fs::read_to_string(&path).map_err(|error| {
        invalid_parity_catalog(format!("failed to read {}: {error}", path.display()))
    })?;
    let catalog: ParityCatalog = serde_json::from_str(&raw).map_err(|error| {
        invalid_parity_catalog(format!("failed to parse {}: {error}", path.display()))
    })?;

    if catalog.version != 1 {
        return Err(invalid_parity_catalog(format!(
            "unsupported catalog version {}; expected 1",
            catalog.version
        )));
    }

    Ok(catalog)
}

/// Build deterministic fixture cases (joined with rule metadata), sorted by fixture_id.
pub fn parity_fixture_cases() -> Result<Vec<ParityFixtureCase>, SetupError> {
    let catalog = load_parity_catalog()?;
    let rule_index =
        catalog.rules.iter().map(|rule| (rule.rule_id.clone(), rule)).collect::<BTreeMap<_, _>>();

    let mut cases = Vec::with_capacity(catalog.fixtures.len());
    for fixture in &catalog.fixtures {
        if !parity_is_non_empty(&fixture.fixture_id) {
            return Err(invalid_parity_catalog("fixture_id must be non-empty"));
        }
        if !parity_is_non_empty(&fixture.rule_id) {
            return Err(invalid_parity_catalog(format!(
                "fixture {} has empty rule_id",
                fixture.fixture_id
            )));
        }
        if !parity_is_non_empty(&fixture.expected.descriptor) {
            return Err(invalid_parity_catalog(format!(
                "fixture {} has empty expected.descriptor",
                fixture.fixture_id
            )));
        }

        let Some(rule) = rule_index.get(&fixture.rule_id) else {
            return Err(invalid_parity_catalog(format!(
                "fixture {} references unknown rule_id {}",
                fixture.fixture_id, fixture.rule_id
            )));
        };
        if !parity_is_non_empty(&rule.title) || !parity_is_non_empty(&rule.source_section) {
            return Err(invalid_parity_catalog(format!(
                "rule {} is missing title/source_section",
                rule.rule_id
            )));
        }

        cases.push(ParityFixtureCase {
            fixture_id: fixture.fixture_id.clone(),
            rule_id: fixture.rule_id.clone(),
            title: rule.title.clone(),
            source_section: rule.source_section.clone(),
            descriptor: fixture.expected.descriptor.clone(),
        });
    }

    cases.sort_by(|left, right| left.fixture_id.cmp(&right.fixture_id));
    Ok(cases)
}

/// Resolve one parity fixture case by id.
pub fn parity_fixture_case(fixture_id: &str) -> Result<ParityFixtureCase, SetupError> {
    parity_fixture_cases()?
        .into_iter()
        .find(|case| case.fixture_id == fixture_id)
        .ok_or_else(|| invalid_parity_catalog(format!("unknown fixture_id {fixture_id}")))
}

/// Format a parity assertion failure with fixture metadata and ordered evidence.
pub fn format_parity_failure(
    case: &ParityFixtureCase,
    detail: &str,
    evidence: &BTreeMap<String, String>,
) -> String {
    format!(
        "parity differential failure: fixture_id={} rule_id={} title={} expected={} detail={} evidence={:?}",
        case.fixture_id, case.rule_id, case.title, case.descriptor, detail, evidence
    )
}

/// Reusable fixture entrypoint for rstest tests.
#[cfg_attr(test, rstest::fixture)]
pub fn scenario_fixture() -> ScenarioSpec {
    fixture_scenario()
}

/// Reusable compiled fixture entrypoint for rstest tests.
#[cfg_attr(test, rstest::fixture)]
pub fn compiled_scenario_fixture() -> CompiledScenario {
    fixture_compiled_scenario().expect("fixture scenario should compile")
}

/// Reusable run config fixture entrypoint for rstest tests.
#[cfg_attr(test, rstest::fixture)]
pub fn run_config_fixture() -> RunConfig {
    deterministic_run_config()
}

/// Reusable batch config fixture entrypoint for rstest tests.
#[cfg_attr(test, rstest::fixture)]
pub fn batch_config_fixture() -> BatchConfig {
    deterministic_batch_config()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use super::{
        batch_config_fixture, compiled_scenario_fixture, deterministic_batch_config,
        deterministic_run_config, fixture_batch_config, fixture_compiled_scenario,
        fixture_scenario, invalid_parity_catalog, parity_catalog_path, parity_fixture_case,
        parity_fixture_cases, parity_is_non_empty, run_config_fixture, scenario_fixture,
        FIXTURE_BATCH_BASE_SEED, FIXTURE_BATCH_RUNS, FIXTURE_RUN_MAX_STEPS, FIXTURE_RUN_SEED,
    };
    use crate::error::SetupError;
    use crate::types::{ExecutionMode, MetricKey};

    #[test]
    fn deterministic_config_fixtures_use_documented_defaults() {
        let run = deterministic_run_config();
        assert_eq!(run.seed, FIXTURE_RUN_SEED);
        assert_eq!(run.max_steps, FIXTURE_RUN_MAX_STEPS);
        assert_eq!(run.capture.every_n_steps, 1);
        assert!(run.capture.include_step_zero);
        assert!(run.capture.include_final_state);

        let batch = deterministic_batch_config();
        assert_eq!(batch.runs, FIXTURE_BATCH_RUNS);
        assert_eq!(batch.base_seed, FIXTURE_BATCH_BASE_SEED);
        assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
        assert_eq!(batch.run, run);
    }

    #[test]
    fn fixture_batch_config_uses_requested_inputs() {
        let batch = fixture_batch_config(9, 123, ExecutionMode::Rayon);
        assert_eq!(batch.runs, 9);
        assert_eq!(batch.base_seed, 123);
        assert_eq!(batch.execution_mode, ExecutionMode::Rayon);
        assert_eq!(batch.run.seed, FIXTURE_RUN_SEED);
        assert_eq!(batch.run.max_steps, FIXTURE_RUN_MAX_STEPS);
    }

    #[test]
    fn fixture_scenario_has_expected_nodes_edges_and_end_condition() {
        let scenario = fixture_scenario();
        assert_eq!(scenario.nodes.len(), 2);
        assert_eq!(scenario.edges.len(), 1);
        assert_eq!(scenario.end_conditions.len(), 1);
        assert_eq!(
            scenario.end_conditions[0],
            crate::types::EndConditionSpec::MaxSteps { steps: 3 }
        );
        assert!(scenario.tracked_metrics.contains(&MetricKey::fixture("sink")));
    }

    #[test]
    fn parity_fixture_cases_are_sorted_and_non_empty() {
        let cases = parity_fixture_cases().expect("catalog should parse");
        assert!(!cases.is_empty());
        assert!(cases.windows(2).all(|pair| pair[0].fixture_id <= pair[1].fixture_id));
    }

    #[test]
    fn parity_fixture_case_resolves_known_fixture() {
        let case = parity_fixture_case("fx-001-pool-default-zero").expect("fixture must exist");
        assert_eq!(case.rule_id, "PRB-001");
        assert!(case.title.contains("Pool"));
        assert!(case.descriptor.contains("zero"));
    }

    #[test]
    fn parity_fixture_case_rejects_unknown_fixture_id() {
        let err = parity_fixture_case("fx-999-missing").expect_err("missing fixture should fail");
        match err {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "tests.fixtures.parity.catalog");
                assert!(reason.contains("unknown fixture_id fx-999-missing"));
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn format_parity_failure_contains_fixture_metadata_and_evidence() {
        let case = parity_fixture_case("fx-002-pool-negative-via-state-only")
            .expect("fixture must exist for formatting test");
        let evidence = BTreeMap::from([
            ("actual".to_string(), "allowed".to_string()),
            ("step".to_string(), "3".to_string()),
        ]);

        let message = super::format_parity_failure(&case, "behavior mismatch", &evidence);
        assert!(message.contains("fixture_id=fx-002-pool-negative-via-state-only"));
        assert!(message.contains("rule_id=PRB-002"));
        assert!(message.contains("detail=behavior mismatch"));
        assert!(message.contains("evidence={"));
    }

    #[test]
    fn parity_helpers_validate_empty_and_error_shape() {
        assert!(parity_is_non_empty("x"));
        assert!(!parity_is_non_empty(""));
        assert!(!parity_is_non_empty("   "));

        let err = invalid_parity_catalog("boom");
        assert!(matches!(
            err,
            SetupError::InvalidParameter { name, reason }
                if name == "tests.fixtures.parity.catalog" && reason == "boom"
        ));
    }

    #[test]
    fn parity_catalog_path_points_to_fixture_file() {
        let path = parity_catalog_path();
        assert!(path.ends_with(Path::new("tests/fixtures/parity/catalog.json")));
        assert!(path.is_file());
    }

    #[test]
    fn fixture_and_wrapper_entrypoints_are_consistent() {
        let direct_compiled = fixture_compiled_scenario().expect("fixture should compile");
        let wrapped_compiled = compiled_scenario_fixture();
        assert_eq!(direct_compiled, wrapped_compiled);

        assert_eq!(scenario_fixture(), fixture_scenario());
        assert_eq!(run_config_fixture(), deterministic_run_config());
        assert_eq!(batch_config_fixture(), deterministic_batch_config());
    }
}
