//! Scenario compile pipeline and run/batch config validation gates.
//!
//! The crate-private compiler rejects broken graphs (missing endpoints, resource cycles,
//! invalid node configs, bad variable sources) and returns a
//! [`CompiledScenario`] with stable BTreeMap-derived iteration indexes. Run and
//! batch configs are checked separately so execution never starts with zero step
//! budgets or empty capture strides.

use std::collections::BTreeMap;

use crate::error::SetupError;
use crate::expr::ExprRuntime;
use crate::plan::{
    CompiledEdge, CompiledExpressions, CompiledNode, CompiledScenario, CompiledTransfer, EdgeIndex,
    ExpressionSlots, MetricPlan, NodeIndex, PlanIndexes, PlanProjections, RoutingPlan,
};
use crate::types::{
    BatchConfig, BatchRunTemplate, ConnectionKind, ConnectionSpec, ConverterConfig, DelayConfig,
    DelayNodeConfig, DrainConfig, EdgeId, EdgeSpec, EndConditionSpec, MetricKey, MixedGateConfig,
    NodeBehavior, NodeConfig, NodeId, NodeKind, NodeSpec, PoolConfig, QueueConfig, QueueNodeConfig,
    RegisterConfig, ResourceConnection, ResourceConnectionConfig, RunConfig, Scenario,
    ScenarioEdge, ScenarioNode, ScenarioSpec, SortingGateConfig, StateConnection,
    StateConnectionRole, StateConnectionTarget, StateTarget, TraderConfig, TransferSpec,
    TriggerGateConfig, ValidatedExpressions, VariableSourceSpec,
};

use std::num::NonZeroU64;

/// Validates structural invariants and builds deterministic iteration indexes.
///
/// # Errors
/// Returns [`SetupError`] for missing graph references, resource cycles, invalid
/// parameters, or end-condition/metric reference failures.
#[cfg(test)]
pub(crate) fn compile_scenario(spec: &ScenarioSpec) -> Result<CompiledScenario, SetupError> {
    compile_scenario_owned(spec.clone())
}

fn compile_scenario_owned(spec: ScenarioSpec) -> Result<CompiledScenario, SetupError> {
    assemble_checked_scenario(check_scenario(spec)?)
}

fn validate_edge_references(spec: &ScenarioSpec) -> Result<(), SetupError> {
    for (edge_id, edge) in &spec.edges {
        if !spec.nodes.contains_key(&edge.from) {
            return Err(SetupError::InvalidGraphReference {
                graph: format!("scenario[{}].nodes", spec.id),
                reference: with_available_ids_hint(
                    format!("edges.{edge_id}.from references missing nodes.{}", edge.from),
                    "node IDs",
                    available_node_ids(spec),
                ),
            });
        }
        if !spec.nodes.contains_key(&edge.to) {
            return Err(SetupError::InvalidGraphReference {
                graph: format!("scenario[{}].nodes", spec.id),
                reference: with_available_ids_hint(
                    format!("edges.{edge_id}.to references missing nodes.{}", edge.to),
                    "node IDs",
                    available_node_ids(spec),
                ),
            });
        }
    }

    Ok(())
}

fn assemble_checked_scenario(scenario: Scenario) -> Result<CompiledScenario, SetupError> {
    let source_spec = scenario.source_spec().clone();
    let checked_nodes = scenario.nodes().clone();
    let checked_edges = scenario.edges().clone();
    let mut validated_expressions = scenario.expressions;

    let node_ids = checked_nodes.keys().cloned().collect::<Box<[_]>>();
    let edge_ids = checked_edges.keys().cloned().collect::<Box<[_]>>();

    let node_index_by_id: BTreeMap<NodeId, NodeIndex> = node_ids
        .iter()
        .enumerate()
        .map(|(index, node_id)| (node_id.clone(), NodeIndex::new(index)))
        .collect();
    let edge_index_by_id: BTreeMap<EdgeId, EdgeIndex> = edge_ids
        .iter()
        .enumerate()
        .map(|(index, edge_id)| (edge_id.clone(), EdgeIndex::new(index)))
        .collect();
    let nodes =
        node_ids.iter().map(|id| CompiledNode::new(id.clone(), &checked_nodes[id])).collect();
    let mut expression_slots = Vec::with_capacity(edge_ids.len());
    let mut edges = Vec::with_capacity(edge_ids.len());
    for id in edge_ids.iter() {
        let edge = &checked_edges[id];
        let (transfer, transfer_expression) = match edge.connection() {
            ConnectionSpec::Resource(_) => {
                let transfer = match edge.transfer() {
                    TransferSpec::Fixed { amount } => CompiledTransfer::Fixed { amount: *amount },
                    TransferSpec::Fraction { numerator, denominator } => {
                        let denominator = NonZeroU64::new(*denominator).ok_or_else(|| {
                            SetupError::InvalidParameter {
                                name: format!("edges.{id}.transfer.fraction.denominator"),
                                reason: "must be greater than 0".into(),
                            }
                        })?;
                        CompiledTransfer::Fraction { numerator: *numerator, denominator }
                    }
                    TransferSpec::Remaining => CompiledTransfer::Remaining,
                    TransferSpec::MetricScaled { metric, factor } => {
                        CompiledTransfer::MetricScaled { metric: metric.clone(), factor: *factor }
                    }
                    TransferSpec::Expression { .. } => CompiledTransfer::Expression,
                };
                let expression = if matches!(transfer, CompiledTransfer::Expression) {
                    Some(
                        validated_expressions
                            .transfer
                            .remove(id)
                            .ok_or_else(|| missing_expression(id, "transfer.expression.formula"))?,
                    )
                } else {
                    None
                };
                (Some(transfer), expression)
            }
            ConnectionSpec::State(_) => (None, None),
        };
        let state_expression = match edge.connection() {
            ConnectionSpec::State(state)
                if matches!(state.role(), StateConnectionRole::Modifier) =>
            {
                Some(
                    validated_expressions
                        .state
                        .remove(id)
                        .ok_or_else(|| missing_expression(id, "connection.state.formula"))?,
                )
            }
            ConnectionSpec::Resource(_) | ConnectionSpec::State(_) => None,
        };
        expression_slots
            .push(ExpressionSlots { transfer: transfer_expression, state: state_expression });
        edges.push(CompiledEdge::new(
            id.clone(),
            edge,
            transfer,
            node_index_by_id[edge.from()].value(),
            node_index_by_id[edge.to()].value(),
        ));
    }
    if !validated_expressions.transfer.is_empty() || !validated_expressions.state.is_empty() {
        return Err(SetupError::InvalidParameter {
            name: "scenario.expressions".into(),
            reason: "validated expression bundle contains entries not used by checked edges".into(),
        });
    }

    let routing = RoutingPlan::from_checked(
        &checked_nodes,
        &checked_edges,
        &node_index_by_id,
        &edge_index_by_id,
    );
    let metrics = MetricPlan::from_spec(&source_spec, &node_index_by_id);
    Ok(CompiledScenario::from_validated(
        source_spec,
        PlanProjections::new(node_ids, edge_ids, nodes, edges.into_boxed_slice()),
        PlanIndexes::new(node_index_by_id.clone(), edge_index_by_id.clone()),
        CompiledExpressions::from_slots(expression_slots),
        routing,
        metrics,
    ))
}

fn missing_expression(edge_id: &EdgeId, field: &str) -> SetupError {
    SetupError::InvalidParameter {
        name: format!("edges.{edge_id}.{field}"),
        reason: "validated expression is missing from the checked scenario".into(),
    }
}

impl TryFrom<ScenarioSpec> for CompiledScenario {
    type Error = SetupError;

    fn try_from(spec: ScenarioSpec) -> Result<Self, Self::Error> {
        compile_scenario_owned(spec)
    }
}

impl TryFrom<Scenario> for CompiledScenario {
    type Error = SetupError;

    fn try_from(scenario: Scenario) -> Result<Self, Self::Error> {
        assemble_checked_scenario(scenario)
    }
}

/// Converts the serde document into the immutable checked domain.  This is kept
/// beside the existing graph validation so both entry points use identical graph
/// passes and error vocabulary.
pub(crate) fn check_scenario(spec: ScenarioSpec) -> Result<Scenario, SetupError> {
    for (key, node) in &spec.nodes {
        if key != &node.id {
            return Err(SetupError::InvalidParameter {
                name: format!("nodes.{key}.id"),
                reason: format!("map key `{key}` does not match embedded id `{}`", node.id),
            });
        }
    }
    for (key, edge) in &spec.edges {
        if key != &edge.id {
            return Err(SetupError::InvalidParameter {
                name: format!("edges.{key}.id"),
                reason: format!("map key `{key}` does not match embedded id `{}`", edge.id),
            });
        }
    }
    let mut nodes = BTreeMap::new();
    for (id, node) in &spec.nodes {
        nodes.insert(id.clone(), check_node(id, node)?);
    }
    let mut edges = BTreeMap::new();
    for (id, edge) in &spec.edges {
        edges.insert(id.clone(), check_edge(id, edge)?);
    }

    // Preserve the legacy graph passes after local tag/payload reconciliation.
    validate_edge_references(&spec)?;
    for (index, condition) in spec.end_conditions.iter().enumerate() {
        let path = format!("end_conditions[{index}]");
        validate_end_condition_shape(condition, &path)?;
        validate_end_condition_references(&spec, condition, &path)?;
    }
    validate_transfer_metric_references(&spec)?;
    validate_tracked_metric_references(&spec)?;
    validate_resource_connection_cycles(&spec)?;
    let slots = validate_connection_invariants(&spec)?;
    validate_node_invariants(&spec)?;
    validate_variable_sources(&spec)?;
    let mut expressions = ValidatedExpressions::default();
    for (id, slot) in spec.edges.keys().zip(slots) {
        if let Some(expr) = slot.transfer {
            expressions.transfer.insert(id.clone(), expr);
        }
        if let Some(expr) = slot.state {
            expressions.state.insert(id.clone(), expr);
        }
    }
    Ok(Scenario::from_parts(spec, nodes, edges, expressions))
}

