use super::*;

// ── Fixtures ───────────────────────────────────────────────────────

const EPS: f64 = 1e-12;

fn assert_close(actual: f64, expected: f64, what: &str) {
    assert!(
        (actual - expected).abs() < EPS,
        "{what}: expected {expected}, got {actual}"
    );
}

fn assumption(note: &str) -> FeeProvenance {
    FeeProvenance::OperatorAssumption {
        note: note.to_string(),
    }
}

/// Two Kraken-shaped volume bands. The rates are deliberately round numbers:
/// this corpus proves the *shape* arithmetic, and never claims to be Kraken's
/// live schedule.
fn kraken_shape() -> FeeScheduleShape {
    FeeScheduleShape::KrakenSpot {
        tiers: vec![
            VolumeTier {
                min_volume: 0.0,
                maker_percent: 0.25,
                taker_percent: 0.40,
            },
            VolumeTier {
                min_volume: 50_000.0,
                maker_percent: 0.20,
                taker_percent: 0.30,
            },
        ],
    }
}

fn kraken_schedule() -> FeeSchedule {
    FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        assumption("hand-picked round rates for the M1 golden corpus"),
        kraken_shape(),
    )
    .expect("kraken shape is valid")
}

/// Alpaca's published shape: a per-share base plus sell-only regulatory
/// pass-throughs. Values are again round test assumptions.
fn alpaca_shape() -> FeeScheduleShape {
    FeeScheduleShape::AlpacaUsEquity {
        per_share: 0.0,
        minimum: 0.0,
        sell_notional_percent: 0.001,
        sell_per_share: 0.0001,
        sell_per_order_cap: 5.0,
    }
}

fn alpaca_schedule() -> FeeSchedule {
    FeeSchedule::build(
        FeeVenue::AlpacaUsEquity,
        1,
        "2026-07-27",
        assumption("hand-picked round rates for the M1 golden corpus"),
        alpaca_shape(),
    )
    .expect("alpaca shape is valid")
}

// ── Kraken shape ───────────────────────────────────────────────────

#[test]
fn kraken_taker_charges_percent_of_notional_on_both_sides() {
    let binding = FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Taker)
        .expect("tier 0 exists");
    // 0.40 % of 10 × 100 = 1000 notional = 4.
    assert_close(binding.charge(FeeSide::Buy, 10.0, 100.0), 4.0, "buy");
    assert_close(binding.charge(FeeSide::Sell, 10.0, 100.0), 4.0, "sell");
}

#[test]
fn kraken_maker_and_higher_tiers_are_selectable_assumptions() {
    let maker = FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Maker)
        .expect("tier 0 exists");
    let deep = FeeScheduleBinding::build(kraken_schedule(), 1, LiquidityAssumption::Maker)
        .expect("tier 1 exists");
    assert_close(maker.charge(FeeSide::Buy, 10.0, 100.0), 2.5, "tier 0 maker");
    assert_close(deep.charge(FeeSide::Buy, 10.0, 100.0), 2.0, "tier 1 maker");
}

#[test]
fn selecting_a_tier_outside_the_schedule_is_refused() {
    let error = FeeScheduleBinding::build(kraken_schedule(), 2, LiquidityAssumption::Taker)
        .expect_err("tier 2 does not exist");
    assert_eq!(error, FeeScheduleError::UnknownTier { index: 2, count: 2 });
}

#[test]
fn kraken_tiers_must_be_ordered_and_non_empty() {
    let unordered = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        assumption("unordered"),
        FeeScheduleShape::KrakenSpot {
            tiers: vec![
                VolumeTier {
                    min_volume: 10.0,
                    maker_percent: 0.1,
                    taker_percent: 0.2,
                },
                VolumeTier {
                    min_volume: 0.0,
                    maker_percent: 0.1,
                    taker_percent: 0.2,
                },
            ],
        },
    )
    .expect_err("descending bands are ambiguous");
    assert_eq!(unordered, FeeScheduleError::UnorderedTiers { index: 1 });

    let empty = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        assumption("empty"),
        FeeScheduleShape::KrakenSpot { tiers: Vec::new() },
    )
    .expect_err("a schedule with no tier charges nothing silently");
    assert_eq!(empty, FeeScheduleError::EmptyTiers);
}

// ── Alpaca shape ───────────────────────────────────────────────────

#[test]
fn alpaca_charges_regulatory_pass_through_on_sells_only() {
    let binding = FeeScheduleBinding::build(alpaca_schedule(), 0, LiquidityAssumption::Taker)
        .expect("the equity shape has a single tier");
    assert_close(binding.charge(FeeSide::Buy, 100.0, 50.0), 0.0, "buy");
    // Sell 100 shares at 50: 0.001 % of 5000 = 0.05, plus 0.0001 × 100 = 0.01.
    assert_close(binding.charge(FeeSide::Sell, 100.0, 50.0), 0.06, "sell");
}

