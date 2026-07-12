#[test]
fn scenario_macro_ui() {
    let cases = trybuild::TestCases::new();
    cases.pass("tests/ui/scenario_macro/pass/exact_example.rs");
    cases.pass("tests/ui/scenario_macro/pass/full_surface.rs");
    cases.pass("tests/ui/scenario_macro/pass/trailing_separators.rs");
    cases.pass("tests/ui/scenario_macro/pass/hygiene_no_prelude.rs");
    cases.pass("tests/ui/scenario_macro/pass/prelude_import.rs");
    cases.pass("tests/ui/scenario_macro/pass/crate_alias.rs");
    cases.pass("tests/ui/scenario_macro/pass/one_evaluation.rs");

    cases.compile_fail("tests/ui/scenario_macro/fail/unknown_family.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/unknown_node_field.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/wrong_family_field.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/mixed_config.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/duplicate_scalar.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/malformed_transfer.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/malformed_state_target.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/malformed_end.rs");
    cases.compile_fail("tests/ui/scenario_macro/fail/unexpected_top_level.rs");
}
