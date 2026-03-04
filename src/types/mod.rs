//! Core domain types for scenario definitions, execution reports, and artifacts.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
/// Validation errors for strongly-typed identifier wrappers.
pub enum IdentifierError {
    #[error("{kind} cannot be empty")]
    Empty { kind: &'static str },
    #[error("{kind} cannot contain control characters")]
    ContainsControl { kind: &'static str },
}

macro_rules! define_identifier {
    ($name:ident, $kind:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        #[doc = concat!("Validated ", $kind, " wrapper used in persisted/session-facing APIs.")]
        pub struct $name(String);

        impl $name {
            #[doc = concat!("Creates a validated ", $kind, ".")]
            pub fn new(value: impl Into<String>) -> Result<Self, IdentifierError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(IdentifierError::Empty { kind: $kind });
                }
                if value.chars().any(char::is_control) {
                    return Err(IdentifierError::ContainsControl { kind: $kind });
                }
                Ok(Self(value))
            }

            #[doc = concat!("Creates a fixture ", $kind, " and panics if invalid.")]
            pub fn fixture(value: impl Into<String>) -> Self {
                Self::new(value).expect(concat!("invalid ", $kind, " fixture identifier"))
            }

            #[doc = concat!("Returns the ", $kind, " as `&str`.")]
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl TryFrom<&str> for $name {
            type Error = IdentifierError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentifierError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_identifier!(ScenarioId, "scenario id");
define_identifier!(NodeId, "node id");
define_identifier!(EdgeId, "edge id");
define_identifier!(MetricKey, "metric key");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Supported node families in simulation graphs.
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
/// Trigger policy for gate-like nodes.
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
/// Action policy for gate-like nodes.
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
/// Transfer rule used by edges when moving resource values.
pub enum TransferSpec {
    Fixed { amount: f64 },
    Fraction { numerator: u64, denominator: u64 },
    Remaining,
    MetricScaled { metric: MetricKey, factor: f64 },
    Expression { formula: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Stopping conditions for a simulation run.
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
/// Update schedule for runtime variables.
pub enum VariableUpdateTiming {
    #[default]
    EveryStep,
    RunStart,
    Never,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
/// Value source used for runtime variables.
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
/// Connection family for an edge.
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
    pub fn with_initial_value(mut self, initial_value: f64) -> Self {
        self.initial_value = initial_value;
        self
    }

    /// Replaces the node's family-specific config.
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
/// scenario.end_conditions = vec![EndConditionSpec::MaxSteps { steps: 3 }];
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

    /// Inserts or replaces a node by id.
    pub fn with_node(mut self, node: NodeSpec) -> Self {
        self.nodes.insert(node.id.clone(), node);
        self
    }

    /// Inserts or replaces an edge by id.
    pub fn with_edge(mut self, edge: EdgeSpec) -> Self {
        self.edges.insert(edge.id.clone(), edge);
        self
    }

    /// Appends an additional end condition.
    pub fn with_end_condition(mut self, condition: EndConditionSpec) -> Self {
        self.end_conditions.push(condition);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Execution strategy for batch runs.
pub enum ExecutionMode {
    #[default]
    SingleThread,
    Rayon,
}

/// Capture policy for per-step node and metric snapshots.
///
/// Use [`CaptureConfig::default`] for debugging/analysis-friendly traces
/// or [`CaptureConfig::disabled`] for throughput-oriented runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureConfig {
    pub capture_nodes: BTreeSet<NodeId>,
    pub capture_metrics: BTreeSet<MetricKey>,
    pub every_n_steps: u64,
    pub include_step_zero: bool,
    pub include_final_state: bool,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            capture_nodes: BTreeSet::new(),
            capture_metrics: BTreeSet::new(),
            every_n_steps: 1,
            include_step_zero: true,
            include_final_state: true,
        }
    }
}

impl CaptureConfig {
    /// Disables step-zero/final captures while preserving explicit selection sets.
    pub fn disabled() -> Self {
        Self { include_step_zero: false, include_final_state: false, ..Self::default() }
    }
}

/// Deterministic controls for one simulation run.
///
/// # Example
/// ```rust
/// use anapao::types::{CaptureConfig, RunConfig};
///
/// let run = RunConfig::for_seed(42).with_max_steps(250).with_capture(CaptureConfig {
///     every_n_steps: 5,
///     include_step_zero: false,
///     ..CaptureConfig::default()
/// });
///
/// assert_eq!(run.seed, 42);
/// assert_eq!(run.max_steps, 250);
/// assert_eq!(run.capture.every_n_steps, 5);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunConfig {
    pub seed: u64,
    pub max_steps: u64,
    pub capture: CaptureConfig,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self { seed: 0, max_steps: 100, capture: CaptureConfig::default() }
    }
}

impl RunConfig {
    /// Creates a run config from a seed with default limits/capture policy.
    pub fn for_seed(seed: u64) -> Self {
        Self { seed, ..Self::default() }
    }

    /// Sets the run step limit.
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Replaces capture settings for the run.
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.capture = capture;
        self
    }
}

/// Deterministic Monte Carlo controls for many runs.
///
/// The `base_seed` is used with run index derivation to produce stable per-run seeds.
///
/// # Example
/// ```rust
/// use anapao::types::{BatchConfig, CaptureConfig, ExecutionMode, RunConfig};
///
/// let batch = BatchConfig::for_runs(128)
///     .with_execution_mode(ExecutionMode::SingleThread)
///     .with_run(RunConfig::for_seed(999))
///     .with_max_steps(50)
///     .with_capture(CaptureConfig::disabled());
///
/// assert_eq!(batch.runs, 128);
/// assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
/// assert_eq!(batch.run.seed, 999);
/// assert_eq!(batch.run.max_steps, 50);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchConfig {
    pub runs: u64,
    pub base_seed: u64,
    pub execution_mode: ExecutionMode,
    pub run: RunConfig,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            runs: 1,
            base_seed: 0,
            execution_mode: ExecutionMode::default(),
            run: RunConfig::default(),
        }
    }
}

