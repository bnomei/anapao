//! Step engine that advances a compiled scenario under a seeded run config.
//!
//! Owns per-run state (node values, metrics, variables, delays/queues), evaluates
//! edges and expressions, applies capacity/backpressure rules, samples stochastic
//! variable and gate draws from salted RNGs, and optionally streams ordered run
//! events. Callers should prefer [`crate::Simulator`]; this module is the
//! execution core used by single-run and batch paths.

use std::{
    collections::{BTreeMap, BTreeSet},
    num::NonZeroU64,
};

use crate::error::RunError;
use crate::events::{
    EventSink, EventSinkError, MetricSnapshotEvent, RunEvent, StepEndEvent, StepStartEvent,
    TransferEvent,
};
use crate::expr::{CompiledExpr, ExprError, ExprRuntime};
use crate::plan::{
    CompiledEdge, CompiledTransfer, EdgeIndex, NodeIndex, TransferControl, TriggerTarget,
};
use crate::rng::{rng_from_seed, BaseRng};
use crate::stochastic::{
    sample_chance_percent, sample_closed_interval, sample_from_list, sample_from_matrix,
    sample_weighted_index,
};
use crate::types::{
    AggregationConfig, CaptureConfig, CaptureSchedule, EdgeId, EndConditionSpec, ManifestRef,
    MetricKey, NodeBehavior, NodeId, NodeModeConfig, NodeSnapshot, RunConfig, RunReport, Selection,
    SeriesPoint, SeriesTable, StateConnectionRole, StateTarget, TransferRecord, TriggerMode,
    VariableSnapshot, VariableSourceSpec, VariableUpdateTiming,
};
use crate::CompiledScenario;

const VALUE_SCALE: f64 = 1_000_000.0;
const VARIABLE_RNG_SALT: u64 = 0xA11C_E5E0_0023_0001;
const GATE_RNG_SALT: u64 = 0xA11C_E5E0_0023_0002;

#[derive(Debug, Clone, PartialEq)]
/// Mutable step-local inventory for one run: step counter, node values, metrics.
pub(crate) struct EngineState {
    pub step: u64,
    pub node_values: Vec<f64>,
    pub metrics: BTreeMap<MetricKey, f64>,
}

/// Private retention sink for the single shared simulation transition loop.
///
/// It owns the full single-run report shape while keeping capture policy decisions
/// separate from event emission. Transfer records are retained only when selected;
/// each step's transient records remain available long enough for live events.
struct FullReportCollector<'a> {
    capture: &'a CaptureConfig,
    report: RunReport,
    captured_steps: BTreeSet<u64>,
    step_transfers: Vec<TransferRecord>,
    emit_transfers: bool,
}

trait TransferCollector {
    fn wants_transfer_records(&self) -> bool;
    fn record_transfer(&mut self, transfer: TransferRecord);
}

trait RunCollector: TransferCollector {
    type Output;

    fn capture_step(
        &mut self,
        compiled: &CompiledScenario,
        state: &EngineState,
        runtime_variables: &BTreeMap<String, f64>,
        force: bool,
    );
    fn captures_final(&self) -> bool;
    fn take_step_transfers(&mut self) -> Vec<TransferRecord>;
    fn finish(
        self,
        compiled: &CompiledScenario,
        state: &EngineState,
        completed: bool,
    ) -> Self::Output;
}

impl<'a> FullReportCollector<'a> {
    fn new(compiled: &CompiledScenario, config: &'a RunConfig, emit_transfers: bool) -> Self {
        Self {
            capture: &config.capture,
            report: RunReport::new(compiled.scenario_id().clone(), config.seed),
            captured_steps: BTreeSet::new(),
            step_transfers: Vec::new(),
            emit_transfers,
        }
    }
}

impl RunCollector for FullReportCollector<'_> {
    type Output = RunReport;

    fn capture_step(
        &mut self,
        compiled: &CompiledScenario,
        state: &EngineState,
        runtime_variables: &BTreeMap<String, f64>,
        force: bool,
    ) {
        if !should_capture_step(self.capture, state.step, force)
            || !self.captured_steps.insert(state.step)
        {
            return;
        }

        let mut snapshot = NodeSnapshot::new(state.step);
        for (index, node_id) in compiled.node_ids().iter().enumerate() {
            if selected(self.capture.nodes(), node_id) {
                snapshot
                    .values
                    .insert(node_id.clone(), canonicalize_float(state.node_values[index]));
            }
        }
        if !snapshot.values.is_empty() {
            self.report.node_snapshots.push(snapshot);
        }

        if !runtime_variables.is_empty() && !self.capture.variables().is_none() {
            let mut snapshot = VariableSnapshot::new(state.step);
            for (name, value) in runtime_variables {
                if selected(self.capture.variables(), name) {
                    snapshot.values.insert(name.clone(), canonicalize_float(*value));
                }
            }
            if !snapshot.values.is_empty() {
                self.report.variable_snapshots.push(snapshot);
            }
        }

        match self.capture.metrics() {
            Selection::None => {}
            Selection::All => {
                for (metric, value) in &state.metrics {
                    let table = self
                        .report
                        .series
                        .entry(metric.clone())
                        .or_insert_with(|| SeriesTable::new(metric.clone()));
                    table.points.push(SeriesPoint::new(state.step, canonicalize_float(*value)));
                }
            }
            Selection::Only(metrics) => {
                for metric in metrics {
                    let value = metric_value(compiled, state, metric);
                    let table = self
                        .report
                        .series
                        .entry(metric.clone())
                        .or_insert_with(|| SeriesTable::new(metric.clone()));
                    table.points.push(SeriesPoint::new(state.step, canonicalize_float(value)));
                }
            }
        }
    }

    fn captures_final(&self) -> bool {
        captures_final(self.capture.schedule())
    }

    fn take_step_transfers(&mut self) -> Vec<TransferRecord> {
        std::mem::take(&mut self.step_transfers)
    }

    fn finish(
        mut self,
        compiled: &CompiledScenario,
        state: &EngineState,
        completed: bool,
    ) -> RunReport {
        self.report.steps_executed = state.step;
        self.report.completed = completed;
        self.report.final_node_values = compiled
            .node_ids()
            .iter()
            .enumerate()
            .map(|(index, node_id)| (node_id.clone(), canonicalize_float(state.node_values[index])))
            .collect();
        self.report.final_metrics = state.metrics.clone();
        self.report
    }
}

impl TransferCollector for FullReportCollector<'_> {
    fn wants_transfer_records(&self) -> bool {
        self.emit_transfers || !self.capture.transfers().is_none()
    }

    fn record_transfer(&mut self, transfer: TransferRecord) {
        if selected(self.capture.transfers(), &transfer.edge_id) {
            self.report.transfers.push(transfer.clone());
        }
        self.step_transfers.push(transfer);
    }
}

/// Private compact batch result produced by the shared engine loop.
///
/// Unlike [`RunReport`], this deliberately has no snapshots, transfers, or
/// terminal node map. Batch aggregation needs only final metadata plus the
/// explicitly requested metric observations.
#[derive(Debug)]
pub(crate) struct BatchSample {
    pub(crate) seed: u64,
    pub(crate) completed: bool,
    pub(crate) steps_executed: u64,
    pub(crate) final_metrics: BTreeMap<MetricKey, f64>,
    pub(crate) manifest: Option<ManifestRef>,
    pub(crate) series: BTreeMap<MetricKey, SeriesTable>,
}

/// Batch-only metric selection resolved from the immutable compiled plan before
/// any seed starts. Keeping this private prevents the compiled-plan lookup from
/// leaking into the per-step compact collector path.
#[derive(Debug)]
pub(crate) enum ResolvedAggregationSelection {
    None,
    All,
    Only(Vec<ResolvedAggregationMetric>),
}

#[derive(Debug)]
pub(crate) struct ResolvedAggregationMetric {
    metric: MetricKey,
    source: ResolvedAggregationMetricSource,
}

#[derive(Debug)]
enum ResolvedAggregationMetricSource {
    Node(usize),
    TrackedTotal,
}

impl ResolvedAggregationMetric {
    fn value(&self, state: &EngineState) -> f64 {
        match self.source {
            ResolvedAggregationMetricSource::Node(index) => {
                canonicalize_float(state.node_values[index])
            }
            ResolvedAggregationMetricSource::TrackedTotal => total_node_value(state),
        }
    }
}

/// Private compact collector for batch aggregation.
///
/// It is intentionally separate from `FullReportCollector`: batch execution
/// never allocates report-only diagnostics that it would immediately discard.
struct BatchSampleCollector<'a> {
    aggregation: &'a AggregationConfig,
    metrics: &'a ResolvedAggregationSelection,
    last_captured_step: Option<u64>,
    series: BTreeMap<MetricKey, SeriesTable>,
    seed: u64,
}

impl<'a> BatchSampleCollector<'a> {
    fn new(
        aggregation: &'a AggregationConfig,
        metrics: &'a ResolvedAggregationSelection,
        seed: u64,
    ) -> Self {
        Self { aggregation, metrics, last_captured_step: None, series: BTreeMap::new(), seed }
    }
}

impl RunCollector for BatchSampleCollector<'_> {
    type Output = BatchSample;

    fn capture_step(
        &mut self,
        _compiled: &CompiledScenario,
        state: &EngineState,
        _runtime_variables: &BTreeMap<String, f64>,
        force: bool,
    ) {
        if !should_capture_aggregation_step(self.aggregation, state.step, force)
            || self.last_captured_step == Some(state.step)
        {
            return;
        }
        self.last_captured_step = Some(state.step);

        match self.metrics {
            ResolvedAggregationSelection::None => {}
            ResolvedAggregationSelection::All => {
                for (metric, value) in &state.metrics {
                    let table = self
                        .series
                        .entry(metric.clone())
                        .or_insert_with(|| SeriesTable::new(metric.clone()));
                    table.points.push(SeriesPoint::new(state.step, canonicalize_float(*value)));
                }
            }
            ResolvedAggregationSelection::Only(metrics) => {
                for metric in metrics {
                    let table = self
                        .series
                        .entry(metric.metric.clone())
                        .or_insert_with(|| SeriesTable::new(metric.metric.clone()));
                    table.points.push(SeriesPoint::new(state.step, metric.value(state)));
                }
            }
        }
    }

    fn captures_final(&self) -> bool {
        captures_final(self.aggregation.schedule())
    }

    fn take_step_transfers(&mut self) -> Vec<TransferRecord> {
        Vec::new()
    }

    fn finish(
        self,
        _compiled: &CompiledScenario,
        state: &EngineState,
        completed: bool,
    ) -> BatchSample {
        BatchSample {
            seed: self.seed,
            completed,
            steps_executed: state.step,
            final_metrics: state.metrics.clone(),
            manifest: None,
            series: self.series,
        }
    }
}

impl TransferCollector for BatchSampleCollector<'_> {
    fn wants_transfer_records(&self) -> bool {
        false
    }

    fn record_transfer(&mut self, _transfer: TransferRecord) {}
}

#[derive(Debug)]
struct VariableRuntimeState {
    timing: VariableUpdateTiming,
    sources: BTreeMap<String, VariableSourceSpec>,
    values: BTreeMap<String, f64>,
    rng: BaseRng,
}

/// Borrowed view of ASTs retained by the immutable compiled plan.
#[derive(Debug, Clone, Copy)]
struct ExpressionPlanRef<'a>(&'a CompiledScenario);

impl<'a> ExpressionPlanRef<'a> {
    fn new(compiled: &'a CompiledScenario) -> Self {
        Self(compiled)
    }

    fn transfer_expression(&self, edge_id: &EdgeId) -> Option<&'a CompiledExpr> {
        let edge = EdgeIndex::new(self.0.edge_index(edge_id)?);
        self.0.expressions().transfer(edge)
    }

    fn state_expression(&self, edge_id: &EdgeId) -> Option<&'a CompiledExpr> {
        let edge = EdgeIndex::new(self.0.edge_index(edge_id)?);
        self.0.expressions().state(edge)
    }
}

