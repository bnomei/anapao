# Prototype Contract Sketch (Evidence Only)

This sketch tested whether the desired states fit without changing public report schemas. It is not
production code and must not be applied as a patch.

```rust
pub enum CaptureSchedule {
    None,
    Final,
    Every {
        stride: NonZeroU64,
        include_initial: bool,
        include_final: bool,
    },
}

pub enum Selection<T> {
    None,
    All,
    Only(BTreeSet<T>),
}

pub struct CaptureConfig {
    pub schedule: CaptureSchedule,
    pub nodes: Selection<NodeId>,
    pub metrics: Selection<MetricKey>,
    pub variables: Selection<String>,
    pub transfers: Selection<EdgeId>,
}

pub struct AggregationConfig {
    pub schedule: CaptureSchedule,
    pub metrics: Selection<MetricKey>,
}
```

Learnings promoted into the design:

- A schedule alone cannot make `none()` honest because transfers are event-shaped and variables are
  an existing implicit channel.
- Final maps remain usable even when every diagnostic selection is none.
- Batch aggregation needs only schedule and metric selection; reusing full `CaptureConfig` would
  retain impossible-to-observe channels.
- A private sample can feed the existing `BatchRunSummary` and `aggregate_series` shapes without a
  public report migration.
