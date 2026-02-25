//! Event model and sink traits for run/batch diagnostics.
//!
//! This module defines typed run events, deterministic ordering helpers,
//! and sink abstractions used by simulator and artifact layers.

use std::cmp::Ordering;
use std::collections::BTreeMap;

use crate::types::{DiagnosticSeverity, EdgeId, MetricKey, NodeId};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Canonical phase ordering for one run step.
pub enum RunEventPhase {
    StepStart,
    NodeUpdate,
    Transfer,
    MetricSnapshot,
    AssertionCheckpoint,
    StepEnd,
    Debug,
    Violation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
/// Stable ordering key for a single event in a run timeline.
pub struct RunEventOrder {
    pub run_id: String,
    pub step: u64,
    pub phase: RunEventPhase,
    pub ordinal: u64,
}

impl RunEventOrder {
    /// Creates a stable ordering key from run id, step, phase, and ordinal.
    pub fn new(run_id: impl Into<String>, step: u64, phase: RunEventPhase, ordinal: u64) -> Self {
        Self { run_id: run_id.into(), step, phase, ordinal }
    }
}

impl RunEventPhase {
    /// Returns the serialized snake_case name for the phase.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StepStart => "step_start",
            Self::NodeUpdate => "node_update",
            Self::Transfer => "transfer",
            Self::MetricSnapshot => "metric_snapshot",
            Self::AssertionCheckpoint => "assertion_checkpoint",
            Self::StepEnd => "step_end",
            Self::Debug => "debug",
            Self::Violation => "violation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Step boundary event payload emitted at step start.
pub struct StepStartEvent {
    pub seed: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Step boundary event payload emitted at step end.
pub struct StepEndEvent {
    pub completed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Node value transition payload for a simulation step.
pub struct NodeUpdateEvent {
    pub node_id: NodeId,
    pub previous_value: f64,
    pub next_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Edge transfer payload for one evaluated edge.
pub struct TransferEvent {
    pub edge_id: EdgeId,
    pub from_node_id: NodeId,
    pub to_node_id: NodeId,
    pub requested_amount: f64,
    pub transferred_amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Metric value payload captured for reporting.
pub struct MetricSnapshotEvent {
    pub metric: MetricKey,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Assertion evaluation payload emitted as a checkpoint event.
pub struct AssertionCheckpointEvent {
    pub checkpoint_id: String,
    pub passed: bool,
    pub expected: String,
    pub actual: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Free-form debug payload for engine diagnostics.
pub struct DebugEvent {
    pub topic: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Structured rule or invariant violation payload.
pub struct ViolationEvent {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub evidence: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
/// Tagged run event envelope with typed payloads.
pub enum RunEvent {
    StepStart { order: RunEventOrder, payload: StepStartEvent },
    StepEnd { order: RunEventOrder, payload: StepEndEvent },
    NodeUpdate { order: RunEventOrder, payload: NodeUpdateEvent },
    Transfer { order: RunEventOrder, payload: TransferEvent },
    MetricSnapshot { order: RunEventOrder, payload: MetricSnapshotEvent },
    AssertionCheckpoint { order: RunEventOrder, payload: AssertionCheckpointEvent },
    Debug { order: RunEventOrder, payload: DebugEvent },
    Violation { order: RunEventOrder, payload: ViolationEvent },
}

impl RunEvent {
    /// Builds a `step_start` event.
    pub fn step_start(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: StepStartEvent,
    ) -> Self {
        Self::StepStart {
            order: RunEventOrder::new(run_id, step, RunEventPhase::StepStart, ordinal),
            payload,
        }
    }

    /// Builds a `step_end` event.
    pub fn step_end(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: StepEndEvent,
    ) -> Self {
        Self::StepEnd {
            order: RunEventOrder::new(run_id, step, RunEventPhase::StepEnd, ordinal),
            payload,
        }
    }

