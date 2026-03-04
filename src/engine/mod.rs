//! Deterministic simulation engine internals.

use std::collections::{BTreeMap, BTreeSet};

use crate::error::RunError;
use crate::events::{
    EventSink, EventSinkError, MetricSnapshotEvent, RunEvent, StepEndEvent, StepStartEvent,
    TransferEvent,
};
use crate::expr::{CompiledExpr, ExprRuntime};
use crate::rng::{rng_from_seed, BaseRng};
use crate::stochastic::{
    sample_chance_percent, sample_closed_interval, sample_from_list, sample_from_matrix,
    sample_weighted_index,
};
use crate::types::{
    ActionMode, ConnectionKind, EdgeId, EndConditionSpec, MetricKey, NodeConfig, NodeId, NodeKind,
    NodeModeConfig, NodeSnapshot, RunConfig, RunReport, SeriesPoint, SeriesTable,
    StateConnectionRole, StateConnectionTarget, TransferRecord, TransferSpec, TriggerMode,
    VariableSnapshot, VariableSourceSpec, VariableUpdateTiming,
};
use crate::validation::CompiledScenario;

const VALUE_SCALE: f64 = 1_000_000.0;
const VARIABLE_RNG_SALT: u64 = 0xA11C_E5E0_0023_0001;
const GATE_RNG_SALT: u64 = 0xA11C_E5E0_0023_0002;

#[derive(Debug, Clone, PartialEq)]
/// Mutable execution state for one run.
pub struct EngineState {
    pub step: u64,
    pub node_values: Vec<f64>,
    pub metrics: BTreeMap<MetricKey, f64>,
}

#[derive(Debug)]
struct VariableRuntimeState {
    timing: VariableUpdateTiming,
    sources: BTreeMap<String, VariableSourceSpec>,
    values: BTreeMap<String, f64>,
    rng: BaseRng,
}

