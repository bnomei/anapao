//! Declarative scenario graph: nodes, edges, transfers, variables, and end conditions.
//!
//! [`ScenarioSpec`] is the stable serde document surface compiled by
//! [`crate::Simulator::compile`]. To reject inconsistent tag/payload combinations before
//! execution, convert it with [`super::Scenario::try_from`]. Programmatic checked authoring uses
//! [`super::ScenarioBuilder`]; checked values deliberately do not define a second serde format.
//! Node families, connection kinds, transfer rules, and end conditions encode the model the
//! engine executes. Legacy node-kind aliases remain for older fixtures.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::{EdgeId, MetricKey, NodeId, ScenarioId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Node family selecting runtime behavior and valid config shapes.
pub enum NodeKind {
    Source,
    Pool,
    Drain,
    SortingGate,
    TriggerGate,
    MixedGate,
    Converter,
    Trader,
    Register,
    Delay,
    Queue,
    // Legacy compatibility aliases kept for older persisted sessions and tests.
    Process,
    Sink,
    Gate,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// When a gate-like node evaluates during a step.
pub enum TriggerMode {
    #[serde(alias = "passive")]
    Passive,
    #[serde(alias = "interactive")]
    Interactive,
    #[serde(alias = "automatic")]
    #[default]
    Automatic,
    #[serde(alias = "enabling")]
    Enabling,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// How a gate-like node pulls or pushes resource flow.
pub enum ActionMode {
    #[serde(alias = "push-any")]
    #[default]
    PushAny,
    #[serde(alias = "push-all")]
    PushAll,
    #[serde(alias = "pull-any")]
    PullAny,
    #[serde(alias = "pull-all")]
    PullAll,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Shared trigger/action mode settings for configurable nodes.
pub struct NodeModeConfig {
    #[serde(default)]
    pub trigger_mode: TriggerMode,
    #[serde(default)]
    pub action_mode: ActionMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `pool` nodes.
pub struct PoolNodeConfig {
    pub capacity: Option<u64>,
    #[serde(default)]
    pub allow_negative_start: bool,
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `drain` nodes.
pub struct DrainNodeConfig {
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `sorting_gate` nodes.
pub struct SortingGateNodeConfig {
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `trigger_gate` nodes.
pub struct TriggerGateNodeConfig {
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `mixed_gate` nodes.
pub struct MixedGateNodeConfig {
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `converter` nodes.
pub struct ConverterNodeConfig {
    #[serde(default)]
    pub ignore_disabled_inputs: bool,
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `trader` nodes.
pub struct TraderNodeConfig {
    #[serde(default)]
    pub ignore_disabled_inputs: bool,
    #[serde(default)]
    pub mode: NodeModeConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Configuration for `register` nodes.
pub struct RegisterNodeConfig {
    #[serde(default)]
    pub interactive: bool,
    pub min_value: Option<i64>,
    pub max_value: Option<i64>,
}

fn default_delay_steps() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Configuration for `delay` nodes.
pub struct DelayNodeConfig {
    #[serde(default = "default_delay_steps")]
    pub delay_steps: u64,
    #[serde(default)]
    pub mode: NodeModeConfig,
}

impl Default for DelayNodeConfig {
    fn default() -> Self {
        Self { delay_steps: default_delay_steps(), mode: NodeModeConfig::default() }
    }
}

fn default_queue_release_per_step() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Configuration for `queue` nodes.
pub struct QueueNodeConfig {
    pub capacity: Option<u64>,
    #[serde(default = "default_queue_release_per_step")]
    pub release_per_step: u64,
    #[serde(default)]
    pub mode: NodeModeConfig,
}

impl Default for QueueNodeConfig {
    fn default() -> Self {
        Self {
            capacity: None,
            release_per_step: default_queue_release_per_step(),
            mode: NodeModeConfig::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "family", content = "config", rename_all = "snake_case")]
/// Typed node-configuration envelope keyed by node family.
pub enum NodeConfig {
    #[default]
    None,
    Pool(PoolNodeConfig),
    Drain(DrainNodeConfig),
    SortingGate(SortingGateNodeConfig),
    TriggerGate(TriggerGateNodeConfig),
    MixedGate(MixedGateNodeConfig),
    Converter(ConverterNodeConfig),
    Trader(TraderNodeConfig),
    Register(RegisterNodeConfig),
    Delay(DelayNodeConfig),
    Queue(QueueNodeConfig),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// How much resource an edge attempts to move on each evaluation.
pub enum TransferSpec {
    Fixed { amount: f64 },
    Fraction { numerator: u64, denominator: u64 },
    Remaining,
    MetricScaled { metric: MetricKey, factor: f64 },
    Expression { formula: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// When a run stops early or after a step budget (AND/OR composable).
pub enum EndConditionSpec {
    MaxSteps { steps: u64 },
    MetricAtLeast { metric: MetricKey, value_scaled: i64 },
    MetricAtMost { metric: MetricKey, value_scaled: i64 },
    NodeAtLeast { node_id: NodeId, value_scaled: i64 },
    NodeAtMost { node_id: NodeId, value_scaled: i64 },
    Any(Vec<EndConditionSpec>),
    All(Vec<EndConditionSpec>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// When scenario variables are re-sampled relative to the run lifecycle.
pub enum VariableUpdateTiming {
    #[default]
    EveryStep,
    RunStart,
    Never,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Stochastic or constant sampler backing one named runtime variable.
pub enum VariableSourceSpec {
    Constant { value: f64 },
    RandomInterval { min: i64, max: i64 },
    RandomList { values: Vec<f64> },
    RandomMatrix { values: Vec<Vec<f64>> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
/// Runtime variable configuration attached to a scenario.
pub struct VariableRuntimeConfig {
    #[serde(default)]
    pub update_timing: VariableUpdateTiming,
    #[serde(default)]
    pub sources: BTreeMap<String, VariableSourceSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Resource flow edge versus state/modifier edge semantics.
pub enum ConnectionKind {
    #[default]
    Resource,
    State,
}

fn default_resource_token_size() -> u64 {
    1
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Resource-connection properties for tokenized flow.
pub struct ResourceConnectionConfig {
    #[serde(default = "default_resource_token_size")]
    pub token_size: u64,
}

impl Default for ResourceConnectionConfig {
    fn default() -> Self {
        Self { token_size: default_resource_token_size() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// State-connection semantic role.
pub enum StateConnectionRole {
    Activator,
    Trigger,
    #[default]
    Modifier,
    Filter,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Target domain for a state-connection formula.
pub enum StateConnectionTarget {
    #[default]
    Node,
    ResourceConnection,
    StateConnection,
    Formula,
}

fn default_state_connection_formula() -> String {
    "+1".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// State-connection behavior and targeting options.
pub struct StateConnectionConfig {
    #[serde(default)]
    pub role: StateConnectionRole,
    #[serde(default = "default_state_connection_formula")]
    pub formula: String,
    #[serde(default)]
    pub target: StateConnectionTarget,
    #[serde(default, alias = "target_edge")]
    pub target_connection: Option<EdgeId>,
    #[serde(default, alias = "filter")]
    pub resource_filter: Option<String>,
}

impl Default for StateConnectionConfig {
    fn default() -> Self {
        Self {
            role: StateConnectionRole::default(),
            formula: default_state_connection_formula(),
            target: StateConnectionTarget::default(),
            target_connection: None,
            resource_filter: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Complete edge-connection configuration.
pub struct EdgeConnectionConfig {
    #[serde(default, alias = "connection_kind")]
    pub kind: ConnectionKind,
    #[serde(default)]
    pub resource: ResourceConnectionConfig,
    #[serde(default)]
    pub state: StateConnectionConfig,
}

impl EdgeConnectionConfig {
    /// Returns true when the connection is equivalent to the default resource mode.
    pub fn is_default_resource(&self) -> bool {
        self == &Self::default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Declarative node definition in a scenario graph.
pub struct NodeSpec {
    pub id: NodeId,
    pub kind: NodeKind,
    #[serde(default)]
    pub config: NodeConfig,
    pub label: Option<String>,
    pub initial_value: f64,
    pub tags: BTreeSet<String>,
    pub metadata: BTreeMap<String, String>,
}

impl NodeSpec {
    /// Creates a node with default config and zero initial value.
    pub fn new(id: NodeId, kind: NodeKind) -> Self {
        Self {
            id,
            kind,
            config: NodeConfig::default(),
            label: None,
            initial_value: 0.0,
            tags: BTreeSet::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Sets the node's initial value.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_initial_value(mut self, initial_value: f64) -> Self {
        self.initial_value = initial_value;
        self
    }

    /// Replaces the node's family-specific config.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_config(mut self, config: NodeConfig) -> Self {
        self.config = config;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Declarative edge definition between two nodes.
pub struct EdgeSpec {
    pub id: EdgeId,
    pub from: NodeId,
    pub to: NodeId,
    pub transfer: TransferSpec,
    #[serde(default, skip_serializing_if = "EdgeConnectionConfig::is_default_resource")]
    pub connection: EdgeConnectionConfig,
    pub enabled: bool,
    pub metadata: BTreeMap<String, String>,
}

impl EdgeSpec {
    /// Creates an enabled edge with default resource connection metadata.
    pub fn new(id: EdgeId, from: NodeId, to: NodeId, transfer: TransferSpec) -> Self {
        Self {
            id,
            from,
            to,
            transfer,
            connection: EdgeConnectionConfig::default(),
            enabled: true,
            metadata: BTreeMap::new(),
        }
    }

    /// Replaces edge connection settings.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_connection(mut self, connection: EdgeConnectionConfig) -> Self {
        self.connection = connection;
        self
    }
}

/// Declarative simulation graph consumed by [`crate::Simulator::compile`].
///
/// Build with [`ScenarioSpec::new`] and add nodes/edges via builder helpers.
/// Defaults include:
/// - empty node/edge collections,
/// - empty tracked metrics,
/// - one fallback end condition (`MaxSteps { steps: 1 }`).
///
/// # Example
/// ```rust
/// use anapao::types::{
///     EdgeId, EdgeSpec, EndConditionSpec, MetricKey, NodeId, NodeKind, NodeSpec, ScenarioId,
///     ScenarioSpec, TransferSpec,
/// };
///
/// let source = NodeId::fixture("source");
/// let sink = NodeId::fixture("sink");
///
/// let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-doc"))
///     .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
///     .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
///     .with_edge(EdgeSpec::new(
///         EdgeId::fixture("edge-source-sink"),
///         source,
///         sink,
///         TransferSpec::Fixed { amount: 1.0 },
///     ));
/// scenario = scenario.with_end_condition(EndConditionSpec::MaxSteps { steps: 3 });
/// scenario.tracked_metrics.insert(MetricKey::fixture("sink"));
///
/// assert_eq!(scenario.nodes.len(), 2);
/// assert_eq!(scenario.edges.len(), 1);
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScenarioSpec {
    pub id: ScenarioId,
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: BTreeSet<String>,
    pub nodes: BTreeMap<NodeId, NodeSpec>,
    pub edges: BTreeMap<EdgeId, EdgeSpec>,
    #[serde(default)]
    pub variables: VariableRuntimeConfig,
    pub end_conditions: Vec<EndConditionSpec>,
    pub tracked_metrics: BTreeSet<MetricKey>,
    pub metadata: BTreeMap<String, String>,
}

impl ScenarioSpec {
    /// Creates an empty scenario with a default `MaxSteps(1)` end condition.
    pub fn new(id: ScenarioId) -> Self {
        Self {
            id,
            title: None,
            description: None,
            tags: BTreeSet::new(),
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            variables: VariableRuntimeConfig::default(),
            end_conditions: vec![EndConditionSpec::MaxSteps { steps: 1 }],
            tracked_metrics: BTreeSet::new(),
            metadata: BTreeMap::new(),
        }
    }

    /// Builds a minimal source-to-sink scenario with one edge.
    ///
    /// The generated ids are stable (`scenario-source-sink`, `source`, `sink`, `edge-source-sink`)
    /// so callers can customize fields (for example `id`, `end_conditions`) after construction.
    pub fn source_sink(transfer: TransferSpec) -> Self {
        let source = NodeId::fixture("source");
        let sink = NodeId::fixture("sink");

        let mut scenario = Self::new(ScenarioId::fixture("scenario-source-sink"))
            .with_node(NodeSpec::new(source.clone(), NodeKind::Source).with_initial_value(1.0))
            .with_node(NodeSpec::new(sink.clone(), NodeKind::Sink))
            .with_edge(EdgeSpec::new(EdgeId::fixture("edge-source-sink"), source, sink, transfer));
        scenario.tracked_metrics.insert(MetricKey::fixture("sink"));
        scenario
    }

    /// Builds a linear source -> stage* -> sink pipeline with fixed unit transfers.
    ///
    /// `node_count` is clamped to at least two nodes (`source` and `sink`).
    /// Intermediate nodes are named `stage-1`, `stage-2`, ... and use [`NodeKind::Pool`].
    pub fn linear_pipeline(node_count: usize) -> Self {
        let node_count = node_count.max(2);
        let mut scenario = Self::new(ScenarioId::fixture("scenario-linear-pipeline"));
        let mut node_ids = Vec::with_capacity(node_count);

        for index in 0..node_count {
            let (id, kind) = match index {
                0 => ("source".to_string(), NodeKind::Source),
                last if last == node_count - 1 => ("sink".to_string(), NodeKind::Sink),
                _ => (format!("stage-{index}"), NodeKind::Pool),
            };

            let node_id = NodeId::fixture(id);
            let node = if index == 0 {
                NodeSpec::new(node_id.clone(), kind).with_initial_value(1.0)
            } else {
                NodeSpec::new(node_id.clone(), kind)
            };

            node_ids.push(node_id);
            scenario = scenario.with_node(node);
        }

        for edge_index in 0..(node_ids.len() - 1) {
            let from = node_ids[edge_index].clone();
            let to = node_ids[edge_index + 1].clone();
            let edge = EdgeSpec::new(
                EdgeId::fixture(format!("edge-{edge_index}")),
                from,
                to,
                TransferSpec::Fixed { amount: 1.0 },
            );
            scenario = scenario.with_edge(edge);
        }

        scenario.tracked_metrics.insert(MetricKey::fixture("sink"));
        scenario
    }

    /// Inserts or replaces a node by id.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_node(mut self, node: NodeSpec) -> Self {
        self.nodes.insert(node.id.clone(), node);
        self
    }

    /// Inserts or replaces an edge by id.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_edge(mut self, edge: EdgeSpec) -> Self {
        self.edges.insert(edge.id.clone(), edge);
        self
    }

    /// Replaces all end conditions with one condition.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_end_condition(mut self, condition: EndConditionSpec) -> Self {
        self.end_conditions = vec![condition];
        self
    }

    /// Replaces all end conditions with a provided ordered list.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_end_conditions<I>(mut self, conditions: I) -> Self
    where
        I: IntoIterator<Item = EndConditionSpec>,
    {
        self.end_conditions = conditions.into_iter().collect();
        self
    }

    /// Appends an additional end condition to the existing list.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn push_end_condition(mut self, condition: EndConditionSpec) -> Self {
        self.end_conditions.push(condition);
        self
    }

    /// Sets the optional human-readable scenario title.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the optional human-readable scenario description.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a scenario tag.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.insert(tag.into());
        self
    }

    /// Replaces the runtime variable configuration.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_variables(mut self, variables: VariableRuntimeConfig) -> Self {
        self.variables = variables;
        self
    }

    /// Adds one terminal metric to retain in scenario reports.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_tracked_metric(mut self, metric: MetricKey) -> Self {
        self.tracked_metrics.insert(metric);
        self
    }

    /// Adds or replaces one string metadata entry.
    #[must_use = "scenario authoring methods return a new value; use the result"]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}
