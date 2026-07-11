//! Run and batch execution controls: seeds, step limits, capture, and confidence.
//!
//! [`RunConfig`] pins a single deterministic run. [`BatchConfig`] plus
//! [`BatchRunTemplate`] describe Monte Carlo batches whose per-run seeds are
//! derived from `base_seed` and run index.

use std::{collections::BTreeSet, num::NonZeroU64};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::{EdgeId, MetricKey, NodeId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Sequential vs parallel batch scheduling strategy.
///
/// `Rayon` requires the `parallel` feature; without it the batch layer falls back
/// to single-thread execution.
pub enum ExecutionMode {
    #[default]
    SingleThread,
    Rayon,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
/// Two-sided confidence levels used by prediction-interval diagnostics.
pub enum ConfidenceLevel {
    P90,
    #[default]
    P95,
    P99,
}

impl ConfidenceLevel {
    /// Returns the two-sided normal-distribution Z-score for this confidence level.
    pub fn z_score(self) -> f64 {
        match self {
            Self::P90 => 1.644_853_626_951_472_2,
            Self::P95 => 1.959_963_984_540_054,
            Self::P99 => 2.575_829_303_548_900_4,
        }
    }
}

/// When diagnostics are sampled from a run lifecycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum CaptureSchedule {
    /// Do not retain step-addressed diagnostics.
    None,
    /// Retain diagnostics only after the terminal state is known.
    Final,
    /// Retain diagnostics at a positive periodic stride.
    Every {
        /// Positive stride between captured steps.
        stride: NonZeroU64,
        /// Whether step zero is retained.
        include_initial: bool,
        /// Whether the terminal state is retained when off-stride.
        include_final: bool,
    },
}

/// Selection policy for one diagnostic channel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
#[serde(tag = "kind", content = "items", rename_all = "snake_case")]
#[serde(bound(deserialize = "T: Ord + Deserialize<'de>"))]
#[serde(deny_unknown_fields)]
pub enum Selection<T> {
    /// Retain no values from this channel.
    None,
    /// Retain every available value from this channel.
    All,
    /// Retain the named values from this channel.
    Only(BTreeSet<T>),
}

impl<T> Selection<T> {
    /// Returns whether this policy explicitly retains no values.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// Returns the selected values when this is a concrete selection.
    pub fn only(&self) -> Option<&BTreeSet<T>> {
        match self {
            Self::Only(values) => Some(values),
            Self::None | Self::All => None,
        }
    }
}

/// Typed per-run diagnostic retention policy.
///
/// The fields are deliberately private so a config cannot reintroduce the old
/// empty-set-means-all or zero-stride sentinels. Use constructors and consuming
/// builders to make the requested retention policy explicit. Legacy five-field
/// JSON remains readable for the 0.2 migration, but serialization always emits
/// the canonical typed shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureConfig {
    schedule: CaptureSchedule,
    nodes: Selection<NodeId>,
    metrics: Selection<MetricKey>,
    variables: Selection<String>,
    transfers: Selection<EdgeId>,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            schedule: CaptureSchedule::Every {
                stride: NonZeroU64::MIN,
                include_initial: true,
                include_final: true,
            },
            nodes: Selection::All,
            metrics: Selection::All,
            variables: Selection::All,
            transfers: Selection::All,
        }
    }
}

impl CaptureConfig {
    /// Retains no step snapshots, series, variables, or transfer records.
    #[must_use]
    pub fn none() -> Self {
        Self {
            schedule: CaptureSchedule::None,
            nodes: Selection::None,
            metrics: Selection::None,
            variables: Selection::None,
            transfers: Selection::None,
        }
    }

    /// Retains final node, metric, and variable diagnostics without transfers.
    #[must_use]
    pub fn final_only() -> Self {
        Self {
            schedule: CaptureSchedule::Final,
            nodes: Selection::All,
            metrics: Selection::All,
            variables: Selection::All,
            transfers: Selection::None,
        }
    }

    /// Compatibility spelling for [`CaptureConfig::none`].
    #[deprecated(since = "0.2.0", note = "use CaptureConfig::none()")]
    #[must_use]
    pub fn disabled() -> Self {
        Self::none()
    }

    /// Returns the diagnostic sampling schedule.
    pub fn schedule(&self) -> &CaptureSchedule {
        &self.schedule
    }

    /// Returns the node snapshot selection.
    pub fn nodes(&self) -> &Selection<NodeId> {
        &self.nodes
    }

    /// Returns the metric series selection.
    pub fn metrics(&self) -> &Selection<MetricKey> {
        &self.metrics
    }

    /// Returns the variable snapshot selection.
    pub fn variables(&self) -> &Selection<String> {
        &self.variables
    }

    /// Returns the transfer-record selection.
    pub fn transfers(&self) -> &Selection<EdgeId> {
        &self.transfers
    }

