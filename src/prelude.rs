//! Common compile/run/assert types re-exported for short import paths.
//!
//! Prefer this when a test or tool only needs [`Simulator`], core report types,
//! expectations, and event sinks without importing each submodule. `CaptureConfig` controls
//! retained run diagnostics, while `AggregationConfig` controls retained batch metric series;
//! terminal report metrics remain available under either `none()` policy.

pub use crate::assertions::{AssertionReport, Expectation, MetricSelector};
pub use crate::events::{EventSink, VecEventSink};
pub use crate::types::{
    AggregationConfig, BatchConfig, BatchReport, BatchRunTemplate, CaptureConfig, CaptureSchedule,
    EndConditionSpec, ExecutionMode, MetricKey, RunConfig, RunReport, ScenarioSpec, Selection,
    TransferSpec,
};
pub use crate::{CompiledScenario, Simulator};