impl VariableRuntimeState {
    fn from_compiled(compiled: &CompiledScenario, seed: u64) -> Self {
        Self {
            timing: compiled.scenario.variables.update_timing.clone(),
            sources: compiled.scenario.variables.sources.clone(),
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

#[derive(Debug, Default)]
struct EngineExpressionCache {
    transfer_by_edge: BTreeMap<EdgeId, Option<CompiledExpr>>,
    state_by_edge: BTreeMap<EdgeId, Option<CompiledExpr>>,
}

impl EngineExpressionCache {
    fn from_compiled(compiled: &CompiledScenario, runtime: &ExprRuntime) -> Self {
        let mut cache = Self::default();

        for edge_id in &compiled.edge_order {
            let Some(edge) = compiled.scenario.edges.get(edge_id) else {
                continue;
            };
            if !edge.enabled {
                continue;
            }

            if let TransferSpec::Expression { formula } = &edge.transfer {
                cache.transfer_by_edge.insert(edge_id.clone(), runtime.compile(formula).ok());
            }

            if matches!(edge.connection.kind, ConnectionKind::State) {
                cache
                    .state_by_edge
                    .insert(edge_id.clone(), runtime.compile(&edge.connection.state.formula).ok());
            }
        }

        cache
    }

    fn transfer_expression(&self, edge_id: &EdgeId) -> Option<&CompiledExpr> {
        self.transfer_by_edge.get(edge_id).and_then(|compiled| compiled.as_ref())
    }

    fn state_expression(&self, edge_id: &EdgeId) -> Option<&CompiledExpr> {
        self.state_by_edge.get(edge_id).and_then(|compiled| compiled.as_ref())
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

fn edge_to_node_id(compiled: &CompiledScenario, edge_id: &EdgeId) -> NodeId {
    compiled
        .scenario
        .edges
        .get(edge_id)
        .map(|edge| edge.to.clone())
        .unwrap_or_else(|| NodeId::fixture("unknown-node"))
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

        for (index, node_id) in compiled.node_order.iter().enumerate() {
            let value =
                canonicalize_float(state.node_values.get(index).copied().unwrap_or(0.0).max(0.0));
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

        for node_id in &compiled.node_order {
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
                    let per_step = queue_release_per_step_for_node(compiled, node_id).max(1) as f64;
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
    ) -> Option<f64> {
        timeline_node_kind(compiled, node_id)?;

        let budget =
            canonicalize_float(self.release_budgets.get(node_id).copied().unwrap_or(0.0).max(0.0));
        let available = compiled
            .node_index_by_id
            .get(node_id)
            .and_then(|index| state.node_values.get(*index))
            .copied()
            .unwrap_or(0.0)
            .max(0.0);
        Some(canonicalize_float(available.min(budget)))
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

/// Initializes engine state from compiled scenario defaults.
pub fn init_state(compiled: &CompiledScenario) -> EngineState {
    let node_values = compiled
        .node_order
        .iter()
        .map(|node_id| {
            let node = compiled
                .scenario
                .nodes
                .get(node_id)
                .expect("compiled.node_order must reference known nodes");
            canonicalize_float(node.initial_value)
        })
        .collect::<Vec<_>>();

    let metrics = compiled
        .scenario
        .tracked_metrics
        .iter()
        .cloned()
        .map(|metric| (metric, 0.0))
        .collect::<BTreeMap<_, _>>();

    let mut state = EngineState { step: 0, node_values, metrics };
    refresh_metrics(compiled, &mut state);
    state
}

/// Runs one deterministic simulation from initial state to completion.
pub fn run_single(compiled: &CompiledScenario, config: &RunConfig) -> Result<RunReport, RunError> {
    let mut emit = |_event: RunEvent| Ok(());
    run_single_internal(compiled, config, "run-0", false, &mut emit)
}

/// Runs one deterministic simulation and streams run events while execution progresses.
pub(crate) fn run_single_streaming(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    sink: &mut dyn EventSink,
) -> Result<RunReport, RunError> {
    let mut emit = |event: RunEvent| sink.push(event).map_err(map_event_sink_error);
    run_single_internal(compiled, config, run_id, false, &mut emit)
}

/// Runs one deterministic simulation and streams run events while deferring the terminal step_end.
///
/// Intended for `run_with_assertions`, where assertion checkpoints should be emitted at the
/// terminal step before emitting the final `step_end`.
pub(crate) fn run_single_streaming_for_assertions(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    sink: &mut dyn EventSink,
) -> Result<RunReport, RunError> {
    let mut emit = |event: RunEvent| sink.push(event).map_err(map_event_sink_error);
    run_single_internal(compiled, config, run_id, true, &mut emit)
}

fn run_single_internal(
    compiled: &CompiledScenario,
    config: &RunConfig,
    run_id: &str,
    defer_terminal_step_end: bool,
    emit_event: &mut dyn FnMut(RunEvent) -> Result<(), RunError>,
) -> Result<RunReport, RunError> {
    let mut report = RunReport::new(compiled.scenario.id.clone(), config.seed);
    let mut state = init_state(compiled);
    let runtime = ExprRuntime::new();
    let expression_cache = EngineExpressionCache::from_compiled(compiled, &runtime);
    let step_plan = EngineStepPlan::from_compiled(compiled);
    let mut variables = VariableRuntimeState::from_compiled(compiled, config.seed);
    let mut gates = GateRuntimeState::from_seed(config.seed);
    let mut timeline = TimelineRuntimeState::from_compiled(compiled, &state);
    let mut transfer_log = Vec::<TransferRecord>::new();
    variables.refresh_initial();
    let mut captured_steps = BTreeSet::new();

    capture_step(
        compiled,
        config,
        &state,
        variables.values(),
        &mut report,
        &mut captured_steps,
        false,
    );

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
        let transfer_start = transfer_log.len();
        apply_edge_transfers(
            compiled,
            &step_plan,
            &mut state,
            &runtime,
            &expression_cache,
            variables.values(),
            &mut gates,
            &mut timeline,
            attempted_step,
            &mut transfer_log,
        )?;
        for (ordinal, transfer) in transfer_log[transfer_start..].iter().enumerate() {
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
        );
        state.step = attempted_step;
        refresh_metrics(compiled, &mut state);
        emit_metric_snapshots(run_id, state.step, &state.metrics, emit_event)?;

        capture_step(
            compiled,
            config,
            &state,
            variables.values(),
            &mut report,
            &mut captured_steps,
            false,
        );
        completed = end_conditions_met(compiled, &state);
        let terminal_step_reached = completed || state.step >= config.max_steps;
        if !(defer_terminal_step_end && terminal_step_reached) {
            emit_event(RunEvent::step_end(run_id, state.step, 0, StepEndEvent { completed }))?;
        }
    }

    if config.capture.include_final_state {
        capture_step(
            compiled,
            config,
            &state,
            variables.values(),
            &mut report,
            &mut captured_steps,
            true,
        );
    }

    report.steps_executed = state.step;
    report.completed = completed;
    report.final_node_values = compiled
        .node_order
        .iter()
        .enumerate()
        .map(|(index, node_id)| {
            let value = state.node_values.get(index).copied().unwrap_or(0.0);
            (node_id.clone(), canonicalize_float(value))
        })
        .collect::<BTreeMap<_, _>>();
    report.final_metrics = state.metrics.clone();
    report.transfers = transfer_log;

    Ok(report)
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

fn apply_source_generation(compiled: &CompiledScenario, state: &mut EngineState) {
    for (index, node_id) in compiled.node_order.iter().enumerate() {
        let node = compiled
            .scenario
            .nodes
            .get(node_id)
            .expect("compiled.node_order must reference known nodes");
        if !matches!(node.kind, NodeKind::Source) {
            continue;
        }

        let generation = canonicalize_float(node.initial_value);
        if generation <= 0.0 || !generation.is_finite() {
            continue;
        }

        if let Some(value) = state.node_values.get_mut(index) {
            *value = canonicalize_float(*value + generation);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum TransferControl {
    PullAny,
    PullAll,
    PushAny,
    PushAll,
}

#[derive(Debug, Default)]
struct StepTriggers {
    nodes: BTreeSet<NodeId>,
    edges: BTreeSet<EdgeId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum TriggerTarget {
    Node(NodeId),
    Edge(EdgeId),
}

#[derive(Debug, Default)]
struct EngineStepPlan {
    resource_groups_by_controller: BTreeMap<NodeId, BTreeMap<TransferControl, Vec<EdgeId>>>,
    passive_state_triggers: Vec<(NodeId, Vec<TriggerTarget>)>,
    trigger_outputs_by_source: BTreeMap<NodeId, Vec<TriggerTarget>>,
}

impl EngineStepPlan {
    fn from_compiled(compiled: &CompiledScenario) -> Self {
        let mut plan = Self::default();

        for edge_id in &compiled.edge_order {
            let Some(edge) = compiled.scenario.edges.get(edge_id) else {
                continue;
            };
            if !edge.enabled {
                continue;
            }

            if matches!(edge.connection.kind, ConnectionKind::Resource) {
                let target_action =
                    normalized_action_mode(action_mode_for_node(compiled, &edge.to));
                let (controller, control) = match target_action {
                    TransferControl::PullAny | TransferControl::PullAll => {
                        (edge.to.clone(), target_action)
                    }
                    TransferControl::PushAny | TransferControl::PushAll => (
                        edge.from.clone(),
                        normalized_action_mode(action_mode_for_node(compiled, &edge.from)),
                    ),
                };
                plan.resource_groups_by_controller
                    .entry(controller)
                    .or_default()
                    .entry(control)
                    .or_default()
                    .push(edge_id.clone());
            }

            if !matches!(edge.connection.kind, ConnectionKind::State)
                || !matches!(edge.connection.state.role, StateConnectionRole::Trigger)
            {
                continue;
            }

            plan.trigger_outputs_by_source
                .entry(edge.from.clone())
                .or_default()
                .extend(trigger_targets_for_state_connection(&edge.connection.state, &edge.to));

            if !matches!(
                gate_behavior_for_node(compiled, &edge.from),
                GateBehavior::Trigger | GateBehavior::Mixed
            ) {
                plan.passive_state_triggers.push((
                    edge.from.clone(),
                    trigger_targets_for_state_connection(&edge.connection.state, &edge.to),
                ));
            }
        }

        plan
    }
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

#[allow(clippy::too_many_arguments)]
fn apply_edge_transfers(
    compiled: &CompiledScenario,
    step_plan: &EngineStepPlan,
    state: &mut EngineState,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    gates: &mut GateRuntimeState,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_log: &mut Vec<TransferRecord>,
) -> Result<(), RunError> {
    let mut triggers = collect_step_triggers(compiled, step_plan, state);

    for node_id in &compiled.node_order {
        let gate_behavior = gate_behavior_for_node(compiled, node_id);
        let mut node_acted = false;
        let mut had_resource_groups = false;
        let node_groups = step_plan.resource_groups_by_controller.get(node_id);

        for control in [
            TransferControl::PullAny,
            TransferControl::PullAll,
            TransferControl::PushAny,
            TransferControl::PushAll,
        ] {
            let Some(edge_ids) = node_groups.and_then(|groups| groups.get(&control)) else {
                continue;
            };
            had_resource_groups = true;

            if !controller_can_fire(compiled, state, node_id, edge_ids, &triggers) {
                continue;
            }

            let acted = if should_use_gate_routing(compiled, node_id, control, edge_ids) {
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
                    transfer_log,
                )?
            } else {
                match control {
                    TransferControl::PullAny | TransferControl::PushAny => apply_any_edge_group(
                        compiled,
                        state,
                        edge_ids,
                        runtime,
                        expression_cache,
                        runtime_variables,
                        timeline,
                        step,
                        transfer_log,
                    ),
                    TransferControl::PullAll | TransferControl::PushAll => apply_all_edge_group(
                        compiled,
                        state,
                        edge_ids,
                        runtime,
                        expression_cache,
                        runtime_variables,
                        timeline,
                        step,
                        transfer_log,
                    ),
                }
            };

            node_acted |= acted;
        }

        match gate_behavior {
            GateBehavior::Mixed if node_acted => {
                append_node_trigger_outputs(step_plan, node_id, &mut triggers);
            }
            GateBehavior::Trigger => {
                let trigger_gate_acted = if had_resource_groups {
                    node_acted
                } else {
                    controller_can_fire(compiled, state, node_id, &[], &triggers)
                };
                if trigger_gate_acted {
                    append_node_trigger_outputs(step_plan, node_id, &mut triggers);
                }
            }
            GateBehavior::None | GateBehavior::Sorting | GateBehavior::Mixed => {}
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn apply_any_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    edge_ids: &[EdgeId],
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_log: &mut Vec<TransferRecord>,
) -> bool {
    let mut acted = false;
    for edge_id in edge_ids {
        let Some(edge) = compiled.scenario.edges.get(edge_id) else {
            continue;
        };
        let from_available_override =
            timeline.transfer_available_for_source(compiled, state, &edge.from);
        let Some(plan) = plan_edge_transfer_any(
            compiled,
            state,
            edge,
            runtime,
            expression_cache,
            runtime_variables,
            from_available_override,
        ) else {
            continue;
        };
        apply_transfer_plan(compiled, state, plan, timeline, step, transfer_log);
        acted = true;
    }
    acted
}

#[allow(clippy::too_many_arguments)]
fn apply_all_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    edge_ids: &[EdgeId],
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_log: &mut Vec<TransferRecord>,
) -> bool {
    let mut plans = Vec::new();
    let mut total_requested_by_source = BTreeMap::<usize, f64>::new();
    let mut available_by_source = BTreeMap::<usize, f64>::new();

    for edge_id in edge_ids {
        let Some(edge) = compiled.scenario.edges.get(edge_id) else {
            return false;
        };
        let from_available_override =
            timeline.transfer_available_for_source(compiled, state, &edge.from);
        let Some(plan) = plan_edge_transfer_all(
            compiled,
            state,
            edge,
            runtime,
            expression_cache,
            runtime_variables,
            from_available_override,
        ) else {
            return false;
        };

        let available = canonicalize_float(
            from_available_override
                .unwrap_or_else(|| state.node_values.get(plan.from_index).copied().unwrap_or(0.0))
                .max(0.0),
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
            .unwrap_or_else(|| state.node_values.get(from_index).copied().unwrap_or(0.0).max(0.0));
        if canonicalize_float(available) + f64::EPSILON < requested_total {
            return false;
        }
    }

    for plan in plans {
        apply_transfer_plan(compiled, state, plan, timeline, step, transfer_log);
    }

    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateWeightKind {
    Ratio,
    Percentage,
    Chance,
}

fn should_use_gate_routing(
    compiled: &CompiledScenario,
    node_id: &NodeId,
    control: TransferControl,
    edge_ids: &[EdgeId],
) -> bool {
    if !matches!(control, TransferControl::PushAny | TransferControl::PushAll) {
        return false;
    }
    if !matches!(
        gate_behavior_for_node(compiled, node_id),
        GateBehavior::Sorting | GateBehavior::Mixed
    ) {
        return false;
    }

    edge_ids.iter().all(|edge_id| {
        let Some(edge) = compiled.scenario.edges.get(edge_id) else {
            return false;
        };
        edge.from == *node_id
            && matches!(edge.connection.kind, ConnectionKind::Resource)
            && edge.connection.resource.token_size == 1
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_gate_edge_group(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeId],
    control: TransferControl,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    gates: &mut GateRuntimeState,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_log: &mut Vec<TransferRecord>,
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
                    transfer_log,
                ),
                TransferControl::PullAll | TransferControl::PushAll => apply_all_edge_group(
                    compiled,
                    state,
                    edge_ids,
                    runtime,
                    expression_cache,
                    runtime_variables,
                    timeline,
                    step,
                    transfer_log,
                ),
            });
        }
    };

    let Some(&from_index) = compiled.node_index_by_id.get(node_id) else {
        return Ok(false);
    };
    let available_tokens =
        state.node_values.get(from_index).copied().unwrap_or(0.0).max(0.0).floor() as u64;
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
            if let Some(value) = state.node_values.get_mut(from_index) {
                *value = canonicalize_float(*value - 1.0);
                acted = true;
            }
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
                    .node_order
                    .get(to_index)
                    .cloned()
                    .unwrap_or_else(|| edge_to_node_id(compiled, selected_edge_id)),
                from_index,
                to_index,
                requested: 1.0,
                transfer: 1.0,
            },
            timeline,
            step,
            transfer_log,
        );
        acted = true;
    }

    Ok(acted)
}

fn gate_routing_for_group(
    compiled: &CompiledScenario,
    state: &EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeId],
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
) -> Result<Option<(GateRoutingMode, Vec<GateRoutingLane>)>, RunError> {
    let mut lanes = Vec::<GateRoutingLane>::new();
    let mut seen_ratio = false;
    let mut seen_percentage = false;
    let mut seen_chance = false;

    for edge_id in edge_ids {
        let Some(edge) = compiled.scenario.edges.get(edge_id) else {
            return Ok(None);
        };
        let Some(&to_index) = compiled.node_index_by_id.get(&edge.to) else {
            return Ok(None);
        };
        let Some(&from_index) = compiled.node_index_by_id.get(&edge.from) else {
            return Ok(None);
        };
        if from_index == to_index {
            continue;
        }

        let Some((kind, weight)) = gate_weight_for_edge(
            compiled,
            state,
            edge,
            runtime,
            expression_cache,
            runtime_variables,
        ) else {
            return Ok(None);
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
            edge_id: Some(edge_id.clone()),
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
    edge: &crate::types::EdgeSpec,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
) -> Option<(GateWeightKind, f64)> {
    match &edge.transfer {
        TransferSpec::Fixed { amount } => {
            amount.is_finite().then_some((GateWeightKind::Ratio, canonicalize_float(*amount)))
        }
        TransferSpec::Fraction { numerator, denominator } => {
            if *denominator == 0 || *numerator == 0 {
                return None;
            }
            let weight = *numerator as f64 / *denominator as f64 * 100.0;
            weight.is_finite().then_some((GateWeightKind::Percentage, canonicalize_float(weight)))
        }
        TransferSpec::MetricScaled { metric, factor } => {
            let weight = metric_value(compiled, state, metric) * *factor;
            weight.is_finite().then_some((GateWeightKind::Chance, canonicalize_float(weight)))
        }
        TransferSpec::Expression { .. } => {
            let from_value = node_value(compiled, state, &edge.from);
            let requested = transfer_request(
                compiled,
                state,
                edge,
                from_value,
                runtime,
                expression_cache,
                runtime_variables,
            );
            requested.is_finite().then_some((GateWeightKind::Chance, canonicalize_float(requested)))
        }
        TransferSpec::Remaining => Some((GateWeightKind::Chance, 100.0)),
    }
}

fn plan_edge_transfer_any(
    compiled: &CompiledScenario,
    state: &EngineState,
    edge: &crate::types::EdgeSpec,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    from_value_override: Option<f64>,
) -> Option<EdgeTransferPlan> {
    let from_index = *compiled.node_index_by_id.get(&edge.from)?;
    let to_index = *compiled.node_index_by_id.get(&edge.to)?;
    if from_index == to_index {
        return None;
    }

    let from_value = canonicalize_float(
        from_value_override
            .unwrap_or_else(|| state.node_values.get(from_index).copied().unwrap_or(0.0))
            .max(0.0),
    );
    let requested = transfer_request(
        compiled,
        state,
        edge,
        from_value,
        runtime,
        expression_cache,
        runtime_variables,
    );
    let transfer =
        clamp_transfer_amount(edge.connection.resource.token_size, from_value, requested);
    if transfer <= 0.0 {
        return None;
    }

    Some(EdgeTransferPlan {
        edge_id: edge.id.clone(),
        from_node_id: edge.from.clone(),
        to_node_id: edge.to.clone(),
        from_index,
        to_index,
        requested,
        transfer,
    })
}

fn plan_edge_transfer_all(
    compiled: &CompiledScenario,
    state: &EngineState,
    edge: &crate::types::EdgeSpec,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
    from_value_override: Option<f64>,
) -> Option<EdgeTransferPlan> {
    let from_index = *compiled.node_index_by_id.get(&edge.from)?;
    let to_index = *compiled.node_index_by_id.get(&edge.to)?;
    if from_index == to_index {
        return None;
    }

    let from_value = canonicalize_float(
        from_value_override
            .unwrap_or_else(|| state.node_values.get(from_index).copied().unwrap_or(0.0))
            .max(0.0),
    );
    let requested = transfer_request(
        compiled,
        state,
        edge,
        from_value,
        runtime,
        expression_cache,
        runtime_variables,
    );
    let transfer = quantize_requested_amount(edge.connection.resource.token_size, requested);
    if transfer <= 0.0 {
        return None;
    }

    Some(EdgeTransferPlan {
        edge_id: edge.id.clone(),
        from_node_id: edge.from.clone(),
        to_node_id: edge.to.clone(),
        from_index,
        to_index,
        requested,
        transfer,
    })
}

fn apply_transfer_plan(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    plan: EdgeTransferPlan,
    timeline: &mut TimelineRuntimeState,
    step: u64,
    transfer_log: &mut Vec<TransferRecord>,
) {
    if let Some(value) = state.node_values.get_mut(plan.from_index) {
        *value = canonicalize_float(*value - plan.transfer);
    }
    if let Some(value) = state.node_values.get_mut(plan.to_index) {
        *value = canonicalize_float(*value + plan.transfer);
    }

    if let Some(node_id) = compiled.node_order.get(plan.from_index) {
        timeline.record_release(compiled, node_id, plan.transfer);
    }
    if let Some(node_id) = compiled.node_order.get(plan.to_index) {
        timeline.record_arrival(compiled, node_id, plan.transfer, step);
    }

    transfer_log.push(TransferRecord {
        step,
        edge_id: plan.edge_id,
        from_node_id: plan.from_node_id,
        to_node_id: plan.to_node_id,
        requested_amount: canonicalize_float(plan.requested),
        transferred_amount: canonicalize_float(plan.transfer),
    });
}

fn collect_step_triggers(
    compiled: &CompiledScenario,
    step_plan: &EngineStepPlan,
    state: &EngineState,
) -> StepTriggers {
    let mut triggers = StepTriggers::default();

    for (source_node_id, targets) in &step_plan.passive_state_triggers {
        let source_state = node_value(compiled, state, source_node_id);
        if !source_state.is_finite() || source_state <= 0.0 {
            continue;
        }
        append_trigger_targets(targets, &mut triggers);
    }

    triggers
}

fn append_node_trigger_outputs(
    step_plan: &EngineStepPlan,
    node_id: &NodeId,
    triggers: &mut StepTriggers,
) {
    if let Some(targets) = step_plan.trigger_outputs_by_source.get(node_id) {
        append_trigger_targets(targets, triggers);
    }
}

fn trigger_targets_for_state_connection(
    state_config: &crate::types::StateConnectionConfig,
    edge_to: &NodeId,
) -> Vec<TriggerTarget> {
    match state_config.target {
        StateConnectionTarget::Node => vec![TriggerTarget::Node(edge_to.clone())],
        StateConnectionTarget::ResourceConnection | StateConnectionTarget::StateConnection => {
            if let Some(target_edge_id) = state_config.target_connection.clone() {
                vec![TriggerTarget::Edge(target_edge_id)]
            } else {
                Vec::new()
            }
        }
        StateConnectionTarget::Formula => Vec::new(),
    }
}

fn append_trigger_targets(targets: &[TriggerTarget], triggers: &mut StepTriggers) {
    for target in targets {
        match target {
            TriggerTarget::Node(node_id) => {
                triggers.nodes.insert(node_id.clone());
            }
            TriggerTarget::Edge(edge_id) => {
                triggers.edges.insert(edge_id.clone());
            }
        }
    }
}

fn controller_can_fire(
    compiled: &CompiledScenario,
    state: &EngineState,
    node_id: &NodeId,
    edge_ids: &[EdgeId],
    triggers: &StepTriggers,
) -> bool {
    match trigger_mode_for_node(compiled, node_id) {
        TriggerMode::Automatic => true,
        TriggerMode::Interactive => false,
        TriggerMode::Enabling => state.step == 0,
        TriggerMode::Passive => {
            triggers.nodes.contains(node_id)
                || edge_ids.iter().any(|edge_id| triggers.edges.contains(edge_id))
        }
        TriggerMode::Custom(_) => true,
    }
}

fn trigger_mode_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> TriggerMode {
    let mode = node_mode_for_node(compiled, node_id);
    mode.map(|m| m.trigger_mode.clone()).unwrap_or(TriggerMode::Automatic)
}

fn gate_behavior_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> GateBehavior {
    match node_kind_for_node(compiled, node_id) {
        Some(NodeKind::SortingGate) => GateBehavior::Sorting,
        Some(NodeKind::TriggerGate) => GateBehavior::Trigger,
        Some(NodeKind::MixedGate) => GateBehavior::Mixed,
        _ => GateBehavior::None,
    }
}

fn action_mode_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> ActionMode {
    let mode = node_mode_for_node(compiled, node_id);
    mode.map(|m| m.action_mode.clone()).unwrap_or(ActionMode::PushAny)
}

fn node_kind_for_node<'a>(
    compiled: &'a CompiledScenario,
    node_id: &NodeId,
) -> Option<&'a NodeKind> {
    let node = compiled.scenario.nodes.get(node_id)?;
    Some(&node.kind)
}

fn timeline_node_kind(compiled: &CompiledScenario, node_id: &NodeId) -> Option<TimelineNodeKind> {
    match node_kind_for_node(compiled, node_id) {
        Some(NodeKind::Delay) => Some(TimelineNodeKind::Delay),
        Some(NodeKind::Queue) => Some(TimelineNodeKind::Queue),
        _ => None,
    }
}

fn delay_steps_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> u64 {
    let Some(node) = compiled.scenario.nodes.get(node_id) else {
        return 1;
    };
    match &node.config {
        NodeConfig::Delay(config) => config.delay_steps.max(1),
        _ => 1,
    }
}

fn queue_release_per_step_for_node(compiled: &CompiledScenario, node_id: &NodeId) -> u64 {
    let Some(node) = compiled.scenario.nodes.get(node_id) else {
        return 1;
    };
    match &node.config {
        NodeConfig::Queue(config) => config.release_per_step.max(1),
        _ => 1,
    }
}

fn node_mode_for_node<'a>(
    compiled: &'a CompiledScenario,
    node_id: &NodeId,
) -> Option<&'a NodeModeConfig> {
    let node = compiled.scenario.nodes.get(node_id)?;
    match &node.config {
        NodeConfig::Pool(config) => Some(&config.mode),
        NodeConfig::Drain(config) => Some(&config.mode),
        NodeConfig::SortingGate(config) => Some(&config.mode),
        NodeConfig::TriggerGate(config) => Some(&config.mode),
        NodeConfig::MixedGate(config) => Some(&config.mode),
        NodeConfig::Converter(config) => Some(&config.mode),
        NodeConfig::Trader(config) => Some(&config.mode),
        NodeConfig::Delay(config) => Some(&config.mode),
        NodeConfig::Queue(config) => Some(&config.mode),
        NodeConfig::None | NodeConfig::Register(_) => None,
    }
}

fn normalized_action_mode(mode: ActionMode) -> TransferControl {
    match mode {
        ActionMode::PullAny => TransferControl::PullAny,
        ActionMode::PullAll => TransferControl::PullAll,
        ActionMode::PushAll => TransferControl::PushAll,
        ActionMode::PushAny | ActionMode::Custom(_) => TransferControl::PushAny,
    }
}

fn apply_state_connections(
    compiled: &CompiledScenario,
    state: &mut EngineState,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
) {
    let mut next_step_node_deltas = vec![0.0; state.node_values.len()];

    for edge_id in &compiled.edge_order {
        let edge = compiled
            .scenario
            .edges
            .get(edge_id)
            .expect("compiled.edge_order must reference known edges");
        if !edge.enabled || !matches!(edge.connection.kind, ConnectionKind::State) {
            continue;
        }

        let state_config = &edge.connection.state;
        if !matches!(state_config.role, StateConnectionRole::Modifier)
            || !matches!(state_config.target, StateConnectionTarget::Node)
        {
            continue;
        }

        let Some(&from_index) = compiled.node_index_by_id.get(&edge.from) else {
            continue;
        };
        let Some(&to_index) = compiled.node_index_by_id.get(&edge.to) else {
            continue;
        };

        let source_state = state.node_values.get(from_index).copied().unwrap_or(0.0);
        if !source_state.is_finite() || source_state == 0.0 {
            continue;
        }
        let target_state = state.node_values.get(to_index).copied().unwrap_or(0.0);

        let Some(delta) = evaluate_state_formula_delta(
            compiled,
            state,
            edge_id,
            source_state,
            target_state,
            runtime,
            expression_cache,
            runtime_variables,
        ) else {
            continue;
        };

        let effect = canonicalize_float(delta * source_state);
        if effect == 0.0 {
            continue;
        }

        if let Some(slot) = next_step_node_deltas.get_mut(to_index) {
            *slot = canonicalize_float(*slot + effect);
        }
    }

    for (index, delta) in next_step_node_deltas.into_iter().enumerate() {
        if delta == 0.0 {
            continue;
        }
        if let Some(value) = state.node_values.get_mut(index) {
            *value = canonicalize_float(*value + delta);
        }
    }
}

fn transfer_request(
    compiled: &CompiledScenario,
    state: &EngineState,
    edge: &crate::types::EdgeSpec,
    from_value: f64,
    runtime: &ExprRuntime,
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
) -> f64 {
    let requested = match &edge.transfer {
        TransferSpec::Fixed { amount } => *amount,
        TransferSpec::Fraction { numerator, denominator } => {
            if *denominator == 0 {
                0.0
            } else {
                from_value * (*numerator as f64 / *denominator as f64)
            }
        }
        TransferSpec::Remaining => from_value,
        TransferSpec::MetricScaled { metric, factor } => {
            metric_value(compiled, state, metric) * *factor
        }
        TransferSpec::Expression { .. } => {
            let Some(compiled_expression) = expression_cache.transfer_expression(&edge.id) else {
                return 0.0;
            };

            let step = state.step as f64;
            let total = total_node_value(state);
            let to_value = compiled
                .node_index_by_id
                .get(&edge.to)
                .and_then(|to_index| state.node_values.get(*to_index))
                .copied()
                .map(canonicalize_float);

            evaluate_compiled_formula(
                runtime,
                compiled_expression,
                runtime_variables,
                &FormulaBindings {
                    step: canonicalize_float(step),
                    total: canonicalize_float(total),
                    nodes: canonicalize_float(compiled.node_order.len() as f64),
                    next_step: canonicalize_float(step + 1.0),
                    is_positive_total: canonicalize_float(total.max(0.0)),
                    from: Some(canonicalize_float(from_value)),
                    to: to_value,
                    source: None,
                    target: None,
                    available: Some(canonicalize_float(from_value.max(0.0))),
                    s: None,
                },
            )
            .unwrap_or(0.0)
        }
    };

    canonicalize_float(requested)
}

fn clamp_transfer_amount(token_size: u64, from_value: f64, requested: f64) -> f64 {
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

fn quantize_requested_amount(token_size: u64, requested: f64) -> f64 {
    if !requested.is_finite() || requested <= 0.0 {
        return 0.0;
    }

    let token_size = token_size.max(1) as f64;
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
    bindings: &FormulaBindings,
) -> Option<f64> {
    let value = runtime
        .evaluate_compiled_with_resolver(expression, |name| {
            resolve_formula_variable(name, bindings, runtime_variables)
        })
        .ok()?;
    Some(canonicalize_float(value))
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
    expression_cache: &EngineExpressionCache,
    runtime_variables: &BTreeMap<String, f64>,
) -> Option<f64> {
    let compiled_expression = expression_cache.state_expression(edge_id)?;
    let step = state.step as f64;
    let total = total_node_value(state);

    evaluate_compiled_formula(
        runtime,
        compiled_expression,
        runtime_variables,
        &FormulaBindings {
            step: canonicalize_float(step),
            total: canonicalize_float(total),
            nodes: canonicalize_float(compiled.node_order.len() as f64),
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
}

fn refresh_metrics(compiled: &CompiledScenario, state: &mut EngineState) {
    if state.metrics.is_empty() {
        return;
    }

    let total_value = total_node_value(state);

    for metric in &compiled.scenario.tracked_metrics {
        let value = match metric_node_index(compiled, metric) {
            Some(index) => state.node_values.get(index).copied().unwrap_or(0.0),
            None => total_value,
        };
        state.metrics.insert(metric.clone(), canonicalize_float(value));
    }
}

fn total_node_value(state: &EngineState) -> f64 {
    state.node_values.iter().copied().fold(0.0, |acc, value| canonicalize_float(acc + value))
}

fn metric_node_index(compiled: &CompiledScenario, metric: &MetricKey) -> Option<usize> {
    compiled.metric_index_by_name.get(metric.as_str()).copied()
}

fn node_value(compiled: &CompiledScenario, state: &EngineState, node_id: &NodeId) -> f64 {
    let Some(index) = compiled.node_index_by_id.get(node_id).copied() else {
        return 0.0;
    };
    state.node_values.get(index).copied().unwrap_or(0.0)
}

fn metric_value(compiled: &CompiledScenario, state: &EngineState, metric: &MetricKey) -> f64 {
    if let Some(value) = state.metrics.get(metric).copied() {
        return canonicalize_float(value);
    }

    if let Some(index) = metric_node_index(compiled, metric) {
        return canonicalize_float(state.node_values.get(index).copied().unwrap_or(0.0));
    }

    0.0
}

fn end_conditions_met(compiled: &CompiledScenario, state: &EngineState) -> bool {
    compiled
        .scenario
        .end_conditions
        .iter()
        .any(|condition| end_condition_met(compiled, state, condition))
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

fn capture_step(
    compiled: &CompiledScenario,
    config: &RunConfig,
    state: &EngineState,
    runtime_variables: &BTreeMap<String, f64>,
    report: &mut RunReport,
    captured_steps: &mut BTreeSet<u64>,
    force: bool,
) {
    if !should_capture_step(config, state.step, force) {
        return;
    }

    if !captured_steps.insert(state.step) {
        return;
    }

    let capture_all_nodes = config.capture.capture_nodes.is_empty();
    let mut snapshot = NodeSnapshot::new(state.step);
    for (index, node_id) in compiled.node_order.iter().enumerate() {
        if capture_all_nodes || config.capture.capture_nodes.contains(node_id) {
            let value = state.node_values.get(index).copied().unwrap_or(0.0);
            snapshot.values.insert(node_id.clone(), canonicalize_float(value));
        }
    }
    if !snapshot.values.is_empty() {
        report.node_snapshots.push(snapshot);
    }

    if !runtime_variables.is_empty() {
        let mut snapshot = VariableSnapshot::new(state.step);
        for (name, value) in runtime_variables {
            snapshot.values.insert(name.clone(), canonicalize_float(*value));
        }
        if !snapshot.values.is_empty() {
            report.variable_snapshots.push(snapshot);
        }
    }

    if config.capture.capture_metrics.is_empty() {
        for (metric, value) in &state.metrics {
            let table = report
                .series
                .entry(metric.clone())
                .or_insert_with(|| SeriesTable::new(metric.clone()));
            table.points.push(SeriesPoint::new(state.step, canonicalize_float(*value)));
        }
    } else {
        for metric in &config.capture.capture_metrics {
            let value = metric_value(compiled, state, metric);
            let table = report
                .series
                .entry(metric.clone())
                .or_insert_with(|| SeriesTable::new(metric.clone()));
            table.points.push(SeriesPoint::new(state.step, canonicalize_float(value)));
        }
    }
}

fn should_capture_step(config: &RunConfig, step: u64, force: bool) -> bool {
    if force {
        return true;
    }

    if step == 0 {
        return config.capture.include_step_zero;
    }

    let interval = config.capture.every_n_steps.max(1);
    step % interval == 0
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
    use std::collections::BTreeMap;

    use crate::rng::rng_from_seed;
    use crate::stochastic::{sample_closed_interval, sample_from_list, sample_from_matrix};
    use crate::types::{
        ActionMode, CaptureConfig, ConnectionKind, DelayNodeConfig, EdgeConnectionConfig, EdgeId,
        EdgeSpec, EndConditionSpec, MetricKey, NodeConfig, NodeId, NodeKind, NodeModeConfig,
        NodeSpec, PoolNodeConfig, QueueNodeConfig, RunConfig, ScenarioId, ScenarioSpec,
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
        let config = RunConfig { seed: 1, max_steps: 5, capture: CaptureConfig::disabled() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&pool), Some(&0.0));
        assert_eq!(report.final_node_values.get(&sink_a), Some(&5.0));
        assert_eq!(report.final_node_values.get(&sink_b), Some(&5.0));
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
        let config = RunConfig { seed: 2, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 8, max_steps: 10, capture: CaptureConfig::disabled() };
        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report_a.final_node_values.get(&sink), Some(&5.0));
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
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 7, max_steps: 5, capture: CaptureConfig::disabled() };

        let report_a = run_single(&compiled, &config).expect("run should succeed");
        let report_b = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report_a, report_b);
        assert_eq!(report_a.final_node_values.get(&source), Some(&2.0));
        assert_eq!(report_a.final_node_values.get(&sink), Some(&4.0));
        assert_eq!(report_a.steps_executed, 2);
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
        let config = RunConfig { seed: 16, max_steps: 10, capture: CaptureConfig::disabled() };
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
                        formula: "+1".to_string(),
                        target: StateConnectionTarget::Node,
                        target_connection: None,
                        resource_filter: None,
                    },
                }),
            );
        scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 1 }];

        let compiled = compile_scenario(&scenario).expect("scenario should compile");
        let config = RunConfig { seed: 9, max_steps: 5, capture: CaptureConfig::disabled() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&actor), Some(&2.0));
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
        let config = RunConfig { seed: 10, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 11, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 12, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 13, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 14, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 15, max_steps: 5, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 31, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 32, max_steps: 10, capture: CaptureConfig::disabled() };
        let report = run_single(&compiled, &config).expect("run should succeed");

        assert_eq!(report.final_node_values.get(&source), Some(&0.0));
        assert_eq!(report.final_node_values.get(&queue), Some(&1.0));
        assert_eq!(report.final_node_values.get(&sink), Some(&2.0));
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
        let config = RunConfig { seed: 33, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 3, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 4, max_steps: 10, capture: CaptureConfig::disabled() };
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
        let config = RunConfig { seed: 5, max_steps: 3, capture: CaptureConfig::disabled() };
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
