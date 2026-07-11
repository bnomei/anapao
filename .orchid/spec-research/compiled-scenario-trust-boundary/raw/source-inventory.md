# Local Source Inventory

Run date: 2026-07-11

Frigg MCP was unavailable for this delegated task. Shell `rg` plus bounded numbered reads were the
required fallback.

## Primary Discovery Commands

```text
git status --short
rg --files src tests benches specs .orchid | sort
rg -n "CompiledScenario|compile_scenario|compile_formula|ExpressionCache|StepPlan|expect\(" src tests benches
rg -n "compiled\.(scenario|node_order|edge_order|node_index_by_id|edge_index_by_id|metric_index_by_name)" src tests benches README.md
rg -n "validate_formula\(" src/validation/mod.rs
rg -n "EngineExpressionCache|EngineStepPlan" src/engine/mod.rs tests benches
rg -n "edge_index_by_id" src tests benches
rg -n "metric_index_by_name" src tests benches
rg -n "anapao::(validation|engine|batch)|use anapao::.*(CompiledScenario|compile_scenario|run_single|run_batch)" README.md benches tests src/testkit
rg -n "pub (fn|struct|enum|trait|type|const|mod)" src/validation/mod.rs src/engine/mod.rs src/batch/mod.rs src/expr/mod.rs
```

## High-Signal Results

```text
src/validation/mod.rs:25:pub struct CompiledScenario {
src/validation/mod.rs:39:pub fn compile_scenario(spec: &ScenarioSpec) -> Result<CompiledScenario, SetupError> {
src/validation/mod.rs:595:fn validate_formula(name: String, formula: &str) -> Result<(), SetupError> {
src/engine/mod.rs:86:struct EngineExpressionCache {
src/engine/mod.rs:92:    fn from_compiled(compiled: &CompiledScenario, runtime: &ExprRuntime) -> Result<Self, RunError> {
src/engine/mod.rs:471:pub fn init_state(compiled: &CompiledScenario) -> EngineState {
src/engine/mod.rs:499:pub fn run_single(compiled: &CompiledScenario, config: &RunConfig) -> Result<RunReport, RunError> {
src/engine/mod.rs:541:    let expression_cache = EngineExpressionCache::from_compiled(compiled, &runtime)?;
src/engine/mod.rs:542:    let step_plan = EngineStepPlan::from_compiled(compiled);
src/engine/mod.rs:747:struct EngineStepPlan {
src/batch/mod.rs:29:pub fn run_batch(
src/types/identifiers.rs:21:macro_rules! define_identifier {
src/lib.rs:127:pub mod batch;
src/lib.rs:128:pub mod engine;
src/lib.rs:139:pub mod validation;
README.md:79:assert_eq!(compiled.scenario.id.as_str(), "scenario-source-sink");
benches/simulation.rs:27:use anapao::validation::CompiledScenario;
tests/perf_determinism.rs:3:use anapao::batch::run_batch;
tests/perf_determinism.rs:4:use anapao::engine::run_single;
tests/perf_determinism.rs:12:use anapao::validation::{compile_scenario, CompiledScenario};
tests/rstest_testkit.rs:1:use anapao::batch::run_batch;
tests/rstest_testkit.rs:2:use anapao::engine::run_single;
tests/parity/differential.rs:3:use anapao::engine::run_single;
tests/parity/differential.rs:17:use anapao::validation::{compile_scenario, CompiledScenario};
```

`edge_index_by_id` was found only at construction and in its validation unit test. The metric-name
index had one engine consumer. The direct compiled-field consumer search identified README,
benchmark, Pikmin testkit, parity diagnostics, and README drift tests.
