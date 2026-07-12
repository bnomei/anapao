//! Immutable, semantically checked scenario values.
//!
//! This module deliberately has no serde derives.  [`ScenarioSpec`] remains the
//! stable document contract; a [`Scenario`] is the boundary after validation.

use std::collections::{btree_map::Entry, BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use crate::expr::CompiledExpr;

use super::{
    ConnectionKind, EdgeConnectionConfig, EdgeId, EdgeSpec, EndConditionSpec, MetricKey,
    NodeConfig, NodeId, NodeKind, NodeModeConfig, NodeSpec, PoolNodeConfig, ScenarioId,
    ScenarioSpec, StateConnectionConfig, StateConnectionRole, StateConnectionTarget, TransferSpec,
    VariableRuntimeConfig,
};
use crate::error::SetupError;

macro_rules! mode_config {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Default)]
        pub struct $name {
            mode: NodeModeConfig,
        }
        impl $name {
            pub fn mode(&self) -> &NodeModeConfig {
                &self.mode
            }
            #[must_use = "checked configuration methods return a new value; use the result"]
            pub fn with_mode(mut self, mode: NodeModeConfig) -> Self {
                self.mode = mode;
                self
            }
        }
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PoolConfig {
    capacity: Option<u64>,
    allow_negative_start: bool,
    mode: NodeModeConfig,
}
impl PoolConfig {
    pub fn capacity(&self) -> Option<u64> {
        self.capacity
    }
    pub fn allow_negative_start(&self) -> bool {
        self.allow_negative_start
    }
    pub fn mode(&self) -> &NodeModeConfig {
        &self.mode
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_capacity(mut self, capacity: u64) -> Self {
        self.capacity = Some(capacity);
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn without_capacity(mut self) -> Self {
        self.capacity = None;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_allow_negative_start(mut self, value: bool) -> Self {
        self.allow_negative_start = value;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_mode(mut self, mode: NodeModeConfig) -> Self {
        self.mode = mode;
        self
    }
}
mode_config!(DrainConfig);
mode_config!(SortingGateConfig);
mode_config!(TriggerGateConfig);
mode_config!(MixedGateConfig);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConverterConfig {
    ignore_disabled_inputs: bool,
    mode: NodeModeConfig,
}
impl ConverterConfig {
    pub fn ignore_disabled_inputs(&self) -> bool {
        self.ignore_disabled_inputs
    }
    pub fn mode(&self) -> &NodeModeConfig {
        &self.mode
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_ignore_disabled_inputs(mut self, v: bool) -> Self {
        self.ignore_disabled_inputs = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_mode(mut self, v: NodeModeConfig) -> Self {
        self.mode = v;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TraderConfig {
    ignore_disabled_inputs: bool,
    mode: NodeModeConfig,
}
impl TraderConfig {
    pub fn ignore_disabled_inputs(&self) -> bool {
        self.ignore_disabled_inputs
    }
    pub fn mode(&self) -> &NodeModeConfig {
        &self.mode
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_ignore_disabled_inputs(mut self, v: bool) -> Self {
        self.ignore_disabled_inputs = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_mode(mut self, v: NodeModeConfig) -> Self {
        self.mode = v;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RegisterConfig {
    interactive: bool,
    min_value: Option<i64>,
    max_value: Option<i64>,
}
impl RegisterConfig {
    pub fn interactive(&self) -> bool {
        self.interactive
    }
    pub fn min_value(&self) -> Option<i64> {
        self.min_value
    }
    pub fn max_value(&self) -> Option<i64> {
        self.max_value
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_interactive(mut self, v: bool) -> Self {
        self.interactive = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_min_value(mut self, v: i64) -> Self {
        self.min_value = Some(v);
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn without_min_value(mut self) -> Self {
        self.min_value = None;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_max_value(mut self, v: i64) -> Self {
        self.max_value = Some(v);
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn without_max_value(mut self) -> Self {
        self.max_value = None;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DelayConfig {
    delay_steps: NonZeroU64,
    mode: NodeModeConfig,
}
impl Default for DelayConfig {
    fn default() -> Self {
        Self { delay_steps: NonZeroU64::MIN, mode: NodeModeConfig::default() }
    }
}
impl DelayConfig {
    pub fn delay_steps(&self) -> NonZeroU64 {
        self.delay_steps
    }
    pub fn mode(&self) -> &NodeModeConfig {
        &self.mode
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_delay_steps(mut self, v: NonZeroU64) -> Self {
        self.delay_steps = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_mode(mut self, v: NodeModeConfig) -> Self {
        self.mode = v;
        self
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueConfig {
    capacity: Option<NonZeroU64>,
    release_per_step: NonZeroU64,
    mode: NodeModeConfig,
}
impl Default for QueueConfig {
    fn default() -> Self {
        Self { capacity: None, release_per_step: NonZeroU64::MIN, mode: NodeModeConfig::default() }
    }
}
impl QueueConfig {
    pub fn capacity(&self) -> Option<NonZeroU64> {
        self.capacity
    }
    pub fn release_per_step(&self) -> NonZeroU64 {
        self.release_per_step
    }
    pub fn mode(&self) -> &NodeModeConfig {
        &self.mode
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_capacity(mut self, v: NonZeroU64) -> Self {
        self.capacity = Some(v);
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn without_capacity(mut self) -> Self {
        self.capacity = None;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_release_per_step(mut self, v: NonZeroU64) -> Self {
        self.release_per_step = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_mode(mut self, v: NodeModeConfig) -> Self {
        self.mode = v;
        self
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeBehavior {
    Source,
    Pool(PoolConfig),
    Drain(DrainConfig),
    SortingGate(SortingGateConfig),
    TriggerGate(TriggerGateConfig),
    MixedGate(MixedGateConfig),
    Converter(ConverterConfig),
    Trader(TraderConfig),
    Register(RegisterConfig),
    Delay(DelayConfig),
    Queue(QueueConfig),
    Process,
    Sink,
    Gate,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioNode {
    id: NodeId,
    behavior: NodeBehavior,
    label: Option<String>,
    initial_value: f64,
    tags: BTreeSet<String>,
    metadata: BTreeMap<String, String>,
}
impl ScenarioNode {
    fn new(id: NodeId, behavior: NodeBehavior) -> Self {
        Self {
            id,
            behavior,
            label: None,
            initial_value: 0.0,
            tags: BTreeSet::new(),
            metadata: BTreeMap::new(),
        }
    }
    pub fn id(&self) -> &NodeId {
        &self.id
    }
    pub fn behavior(&self) -> &NodeBehavior {
        &self.behavior
    }
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }
    pub fn initial_value(&self) -> f64 {
        self.initial_value
    }
    pub fn tags(&self) -> &BTreeSet<String> {
        &self.tags
    }
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }
    pub fn source(id: NodeId) -> Self {
        Self::new(id, NodeBehavior::Source)
    }
    pub fn pool(id: NodeId, c: PoolConfig) -> Self {
        Self::new(id, NodeBehavior::Pool(c))
    }
    pub fn drain(id: NodeId, c: DrainConfig) -> Self {
        Self::new(id, NodeBehavior::Drain(c))
    }
    pub fn sorting_gate(id: NodeId, c: SortingGateConfig) -> Self {
        Self::new(id, NodeBehavior::SortingGate(c))
    }
    pub fn trigger_gate(id: NodeId, c: TriggerGateConfig) -> Self {
        Self::new(id, NodeBehavior::TriggerGate(c))
    }
    pub fn mixed_gate(id: NodeId, c: MixedGateConfig) -> Self {
        Self::new(id, NodeBehavior::MixedGate(c))
    }
    pub fn converter(id: NodeId, c: ConverterConfig) -> Self {
        Self::new(id, NodeBehavior::Converter(c))
    }
    pub fn trader(id: NodeId, c: TraderConfig) -> Self {
        Self::new(id, NodeBehavior::Trader(c))
    }
    pub fn register(id: NodeId, c: RegisterConfig) -> Self {
        Self::new(id, NodeBehavior::Register(c))
    }
    pub fn delay(id: NodeId, c: DelayConfig) -> Self {
        Self::new(id, NodeBehavior::Delay(c))
    }
    pub fn queue(id: NodeId, c: QueueConfig) -> Self {
        Self::new(id, NodeBehavior::Queue(c))
    }
    pub fn process(id: NodeId) -> Self {
        Self::new(id, NodeBehavior::Process)
    }
    pub fn sink(id: NodeId) -> Self {
        Self::new(id, NodeBehavior::Sink)
    }
    pub fn gate(id: NodeId) -> Self {
        Self::new(id, NodeBehavior::Gate)
    }
    pub fn custom(id: NodeId, v: impl Into<String>) -> Self {
        Self::new(id, NodeBehavior::Custom(v.into()))
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_initial_value(mut self, v: f64) -> Self {
        self.initial_value = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_tag(mut self, v: impl Into<String>) -> Self {
        self.tags.insert(v.into());
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_metadata(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.metadata.insert(k.into(), v.into());
        self
    }
    pub(crate) fn from_parts(
        id: NodeId,
        behavior: NodeBehavior,
        label: Option<String>,
        initial_value: f64,
        tags: BTreeSet<String>,
        metadata: BTreeMap<String, String>,
    ) -> Self {
        Self { id, behavior, label, initial_value, tags, metadata }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceConnection {
    token_size: NonZeroU64,
}
impl Default for ResourceConnection {
    fn default() -> Self {
        Self { token_size: NonZeroU64::MIN }
    }
}
impl ResourceConnection {
    pub fn token_size(&self) -> NonZeroU64 {
        self.token_size
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_token_size(mut self, v: NonZeroU64) -> Self {
        self.token_size = v;
        self
    }
}
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StateTarget {
    Node,
    ResourceConnection(EdgeId),
    StateConnection(EdgeId),
    Formula(EdgeId),
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateConnection {
    role: StateConnectionRole,
    formula: String,
    target: StateTarget,
    resource_filter: Option<String>,
}
impl Default for StateConnection {
    fn default() -> Self {
        Self {
            role: StateConnectionRole::Modifier,
            formula: "+1".into(),
            target: StateTarget::Node,
            resource_filter: None,
        }
    }
}
impl StateConnection {
    pub fn new(role: StateConnectionRole, formula: impl Into<String>, target: StateTarget) -> Self {
        Self { role, formula: formula.into(), target, resource_filter: None }
    }
    pub fn role(&self) -> &StateConnectionRole {
        &self.role
    }
    pub fn formula(&self) -> &str {
        &self.formula
    }
    pub fn target(&self) -> &StateTarget {
        &self.target
    }
    pub fn resource_filter(&self) -> Option<&str> {
        self.resource_filter.as_deref()
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_role(mut self, v: StateConnectionRole) -> Self {
        self.role = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_formula(mut self, v: impl Into<String>) -> Self {
        self.formula = v.into();
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_target(mut self, v: StateTarget) -> Self {
        self.target = v;
        self
    }
    #[must_use = "checked configuration methods return a new value; use the result"]
    pub fn with_resource_filter(mut self, v: impl Into<String>) -> Self {
        self.resource_filter = Some(v.into());
        self
    }
}
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionSpec {
    Resource(ResourceConnection),
    State(StateConnection),
}
#[derive(Debug, Clone, PartialEq)]
pub struct ScenarioEdge {
    id: EdgeId,
    from: NodeId,
    to: NodeId,
    transfer: TransferSpec,
    connection: ConnectionSpec,
    enabled: bool,
    metadata: BTreeMap<String, String>,
}
impl ScenarioEdge {
    fn new(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        transfer: TransferSpec,
        connection: ConnectionSpec,
    ) -> Self {
        Self { id, from, to, transfer, connection, enabled: true, metadata: BTreeMap::new() }
    }
    pub fn resource(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        transfer: TransferSpec,
        c: ResourceConnection,
    ) -> Self {
        Self::new(id, from, to, transfer, ConnectionSpec::Resource(c))
    }
    pub fn state(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        transfer: TransferSpec,
        c: StateConnection,
    ) -> Self {
        Self::new(id, from, to, transfer, ConnectionSpec::State(c))
    }
    pub fn id(&self) -> &EdgeId {
        &self.id
    }
    pub fn from(&self) -> &NodeId {
        &self.from
    }
    pub fn to(&self) -> &NodeId {
        &self.to
    }
    pub fn transfer(&self) -> &TransferSpec {
        &self.transfer
    }
    pub fn connection(&self) -> &ConnectionSpec {
        &self.connection
    }
    pub fn enabled(&self) -> bool {
        self.enabled
    }
    pub fn metadata(&self) -> &BTreeMap<String, String> {
        &self.metadata
    }
    #[must_use = "checked edge authoring methods return a new value; use the result"]
    pub fn with_enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }
    #[must_use = "checked edge authoring methods return a new value; use the result"]
    pub fn with_metadata(mut self, k: impl Into<String>, v: impl Into<String>) -> Self {
        self.metadata.insert(k.into(), v.into());
        self
    }
    pub(crate) fn from_parts(
        id: EdgeId,
        from: NodeId,
        to: NodeId,
        transfer: TransferSpec,
        connection: ConnectionSpec,
        enabled: bool,
        metadata: BTreeMap<String, String>,
    ) -> Self {
        Self { id, from, to, transfer, connection, enabled, metadata }
    }
}

/// Conventional checked authoring surface for a complete scenario document.
///
/// Unlike the legacy [`ScenarioSpec`] helpers, duplicate node and edge identifiers are rejected
/// and the first definition is retained.
///
/// ```
/// use anapao::types::{
///     EdgeId, NodeId, ResourceConnection, ScenarioBuilder, ScenarioEdge, ScenarioId,
///     ScenarioNode, TransferSpec,
/// };
///
/// let source = NodeId::fixture("source");
/// let sink = NodeId::fixture("sink");
/// let scenario = ScenarioBuilder::new(ScenarioId::fixture("authored"))
///     .with_node(ScenarioNode::source(source.clone()).with_initial_value(1.0))?
///     .with_node(ScenarioNode::sink(sink.clone()))?
///     .with_edge(ScenarioEdge::resource(
///         EdgeId::fixture("source-sink"),
///         source,
///         sink,
///         TransferSpec::Fixed { amount: 1.0 },
///         ResourceConnection::default(),
///     ))?
///     .build()?;
/// assert_eq!(scenario.nodes().len(), 2);
/// # Ok::<(), anapao::error::SetupError>(())
/// ```
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use anapao::types::{NodeId, ScenarioNode};
/// ScenarioNode::source(NodeId::fixture("source")).with_tag("discarded");
/// ```
///
/// ```compile_fail
/// #![deny(unused_must_use)]
/// use anapao::types::{NodeId, NodeKind, NodeSpec};
/// NodeSpec::new(NodeId::fixture("source"), NodeKind::Source).with_initial_value(1.0);
/// ```
#[must_use = "a ScenarioBuilder must be built or otherwise used"]
#[derive(Debug, Clone)]
pub struct ScenarioBuilder {
    draft: ScenarioSpec,
}

impl ScenarioBuilder {
    /// Starts an empty scenario using [`ScenarioSpec::new`] document defaults.
    pub fn new(id: ScenarioId) -> Self {
        Self { draft: ScenarioSpec::new(id) }
    }

    /// Inserts a checked node, rejecting a duplicate without replacing the original value.
    pub fn insert_node(&mut self, node: ScenarioNode) -> Result<(), SetupError> {
        let id = node.id().clone();
        match self.draft.nodes.entry(id.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(node.into());
                Ok(())
            }
            Entry::Occupied(_) => Err(duplicate_error("nodes", id.as_ref())),
        }
    }

    /// Inserts a checked edge, rejecting a duplicate without replacing the original value.
    pub fn insert_edge(&mut self, edge: ScenarioEdge) -> Result<(), SetupError> {
        let id = edge.id().clone();
        match self.draft.edges.entry(id.clone()) {
            Entry::Vacant(slot) => {
                slot.insert(edge.into());
                Ok(())
            }
            Entry::Occupied(_) => Err(duplicate_error("edges", id.as_ref())),
        }
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_node(mut self, node: ScenarioNode) -> Result<Self, SetupError> {
        self.insert_node(node)?;
        Ok(self)
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_edge(mut self, edge: ScenarioEdge) -> Result<Self, SetupError> {
        self.insert_edge(edge)?;
        Ok(self)
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.draft.title = Some(title.into());
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.draft.description = Some(description.into());
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.draft.tags.insert(tag.into());
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_variables(mut self, variables: VariableRuntimeConfig) -> Self {
        self.draft.variables = variables;
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_end_condition(mut self, condition: EndConditionSpec) -> Self {
        self.draft.end_conditions = vec![condition];
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_end_conditions<I>(mut self, conditions: I) -> Self
    where
        I: IntoIterator<Item = EndConditionSpec>,
    {
        self.draft.end_conditions = conditions.into_iter().collect();
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn push_end_condition(mut self, condition: EndConditionSpec) -> Self {
        self.draft.end_conditions.push(condition);
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_tracked_metric(mut self, metric: MetricKey) -> Self {
        self.draft.tracked_metrics.insert(metric);
        self
    }

    #[must_use = "ScenarioBuilder methods return a new builder; use the result"]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.draft.metadata.insert(key.into(), value.into());
        self
    }

    /// Validates the complete draft once, through the shared checked conversion gate.
    pub fn build(self) -> Result<Scenario, SetupError> {
        Scenario::try_from(self.draft)
    }
}

fn duplicate_error(kind: &str, id: &str) -> SetupError {
    SetupError::InvalidParameter {
        name: format!("{kind}.{id}"),
        reason: "a definition with this id already exists".into(),
    }
}

impl From<ScenarioNode> for NodeSpec {
    fn from(node: ScenarioNode) -> Self {
        let ScenarioNode { id, behavior, label, initial_value, tags, metadata } = node;
        let (kind, config) = match behavior {
            NodeBehavior::Source => (NodeKind::Source, NodeConfig::None),
            NodeBehavior::Pool(c) => (
                NodeKind::Pool,
                NodeConfig::Pool(PoolNodeConfig {
                    capacity: c.capacity,
                    allow_negative_start: c.allow_negative_start,
                    mode: c.mode,
                }),
            ),
            NodeBehavior::Drain(c) => {
                (NodeKind::Drain, NodeConfig::Drain(super::DrainNodeConfig { mode: c.mode }))
            }
            NodeBehavior::SortingGate(c) => (
                NodeKind::SortingGate,
                NodeConfig::SortingGate(super::SortingGateNodeConfig { mode: c.mode }),
            ),
            NodeBehavior::TriggerGate(c) => (
                NodeKind::TriggerGate,
                NodeConfig::TriggerGate(super::TriggerGateNodeConfig { mode: c.mode }),
            ),
            NodeBehavior::MixedGate(c) => (
                NodeKind::MixedGate,
                NodeConfig::MixedGate(super::MixedGateNodeConfig { mode: c.mode }),
            ),
            NodeBehavior::Converter(c) => (
                NodeKind::Converter,
                NodeConfig::Converter(super::ConverterNodeConfig {
                    ignore_disabled_inputs: c.ignore_disabled_inputs,
                    mode: c.mode,
                }),
            ),
            NodeBehavior::Trader(c) => (
                NodeKind::Trader,
                NodeConfig::Trader(super::TraderNodeConfig {
                    ignore_disabled_inputs: c.ignore_disabled_inputs,
                    mode: c.mode,
                }),
            ),
            NodeBehavior::Register(c) => (
                NodeKind::Register,
                NodeConfig::Register(super::RegisterNodeConfig {
                    interactive: c.interactive,
                    min_value: c.min_value,
                    max_value: c.max_value,
                }),
            ),
            NodeBehavior::Delay(c) => (
                NodeKind::Delay,
                NodeConfig::Delay(super::DelayNodeConfig {
                    delay_steps: c.delay_steps.get(),
                    mode: c.mode,
                }),
            ),
            NodeBehavior::Queue(c) => (
                NodeKind::Queue,
                NodeConfig::Queue(super::QueueNodeConfig {
                    capacity: c.capacity.map(NonZeroU64::get),
                    release_per_step: c.release_per_step.get(),
                    mode: c.mode,
                }),
            ),
            NodeBehavior::Process => (NodeKind::Process, NodeConfig::None),
            NodeBehavior::Sink => (NodeKind::Sink, NodeConfig::None),
            NodeBehavior::Gate => (NodeKind::Gate, NodeConfig::None),
            NodeBehavior::Custom(value) => (NodeKind::Custom(value), NodeConfig::None),
        };
        NodeSpec { id, kind, config, label, initial_value, tags, metadata }
    }
}

impl From<ScenarioEdge> for EdgeSpec {
    fn from(edge: ScenarioEdge) -> Self {
        let ScenarioEdge { id, from, to, transfer, connection, enabled, metadata } = edge;
        let connection = match connection {
            ConnectionSpec::Resource(resource) => EdgeConnectionConfig {
                kind: ConnectionKind::Resource,
                resource: super::ResourceConnectionConfig { token_size: resource.token_size.get() },
                state: StateConnectionConfig::default(),
            },
            ConnectionSpec::State(state) => {
                let (target, target_connection) = match state.target {
                    StateTarget::Node => (StateConnectionTarget::Node, None),
                    StateTarget::ResourceConnection(id) => {
                        (StateConnectionTarget::ResourceConnection, Some(id))
                    }
                    StateTarget::StateConnection(id) => {
                        (StateConnectionTarget::StateConnection, Some(id))
                    }
                    StateTarget::Formula(id) => (StateConnectionTarget::Formula, Some(id)),
                };
                EdgeConnectionConfig {
                    kind: ConnectionKind::State,
                    resource: super::ResourceConnectionConfig::default(),
                    state: StateConnectionConfig {
                        role: state.role,
                        formula: state.formula,
                        target,
                        target_connection,
                        resource_filter: state.resource_filter,
                    },
                }
            }
        };
        EdgeSpec { id, from, to, transfer, connection, enabled, metadata }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ValidatedExpressions {
    pub(crate) transfer: BTreeMap<EdgeId, CompiledExpr>,
    pub(crate) state: BTreeMap<EdgeId, CompiledExpr>,
}
#[derive(Debug, Clone)]
pub struct Scenario {
    source: ScenarioSpec,
    nodes: BTreeMap<NodeId, ScenarioNode>,
    edges: BTreeMap<EdgeId, ScenarioEdge>,
    #[allow(dead_code, reason = "moved into the opaque compiled plan by the checked compile path")]
    pub(crate) expressions: ValidatedExpressions,
}
impl Scenario {
    pub fn id(&self) -> &ScenarioId {
        &self.source.id
    }
    pub fn source_spec(&self) -> &ScenarioSpec {
        &self.source
    }
    pub fn nodes(&self) -> &BTreeMap<NodeId, ScenarioNode> {
        &self.nodes
    }
    pub fn edges(&self) -> &BTreeMap<EdgeId, ScenarioEdge> {
        &self.edges
    }
    pub(crate) fn from_parts(
        source: ScenarioSpec,
        nodes: BTreeMap<NodeId, ScenarioNode>,
        edges: BTreeMap<EdgeId, ScenarioEdge>,
        expressions: ValidatedExpressions,
    ) -> Self {
        Self { source, nodes, edges, expressions }
    }
}
impl From<&Scenario> for ScenarioSpec {
    fn from(v: &Scenario) -> Self {
        v.source.clone()
    }
}
impl From<Scenario> for ScenarioSpec {
    fn from(v: Scenario) -> Self {
        v.source
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        ConnectionKind, EdgeConnectionConfig, EdgeSpec, NodeConfig, NodeKind, NodeSpec,
        PoolNodeConfig, ScenarioId, StateConnectionConfig, StateConnectionRole,
        StateConnectionTarget,
    };

    #[test]
    fn checked_round_trip_preserves_the_parsed_document() {
        let spec = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 });
        let checked = Scenario::try_from(spec.clone()).expect("valid document checks");
        assert_eq!(
            serde_json::to_value(spec).unwrap(),
            serde_json::to_value(ScenarioSpec::from(&checked)).unwrap()
        );
        assert!(matches!(
            checked.nodes()[&NodeId::fixture("source")].behavior(),
            NodeBehavior::Source
        ));
    }

    #[test]
    fn scenario_builder_supports_mutating_and_consuming_authoring() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let edge = EdgeId::fixture("source-sink");

        let mut mutating = ScenarioBuilder::new(ScenarioId::fixture("builder"));
        mutating.insert_node(ScenarioNode::source(source.clone()).with_initial_value(1.0)).unwrap();
        mutating.insert_node(ScenarioNode::sink(sink.clone())).unwrap();
        mutating
            .insert_edge(ScenarioEdge::resource(
                edge.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
                ResourceConnection::default(),
            ))
            .unwrap();
        let mutating = mutating
            .with_title("A builder")
            .with_description("checked authoring")
            .with_tag("example")
            .with_variables(VariableRuntimeConfig::default())
            .with_end_conditions([EndConditionSpec::MaxSteps { steps: 2 }])
            .push_end_condition(EndConditionSpec::NodeAtLeast {
                node_id: sink.clone(),
                value_scaled: 1,
            })
            .with_tracked_metric(MetricKey::fixture("sink"))
            .with_metadata("source", "unit")
            .build()
            .unwrap();

        let consuming = ScenarioBuilder::new(ScenarioId::fixture("builder"))
            .with_node(ScenarioNode::source(source.clone()).with_initial_value(1.0))
            .unwrap()
            .with_node(ScenarioNode::sink(sink.clone()))
            .unwrap()
            .with_edge(ScenarioEdge::resource(
                edge,
                source,
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
                ResourceConnection::default(),
            ))
            .unwrap()
            .with_title("A builder")
            .with_description("checked authoring")
            .with_tag("example")
            .with_variables(VariableRuntimeConfig::default())
            .with_end_conditions([EndConditionSpec::MaxSteps { steps: 2 }])
            .push_end_condition(EndConditionSpec::NodeAtLeast { node_id: sink, value_scaled: 1 })
            .with_tracked_metric(MetricKey::fixture("sink"))
            .with_metadata("source", "unit")
            .build()
            .unwrap();

        assert_eq!(ScenarioSpec::from(mutating), ScenarioSpec::from(consuming));
    }

    #[test]
    fn scenario_builder_duplicate_insertion_keeps_the_first_definition() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let mut builder = ScenarioBuilder::new(ScenarioId::fixture("duplicates"));
        let first_node = ScenarioNode::source(source.clone())
            .with_label("first source")
            .with_initial_value(1.0)
            .with_tag("retained")
            .with_metadata("definition", "first");
        builder.insert_node(first_node.clone()).unwrap();
        let node_error = builder
            .insert_node(
                ScenarioNode::custom(source.clone(), "replacement")
                    .with_label("second source")
                    .with_initial_value(2.0)
                    .with_tag("rejected")
                    .with_metadata("definition", "second"),
            )
            .unwrap_err();
        assert!(
            matches!(node_error, SetupError::InvalidParameter { ref name, .. } if name == "nodes.source")
        );
        builder.insert_node(ScenarioNode::sink(sink.clone())).unwrap();

        let edge_id = EdgeId::fixture("source-sink");
        let first_edge = ScenarioEdge::resource(
            edge_id.clone(),
            source.clone(),
            sink,
            TransferSpec::Fixed { amount: 1.0 },
            ResourceConnection::default(),
        )
        .with_metadata("definition", "first");
        builder.insert_edge(first_edge.clone()).unwrap();
        let edge_error = builder
            .insert_edge(
                ScenarioEdge::resource(
                    edge_id.clone(),
                    source.clone(),
                    source.clone(),
                    TransferSpec::Fixed { amount: 2.0 },
                    ResourceConnection::default().with_token_size(NonZeroU64::new(2).unwrap()),
                )
                .with_enabled(false)
                .with_metadata("definition", "second"),
            )
            .unwrap_err();
        assert!(
            matches!(edge_error, SetupError::InvalidParameter { ref name, .. } if name == "edges.source-sink")
        );

        let scenario = builder.build().unwrap();
        assert_eq!(&scenario.nodes()[&source], &first_node);
        assert_eq!(&scenario.edges()[&edge_id], &first_edge);
    }

    #[test]
    fn scenario_builder_consuming_duplicates_return_errors() {
        let source = NodeId::fixture("source");
        let node_error = ScenarioBuilder::new(ScenarioId::fixture("duplicate-node"))
            .with_node(ScenarioNode::source(source.clone()))
            .unwrap()
            .with_node(ScenarioNode::custom(source.clone(), "replacement"))
            .unwrap_err();
        assert!(
            matches!(node_error, SetupError::InvalidParameter { ref name, .. } if name == "nodes.source")
        );

        let sink = NodeId::fixture("sink");
        let edge_id = EdgeId::fixture("source-sink");
        let edge_error = ScenarioBuilder::new(ScenarioId::fixture("duplicate-edge"))
            .with_node(ScenarioNode::source(source.clone()))
            .unwrap()
            .with_node(ScenarioNode::sink(sink.clone()))
            .unwrap()
            .with_edge(ScenarioEdge::resource(
                edge_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Fixed { amount: 1.0 },
                ResourceConnection::default(),
            ))
            .unwrap()
            .with_edge(ScenarioEdge::resource(
                edge_id,
                source,
                sink,
                TransferSpec::Fixed { amount: 2.0 },
                ResourceConnection::default(),
            ))
            .unwrap_err();
        assert!(
            matches!(edge_error, SetupError::InvalidParameter { ref name, .. } if name == "edges.source-sink")
        );
    }

    #[test]
    fn legacy_dto_helpers_still_replace_by_identifier() {
        let id = NodeId::fixture("source");
        let spec = ScenarioSpec::new(ScenarioId::fixture("legacy"))
            .with_node(NodeSpec::new(id.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(id.clone(), NodeKind::Source).with_initial_value(2.0));
        assert_eq!(spec.nodes[&id].initial_value, 2.0);
    }

    #[test]
    fn key_drift_is_rejected_before_later_graph_errors() {
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("checked"));
        let node = NodeSpec::new(NodeId::fixture("embedded"), NodeKind::Source);
        spec.nodes.insert(NodeId::fixture("key"), node);
        let error = Scenario::try_from(spec).expect_err("key drift is invalid");
        assert!(error.to_string().contains("nodes.key.id"));
    }

    #[test]
    fn checked_connections_require_positive_token_sizes() {
        let mut spec = ScenarioSpec::source_sink(TransferSpec::Fixed { amount: 1.0 });
        let edge = spec.edges.get_mut(&EdgeId::fixture("edge-source-sink")).unwrap();
        edge.connection.resource.token_size = 0;
        let error = Scenario::try_from(spec).expect_err("zero token size is invalid");
        assert!(error.to_string().contains("token_size"));
    }

    #[test]
    fn checked_nodes_cover_every_kind_and_default_config() {
        let cases = [
            ("source", NodeKind::Source),
            ("pool", NodeKind::Pool),
            ("drain", NodeKind::Drain),
            ("sorting", NodeKind::SortingGate),
            ("trigger", NodeKind::TriggerGate),
            ("mixed", NodeKind::MixedGate),
            ("converter", NodeKind::Converter),
            ("trader", NodeKind::Trader),
            ("register", NodeKind::Register),
            ("delay", NodeKind::Delay),
            ("queue", NodeKind::Queue),
            ("process", NodeKind::Process),
            ("sink", NodeKind::Sink),
            ("gate", NodeKind::Gate),
            ("custom", NodeKind::Custom("widget".into())),
        ];
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("node-matrix"));
        for (name, kind) in cases {
            spec = spec.with_node(NodeSpec::new(NodeId::fixture(name), kind));
        }
        for (id, from, to) in [
            ("source-converter", "source", "converter"),
            ("converter-sink", "converter", "sink"),
            ("source-trader", "source", "trader"),
            ("trader-sink", "trader", "sink"),
        ] {
            spec = spec.with_edge(EdgeSpec::new(
                EdgeId::fixture(id),
                NodeId::fixture(from),
                NodeId::fixture(to),
                TransferSpec::Remaining,
            ));
        }

        let checked = Scenario::try_from(spec).expect("all documented node defaults check");
        let expected = [
            ("source", "Source"),
            ("pool", "Pool"),
            ("drain", "Drain"),
            ("sorting", "SortingGate"),
            ("trigger", "TriggerGate"),
            ("mixed", "MixedGate"),
            ("converter", "Converter"),
            ("trader", "Trader"),
            ("register", "Register"),
            ("delay", "Delay"),
            ("queue", "Queue"),
            ("process", "Process"),
            ("sink", "Sink"),
            ("gate", "Gate"),
            ("custom", "Custom"),
        ];
        for (id, behavior) in expected {
            let node = &checked.nodes()[&NodeId::fixture(id)];
            assert_eq!(
                match node.behavior() {
                    NodeBehavior::Source => "Source",
                    NodeBehavior::Pool(_) => "Pool",
                    NodeBehavior::Drain(_) => "Drain",
                    NodeBehavior::SortingGate(_) => "SortingGate",
                    NodeBehavior::TriggerGate(_) => "TriggerGate",
                    NodeBehavior::MixedGate(_) => "MixedGate",
                    NodeBehavior::Converter(_) => "Converter",
                    NodeBehavior::Trader(_) => "Trader",
                    NodeBehavior::Register(_) => "Register",
                    NodeBehavior::Delay(_) => "Delay",
                    NodeBehavior::Queue(_) => "Queue",
                    NodeBehavior::Process => "Process",
                    NodeBehavior::Sink => "Sink",
                    NodeBehavior::Gate => "Gate",
                    NodeBehavior::Custom(_) => "Custom",
                },
                behavior
            );
        }
    }

    #[test]
    fn checked_node_config_matrix_accepts_matching_and_rejects_mismatches() {
        let mut matching = ScenarioSpec::new(ScenarioId::fixture("configs"));
        let configurations = [
            ("pool", NodeKind::Pool, NodeConfig::Pool(PoolNodeConfig::default())),
            ("drain", NodeKind::Drain, NodeConfig::Drain(Default::default())),
            ("sorting", NodeKind::SortingGate, NodeConfig::SortingGate(Default::default())),
            ("trigger", NodeKind::TriggerGate, NodeConfig::TriggerGate(Default::default())),
            ("mixed", NodeKind::MixedGate, NodeConfig::MixedGate(Default::default())),
            ("converter", NodeKind::Converter, NodeConfig::Converter(Default::default())),
            ("trader", NodeKind::Trader, NodeConfig::Trader(Default::default())),
            ("register", NodeKind::Register, NodeConfig::Register(Default::default())),
            ("delay", NodeKind::Delay, NodeConfig::Delay(Default::default())),
            ("queue", NodeKind::Queue, NodeConfig::Queue(Default::default())),
        ];
        for (id, kind, config) in configurations {
            matching =
                matching.with_node(NodeSpec::new(NodeId::fixture(id), kind).with_config(config));
        }
        matching = matching
            .with_node(NodeSpec::new(NodeId::fixture("source"), NodeKind::Source))
            .with_node(NodeSpec::new(NodeId::fixture("sink"), NodeKind::Sink));
        for (id, from, to) in [
            ("source-converter", "source", "converter"),
            ("converter-sink", "converter", "sink"),
            ("source-trader", "source", "trader"),
            ("trader-sink", "trader", "sink"),
        ] {
            matching = matching.with_edge(EdgeSpec::new(
                EdgeId::fixture(id),
                NodeId::fixture(from),
                NodeId::fixture(to),
                TransferSpec::Remaining,
            ));
        }
        Scenario::try_from(matching).expect("every matching family config checks");

        let cases = [
            ("source", NodeKind::Source),
            ("pool", NodeKind::Pool),
            ("drain", NodeKind::Drain),
            ("sorting", NodeKind::SortingGate),
            ("trigger", NodeKind::TriggerGate),
            ("mixed", NodeKind::MixedGate),
            ("converter", NodeKind::Converter),
            ("trader", NodeKind::Trader),
            ("register", NodeKind::Register),
            ("delay", NodeKind::Delay),
            ("queue", NodeKind::Queue),
            ("process", NodeKind::Process),
            ("sink", NodeKind::Sink),
            ("gate", NodeKind::Gate),
            ("custom", NodeKind::Custom("widget".into())),
        ];
        for (id, kind) in cases {
            let wrong_config = if id == "pool" {
                NodeConfig::Drain(Default::default())
            } else {
                NodeConfig::Pool(PoolNodeConfig::default())
            };
            let spec = ScenarioSpec::new(ScenarioId::fixture("config-mismatch"))
                .with_node(NodeSpec::new(NodeId::fixture(id), kind).with_config(wrong_config));
            let error = Scenario::try_from(spec).expect_err("wrong config family must fail");
            assert!(error.to_string().contains(&format!("nodes.{id}.config")));
        }
    }

    #[test]
    fn checked_edges_cover_resource_and_every_state_target() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let resource_id = EdgeId::fixture("resource");
        let state_id = EdgeId::fixture("state");
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("edge-matrix"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(
                resource_id.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Remaining,
            ));
        for (name, target, target_connection) in [
            ("node", StateConnectionTarget::Node, None),
            (
                "resource-target",
                StateConnectionTarget::ResourceConnection,
                Some(resource_id.clone()),
            ),
            ("state-target", StateConnectionTarget::StateConnection, Some(state_id.clone())),
            ("formula-target", StateConnectionTarget::Formula, Some(resource_id.clone())),
        ] {
            let id = if name == "state-target" { state_id.clone() } else { EdgeId::fixture(name) };
            let connection = EdgeConnectionConfig {
                kind: ConnectionKind::State,
                state: StateConnectionConfig {
                    role: StateConnectionRole::Modifier,
                    formula: "+1".into(),
                    target,
                    target_connection,
                    resource_filter: None,
                },
                ..Default::default()
            };
            spec = spec.with_edge(
                EdgeSpec::new(id, source.clone(), sink.clone(), TransferSpec::Remaining)
                    .with_connection(connection),
            );
        }
        let checked = Scenario::try_from(spec).expect("resource and all state targets check");
        assert!(matches!(checked.edges()[&resource_id].connection(), ConnectionSpec::Resource(_)));
        assert!(matches!(
            checked.edges()[&EdgeId::fixture("node")].connection(),
            ConnectionSpec::State(StateConnection { target: StateTarget::Node, .. })
        ));
        assert!(matches!(
            checked.edges()[&EdgeId::fixture("resource-target")].connection(),
            ConnectionSpec::State(StateConnection {
                target: StateTarget::ResourceConnection(_),
                ..
            })
        ));
        assert!(matches!(
            checked.edges()[&state_id].connection(),
            ConnectionSpec::State(StateConnection { target: StateTarget::StateConnection(_), .. })
        ));
        assert!(matches!(
            checked.edges()[&EdgeId::fixture("formula-target")].connection(),
            ConnectionSpec::State(StateConnection { target: StateTarget::Formula(_), .. })
        ));
    }

    #[test]
    fn checked_formula_bundle_includes_only_compiled_formula_kinds() {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");
        let mut spec = ScenarioSpec::new(ScenarioId::fixture("formula-matrix"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink));
        let resource_expression = EdgeId::fixture("resource-expression");
        spec = spec.with_edge(
            EdgeSpec::new(
                resource_expression.clone(),
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "1".into() },
            )
            .with_connection(EdgeConnectionConfig::default()),
        );
        spec.edges.get_mut(&resource_expression).unwrap().enabled = false;

        // A node-target modifier provides the state-connection target used below. Each
        // subsequent modifier must retain its AST regardless of which state target it writes.
        let node_modifier = EdgeId::fixture("modifier-node");
        for (id, target, target_connection, enabled) in [
            (node_modifier.clone(), StateConnectionTarget::Node, None, true),
            (
                EdgeId::fixture("modifier-resource"),
                StateConnectionTarget::ResourceConnection,
                Some(resource_expression.clone()),
                true,
            ),
            (
                EdgeId::fixture("modifier-state"),
                StateConnectionTarget::StateConnection,
                Some(node_modifier.clone()),
                true,
            ),
            (
                EdgeId::fixture("modifier-formula-disabled"),
                StateConnectionTarget::Formula,
                Some(resource_expression.clone()),
                false,
            ),
        ] {
            let connection = EdgeConnectionConfig {
                kind: ConnectionKind::State,
                state: StateConnectionConfig {
                    role: StateConnectionRole::Modifier,
                    formula: "+1".into(),
                    target,
                    target_connection,
                    resource_filter: None,
                },
                ..Default::default()
            };
            let mut edge = EdgeSpec::new(
                id,
                source.clone(),
                sink.clone(),
                TransferSpec::Expression { formula: "not parsed".into() },
            )
            .with_connection(connection);
            edge.enabled = enabled;
            spec = spec.with_edge(edge);
        }

        // These role-specific strings are control data, not expression-language input.
        for (id, role, resource_filter) in [
            ("trigger", StateConnectionRole::Trigger, None),
            ("activator", StateConnectionRole::Activator, None),
            ("filter", StateConnectionRole::Filter, Some("any")),
        ] {
            let connection = EdgeConnectionConfig {
                kind: ConnectionKind::State,
                state: StateConnectionConfig {
                    role,
                    formula: "not parsed".into(),
                    target: StateConnectionTarget::Node,
                    target_connection: None,
                    resource_filter: resource_filter.map(str::to_owned),
                },
                ..Default::default()
            };
            spec = spec.with_edge(
                EdgeSpec::new(
                    EdgeId::fixture(id),
                    source.clone(),
                    sink.clone(),
                    TransferSpec::Expression { formula: "not parsed".into() },
                )
                .with_connection(connection),
            );
        }
        let checked =
            Scenario::try_from(spec).expect("included formulas compile while excluded ones do not");
        assert!(checked.expressions.transfer.contains_key(&resource_expression));
        for id in
            ["modifier-node", "modifier-resource", "modifier-state", "modifier-formula-disabled"]
        {
            let id = EdgeId::fixture(id);
            assert!(checked.expressions.state.contains_key(&id));
            assert!(!checked.expressions.transfer.contains_key(&id));
        }
        for id in ["trigger", "activator", "filter"] {
            let id = EdgeId::fixture(id);
            assert!(!checked.expressions.transfer.contains_key(&id));
            assert!(!checked.expressions.state.contains_key(&id));
        }

        let mut invalid =
            ScenarioSpec::source_sink(TransferSpec::Expression { formula: "!".into() });
        let error =
            Scenario::try_from(invalid.clone()).expect_err("invalid resource expression fails");
        assert!(error.to_string().contains("edges.edge-source-sink.transfer.expression.formula"));
        invalid.edges.get_mut(&EdgeId::fixture("edge-source-sink")).unwrap().transfer =
            TransferSpec::Fixed { amount: 1.0 };
        Scenario::try_from(invalid).expect("replacement valid transfer checks");
    }
}