    /// Replaces the diagnostic sampling schedule.
    #[must_use]
    pub fn with_schedule(mut self, schedule: CaptureSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// Replaces the node snapshot selection.
    #[must_use]
    pub fn with_nodes(mut self, nodes: Selection<NodeId>) -> Self {
        self.nodes = nodes;
        self
    }

    /// Replaces the metric series selection.
    #[must_use]
    pub fn with_metrics(mut self, metrics: Selection<MetricKey>) -> Self {
        self.metrics = metrics;
        self
    }

    /// Replaces the variable snapshot selection.
    #[must_use]
    pub fn with_variables(mut self, variables: Selection<String>) -> Self {
        self.variables = variables;
        self
    }

    /// Replaces the transfer-record selection.
    #[must_use]
    pub fn with_transfers(mut self, transfers: Selection<EdgeId>) -> Self {
        self.transfers = transfers;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if selection_is_empty(&self.nodes)
            || selection_is_empty(&self.metrics)
            || selection_is_empty(&self.variables)
            || selection_is_empty(&self.transfers)
        {
            return Err("Only selections must not be empty");
        }
        Ok(())
    }
}

fn selection_is_empty<T>(selection: &Selection<T>) -> bool {
    matches!(selection, Selection::Only(values) if values.is_empty())
}

/// Retention policy for batch aggregation output.
///
/// Batch aggregation is intentionally narrower than per-run diagnostics: it
/// controls only the metric series that become observable on a [`BatchReport`].
/// Node snapshots, variables, and transfer records have never had a batch
/// report destination and therefore are not configurable here. Per-run terminal
/// summaries remain available even when this policy is [`AggregationConfig::none`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AggregationConfig {
    schedule: CaptureSchedule,
    metrics: Selection<MetricKey>,
}

impl Default for AggregationConfig {
    fn default() -> Self {
        Self { schedule: CaptureConfig::default().schedule, metrics: Selection::All }
    }
}

impl AggregationConfig {
    /// Suppresses aggregate metric series.
    #[must_use]
    pub fn none() -> Self {
        Self { schedule: CaptureSchedule::None, metrics: Selection::None }
    }

    /// Retains only terminal aggregate metric points.
    #[must_use]
    pub fn final_only() -> Self {
        Self { schedule: CaptureSchedule::Final, metrics: Selection::All }
    }

    /// Returns the aggregate metric sampling schedule.
    pub fn schedule(&self) -> &CaptureSchedule {
        &self.schedule
    }

    /// Returns the aggregate metric selection.
    pub fn metrics(&self) -> &Selection<MetricKey> {
        &self.metrics
    }

    /// Replaces the aggregate metric sampling schedule.
    #[must_use]
    pub fn with_schedule(mut self, schedule: CaptureSchedule) -> Self {
        self.schedule = schedule;
        self
    }

    /// Replaces the aggregate metric selection.
    #[must_use]
    pub fn with_metrics(mut self, metrics: Selection<MetricKey>) -> Self {
        self.metrics = metrics;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), &'static str> {
        if selection_is_empty(&self.metrics) {
            return Err("Only selections must not be empty");
        }
        Ok(())
    }

    fn from_capture(capture: CaptureConfig) -> Self {
        Self { schedule: capture.schedule, metrics: capture.metrics }
    }
}

#[derive(Serialize)]
struct CurrentCaptureConfigWire<'a> {
    schedule: &'a CaptureSchedule,
    nodes: &'a Selection<NodeId>,
    metrics: &'a Selection<MetricKey>,
    variables: &'a Selection<String>,
    transfers: &'a Selection<EdgeId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentCaptureConfigWireOwned {
    schedule: CaptureSchedule,
    nodes: Selection<NodeId>,
    metrics: Selection<MetricKey>,
    variables: Selection<String>,
    transfers: Selection<EdgeId>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyCaptureConfigWire {
    #[serde(default)]
    capture_nodes: BTreeSet<NodeId>,
    #[serde(default)]
    capture_metrics: BTreeSet<MetricKey>,
    every_n_steps: u64,
    include_step_zero: bool,
    include_final_state: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CaptureConfigWire {
    Current(CurrentCaptureConfigWireOwned),
    Legacy(LegacyCaptureConfigWire),
}

impl Serialize for CaptureConfig {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CurrentCaptureConfigWire {
            schedule: &self.schedule,
            nodes: &self.nodes,
            metrics: &self.metrics,
            variables: &self.variables,
            transfers: &self.transfers,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CaptureConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let config = match CaptureConfigWire::deserialize(deserializer)? {
            CaptureConfigWire::Current(CurrentCaptureConfigWireOwned {
                schedule,
                nodes,
                metrics,
                variables,
                transfers,
            }) => Self { schedule, nodes, metrics, variables, transfers },
            CaptureConfigWire::Legacy(LegacyCaptureConfigWire {
                capture_nodes,
                capture_metrics,
                every_n_steps,
                include_step_zero,
                include_final_state,
            }) => {
                let stride = NonZeroU64::new(every_n_steps).ok_or_else(|| {
                    serde::de::Error::custom("every_n_steps must be greater than 0")
                })?;
                Self {
                    schedule: CaptureSchedule::Every {
                        stride,
                        include_initial: include_step_zero,
                        include_final: include_final_state,
                    },
                    nodes: if capture_nodes.is_empty() {
                        Selection::All
                    } else {
                        Selection::Only(capture_nodes)
                    },
                    metrics: if capture_metrics.is_empty() {
                        Selection::All
                    } else {
                        Selection::Only(capture_metrics)
                    },
                    variables: Selection::All,
                    transfers: Selection::All,
                }
            }
        };
        config.validate().map_err(serde::de::Error::custom)?;
        Ok(config)
    }
}

/// Deterministic controls for one simulation run.
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
    #[must_use]
    pub fn for_seed(seed: u64) -> Self {
        Self { seed, ..Self::default() }
    }

    /// Sets the run step limit.
    #[must_use]
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    /// Replaces retained diagnostic settings for the run.
    #[must_use]
    pub fn with_capture(mut self, capture: CaptureConfig) -> Self {
        self.capture = capture;
        self
    }
}

