//! Immutable, semantically checked scenario values.
//!
//! This module deliberately has no serde derives.  [`ScenarioSpec`] remains the
//! stable document contract; a [`Scenario`] is the boundary after validation.

use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use crate::expr::CompiledExpr;

use super::{
    EdgeId, NodeId, NodeModeConfig, ScenarioId, ScenarioSpec, StateConnectionRole, TransferSpec,
};

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
            #[must_use]
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
    #[must_use]
    pub fn with_capacity(mut self, capacity: u64) -> Self {
        self.capacity = Some(capacity);
        self
    }
    #[must_use]
    pub fn without_capacity(mut self) -> Self {
        self.capacity = None;
        self
    }
    #[must_use]
    pub fn with_allow_negative_start(mut self, value: bool) -> Self {
        self.allow_negative_start = value;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_ignore_disabled_inputs(mut self, v: bool) -> Self {
        self.ignore_disabled_inputs = v;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_ignore_disabled_inputs(mut self, v: bool) -> Self {
        self.ignore_disabled_inputs = v;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_interactive(mut self, v: bool) -> Self {
        self.interactive = v;
        self
    }
    #[must_use]
    pub fn with_min_value(mut self, v: i64) -> Self {
        self.min_value = Some(v);
        self
    }
    #[must_use]
    pub fn without_min_value(mut self) -> Self {
        self.min_value = None;
        self
    }
    #[must_use]
    pub fn with_max_value(mut self, v: i64) -> Self {
        self.max_value = Some(v);
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_delay_steps(mut self, v: NonZeroU64) -> Self {
        self.delay_steps = v;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_capacity(mut self, v: NonZeroU64) -> Self {
        self.capacity = Some(v);
        self
    }
    #[must_use]
    pub fn without_capacity(mut self) -> Self {
        self.capacity = None;
        self
    }
    #[must_use]
    pub fn with_release_per_step(mut self, v: NonZeroU64) -> Self {
        self.release_per_step = v;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_label(mut self, v: impl Into<String>) -> Self {
        self.label = Some(v.into());
        self
    }
    #[must_use]
    pub fn with_initial_value(mut self, v: f64) -> Self {
        self.initial_value = v;
        self
    }
    #[must_use]
    pub fn with_tag(mut self, v: impl Into<String>) -> Self {
        self.tags.insert(v.into());
        self
    }
    #[must_use]
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
    #[must_use]
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
    #[must_use]
    pub fn with_role(mut self, v: StateConnectionRole) -> Self {
        self.role = v;
        self
    }
    #[must_use]
    pub fn with_formula(mut self, v: impl Into<String>) -> Self {
        self.formula = v.into();
        self
    }
    #[must_use]
    pub fn with_target(mut self, v: StateTarget) -> Self {
        self.target = v;
        self
    }
    #[must_use]
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
    #[must_use]
    pub fn with_enabled(mut self, v: bool) -> Self {
        self.enabled = v;
        self
    }
    #[must_use]
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
            let mut connection = EdgeConnectionConfig::default();
            connection.kind = ConnectionKind::State;
            connection.state = StateConnectionConfig {
                role: StateConnectionRole::Modifier,
                formula: "+1".into(),
                target,
                target_connection,
                resource_filter: None,
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
            let mut connection = EdgeConnectionConfig::default();
            connection.kind = ConnectionKind::State;
            connection.state = StateConnectionConfig {
                role: StateConnectionRole::Modifier,
                formula: "+1".into(),
                target: StateConnectionTarget::Node,
                target_connection,
                resource_filter: None,
            };
            connection.state.target = target;
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
            let mut connection = EdgeConnectionConfig::default();
            connection.kind = ConnectionKind::State;
            connection.state = StateConnectionConfig {
                role,
                formula: "not parsed".into(),
                target: StateConnectionTarget::Node,
                target_connection: None,
                resource_filter: resource_filter.map(str::to_owned),
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
