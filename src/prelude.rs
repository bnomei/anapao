//! Common checked-authoring, compile/run, and assertion types for short import paths.
//!
//! Prefer this when a test or tool only needs [`Simulator`], core report types,
//! expectations, and event sinks without importing each submodule. `CaptureConfig` controls
//! retained run diagnostics, while `AggregationConfig` controls retained batch metric series;
//! terminal report metrics remain available under either `none()` policy.

pub use crate::assertions::{AssertionReport, Expectation, MetricSelector};
pub use crate::events::{EventSink, VecEventSink};
/// The one declarative checked-scenario macro. It is also available as [`crate::scenario!`].
pub use crate::scenario;
pub use crate::types::{
    AggregationConfig, BatchConfig, BatchReport, BatchRunTemplate, CaptureConfig, CaptureSchedule,
    ConnectionSpec, ConverterConfig, DelayConfig, DrainConfig, EdgeId, EndConditionSpec,
    ExecutionMode, MetricKey, MixedGateConfig, NodeBehavior, NodeId, NodeModeConfig, PoolConfig,
    QueueConfig, RegisterConfig, ResourceConnection, RunConfig, RunReport, Scenario,
    ScenarioBuilder, ScenarioEdge, ScenarioId, ScenarioNode, ScenarioSpec, Selection,
    SortingGateConfig, StateConnection, StateConnectionRole, StateTarget, TraderConfig,
    TransferSpec, TriggerGateConfig, VariableRuntimeConfig,
};
pub use crate::{CompiledScenario, Simulator};
