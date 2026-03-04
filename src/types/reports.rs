use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{EdgeId, ExecutionMode, ManifestRef, MetricKey, NodeId, ScenarioId};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Captured node values at a specific simulation step.
pub struct NodeSnapshot {
    pub step: u64,
    pub values: BTreeMap<NodeId, f64>,
}

impl NodeSnapshot {
    /// Creates an empty node snapshot for a step.
    pub fn new(step: u64) -> Self {
        Self { step, values: BTreeMap::new() }
    }

    /// Adds or replaces one node value in the snapshot.
    pub fn with_value(mut self, node_id: NodeId, value: f64) -> Self {
        self.values.insert(node_id, value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Captured runtime-variable values at a specific simulation step.
pub struct VariableSnapshot {
    pub step: u64,
    pub values: BTreeMap<String, f64>,
}

impl VariableSnapshot {
    /// Creates an empty variable snapshot for a step.
    pub fn new(step: u64) -> Self {
        Self { step, values: BTreeMap::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// One metric observation at a simulation step.
pub struct SeriesPoint {
    pub step: u64,
    pub value: f64,
}

impl SeriesPoint {
    /// Creates a series point from `(step, value)`.
    pub fn new(step: u64, value: f64) -> Self {
        Self { step, value }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Ordered metric time series for one metric key.
pub struct SeriesTable {
    pub metric: MetricKey,
    pub points: Vec<SeriesPoint>,
}

impl SeriesTable {
    /// Creates an empty series table for a metric.
    pub fn new(metric: MetricKey) -> Self {
        Self { metric, points: Vec::new() }
    }

    /// Appends one point and keeps points sorted by step.
    pub fn with_point(mut self, point: SeriesPoint) -> Self {
        self.points.push(point);
        self.points.sort_by_key(|p| p.step);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Result of one deterministic simulation run.
pub struct RunReport {
    pub scenario_id: ScenarioId,
    pub seed: u64,
    pub steps_executed: u64,
    pub completed: bool,
    pub node_snapshots: Vec<NodeSnapshot>,
    pub variable_snapshots: Vec<VariableSnapshot>,
    pub transfers: Vec<TransferRecord>,
    pub series: BTreeMap<MetricKey, SeriesTable>,
    pub final_node_values: BTreeMap<NodeId, f64>,
    pub final_metrics: BTreeMap<MetricKey, f64>,
    pub manifest: Option<ManifestRef>,
}

impl RunReport {
    /// Creates an empty run report initialized with scenario id and seed.
    pub fn new(scenario_id: ScenarioId, seed: u64) -> Self {
        Self {
            scenario_id,
            seed,
            steps_executed: 0,
            completed: false,
            node_snapshots: Vec::new(),
            variable_snapshots: Vec::new(),
            transfers: Vec::new(),
            series: BTreeMap::new(),
            final_node_values: BTreeMap::new(),
            final_metrics: BTreeMap::new(),
            manifest: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Reduced per-run payload used by batch reports.
pub struct BatchRunSummary {
    pub run_index: u64,
    pub seed: u64,
    pub completed: bool,
    pub steps_executed: u64,
    pub final_metrics: BTreeMap<MetricKey, f64>,
    pub manifest: Option<ManifestRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Transfer record emitted by the engine for one edge evaluation.
pub struct TransferRecord {
    pub step: u64,
    pub edge_id: EdgeId,
    pub from_node_id: NodeId,
    pub to_node_id: NodeId,
    pub requested_amount: f64,
    pub transferred_amount: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Aggregated output of a multi-run batch execution.
pub struct BatchReport {
    pub scenario_id: ScenarioId,
    /// Number of runs requested in the input batch configuration.
    pub requested_runs: u64,
    /// Number of run summaries present in `runs` (not the count of `run.completed == true`).
    pub completed_runs: u64,
    pub execution_mode: ExecutionMode,
    pub runs: Vec<BatchRunSummary>,
    pub aggregate_series: BTreeMap<MetricKey, SeriesTable>,
    pub manifest: Option<ManifestRef>,
}

impl BatchReport {
    /// Creates an empty batch report shell.
    pub fn new(
        scenario_id: ScenarioId,
        requested_runs: u64,
        execution_mode: ExecutionMode,
    ) -> Self {
        Self {
            scenario_id,
            requested_runs,
            completed_runs: 0,
            execution_mode,
            runs: Vec::new(),
            aggregate_series: BTreeMap::new(),
            manifest: None,
        }
    }

    /// Appends a run summary and refreshes deterministic ordering/counters.
    pub fn push_run(mut self, run: BatchRunSummary) -> Self {
        self.runs.push(run);
        self.runs.sort_by_key(|entry| entry.run_index);
        self.completed_runs = self.runs.len() as u64;
        self
    }
}