impl VariableRuntimeState {
    fn from_compiled(compiled: &CompiledScenario, seed: u64) -> Self {
        Self {
            timing: compiled.variables().update_timing.clone(),
            sources: compiled.variables().sources.clone(),
            values: BTreeMap::new(),
            rng: rng_from_seed(seed ^ VARIABLE_RNG_SALT),
        }
    }

    fn refresh_initial(&mut self) {
        if matches!(self.timing, VariableUpdateTiming::RunStart) {
            self.refresh_all();
        }
    }

    fn refresh_for_step(&mut self, _step: u64) {
        if matches!(self.timing, VariableUpdateTiming::EveryStep) {
            self.refresh_all();
        }
    }

    fn values(&self) -> &BTreeMap<String, f64> {
        &self.values
    }

    fn refresh_all(&mut self) {
        for (name, source) in &self.sources {
            if let Some(value) = sample_variable_source(source, &mut self.rng) {
                self.values.insert(name.clone(), canonicalize_float(value));
            }
        }
    }
}

#[derive(Debug)]
struct GateRuntimeState {
    rng: BaseRng,
    weighted_balancers: BTreeMap<NodeId, GateWeightedBalancer>,
}

impl GateRuntimeState {
    fn from_seed(seed: u64) -> Self {
        Self { rng: rng_from_seed(seed ^ GATE_RNG_SALT), weighted_balancers: BTreeMap::new() }
    }

    fn pick_deterministic_target(
        &mut self,
        gate_id: &NodeId,
        lanes: &[GateRoutingLane],
    ) -> Option<usize> {
        if lanes.is_empty() {
            return None;
        }

        let total_weight =
            lanes.iter().fold(0.0, |acc, lane| canonicalize_float(acc + lane.weight.max(0.0)));
        if !total_weight.is_finite() || total_weight <= 0.0 {
            return None;
        }

        let balancer = self.weighted_balancers.entry(gate_id.clone()).or_default();
        let lane_keys = lanes.iter().map(GateRoutingLane::lane_key).collect::<Vec<_>>();
        let active_keys = lane_keys.iter().cloned().collect::<BTreeSet<_>>();
        balancer.scores.retain(|key, _| active_keys.contains(key));
        for (lane_key, lane) in lane_keys.iter().zip(lanes.iter()) {
            let score = balancer.scores.entry(lane_key.clone()).or_insert(0.0);
            *score = canonicalize_float(*score + lane.weight);
        }

        let mut selected = None::<usize>;
        let mut selected_score = f64::NEG_INFINITY;
        for (index, lane_key) in lane_keys.iter().enumerate() {
            let score = balancer.scores.get(lane_key).copied().unwrap_or(0.0);
            if score > selected_score + f64::EPSILON {
                selected_score = score;
                selected = Some(index);
            }
        }

        if let Some(target) = selected {
            if let Some(score) = balancer.scores.get_mut(&lane_keys[target]) {
                *score = canonicalize_float(*score - total_weight);
            }
            return Some(target);
        }

        None
    }

    fn pick_chance_target(&mut self, lanes: &[GateRoutingLane]) -> Option<usize> {
        if lanes.is_empty() {
            return None;
        }

        if lanes.len() == 2 {
            let edge_index = lanes.iter().position(|lane| lane.edge_id.is_some());
            let drop_index = lanes.iter().position(|lane| lane.edge_id.is_none());
            if let (Some(edge_index), Some(drop_index)) = (edge_index, drop_index) {
                let edge_lane = &lanes[edge_index];
                let drop_lane = &lanes[drop_index];
                let total =
                    canonicalize_float(edge_lane.weight.max(0.0) + drop_lane.weight.max(0.0));
                if total > 0.0 {
                    let edge_percent =
                        canonicalize_float(edge_lane.weight.max(0.0) / total * 100.0);
                    let route_to_edge = sample_chance_percent(edge_percent, &mut self.rng).ok()?;
                    return Some(if route_to_edge { edge_index } else { drop_index });
                }
            }
        }

        let mut weights = Vec::with_capacity(lanes.len());
        let mut has_non_zero = false;
        for lane in lanes {
            let weight = canonicalize_float(lane.weight.max(0.0));
            has_non_zero |= weight > 0.0;
            weights.push(weight);
        }
        if !has_non_zero {
            return None;
        }

        let index = sample_weighted_index(&weights, &mut self.rng).ok()?;
        lanes.get(index).map(|_| index)
    }
}