impl TryFrom<ScenarioSpec> for Scenario {
    type Error = SetupError;
    fn try_from(spec: ScenarioSpec) -> Result<Self, Self::Error> {
        check_scenario(spec)
    }
}

fn config_mismatch(id: &NodeId) -> SetupError {
    SetupError::InvalidParameter {
        name: format!("nodes.{id}.config"),
        reason: "config does not match node family".into(),
    }
}
fn nz(value: u64, name: String, reason: &str) -> Result<NonZeroU64, SetupError> {
    NonZeroU64::new(value)
        .ok_or_else(|| SetupError::InvalidParameter { name, reason: reason.into() })
}
fn check_node(id: &NodeId, node: &NodeSpec) -> Result<ScenarioNode, SetupError> {
    let mode = |m: &crate::types::NodeModeConfig| m.clone();
    let behavior = match (&node.kind, &node.config) {
        (NodeKind::Source, NodeConfig::None) => NodeBehavior::Source,
        (NodeKind::Pool, NodeConfig::None) => NodeBehavior::Pool(PoolConfig::default()),
        (NodeKind::Pool, NodeConfig::Pool(c)) => {
            let mut p = PoolConfig::default()
                .with_allow_negative_start(c.allow_negative_start)
                .with_mode(mode(&c.mode));
            if let Some(v) = c.capacity {
                p = p.with_capacity(v)
            }
            NodeBehavior::Pool(p)
        }
        (NodeKind::Drain, NodeConfig::None) => NodeBehavior::Drain(DrainConfig::default()),
        (NodeKind::Drain, NodeConfig::Drain(c)) => {
            NodeBehavior::Drain(DrainConfig::default().with_mode(mode(&c.mode)))
        }
        (NodeKind::SortingGate, NodeConfig::None) => {
            NodeBehavior::SortingGate(SortingGateConfig::default())
        }
        (NodeKind::SortingGate, NodeConfig::SortingGate(c)) => {
            NodeBehavior::SortingGate(SortingGateConfig::default().with_mode(mode(&c.mode)))
        }
        (NodeKind::TriggerGate, NodeConfig::None) => {
            NodeBehavior::TriggerGate(TriggerGateConfig::default())
        }
        (NodeKind::TriggerGate, NodeConfig::TriggerGate(c)) => {
            NodeBehavior::TriggerGate(TriggerGateConfig::default().with_mode(mode(&c.mode)))
        }
        (NodeKind::MixedGate, NodeConfig::None) => {
            NodeBehavior::MixedGate(MixedGateConfig::default())
        }
        (NodeKind::MixedGate, NodeConfig::MixedGate(c)) => {
            NodeBehavior::MixedGate(MixedGateConfig::default().with_mode(mode(&c.mode)))
        }
        (NodeKind::Converter, NodeConfig::None) => {
            NodeBehavior::Converter(ConverterConfig::default())
        }
        (NodeKind::Converter, NodeConfig::Converter(c)) => NodeBehavior::Converter(
            ConverterConfig::default()
                .with_ignore_disabled_inputs(c.ignore_disabled_inputs)
                .with_mode(mode(&c.mode)),
        ),
        (NodeKind::Trader, NodeConfig::None) => NodeBehavior::Trader(TraderConfig::default()),
        (NodeKind::Trader, NodeConfig::Trader(c)) => NodeBehavior::Trader(
            TraderConfig::default()
                .with_ignore_disabled_inputs(c.ignore_disabled_inputs)
                .with_mode(mode(&c.mode)),
        ),
        (NodeKind::Register, NodeConfig::None) => NodeBehavior::Register(RegisterConfig::default()),
        (NodeKind::Register, NodeConfig::Register(c)) => {
            let mut r = RegisterConfig::default().with_interactive(c.interactive);
            if let Some(v) = c.min_value {
                r = r.with_min_value(v)
            }
            if let Some(v) = c.max_value {
                r = r.with_max_value(v)
            }
            NodeBehavior::Register(r)
        }
        (NodeKind::Delay, NodeConfig::None) => NodeBehavior::Delay(DelayConfig::default()),
        (NodeKind::Delay, NodeConfig::Delay(c)) => NodeBehavior::Delay(
            DelayConfig::default()
                .with_delay_steps(nz(
                    c.delay_steps,
                    format!("nodes.{id}.config.delay_steps"),
                    "must be greater than 0",
                )?)
                .with_mode(mode(&c.mode)),
        ),
        (NodeKind::Queue, NodeConfig::None) => NodeBehavior::Queue(QueueConfig::default()),
        (NodeKind::Queue, NodeConfig::Queue(c)) => {
            let mut q = QueueConfig::default()
                .with_release_per_step(nz(
                    c.release_per_step,
                    format!("nodes.{id}.config.release_per_step"),
                    "must be greater than 0",
                )?)
                .with_mode(mode(&c.mode));
            if let Some(v) = c.capacity {
                q = q.with_capacity(nz(
                    v,
                    format!("nodes.{id}.config.capacity"),
                    "must be greater than 0 when specified",
                )?)
            }
            NodeBehavior::Queue(q)
        }
        (NodeKind::Process, NodeConfig::None) => NodeBehavior::Process,
        (NodeKind::Sink, NodeConfig::None) => NodeBehavior::Sink,
        (NodeKind::Gate, NodeConfig::None) => NodeBehavior::Gate,
        (NodeKind::Custom(v), NodeConfig::None) => NodeBehavior::Custom(v.clone()),
        _ => return Err(config_mismatch(id)),
    };
    Ok(ScenarioNode::from_parts(
        node.id.clone(),
        behavior,
        node.label.clone(),
        node.initial_value,
        node.tags.clone(),
        node.metadata.clone(),
    ))
}
fn check_edge(id: &EdgeId, edge: &EdgeSpec) -> Result<ScenarioEdge, SetupError> {
    let connection = match edge.connection.kind {
        ConnectionKind::Resource => {
            if edge.connection.state != Default::default() {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{id}.connection.state"),
                    reason: "resource connections cannot declare state connection semantics".into(),
                });
            }
            ConnectionSpec::Resource(ResourceConnection::default().with_token_size(nz(
                edge.connection.resource.token_size,
                format!("edges.{id}.connection.resource.token_size"),
                "must be greater than 0",
            )?))
        }
        ConnectionKind::State => {
            if edge.connection.resource != Default::default() {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{id}.connection.resource"),
                    reason: "state connections cannot customize resource token settings".into(),
                });
            }
            let s = &edge.connection.state;
            let target = match s.target {
                StateConnectionTarget::Node => {
                    if s.target_connection.is_some() {
                        return Err(SetupError::InvalidParameter {
                            name: format!("edges.{id}.connection.state.target_connection"),
                            reason: "node targets cannot also declare a target connection".into(),
                        });
                    }
                    StateTarget::Node
                }
                StateConnectionTarget::ResourceConnection => StateTarget::ResourceConnection(
                    s.target_connection.clone().ok_or_else(|| SetupError::InvalidParameter {
                        name: format!("edges.{id}.connection.state.target_connection"),
                        reason: "must be set for this target kind".into(),
                    })?,
                ),
                StateConnectionTarget::StateConnection => {
                    StateTarget::StateConnection(s.target_connection.clone().ok_or_else(|| {
                        SetupError::InvalidParameter {
                            name: format!("edges.{id}.connection.state.target_connection"),
                            reason: "must be set for this target kind".into(),
                        }
                    })?)
                }
                StateConnectionTarget::Formula => {
                    StateTarget::Formula(s.target_connection.clone().ok_or_else(|| {
                        SetupError::InvalidParameter {
                            name: format!("edges.{id}.connection.state.target_connection"),
                            reason: "must be set for this target kind".into(),
                        }
                    })?)
                }
            };
            let mut c = StateConnection::new(s.role.clone(), s.formula.clone(), target);
            if let Some(f) = &s.resource_filter {
                c = c.with_resource_filter(f.clone())
            }
            ConnectionSpec::State(c)
        }
    };
    Ok(ScenarioEdge::from_parts(
        edge.id.clone(),
        edge.from.clone(),
        edge.to.clone(),
        edge.transfer.clone(),
        connection,
        edge.enabled,
        edge.metadata.clone(),
    ))
}