impl BatchConfig {
    /// Creates a batch config from a requested run count and default options.
    pub fn for_runs(runs: u64) -> Self {
        Self { runs, ..Self::default() }
    }

    /// Sets execution mode for the batch.
    pub fn with_execution_mode(mut self, execution_mode: ExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    /// Replaces the default run template used for each batch run.
    pub fn with_run(mut self, run: RunConfig) -> Self {
        self.run = run;
        self
    }

    /// Sets max steps on the batch run template.
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.run.max_steps = max_steps;
        self
    }

    /// Replaces capture settings on the batch run template.
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.run.capture = capture;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Artifact type labels used in manifests.
pub enum ArtifactKind {
    Manifest,
    EventLog,
    VariableSeries,
    AssertionReport,
    HistoryIndex,
    ReplayIndex,
    Series,
    Summary,
    Prediction,
    Custom(String),
}

/// Legacy artifact schema version used by persisted compatibility readers.
pub const ARTIFACT_SCHEMA_VERSION_V1: u32 = 1;
/// Current artifact schema version for newly-written manifests.
pub const ARTIFACT_SCHEMA_VERSION_V2: u32 = 2;

fn default_artifact_schema_version() -> u32 {
    ARTIFACT_SCHEMA_VERSION_V1
}

fn default_manifest_setup_hash() -> String {
    "unknown".to_string()
}

fn default_manifest_seed_strategy() -> String {
    "unknown".to_string()
}

fn default_manifest_crate_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn default_manifest_generated_at_unix_seconds() -> u64 {
    0
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Manifest entry referencing one concrete artifact file.
pub struct ArtifactRef {
    pub kind: ArtifactKind,
    pub path: String,
    pub content_type: Option<String>,
}

impl ArtifactRef {
    /// Creates an artifact reference from kind and relative path.
    pub fn new(kind: ArtifactKind, path: impl Into<String>) -> Self {
        Self { kind, path: path.into(), content_type: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
/// Section pointer into a named artifact in the manifest.
pub struct ArtifactSectionRef {
    pub artifact_key: String,
    pub kind: ArtifactKind,
}

impl ArtifactSectionRef {
    /// Creates a section pointer.
    pub fn new(artifact_key: impl Into<String>, kind: ArtifactKind) -> Self {
        Self { artifact_key: artifact_key.into(), kind }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
/// Optional manifest section index for downstream tooling.
pub struct ArtifactSchemaSections {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assertions: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<ArtifactSectionRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replay: Option<ArtifactSectionRef>,
}

impl ArtifactSchemaSections {
    /// Returns true when no sections are currently linked.
    pub fn is_empty(&self) -> bool {
        self.accuracy.is_none()
            && self.assertions.is_none()
            && self.debug.is_none()
            && self.history.is_none()
            && self.replay.is_none()
    }

    /// Heuristically infers canonical section links from known artifact kinds.
    pub fn infer_from_artifacts(artifacts: &BTreeMap<String, ArtifactRef>) -> Self {
        let accuracy = find_section(artifacts, &[ArtifactKind::Prediction, ArtifactKind::Summary]);
        let assertions = find_section(artifacts, &[ArtifactKind::AssertionReport]);
        let debug = find_section(artifacts, &[ArtifactKind::EventLog]);
        let history = find_section(artifacts, &[ArtifactKind::HistoryIndex]);
        let replay = find_section(artifacts, &[ArtifactKind::ReplayIndex]);

        Self { accuracy, assertions, debug, history, replay }
    }
}

fn find_section(
    artifacts: &BTreeMap<String, ArtifactRef>,
    preferred_kinds: &[ArtifactKind],
) -> Option<ArtifactSectionRef> {
    for preferred_kind in preferred_kinds {
        for (key, artifact) in artifacts {
            if &artifact.kind == preferred_kind {
                return Some(ArtifactSectionRef::new(key.clone(), artifact.kind.clone()));
            }
        }
    }

    None
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Prediction diagnostics computed from batch metric samples.
pub struct PredictionMetricIndicators {
    pub n: usize,
    pub mean: f64,
    pub variance: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub median: f64,
    pub p90: f64,
    pub p95: f64,
    pub p99: f64,
    pub confidence_lower_95: f64,
    pub confidence_upper_95: f64,
    pub confidence_margin_95: f64,
    pub reliability_score: f64,
    pub convergence_delta: f64,
    pub convergence_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Aggregated prediction diagnostics for one scenario.
pub struct PredictionSummaryReport {
    pub scenario_id: ScenarioId,
    pub metrics: BTreeMap<MetricKey, PredictionMetricIndicators>,
}

impl PredictionSummaryReport {
    /// Creates a prediction summary report.
    pub fn new(
        scenario_id: ScenarioId,
        metrics: BTreeMap<MetricKey, PredictionMetricIndicators>,
    ) -> Self {
        Self { scenario_id, metrics }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
/// Diagnostic severity levels used by violation/debug artifacts.
pub enum DiagnosticSeverity {
    Debug,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Flattened event index entry optimized for history browsing.
pub struct HistoryIndexEntry {
    pub event_index: u64,
    pub run_id: String,
    pub step: u64,
    pub phase: String,
    pub ordinal: u64,
    pub event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<DiagnosticSeverity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Indexed event history for one scenario source.
pub struct HistoryIndexReport {
    pub scenario_id: ScenarioId,
    pub source: String,
    pub entries: Vec<HistoryIndexEntry>,
}

impl HistoryIndexReport {
    /// Creates an empty history index report.
    pub fn new(scenario_id: ScenarioId, source: impl Into<String>) -> Self {
        Self { scenario_id, source: source.into(), entries: Vec::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Replay span metadata for a single run id.
pub struct ReplayRunIndex {
    pub first_event_index: u64,
    pub last_event_index: u64,
    pub first_step: u64,
    pub last_step: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// Replay index grouped by run id.
pub struct ReplayIndexReport {
    pub scenario_id: ScenarioId,
    pub source: String,
    pub event_count: u64,
    pub runs: BTreeMap<String, ReplayRunIndex>,
}

impl ReplayIndexReport {
    /// Creates an empty replay index with known event count.
    pub fn new(scenario_id: ScenarioId, source: impl Into<String>, event_count: u64) -> Self {
        Self { scenario_id, source: source.into(), event_count, runs: BTreeMap::new() }
    }
}

/// Stable manifest for run/batch artifacts.
///
/// `ManifestRef` is intended as the machine-readable index for CI or downstream tooling.
/// It binds scenario identity, seed strategy metadata, and typed artifact references.
///
/// # Example
/// ```rust
/// use anapao::types::{ArtifactKind, ArtifactRef, ManifestRef, ScenarioId};
///
/// let manifest = ManifestRef::new(ScenarioId::fixture("scenario-doc"))
///     .with_seed_strategy("single(seed=42)")
///     .with_generated_at_unix_seconds(42)
///     .with_artifact(
///         "events",
///         ArtifactRef::new(ArtifactKind::EventLog, "events.jsonl"),
///     )
///     .with_inferred_sections();
///
/// assert!(manifest.artifacts.contains_key("events"));
/// assert_eq!(manifest.seed_strategy, "single(seed=42)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestRef {
    #[serde(default = "default_artifact_schema_version")]
    pub schema_version: u32,
    #[serde(default = "default_manifest_setup_hash")]
    pub setup_hash: String,
    #[serde(default = "default_manifest_seed_strategy")]
    pub seed_strategy: String,
    #[serde(default = "default_manifest_crate_version")]
    pub crate_version: String,
    #[serde(default = "default_manifest_generated_at_unix_seconds")]
    pub generated_at_unix_seconds: u64,
    pub scenario_id: ScenarioId,
    pub artifacts: BTreeMap<String, ArtifactRef>,
    #[serde(default, skip_serializing_if = "ArtifactSchemaSections::is_empty")]
    pub sections: ArtifactSchemaSections,
}

impl ManifestRef {
    /// Creates a manifest with current schema defaults.
    pub fn new(scenario_id: ScenarioId) -> Self {
        Self {
            schema_version: ARTIFACT_SCHEMA_VERSION_V2,
            setup_hash: default_manifest_setup_hash(),
            seed_strategy: default_manifest_seed_strategy(),
            crate_version: default_manifest_crate_version(),
            generated_at_unix_seconds: default_manifest_generated_at_unix_seconds(),
            scenario_id,
            artifacts: BTreeMap::new(),
            sections: ArtifactSchemaSections::default(),
        }
    }

    /// Adds or replaces a named artifact reference.
    pub fn with_artifact(mut self, key: impl Into<String>, artifact: ArtifactRef) -> Self {
        self.artifacts.insert(key.into(), artifact);
        self
    }

    /// Replaces the optional section mapping.
    pub fn with_sections(mut self, sections: ArtifactSchemaSections) -> Self {
        self.sections = sections;
        self
    }

    /// Infers and sets sections from current artifact map.
    pub fn with_inferred_sections(mut self) -> Self {
        self.sections = ArtifactSchemaSections::infer_from_artifacts(&self.artifacts);
        self
    }

    /// Sets setup-hash metadata.
    pub fn with_setup_hash(mut self, setup_hash: impl Into<String>) -> Self {
        self.setup_hash = setup_hash.into();
        self
    }

    /// Sets seed-strategy metadata.
    pub fn with_seed_strategy(mut self, seed_strategy: impl Into<String>) -> Self {
        self.seed_strategy = seed_strategy.into();
        self
    }

    /// Sets generation timestamp metadata.
    pub fn with_generated_at_unix_seconds(mut self, generated_at_unix_seconds: u64) -> Self {
        self.generated_at_unix_seconds = generated_at_unix_seconds;
        self
    }

    /// Upgrades older manifest payloads to current compatibility defaults.
    pub fn upgrade_compat(mut self) -> Self {
        if self.schema_version < ARTIFACT_SCHEMA_VERSION_V2 {
            self.schema_version = ARTIFACT_SCHEMA_VERSION_V2;
        }
        if self.setup_hash.is_empty() {
            self.setup_hash = default_manifest_setup_hash();
        }
        if self.seed_strategy.is_empty() {
            self.seed_strategy = default_manifest_seed_strategy();
        }
        if self.crate_version.is_empty() {
            self.crate_version = default_manifest_crate_version();
        }
        if self.sections.is_empty() {
            self.sections = ArtifactSchemaSections::infer_from_artifacts(&self.artifacts);
        }
        self
    }
}

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
    pub requested_runs: u64,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_validate_empty_and_control_values() {
        let empty = ScenarioId::new("   ");
        assert!(matches!(empty, Err(IdentifierError::Empty { .. })));

        let contains_control = NodeId::new("a\nb");
        assert!(matches!(contains_control, Err(IdentifierError::ContainsControl { .. })));
    }

    #[test]
    fn node_kind_supports_new_and_legacy_families() {
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"pool\"").expect("pool kind should deserialize"),
            NodeKind::Pool
        );
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"trigger_gate\"")
                .expect("trigger_gate kind should deserialize"),
            NodeKind::TriggerGate
        );
        assert_eq!(
            serde_json::from_str::<NodeKind>("\"process\"")
                .expect("legacy process kind should deserialize"),
            NodeKind::Process
        );
        assert_eq!(
            serde_json::to_string(&NodeKind::Queue).expect("queue kind should serialize"),
            "\"queue\""
        );
        assert_eq!(
            serde_json::to_string(&NodeKind::Gate).expect("legacy gate kind should serialize"),
            "\"gate\""
        );
    }

    #[test]
    fn node_mode_defaults_and_action_aliases_are_stable() {
        let defaults = NodeModeConfig::default();
        assert_eq!(defaults.trigger_mode, TriggerMode::Automatic);
        assert_eq!(defaults.action_mode, ActionMode::PushAny);

        let trigger: TriggerMode =
            serde_json::from_str("\"automatic\"").expect("automatic trigger mode should parse");
        assert_eq!(trigger, TriggerMode::Automatic);

        let action: ActionMode =
            serde_json::from_str("\"pull-any\"").expect("kebab-case alias should deserialize");
        assert_eq!(action, ActionMode::PullAny);

        let encoded = serde_json::to_string(&ActionMode::PushAll)
            .expect("action mode should serialize in snake_case");
        assert_eq!(encoded, "\"push_all\"");
    }

    #[test]
    fn node_spec_defaults_config_when_missing() {
        let encoded = r#"{
            "id":"pool-a",
            "kind":"pool",
            "label":null,
            "initial_value":0.0,
            "tags":[],
            "metadata":{}
        }"#;
        let node: NodeSpec =
            serde_json::from_str(encoded).expect("node spec should deserialize without config");
        assert_eq!(node.config, NodeConfig::None);
    }

    #[test]
    fn node_spec_round_trips_typed_config() {
        let node = NodeSpec::new(NodeId::fixture("pool-a"), NodeKind::Pool).with_config(
            NodeConfig::Pool(PoolNodeConfig {
                capacity: Some(10),
                allow_negative_start: true,
                mode: NodeModeConfig {
                    trigger_mode: TriggerMode::Passive,
                    action_mode: ActionMode::PullAll,
                },
            }),
        );

        let encoded = serde_json::to_string(&node).expect("node spec should serialize");
        let decoded: NodeSpec =
            serde_json::from_str(&encoded).expect("node spec should deserialize");

        assert_eq!(decoded, node);
    }

    #[test]
    fn edge_spec_defaults_resource_connection_when_missing() {
        let encoded = r#"{
            "id":"edge-a",
            "from":"source",
            "to":"sink",
            "transfer":"remaining",
            "enabled":true,
            "metadata":{}
        }"#;
        let edge: EdgeSpec =
            serde_json::from_str(encoded).expect("legacy edge spec should deserialize");

        assert_eq!(edge.connection.kind, ConnectionKind::Resource);
        assert_eq!(edge.connection.resource.token_size, 1);
        assert_eq!(edge.connection.state.formula, "+1");

        let reencoded = serde_json::to_value(&edge).expect("edge spec should serialize");
        assert!(
            reencoded.get("connection").is_none(),
            "default resource connection should preserve legacy payload shape"
        );
    }

    #[test]
    fn edge_spec_state_connection_defaults_formula_and_accepts_legacy_aliases() {
        let encoded = r#"{
            "id":"edge-state",
            "from":"source",
            "to":"sink",
            "transfer":"remaining",
            "enabled":true,
            "metadata":{},
            "connection":{
                "kind":"state",
                "state":{
                    "role":"modifier",
                    "target":"formula",
                    "target_edge":"edge-resource",
                    "filter":"gold"
                }
            }
        }"#;
        let edge: EdgeSpec =
            serde_json::from_str(encoded).expect("state edge spec should deserialize");

        assert_eq!(edge.connection.kind, ConnectionKind::State);
        assert_eq!(edge.connection.state.role, StateConnectionRole::Modifier);
        assert_eq!(edge.connection.state.formula, "+1");
        assert_eq!(edge.connection.state.target, StateConnectionTarget::Formula);
        assert_eq!(edge.connection.state.target_connection, Some(EdgeId::fixture("edge-resource")));
        assert_eq!(edge.connection.state.resource_filter.as_deref(), Some("gold"));
    }

    #[test]
    fn edge_spec_round_trips_explicit_connection_semantics() {
        let edge = EdgeSpec::new(
            EdgeId::fixture("edge-state"),
            NodeId::fixture("source"),
            NodeId::fixture("sink"),
            TransferSpec::Remaining,
        )
        .with_connection(EdgeConnectionConfig {
            kind: ConnectionKind::State,
            resource: ResourceConnectionConfig { token_size: 1 },
            state: StateConnectionConfig {
                role: StateConnectionRole::Activator,
                formula: "*".to_string(),
                target: StateConnectionTarget::ResourceConnection,
                target_connection: Some(EdgeId::fixture("edge-resource")),
                resource_filter: None,
            },
        });

        let encoded = serde_json::to_string(&edge).expect("edge spec should serialize");
        let decoded: EdgeSpec =
            serde_json::from_str(&encoded).expect("edge spec should deserialize");

        assert_eq!(decoded, edge);
    }

    #[test]
    fn transfer_spec_expression_round_trips_formula_payload() {
        let transfer = TransferSpec::Expression { formula: "max(1, step + 1)".to_string() };

        let encoded = serde_json::to_string(&transfer).expect("transfer spec should serialize");
        let decoded: TransferSpec =
            serde_json::from_str(&encoded).expect("transfer spec should deserialize");

        assert_eq!(decoded, transfer);
    }

    #[test]
    fn scenario_spec_defaults_variable_runtime_config() {
        let scenario = ScenarioSpec::new(ScenarioId::fixture("scenario"));
        assert_eq!(scenario.variables.update_timing, VariableUpdateTiming::EveryStep);
        assert!(scenario.variables.sources.is_empty());
    }

    #[test]
    fn variable_runtime_config_round_trips_typed_sources() {
        let mut sources = BTreeMap::new();
        sources.insert("roll".to_string(), VariableSourceSpec::RandomInterval { min: 1, max: 6 });
        sources.insert(
            "table".to_string(),
            VariableSourceSpec::RandomMatrix { values: vec![vec![1.0], vec![2.0, 3.0]] },
        );
        let config =
            VariableRuntimeConfig { update_timing: VariableUpdateTiming::RunStart, sources };

        let encoded =
            serde_json::to_string(&config).expect("variable runtime config should serialize");
        let decoded: VariableRuntimeConfig =
            serde_json::from_str(&encoded).expect("variable runtime config should deserialize");

        assert_eq!(decoded, config);
    }

    #[test]
    fn scenario_builder_keeps_deterministic_ordering() {
        let scenario = ScenarioSpec::new(ScenarioId::fixture("scenario"))
            .with_node(NodeSpec::new(NodeId::fixture("z-node"), NodeKind::Process))
            .with_node(NodeSpec::new(NodeId::fixture("a-node"), NodeKind::Source))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-z"),
                NodeId::fixture("z-node"),
                NodeId::fixture("a-node"),
                TransferSpec::Remaining,
            ))
            .with_edge(EdgeSpec::new(
                EdgeId::fixture("edge-a"),
                NodeId::fixture("a-node"),
                NodeId::fixture("z-node"),
                TransferSpec::Fixed { amount: 1.0 },
            ));

        let node_ids: Vec<&str> = scenario.nodes.keys().map(NodeId::as_str).collect();
        assert_eq!(node_ids, vec!["a-node", "z-node"]);

        let edge_ids: Vec<&str> = scenario.edges.keys().map(EdgeId::as_str).collect();
        assert_eq!(edge_ids, vec!["edge-a", "edge-z"]);
    }

    #[test]
    fn capture_and_run_defaults_are_stable() {
        let capture = CaptureConfig::default();
        assert_eq!(capture.every_n_steps, 1);
        assert!(capture.include_step_zero);
        assert!(capture.include_final_state);
        assert!(capture.capture_nodes.is_empty());
        assert!(capture.capture_metrics.is_empty());

        let run = RunConfig::for_seed(99);
        assert_eq!(run.seed, 99);
        assert_eq!(run.max_steps, 100);
        assert_eq!(run.capture, CaptureConfig::default());
    }

    #[test]
    fn run_config_builders_override_defaults() {
        let capture = CaptureConfig {
            every_n_steps: 5,
            include_step_zero: false,
            include_final_state: false,
            ..CaptureConfig::default()
        };
        let run = RunConfig::for_seed(77).with_max_steps(250).with_capture(capture.clone());

        assert_eq!(run.seed, 77);
        assert_eq!(run.max_steps, 250);
        assert_eq!(run.capture, capture);
    }

    #[test]
    fn batch_config_builders_update_execution_and_run_template() {
        let capture = CaptureConfig {
            every_n_steps: 3,
            include_step_zero: false,
            include_final_state: true,
            ..CaptureConfig::default()
        };
        let batch = BatchConfig::for_runs(12)
            .with_execution_mode(ExecutionMode::SingleThread)
            .with_run(RunConfig::for_seed(991))
            .with_max_steps(40)
            .with_capture(capture.clone());

        assert_eq!(batch.runs, 12);
        assert_eq!(batch.execution_mode, ExecutionMode::SingleThread);
        assert_eq!(batch.run.seed, 991);
        assert_eq!(batch.run.max_steps, 40);
        assert_eq!(batch.run.capture, capture);
    }

    #[test]
    fn series_and_batch_helpers_keep_stable_order() {
        let table = SeriesTable::new(MetricKey::fixture("throughput"))
            .with_point(SeriesPoint::new(8, 2.0))
            .with_point(SeriesPoint::new(2, 1.0))
            .with_point(SeriesPoint::new(5, 1.5));
        let steps: Vec<u64> = table.points.iter().map(|p| p.step).collect();
        assert_eq!(steps, vec![2, 5, 8]);

        let report =
            BatchReport::new(ScenarioId::fixture("scenario"), 2, ExecutionMode::SingleThread)
                .push_run(BatchRunSummary {
                    run_index: 9,
                    seed: 9,
                    completed: true,
                    steps_executed: 20,
                    final_metrics: BTreeMap::new(),
                    manifest: None,
                })
                .push_run(BatchRunSummary {
                    run_index: 1,
                    seed: 1,
                    completed: true,
                    steps_executed: 20,
                    final_metrics: BTreeMap::new(),
                    manifest: None,
                });

        let run_indexes: Vec<u64> = report.runs.iter().map(|run| run.run_index).collect();
        assert_eq!(run_indexes, vec![1, 9]);
        assert_eq!(report.completed_runs, 2);
    }

    #[test]
    fn manifest_refs_use_sorted_artifact_keys() {
        let manifest = ManifestRef::new(ScenarioId::fixture("scenario"))
            .with_artifact("zeta", ArtifactRef::new(ArtifactKind::Series, "series/zeta.json"))
            .with_artifact("alpha", ArtifactRef::new(ArtifactKind::Summary, "summary/alpha.json"));

        let keys: Vec<&str> = manifest.artifacts.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["alpha", "zeta"]);
        assert_eq!(manifest.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
    }

    #[test]
    fn prediction_artifact_kind_serializes_snake_case() {
        let encoded = serde_json::to_string(&ArtifactKind::Prediction)
            .expect("prediction artifact kind should serialize");
        assert_eq!(encoded, "\"prediction\"");
    }

    #[test]
    fn history_and_replay_artifact_kinds_serialize_snake_case() {
        let history = serde_json::to_string(&ArtifactKind::HistoryIndex)
            .expect("history artifact kind should serialize");
        let replay = serde_json::to_string(&ArtifactKind::ReplayIndex)
            .expect("replay artifact kind should serialize");
        assert_eq!(history, "\"history_index\"");
        assert_eq!(replay, "\"replay_index\"");
    }

    #[test]
    fn prediction_summary_report_keeps_deterministic_metric_order() {
        let report = PredictionSummaryReport::new(
            ScenarioId::fixture("scenario"),
            BTreeMap::from([
                (
                    MetricKey::fixture("beta"),
                    PredictionMetricIndicators {
                        n: 2,
                        mean: 2.0,
                        variance: 1.0,
                        std_dev: 1.0,
                        min: 1.0,
                        max: 3.0,
                        median: 2.0,
                        p90: 2.8,
                        p95: 2.9,
                        p99: 2.98,
                        confidence_lower_95: 0.0,
                        confidence_upper_95: 4.0,
                        confidence_margin_95: 2.0,
                        reliability_score: 0.5,
                        convergence_delta: 1.0,
                        convergence_ratio: 0.5,
                    },
                ),
                (
                    MetricKey::fixture("alpha"),
                    PredictionMetricIndicators {
                        n: 2,
                        mean: 10.0,
                        variance: 0.0,
                        std_dev: 0.0,
                        min: 10.0,
                        max: 10.0,
                        median: 10.0,
                        p90: 10.0,
                        p95: 10.0,
                        p99: 10.0,
                        confidence_lower_95: 10.0,
                        confidence_upper_95: 10.0,
                        confidence_margin_95: 0.0,
                        reliability_score: 1.0,
                        convergence_delta: 0.0,
                        convergence_ratio: 0.0,
                    },
                ),
            ]),
        );

        let encoded = serde_json::to_string(&report).expect("prediction report should serialize");
        let decoded: PredictionSummaryReport =
            serde_json::from_str(&encoded).expect("prediction report should deserialize");
        let keys: Vec<&str> = decoded.metrics.keys().map(MetricKey::as_str).collect();
        assert_eq!(keys, vec!["alpha", "beta"]);
    }

    #[test]
    fn history_index_report_round_trips_typed_diagnostics() {
        let mut report = HistoryIndexReport::new(ScenarioId::fixture("scenario"), "events.jsonl");
        report.entries = vec![
            HistoryIndexEntry {
                event_index: 0,
                run_id: "run-a".to_string(),
                step: 0,
                phase: "step_start".to_string(),
                ordinal: 0,
                event: "step_start".to_string(),
                severity: None,
                code: None,
            },
            HistoryIndexEntry {
                event_index: 1,
                run_id: "run-a".to_string(),
                step: 0,
                phase: "violation".to_string(),
                ordinal: 1,
                event: "violation".to_string(),
                severity: Some(DiagnosticSeverity::Warning),
                code: Some("RULE-001".to_string()),
            },
        ];

        let encoded = serde_json::to_string(&report).expect("history report should serialize");
        let decoded: HistoryIndexReport =
            serde_json::from_str(&encoded).expect("history report should deserialize");
        assert_eq!(decoded, report);
    }

    #[test]
    fn replay_index_report_keeps_sorted_runs() {
        let mut report = ReplayIndexReport::new(ScenarioId::fixture("scenario"), "events.jsonl", 3);
        report.runs.insert(
            "run-z".to_string(),
            ReplayRunIndex {
                first_event_index: 2,
                last_event_index: 3,
                first_step: 1,
                last_step: 1,
            },
        );
        report.runs.insert(
            "run-a".to_string(),
            ReplayRunIndex {
                first_event_index: 0,
                last_event_index: 1,
                first_step: 0,
                last_step: 1,
            },
        );

        let keys: Vec<&str> = report.runs.keys().map(String::as_str).collect();
        assert_eq!(keys, vec!["run-a", "run-z"]);
    }

    #[test]
    fn manifest_ref_upgrade_compat_adds_v2_version_and_sections() {
        let legacy = ManifestRef {
            schema_version: ARTIFACT_SCHEMA_VERSION_V1,
            setup_hash: "".to_string(),
            seed_strategy: "".to_string(),
            crate_version: "".to_string(),
            generated_at_unix_seconds: 0,
            scenario_id: ScenarioId::fixture("scenario"),
            artifacts: BTreeMap::from([
                ("events".to_string(), ArtifactRef::new(ArtifactKind::EventLog, "events.jsonl")),
                (
                    "history".to_string(),
                    ArtifactRef::new(ArtifactKind::HistoryIndex, "history.json"),
                ),
                (
                    "prediction".to_string(),
                    ArtifactRef::new(ArtifactKind::Prediction, "prediction.json"),
                ),
            ]),
            sections: ArtifactSchemaSections::default(),
        };

        let upgraded = legacy.upgrade_compat();
        assert_eq!(upgraded.schema_version, ARTIFACT_SCHEMA_VERSION_V2);
        assert_eq!(upgraded.setup_hash, "unknown");
        assert_eq!(upgraded.seed_strategy, "unknown");
        assert_eq!(upgraded.crate_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(
            upgraded.sections.accuracy,
            Some(ArtifactSectionRef::new("prediction", ArtifactKind::Prediction))
        );
        assert_eq!(
            upgraded.sections.debug,
            Some(ArtifactSectionRef::new("events", ArtifactKind::EventLog))
        );
        assert_eq!(
            upgraded.sections.history,
            Some(ArtifactSectionRef::new("history", ArtifactKind::HistoryIndex))
        );
    }

    #[test]
    fn manifest_ref_deserializes_v1_without_schema_fields() {
        let raw = r#"{
            "scenario_id":"scenario-v1",
            "artifacts":{
                "summary":{"kind":"summary","path":"summary.csv","content_type":"text/csv"}
            }
        }"#;
        let decoded: ManifestRef =
            serde_json::from_str(raw).expect("legacy v1 manifest should deserialize");
        assert_eq!(decoded.schema_version, ARTIFACT_SCHEMA_VERSION_V1);
        assert_eq!(decoded.setup_hash, "unknown");
        assert_eq!(decoded.seed_strategy, "unknown");
        assert_eq!(decoded.crate_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(decoded.generated_at_unix_seconds, 0);
        assert!(decoded.sections.is_empty());
        assert!(decoded.artifacts.contains_key("summary"));
    }
}