#[test]
fn alpaca_taf_is_capped_per_order() {
    let binding = FeeScheduleBinding::build(alpaca_schedule(), 0, LiquidityAssumption::Taker)
        .expect("the equity shape has a single tier");
    // 1,000,000 shares × 0.0001 = 100 uncapped, capped at 5. Notional fee is
    // 0.001 % of 1,000,000 × 1 = 10.
    assert_close(binding.charge(FeeSide::Sell, 1_000_000.0, 1.0), 15.0, "cap");
}

#[test]
fn alpaca_per_share_base_and_minimum_apply_to_both_sides() {
    let schedule = FeeSchedule::build(
        FeeVenue::AlpacaUsEquity,
        2,
        "2026-07-27",
        assumption("per-share base"),
        FeeScheduleShape::AlpacaUsEquity {
            per_share: 0.005,
            minimum: 1.0,
            sell_notional_percent: 0.0,
            sell_per_share: 0.0,
            sell_per_order_cap: 0.0,
        },
    )
    .expect("valid");
    let binding =
        FeeScheduleBinding::build(schedule, 0, LiquidityAssumption::Taker).expect("single tier");
    // 10 shares × 0.005 = 0.05, lifted to the 1.00 minimum.
    assert_close(binding.charge(FeeSide::Buy, 10.0, 100.0), 1.0, "minimum");
    // 1,000 shares × 0.005 = 5.00, above the minimum.
    assert_close(
        binding.charge(FeeSide::Sell, 1_000.0, 100.0),
        5.0,
        "per share",
    );
}

#[test]
fn the_equity_shape_rejects_a_tier_other_than_zero() {
    let error = FeeScheduleBinding::build(alpaca_schedule(), 1, LiquidityAssumption::Taker)
        .expect_err("the equity shape is not tiered");
    assert_eq!(error, FeeScheduleError::UnknownTier { index: 1, count: 1 });
}

// ── Versioning and provenance ──────────────────────────────────────

#[test]
fn a_schedule_records_venue_version_effective_date_and_provenance() {
    let schedule = kraken_schedule();
    assert_eq!(schedule.venue(), FeeVenue::KrakenSpot);
    assert_eq!(schedule.schedule_version(), 1);
    assert_eq!(schedule.effective_date(), "2026-07-27");
    assert_eq!(schedule.schema_version(), FEE_SCHEDULE_SCHEMA_VERSION);
    assert!(matches!(
        schedule.provenance(),
        FeeProvenance::OperatorAssumption { .. }
    ));
}

#[test]
fn a_malformed_effective_date_is_refused() {
    for bad in ["2026-7-27", "27-07-2026", "2026-13-01", "2026-02-30", ""] {
        let error = FeeSchedule::build(
            FeeVenue::KrakenSpot,
            1,
            bad,
            assumption("bad date"),
            kraken_shape(),
        )
        .expect_err("a schedule without a real effective date is not reproducible");
        assert_eq!(
            error,
            FeeScheduleError::InvalidEffectiveDate {
                value: bad.to_string()
            },
            "input {bad}"
        );
    }
}

#[test]
fn provenance_text_must_be_present_and_canonical() {
    for bad in ["", "  ", " leading", "trailing "] {
        let error = FeeSchedule::build(
            FeeVenue::KrakenSpot,
            1,
            "2026-07-27",
            FeeProvenance::OperatorAssumption {
                note: bad.to_string(),
            },
            kraken_shape(),
        )
        .expect_err("an unsourced rate table must say so in words");
        assert!(
            matches!(error, FeeScheduleError::InvalidText { field, .. } if field == "provenance.note"),
            "input {bad:?} produced {error:?}"
        );
    }
}

#[test]
fn a_published_schedule_must_name_its_source_document() {
    let error = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        FeeProvenance::VendorPublished {
            source: String::new(),
            retrieved_date: "2026-07-27".to_string(),
        },
        kraken_shape(),
    )
    .expect_err("a published claim without a source is not evidence");
    assert!(
        matches!(error, FeeScheduleError::InvalidText { field, .. } if field == "provenance.source"),
        "got {error:?}"
    );
}

#[test]
fn a_schedule_version_of_zero_is_refused() {
    let error = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        0,
        "2026-07-27",
        assumption("v0"),
        kraken_shape(),
    )
    .expect_err("versions start at 1 so an unset field cannot pass");
    assert_eq!(
        error,
        FeeScheduleError::OutOfRange {
            field: "schedule_version",
            expected: "a version of at least 1",
        }
    );
}

// ── Numeric hygiene ────────────────────────────────────────────────