/// Rejects zero `max_steps` and malformed typed capture selections on a [`RunConfig`].
pub(crate) fn validate_run_config(config: &RunConfig) -> Result<(), SetupError> {
    validate_run_config_with_prefix(config, "run")
}

/// Rejects zero batch runs and invalid nested run-template limits.
pub(crate) fn validate_batch_config(config: &BatchConfig) -> Result<(), SetupError> {
    if config.runs == 0 {
        return Err(SetupError::InvalidParameter {
            name: "batch.runs".to_string(),
            reason: "must be greater than 0".to_string(),
        });
    }

    validate_batch_run_template_with_prefix(&config.run_template, "batch.run_template")
}

fn validate_run_config_with_prefix(config: &RunConfig, prefix: &str) -> Result<(), SetupError> {
    if config.max_steps == 0 {
        return Err(SetupError::InvalidParameter {
            name: format!("{prefix}.max_steps"),
            reason: "must be greater than 0".to_string(),
        });
    }

    config.capture.validate().map_err(|reason| SetupError::InvalidParameter {
        name: format!("{prefix}.capture"),
        reason: reason.to_string(),
    })?;

    Ok(())
}

fn validate_batch_run_template_with_prefix(
    template: &BatchRunTemplate,
    prefix: &str,
) -> Result<(), SetupError> {
    if template.max_steps == 0 {
        return Err(SetupError::InvalidParameter {
            name: format!("{prefix}.max_steps"),
            reason: "must be greater than 0".to_string(),
        });
    }

    template.aggregation.validate().map_err(|reason| SetupError::InvalidParameter {
        name: format!("{prefix}.aggregation"),
        reason: reason.to_string(),
    })?;

    Ok(())
}

fn validate_end_condition_shape(
    condition: &EndConditionSpec,
    path: &str,
) -> Result<(), SetupError> {
    match condition {
        EndConditionSpec::Any(nested_conditions) => {
            let field_name = format!("{path}.any");
            if nested_conditions.is_empty() {
                return Err(SetupError::InvalidParameter {
                    name: field_name,
                    reason: "must contain at least one condition".to_string(),
                });
            }

            for (index, nested) in nested_conditions.iter().enumerate() {
                validate_end_condition_shape(nested, &format!("{path}.any[{index}]"))?;
            }
        }
        EndConditionSpec::All(nested_conditions) => {
            let field_name = format!("{path}.all");
            if nested_conditions.is_empty() {
                return Err(SetupError::InvalidParameter {
                    name: field_name,
                    reason: "must contain at least one condition".to_string(),
                });
            }

            for (index, nested) in nested_conditions.iter().enumerate() {
                validate_end_condition_shape(nested, &format!("{path}.all[{index}]"))?;
            }
        }
        EndConditionSpec::MaxSteps { .. }
        | EndConditionSpec::MetricAtLeast { .. }
        | EndConditionSpec::MetricAtMost { .. }
        | EndConditionSpec::NodeAtLeast { .. }
        | EndConditionSpec::NodeAtMost { .. } => {}
    }

    Ok(())
}

fn validate_end_condition_references(
    spec: &ScenarioSpec,
    condition: &EndConditionSpec,
    path: &str,
) -> Result<(), SetupError> {
    match condition {
        EndConditionSpec::Any(nested_conditions) => {
            for (index, nested) in nested_conditions.iter().enumerate() {
                validate_end_condition_references(spec, nested, &format!("{path}.any[{index}]"))?;
            }
        }
        EndConditionSpec::All(nested_conditions) => {
            for (index, nested) in nested_conditions.iter().enumerate() {
                validate_end_condition_references(spec, nested, &format!("{path}.all[{index}]"))?;
            }
        }
        EndConditionSpec::NodeAtLeast { node_id, .. }
        | EndConditionSpec::NodeAtMost { node_id, .. } => {
            if !spec.nodes.contains_key(node_id) {
                return Err(SetupError::InvalidGraphReference {
                    graph: format!("scenario[{}].nodes", spec.id),
                    reference: with_available_ids_hint(
                        format!("{path}.node_id references missing nodes.{node_id}"),
                        "node IDs",
                        available_node_ids(spec),
                    ),
                });
            }
        }
        EndConditionSpec::MetricAtLeast { metric, .. }
        | EndConditionSpec::MetricAtMost { metric, .. } => {
            if !metric_resolves_to_node(spec, metric) {
                return Err(SetupError::InvalidGraphReference {
                    graph: format!("scenario[{}].metrics", spec.id),
                    reference: with_available_ids_hint(
                        format!("{path}.metric references unresolved metric `{metric}`"),
                        "metric keys",
                        available_metric_keys(spec),
                    ),
                });
            }
        }
        EndConditionSpec::MaxSteps { .. } => {}
    }

    Ok(())
}

fn validate_transfer_metric_references(spec: &ScenarioSpec) -> Result<(), SetupError> {
    for (edge_id, edge) in &spec.edges {
        if let TransferSpec::MetricScaled { metric, .. } = &edge.transfer {
            if !metric_resolves_to_node(spec, metric) {
                return Err(SetupError::InvalidGraphReference {
                    graph: format!("scenario[{}].metrics", spec.id),
                    reference: with_available_ids_hint(
                        format!(
                            "edges.{edge_id}.transfer.metric references unresolved metric `{metric}`"
                        ),
                        "metric keys",
                        available_metric_keys(spec),
                    ),
                });
            }
        }
    }

    Ok(())
}

fn validate_tracked_metric_references(spec: &ScenarioSpec) -> Result<(), SetupError> {
    for metric in &spec.tracked_metrics {
        if !metric_resolves_to_node(spec, metric) {
            return Err(SetupError::InvalidGraphReference {
                graph: format!("scenario[{}].metrics", spec.id),
                reference: with_available_ids_hint(
                    format!("tracked_metrics[{metric}] references unresolved metric `{metric}`"),
                    "metric keys",
                    available_metric_keys(spec),
                ),
            });
        }
    }

    Ok(())
}

fn metric_resolves_to_node(spec: &ScenarioSpec, metric: &MetricKey) -> bool {
    spec.nodes.keys().any(|node_id| node_id.as_str() == metric.as_str())
}

fn available_node_ids(spec: &ScenarioSpec) -> Vec<String> {
    spec.nodes.keys().map(ToString::to_string).collect::<Vec<_>>()
}

fn available_edge_ids(spec: &ScenarioSpec) -> Vec<String> {
    spec.edges.keys().map(ToString::to_string).collect::<Vec<_>>()
}

fn available_metric_keys(spec: &ScenarioSpec) -> Vec<String> {
    spec.nodes.keys().map(ToString::to_string).collect::<Vec<_>>()
}

fn with_available_ids_hint(reference: String, label: &str, available_ids: Vec<String>) -> String {
    let available =
        if available_ids.is_empty() { "<none>".to_string() } else { available_ids.join(", ") };
    format!("{reference}; hint: choose one of the available {label}: [{available}]")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Visited,
}

fn validate_resource_connection_cycles(spec: &ScenarioSpec) -> Result<(), SetupError> {
    let mut adjacency: BTreeMap<NodeId, Vec<NodeId>> = BTreeMap::new();
    for edge in spec.edges.values() {
        if matches!(edge.connection.kind, ConnectionKind::Resource) {
            adjacency.entry(edge.from.clone()).or_default().push(edge.to.clone());
        }
    }

    for targets in adjacency.values_mut() {
        targets.sort();
        targets.dedup();
    }

    let mut visit_state: BTreeMap<NodeId, VisitState> = BTreeMap::new();
    let mut active_path: Vec<NodeId> = Vec::new();

    for node_id in spec.nodes.keys() {
        if visit_state.contains_key(node_id) {
            continue;
        }

        if let Some(cycle_path) =
            detect_cycle_from(node_id, &adjacency, &mut visit_state, &mut active_path)
        {
            return Err(SetupError::CyclicGraph {
                graph: format!("scenario[{}].resource_connections", spec.id),
                cycle_path,
            });
        }
    }

    Ok(())
}

fn detect_cycle_from(
    node_id: &NodeId,
    adjacency: &BTreeMap<NodeId, Vec<NodeId>>,
    visit_state: &mut BTreeMap<NodeId, VisitState>,
    active_path: &mut Vec<NodeId>,
) -> Option<Vec<String>> {
    visit_state.insert(node_id.clone(), VisitState::Visiting);
    active_path.push(node_id.clone());

    if let Some(targets) = adjacency.get(node_id) {
        for target in targets {
            match visit_state.get(target).copied() {
                Some(VisitState::Visited) => {}
                Some(VisitState::Visiting) => {
                    if let Some(start) =
                        active_path.iter().position(|path_node| path_node == target)
                    {
                        let mut cycle_path = active_path[start..]
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>();
                        cycle_path.push(target.to_string());
                        return Some(cycle_path);
                    }
                    return Some(vec![target.to_string(), target.to_string()]);
                }
                None => {
                    if let Some(cycle_path) =
                        detect_cycle_from(target, adjacency, visit_state, active_path)
                    {
                        return Some(cycle_path);
                    }
                }
            }
        }
    }

    active_path.pop();
    visit_state.insert(node_id.clone(), VisitState::Visited);
    None
}