#[derive(Debug, Default)]
struct GateWeightedBalancer {
    scores: BTreeMap<GateLaneKey, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GateLaneKey {
    Drop,
    Edge(EdgeId),
}

fn compiled_plan_error(detail: impl Into<String>) -> RunError {
    RunError::InvalidRunConfig { name: "compiled_plan".to_string(), reason: detail.into() }
}

#[derive(Debug, Clone)]
struct GateRoutingLane {
    edge_id: Option<EdgeId>,
    to_index: Option<usize>,
    weight: f64,
}

impl GateRoutingLane {
    fn lane_key(&self) -> GateLaneKey {
        match &self.edge_id {
            Some(edge_id) => GateLaneKey::Edge(edge_id.clone()),
            None => GateLaneKey::Drop,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateRoutingMode {
    Deterministic,
    Chance,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateBehavior {
    None,
    Sorting,
    Trigger,
    Mixed,
}

#[derive(Debug, Default)]
struct TimelineRuntimeState {
    delay_scheduled: BTreeMap<NodeId, BTreeMap<u64, f64>>,
    delay_ready: BTreeMap<NodeId, f64>,
    queue_ready: BTreeMap<NodeId, f64>,
    queue_incoming: BTreeMap<NodeId, f64>,
    release_budgets: BTreeMap<NodeId, f64>,
}

impl TimelineRuntimeState {
    fn from_compiled(compiled: &CompiledScenario, state: &EngineState) -> Self {
        let mut runtime = Self::default();

        for (index, (node_id, _)) in compiled.nodes().enumerate() {
            let value = canonicalize_float(state.node_values[index].max(0.0));
            if value <= 0.0 {
                continue;
            }

            match timeline_node_kind(compiled, node_id) {
                Some(TimelineNodeKind::Delay) => {
                    let ready_step = delay_steps_for_node(compiled, node_id);
                    let schedule = runtime.delay_scheduled.entry(node_id.clone()).or_default();
                    let slot = schedule.entry(ready_step).or_insert(0.0);
                    *slot = canonicalize_float(*slot + value);
                }
                Some(TimelineNodeKind::Queue) => {
                    runtime.queue_ready.insert(node_id.clone(), value);
                }
                None => {}
            }
        }

        runtime
    }

    fn begin_step(&mut self, compiled: &CompiledScenario, step: u64) {
        self.release_budgets.clear();

        for node_id in compiled.node_ids() {
            match timeline_node_kind(compiled, node_id) {
                Some(TimelineNodeKind::Delay) => {
                    let mut newly_ready = 0.0;
                    if let Some(schedule) = self.delay_scheduled.get_mut(node_id) {
                        let ready_steps = schedule
                            .range(..=step)
                            .map(|(ready_step, _)| *ready_step)
                            .collect::<Vec<_>>();
                        for ready_step in ready_steps {
                            if let Some(amount) = schedule.remove(&ready_step) {
                                newly_ready = canonicalize_float(newly_ready + amount);
                            }
                        }
                    }

                    if newly_ready > 0.0 {
                        let slot = self.delay_ready.entry(node_id.clone()).or_insert(0.0);
                        *slot = canonicalize_float(*slot + newly_ready);
                    }

                    let available = canonicalize_float(
                        self.delay_ready.get(node_id).copied().unwrap_or(0.0).max(0.0),
                    );
                    if available > 0.0 {
                        self.release_budgets.insert(node_id.clone(), available);
                    }
                }
                Some(TimelineNodeKind::Queue) => {
                    let ready = canonicalize_float(
                        self.queue_ready.get(node_id).copied().unwrap_or(0.0).max(0.0),
                    );
                    if ready <= 0.0 {
                        continue;
                    }
                    let per_step = queue_release_per_step_for_node(compiled, node_id) as f64;
                    let available = canonicalize_float(ready.min(per_step));
                    if available > 0.0 {
                        self.release_budgets.insert(node_id.clone(), available);
                    }
                }
                None => {}
            }
        }
    }

    fn finalize_step(&mut self) {
        for (node_id, incoming) in std::mem::take(&mut self.queue_incoming) {
            let slot = self.queue_ready.entry(node_id).or_insert(0.0);
            *slot = canonicalize_float(*slot + incoming);
        }
        self.release_budgets.clear();
    }

    fn transfer_available_for_source(
        &self,
        compiled: &CompiledScenario,
        state: &EngineState,
        node_id: &NodeId,
    ) -> Result<Option<f64>, RunError> {
        if timeline_node_kind(compiled, node_id).is_none() {
            return Ok(None);
        }

        let budget =
            canonicalize_float(self.release_budgets.get(node_id).copied().unwrap_or(0.0).max(0.0));
        let index = compiled
            .node_index(node_id)
            .ok_or_else(|| compiled_plan_error(format!("missing node projection `{node_id}`")))?;
        let available = state.node_values[index].max(0.0);
        Ok(Some(canonicalize_float(available.min(budget))))
    }

    fn record_release(&mut self, compiled: &CompiledScenario, node_id: &NodeId, transfer: f64) {
        let Some(kind) = timeline_node_kind(compiled, node_id) else {
            return;
        };

        let amount = canonicalize_float(transfer.max(0.0));
        if amount <= 0.0 {
            return;
        }

        if let Some(budget) = self.release_budgets.get_mut(node_id) {
            *budget = canonicalize_float((*budget - amount).max(0.0));
        }

        match kind {
            TimelineNodeKind::Delay => {
                if let Some(ready) = self.delay_ready.get_mut(node_id) {
                    *ready = canonicalize_float((*ready - amount).max(0.0));
                }
            }
            TimelineNodeKind::Queue => {
                if let Some(ready) = self.queue_ready.get_mut(node_id) {
                    *ready = canonicalize_float((*ready - amount).max(0.0));
                }
            }
        }
    }

    fn record_arrival(
        &mut self,
        compiled: &CompiledScenario,
        node_id: &NodeId,
        transfer: f64,
        step: u64,
    ) {
        let Some(kind) = timeline_node_kind(compiled, node_id) else {
            return;
        };

        let amount = canonicalize_float(transfer.max(0.0));
        if amount <= 0.0 {
            return;
        }

        match kind {
            TimelineNodeKind::Delay => {
                let ready_step = step.saturating_add(delay_steps_for_node(compiled, node_id));
                let schedule = self.delay_scheduled.entry(node_id.clone()).or_default();
                let slot = schedule.entry(ready_step).or_insert(0.0);
                *slot = canonicalize_float(*slot + amount);
            }
            TimelineNodeKind::Queue => {
                let slot = self.queue_incoming.entry(node_id.clone()).or_insert(0.0);
                *slot = canonicalize_float(*slot + amount);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TimelineNodeKind {
    Delay,
    Queue,
}

fn sample_variable_source(source: &VariableSourceSpec, rng: &mut BaseRng) -> Option<f64> {
    match source {
        VariableSourceSpec::Constant { value } => value.is_finite().then_some(*value),
        VariableSourceSpec::RandomInterval { min, max } => {
            sample_closed_interval(*min, *max, rng).ok()
        }
        VariableSourceSpec::RandomList { values } => sample_from_list(values, rng).ok(),
        VariableSourceSpec::RandomMatrix { values } => sample_from_matrix(values, rng).ok(),
    }
}

/// Builds initial engine state from compiled node defaults and tracked metrics.
pub(crate) fn init_state(compiled: &CompiledScenario) -> EngineState {
    let node_values = compiled
        .nodes()
        .map(|(_, node)| canonicalize_float(node.initial_value()))
        .collect::<Vec<_>>();

    let metrics = compiled
        .tracked_metrics()
        .iter()
        .cloned()
        .map(|metric| (metric, 0.0))
        .collect::<BTreeMap<_, _>>();

    let mut state = EngineState { step: 0, node_values, metrics };
    refresh_metrics(compiled, &mut state);
    state
}

/// Advances one run from init to completion without streaming events.
pub(crate) fn run_single(
    compiled: &CompiledScenario,
    config: &RunConfig,
) -> Result<RunReport, RunError> {
    let mut emit = |_event: RunEvent| Ok(());
    run_single_internal(compiled, config, "run-0", false, false, &mut emit)
}

/// Advances one run while pushing live events to `sink` as steps progress.
pub(crate) fn run_single_streaming(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    sink: &mut dyn EventSink,
) -> Result<RunReport, RunError> {
    let mut emit = |event: RunEvent| sink.push(event).map_err(map_event_sink_error);
    run_single_internal(compiled, config, run_id, false, true, &mut emit)
}

/// Streams run events but defers the terminal `step_end` for assertion interleaving.
///
/// Used by `Simulator::run_with_assertions*` so checkpoints land at the terminal
/// step before the final `step_end`.
pub(crate) fn run_single_streaming_for_assertions(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    sink: &mut dyn EventSink,
) -> Result<RunReport, RunError> {
    let mut emit = |event: RunEvent| sink.push(event).map_err(map_event_sink_error);
    run_single_internal(compiled, config, run_id, true, true, &mut emit)
}

/// Runs one batch seed through the shared transition loop without allocating a
/// full [`RunReport`]. The batch entry point validates `aggregation` once before
/// calling this helper for individual seeds.
pub(crate) fn run_batch_sample(
    compiled: &CompiledScenario,
    config: &RunConfig,
    aggregation: &AggregationConfig,
    metrics: &ResolvedAggregationSelection,
) -> Result<BatchSample, RunError> {
    let mut emit = |_event: RunEvent| Ok(());
    run_collected(
        compiled,
        config,
        "run-0",
        false,
        BatchSampleCollector::new(aggregation, metrics, config.seed),
        &mut emit,
    )
}

fn run_single_internal(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    defer_terminal_step_end: bool,
    emit_transfers: bool,
    emit_event: &mut dyn FnMut(RunEvent) -> Result<(), RunError>,
) -> Result<RunReport, RunError> {
    validate_capture_selection(compiled, config)?;
    run_collected(
        compiled,
        config,
        run_id,
        defer_terminal_step_end,
        FullReportCollector::new(compiled, config, emit_transfers),
        emit_event,
    )
}

fn run_collected<C: RunCollector>(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    defer_terminal_step_end: bool,
    mut collector: C,
    emit_event: &mut dyn FnMut(RunEvent) -> Result<(), RunError>,
) -> Result<C::Output, RunError> {
    let mut state = init_state(compiled);
    let runtime = ExprRuntime::new();
    let expression_cache = ExpressionPlanRef::new(compiled);
    let step_plan = compiled.routing();
    let mut variables = VariableRuntimeState::from_compiled(compiled, config.seed);
    let mut gates = GateRuntimeState::from_seed(config.seed);
    let mut timeline = TimelineRuntimeState::from_compiled(compiled, &state);
    variables.refresh_initial();
    collector.capture_step(compiled, &state, variables.values(), false);

    let mut completed = end_conditions_met(compiled, &state);

    if completed {
        emit_event(RunEvent::step_start(
            run_id,
            state.step,
            0,
            StepStartEvent { seed: config.seed },
        ))?;
        emit_metric_snapshots(run_id, state.step, &state.metrics, emit_event)?;
        if !defer_terminal_step_end {
            emit_event(RunEvent::step_end(
                run_id,
                state.step,
                0,
                StepEndEvent { completed: true },
            ))?;
        }
    }

    while !completed && state.step < config.max_steps {
        let attempted_step = state
            .step
            .checked_add(1)
            .ok_or(RunError::StepOverflow { attempted: u64::MAX, max: config.max_steps })?;
        emit_event(RunEvent::step_start(
            run_id,
            attempted_step,
            0,
            StepStartEvent { seed: config.seed },
        ))?;
        variables.refresh_for_step(attempted_step);

        apply_source_generation(compiled, &mut state);
        timeline.begin_step(compiled, attempted_step);
        apply_edge_transfers(
            compiled,
            step_plan,
            &mut state,
            &runtime,
            &expression_cache,
            variables.values(),
            &mut gates,
            &mut timeline,
            attempted_step,
            &mut collector,
        )?;
        for (ordinal, transfer) in collector.take_step_transfers().into_iter().enumerate() {
            emit_event(RunEvent::transfer(
                run_id,
                transfer.step,
                ordinal as u64,
                TransferEvent {
                    edge_id: transfer.edge_id.clone(),
                    from_node_id: transfer.from_node_id.clone(),
                    to_node_id: transfer.to_node_id.clone(),
                    requested_amount: transfer.requested_amount,
                    transferred_amount: transfer.transferred_amount,
                },
            ))?;
        }
        timeline.finalize_step();
        apply_state_connections(
            compiled,
            &mut state,
            &runtime,
            &expression_cache,
            variables.values(),
        )?;
        state.step = attempted_step;
        refresh_metrics(compiled, &mut state);
        emit_metric_snapshots(run_id, state.step, &state.metrics, emit_event)?;

        collector.capture_step(compiled, &state, variables.values(), false);
        completed = end_conditions_met(compiled, &state);
        let terminal_step_reached = completed || state.step >= config.max_steps;
        if !(defer_terminal_step_end && terminal_step_reached) {
            emit_event(RunEvent::step_end(run_id, state.step, 0, StepEndEvent { completed }))?;
        }
    }

    if collector.captures_final() {
        collector.capture_step(compiled, &state, variables.values(), true);
    }

    Ok(collector.finish(compiled, &state, completed))
}

fn emit_metric_snapshots(
    run_id: &str,
    step: u64,
    metrics: &BTreeMap<MetricKey, f64>,
    emit_event: &mut dyn FnMut(RunEvent) -> Result<(), RunError>,
) -> Result<(), RunError> {
    for (ordinal, (metric, value)) in metrics.iter().enumerate() {
        emit_event(RunEvent::metric_snapshot(
            run_id,
            step,
            ordinal as u64,
            MetricSnapshotEvent { metric: metric.clone(), value: *value },
        ))?;
    }
    Ok(())
}

fn map_event_sink_error(error: EventSinkError) -> RunError {
    RunError::EventSink { message: error.to_string() }
}

fn formula_run_error(name: String, error: ExprError) -> RunError {
    RunError::InvalidRunConfig { name, reason: format!("formula evaluation failed: {error}") }
}

fn apply_source_generation(compiled: &CompiledScenario, state: &mut EngineState) {
    for (index, (_, node)) in compiled.nodes().enumerate() {
        if !matches!(node.behavior(), NodeBehavior::Source) {
            continue;
        }

        let generation = canonicalize_float(node.initial_value());
        if generation <= 0.0 || !generation.is_finite() {
            continue;
        }

        let value = &mut state.node_values[index];
        *value = canonicalize_float(*value + generation);
    }
}

#[derive(Debug, Default)]
struct StepTriggers {
    nodes: BTreeSet<NodeId>,
    edges: BTreeSet<EdgeId>,
}

#[derive(Debug, Clone)]
struct EdgeTransferPlan {
    edge_id: EdgeId,
    from_node_id: NodeId,
    to_node_id: NodeId,
    from_index: usize,
    to_index: usize,
    requested: f64,
    transfer: f64,
}

/// Applies resource transfers for one step, iterating control groups to a fixpoint.
///
/// A single forward pass over `node_order` cannot fire a passive controller that
/// sorts *before* the gate that triggers it mid-step. Triggers only grow within a
/// step and emission is idempotent (`BTreeSet` insert), so each pass settles newly
/// eligible groups without double-transferring already settled ones. Pure
/// TriggerGates can add triggers without settling a group; trigger growth alone
/// counts as progress so earlier controllers can be revisited.
#[allow(clippy::too_many_arguments)]
fn apply_edge_transfers(
    compiled: &CompiledScenario,
    step_plan: &crate::plan::RoutingPlan,
    state: &mut EngineState,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    gates: &mut GateRuntimeState,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_collector: &mut dyn TransferCollector,
) -> Result<(), RunError> {
    let mut triggers = collect_step_triggers(compiled, step_plan, state);
    let mut settled_groups: BTreeSet<(usize, TransferControl)> = BTreeSet::new();

    loop {
        let mut progress = false;
        let triggers_before = triggers.nodes.len() + triggers.edges.len();

        for (node_index, node_id) in compiled.node_ids().iter().enumerate() {
            let gate_behavior = gate_behavior_for_node(compiled, node_id);
            let mut node_acted = false;
            let mut had_resource_groups = false;
            let controller = NodeIndex::new(node_index);

            for control in [
                TransferControl::PullAny,
                TransferControl::PullAll,
                TransferControl::PushAny,
                TransferControl::PushAll,
            ] {
                let Some(edge_ids) = step_plan.resource_group(controller, control) else {
                    continue;
                };
                had_resource_groups = true;

                if settled_groups.contains(&(node_index, control)) {
                    continue;
                }

                if !controller_can_fire(compiled, state, node_id, edge_ids, &triggers) {
                    continue;
                }

                settled_groups.insert((node_index, control));
                progress = true;

                let acted = if should_use_gate_routing(compiled, node_id, control, edge_ids)? {
                    apply_gate_edge_group(
                        compiled,
                        state,
                        node_id,
                        edge_ids,
                        control,
                        runtime,
                        expression_cache,
                        runtime_variables,
                        gates,
                        timeline,
                        step,
                        transfer_collector,
                    )?
                } else {
                    match control {
                        TransferControl::PullAny | TransferControl::PushAny => {
                            apply_any_edge_group(
                                compiled,
                                state,
                                edge_ids,
                                runtime,
                                expression_cache,
                                runtime_variables,
                                timeline,
                                step,
                                transfer_collector,
                            )?
                        }
                        TransferControl::PullAll | TransferControl::PushAll => {
                            apply_all_edge_group(
                                compiled,
                                state,
                                edge_ids,
                                runtime,
                                expression_cache,
                                runtime_variables,
                                timeline,
                                step,
                                transfer_collector,
                            )?
                        }
                    }
                };

                node_acted |= acted;
            }

            match gate_behavior {
                GateBehavior::Mixed if node_acted => {
                    append_node_trigger_outputs(compiled, step_plan, node_id, &mut triggers);
                }
                GateBehavior::Trigger => {
                    let trigger_gate_acted = if had_resource_groups {
                        node_acted
                    } else {
                        controller_can_fire(compiled, state, node_id, &[], &triggers)
                    };
                    if trigger_gate_acted {
                        append_node_trigger_outputs(compiled, step_plan, node_id, &mut triggers);
                    }
                }
                GateBehavior::None | GateBehavior::Sorting | GateBehavior::Mixed => {}
            }
        }

        let triggers_grew = triggers.nodes.len() + triggers.edges.len() > triggers_before;
        if !progress && !triggers_grew {
            break;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_any_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    edge_ids: &[EdgeIndex],
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_collector: &mut dyn TransferCollector,
) -> Result<bool, RunError> {
    let mut acted = false;
    for edge_id in edge_ids {
        let compiled_edge = compiled.edge_at(*edge_id);
        let from_available_override =
            timeline.transfer_available_for_source(compiled, state, compiled_edge.from())?;
        let Some(plan) = plan_edge_transfer_any(
            compiled,
            state,
            compiled_edge,
            runtime,
            expression_cache,
            runtime_variables,
            from_available_override,
        )?
        else {
            continue;
        };
        apply_transfer_plan(compiled, state, plan, timeline, step, transfer_collector)?;
        acted = true;
    }
    Ok(acted)
}

/// All-or-nothing transfer group: skips zero-request edges instead of aborting.
///
/// `None` from planning means a trivially empty request (zero fraction / scaled),
/// not failure. Atomicity is still enforced over the remaining non-zero plans.
#[allow(clippy::too_many_arguments)]
fn apply_all_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    edge_ids: &[EdgeIndex],
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_collector: &mut dyn TransferCollector,
) -> Result<bool, RunError> {
    let mut plans = Vec::new();
    let mut total_requested_by_source = BTreeMap::<usize, f64>::new();
    let mut available_by_source = BTreeMap::<usize, f64>::new();

    for edge_id in edge_ids {
        let compiled_edge = compiled.edge_at(*edge_id);
        let from_available_override =
            timeline.transfer_available_for_source(compiled, state, compiled_edge.from())?;
        let Some(plan) = plan_edge_transfer_all(
            compiled,
            state,
            compiled_edge,
            runtime,
            expression_cache,
            runtime_variables,
            from_available_override,
        )?
        else {
            continue;
        };

        let available = canonicalize_float(
            from_available_override.unwrap_or(state.node_values[plan.from_index]).max(0.0),
        );
        available_by_source.entry(plan.from_index).or_insert(available);

        let total = total_requested_by_source.entry(plan.from_index).or_insert(0.0);
        *total = canonicalize_float(*total + plan.transfer);
        plans.push(plan);
    }

    for (from_index, requested_total) in total_requested_by_source {
        let available = available_by_source
            .get(&from_index)
            .copied()
            .unwrap_or(state.node_values[from_index].max(0.0));
        if canonicalize_float(available) + f64::EPSILON < requested_total {
            return Ok(false);
        }
    }

    for plan in plans {
        apply_transfer_plan(compiled, state, plan, timeline, step, transfer_collector)?;
    }

    Ok(true)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateWeightKind {
    Ratio,
    Percentage,
    Chance,
}

fn required_resource(
    edge: &CompiledEdge,
) -> Result<(&crate::types::ResourceConnection, &CompiledTransfer), RunError> {
    edge.resource().ok_or_else(|| {
        compiled_plan_error(format!("edge {} is not a compiled resource transfer", edge.id()))
    })
}

fn should_use_gate_routing(
    compiled: &CompiledScenario,
    node_id: &NodeId,
    control: TransferControl,
    edge_ids: &[EdgeIndex],
) -> Result<bool, RunError> {
    if !matches!(control, TransferControl::PushAny | TransferControl::PushAll) {
        return Ok(false);
    }
    if !matches!(
        gate_behavior_for_node(compiled, node_id),
        GateBehavior::Sorting | GateBehavior::Mixed
    ) {
        return Ok(false);
    }

    for edge_id in edge_ids {
        let edge = compiled.edge_at(*edge_id);
        let Some((resource, _)) = edge.resource() else {
            return Err(compiled_plan_error(format!(
                "routing contains non-resource edge {}",
                edge.id()
            )));
        };
        if edge.from() != node_id || resource.token_size().get() != 1 {
            return Ok(false);
        }
    }
    Ok(true)
}

#[allow(clippy::too_many_arguments)]
fn apply_gate_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeIndex],
    control: TransferControl,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    gates: &mut GateRuntimeState,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_collector: &mut dyn TransferCollector,
) -> Result<bool, RunError> {
    let routing = match gate_routing_for_group(
        compiled,
        state,
        node_id,
        edge_ids,
        runtime,
        expression_cache,
        runtime_variables,
    )? {
        Some(routing) => routing,
        None => {
            return Ok(match control {
                TransferControl::PullAny | TransferControl::PushAny => apply_any_edge_group(
                    compiled,
                    state,
                    edge_ids,
                    runtime,
                    expression_cache,
                    runtime_variables,
                    timeline,
                    step,
                    transfer_collector,
                )?,
                TransferControl::PullAll | TransferControl::PushAll => apply_all_edge_group(
                    compiled,
                    state,
                    edge_ids,
                    runtime,
                    expression_cache,
                    runtime_variables,
                    timeline,
                    step,
                    transfer_collector,
                )?,
            });
        }
    };

    let from_index = compiled
        .node_index(node_id)
        .ok_or_else(|| compiled_plan_error(format!("missing node projection `{node_id}`")))?;
    let available_tokens = state.node_values[from_index].max(0.0).floor() as u64;
    if available_tokens == 0 {
        return Ok(false);
    }

    let mut acted = false;
    for _ in 0..available_tokens {
        let selected = match routing.0 {
            GateRoutingMode::Deterministic => gates.pick_deterministic_target(node_id, &routing.1),
            GateRoutingMode::Chance => gates.pick_chance_target(&routing.1),
        };
        let Some(selected_index) = selected else {
            continue;
        };
        let Some(lane) = routing.1.get(selected_index) else {
            continue;
        };

        if lane.edge_id.is_none() {
            let value = &mut state.node_values[from_index];
            *value = canonicalize_float(*value - 1.0);
            acted = true;
            continue;
        }

        let Some(selected_edge_id) = lane.edge_id.as_ref() else {
            continue;
        };
        let Some(to_index) = lane.to_index else {
            continue;
        };
        if to_index == from_index {
            continue;
        }
        apply_transfer_plan(
            compiled,
            state,
            EdgeTransferPlan {
                edge_id: selected_edge_id.clone(),
                from_node_id: node_id.clone(),
                to_node_id: compiled
                    .node_id_at(to_index)
                    .ok_or_else(|| {
                        compiled_plan_error(format!("missing node projection at index {to_index}"))
                    })?
                    .clone(),
                from_index,
                to_index,
                requested: 1.0,
                transfer: 1.0,
            },
            timeline,
            step,
            transfer_collector,
        )?;
        acted = true;
    }

    Ok(acted)
}

/// Builds weighted routing lanes; zero/non-finite weights skip that lane only.
///
/// Skipping a dead lane keeps the remaining weights usable. Returning `None` for
/// the whole group would fall back to flat dispatch and change gate outcomes.
fn gate_routing_for_group(
    compiled: &CompiledScenario,
    state: &EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeIndex],
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<Option<(GateRoutingMode, Vec<GateRoutingLane>)>, RunError> {
    let mut lanes = Vec::<GateRoutingLane>::new();
    let mut seen_ratio = false;
    let mut seen_percentage = false;
    let mut seen_chance = false;

    for edge_id in edge_ids {
        let compiled_edge = compiled.edge_at(*edge_id);
        let to_index = compiled_edge.target_index();
        let from_index = compiled_edge.source_index();
        if from_index == to_index {
            continue;
        }

        let Some((kind, weight)) = gate_weight_for_edge(
            compiled,
            state,
            compiled_edge,
            runtime,
            expression_cache,
            runtime_variables,
        )?
        else {
            continue;
        };
        if weight <= 0.0 {
            continue;
        }

        match kind {
            GateWeightKind::Ratio => seen_ratio = true,
            GateWeightKind::Percentage => seen_percentage = true,
            GateWeightKind::Chance => seen_chance = true,
        }

        lanes.push(GateRoutingLane {
            edge_id: Some(compiled_edge.id().clone()),
            to_index: Some(to_index),
            weight,
        });
    }

    if lanes.is_empty() {
        return Ok(None);
    }

    if seen_ratio && seen_percentage {
        return Err(RunError::InvalidRunConfig {
            name: format!("nodes.{node_id}.outputs"),
            reason: "gate output distribution cannot mix percentage and whole-number ratio styles"
                .to_string(),
        });
    }

    let total_weight =
        lanes.iter().fold(0.0, |acc, lane| canonicalize_float(acc + lane.weight.max(0.0)));
    if total_weight <= 0.0 || !total_weight.is_finite() {
        return Ok(None);
    }

    let uses_percentage_scale = seen_percentage || (seen_chance && !seen_ratio);
    if uses_percentage_scale && total_weight + f64::EPSILON < 100.0 {
        lanes.push(GateRoutingLane {
            edge_id: None,
            to_index: None,
            weight: canonicalize_float(100.0 - total_weight),
        });
    }

    let mode = if seen_chance { GateRoutingMode::Chance } else { GateRoutingMode::Deterministic };
    Ok(Some((mode, lanes)))
}

fn gate_weight_for_edge(
    compiled: &CompiledScenario,
    state: &EngineState,
    compiled_edge: &CompiledEdge,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<Option<(GateWeightKind, f64)>, RunError> {
    let (_, transfer) = required_resource(compiled_edge)?;
    match transfer {
        CompiledTransfer::Fixed { amount } => {
            Ok(amount.is_finite().then_some((GateWeightKind::Ratio, canonicalize_float(*amount))))
        }
        CompiledTransfer::Fraction { numerator, denominator } => {
            if *numerator == 0 {
                return Ok(None);
            }
            let weight = *numerator as f64 / denominator.get() as f64 * 100.0;
            Ok(weight
                .is_finite()
                .then_some((GateWeightKind::Percentage, canonicalize_float(weight))))
        }
        CompiledTransfer::MetricScaled { metric, factor } => {
            let weight = metric_value(compiled, state, metric) * *factor;
            Ok(weight.is_finite().then_some((GateWeightKind::Chance, canonicalize_float(weight))))
        }
        CompiledTransfer::Expression => {
            let from_value = state.node_values[compiled_edge.source_index()];
            let requested = transfer_request(
                compiled,
                state,
                compiled_edge,
                from_value,
                runtime,
                expression_cache,
                runtime_variables,
            )?;
            Ok(requested
                .is_finite()
                .then_some((GateWeightKind::Chance, canonicalize_float(requested))))
        }
        CompiledTransfer::Remaining => Ok(Some((GateWeightKind::Chance, 100.0))),
    }
}

fn plan_edge_transfer_any(
    compiled: &CompiledScenario,
    state: &EngineState,
    compiled_edge: &CompiledEdge,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    from_value_override: Option<f64>,
) -> Result<Option<EdgeTransferPlan>, RunError> {
    let (resource, _) = required_resource(compiled_edge)?;
    let from_index = compiled_edge.source_index();
    let to_index = compiled_edge.target_index();
    if from_index == to_index {
        return Ok(None);
    }

    let from_value =
        canonicalize_float(from_value_override.unwrap_or(state.node_values[from_index]).max(0.0));
    let requested = transfer_request(
        compiled,
        state,
        compiled_edge,
        from_value,
        runtime,
        expression_cache,
        runtime_variables,
    )?;
    let transfer = clamp_transfer_amount(resource.token_size(), from_value, requested);
    if transfer <= 0.0 {
        return Ok(None);
    }

    Ok(Some(EdgeTransferPlan {
        edge_id: compiled_edge.id().clone(),
        from_node_id: compiled_edge.from().clone(),
        to_node_id: compiled_edge.to().clone(),
        from_index,
        to_index,
        requested,
        transfer,
    }))
}

fn plan_edge_transfer_all(
    compiled: &CompiledScenario,
    state: &EngineState,
    compiled_edge: &CompiledEdge,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
    from_value_override: Option<f64>,
) -> Result<Option<EdgeTransferPlan>, RunError> {
    let (resource, _) = required_resource(compiled_edge)?;
    let from_index = compiled_edge.source_index();
    let to_index = compiled_edge.target_index();
    if from_index == to_index {
        return Ok(None);
    }

    let from_value =
        canonicalize_float(from_value_override.unwrap_or(state.node_values[from_index]).max(0.0));
    let requested = transfer_request(
        compiled,
        state,
        compiled_edge,
        from_value,
        runtime,
        expression_cache,
        runtime_variables,
    )?;
    let transfer = quantize_requested_amount(resource.token_size(), requested);
    if transfer <= 0.0 {
        return Ok(None);
    }

    Ok(Some(EdgeTransferPlan {
        edge_id: compiled_edge.id().clone(),
        from_node_id: compiled_edge.from().clone(),
        to_node_id: compiled_edge.to().clone(),
        from_index,
        to_index,
        requested,
        transfer,
    }))
}

fn apply_transfer_plan(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    plan: EdgeTransferPlan,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_collector: &mut dyn TransferCollector,
) -> Result<(), RunError> {
    let transfer = accepted_arrival(compiled, state, plan.to_index, plan.transfer)?;

    state.node_values[plan.from_index] =
        canonicalize_float(state.node_values[plan.from_index] - transfer);
    state.node_values[plan.to_index] =
        canonicalize_float(state.node_values[plan.to_index] + transfer);

    let from_node_id = compiled.node_id_at(plan.from_index).ok_or_else(|| {
        compiled_plan_error(format!("missing node projection at index {}", plan.from_index))
    })?;
    let to_node_id = compiled.node_id_at(plan.to_index).ok_or_else(|| {
        compiled_plan_error(format!("missing node projection at index {}", plan.to_index))
    })?;
    timeline.record_release(compiled, from_node_id, transfer);
    timeline.record_arrival(compiled, to_node_id, transfer, step);

    if transfer_collector.wants_transfer_records() {
        transfer_collector.record_transfer(TransferRecord {
            step,
            edge_id: plan.edge_id,
            from_node_id: plan.from_node_id,
            to_node_id: plan.to_node_id,
            requested_amount: canonicalize_float(plan.requested),
            transferred_amount: canonicalize_float(transfer),
        });
    }
    Ok(())
}

fn collect_step_triggers(
    compiled: &CompiledScenario,
    step_plan: &crate::plan::RoutingPlan,
    state: &EngineState,
) -> StepTriggers {
    let mut triggers = StepTriggers::default();

    for (source_node_index, targets) in step_plan.passive_state_triggers() {
        let source_state = state.node_values[source_node_index.value()];
        if !source_state.is_finite() || source_state <= 0.0 {
            continue;
        }
        append_trigger_targets(compiled, targets, &mut triggers);
    }

    triggers
}

fn append_node_trigger_outputs(
    compiled: &CompiledScenario,
    step_plan: &crate::plan::RoutingPlan,
    node_id: &NodeId,
    triggers: &mut StepTriggers,
) {
    if let Some(node_index) = compiled.node_index(node_id) {
        if let Some(targets) = step_plan.trigger_outputs(NodeIndex::new(node_index)) {
            append_trigger_targets(compiled, targets, triggers);
        }
    }
}

fn append_trigger_targets(
    compiled: &CompiledScenario,
    targets: &[TriggerTarget],
    triggers: &mut StepTriggers,
) {
    for target in targets {
        match target {
            TriggerTarget::Node(node_index) => {
                triggers.nodes.insert(compiled.node_id_at_index(*node_index).clone());
            }
            TriggerTarget::Edge(edge_index) => {
                triggers.edges.insert(compiled.edge_at(*edge_index).id().clone());
            }
        }
    }
}

fn controller_can_fire(
    compiled: &CompiledScenario,
    state: &EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeIndex],
    triggers: &StepTriggers,
) -> bool {
    match trigger_mode_for_node(compiled, node_id) {
        TriggerMode::Automatic => true,
        TriggerMode::Interactive => false,
        TriggerMode::Enabling => state.step == 0,
        TriggerMode::Passive => {
            triggers.nodes.contains(node_id)
                || edge_ids
                    .iter()
                    .any(|edge_id| triggers.edges.contains(compiled.edge_at(*edge_id).id()))
        }
        TriggerMode::Custom(_) => true,
    }
}

fn trigger_mode_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> TriggerMode {
    let mode = node_mode_for_node(compiled, node_id);
    mode.map(|m| m.trigger_mode.clone()).unwrap_or(TriggerMode::Automatic)
}

fn gate_behavior_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> GateBehavior {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::SortingGate(_) => GateBehavior::Sorting,
        NodeBehavior::TriggerGate(_) => GateBehavior::Trigger,
        NodeBehavior::MixedGate(_) => GateBehavior::Mixed,
        _ => GateBehavior::None,
    }
}

fn timeline_node_kind(compiled: &CompiledScenario, node_id: &NodeId) -> Option<TimelineNodeKind> {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::Delay(_) => Some(TimelineNodeKind::Delay),
        NodeBehavior::Queue(_) => Some(TimelineNodeKind::Queue),
        _ => None,
    }
}

fn delay_steps_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> u64 {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::Delay(config) => config.delay_steps().get(),
        _ => unreachable!("timeline routing only calls delay helper for checked delay nodes"),
    }
}

fn queue_release_per_step_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> u64 {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::Queue(config) => config.release_per_step().get(),
        _ => unreachable!("timeline routing only calls queue helper for checked queue nodes"),
    }
}

/// Returns the configured capacity of a capacity-bounded target node. Both Pool
/// and Queue nodes treat `capacity` as a hard upper bound on the value they store
/// (validation rejects an over-capacity initial value for each), so both are
/// enforced at runtime. Other node kinds have no stored-value capacity.
fn node_capacity_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> Option<u64> {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::Pool(config) => config.capacity(),
        NodeBehavior::Queue(config) => config.capacity().map(NonZeroU64::get),
        _ => None,
    }
}

/// Clips a transfer arriving at `to_index` so a capacity-bounded target (Pool or
/// Queue) never holds more than its configured capacity. The held inventory is
/// tracked by the node value (arrivals raise it, releases/outflows lower it), so
/// the remaining headroom is `capacity - held`. Targets without a capacity are
/// unaffected. The un-accepted remainder stays at the source node, modelling
/// buffer backpressure.
fn accepted_arrival(
    compiled: &CompiledScenario,
    state: &EngineState,
    to_index: usize,
    transfer: f64,
) -> Result<f64, RunError> {
    let node_id = compiled.node_id_at(to_index).ok_or_else(|| {
        compiled_plan_error(format!("missing node projection at index {to_index}"))
    })?;
    let Some(capacity) = node_capacity_for_node(compiled, node_id) else {
        return Ok(transfer);
    };
    let held = state.node_values[to_index].max(0.0);
    let remaining = canonicalize_float((capacity as f64 - held).max(0.0));
    Ok(canonicalize_float(transfer.min(remaining)))
}

fn node_mode_for_node<'a>(
    compiled: &'a CompiledScenario,
    node_id: &NodeId,
) -> Option<&'a NodeModeConfig> {
    match compiled.required_node(node_id).behavior() {
        NodeBehavior::Pool(config) => Some(config.mode()),
        NodeBehavior::Drain(config) => Some(config.mode()),
        NodeBehavior::SortingGate(config) => Some(config.mode()),
        NodeBehavior::TriggerGate(config) => Some(config.mode()),
        NodeBehavior::MixedGate(config) => Some(config.mode()),
        NodeBehavior::Converter(config) => Some(config.mode()),
        NodeBehavior::Trader(config) => Some(config.mode()),
        NodeBehavior::Delay(config) => Some(config.mode()),
        NodeBehavior::Queue(config) => Some(config.mode()),
        NodeBehavior::Source
        | NodeBehavior::Register(_)
        | NodeBehavior::Process
        | NodeBehavior::Sink
        | NodeBehavior::Gate
        | NodeBehavior::Custom(_) => None,
    }
}

fn apply_state_connections(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<(), RunError> {
    let mut next_step_node_deltas = vec![0.0; state.node_values.len()];

    for compiled_edge in compiled.edges() {
        let edge_id = compiled_edge.id();
        if !compiled_edge.enabled() {
            continue;
        }

        let Some(state_config) = compiled_edge.state() else {
            continue;
        };
        if !matches!(state_config.role(), StateConnectionRole::Modifier)
            || !matches!(state_config.target(), StateTarget::Node)
        {
            continue;
        }

        let from_index = compiled_edge.source_index();
        let to_index = compiled_edge.target_index();

        let source_state = state.node_values[from_index];
        if !source_state.is_finite() || source_state == 0.0 {
            continue;
        }
        let target_state = state.node_values[to_index];

        let Some(delta) = evaluate_state_formula_delta(
            compiled,
            state,
            edge_id,
            source_state,
            target_state,
            runtime,
            expression_cache,
            runtime_variables,
        )?
        else {
            continue;
        };

        let effect = canonicalize_float(delta * source_state);
        if effect == 0.0 {
            continue;
        }

        let slot = &mut next_step_node_deltas[to_index];
        *slot = canonicalize_float(*slot + effect);
    }

    for (index, delta) in next_step_node_deltas.into_iter().enumerate() {
        if delta == 0.0 {
            continue;
        }
        let value = &mut state.node_values[index];
        *value = canonicalize_float(*value + delta);
    }

    Ok(())
}

fn transfer_request(
    compiled: &CompiledScenario,
    state: &EngineState,
    compiled_edge: &CompiledEdge,
    from_value: f64,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<f64, RunError> {
    let (_, transfer) = required_resource(compiled_edge)?;
    let requested = match transfer {
        CompiledTransfer::Fixed { amount } => *amount,
        CompiledTransfer::Fraction { numerator, denominator } => {
            from_value * (*numerator as f64 / denominator.get() as f64)
        }
        CompiledTransfer::Remaining => from_value,
        CompiledTransfer::MetricScaled { metric, factor } => {
            metric_value(compiled, state, metric) * *factor
        }
        CompiledTransfer::Expression => {
            let Some(compiled_expression) =
                expression_cache.transfer_expression(compiled_edge.id())
            else {
                return Err(RunError::InvalidRunConfig {
                    name: format!("edges.{}.transfer.expression.formula", compiled_edge.id()),
                    reason: "expression was not compiled".to_string(),
                });
            };

            let step = state.step as f64;
            let total = total_node_value(state);
            let to_value =
                Some(canonicalize_float(state.node_values[compiled_edge.target_index()]));

            evaluate_compiled_formula(
                runtime,
                compiled_expression,
                runtime_variables,
                format!("edges.{}.transfer.expression.formula", compiled_edge.id()),
                &FormulaBindings {
                    step: canonicalize_float(step),
                    total: canonicalize_float(total),
                    nodes: canonicalize_float(compiled.node_count() as f64),
                    next_step: canonicalize_float(step + 1.0),
                    is_positive_total: canonicalize_float(total.max(0.0)),
                    from: Some(canonicalize_float(from_value)),
                    to: to_value,
                    source: None,
                    target: None,
                    available: Some(canonicalize_float(from_value.max(0.0))),
                    s: None,
                },
            )?
        }
    };

    Ok(canonicalize_float(requested))
}

fn clamp_transfer_amount(token_size: NonZeroU64, from_value: f64, requested: f64) -> f64 {
    if !requested.is_finite() || requested <= 0.0 {
        return 0.0;
    }

    let available = canonicalize_float(from_value.max(0.0));
    if available <= 0.0 {
        return 0.0;
    }

    let bounded = requested.min(available);
    quantize_requested_amount(token_size, bounded)
}

fn quantize_requested_amount(token_size: NonZeroU64, requested: f64) -> f64 {
    if !requested.is_finite() || requested <= 0.0 {
        return 0.0;
    }

    let token_size = token_size.get() as f64;
    let transferable_tokens = (requested / token_size).floor();
    if transferable_tokens <= 0.0 {
        return 0.0;
    }

    canonicalize_float(transferable_tokens * token_size)
}

#[derive(Debug, Clone, Copy)]
struct FormulaBindings {
    step: f64,
    total: f64,
    nodes: f64,
    next_step: f64,
    is_positive_total: f64,
    from: Option<f64>,
    to: Option<f64>,
    source: Option<f64>,
    target: Option<f64>,
    available: Option<f64>,
    s: Option<f64>,
}

fn evaluate_compiled_formula(
    runtime: &ExprRuntime,
    expression: &CompiledExpr,
    runtime_variables: &BTreeMap<String, f64>,
    name: String,
    bindings: &FormulaBindings,
) -> Result<f64, RunError> {
    let value = runtime
        .evaluate_compiled_with_resolver(expression, |name| {
            resolve_formula_variable(name, bindings, runtime_variables)
        })
        .map_err(|error| formula_run_error(name, error))?;
    Ok(canonicalize_float(value))
}

fn resolve_formula_variable(
    name: &str,
    bindings: &FormulaBindings,
    runtime_variables: &BTreeMap<String, f64>,
) -> Option<f64> {
    match name {
        "from" => bindings.from.or_else(|| runtime_variables.get(name).copied()),
        "to" => bindings.to.or_else(|| runtime_variables.get(name).copied()),
        "source" => bindings.source.or_else(|| runtime_variables.get(name).copied()),
        "target" => bindings.target.or_else(|| runtime_variables.get(name).copied()),
        "available" => bindings.available.or_else(|| runtime_variables.get(name).copied()),
        "S" => bindings.s.or_else(|| runtime_variables.get(name).copied()),
        "step" => Some(bindings.step),
        "total" => Some(bindings.total),
        "nodes" => Some(bindings.nodes),
        "next_step" => Some(bindings.next_step),
        "is_positive_total" => Some(bindings.is_positive_total),
        _ => runtime_variables.get(name).copied(),
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_state_formula_delta(
    compiled: &CompiledScenario,
    state: &EngineState,
    edge_id: &EdgeId,
    source_state: f64,
    target_state: f64,
    runtime: &ExprRuntime,
    expression_cache: &ExpressionPlanRef<'_>,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<Option<f64>, RunError> {
    let Some(compiled_expression) = expression_cache.state_expression(edge_id) else {
        return Err(RunError::InvalidRunConfig {
            name: format!("edges.{edge_id}.connection.state.formula"),
            reason: "expression was not compiled".to_string(),
        });
    };
    let step = state.step as f64;
    let total = total_node_value(state);

    evaluate_compiled_formula(
        runtime,
        compiled_expression,
        runtime_variables,
        format!("edges.{edge_id}.connection.state.formula"),
        &FormulaBindings {
            step: canonicalize_float(step),
            total: canonicalize_float(total),
            nodes: canonicalize_float(compiled.node_count() as f64),
            next_step: canonicalize_float(step + 1.0),
            is_positive_total: canonicalize_float(total.max(0.0)),
            from: None,
            to: None,
            source: Some(canonicalize_float(source_state)),
            target: Some(canonicalize_float(target_state)),
            available: Some(canonicalize_float(source_state.max(0.0))),
            s: Some(canonicalize_float(source_state)),
        },
    )
    .map(Some)
}

fn refresh_metrics(compiled: &CompiledScenario, state: &mut EngineState) {
    if state.metrics.is_empty() {
        return;
    }

    let total_value = total_node_value(state);

    for metric in compiled.tracked_metrics() {
        let value = match metric_node_index(compiled, metric) {
            Some(index) => state.node_values[index],
            None => total_value,
        };
        state.metrics.insert(metric.clone(), canonicalize_float(value));
    }
}

fn total_node_value(state: &EngineState) -> f64 {
    state.node_values.iter().copied().fold(0.0, |acc, value| canonicalize_float(acc + value))
}

fn metric_node_index(compiled: &CompiledScenario, metric: &MetricKey) -> Option<usize> {
    compiled.metric_node_index(metric)
}

fn node_value(compiled: &CompiledScenario, state: &EngineState, node_id: &NodeId) -> f64 {
    state.node_values[compiled.required_node_index(node_id)]
}

/// Resolves a metric for edge evaluation using live node values, not the end-of-step cache.
///
/// `state.metrics` is only refreshed via `refresh_metrics` after the step, so
/// reading it mid-step would stale `MetricScaled` transfers and gate weights that
/// depend on earlier edges in the same step.
fn metric_value(compiled: &CompiledScenario, state: &EngineState, metric: &MetricKey) -> f64 {
    if let Some(index) = metric_node_index(compiled, metric) {
        return canonicalize_float(state.node_values[index]);
    }

    if compiled.tracked_metrics().contains(metric) {
        return canonicalize_float(total_node_value(state));
    }

    if let Some(value) = state.metrics.get(metric).copied() {
        return canonicalize_float(value);
    }

    0.0
}

fn end_conditions_met(compiled: &CompiledScenario, state: &EngineState) -> bool {
    compiled.end_conditions().iter().any(|condition| end_condition_met(compiled, state, condition))
}

fn end_condition_met(
    compiled: &CompiledScenario,
    state: &EngineState,
    condition: &EndConditionSpec,
) -> bool {
    match condition {
        EndConditionSpec::MaxSteps { steps } => state.step >= *steps,
        EndConditionSpec::MetricAtLeast { metric, value_scaled } => {
            to_scaled_i64(metric_value(compiled, state, metric)) >= *value_scaled
        }
        EndConditionSpec::MetricAtMost { metric, value_scaled } => {
            to_scaled_i64(metric_value(compiled, state, metric)) <= *value_scaled
        }
        EndConditionSpec::NodeAtLeast { node_id, value_scaled } => {
            to_scaled_i64(node_value(compiled, state, node_id)) >= *value_scaled
        }
        EndConditionSpec::NodeAtMost { node_id, value_scaled } => {
            to_scaled_i64(node_value(compiled, state, node_id)) <= *value_scaled
        }
        EndConditionSpec::Any(conditions) => {
            conditions.iter().any(|nested| end_condition_met(compiled, state, nested))
        }
        EndConditionSpec::All(conditions) => {
            conditions.iter().all(|nested| end_condition_met(compiled, state, nested))
        }
    }
}

/// Rejects typed capture selections that reference nothing in the compiled scenario.
/// An unknown metric key would otherwise resolve to `0.0` via
/// `metric_value` and emit a fabricated all-zero series under the wrong label; an
/// unknown node id would be silently ignored. A capture metric is valid
/// if it resolves to a node (node-backed metric) or is a tracked metric, mirroring
/// what `metric_value` can actually resolve.
fn validate_capture_selection(
    compiled: &CompiledScenario,
    config: &RunConfig,
) -> Result<(), RunError> {
    config.capture.validate().map_err(|reason| RunError::InvalidRunConfig {
        name: "run.capture".to_string(),
        reason: reason.to_string(),
    })?;

    if let Selection::Only(node_ids) = config.capture.nodes() {
        for node_id in node_ids {
            if compiled.node_index(node_id).is_none() {
                return Err(RunError::InvalidRunConfig {
                    name: format!("run.capture.nodes.{node_id}"),
                    reason: "references an unknown node".to_string(),
                });
            }
        }
    }

    if let Selection::Only(metrics) = config.capture.metrics() {
        for metric in metrics {
            let resolves = metric_node_index(compiled, metric).is_some()
                || compiled.tracked_metrics().contains(metric);
            if !resolves {
                return Err(RunError::InvalidRunConfig {
                    name: format!("run.capture.metrics.{metric}"),
                    reason: "does not resolve to a tracked metric or node".to_string(),
                });
            }
        }
    }

    if let Selection::Only(variables) = config.capture.variables() {
        for variable in variables {
            if !compiled.variables().sources.contains_key(variable) {
                return Err(RunError::InvalidRunConfig {
                    name: format!("run.capture.variables.{variable}"),
                    reason: "references an unknown scenario variable".to_string(),
                });
            }
        }
    }

    if let Selection::Only(edges) = config.capture.transfers() {
        for edge in edges {
            if compiled.edge_index(edge).is_none() {
                return Err(RunError::InvalidRunConfig {
                    name: format!("run.capture.transfers.{edge}"),
                    reason: "references an unknown edge".to_string(),
                });
            }
        }
    }

    Ok(())
}

/// Resolves concrete batch aggregation metrics against the finalized plan.
///
/// This deliberately runs once at batch entry; compact collectors consume the
/// resolved values for every derived seed and never look up compiled metrics.
pub(crate) fn resolve_aggregation_selection(
    compiled: &CompiledScenario,
    aggregation: &AggregationConfig,
) -> Result<ResolvedAggregationSelection, RunError> {
    aggregation.validate().map_err(|reason| RunError::InvalidRunConfig {
        name: "batch.aggregation".to_string(),
        reason: reason.to_string(),
    })?;

    match aggregation.metrics() {
        Selection::None => Ok(ResolvedAggregationSelection::None),
        Selection::All => Ok(ResolvedAggregationSelection::All),
        Selection::Only(metrics) => metrics
            .iter()
            .map(|metric| {
                let source = if let Some(index) = metric_node_index(compiled, metric) {
                    ResolvedAggregationMetricSource::Node(index)
                } else if compiled.tracked_metrics().contains(metric) {
                    ResolvedAggregationMetricSource::TrackedTotal
                } else {
                    return Err(RunError::InvalidRunConfig {
                        name: format!("batch.aggregation.metrics.{metric}"),
                        reason: "does not resolve to a tracked metric or node".to_string(),
                    });
                };
                Ok(ResolvedAggregationMetric { metric: metric.clone(), source })
            })
            .collect::<Result<Vec<_>, _>>()
            .map(ResolvedAggregationSelection::Only),
    }
}

fn should_capture_step(capture: &CaptureConfig, step: u64, force: bool) -> bool {
    if force {
        return true;
    }

    match capture.schedule() {
        CaptureSchedule::None | CaptureSchedule::Final => false,
        CaptureSchedule::Every { stride, include_initial, .. } if step == 0 => *include_initial,
        CaptureSchedule::Every { stride, .. } => step % stride.get() == 0,
    }
}

fn should_capture_aggregation_step(
    aggregation: &AggregationConfig,
    step: u64,
    force: bool,
) -> bool {
    if force {
        return true;
    }

    match aggregation.schedule() {
        CaptureSchedule::None | CaptureSchedule::Final => false,
        CaptureSchedule::Every { stride, include_initial, .. } if step == 0 => *include_initial,
        CaptureSchedule::Every { stride, .. } => step % stride.get() == 0,
    }
}

fn captures_final(schedule: &CaptureSchedule) -> bool {
    matches!(schedule, CaptureSchedule::Final | CaptureSchedule::Every { include_final: true, .. })
}

fn selected<T: Ord>(selection: &Selection<T>, value: &T) -> bool {
    matches!(selection, Selection::All)
        || matches!(selection, Selection::Only(values) if values.contains(value))
}

fn canonicalize_float(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }

    let rounded = (value * VALUE_SCALE).round() / VALUE_SCALE;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}

fn to_scaled_i64(value: f64) -> i64 {
    let scaled = (canonicalize_float(value) * VALUE_SCALE).round();
    if !scaled.is_finite() {
        return if scaled.is_sign_negative() { i64::MIN } else { i64::MAX };
    }
    if scaled > i64::MAX as f64 {
        i64::MAX
    } else if scaled < i64::MIN as f64 {
        i64::MIN
    } else {
        scaled as i64
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use crate::error::RunError;
    use crate::rng::rng_from_seed;
    use crate::stochastic::{sample_closed_interval, sample_from_list, sample_from_matrix};
    use crate::types::{
        ActionMode, CaptureConfig, ConnectionKind, DelayNodeConfig, EdgeConnectionConfig, EdgeId,
        EdgeSpec, EndConditionSpec, MetricKey, NodeConfig, NodeId, NodeKind, NodeModeConfig,
        NodeSpec, PoolNodeConfig, QueueNodeConfig, RunConfig, ScenarioId, ScenarioSpec, Selection,
        StateConnectionConfig, StateConnectionRole, StateConnectionTarget, TransferSpec,
        TriggerMode, VariableRuntimeConfig, VariableSourceSpec, VariableUpdateTiming,
    };
    use crate::validation::compile_scenario;

    use super::{run_single, GateRoutingLane, GateRuntimeState, VALUE_SCALE, VARIABLE_RNG_SALT};

    #[test]
    fn run_single_is_deterministic_for_same_inputs() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let metric_sink = MetricKey::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-deterministic"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
        scenario.tracked_metrics.insert(metric_sink);

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 42, max_steps: 10, capture: CaptureConfig::default() };

        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.steps_executed, 3);
        assert!(report_a.completed);
    }

    #[test]
    fn run_single_respects_compiled_edge_order() {
        let pool = NodeId::fixture("pool");
        let sink_a = NodeId::fixture("sink-a");
        let sink_b = NodeId::fixture("sink-b");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-edge-order"))
            .with_node(NodeSpec::new(pool.clone(), NodeKind::Process).with_initial_value(10.0))
            .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Sink))
            .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                pool.clone(),
                sink_b.clone(),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                pool.clone(),
                sink_a.clone(),
                TransferSpec::Fraction { numerator: 1, denominator: 2 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 1, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&pool), Some(&0.0));
        assert_eq!(report.final_node_values.get(&sink_a), Some(&5.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&5.0));
        assert_eq!(report.steps_executed, 1);
    }

