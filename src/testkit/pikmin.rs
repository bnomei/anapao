//! Typed Pikmin fixture builders and profile presets for integration tests.

use std::collections::BTreeMap;

use crate::error::SetupError;
use crate::types::{
    EdgeId, EdgeSpec, EndConditionSpec, MetricKey, NodeId, NodeKind, NodeSpec, ScenarioId,
    ScenarioSpec, TransferSpec, VariableSourceSpec,
};
use crate::validation::{compile_scenario, CompiledScenario};

const SCALE: f64 = 1_000_000.0;

const NODE_DAYS_SPENT: &str = "n02_days_spent";
const NODE_PIKMIN: &str = "n09_pikmin";
const NODE_PIKMIN_DIE: &str = "n15_pikmin_die";
const NODE_SHIP_PARTS: &str = "n17_ship_parts";

fn scaled(value: f64) -> i64 {
    (value * SCALE).round() as i64
}

/// Named fixture tuning profiles used in Pikmin scenario tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PikminFixtureProfile {
    BadEndingBiased,
    GoodEndingBiased,
    Balanced,
}

impl PikminFixtureProfile {
    /// Returns the canonical tuning for this profile.
    pub fn tuning(self) -> PikminFixtureTuning {
        match self {
            Self::BadEndingBiased => PikminFixtureTuning {
                enemy_fight_per_step: 3,
                explore_tokens_per_step: 1,
                ship_part_chance_percent: 5.0,
            },
            Self::GoodEndingBiased => PikminFixtureTuning {
                enemy_fight_per_step: 5,
                explore_tokens_per_step: 3,
                ship_part_chance_percent: 100.0,
            },
            Self::Balanced => PikminFixtureTuning {
                enemy_fight_per_step: 2,
                explore_tokens_per_step: 2,
                ship_part_chance_percent: 60.0,
            },
        }
    }
}

/// Typed knobs for constructing Pikmin-style scenario fixtures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PikminFixtureTuning {
    pub enemy_fight_per_step: u64,
    pub explore_tokens_per_step: u64,
    pub ship_part_chance_percent: f64,
}

impl PikminFixtureTuning {
    /// Creates validated fixture tuning.
    pub fn new(
        enemy_fight_per_step: u64,
        explore_tokens_per_step: u64,
        ship_part_chance_percent: f64,
    ) -> Result<Self, SetupError> {
        let tuning =
            Self { enemy_fight_per_step, explore_tokens_per_step, ship_part_chance_percent };
        validate_tuning(&tuning)?;
        Ok(tuning)
    }
}

/// Canonical node id helper for day progression.
pub fn days_spent_node_id() -> NodeId {
    NodeId::fixture(NODE_DAYS_SPENT)
}

/// Canonical node id helper for active Pikmin count.
pub fn pikmin_node_id() -> NodeId {
    NodeId::fixture(NODE_PIKMIN)
}

/// Canonical node id helper for cumulative Pikmin losses.
pub fn pikmin_die_node_id() -> NodeId {
    NodeId::fixture(NODE_PIKMIN_DIE)
}

/// Canonical node id helper for ship part progression.
pub fn ship_parts_node_id() -> NodeId {
    NodeId::fixture(NODE_SHIP_PARTS)
}

/// Canonical metric key helper for day progression.
pub fn days_spent_metric_key() -> MetricKey {
    MetricKey::fixture(NODE_DAYS_SPENT)
}

/// Canonical metric key helper for active Pikmin count.
pub fn pikmin_metric_key() -> MetricKey {
    MetricKey::fixture(NODE_PIKMIN)
}

/// Canonical metric key helper for cumulative Pikmin losses.
pub fn pikmin_die_metric_key() -> MetricKey {
    MetricKey::fixture(NODE_PIKMIN_DIE)
}

/// Canonical metric key helper for ship part progression.
pub fn ship_parts_metric_key() -> MetricKey {
    MetricKey::fixture(NODE_SHIP_PARTS)
}

/// Builds a Pikmin scenario for one profile.
pub fn pikmin_scenario_for_profile(
    profile: PikminFixtureProfile,
) -> Result<ScenarioSpec, SetupError> {
    pikmin_scenario(profile.tuning())
}

/// Builds and compiles a Pikmin scenario for one profile.
pub fn compiled_pikmin_scenario_for_profile(
    profile: PikminFixtureProfile,
) -> Result<CompiledScenario, SetupError> {
    compile_scenario(&pikmin_scenario_for_profile(profile)?)
}