fn validate_connection_invariants(spec: &ScenarioSpec) -> Result<Vec<ExpressionSlots>, SetupError> {
    let mut expressions = Vec::with_capacity(spec.edges.len());
    let runtime = ExprRuntime::new();
    for (edge_id, edge) in &spec.edges {
        let transfer = match edge.connection.kind {
            ConnectionKind::Resource => {
                let transfer = validate_resource_connection_invariants(&runtime, edge_id, edge)?;
                if edge.connection.state != Default::default() {
                    return Err(SetupError::InvalidParameter {
                        name: format!("edges.{edge_id}.connection.state"),
                        reason: "resource connections cannot declare state connection semantics"
                            .to_string(),
                    });
                }
                transfer
            }
            ConnectionKind::State => None,
        };
        let state = match edge.connection.kind {
            ConnectionKind::State => {
                validate_state_connection_invariants(&runtime, spec, edge_id, edge)?
            }
            ConnectionKind::Resource => None,
        };
        expressions.push(ExpressionSlots { transfer, state });
    }

    Ok(expressions)
}

fn validate_resource_connection_invariants(
    runtime: &ExprRuntime,
    edge_id: &EdgeId,
    edge: &EdgeSpec,
) -> Result<Option<crate::expr::CompiledExpr>, SetupError> {
    if edge.connection.resource.token_size == 0 {
        return Err(SetupError::InvalidParameter {
            name: format!("edges.{edge_id}.connection.resource.token_size"),
            reason: "must be greater than 0".to_string(),
        });
    }

    match &edge.transfer {
        TransferSpec::Fixed { amount } => {
            if !amount.is_finite() || *amount <= 0.0 || !is_whole_number(*amount) {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.transfer.fixed.amount"),
                    reason: "resource transfers must use positive integer token quantities"
                        .to_string(),
                });
            }
        }
        TransferSpec::Fraction { denominator, .. } => {
            if *denominator == 0 {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.transfer.fraction.denominator"),
                    reason: "must be greater than 0".to_string(),
                });
            }
        }
        TransferSpec::Expression { .. } => {}
        TransferSpec::MetricScaled { .. } | TransferSpec::Remaining => {}
    }

    let expression = match &edge.transfer {
        TransferSpec::Expression { formula } => Some(validate_formula(
            runtime,
            format!("edges.{edge_id}.transfer.expression.formula"),
            formula,
        )?),
        _ => None,
    };
    Ok(expression)
}

/// Checks state-connection shape. Rejects Modifier → Delay/Queue: modifiers write
/// node values without timeline `record_arrival`/`record_release`, which desyncs
/// scheduled tokens from physical inventory.
fn validate_state_connection_invariants(
    runtime: &ExprRuntime,
    spec: &ScenarioSpec,
    edge_id: &EdgeId,
    edge: &EdgeSpec,
) -> Result<Option<crate::expr::CompiledExpr>, SetupError> {
    if edge.connection.resource != ResourceConnectionConfig::default() {
        return Err(SetupError::InvalidParameter {
            name: format!("edges.{edge_id}.connection.resource"),
            reason: "state connections cannot customize resource token settings".to_string(),
        });
    }

    let state = &edge.connection.state;
    let formula = state.formula.trim();
    if formula.is_empty() {
        return Err(SetupError::InvalidParameter {
            name: format!("edges.{edge_id}.connection.state.formula"),
            reason: "must not be empty".to_string(),
        });
    }

    if matches!(state.role, StateConnectionRole::Modifier) && !formula_has_explicit_sign(formula) {
        return Err(SetupError::InvalidParameter {
            name: format!("edges.{edge_id}.connection.state.formula"),
            reason: "modifier formulas must start with `+` or `-`".to_string(),
        });
    }

    let expression = if matches!(state.role, StateConnectionRole::Modifier) {
        Some(validate_formula(
            runtime,
            format!("edges.{edge_id}.connection.state.formula"),
            formula,
        )?)
    } else {
        None
    };

    if let Some(filter) = state.resource_filter.as_deref() {
        if filter.trim().is_empty() {
            return Err(SetupError::InvalidParameter {
                name: format!("edges.{edge_id}.connection.state.resource_filter"),
                reason: "must not be blank when specified".to_string(),
            });
        }
    } else if matches!(state.role, StateConnectionRole::Filter) {
        return Err(SetupError::InvalidParameter {
            name: format!("edges.{edge_id}.connection.state.resource_filter"),
            reason: "filter state connections require a resource filter".to_string(),
        });
    }

    match state.target {
        StateConnectionTarget::Node => {
            if state.target_connection.is_some() {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.connection.state.target_connection"),
                    reason: "node targets cannot also declare a target connection".to_string(),
                });
            }

            if matches!(state.role, StateConnectionRole::Modifier) {
                if let Some(target) = spec.nodes.get(&edge.to) {
                    if matches!(target.kind, NodeKind::Delay | NodeKind::Queue) {
                        return Err(SetupError::InvalidParameter {
                            name: format!("edges.{edge_id}.connection.state.target"),
                            reason: "modifier state connections cannot target delay or queue nodes"
                                .to_string(),
                        });
                    }
                }
            }
        }
        StateConnectionTarget::ResourceConnection => {
            let target_id =
                required_state_target_connection(edge_id, state.target_connection.as_ref())?;
            let target_edge = required_target_edge(spec, edge_id, target_id)?;
            if !matches!(target_edge.connection.kind, ConnectionKind::Resource) {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.connection.state.target_connection"),
                    reason: "target must reference a resource connection".to_string(),
                });
            }
        }
        StateConnectionTarget::StateConnection => {
            let target_id =
                required_state_target_connection(edge_id, state.target_connection.as_ref())?;
            let target_edge = required_target_edge(spec, edge_id, target_id)?;
            if !matches!(target_edge.connection.kind, ConnectionKind::State) {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.connection.state.target_connection"),
                    reason: "target must reference a state connection".to_string(),
                });
            }
        }
        StateConnectionTarget::Formula => {
            if matches!(state.role, StateConnectionRole::Trigger) {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.connection.state.target"),
                    reason: "trigger connections cannot target formulas".to_string(),
                });
            }

            if !matches!(state.role, StateConnectionRole::Modifier) {
                return Err(SetupError::InvalidParameter {
                    name: format!("edges.{edge_id}.connection.state.target"),
                    reason: "formula targets are only valid for modifier state connections"
                        .to_string(),
                });
            }

            let target_id =
                required_state_target_connection(edge_id, state.target_connection.as_ref())?;
            let _ = required_target_edge(spec, edge_id, target_id)?;
        }
    }

    Ok(expression)
}

fn required_state_target_connection<'a>(
    edge_id: &EdgeId,
    target: Option<&'a EdgeId>,
) -> Result<&'a EdgeId, SetupError> {
    target.ok_or_else(|| SetupError::InvalidParameter {
        name: format!("edges.{edge_id}.connection.state.target_connection"),
        reason: "must be set for this target kind".to_string(),
    })
}

fn required_target_edge<'a>(
    spec: &'a ScenarioSpec,
    edge_id: &EdgeId,
    target_edge_id: &EdgeId,
) -> Result<&'a EdgeSpec, SetupError> {
    spec.edges.get(target_edge_id).ok_or_else(|| SetupError::InvalidGraphReference {
        graph: format!("scenario[{}].edges", spec.id),
        reference: with_available_ids_hint(
            format!(
                "edges.{edge_id}.connection.state.target_connection references missing edges.{target_edge_id}"
            ),
            "edge IDs",
            available_edge_ids(spec),
        ),
    })
}

fn formula_has_explicit_sign(formula: &str) -> bool {
    matches!(formula.chars().next(), Some('+') | Some('-'))
}

fn validate_formula(
    runtime: &ExprRuntime,
    name: String,
    formula: &str,
) -> Result<crate::expr::CompiledExpr, SetupError> {
    runtime.compile(formula).map_err(|error| SetupError::InvalidParameter {
        name,
        reason: format!("invalid expression: {error}"),
    })
}

fn is_whole_number(value: f64) -> bool {
    (value.fract()).abs() <= f64::EPSILON
}

