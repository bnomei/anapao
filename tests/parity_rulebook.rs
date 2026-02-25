use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct FixtureCatalog {
    version: u32,
    rules: Vec<RuleDeclaration>,
    fixtures: Vec<FixtureDeclaration>,
}

#[derive(Debug, Deserialize)]
struct RuleDeclaration {
    rule_id: String,
    title: String,
    source_section: String,
}

#[derive(Debug, Deserialize)]
struct FixtureDeclaration {
    fixture_id: String,
    rule_id: String,
    expected: ExpectedDeclaration,
}

#[derive(Debug, Deserialize)]
struct ExpectedDeclaration {
    descriptor: String,
}

fn load_catalog() -> FixtureCatalog {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parity/catalog.json");
    let raw = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("failed to read fixture catalog at {}: {error}", path.display())
    });

    serde_json::from_str(&raw).unwrap_or_else(|error| {
        panic!("failed to parse fixture catalog at {}: {error}", path.display())
    })
}

fn is_non_empty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn assert_strict_lexical_order(values: &[String], label: &str) {
    for pair in values.windows(2) {
        assert!(
            pair[0] < pair[1],
            "{label} must be in strictly increasing lexicographic order; found {:?} then {:?}",
            pair[0],
            pair[1]
        );
    }
}

#[cfg(test)]
mod parity_rulebook {
    use super::*;

    #[test]
    fn fixtures_have_rule_id_and_expected_outcome_descriptor() {
        let catalog = load_catalog();
        assert_eq!(catalog.version, 1, "catalog version must be pinned");
        assert!(!catalog.fixtures.is_empty(), "fixture catalog must not be empty");

        for fixture in &catalog.fixtures {
            assert!(is_non_empty(&fixture.fixture_id), "fixture_id must be non-empty");
            assert!(is_non_empty(&fixture.rule_id), "rule_id must be non-empty");
            assert!(
                is_non_empty(&fixture.expected.descriptor),
                "expected.descriptor must be non-empty for fixture {}",
                fixture.fixture_id
            );
        }
    }

    #[test]
    fn every_declared_rule_has_at_least_one_fixture() {
        let catalog = load_catalog();
        assert!(!catalog.rules.is_empty(), "declared rule list must not be empty");

        let declared_rules =
            catalog.rules.iter().map(|rule| rule.rule_id.clone()).collect::<BTreeSet<_>>();

        for rule in &catalog.rules {
            assert!(is_non_empty(&rule.rule_id), "rule_id must be non-empty");
            assert!(is_non_empty(&rule.title), "rule title must be non-empty");
            assert!(is_non_empty(&rule.source_section), "rule source_section must be non-empty");
        }

        let mut fixture_count_by_rule = BTreeMap::<String, usize>::new();
        for fixture in &catalog.fixtures {
            assert!(
                declared_rules.contains(&fixture.rule_id),
                "fixture {} references unknown rule_id {}",
                fixture.fixture_id,
                fixture.rule_id
            );

            *fixture_count_by_rule.entry(fixture.rule_id.clone()).or_insert(0) += 1;
        }

        for rule_id in &declared_rules {
            let count = fixture_count_by_rule.get(rule_id).copied().unwrap_or(0);
            assert!(count >= 1, "declared rule {rule_id} must have at least one fixture");
        }
    }

    #[test]
    fn catalog_ordering_is_deterministic() {
        let catalog = load_catalog();

        let rule_ids = catalog.rules.iter().map(|rule| rule.rule_id.clone()).collect::<Vec<_>>();
        assert_strict_lexical_order(&rule_ids, "rules.rule_id");

        let fixture_ids =
            catalog.fixtures.iter().map(|fixture| fixture.fixture_id.clone()).collect::<Vec<_>>();
        assert_strict_lexical_order(&fixture_ids, "fixtures.fixture_id");
    }
}