    /// Builds a `node_update` event.
    pub fn node_update(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: NodeUpdateEvent,
    ) -> Self {
        Self::NodeUpdate {
            order: RunEventOrder::new(run_id, step, RunEventPhase::NodeUpdate, ordinal),
            payload,
        }
    }

    /// Builds a `transfer` event.
    pub fn transfer(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: TransferEvent,
    ) -> Self {
        Self::Transfer {
            order: RunEventOrder::new(run_id, step, RunEventPhase::Transfer, ordinal),
            payload,
        }
    }

    /// Builds a `metric_snapshot` event.
    pub fn metric_snapshot(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: MetricSnapshotEvent,
    ) -> Self {
        Self::MetricSnapshot {
            order: RunEventOrder::new(run_id, step, RunEventPhase::MetricSnapshot, ordinal),
            payload,
        }
    }

    /// Builds an `assertion_checkpoint` event.
    pub fn assertion_checkpoint(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: AssertionCheckpointEvent,
    ) -> Self {
        Self::AssertionCheckpoint {
            order: RunEventOrder::new(run_id, step, RunEventPhase::AssertionCheckpoint, ordinal),
            payload,
        }
    }

    /// Builds a `debug` event.
    pub fn debug(run_id: impl Into<String>, step: u64, ordinal: u64, payload: DebugEvent) -> Self {
        Self::Debug {
            order: RunEventOrder::new(run_id, step, RunEventPhase::Debug, ordinal),
            payload,
        }
    }

    /// Builds a `violation` event.
    pub fn violation(
        run_id: impl Into<String>,
        step: u64,
        ordinal: u64,
        payload: ViolationEvent,
    ) -> Self {
        Self::Violation {
            order: RunEventOrder::new(run_id, step, RunEventPhase::Violation, ordinal),
            payload,
        }
    }

    /// Returns the ordering key for this event.
    pub fn order(&self) -> &RunEventOrder {
        match self {
            Self::StepStart { order, .. }
            | Self::StepEnd { order, .. }
            | Self::NodeUpdate { order, .. }
            | Self::Transfer { order, .. }
            | Self::MetricSnapshot { order, .. }
            | Self::AssertionCheckpoint { order, .. }
            | Self::Debug { order, .. }
            | Self::Violation { order, .. } => order,
        }
    }

    /// Returns the snake_case event tag.
    pub fn event_name(&self) -> &'static str {
        match self {
            Self::StepStart { .. } => "step_start",
            Self::StepEnd { .. } => "step_end",
            Self::NodeUpdate { .. } => "node_update",
            Self::Transfer { .. } => "transfer",
            Self::MetricSnapshot { .. } => "metric_snapshot",
            Self::AssertionCheckpoint { .. } => "assertion_checkpoint",
            Self::Debug { .. } => "debug",
            Self::Violation { .. } => "violation",
        }
    }

    /// Returns severity metadata when the event represents diagnostics.
    pub fn diagnostic_marker(&self) -> Option<(DiagnosticSeverity, Option<&str>)> {
        match self {
            Self::Debug { .. } => Some((DiagnosticSeverity::Debug, None)),
            Self::Violation { payload, .. } => {
                Some((payload.severity, Some(payload.code.as_str())))
            }
            _ => None,
        }
    }
}

/// Sorts events by deterministic run/step/phase/ordinal order.
pub fn sort_events_by_order(events: &mut [RunEvent]) {
    events.sort_by(|left, right| compare_event_order(left.order(), right.order()));
}

fn compare_event_order(left: &RunEventOrder, right: &RunEventOrder) -> Ordering {
    compare_run_id(&left.run_id, &right.run_id)
        .then_with(|| left.step.cmp(&right.step))
        .then_with(|| left.phase.cmp(&right.phase))
        .then_with(|| left.ordinal.cmp(&right.ordinal))
}

fn compare_run_id(left: &str, right: &str) -> Ordering {
    match (parse_run_index(left), parse_run_index(right)) {
        (Some(left_index), Some(right_index)) => {
            left_index.cmp(&right_index).then_with(|| left.cmp(right))
        }
        _ => left.cmp(right),
    }
}

fn parse_run_index(run_id: &str) -> Option<u64> {
    run_id.strip_prefix("run-")?.parse::<u64>().ok()
}

#[derive(Debug, Error)]
/// Event sink failures surfaced by streaming adapters.
pub enum EventSinkError {
    #[error("event sink io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("event sink serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("event sink custom error ({sink}): {message}")]
    Custom { sink: String, message: String },
}

impl EventSinkError {
    /// Creates a custom sink error with sink label and message.
    pub fn custom(sink: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Custom { sink: sink.into(), message: message.into() }
    }
}

/// Consumer trait for run events emitted by simulator workflows.
pub trait EventSink {
    /// Pushes one event into the sink.
    fn push(&mut self, event: RunEvent) -> Result<(), EventSinkError>;