#[test]
fn non_finite_and_negative_rates_are_refused() {
    let cases: [(f64, &str); 3] = [
        (f64::NAN, "kraken_spot.tiers[0].maker_percent"),
        (f64::INFINITY, "kraken_spot.tiers[0].maker_percent"),
        (-0.1, "kraken_spot.tiers[0].maker_percent"),
    ];
    for (value, field) in cases {
        let error = FeeSchedule::build(
            FeeVenue::KrakenSpot,
            1,
            "2026-07-27",
            assumption("bad rate"),
            FeeScheduleShape::KrakenSpot {
                tiers: vec![VolumeTier {
                    min_volume: 0.0,
                    maker_percent: value,
                    taker_percent: 0.1,
                }],
            },
        )
        .expect_err("a rate that is not a finite non-negative percent is refused");
        match error {
            FeeScheduleError::NonFiniteValue { field: actual }
            | FeeScheduleError::OutOfRange { field: actual, .. } => {
                assert_eq!(actual, field, "value {value}");
            }
            other => panic!("value {value} produced {other:?}"),
        }
    }
}

#[test]
fn a_rate_above_one_hundred_percent_is_refused() {
    let error = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        assumption("absurd"),
        FeeScheduleShape::KrakenSpot {
            tiers: vec![VolumeTier {
                min_volume: 0.0,
                maker_percent: 0.1,
                taker_percent: 100.5,
            }],
        },
    )
    .expect_err("a fee larger than the trade is a typo, not a schedule");
    assert_eq!(
        error,
        FeeScheduleError::OutOfRange {
            field: "kraken_spot.tiers[0].taker_percent",
            expected: "a percentage in [0, 100]",
        }
    );
}

#[test]
fn charging_a_non_finite_trade_yields_no_fee_rather_than_a_nan() {
    let binding = FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Taker)
        .expect("tier 0 exists");
    for (quantity, price) in [
        (f64::NAN, 100.0),
        (10.0, f64::INFINITY),
        (-1.0, 100.0),
        (10.0, -100.0),
    ] {
        let fee = binding.charge(FeeSide::Buy, quantity, price);
        assert!(
            fee.is_finite() && fee >= 0.0,
            "quantity {quantity} price {price} produced {fee}"
        );
    }
}

#[test]
fn too_many_tiers_is_refused() {
    let tiers = (0..=MAX_FEE_TIERS)
        .map(|index| VolumeTier {
            min_volume: index as f64,
            maker_percent: 0.1,
            taker_percent: 0.2,
        })
        .collect();
    let error = FeeSchedule::build(
        FeeVenue::KrakenSpot,
        1,
        "2026-07-27",
        assumption("too many"),
        FeeScheduleShape::KrakenSpot { tiers },
    )
    .expect_err("the tier table is bounded");
    assert_eq!(
        error,
        FeeScheduleError::TooManyTiers {
            limit: MAX_FEE_TIERS,
            found: MAX_FEE_TIERS + 1,
        }
    );
}

// ── Identity ───────────────────────────────────────────────────────

#[test]
fn bindings_that_differ_in_any_recorded_field_are_distinguishable() {
    let base =
        FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Taker).expect("valid");
    let maker =
        FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Maker).expect("valid");
    let tier =
        FeeScheduleBinding::build(kraken_schedule(), 1, LiquidityAssumption::Taker).expect("valid");
    let alpaca =
        FeeScheduleBinding::build(alpaca_schedule(), 0, LiquidityAssumption::Taker).expect("valid");
    let later = FeeScheduleBinding::build(
        FeeSchedule::build(
            FeeVenue::KrakenSpot,
            2,
            "2026-07-27",
            assumption("hand-picked round rates for the M1 golden corpus"),
            kraken_shape(),
        )
        .expect("valid"),
        0,
        LiquidityAssumption::Taker,
    )
    .expect("valid");

    for (left, right, what) in [
        (&base, &maker, "liquidity"),
        (&base, &tier, "tier"),
        (&base, &alpaca, "venue"),
        (&base, &later, "schedule version"),
    ] {
        assert_ne!(left, right, "{what} must change the binding");
    }
    let same =
        FeeScheduleBinding::build(kraken_schedule(), 0, LiquidityAssumption::Taker).expect("valid");
    assert_eq!(base, same, "an identical binding compares equal");
}

#[test]
fn errors_render_without_leaking_internals() {
    let rendered = [
        FeeScheduleError::EmptyTiers.to_string(),
        FeeScheduleError::TooManyTiers { limit: 1, found: 2 }.to_string(),
        FeeScheduleError::UnorderedTiers { index: 1 }.to_string(),
        FeeScheduleError::UnknownTier { index: 3, count: 2 }.to_string(),
        FeeScheduleError::NonFiniteValue { field: "a.b" }.to_string(),
        FeeScheduleError::OutOfRange {
            field: "a.b",
            expected: "a percentage in [0, 100]",
        }
        .to_string(),
        FeeScheduleError::InvalidText {
            field: "provenance.note",
            reason: "must not be empty",
        }
        .to_string(),
        FeeScheduleError::InvalidEffectiveDate {
            value: "nope".to_string(),
        }
        .to_string(),
    ];
    for message in rendered {
        assert!(!message.is_empty());
        assert!(!message.contains("panicked"), "{message}");
    }
}
