use super::*;
use crate::core::strategy_ir::{IndicatorKind, IndicatorRole, StrategyIr};

#[test]
fn general_builder_seals_saves_and_reloads_the_same_canonical_ir() {
    let mut builder = GeneralStrategyBuilder::new("GUI strategy", "operator");
    builder.add_indicator(IndicatorDraft::new("baseline", IndicatorKind::Ema, 20));
    builder.set_baseline_crossover("baseline");

    let sealed = builder.seal().expect("GUI graph is valid");
    let canonical = builder.canonical_text().expect("canonical text");
    let mut loaded = GeneralStrategyBuilder::from_canonical_text(&canonical).expect("reload");

    assert_eq!(loaded.seal().expect("loaded graph seals"), sealed);
    assert_eq!(loaded.canonical_text().expect("text"), canonical);
}

#[test]
fn declaration_order_equivalents_collapse_to_one_strategy_id() {
    let config = NnfxBuilderConfig::default();
    let definition = config.to_definition().expect("guided definition");
    let mut reversed = definition.clone();
    reversed.parameters.reverse();
    reversed.indicators.reverse();
    reversed.roles.reverse();

    let first = StrategyIr::build(&definition).expect("first");
    let second = StrategyIr::build(&reversed).expect("second");
    assert_eq!(first.strategy_id(), second.strategy_id());
}

#[test]
fn every_guided_profile_entry_toggle_direction_and_slot_variant_is_the_general_graph() {
    let profiles = NnfxProfile::ALL;
    let entries = NnfxEntryMode::ALL;
    let directions = DirectionConstraint::ALL;
    let mut checked = 0usize;

    for profile in profiles {
        for entry_mode in entries {
            for direction in directions {
                for bits in 0_u8..16 {
                    let mut guided = NnfxBuilderConfig {
                        profile,
                        entry_mode,
                        direction,
                        one_candle_rule: bits & 1 != 0,
                        bridge_too_far_rule: bits & 2 != 0,
                        news_filter: bits & 4 != 0,
                        market_filter: bits & 8 != 0,
                        ..NnfxBuilderConfig::default()
                    };
                    // Exercise each named slot with a non-default legal period.
                    guided.slots.atr.period = 10 + u32::from(bits & 3);
                    guided.slots.baseline.period = 18 + u32::from(bits & 3);
                    guided.slots.confirmation_1.period = 12 + u32::from(bits & 3);
                    guided.slots.confirmation_2.period = 13 + u32::from(bits & 3);
                    guided.slots.volume.period = 14 + u32::from(bits & 3);
                    guided.slots.exit.period = 15 + u32::from(bits & 3);
                    guided.slots.continuation.period = 16 + u32::from(bits & 3);

                    let guided_ir = guided.to_ir().expect("guided lowers");
                    let mut general = GeneralStrategyBuilder::from_definition(
                        guided
                            .equivalent_general_definition()
                            .expect("general graph"),
                    );
                    let general_ir = general.seal().expect("general seals");
                    assert_eq!(guided_ir, general_ir, "{guided:?}");
                    let round_trip = StrategyIr::from_json_slice(
                        &serde_json::to_vec(&guided_ir).expect("serialize"),
                    )
                    .expect("round-trip");
                    assert_eq!(round_trip, guided_ir);
                    checked += 1;
                }
            }
        }
    }

    assert_eq!(checked, 4 * 4 * 3 * 16);
    let roles: Vec<_> = NnfxBuilderConfig::default()
        .to_ir()
        .unwrap()
        .definition()
        .roles
        .iter()
        .map(|assignment| assignment.role)
        .collect();
    assert_eq!(
        roles,
        vec![
            IndicatorRole::Atr,
            IndicatorRole::Baseline,
            IndicatorRole::Confirmation1,
            IndicatorRole::Confirmation2,
            IndicatorRole::Continuation,
            IndicatorRole::Exit,
            IndicatorRole::Volume,
        ]
    );
}

#[test]
fn invalid_live_edits_report_validation_without_destroying_the_last_sealed_graph() {
    let mut builder = GeneralStrategyBuilder::from_definition(
        NnfxBuilderConfig::default().to_definition().unwrap(),
    );
    let before = builder.seal().unwrap();
    builder.definition_mut().indicators[0].id.clear();
    assert!(builder.validation().is_err());
    assert_eq!(builder.last_valid_ir(), Some(&before));
}
