//! Common compile/run/assert types re-exported for short import paths.
//!
//! Prefer this when a test or tool only needs [`Simulator`], core report types,
//! expectations, and event sinks without importing each submodule.

pub use crate::assertions::{AssertionReport, Expectation, MetricSelector};
pub use crate::events::{EventSink, VecEventSink};
pub use crate::types::{
    BatchConfig, BatchReport, BatchRunTemplate, CaptureConfig, CaptureSchedule, EndConditionSpec,
    ExecutionMode, MetricKey, RunConfig, RunReport, ScenarioSpec, Selection, TransferSpec,
};
pub use crate::{CompiledScenario, Simulator};
