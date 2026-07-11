# Checked API vocabulary prototype

Evidence-only sketch. Names and signatures below are frozen as the intended contract, but this is
not production code.

```rust
#[non_exhaustive]
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

#[non_exhaustive]
pub enum ConnectionSpec {
    Resource(ResourceConnection),
    State(StateConnection),
}

#[non_exhaustive]
pub enum StateTarget {
    Node,
    ResourceConnection(EdgeId),
    StateConnection(EdgeId),
    Formula(EdgeId),
}

#[must_use = "a ScenarioBuilder must be built or its configured scenario is discarded"]
pub struct ScenarioBuilder { /* private deterministic state */ }

impl ScenarioBuilder {
    pub fn new(id: ScenarioId) -> Self;
    pub fn insert_node(&mut self, node: ScenarioNode) -> Result<(), SetupError>;
    pub fn insert_edge(&mut self, edge: ScenarioEdge) -> Result<(), SetupError>;
    #[must_use = "use the returned builder to retain the inserted node"]
    pub fn with_node(self, node: ScenarioNode) -> Result<Self, SetupError>;
    #[must_use = "use the returned builder to retain the inserted edge"]
    pub fn with_edge(self, edge: ScenarioEdge) -> Result<Self, SetupError>;
    pub fn build(self) -> Result<Scenario, SetupError>;
}

impl TryFrom<ScenarioSpec> for Scenario {
    type Error = SetupError;
}

impl From<Scenario> for ScenarioSpec { /* wire DTO reconstruction */ }
```

Key result of the sketch:

- The checked builder can support both standard mutation-style insertion and existing Anapao
  consuming chains without typestate.
- Family constructors on `ScenarioNode` and resource/state constructors on `ScenarioEdge` remove
  independent tag/payload selection.
- `StateTarget` owns the target ID only in variants that require it.
- Checked values can expose accessors while keeping fields private.
- A later `scenario!` macro can expand exclusively to this public builder API.
