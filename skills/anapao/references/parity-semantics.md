# Parity Semantics

The parity suite enforces semantic contracts through fixture IDs and deterministic differential checks.

## Rule Families Covered

- Pool semantics: defaults, negative-state constraints, integer storage behavior.
- Converter/Trader connectivity: require inbound and outbound resource edges.
- Gate semantics: weight-unit homogeneity and trigger-only constraints.
- End-condition semantics: state-only inputs and immediate stop behavior.
- Connection semantics: resource token integer rules and state formula defaults/sign rules.
- Mode semantics: pull/push `any` versus `all`.
- Variable semantics: case sensitivity and inclusive random intervals.

## Source of Truth in Repo

- Catalog declarations: `tests/fixtures/parity/catalog.json`
- Rulebook structure checks: `tests/parity_rulebook.rs`
- Differential mapping and scenario checks: `tests/parity/differential.rs`
- Coverage handshake with rstest: `tests/rstest_testkit.rs`
- Shared parity loaders/helpers: `src/testkit/mod.rs`

## Add a New Parity Fixture Safely

1. Choose a new `fixture_id` and map it to a `rule_id`.
2. Add the fixture entry in `tests/fixtures/parity/catalog.json` with non-empty `expected.descriptor`.
3. Keep catalog ordering strict:
   - `rules` sorted by `rule_id`
   - `fixtures` sorted by `fixture_id`
4. Extend `EXPECTED_PARITY_FIXTURE_IDS` and `run_case` dispatch in `tests/parity/differential.rs`.
5. Add or update the `check_prb*` implementation to assert deterministic outcomes and structured evidence.
6. Ensure `tests/rstest_testkit.rs` parity cases remain aligned with catalog declarations.

## Invariants to Preserve

- Every fixture must reference a declared rule.
- Every declared rule must have at least one fixture.
- Mapping coverage between catalog IDs and expected differential IDs must remain exact.
- Failure paths should include deterministic evidence details (not ad-hoc strings only).

## Recommended Validation

```bash
cargo test --test parity_rulebook
cargo test --test rstest_testkit parity::
cargo test --test rstest_testkit fixture_mapping_matches_catalog
```
