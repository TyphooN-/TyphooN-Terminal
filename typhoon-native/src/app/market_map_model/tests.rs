use super::*;
use typhoon_engine::core::fundamentals::Fundamentals;

fn fundamental(symbol: &str, sector: &str, market_cap: Option<f64>) -> Fundamentals {
    Fundamentals {
        symbol: symbol.to_string(),
        sector: sector.to_string(),
        market_cap,
        ..Fundamentals::default()
    }
}

#[test]
fn static_model_filters_invalid_caps_and_normalizes_symbols_and_sectors() {
    let fundamentals = vec![
        fundamental("brk/b.eq", "  Financials  ", Some(600.0)),
        fundamental("MISSING", "Technology", None),
        fundamental("ZERO", "Technology", Some(0.0)),
        fundamental("NEGATIVE", "Technology", Some(-1.0)),
        fundamental("NAN", "Technology", Some(f64::NAN)),
        fundamental("INFINITE", "Technology", Some(f64::INFINITY)),
        fundamental("OTHER", "   ", Some(100.0)),
    ];

    let model = build_market_map_model(&fundamentals);

    assert_eq!(model.grand_total, 700.0);
    assert_eq!(model.sectors.len(), 2);
    assert_eq!(model.sectors[0].sector, "Financials");
    assert_eq!(model.sectors[0].symbols[0].symbol, "brk/b.eq");
    assert_eq!(model.sectors[0].symbols[0].watchlist_key, "BRKB");
    assert_eq!(model.sectors[1].sector, "Other");
}

#[test]
fn static_model_orders_equal_caps_deterministically() {
    let fundamentals = vec![
        fundamental("ZZZ", "Beta", Some(100.0)),
        fundamental("BBB", "Alpha", Some(50.0)),
        fundamental("AAA", "Alpha", Some(50.0)),
        fundamental("CCC", "Gamma", Some(100.0)),
    ];

    let model = build_market_map_model(&fundamentals);

    let sectors: Vec<&str> = model
        .sectors
        .iter()
        .map(|band| band.sector.as_str())
        .collect();
    assert_eq!(sectors, ["Alpha", "Beta", "Gamma"]);
    let alpha_symbols: Vec<&str> = model.sectors[0]
        .symbols
        .iter()
        .map(|symbol| symbol.symbol.as_str())
        .collect();
    assert_eq!(alpha_symbols, ["AAA", "BBB"]);
}

#[test]
fn empty_static_model_has_no_sectors_or_cap() {
    let model = build_market_map_model(&[]);

    assert!(model.sectors.is_empty());
    assert_eq!(model.grand_total, 0.0);
}

#[test]
fn static_model_omits_sectors_whose_aggregate_cap_overflows() {
    let fundamentals = vec![
        fundamental("HUGE_A", "Corrupt", Some(f64::MAX)),
        fundamental("HUGE_B", "Corrupt", Some(f64::MAX)),
        fundamental("VALID", "Technology", Some(100.0)),
    ];

    let model = build_market_map_model(&fundamentals);

    assert_eq!(model.sectors.len(), 1);
    assert_eq!(model.sectors[0].sector, "Technology");
    assert_eq!(model.grand_total, 100.0);
}

#[test]
fn cap_weighted_change_normalizes_before_multiplying_large_values() {
    let fundamentals = vec![
        fundamental("HUGE_A", "Technology", Some(f64::MAX / 4.0)),
        fundamental("HUGE_B", "Technology", Some(f64::MAX / 4.0)),
    ];
    let model = build_market_map_model(&fundamentals);

    let change = cap_weighted_change(&model.sectors[0], |key| match key {
        "HUGE_A" => f64::MAX,
        "HUGE_B" => -f64::MAX,
        _ => 0.0,
    });

    assert_eq!(change, 0.0);
}

#[test]
fn cap_weighted_change_ignores_nonfinite_live_changes() {
    let fundamentals = vec![
        fundamental("BAD", "Technology", Some(100.0)),
        fundamental("GOOD", "Technology", Some(100.0)),
    ];
    let model = build_market_map_model(&fundamentals);

    let change = cap_weighted_change(&model.sectors[0], |key| match key {
        "BAD" => f64::NAN,
        "GOOD" => 2.0,
        _ => 0.0,
    });

    assert_eq!(change, 1.0);
}