/// Builds a Pikmin scenario for custom tuning.
pub fn pikmin_scenario(tuning: PikminFixtureTuning) -> Result<ScenarioSpec, SetupError> {
    validate_tuning(&tuning)?;

    let select_level = NodeId::fixture("n01_select_level");
    let days_spent = days_spent_node_id();
    let spawn_pellets = NodeId::fixture("n03_spawn_pellets");
    let pellets = NodeId::fixture("n04_pellets");
    let pooled_pellets = NodeId::fixture("n05_pooled_pellets");
    let onion = NodeId::fixture("n06_onion");
    let seeds = NodeId::fixture("n07_seeds");
    let plucking = NodeId::fixture("n08_plucking");
    let pikmin = pikmin_node_id();
    let spawn_enemies = NodeId::fixture("n10_spawn_enemies");
    let enemies = NodeId::fixture("n11_enemies");
    let fight_enemy = NodeId::fixture("n12_fight_enemy");
    let defeat_enemy = NodeId::fixture("n13_defeat_enemy");
    let enemy_carcasses = NodeId::fixture("n14_enemy_carcasses");
    let pikmin_die = pikmin_die_node_id();
    let explore = NodeId::fixture("n16_explore");
    let ship_parts = ship_parts_node_id();

    let mut scenario = ScenarioSpec::new(ScenarioId::fixture("scenario-pikmin-diagram"))
        .with_node(NodeSpec::new(select_level.clone(), NodeKind::Source).with_initial_value(1.0))
        .with_node(NodeSpec::new(days_spent.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(spawn_pellets.clone(), NodeKind::Source).with_initial_value(60.0))
        .with_node(NodeSpec::new(pellets.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(pooled_pellets.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(onion.clone(), NodeKind::Process))
        .with_node(NodeSpec::new(seeds.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(plucking.clone(), NodeKind::Process))
        .with_node(NodeSpec::new(pikmin.clone(), NodeKind::Pool).with_initial_value(20.0))
        .with_node(NodeSpec::new(spawn_enemies.clone(), NodeKind::Source).with_initial_value(20.0))
        .with_node(NodeSpec::new(enemies.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(fight_enemy.clone(), NodeKind::SortingGate))
        .with_node(NodeSpec::new(defeat_enemy.clone(), NodeKind::Process))
        .with_node(NodeSpec::new(enemy_carcasses.clone(), NodeKind::Pool))
        .with_node(NodeSpec::new(pikmin_die.clone(), NodeKind::Drain))
        .with_node(NodeSpec::new(explore.clone(), NodeKind::SortingGate))
        .with_node(NodeSpec::new(ship_parts.clone(), NodeKind::Pool))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e01_select_level_to_days"),
            select_level,
            days_spent.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e02_spawn_pellets_to_pellets"),
            spawn_pellets,
            pellets.clone(),
            TransferSpec::Expression { formula: "pellet_roll".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e03_pellets_to_pooled_pellets"),
            pellets,
            pooled_pellets.clone(),
            TransferSpec::Remaining,
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e04_pooled_pellets_to_onion"),
            pooled_pellets.clone(),
            onion.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e05_onion_to_seeds"),
            onion,
            seeds.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e06_seeds_to_plucking"),
            seeds,
            plucking.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e07_plucking_to_pikmin"),
            plucking,
            pikmin.clone(),
            TransferSpec::Fixed { amount: 1.0 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e08_spawn_enemies_to_enemies"),
            spawn_enemies,
            enemies.clone(),
            TransferSpec::Expression { formula: "enemy_roll".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e09_enemies_to_fight_enemy"),
            enemies,
            fight_enemy.clone(),
            TransferSpec::Fixed { amount: tuning.enemy_fight_per_step as f64 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e10_fight_enemy_to_defeat_enemy"),
            fight_enemy,
            defeat_enemy.clone(),
            TransferSpec::Expression { formula: "100".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e11_defeat_enemy_to_explore"),
            defeat_enemy.clone(),
            explore.clone(),
            TransferSpec::Fixed { amount: tuning.explore_tokens_per_step as f64 },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e12_defeat_enemy_to_enemy_carcasses"),
            defeat_enemy,
            enemy_carcasses.clone(),
            TransferSpec::Expression { formula: "carcass_roll".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e13_enemy_carcasses_to_pooled_pellets"),
            enemy_carcasses,
            pooled_pellets,
            TransferSpec::Remaining,
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e14_pikmin_to_pikmin_die"),
            pikmin,
            pikmin_die,
            TransferSpec::Expression { formula: "loss_roll".to_string() },
        ))
        .with_edge(EdgeSpec::new(
            EdgeId::fixture("e15_explore_to_ship_parts"),
            explore,
            ship_parts.clone(),
            TransferSpec::Expression { formula: tuning.ship_part_chance_percent.to_string() },
        ));

    scenario.variables.sources = BTreeMap::from([
        ("pellet_roll".to_string(), VariableSourceSpec::RandomInterval { min: 6, max: 60 }),
        ("enemy_roll".to_string(), VariableSourceSpec::RandomInterval { min: 10, max: 20 }),
        ("carcass_roll".to_string(), VariableSourceSpec::RandomInterval { min: 1, max: 10 }),
        ("loss_roll".to_string(), VariableSourceSpec::RandomInterval { min: 0, max: 6 }),
    ]);
    scenario.tracked_metrics.insert(days_spent_metric_key());
    scenario.tracked_metrics.insert(pikmin_metric_key());
    scenario.tracked_metrics.insert(pikmin_die_metric_key());
    scenario.tracked_metrics.insert(ship_parts_metric_key());
    scenario.end_conditions = vec![
        EndConditionSpec::NodeAtLeast { node_id: ship_parts.clone(), value_scaled: scaled(30.0) },
        EndConditionSpec::NodeAtLeast { node_id: days_spent, value_scaled: scaled(30.0) },
    ];
    Ok(scenario)
}

fn invalid_tuning(name: &str, reason: &str) -> SetupError {
    SetupError::InvalidParameter { name: name.to_string(), reason: reason.to_string() }
}

fn validate_tuning(tuning: &PikminFixtureTuning) -> Result<(), SetupError> {
    if tuning.enemy_fight_per_step == 0 {
        return Err(invalid_tuning(
            "testkit.pikmin.enemy_fight_per_step",
            "must be greater than 0",
        ));
    }
    if tuning.explore_tokens_per_step == 0 {
        return Err(invalid_tuning(
            "testkit.pikmin.explore_tokens_per_step",
            "must be greater than 0",
        ));
    }
    if !tuning.ship_part_chance_percent.is_finite() {
        return Err(invalid_tuning("testkit.pikmin.ship_part_chance_percent", "must be finite"));
    }
    if !(0.0..=100.0).contains(&tuning.ship_part_chance_percent) {
        return Err(invalid_tuning(
            "testkit.pikmin.ship_part_chance_percent",
            "must be within 0..=100",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        compiled_pikmin_scenario_for_profile, days_spent_metric_key, pikmin_die_metric_key,
        pikmin_metric_key, pikmin_scenario, ship_parts_metric_key, PikminFixtureProfile,
        PikminFixtureTuning,
    };
    use crate::error::SetupError;

    #[test]
    fn profile_tunings_are_distinct_and_sane() {
        let bad = PikminFixtureProfile::BadEndingBiased.tuning();
        let balanced = PikminFixtureProfile::Balanced.tuning();
        let good = PikminFixtureProfile::GoodEndingBiased.tuning();

        assert!(bad.ship_part_chance_percent < balanced.ship_part_chance_percent);
        assert!(balanced.ship_part_chance_percent < good.ship_part_chance_percent);
        assert!(bad.explore_tokens_per_step < good.explore_tokens_per_step);
        assert!(bad.enemy_fight_per_step < good.enemy_fight_per_step);
    }

    #[test]
    fn scenario_rejects_invalid_tuning() {
        let err = pikmin_scenario(PikminFixtureTuning {
            enemy_fight_per_step: 0,
            explore_tokens_per_step: 1,
            ship_part_chance_percent: 50.0,
        })
        .expect_err("zero enemy_fight_per_step must fail");
        assert!(matches!(
            err,
            SetupError::InvalidParameter { name, reason }
                if name == "testkit.pikmin.enemy_fight_per_step"
                    && reason == "must be greater than 0"
        ));

        let err = pikmin_scenario(PikminFixtureTuning {
            enemy_fight_per_step: 1,
            explore_tokens_per_step: 0,
            ship_part_chance_percent: 50.0,
        })
        .expect_err("zero explore_tokens_per_step must fail");
        assert!(matches!(
            err,
            SetupError::InvalidParameter { name, reason }
                if name == "testkit.pikmin.explore_tokens_per_step"
                    && reason == "must be greater than 0"
        ));

        let err = pikmin_scenario(PikminFixtureTuning {
            enemy_fight_per_step: 1,
            explore_tokens_per_step: 1,
            ship_part_chance_percent: 101.0,
        })
        .expect_err("out-of-range chance must fail");
        assert!(matches!(
            err,
            SetupError::InvalidParameter { name, reason }
                if name == "testkit.pikmin.ship_part_chance_percent"
                    && reason == "must be within 0..=100"
        ));
    }

    #[test]
    fn balanced_profile_compiles_with_canonical_tracked_metrics() {
        let compiled = compiled_pikmin_scenario_for_profile(PikminFixtureProfile::Balanced)
            .expect("balanced profile should compile");
        let tracked = &compiled.scenario.tracked_metrics;

        assert!(tracked.contains(&days_spent_metric_key()));
        assert!(tracked.contains(&pikmin_metric_key()));
        assert!(tracked.contains(&pikmin_die_metric_key()));
        assert!(tracked.contains(&ship_parts_metric_key()));
    }

    #[test]
    fn tuning_constructor_validates_inputs() {
        let tuning =
            PikminFixtureTuning::new(2, 2, 60.0).expect("valid custom tuning should construct");
        assert_eq!(tuning.enemy_fight_per_step, 2);
        assert_eq!(tuning.explore_tokens_per_step, 2);
        assert_eq!(tuning.ship_part_chance_percent, 60.0);
    }
}