/// Seed-agnostic run template used by batch execution.
///
/// `aggregation` retains aggregate metric series only; it is not a per-run
/// diagnostic capture policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchRunTemplate {
    pub max_steps: u64,
    pub aggregation: AggregationConfig,
}

impl Default for BatchRunTemplate {
    fn default() -> Self {
        Self { max_steps: 100, aggregation: AggregationConfig::default() }
    }
}

impl BatchRunTemplate {
    #[must_use]
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.max_steps = max_steps;
        self
    }

    #[must_use]
    pub fn with_aggregation(mut self, aggregation: AggregationConfig) -> Self {
        self.aggregation = aggregation;
        self
    }

    /// Compatibility adapter for the 0.2 transition.
    ///
    /// Only the legacy capture schedule and metric selection map to batch
    /// aggregation. Node, variable, and transfer capture settings never had a
    /// corresponding [`BatchReport`] destination.
    #[deprecated(since = "0.2.0", note = "use BatchRunTemplate::with_aggregation()")]
    #[must_use]
    pub fn with_capture(self, capture: CaptureConfig) -> Self {
        self.with_aggregation(AggregationConfig::from_capture(capture))
    }
}

#[derive(Serialize)]
struct CurrentBatchRunTemplateWire<'a> {
    max_steps: u64,
    aggregation: &'a AggregationConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CurrentBatchRunTemplateWireOwned {
    max_steps: u64,
    aggregation: AggregationConfig,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyBatchRunTemplateWire {
    max_steps: u64,
    capture: CaptureConfig,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum BatchRunTemplateWire {
    Current(CurrentBatchRunTemplateWireOwned),
    Legacy(LegacyBatchRunTemplateWire),
}

impl Serialize for BatchRunTemplate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        CurrentBatchRunTemplateWire { max_steps: self.max_steps, aggregation: &self.aggregation }
            .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BatchRunTemplate {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let template = match BatchRunTemplateWire::deserialize(deserializer)? {
            BatchRunTemplateWire::Current(CurrentBatchRunTemplateWireOwned {
                max_steps,
                aggregation,
            }) => Self { max_steps, aggregation },
            BatchRunTemplateWire::Legacy(LegacyBatchRunTemplateWire { max_steps, capture }) => {
                Self { max_steps, aggregation: AggregationConfig::from_capture(capture) }
            }
        };
        template.aggregation.validate().map_err(serde::de::Error::custom)?;
        Ok(template)
    }
}

/// Deterministic Monte Carlo controls for many runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BatchConfig {
    pub runs: u64,
    pub base_seed: u64,
    pub execution_mode: ExecutionMode,
    pub run_template: BatchRunTemplate,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            runs: 1,
            base_seed: 0,
            execution_mode: ExecutionMode::default(),
            run_template: BatchRunTemplate::default(),
        }
    }
}

impl BatchConfig {
    #[must_use]
    pub fn for_runs(runs: u64) -> Self {
        Self { runs, ..Self::default() }
    }

    #[must_use]
    pub fn with_execution_mode(mut self, execution_mode: ExecutionMode) -> Self {
        self.execution_mode = execution_mode;
        self
    }

    #[must_use]
    pub fn with_base_seed(mut self, base_seed: u64) -> Self {
        self.base_seed = base_seed;
        self
    }

    #[must_use]
    pub fn with_run_template(mut self, run_template: BatchRunTemplate) -> Self {
        self.run_template = run_template;
        self
    }

    #[must_use]
    pub fn with_max_steps(mut self, max_steps: u64) -> Self {
        self.run_template.max_steps = max_steps;
        self
    }

    /// Replaces the batch aggregation policy.
    #[must_use]
    pub fn with_aggregation(mut self, aggregation: AggregationConfig) -> Self {
        self.run_template.aggregation = aggregation;
        self
    }

    /// Compatibility adapter for the 0.2 transition.
    ///
    /// Only the supplied capture schedule and metric selection apply to batch
    /// aggregation; node, variable, and transfer settings are ignored.
    #[deprecated(since = "0.2.0", note = "use BatchConfig::with_aggregation()")]
    #[must_use]
    pub fn with_capture(self, capture: CaptureConfig) -> Self {
        self.with_aggregation(AggregationConfig::from_capture(capture))
    }
}