    /// Flushes pending buffered events, if needed.
    fn flush(&mut self) -> Result<(), EventSinkError> {
        Ok(())
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
/// In-memory sink collecting events in insertion order.
pub struct VecEventSink {
    events: Vec<RunEvent>,
}

impl VecEventSink {
    /// Creates an empty in-memory sink.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a borrowed view of collected events.
    pub fn events(&self) -> &[RunEvent] {
        self.events.as_slice()
    }

    /// Consumes the sink and returns the collected events.
    pub fn into_events(self) -> Vec<RunEvent> {
        self.events
    }
}

impl EventSink for VecEventSink {
    fn push(&mut self, event: RunEvent) -> Result<(), EventSinkError> {
        self.events.push(event);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sort_events_by_order_is_deterministic() {
        let mut events = vec![
            RunEvent::step_end("run-a", 1, 3, StepEndEvent { completed: true }),
            RunEvent::transfer(
                "run-a",
                1,
                1,
                TransferEvent {
                    edge_id: EdgeId::fixture("edge-1"),
                    from_node_id: NodeId::fixture("node-a"),
                    to_node_id: NodeId::fixture("node-b"),
                    requested_amount: 2.0,
                    transferred_amount: 1.5,
                },
            ),
            RunEvent::metric_snapshot(
                "run-a",
                1,
                5,
                MetricSnapshotEvent { metric: MetricKey::fixture("m-total"), value: 10.0 },
            ),
            RunEvent::step_start("run-a", 1, 0, StepStartEvent { seed: 42 }),
            RunEvent::node_update(
                "run-a",
                1,
                2,
                NodeUpdateEvent {
                    node_id: NodeId::fixture("node-a"),
                    previous_value: 1.0,
                    next_value: 0.0,
                },
            ),
            RunEvent::step_start("run-a", 0, 0, StepStartEvent { seed: 42 }),
            RunEvent::step_start("run-b", 0, 0, StepStartEvent { seed: 7 }),
        ];

        sort_events_by_order(&mut events);

        let ordered_keys = events.iter().map(|event| event.order().clone()).collect::<Vec<_>>();
        assert_eq!(
            ordered_keys,
            vec![
                RunEventOrder::new("run-a", 0, RunEventPhase::StepStart, 0),
                RunEventOrder::new("run-a", 1, RunEventPhase::StepStart, 0),
                RunEventOrder::new("run-a", 1, RunEventPhase::NodeUpdate, 2),
                RunEventOrder::new("run-a", 1, RunEventPhase::Transfer, 1),
                RunEventOrder::new("run-a", 1, RunEventPhase::MetricSnapshot, 5),
                RunEventOrder::new("run-a", 1, RunEventPhase::StepEnd, 3),
                RunEventOrder::new("run-b", 0, RunEventPhase::StepStart, 0),
            ]
        );
    }

    #[test]
    fn sort_events_orders_debug_and_violation_after_step_end_phase() {
        let mut events = vec![
            RunEvent::violation(
                "run-a",
                1,
                4,
                ViolationEvent {
                    severity: DiagnosticSeverity::Warning,
                    code: "RULE-1".to_string(),
                    message: "soft failure".to_string(),
                    evidence: BTreeMap::from([("node".to_string(), "node-a".to_string())]),
                },
            ),
            RunEvent::debug(
                "run-a",
                1,
                3,
                DebugEvent {
                    topic: "engine".to_string(),
                    message: "intermediate value".to_string(),
                    fields: BTreeMap::from([("value".to_string(), "2.0".to_string())]),
                },
            ),
            RunEvent::step_end("run-a", 1, 2, StepEndEvent { completed: true }),
            RunEvent::step_start("run-a", 1, 0, StepStartEvent { seed: 11 }),
        ];

        sort_events_by_order(&mut events);
        let names = events.iter().map(RunEvent::event_name).collect::<Vec<_>>();
        assert_eq!(names, vec!["step_start", "step_end", "debug", "violation"]);
    }

    #[test]
    fn sort_events_orders_numeric_run_ids_naturally() {
        let mut events = vec![
            RunEvent::step_start("run-10", 0, 0, StepStartEvent { seed: 10 }),
            RunEvent::step_start("run-2", 0, 0, StepStartEvent { seed: 2 }),
        ];
        sort_events_by_order(&mut events);
        let run_ids = events.iter().map(|event| event.order().run_id.as_str()).collect::<Vec<_>>();
        assert_eq!(run_ids, vec!["run-2", "run-10"]);
    }

    #[test]
    fn violation_event_round_trips_with_typed_payload_and_marker() {
        let event = RunEvent::violation(
            "run-a",
            2,
            9,
            ViolationEvent {
                severity: DiagnosticSeverity::Error,
                code: "FLOW-42".to_string(),
                message: "invalid transfer".to_string(),
                evidence: BTreeMap::from([
                    ("edge_id".to_string(), "edge-1".to_string()),
                    ("requested".to_string(), "5".to_string()),
                ]),
            },
        );

        let encoded = serde_json::to_string(&event).expect("violation event should serialize");
        let decoded: RunEvent =
            serde_json::from_str(&encoded).expect("violation event should deserialize");
        assert_eq!(decoded, event);

        let marker = decoded.diagnostic_marker().expect("violation marker");
        assert_eq!(marker.0, DiagnosticSeverity::Error);
        assert_eq!(marker.1, Some("FLOW-42"));
    }

    #[test]
    fn vec_event_sink_collects_events_in_order() {
        let mut sink = VecEventSink::new();

        sink.push(RunEvent::step_start("run-a", 0, 0, StepStartEvent { seed: 5 }))
            .expect("step start should be accepted");
        sink.push(RunEvent::step_end("run-a", 0, 1, StepEndEvent { completed: false }))
            .expect("step end should be accepted");
        sink.flush().expect("flush should be a no-op success for VecEventSink");

        let events = sink.into_events();
        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], RunEvent::StepStart { .. }));
        assert!(matches!(events[1], RunEvent::StepEnd { .. }));
    }

    #[test]
    fn event_sink_error_wraps_io_serialization_and_custom_errors() {
        let io_error = std::io::Error::new(std::io::ErrorKind::Other, "disk full");
        let sink_error = EventSinkError::from(io_error);
        assert!(matches!(sink_error, EventSinkError::Io(_)));

        let serde_error = serde_json::from_str::<RunEventOrder>("not-json")
            .expect_err("invalid JSON should create a serde error");
        let sink_error = EventSinkError::from(serde_error);
        assert!(matches!(sink_error, EventSinkError::Serialization(_)));

        let sink_error = EventSinkError::custom("fixture_sink", "write failed");
        assert!(matches!(
            sink_error,
            EventSinkError::Custom { sink, message }
            if sink == "fixture_sink" && message == "write failed"
        ));
    }
}