    #[test]
    fn run_single_metric_scaled_edges_observe_intra_step_updates() {
        let source = NodeId::fixture("source");
        let sink_a = NodeId::fixture("sink-a");
        let sink_b = NodeId::fixture("sink-b");
        let metric_a = MetricKey::fixture("sink-a");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-intra-step-metric"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(20.0))
            .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Sink).with_initial_value(4.0))
            .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-1"),
                source.clone(),
                sink_a.clone(),
                TransferSpec::MetricScaled { metric: metric_a.clone(), factor: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-2"),
                source.clone(),
                sink_b.clone(),
                TransferSpec::MetricScaled { metric: metric_a.clone(), factor: 1.0 },
            ));
        scenario.tracked_metrics.insert(metric_a);
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 1, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&sink_a), Some(&8.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&8.0));
        assert_eq!(report.final_node_values.get(&source), Some(&8.0));
        assert_eq!(report.steps_executed, 1);
    }

    #[test]
    fn deterministic_gate_balancer_uses_lane_identity_not_position() {
        let gate_id = NodeId::fixture("gate");
        let mut gates = GateRuntimeState::from_seed(42);

        let lane = |edge_id: &str| GateRoutingLane {
            edge_id: Some(EdgeId::fixture(edge_id)),
            to_index: Some(0),
            weight: 1.0,
        };

        let first = gates
            .pick_deterministic_target(&gate_id, &[lane("edge-a"), lane("edge-b")])
            .expect("first pick should exist");
        assert_eq!(first, 0, "first tie should pick the first lane");

        let second = gates
            .pick_deterministic_target(&gate_id, &[lane("edge-c"), lane("edge-d")])
            .expect("second pick should exist");
        assert_eq!(
            second, 0,
            "new lane identities should not inherit prior index-scoped balancer history"
        );
    }

    #[test]
    fn run_single_sorting_gate_skips_zero_fraction_lane_keeps_weighted_routing() {
        let gate = NodeId::fixture("gate");
        let sink_a = NodeId::fixture("sink-a");
        let sink_b = NodeId::fixture("sink-b");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-zero-fraction-gate"))
            .with_node(NodeSpec::new(gate.clone(), NodeKind::SortingGate).with_initial_value(4.0))
            .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Pool))
            .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                gate.clone(),
                sink_a.clone(),
                TransferSpec::Fraction { numerator: 50, denominator: 100 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                gate.clone(),
                sink_b.clone(),
                TransferSpec::Fraction { numerator: 0, denominator: 100 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 7, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&sink_a), Some(&2.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&0.0));
        assert_eq!(report.final_node_values.get(&gate), Some(&0.0));
    }

    #[test]
    fn run_single_push_all_group_skips_zero_request_edge() {
        let node_a = NodeId::fixture("a-source");
        let node_b = NodeId::fixture("b-sink");
        let node_c = NodeId::fixture("c-sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-push-all-zero"))
            .with_node(pool_with_mode(
                "a-source",
                10.0,
                TriggerMode::Automatic,
                ActionMode::PushAll,
            ))
            .with_node(NodeSpec::new(node_b.clone(), NodeKind::Pool))
            .with_node(NodeSpec::new(node_c.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-1"),
                node_a.clone(),
                node_b.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-2"),
                node_a.clone(),
                node_c.clone(),
                TransferSpec::Fraction { numerator: 0, denominator: 1 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 12, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&node_a), Some(&8.0));
        assert_eq!(report.final_node_values.get(&node_b), Some(&2.0));
        assert_eq!(report.final_node_values.get(&node_c), Some(&0.0));
    }

    #[test]
    fn run_single_resource_transfer_quantizes_by_token_size() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-token-size"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(5.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    EdgeId::fixture("edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Fixed { amount: 3.0 },
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::Resource,
                    resource: crate::types::ResourceConnectionConfig { token_size: 2 },
                    state: StateConnectionConfig::default(),
                }),
            );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 2, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&3.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&2.0));
        assert_eq!(report.steps_executed, 1);
    }

    #[test]
    fn run_single_transfer_expression_formula_is_deterministic() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-transfer-expression"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(5.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "min(available, next_step + 1)".to_string() },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 8, max_steps: 10, capture: CaptureConfig::final_only() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report_a.final_node_values.get(&sink), Some(&5.0));
    }

    #[test]
    fn run_single_transfer_expression_unknown_variable_returns_error() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario =
            ScenarioSpec::new(ScenarioId::fixture("scenario-transfer-expression-error"))
                .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(5.0))
                .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
                .with_edge(EdgeSpec::new(
                    EdgeId::fixture("edge"),
                    source,
                    sink,
                    TransferSpec::Expression { formula: "missing + 1".to_string() },
                ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 8, max_steps: 10, capture: CaptureConfig::final_only() };
        let error = run_single(&compiled, &config).expect_err("unknown variable must fail");

        match error {
            RunError::InvalidRunConfig { name, reason } => {
                assert_eq!(name, "edges.edge.transfer.expression.formula");
                assert!(reason.contains("unknown variable `missing`"));
            }
            other => panic!("expected InvalidRunConfig, got {other:?}"),
        }
    }

    #[test]
    fn run_single_variable_random_interval_run_start_refreshes_once() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let seed = 171_u64;

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-variable-run-start"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(20.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "roll".to_string() },
            ));
        scenario.variables = VariableRuntimeConfig {
            update_timing: VariableUpdateTiming::RunStart,
            sources: BTreeMap::from([(
                "roll".to_string(),
                VariableSourceSpec::RandomInterval { min: 1, max: 3 },
            )]),
        };
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];

        let mut expected_rng = rng_from_seed(seed ^ VARIABLE_RNG_SALT);
        let roll = sample_closed_interval(1, 3, &mut expected_rng).expect("valid interval");

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::final_only() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&sink), Some(&(roll * 3.0)));
        assert_eq!(report_a.final_node_values.get(&source), Some(&(20.0 - roll * 3.0)));
    }

    #[test]
    fn run_single_variable_random_interval_every_step_refreshes_each_step() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let seed = 272_u64;

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-variable-every-step"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(20.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "roll".to_string() },
            ));
        scenario.variables = VariableRuntimeConfig {
            update_timing: VariableUpdateTiming::EveryStep,
            sources: BTreeMap::from([(
                "roll".to_string(),
                VariableSourceSpec::RandomInterval { min: 1, max: 3 },
            )]),
        };
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];

        let mut expected_rng = rng_from_seed(seed ^ VARIABLE_RNG_SALT);
        let expected_total = (0..3)
            .map(|_| sample_closed_interval(1, 3, &mut expected_rng).expect("valid interval"))
            .sum::<f64>();

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&sink), Some(&expected_total));
        assert_eq!(report.final_node_values.get(&source), Some(&(20.0 - expected_total)));
    }

    #[test]
    fn run_single_variable_list_matrix_sampling_is_seed_stable() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let seed = 373_u64;

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-variable-list-matrix"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(50.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "list_pick + matrix_pick".to_string() },
            ));
        scenario.variables = VariableRuntimeConfig {
            update_timing: VariableUpdateTiming::EveryStep,
            sources: BTreeMap::from([
                (
                    "list_pick".to_string(),
                    VariableSourceSpec::RandomList { values: vec![1.0, 3.0, 5.0] },
                ),
                (
                    "matrix_pick".to_string(),
                    VariableSourceSpec::RandomMatrix { values: vec![vec![2.0, 4.0], vec![6.0]] },
                ),
            ]),
        };
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

        let mut expected_rng = rng_from_seed(seed ^ VARIABLE_RNG_SALT);
        let expected_total = (0..2)
            .map(|_| {
                let list = sample_from_list(&[1.0, 3.0, 5.0], &mut expected_rng)
                    .expect("valid list source");
                let matrix = sample_from_matrix(&[vec![2.0, 4.0], vec![6.0]], &mut expected_rng)
                    .expect("valid matrix source");
                list + matrix
            })
            .sum::<f64>();

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::final_only() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&sink), Some(&expected_total));
        assert_eq!(report_a.final_node_values.get(&source), Some(&(50.0 - expected_total)));
    }

    #[test]
    fn run_single_applies_state_modifier_to_next_step_deterministically() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-state-modifier"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    EdgeId::fixture("state-edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Modifier,
                        formula: "+1".to_string(),
                        target: StateConnectionTarget::Node,
                        target_connection: None,
                        resource_filter: None,
                    },
                }),
            );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 7, max_steps: 5, capture: CaptureConfig::final_only() };

        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&source), Some(&2.0));
        assert_eq!(report_a.final_node_values.get(&sink), Some(&4.0));
        assert_eq!(report_a.steps_executed, 2);
    }

    #[test]
    fn run_single_state_modifier_unknown_variable_returns_error() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario =
            ScenarioSpec::new(ScenarioId::fixture("scenario-state-expression-error"))
                .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(2.0))
                .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
                .with_edge(
                    EdgeSpec::new(
                        EdgeId::fixture("state-edge"),
                        source,
                        sink,
                        TransferSpec::Remaining,
                    )
                    .with_connection(EdgeConnectionConfig {
                        kind: ConnectionKind::State,
                        resource: Default::default(),
                        state: StateConnectionConfig {
                            role: StateConnectionRole::Modifier,
                            formula: "+missing".to_string(),
                            target: StateConnectionTarget::Node,
                            target_connection: None,
                            resource_filter: None,
                        },
                    }),
                );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 7, max_steps: 5, capture: CaptureConfig::final_only() };
        let error = run_single(&compiled, &config).expect_err("unknown variable must fail");

        match error {
            RunError::InvalidRunConfig { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.formula");
                assert!(reason.contains("unknown variable `missing`"));
            }
            other => panic!("expected InvalidRunConfig, got {other:?}"),
        }
    }

    #[test]
    fn run_single_state_formula_expression_uses_deterministic_graph_values() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario =
            ScenarioSpec::new(ScenarioId::fixture("scenario-state-expression-graph"))
                .with_node(NodeSpec::new(source.clone(), NodeKind::Process).with_initial_value(1.0))
                .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
                .with_edge(
                    EdgeSpec::new(
                        EdgeId::fixture("state-edge"),
                        source.clone(),
                        sink.clone(),
                        TransferSpec::Remaining,
                    )
                    .with_connection(EdgeConnectionConfig {
                        kind: ConnectionKind::State,
                        resource: Default::default(),
                        state: StateConnectionConfig {
                            role: StateConnectionRole::Modifier,
                            formula: "+next_step".to_string(),
                            target: StateConnectionTarget::Node,
                            target_connection: None,
                            resource_filter: None,
                        },
                    }),
                );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 16, max_steps: 10, capture: CaptureConfig::final_only() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&source), Some(&1.0));
        assert_eq!(report_a.final_node_values.get(&sink), Some(&3.0));
    }

    #[test]
    fn run_single_passive_trigger_mode_fires_on_state_trigger() {
        let trigger = NodeId::fixture("trigger");
        let actor = NodeId::fixture("actor");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-passive-trigger"))
            .with_node(NodeSpec::new(trigger.clone(), NodeKind::Process).with_initial_value(1.0))
            .with_node(pool_with_mode("actor", 3.0, TriggerMode::Passive, ActionMode::PushAny))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("resource-edge"),
                actor.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    EdgeId::fixture("state-trigger"),
                    trigger,
                    actor.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Trigger,
                        formula: "*".to_string(),
                        target: StateConnectionTarget::Node,
                        target_connection: None,
                        resource_filter: None,
                    },
                }),
            );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 9, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&actor), Some(&2.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&1.0));
    }

    #[test]
    fn run_single_trigger_gate_fires_passive_target_sorted_before_gate() {
        let actor = NodeId::fixture("actor");
        let gate = NodeId::fixture("gate");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-gate-trigger-order"))
            .with_node(pool_with_mode("actor", 5.0, TriggerMode::Passive, ActionMode::PushAny))
            .with_node(NodeSpec::new(gate.clone(), NodeKind::TriggerGate))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("resource-edge"),
                actor.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    EdgeId::fixture("state-trigger"),
                    gate,
                    actor.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Trigger,
                        formula: "*".to_string(),
                        target: StateConnectionTarget::Node,
                        target_connection: None,
                        resource_filter: None,
                    },
                }),
            );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 11, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&actor), Some(&4.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&1.0));
    }

    #[test]
    fn run_single_interactive_trigger_mode_does_not_fire_without_input() {
        let actor = NodeId::fixture("actor");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-interactive"))
            .with_node(pool_with_mode("actor", 3.0, TriggerMode::Interactive, ActionMode::PushAny))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("resource-edge"),
                actor.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 10, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&actor), Some(&3.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&0.0));
    }

    #[test]
    fn run_single_enabling_trigger_mode_fires_at_start_only() {
        let actor = NodeId::fixture("actor");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-enabling"))
            .with_node(pool_with_mode("actor", 3.0, TriggerMode::Enabling, ActionMode::PushAny))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("resource-edge"),
                actor.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 2 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 11, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&actor), Some(&2.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&1.0));
        assert_eq!(report.steps_executed, 2);
    }

    #[test]
    fn run_single_push_all_requires_full_amounts() {
        let source = NodeId::fixture("source");
        let sink_a = NodeId::fixture("sink-a");
        let sink_b = NodeId::fixture("sink-b");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-push-all"))
            .with_node(pool_with_mode("source", 3.0, TriggerMode::Automatic, ActionMode::PushAll))
            .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Pool))
            .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                source.clone(),
                sink_a.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                source.clone(),
                sink_b.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 12, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&3.0));
        assert_eq!(report.final_node_values.get(&sink_a), Some(&0.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&0.0));
    }

    #[test]
    fn run_single_push_any_uses_available_amounts() {
        let source = NodeId::fixture("source");
        let sink_a = NodeId::fixture("sink-a");
        let sink_b = NodeId::fixture("sink-b");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-push-any"))
            .with_node(pool_with_mode("source", 3.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(sink_a.clone(), NodeKind::Pool))
            .with_node(NodeSpec::new(sink_b.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                source.clone(),
                sink_a.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                source.clone(),
                sink_b.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 13, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report.final_node_values.get(&sink_a), Some(&2.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&1.0));
    }

    #[test]
    fn run_single_pull_all_requires_full_inputs() {
        let source_a = NodeId::fixture("source-a");
        let source_b = NodeId::fixture("source-b");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-pull-all"))
            .with_node(NodeSpec::new(source_a.clone(), NodeKind::Pool).with_initial_value(2.0))
            .with_node(NodeSpec::new(source_b.clone(), NodeKind::Pool).with_initial_value(1.0))
            .with_node(pool_with_mode("sink", 0.0, TriggerMode::Automatic, ActionMode::PullAll))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                source_a.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                source_b.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 14, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source_a), Some(&2.0));
        assert_eq!(report.final_node_values.get(&source_b), Some(&1.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&0.0));
    }

    #[test]
    fn run_single_pull_any_uses_available_inputs() {
        let source_a = NodeId::fixture("source-a");
        let source_b = NodeId::fixture("source-b");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-pull-any"))
            .with_node(NodeSpec::new(source_a.clone(), NodeKind::Pool).with_initial_value(2.0))
            .with_node(NodeSpec::new(source_b.clone(), NodeKind::Pool).with_initial_value(1.0))
            .with_node(pool_with_mode("sink", 0.0, TriggerMode::Automatic, ActionMode::PullAny))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                source_a.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-b"),
                source_b.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 2.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 15, max_steps: 5, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source_a), Some(&0.0));
        assert_eq!(report.final_node_values.get(&source_b), Some(&0.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&3.0));
    }

    #[test]
    fn run_single_delay_node_releases_resources_after_configured_delay() {
        let source = NodeId::fixture("source");
        let delay = NodeId::fixture("delay");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-delay-timeline"))
            .with_node(pool_with_mode("source", 3.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(delay.clone(), NodeKind::Delay).with_config(
                NodeConfig::Delay(DelayNodeConfig {
                    delay_steps: 2,
                    mode: NodeModeConfig {
                        trigger_mode: TriggerMode::Automatic,
                        action_mode: ActionMode::PushAny,
                    },
                }),
            ))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-source-delay"),
                source.clone(),
                delay.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-delay-sink"),
                delay.clone(),
                sink.clone(),
                TransferSpec::Remaining,
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 4 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 31, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report.final_node_values.get(&delay), Some(&1.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&2.0));
    }

    #[test]
    fn run_single_queue_releases_one_resource_per_step() {
        let source = NodeId::fixture("source");
        let queue = NodeId::fixture("queue");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-queue-timeline"))
            .with_node(pool_with_mode("source", 3.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(queue.clone(), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: None,
                    release_per_step: 1,
                    mode: NodeModeConfig {
                        trigger_mode: TriggerMode::Automatic,
                        action_mode: ActionMode::PushAny,
                    },
                }),
            ))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-source-queue"),
                source.clone(),
                queue.clone(),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-queue-sink"),
                queue.clone(),
                sink.clone(),
                TransferSpec::Remaining,
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 32, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report.final_node_values.get(&queue), Some(&1.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&2.0));
    }

    #[test]
    fn run_single_rejects_unresolvable_capture_keys() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let metric_sink = MetricKey::fixture("sink");

        let build = || {
            let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-capture-validate"))
                .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
                .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
                .with_edge(EdgeSpec::new(
                    EdgeId::fixture("edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Fixed { amount: 1.0 },
                ));
            scenario.tracked_metrics.insert(metric_sink.clone());
            scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];
            compile_scenario(&scenario).expect("scenario should compile")
        };

        let compiled = build();
        let capture = CaptureConfig::default()
            .with_metrics(Selection::Only(BTreeSet::from([MetricKey::fixture("snk")])));
        let config = RunConfig { seed: 1, max_steps: 5, capture };
        match run_single(&compiled, &config) {
            Err(RunError::InvalidRunConfig { name, .. }) => {
                assert_eq!(name, "run.capture.metrics.snk");
            }
            other => panic!("expected InvalidRunConfig, got {other:?}"),
        }

        let capture = CaptureConfig::default()
            .with_nodes(Selection::Only(BTreeSet::from([NodeId::fixture("nope")])));
        let config = RunConfig { seed: 1, max_steps: 5, capture };
        match run_single(&compiled, &config) {
            Err(RunError::InvalidRunConfig { name, .. }) => {
                assert_eq!(name, "run.capture.nodes.nope");
            }
            other => panic!("expected InvalidRunConfig, got {other:?}"),
        }

        let capture = CaptureConfig::default()
            .with_metrics(Selection::Only(BTreeSet::from([metric_sink.clone()])))
            .with_nodes(Selection::Only(BTreeSet::from([sink.clone()])));
        let config = RunConfig { seed: 1, max_steps: 5, capture };
        run_single(&compiled, &config).expect("valid capture keys should run");
    }

    #[test]
    fn run_single_pool_capacity_bounds_stored_value() {
        let source = NodeId::fixture("source");
        let pool = NodeId::fixture("pool");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-pool-capacity"))
            .with_node(pool_with_mode("source", 100.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(pool.clone(), NodeKind::Pool).with_config(NodeConfig::Pool(
                PoolNodeConfig {
                    capacity: Some(10),
                    allow_negative_start: false,
                    mode: NodeModeConfig::default(),
                },
            )))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-source-pool"),
                source.clone(),
                pool.clone(),
                TransferSpec::Fixed { amount: 5.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 34, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&pool), Some(&10.0));
        assert_eq!(report.final_node_values.get(&source), Some(&90.0));
    }

    #[test]
    fn run_single_queue_capacity_bounds_held_inventory() {
        let source = NodeId::fixture("source");
        let queue = NodeId::fixture("queue");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-queue-capacity"))
            .with_node(pool_with_mode("source", 5.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(queue.clone(), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: Some(2),
                    release_per_step: 1,
                    mode: NodeModeConfig {
                        trigger_mode: TriggerMode::Automatic,
                        action_mode: ActionMode::PushAny,
                    },
                }),
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-source-queue"),
                source.clone(),
                queue.clone(),
                TransferSpec::Remaining,
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 33, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&queue), Some(&2.0));
        assert_eq!(report.final_node_values.get(&source), Some(&3.0));
    }

    #[test]
    fn run_single_delay_queue_timeline_replay_is_deterministic() {
        let source = NodeId::fixture("source");
        let delay = NodeId::fixture("delay");
        let queue = NodeId::fixture("queue");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-delay-queue-replay"))
            .with_node(pool_with_mode("source", 4.0, TriggerMode::Automatic, ActionMode::PushAny))
            .with_node(NodeSpec::new(delay.clone(), NodeKind::Delay).with_config(
                NodeConfig::Delay(DelayNodeConfig {
                    delay_steps: 2,
                    mode: NodeModeConfig {
                        trigger_mode: TriggerMode::Automatic,
                        action_mode: ActionMode::PushAny,
                    },
                }),
            ))
            .with_node(NodeSpec::new(queue.clone(), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: None,
                    release_per_step: 1,
                    mode: NodeModeConfig {
                        trigger_mode: TriggerMode::Automatic,
                        action_mode: ActionMode::PushAny,
                    },
                }),
            ))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-source-delay"),
                source.clone(),
                delay.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-delay-queue"),
                delay.clone(),
                queue.clone(),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-queue-sink"),
                queue.clone(),
                sink.clone(),
                TransferSpec::Remaining,
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 6 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 33, max_steps: 10, capture: CaptureConfig::final_only() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&sink), Some(&3.0));
        assert_eq!(report_a.final_node_values.get(&queue), Some(&1.0));
    }

    #[test]
    fn run_single_stops_on_node_end_condition() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-node-end"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::NodeAtLeast {
            node_id: sink.clone(),
            value_scaled: scaled(2.0),
        }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 3, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.steps_executed, 2);
        assert!(report.completed);
        assert_eq!(report.final_node_values.get(&sink), Some(&2.0));
    }

    #[test]
    fn run_single_stops_on_nested_metric_end_condition() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let metric_sink = MetricKey::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-metric-end"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source,
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.tracked_metrics.insert(metric_sink.clone());
        scenario.end_conditions = vec![EndConditionSpec::All(vec![
            EndConditionSpec::MetricAtLeast { metric: metric_sink, value_scaled: scaled(2.0) },
            EndConditionSpec::Any(vec![
                EndConditionSpec::NodeAtLeast { node_id: sink.clone(), value_scaled: scaled(2.0) },
                EndConditionSpec::MaxSteps { steps: 99 },
            ]),
        ])];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 4, max_steps: 10, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.steps_executed, 2);
        assert!(report.completed);
        assert_eq!(report.final_metrics.get(&MetricKey::fixture("sink")), Some(&2.0));
    }

    #[test]
    fn run_single_stops_at_run_max_steps_when_end_condition_is_not_met() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-run-max"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink, NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source,
                NodeId::fixture("sink"),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 10 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 5, max_steps: 3, capture: CaptureConfig::final_only() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.steps_executed, 3);
        assert!(!report.completed);
    }

    #[test]
    fn run_single_capture_respects_step_zero_interval_and_final_without_duplicates() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let metric_sink = MetricKey::fixture("sink");

        let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-capture"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink, NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge"),
                source,
                NodeId::fixture("sink"),
                TransferSpec::Fixed { amount: 1.0 },
            ));
        scenario.tracked_metrics.insert(metric_sink.clone());
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 6, max_steps: 10, capture: CaptureConfig::default() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        let snapshot_steps =
            report.node_snapshots.iter().map(|snapshot| snapshot.step).collect::<Vec<_>>();
        assert_eq!(snapshot_steps, vec![0, 1]);

        let metric_steps = report
            .series
            .get(&metric_sink)
            .expect("tracked metric should be captured")
            .points
            .iter()
            .map(|point| point.step)
            .collect::<Vec<_>>();
        assert_eq!(metric_steps, vec![0, 1]);
    }

    fn scaled(value: f64) -> i64 {
        (value * VALUE_SCALE).round() as i64
    }

    fn pool_with_mode(
        id: &str,
        initial_value: f64,
        trigger_mode: TriggerMode,
        action_mode: ActionMode,
    ) -> NodeSpec {
        NodeSpec::new(NodeId::fixture(id), NodeKind::Pool)
            .with_initial_value(initial_value)
            .with_config(NodeConfig::Pool(PoolNodeConfig {
                capacity: None,
                allow_negative_start: false,
                mode: NodeModeConfig { trigger_mode, action_mode },
            }))
    }
}