/// Validates runtime variable sources against the invariants the stochastic
/// samplers enforce, so an invalid source is rejected at compile time instead of
/// being silently dropped at runtime (the samplers' errors are swallowed by
/// `.ok()` in the engine, leaving the variable absent and surfacing later as a
/// confusing `ExprError::UnknownVariable`).
fn validate_variable_sources(spec: &ScenarioSpec) -> Result<(), SetupError> {
    for (name, source) in &spec.variables.sources {
        let path = format!("variables.sources.{name}");
        match source {
            VariableSourceSpec::Constant { value } => {
                if !value.is_finite() {
                    return Err(SetupError::InvalidParameter {
                        name: format!("{path}.value"),
                        reason: "must be finite".to_string(),
                    });
                }
            }
            VariableSourceSpec::RandomInterval { min, max } => {
                if min > max {
                    return Err(SetupError::InvalidParameter {
                        name: format!("{path}.min"),
                        reason: "min must be less than or equal to max".to_string(),
                    });
                }
            }
            VariableSourceSpec::RandomList { values } => {
                if values.is_empty() {
                    return Err(SetupError::InvalidParameter {
                        name: format!("{path}.values"),
                        reason: "must contain at least one element".to_string(),
                    });
                }
                for (index, value) in values.iter().enumerate() {
                    if !value.is_finite() {
                        return Err(SetupError::InvalidParameter {
                            name: format!("{path}.values[{index}]"),
                            reason: "must be finite".to_string(),
                        });
                    }
                }
            }
            VariableSourceSpec::RandomMatrix { values } => {
                if values.is_empty() {
                    return Err(SetupError::InvalidParameter {
                        name: format!("{path}.values"),
                        reason: "must contain at least one row".to_string(),
                    });
                }
                for (row_index, row) in values.iter().enumerate() {
                    if row.is_empty() {
                        return Err(SetupError::InvalidParameter {
                            name: format!("{path}.values[{row_index}]"),
                            reason: "rows must contain at least one element".to_string(),
                        });
                    }
                    for (col_index, value) in row.iter().enumerate() {
                        if !value.is_finite() {
                            return Err(SetupError::InvalidParameter {
                                name: format!("{path}.values[{row_index}][{col_index}]"),
                                reason: "must be finite".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_node_invariants(spec: &ScenarioSpec) -> Result<(), SetupError> {
    for (node_id, node) in &spec.nodes {
        if !node.initial_value.is_finite() {
            return Err(SetupError::InvalidParameter {
                name: format!("nodes.{node_id}.initial_value"),
                reason: "must be finite".to_string(),
            });
        }

        match node.kind {
            NodeKind::Pool => validate_pool_constraints(node_id, node)?,
            NodeKind::Converter | NodeKind::Trader => {
                validate_converter_or_trader_connections(spec, node_id, &node.kind)?
            }
            NodeKind::TriggerGate => validate_trigger_gate_inputs(spec, node_id)?,
            NodeKind::Delay => validate_delay_constraints(node_id, node)?,
            NodeKind::Queue => validate_queue_constraints(node_id, node)?,
            NodeKind::Source
            | NodeKind::Drain
            | NodeKind::SortingGate
            | NodeKind::MixedGate
            | NodeKind::Register
            | NodeKind::Process
            | NodeKind::Sink
            | NodeKind::Gate
            | NodeKind::Custom(_) => {}
        }
    }

    Ok(())
}

fn validate_pool_constraints(node_id: &NodeId, node: &NodeSpec) -> Result<(), SetupError> {
    let (allow_negative_start, capacity) = match &node.config {
        NodeConfig::Pool(config) => (config.allow_negative_start, config.capacity),
        _ => (false, None),
    };

    if node.initial_value < 0.0 && !allow_negative_start {
        return Err(SetupError::InvalidParameter {
            name: format!("nodes.{node_id}.initial_value"),
            reason: "must be non-negative unless config.allow_negative_start is true".to_string(),
        });
    }

    if let Some(capacity) = capacity {
        if node.initial_value > capacity as f64 {
            return Err(SetupError::InvalidParameter {
                name: format!("nodes.{node_id}.initial_value"),
                reason: format!("must not exceed config.capacity ({capacity})"),
            });
        }
    }

    Ok(())
}

fn validate_converter_or_trader_connections(
    spec: &ScenarioSpec,
    node_id: &NodeId,
    kind: &NodeKind,
) -> Result<(), SetupError> {
    let has_inbound = spec.edges.values().any(|edge| {
        edge.to == *node_id && matches!(edge.connection.kind, ConnectionKind::Resource)
    });
    let has_outbound = spec.edges.values().any(|edge| {
        edge.from == *node_id && matches!(edge.connection.kind, ConnectionKind::Resource)
    });

    if has_inbound && has_outbound {
        return Ok(());
    }

    let kind_label = match kind {
        NodeKind::Converter => "converter",
        NodeKind::Trader => "trader",
        _ => "node",
    };

    let reason = match (has_inbound, has_outbound) {
        (false, false) => {
            format!("{kind_label} nodes require at least one inbound and one outbound edge")
        }
        (false, true) => format!("{kind_label} nodes require at least one inbound edge"),
        (true, false) => format!("{kind_label} nodes require at least one outbound edge"),
        (true, true) => unreachable!("handled above"),
    };

    Err(SetupError::InvalidParameter { name: format!("nodes.{node_id}.connections"), reason })
}

fn validate_trigger_gate_inputs(spec: &ScenarioSpec, node_id: &NodeId) -> Result<(), SetupError> {
    if let Some((edge_id, _)) = spec.edges.iter().find(|(_, edge)| {
        edge.to == *node_id && matches!(edge.connection.kind, ConnectionKind::Resource)
    }) {
        return Err(SetupError::InvalidParameter {
            name: format!("nodes.{node_id}.inputs"),
            reason: format!(
                "trigger_gate nodes cannot have incoming resource edges (found edges.{edge_id})"
            ),
        });
    }

    if let Some((edge_id, _)) = spec.edges.iter().find(|(_, edge)| {
        edge.from == *node_id && matches!(edge.connection.kind, ConnectionKind::Resource)
    }) {
        return Err(SetupError::InvalidParameter {
            name: format!("nodes.{node_id}.outputs"),
            reason: format!(
                "trigger_gate nodes emit through state connections only (found edges.{edge_id})"
            ),
        });
    }

    Ok(())
}

fn validate_delay_constraints(node_id: &NodeId, node: &NodeSpec) -> Result<(), SetupError> {
    let delay_steps = match &node.config {
        NodeConfig::Delay(config) => config.delay_steps,
        _ => DelayNodeConfig::default().delay_steps,
    };

    if delay_steps == 0 {
        return Err(SetupError::InvalidParameter {
            name: format!("nodes.{node_id}.config.delay_steps"),
            reason: "must be greater than 0".to_string(),
        });
    }

    Ok(())
}

fn validate_queue_constraints(node_id: &NodeId, node: &NodeSpec) -> Result<(), SetupError> {
    let (release_per_step, capacity) = match &node.config {
        NodeConfig::Queue(config) => (config.release_per_step, config.capacity),
        _ => {
            let config = QueueNodeConfig::default();
            (config.release_per_step, config.capacity)
        }
    };

    if release_per_step == 0 {
        return Err(SetupError::InvalidParameter {
            name: format!("nodes.{node_id}.config.release_per_step"),
            reason: "must be greater than 0".to_string(),
        });
    }

    if let Some(capacity) = capacity {
        if capacity == 0 {
            return Err(SetupError::InvalidParameter {
                name: format!("nodes.{node_id}.config.capacity"),
                reason: "must be greater than 0 when specified".to_string(),
            });
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::error::SetupError;
    use crate::expr::ExprRuntime;
    use crate::types::{
        AggregationConfig, BatchConfig, BatchRunTemplate, CaptureConfig, ConnectionKind,
        DelayNodeConfig, EdgeConnectionConfig, EdgeSpec, EndConditionSpec, ExecutionMode,
        MetricKey, NodeConfig, NodeKind, NodeSpec, PoolNodeConfig, QueueNodeConfig, RunConfig,
        Scenario, ScenarioId, ScenarioSpec, StateConnectionConfig, StateConnectionRole,
        StateConnectionTarget, TransferSpec,
    };

    use super::{
        assemble_checked_scenario, compile_scenario, validate_batch_config, validate_run_config,
    };

    #[test]
    fn compile_scenario_builds_deterministic_indexes() {
        let node_a_id = crate::types::NodeId::fixture("node-a");
        let node_m_id = crate::types::NodeId::fixture("node-m");
        let node_z_id = crate::types::NodeId::fixture("node-z");

        let edge_a_id = crate::types::EdgeId::fixture("edge-a");
        let edge_m_id = crate::types::EdgeId::fixture("edge-m");
        let edge_z_id = crate::types::EdgeId::fixture("edge-z");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(node_z_id.clone(), NodeKind::Sink))
            .with_node(NodeSpec::new(node_a_id.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(node_m_id.clone(), NodeKind::Process))
            .with_edge(EdgeSpec::new(
                edge_z_id.clone(),
                node_a_id.clone(),
                node_z_id.clone(),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                edge_a_id.clone(),
                node_a_id.clone(),
                node_m_id.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                edge_m_id.clone(),
                node_m_id.clone(),
                node_z_id.clone(),
                TransferSpec::Remaining,
            ));

        let compiled = compile_scenario(&spec).expect("scenario should compile");

        assert_eq!(compiled.node_ids(), &[node_a_id.clone(), node_m_id.clone(), node_z_id]);
        assert_eq!(compiled.edge_ids(), &[edge_a_id.clone(), edge_m_id.clone(), edge_z_id]);
        assert_eq!(compiled.node_index(&node_a_id), Some(0));
        assert_eq!(compiled.node_index(&node_m_id), Some(1));
        assert_eq!(compiled.edge_index(&edge_a_id), Some(0));
        assert_eq!(compiled.edge_index(&edge_m_id), Some(1));
        assert_eq!(compiled.source_spec(), &spec);
    }

    #[test]
    fn compile_scenario_rejects_invalid_variable_sources() {
        use crate::types::VariableSourceSpec;

        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let base = || {
            ScenarioSpec::new(ScenarioId::fixture("scenario"))
                .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
                .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
                .with_edge(EdgeSpec::new(
                    crate::types::EdgeId::fixture("edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                ))
        };

        let cases = vec![
            (VariableSourceSpec::RandomInterval { min: 6, max: 1 }, "variables.sources.roll.min"),
            (VariableSourceSpec::RandomList { values: vec![] }, "variables.sources.roll.values"),
            (
                VariableSourceSpec::RandomList { values: vec![f64::NAN] },
                "variables.sources.roll.values[0]",
            ),
            (VariableSourceSpec::RandomMatrix { values: vec![] }, "variables.sources.roll.values"),
            (
                VariableSourceSpec::RandomMatrix { values: vec![vec![]] },
                "variables.sources.roll.values[0]",
            ),
            (
                VariableSourceSpec::RandomMatrix { values: vec![vec![f64::INFINITY]] },
                "variables.sources.roll.values[0][0]",
            ),
            (VariableSourceSpec::Constant { value: f64::NAN }, "variables.sources.roll.value"),
        ];

        for (source_spec, expected_name) in cases {
            let mut spec = base();
            spec.variables.sources.insert("roll".to_string(), source_spec);
            let error = compile_scenario(&spec).expect_err("invalid variable source must fail");
            match error {
                SetupError::InvalidParameter { name, .. } => {
                    assert_eq!(name, expected_name);
                }
                other => panic!("expected InvalidParameter, got {other:?}"),
            }
        }

        let mut spec = base();
        spec.variables
            .sources
            .insert("roll".to_string(), VariableSourceSpec::RandomInterval { min: 1, max: 6 });
        compile_scenario(&spec).expect("valid variable source should compile");
    }

    #[test]
    fn compile_scenario_rejects_missing_edge_source_node() {
        let existing = crate::types::NodeId::fixture("node-existing");
        let missing = crate::types::NodeId::fixture("node-missing");
        let edge_id = crate::types::EdgeId::fixture("edge-1");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(existing.clone(), NodeKind::Source))
            .with_edge(EdgeSpec::new(
                edge_id,
                missing,
                existing,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("missing source node must fail");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].nodes");
                assert_eq!(
                    reference,
                    "edges.edge-1.from references missing nodes.node-missing; hint: choose one of the available node IDs: [node-existing]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_missing_edge_target_node() {
        let existing = crate::types::NodeId::fixture("node-existing");
        let missing = crate::types::NodeId::fixture("node-missing");
        let edge_id = crate::types::EdgeId::fixture("edge-1");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(existing.clone(), NodeKind::Source))
            .with_edge(EdgeSpec::new(
                edge_id,
                existing,
                missing,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("missing target node must fail");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].nodes");
                assert_eq!(
                    reference,
                    "edges.edge-1.to references missing nodes.node-missing; hint: choose one of the available node IDs: [node-existing]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_resource_connection_cycles() {
        let node_a = crate::types::NodeId::fixture("node-a");
        let node_b = crate::types::NodeId::fixture("node-b");
        let node_c = crate::types::NodeId::fixture("node-c");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(node_a.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(node_b.clone(), NodeKind::Process))
            .with_node(NodeSpec::new(node_c.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-ab"),
                node_a.clone(),
                node_b.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-bc"),
                node_b.clone(),
                node_c.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-ca"),
                node_c,
                node_a,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("resource cycle must fail");
        match error {
            SetupError::CyclicGraph { graph, cycle_path } => {
                assert_eq!(graph, "scenario[scenario].resource_connections");
                assert_eq!(
                    cycle_path,
                    vec![
                        "node-a".to_string(),
                        "node-b".to_string(),
                        "node-c".to_string(),
                        "node-a".to_string()
                    ]
                );
            }
            other => panic!("expected CyclicGraph, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_accepts_acyclic_resource_connections() {
        let node_a = crate::types::NodeId::fixture("node-a");
        let node_b = crate::types::NodeId::fixture("node-b");
        let node_c = crate::types::NodeId::fixture("node-c");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(node_a.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(node_b.clone(), NodeKind::Process))
            .with_node(NodeSpec::new(node_c.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-ab"),
                node_a,
                node_b.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-bc"),
                node_b,
                node_c,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        compile_scenario(&spec).expect("acyclic resource graph should compile");
    }

    #[test]
    fn compile_scenario_checks_references_before_cycle_detection() {
        let node_a = crate::types::NodeId::fixture("node-a");
        let node_b = crate::types::NodeId::fixture("node-b");

        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(node_a.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(node_b.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-ab"),
                node_a.clone(),
                node_b.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-ba"),
                node_b,
                node_a,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        spec.tracked_metrics.insert(MetricKey::fixture("missing-metric"));

        let error = compile_scenario(&spec).expect_err("reference validation should run first");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].metrics");
                assert_eq!(
                    reference,
                    "tracked_metrics[missing-metric] references unresolved metric `missing-metric`; hint: choose one of the available metric keys: [node-a, node-b]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_empty_any_end_condition() {
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"));
        spec.end_conditions = vec![EndConditionSpec::Any(Vec::new())];

        let error = compile_scenario(&spec).expect_err("empty any condition must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "end_conditions[0].any");
                assert_eq!(reason, "must contain at least one condition");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_empty_all_end_condition_nested_shape() {
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"));
        spec.end_conditions = vec![EndConditionSpec::Any(vec![EndConditionSpec::All(Vec::new())])];

        let error = compile_scenario(&spec).expect_err("empty all condition must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "end_conditions[0].any[0].all");
                assert_eq!(reason, "must contain at least one condition");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_end_condition_missing_node_reference() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let missing = crate::types::NodeId::fixture("missing-node");

        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        spec.end_conditions = vec![EndConditionSpec::All(vec![EndConditionSpec::NodeAtLeast {
            node_id: missing,
            value_scaled: 1,
        }])];

        let error = compile_scenario(&spec).expect_err("missing end-condition node ref must fail");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].nodes");
                assert_eq!(
                    reference,
                    "end_conditions[0].all[0].node_id references missing nodes.missing-node; hint: choose one of the available node IDs: [sink, source]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_end_condition_missing_metric_reference() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        spec.end_conditions = vec![EndConditionSpec::MetricAtLeast {
            metric: MetricKey::fixture("missing-metric"),
            value_scaled: 1,
        }];

        let error =
            compile_scenario(&spec).expect_err("missing end-condition metric ref must fail");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].metrics");
                assert_eq!(
                    reference,
                    "end_conditions[0].metric references unresolved metric `missing-metric`; hint: choose one of the available metric keys: [sink, source]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_metric_scaled_transfer_missing_metric_reference() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::MetricScaled {
                    metric: MetricKey::fixture("missing-metric"),
                    factor: 1.0,
                },
            ));

        let error = compile_scenario(&spec).expect_err("missing MetricScaled ref must fail");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].metrics");
                assert_eq!(
                    reference,
                    "edges.edge-1.transfer.metric references unresolved metric `missing-metric`; hint: choose one of the available metric keys: [sink, source]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_unresolved_tracked_metric_reference() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));
        spec.tracked_metrics.insert(MetricKey::fixture("missing-metric"));

        let error =
            compile_scenario(&spec).expect_err("unresolved tracked metric should fail compile");
        match error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].metrics");
                assert_eq!(
                    reference,
                    "tracked_metrics[missing-metric] references unresolved metric `missing-metric`; hint: choose one of the available metric keys: [sink, source]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_accepts_resolved_metric_references() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let mut spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(3.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink.clone(),
                TransferSpec::MetricScaled { metric: MetricKey::fixture("sink"), factor: 1.0 },
            ));
        spec.tracked_metrics.insert(MetricKey::fixture("sink"));
        spec.end_conditions = vec![EndConditionSpec::MetricAtLeast {
            metric: MetricKey::fixture("sink"),
            value_scaled: 1,
        }];

        compile_scenario(&spec).expect("resolved metric references should compile");
    }

    #[test]
    fn compile_scenario_rejects_fraction_transfer_zero_denominator() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Fraction { numerator: 1, denominator: 0 },
            ));

        let error = compile_scenario(&spec).expect_err("zero denominator must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.edge-1.transfer.fraction.denominator");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_invalid_transfer_expression_formula() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                sink,
                TransferSpec::Expression { formula: "@ + 1".to_string() },
            ));

        let error = compile_scenario(&spec).expect_err("invalid expression must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.edge-1.transfer.expression.formula");
                assert!(reason.contains("unexpected token `@` at byte 0"));
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_negative_pool_start_by_default() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("pool"), NodeKind::Pool)
                .with_initial_value(-1.0),
        );

        let error = compile_scenario(&spec).expect_err("negative pool start must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.pool.initial_value");
                assert_eq!(
                    reason,
                    "must be non-negative unless config.allow_negative_start is true"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_non_finite_node_initial_value() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("source"), NodeKind::Source)
                .with_initial_value(f64::NAN),
        );

        let error = compile_scenario(&spec).expect_err("non-finite initial value must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.source.initial_value");
                assert_eq!(reason, "must be finite");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_allows_negative_pool_start_with_override() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("pool"), NodeKind::Pool)
                .with_initial_value(-1.0)
                .with_config(NodeConfig::Pool(PoolNodeConfig {
                    capacity: None,
                    allow_negative_start: true,
                    mode: Default::default(),
                })),
        );

        compile_scenario(&spec).expect("pool override should allow negative start");
    }

    #[test]
    fn compile_scenario_rejects_pool_initial_value_above_capacity() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("pool"), NodeKind::Pool)
                .with_initial_value(11.0)
                .with_config(NodeConfig::Pool(PoolNodeConfig {
                    capacity: Some(10),
                    allow_negative_start: false,
                    mode: Default::default(),
                })),
        );

        let error = compile_scenario(&spec).expect_err("pool capacity overrun must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.pool.initial_value");
                assert_eq!(reason, "must not exceed config.capacity (10)");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_converter_without_inbound_edge() {
        let converter = crate::types::NodeId::fixture("converter");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(converter.clone(), NodeKind::Converter))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("converter-out"),
                converter,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("converter without inbound must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.converter.connections");
                assert_eq!(reason, "converter nodes require at least one inbound edge");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_trader_without_outbound_edge() {
        let source = crate::types::NodeId::fixture("source");
        let trader = crate::types::NodeId::fixture("trader");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(trader.clone(), NodeKind::Trader))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("trader-in"),
                source,
                trader,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("trader without outbound must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.trader.connections");
                assert_eq!(reason, "trader nodes require at least one outbound edge");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_trigger_gate_with_incoming_resource_edge() {
        let source = crate::types::NodeId::fixture("source");
        let trigger_gate = crate::types::NodeId::fixture("trigger-gate");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(trigger_gate.clone(), NodeKind::TriggerGate))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                trigger_gate,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("trigger gate with input must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.trigger-gate.inputs");
                assert_eq!(
                    reason,
                    "trigger_gate nodes cannot have incoming resource edges (found edges.edge-1)"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_trigger_gate_with_outgoing_resource_edge() {
        let trigger_gate = crate::types::NodeId::fixture("trigger-gate");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(trigger_gate.clone(), NodeKind::TriggerGate))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                trigger_gate,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let error = compile_scenario(&spec).expect_err("trigger gate output must be state-only");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.trigger-gate.outputs");
                assert_eq!(
                    reason,
                    "trigger_gate nodes emit through state connections only (found edges.edge-1)"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_state_trigger_targeting_formula() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let resource_edge_id = crate::types::EdgeId::fixture("resource-edge");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-edge"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: Default::default(),
            state: StateConnectionConfig {
                role: StateConnectionRole::Trigger,
                formula: "+1".to_string(),
                target: StateConnectionTarget::Formula,
                target_connection: Some(resource_edge_id.clone()),
                resource_filter: None,
            },
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id,
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(state_edge);

        let error = compile_scenario(&spec).expect_err("trigger formula targets must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.target");
                assert_eq!(reason, "trigger connections cannot target formulas");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_modifier_state_edge_targeting_timeline_node() {
        use crate::types::NodeModeConfig;

        let source = crate::types::NodeId::fixture("source");

        let modifier_edge = |target: crate::types::NodeId| {
            EdgeSpec::new(
                crate::types::EdgeId::fixture("state-edge"),
                source.clone(),
                target,
                TransferSpec::Remaining,
            )
            .with_connection(EdgeConnectionConfig {
                kind: ConnectionKind::State,
                resource: Default::default(),
                state: StateConnectionConfig {
                    role: StateConnectionRole::Modifier,
                    formula: "+10".to_string(),
                    target: StateConnectionTarget::Node,
                    target_connection: None,
                    resource_filter: None,
                },
            })
        };

        let delay = crate::types::NodeId::fixture("delay");
        let delay_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(delay.clone(), NodeKind::Delay).with_config(
                NodeConfig::Delay(DelayNodeConfig {
                    delay_steps: 5,
                    mode: NodeModeConfig::default(),
                }),
            ))
            .with_edge(modifier_edge(delay));

        let error = compile_scenario(&delay_spec).expect_err("modifier into delay must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.target");
                assert_eq!(reason, "modifier state connections cannot target delay or queue nodes");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }

        let queue = crate::types::NodeId::fixture("queue");
        let queue_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(queue.clone(), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: None,
                    release_per_step: 1,
                    mode: NodeModeConfig::default(),
                }),
            ))
            .with_edge(modifier_edge(queue));

        let error = compile_scenario(&queue_spec).expect_err("modifier into queue must fail");
        assert!(matches!(
            error,
            SetupError::InvalidParameter { reason, .. }
                if reason == "modifier state connections cannot target delay or queue nodes"
        ));
    }

    #[test]
    fn compile_scenario_rejects_formula_modifier_without_signed_formula() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let resource_edge_id = crate::types::EdgeId::fixture("resource-edge");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-edge"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: Default::default(),
            state: StateConnectionConfig {
                role: StateConnectionRole::Modifier,
                formula: "1".to_string(),
                target: StateConnectionTarget::Formula,
                target_connection: Some(resource_edge_id.clone()),
                resource_filter: None,
            },
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id,
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(state_edge);

        let error = compile_scenario(&spec).expect_err("unsigned formula modifier must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.formula");
                assert_eq!(reason, "modifier formulas must start with `+` or `-`");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_invalid_state_connection_formula() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-edge"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: Default::default(),
            state: StateConnectionConfig {
                role: StateConnectionRole::Modifier,
                formula: "+ @".to_string(),
                target: StateConnectionTarget::Node,
                target_connection: None,
                resource_filter: None,
            },
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source, NodeKind::Source))
            .with_node(NodeSpec::new(sink, NodeKind::Sink))
            .with_edge(state_edge);

        let error = compile_scenario(&spec).expect_err("invalid state formula must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.formula");
                assert!(reason.contains("unexpected token `@` at byte 2"));
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_accepts_trigger_state_star_formula() {
        let trigger = crate::types::NodeId::fixture("trigger");
        let actor = crate::types::NodeId::fixture("actor");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-trigger"),
            trigger.clone(),
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
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(trigger, NodeKind::Source))
            .with_node(NodeSpec::new(actor, NodeKind::Sink))
            .with_edge(state_edge);

        compile_scenario(&spec).expect("trigger star formula should compile");
    }

    #[test]
    fn compile_scenario_rejects_filter_state_connection_without_filter_value() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-edge"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: Default::default(),
            state: StateConnectionConfig {
                role: StateConnectionRole::Filter,
                formula: "+1".to_string(),
                target: StateConnectionTarget::Node,
                target_connection: None,
                resource_filter: None,
            },
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source, NodeKind::Source))
            .with_node(NodeSpec::new(sink, NodeKind::Sink))
            .with_edge(state_edge);

        let error = compile_scenario(&spec).expect_err("missing filter value must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.resource_filter");
                assert_eq!(reason, "filter state connections require a resource filter");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_resource_connection_with_state_semantics() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("edge-1"),
                    source,
                    sink,
                    TransferSpec::Fixed { amount: 1.0 },
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::Resource,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        formula: "+2".to_string(),
                        ..Default::default()
                    },
                }),
            );

        let error = compile_scenario(&spec).expect_err("resource edge with state semantics");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.edge-1.connection.state");
                assert_eq!(
                    reason,
                    "resource connections cannot declare state connection semantics"
                );
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_state_connection_custom_resource_or_blank_formula() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let edge_id = crate::types::EdgeId::fixture("state-edge");

        let with_custom_resource = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    edge_id.clone(),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: crate::types::ResourceConnectionConfig { token_size: 2 },
                    state: StateConnectionConfig::default(),
                }),
            );
        let custom_resource_error = compile_scenario(&with_custom_resource)
            .expect_err("state edge custom resource must fail");
        assert!(matches!(
            custom_resource_error,
            SetupError::InvalidParameter { name, .. } if name == "edges.state-edge.connection.resource"
        ));

        let with_blank_formula = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(edge_id, source, sink, TransferSpec::Remaining).with_connection(
                    EdgeConnectionConfig {
                        kind: ConnectionKind::State,
                        resource: Default::default(),
                        state: StateConnectionConfig {
                            formula: "   ".to_string(),
                            ..Default::default()
                        },
                    },
                ),
            );
        let blank_formula_error =
            compile_scenario(&with_blank_formula).expect_err("blank formula must fail");
        assert!(matches!(
            blank_formula_error,
            SetupError::InvalidParameter { name, .. } if name == "edges.state-edge.connection.state.formula"
        ));
    }

    #[test]
    fn compile_scenario_rejects_state_node_target_with_target_connection() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let resource_edge_id = crate::types::EdgeId::fixture("resource-edge");

        let state_edge = EdgeSpec::new(
            crate::types::EdgeId::fixture("state-edge"),
            source.clone(),
            sink.clone(),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: Default::default(),
            state: StateConnectionConfig {
                target: StateConnectionTarget::Node,
                target_connection: Some(resource_edge_id.clone()),
                ..Default::default()
            },
        });

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id,
                source,
                sink,
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(state_edge);

        let error = compile_scenario(&spec).expect_err("node target cannot include target edge");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "edges.state-edge.connection.state.target_connection");
                assert_eq!(reason, "node targets cannot also declare a target connection");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_missing_or_mismatched_state_target_connection() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let resource_edge_id = crate::types::EdgeId::fixture("resource-edge");

        let missing_target_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        target: StateConnectionTarget::ResourceConnection,
                        target_connection: None,
                        ..Default::default()
                    },
                }),
            );
        let missing_target_error = compile_scenario(&missing_target_spec)
            .expect_err("missing target_connection must fail");
        assert!(matches!(
            missing_target_error,
            SetupError::InvalidParameter { name, .. }
                if name == "edges.state-edge.connection.state.target_connection"
        ));

        let missing_edge_target_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        target: StateConnectionTarget::ResourceConnection,
                        target_connection: Some(crate::types::EdgeId::fixture("missing-edge")),
                        ..Default::default()
                    },
                }),
            );
        let missing_edge_target_error =
            compile_scenario(&missing_edge_target_spec).expect_err("missing target edge must fail");
        match missing_edge_target_error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].edges");
                assert_eq!(
                    reference,
                    "edges.state-edge.connection.state.target_connection references missing edges.missing-edge; hint: choose one of the available edge IDs: [resource-edge, state-edge]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }

        let missing_formula_target_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Modifier,
                        target: StateConnectionTarget::Formula,
                        target_connection: Some(crate::types::EdgeId::fixture("missing-edge")),
                        ..Default::default()
                    },
                }),
            );
        let missing_formula_target_error = compile_scenario(&missing_formula_target_spec)
            .expect_err("missing formula target edge must fail");
        match missing_formula_target_error {
            SetupError::InvalidGraphReference { graph, reference } => {
                assert_eq!(graph, "scenario[scenario].edges");
                assert_eq!(
                    reference,
                    "edges.state-edge.connection.state.target_connection references missing edges.missing-edge; hint: choose one of the available edge IDs: [resource-edge, state-edge]"
                );
            }
            other => panic!("expected InvalidGraphReference, got {other:?}"),
        }

        let mismatched_target_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source,
                    sink,
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        target: StateConnectionTarget::StateConnection,
                        target_connection: Some(resource_edge_id),
                        ..Default::default()
                    },
                }),
            );
        let mismatched_target_error =
            compile_scenario(&mismatched_target_spec).expect_err("target kind mismatch must fail");
        assert!(matches!(
            mismatched_target_error,
            SetupError::InvalidParameter { name, reason }
                if name == "edges.state-edge.connection.state.target_connection"
                    && reason == "target must reference a state connection"
        ));
    }

    #[test]
    fn compile_scenario_rejects_state_filter_blank_and_formula_target_for_non_modifier() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let resource_edge_id = crate::types::EdgeId::fixture("resource-edge");

        let blank_filter_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Filter,
                        resource_filter: Some("  ".to_string()),
                        ..Default::default()
                    },
                }),
            );
        let blank_filter_error =
            compile_scenario(&blank_filter_spec).expect_err("blank filter must fail");
        assert!(matches!(
            blank_filter_error,
            SetupError::InvalidParameter { name, .. }
                if name == "edges.state-edge.connection.state.resource_filter"
        ));

        let formula_target_non_modifier_spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_edge_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
            ))
            .with_edge(
                EdgeSpec::new(
                    crate::types::EdgeId::fixture("state-edge"),
                    source,
                    sink,
                    TransferSpec::Remaining,
                )
                .with_connection(EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: Default::default(),
                    state: StateConnectionConfig {
                        role: StateConnectionRole::Activator,
                        target: StateConnectionTarget::Formula,
                        target_connection: Some(resource_edge_id),
                        ..Default::default()
                    },
                }),
            );
        let formula_target_error = compile_scenario(&formula_target_non_modifier_spec)
            .expect_err("formula target for non-modifier must fail");
        assert!(matches!(
            formula_target_error,
            SetupError::InvalidParameter { name, reason }
                if name == "edges.state-edge.connection.state.target"
                    && reason == "formula targets are only valid for modifier state connections"
        ));
    }

    #[test]
    fn compile_scenario_rejects_zero_delay_steps() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("delay"), NodeKind::Delay).with_config(
                NodeConfig::Delay(DelayNodeConfig { delay_steps: 0, mode: Default::default() }),
            ),
        );

        let error = compile_scenario(&spec).expect_err("zero delay steps must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.delay.config.delay_steps");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_zero_queue_release_per_step() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("queue"), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: Some(5),
                    release_per_step: 0,
                    mode: Default::default(),
                }),
            ),
        );

        let error = compile_scenario(&spec).expect_err("zero queue release_per_step must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.queue.config.release_per_step");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_rejects_zero_queue_capacity_when_specified() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("queue"), NodeKind::Queue).with_config(
                NodeConfig::Queue(QueueNodeConfig {
                    capacity: Some(0),
                    release_per_step: 1,
                    mode: Default::default(),
                }),
            ),
        );

        let error = compile_scenario(&spec).expect_err("zero queue capacity must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "nodes.queue.config.capacity");
                assert_eq!(reason, "must be greater than 0 when specified");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn compile_scenario_keeps_legacy_process_negative_start_compatible() {
        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario")).with_node(
            NodeSpec::new(crate::types::NodeId::fixture("process"), NodeKind::Process)
                .with_initial_value(-5.0),
        );

        compile_scenario(&spec).expect("legacy process negative start remains compatible");
    }

    #[test]
    fn compile_scenario_keeps_legacy_gate_inputs_compatible() {
        let source = crate::types::NodeId::fixture("source");
        let gate = crate::types::NodeId::fixture("gate");

        let spec = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(gate.clone(), NodeKind::Gate))
            .with_edge(EdgeSpec::new(
                crate::types::EdgeId::fixture("edge-1"),
                source,
                gate,
                TransferSpec::Fixed { amount: 1.0 },
            ));

        compile_scenario(&spec).expect("legacy gate input edges remain compatible");
    }

    #[test]
    fn validate_run_config_accepts_default() {
        let config = RunConfig::default();
        validate_run_config(&config).expect("default run config should be valid");
    }

    #[test]
    fn validate_run_config_rejects_zero_max_steps() {
        let config = RunConfig { seed: 42, max_steps: 0, capture: CaptureConfig::default() };

        let error = validate_run_config(&config).expect_err("zero max_steps must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "run.max_steps");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn legacy_capture_deserialization_rejects_zero_capture_interval() {
        let error = serde_json::from_str::<CaptureConfig>(
            r#"{"capture_nodes":[],"capture_metrics":[],"every_n_steps":0,"include_step_zero":true,"include_final_state":true}"#,
        )
        .expect_err("zero legacy capture interval must fail");
        assert!(error.to_string().contains("every_n_steps must be greater than 0"));
    }

    #[test]
    fn validate_batch_config_accepts_default() {
        let config = BatchConfig::default();
        validate_batch_config(&config).expect("default batch config should be valid");
    }

    #[test]
    fn validate_batch_config_rejects_zero_runs() {
        let config = BatchConfig {
            runs: 0,
            base_seed: 1,
            execution_mode: ExecutionMode::SingleThread,
            run_template: BatchRunTemplate::default(),
        };

        let error = validate_batch_config(&config).expect_err("zero runs must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "batch.runs");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn validate_batch_config_rejects_invalid_nested_run() {
        let config = BatchConfig {
            runs: 10,
            base_seed: 1,
            execution_mode: ExecutionMode::Rayon,
            run_template: BatchRunTemplate {
                max_steps: 0,
                aggregation: AggregationConfig::default(),
            },
        };

        let error = validate_batch_config(&config).expect_err("invalid nested run must fail");
        match error {
            SetupError::InvalidParameter { name, reason } => {
                assert_eq!(name, "batch.run_template.max_steps");
                assert_eq!(reason, "must be greater than 0");
            }
            other => panic!("expected InvalidParameter, got {other:?}"),
        }
    }

    #[test]
    fn checked_assembler_rejects_a_missing_required_expression() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let edge = crate::types::EdgeId::fixture("expression");
        let spec = ScenarioSpec::new(ScenarioId::fixture("missing-expression"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(
                edge,
                source,
                sink,
                TransferSpec::Expression { formula: "from".into() },
            ));
        let mut checked = Scenario::try_from(spec).expect("formula validates once");
        checked.expressions.transfer.clear();

        let error = assemble_checked_scenario(checked).expect_err("missing AST must fail");
        assert!(error.to_string().contains("edges.expression.transfer.expression.formula"));
    }

    #[test]
    fn checked_assembler_rejects_an_unexpected_expression_bundle_entry() {
        let source = crate::types::NodeId::fixture("source");
        let sink = crate::types::NodeId::fixture("sink");
        let edge = crate::types::EdgeId::fixture("remaining");
        let spec = ScenarioSpec::new(ScenarioId::fixture("unexpected-expression"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Process))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Pool))
            .with_edge(EdgeSpec::new(edge.clone(), source, sink, TransferSpec::Remaining));
        let mut checked = Scenario::try_from(spec).expect("scenario checks");
        checked
            .expressions
            .transfer
            .insert(edge, ExprRuntime::new().compile("1").expect("test expression compiles"));

        let error = assemble_checked_scenario(checked).expect_err("extra AST must fail");
        assert!(error.to_string().contains("scenario.expressions"));
    }
}
